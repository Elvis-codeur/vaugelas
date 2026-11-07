//! CPU inference of the paradigm-energy model (training/ebm_model.py `EBM`, encoder = training/featmodel.py `Net`).
//! Weights come from training/export_ebm.py (`model.frcm`). The math mirrors PyTorch eval mode exactly:
//! reading embedding = sum of its atom embeddings; word input = mean over present readings + sum of extra-atom
//! embeddings; + learned position; nn.TransformerEncoder (norm_first, ReLU FFN, no dropout); final LayerNorm;
//! energy of candidate c at word i = -(cand_emb(c) . proj(h_i)) / sqrt(d); score = 1 - softmax(-E)[shown form].
pub mod cands;
pub mod pipeline;
use serde_json::Value;
use std::collections::HashMap;
use std::io::Read;

pub const MAXR: usize = 6;
pub const MAXF: usize = 12;
pub const MAXT: usize = 96;
pub const MAXK: usize = 8;
const CLOSED: [&str; 5] = ["DET", "PREP", "PRON", "CONJ", "NEG"];

/// a Linear weight (dout x din): f32, or int8 with one scale per output row (symmetric, scale = max|row| / 127)
pub enum Wt { F32(Vec<f32>), I8 { q: Vec<i8>, s: Vec<f32>, din: usize } }

impl Wt {
    fn quantize(w: Vec<f32>, din: usize) -> Wt {
        let mut q = Vec::with_capacity(w.len()); let mut s = Vec::with_capacity(w.len() / din);
        for row in w.chunks_exact(din) {
            let m = row.iter().fold(0f32, |a, v| a.max(v.abs()));
            let sc = if m > 0.0 { m / 127.0 } else { 1.0 };
            s.push(sc);
            for v in row { q.push((v / sc).round().clamp(-127.0, 127.0) as i8); }
        }
        Wt::I8 { q, s, din }
    }
    pub fn bytes(&self) -> usize { match self { Wt::F32(w) => 4 * w.len(), Wt::I8 { q, s, .. } => q.len() + 4 * s.len() } }
}

thread_local! { static SCRATCH: std::cell::RefCell<Vec<f32>> = std::cell::RefCell::new(Vec::new()); }

/// y = x . W^T + b for either weight kind (int8: rows expanded to f32 in a per-thread scratch buffer, then sgemm)
fn linear_w(x: &[f32], t: usize, din: usize, w: &Wt, b: &[f32], dout: usize) -> Vec<f32> {
    match w {
        Wt::F32(w) => linear(x, t, din, w, b, dout),
        Wt::I8 { q, s, din: dd } => SCRATCH.with(|sc| {
            let mut buf = sc.borrow_mut();
            buf.resize(q.len(), 0.0);
            for (o, (row, out)) in q.chunks_exact(*dd).zip(buf.chunks_exact_mut(*dd)).enumerate() {
                let f = s[o]; for (a, v) in out.iter_mut().zip(row) { *a = *v as f32 * f; }
            }
            linear(x, t, din, &buf, b, dout)
        }),
    }
}

pub struct Layer {
    in_w: Wt, in_b: Vec<f32>, out_w: Wt, out_b: Vec<f32>,
    l1_w: Wt, l1_b: Vec<f32>, l2_w: Wt, l2_b: Vec<f32>,
    n1_w: Vec<f32>, n1_b: Vec<f32>, n2_w: Vec<f32>, n2_b: Vec<f32>,
}

pub struct Model {
    pub d: usize, pub heads: usize, ff: usize,
    pub vocab: Vec<String>, stoi: HashMap<String, u32>, lem_unk: u32,
    emb: Vec<f32>, pos: Vec<f32>, layers: Vec<Layer>, norm_w: Vec<f32>, norm_b: Vec<f32>,
    proj_w: Wt, proj_b: Vec<f32>,
}

fn err<T>(m: &str) -> Result<T, String> { Err(m.to_string()) }

impl Model {
    pub fn load(path: &str) -> Result<Model, String> { Self::load_opts(path, false) }

    /// int8 = true: every Linear of the encoder and the energy projection stored as int8 + per-row scales
    pub fn load_opts(path: &str, int8: bool) -> Result<Model, String> {
        let mut f = std::fs::File::open(path).map_err(|e| format!("{path}: {e}"))?;
        let mut buf = Vec::new(); f.read_to_end(&mut buf).map_err(|e| e.to_string())?;
        if &buf[..6] != b"FRCM1\n" { return err("not a model.frcm file"); }
        let hl = u64::from_le_bytes(buf[6..14].try_into().unwrap()) as usize;
        let head: Value = serde_json::from_slice(&buf[14..14 + hl]).map_err(|e| e.to_string())?;
        let data = &buf[14 + hl..];
        let tens = head["tensors"].as_object().ok_or("no tensors")?;
        let get = |name: &str| -> Result<Vec<f32>, String> {
            let t = tens.get(name).ok_or(format!("missing tensor {name}"))?;
            let n: usize = t["shape"].as_array().unwrap().iter().map(|x| x.as_u64().unwrap() as usize).product();
            let off = t["offset"].as_u64().unwrap() as usize;
            Ok(data[off..off + 4 * n].chunks_exact(4).map(|c| f32::from_le_bytes(c.try_into().unwrap())).collect())
        };
        let cfg = &head["cfg"];
        let d = cfg["d"].as_u64().unwrap() as usize; let nl = cfg["layers"].as_u64().unwrap() as usize;
        let mut heads = cfg.get("heads").and_then(|h| h.as_u64()).unwrap_or(4) as usize;
        if d % heads != 0 { heads = 4; }
        let vocab: Vec<String> = head["vocab"].as_array().unwrap().iter().map(|s| s.as_str().unwrap().to_string()).collect();
        let stoi: HashMap<String, u32> = vocab.iter().enumerate().map(|(i, s)| (s.clone(), i as u32)).collect();
        let lem_unk = *stoi.get("lem=?").unwrap_or(&2);
        let mut layers = Vec::new();
        for l in 0..nl {
            let p = |s: &str| format!("net.body.layers.{l}.{s}");
            let ff = tens[&p("linear1.bias")]["shape"][0].as_u64().unwrap() as usize;
            let w = |v: Vec<f32>, din: usize| if int8 { Wt::quantize(v, din) } else { Wt::F32(v) };
            layers.push(Layer {
                in_w: w(get(&p("self_attn.in_proj_weight"))?, d), in_b: get(&p("self_attn.in_proj_bias"))?,
                out_w: w(get(&p("self_attn.out_proj.weight"))?, d), out_b: get(&p("self_attn.out_proj.bias"))?,
                l1_w: w(get(&p("linear1.weight"))?, d), l1_b: get(&p("linear1.bias"))?,
                l2_w: w(get(&p("linear2.weight"))?, ff), l2_b: get(&p("linear2.bias"))?,
                n1_w: get(&p("norm1.weight"))?, n1_b: get(&p("norm1.bias"))?,
                n2_w: get(&p("norm2.weight"))?, n2_b: get(&p("norm2.bias"))?,
            });
        }
        let l1b = layers[0].l1_b.len();
        Ok(Model { d, heads, ff: l1b, stoi, lem_unk, vocab,
            emb: get("net.emb.weight")?, pos: get("net.pos.weight")?, layers,
            norm_w: get("net.norm.weight")?, norm_b: get("net.norm.bias")?,
            proj_w: if int8 { Wt::quantize(get("proj.weight")?, d) } else { Wt::F32(get("proj.weight")?) }, proj_b: get("proj.bias")? })
    }

    /// bytes of the weights held in memory (embedding table + positions + Linear weights + biases/norms)
    pub fn weight_bytes(&self) -> usize {
        let mut n = 4 * (self.emb.len() + self.pos.len() + self.norm_w.len() + self.norm_b.len() + self.proj_b.len()) + self.proj_w.bytes();
        for l in &self.layers {
            n += l.in_w.bytes() + l.out_w.bytes() + l.l1_w.bytes() + l.l2_w.bytes();
            n += 4 * (l.in_b.len() + l.out_b.len() + l.l1_b.len() + l.l2_b.len() + l.n1_w.len() + l.n1_b.len() + l.n2_w.len() + l.n2_b.len());
        }
        n
    }

    /// featmodel.Vocab.get: unknown "lem=..." -> "lem=?", anything else unknown -> <unk> (2)
    fn id(&self, a: &str) -> u32 {
        match self.stoi.get(a) { Some(&i) => i, None => if a.starts_with("lem=") { self.lem_unk } else { 2 } }
    }
    /// atom ids of one reading key, truncated to MAXF (featmodel.encode / ebm_model.enc_reads)
    fn key_ids(&self, key: &str) -> Vec<u32> {       // no cache: Model is shared read-only across threads
        key.split('|').map(|a| self.id(a)).take(MAXF).collect()
    }
    fn add_row(&self, out: &mut [f32], id: u32) {
        let r = &self.emb[id as usize * self.d..(id as usize + 1) * self.d];
        for (o, x) in out.iter_mut().zip(r) { *o += x; }
    }
    /// mean over present readings of (sum of atom embeddings); readings beyond MAXR ignored
    fn readings_emb(&self, keys: &[&str], out: &mut [f32]) {
        let mut n = 0usize; let mut acc = vec![0f32; self.d];
        for k in keys.iter().take(MAXR) {
            let ids = self.key_ids(k);
            if ids.first().map_or(true, |&i| i == 0) { continue; }     // has = R[..., 0] > 0
            let mut r = vec![0f32; self.d];
            for &i in &ids { self.add_row(&mut r, i); }
            for (a, x) in acc.iter_mut().zip(&r) { *a += x; }
            n += 1;
        }
        let div = n.max(1) as f32;
        for (o, a) in out.iter_mut().zip(&acc) { *o = a / div; }
    }
}

/// featmodel.token_extras + "guessed" flag, first 4 kept
fn extras(word: &str, reads: &[&str], guessed: bool) -> Vec<String> {
    let mut ex = Vec::new();
    if reads.len() == 1 && reads[0] == "pos=PUNCT" { ex.push(format!("pc={word}")); }
    else if !reads.is_empty() && reads.iter().filter(|r| r.starts_with("pos=")).all(|r| {
        let p = r.split('|').next().unwrap(); CLOSED.contains(&&p[4..]) }) { ex.push(format!("cw={}", word.to_lowercase())); }
    if guessed { ex.push("guessed".into()); }
    ex.truncate(4); ex
}

/// y (t x dout) = x (t x din) . W^T + b, W stored dout x din (PyTorch Linear); sgemm from matrixmultiply
/// (a hand-written row-blocked kernel was slower on this CPU: 16-18 ms vs 11-12 ms per sentence).
fn linear(x: &[f32], t: usize, din: usize, w: &[f32], b: &[f32], dout: usize) -> Vec<f32> {
    let mut y = Vec::with_capacity(t * dout);
    for _ in 0..t { y.extend_from_slice(b); }
    unsafe {
        matrixmultiply::sgemm(t, din, dout, 1.0,
            x.as_ptr(), din as isize, 1,               // x: row stride din, col stride 1
            w.as_ptr(), 1, din as isize,               // W^T: element (k, o) = w[o * din + k]
            1.0, y.as_mut_ptr(), dout as isize, 1);
    }
    y
}

fn layer_norm(x: &[f32], t: usize, d: usize, w: &[f32], b: &[f32]) -> Vec<f32> {
    let mut y = vec![0f32; t * d];
    for i in 0..t {
        let r = &x[i * d..(i + 1) * d];
        let mean = r.iter().sum::<f32>() / d as f32;
        let var = r.iter().map(|v| (v - mean) * (v - mean)).sum::<f32>() / d as f32;
        let inv = 1.0 / (var + 1e-5).sqrt();
        for j in 0..d { y[i * d + j] = (r[j] - mean) * inv * w[j] + b[j]; }
    }
    y
}

impl Model {
    /// encoder output h (T x d) for one sentence record (feats, words)
    pub fn hidden(&self, feats: &[Vec<&str>], words: &[&str]) -> (Vec<f32>, usize) {
        let d = self.d; let t = words.len().min(MAXT);
        let mut h = vec![0f32; t * d];
        for i in 0..t {
            let fs = &feats[i];
            let reads: Vec<&str> = fs.iter().copied().filter(|x| *x != "guessed").take(MAXR).collect();
            let row = &mut h[i * d..(i + 1) * d];
            self.readings_emb(&reads, row);
            for e in extras(words[i], &reads, fs.contains(&"guessed")) { let id = self.id(&e); self.add_row(row, id); }
            for (o, p) in row.iter_mut().zip(&self.pos[i * d..(i + 1) * d]) { *o += p; }
        }
        let hd = d / self.heads; let scale = 1.0 / (hd as f32).sqrt();
        for l in &self.layers {
            let x = layer_norm(&h, t, d, &l.n1_w, &l.n1_b);
            let qkv = linear_w(&x, t, d, &l.in_w, &l.in_b, 3 * d);
            let mut att = vec![0f32; t * d];
            let mut sc = vec![0f32; t];
            for hh in 0..self.heads {
                for i in 0..t {
                    let q = &qkv[i * 3 * d + hh * hd..i * 3 * d + (hh + 1) * hd];
                    let mut mx = f32::NEG_INFINITY;
                    for j in 0..t {
                        let k = &qkv[j * 3 * d + d + hh * hd..j * 3 * d + d + (hh + 1) * hd];
                        let s: f32 = q.iter().zip(k).map(|(a, b)| a * b).sum::<f32>() * scale; sc[j] = s; if s > mx { mx = s; }
                    }
                    let mut z = 0f32; for j in 0..t { sc[j] = (sc[j] - mx).exp(); z += sc[j]; }
                    let o = &mut att[i * d + hh * hd..i * d + (hh + 1) * hd];
                    for j in 0..t {
                        let p = sc[j] / z; let v = &qkv[j * 3 * d + 2 * d + hh * hd..j * 3 * d + 2 * d + (hh + 1) * hd];
                        for (oo, vv) in o.iter_mut().zip(v) { *oo += p * vv; }
                    }
                }
            }
            let a = linear_w(&att, t, d, &l.out_w, &l.out_b, d);
            for (hv, av) in h.iter_mut().zip(&a) { *hv += av; }
            let x2 = layer_norm(&h, t, d, &l.n2_w, &l.n2_b);
            let mut f1 = linear_w(&x2, t, d, &l.l1_w, &l.l1_b, self.ff);
            for v in f1.iter_mut() { if *v < 0.0 { *v = 0.0; } }
            let f2 = linear_w(&f1, t, self.ff, &l.l2_w, &l.l2_b, d);
            for (hv, fv) in h.iter_mut().zip(&f2) { *hv += fv; }
        }
        (layer_norm(&h, t, d, &self.norm_w, &self.norm_b), t)
    }

    /// per-word error scores of one ebm JSONL record (ebm_model.token_scores, no corruption)
    pub fn scores(&self, rec: &Value) -> Vec<f32> {
        let words: Vec<&str> = rec["words"].as_array().unwrap().iter().map(|w| w.as_str().unwrap()).collect();
        let feats: Vec<Vec<&str>> = rec["feats"].as_array().unwrap().iter()
            .map(|fs| fs.as_array().unwrap().iter().map(|x| x.as_str().unwrap()).collect()).collect();
        let empty = Vec::new();
        let cands: Vec<CandRef> = rec.get("cands").and_then(|c| c.as_array()).unwrap_or(&empty).iter().map(|c| {
            let c = c.as_array().unwrap();
            (c[0].as_u64().unwrap() as usize, c[1].as_u64().unwrap() as usize,
             c[2].as_array().unwrap().iter().map(|x| { let x = x.as_array().unwrap();
                (x[0].as_str().unwrap(), x[1].as_array().unwrap().iter().map(|k| k.as_str().unwrap()).collect()) }).collect())
        }).collect();
        self.score_parts(&feats, &words, &cands).into_iter().map(|(s, _)| s).collect()
    }

    /// scores from parsed parts: per word (score, index of the lowest-energy candidate when the word has a set).
    /// Same rules as ebm_model.encode: words beyond MAXT ignored, first MAXK candidates, a set whose gold index is
    /// >= MAXK skipped; score = 1 - softmax(-E)[shown form = candidate 0].
    pub fn score_parts(&self, feats: &[Vec<&str>], words: &[&str], cands: &[CandRef]) -> Vec<(f32, Option<usize>)> {
        let t = words.len().min(MAXT); let mut out = vec![(0f32, None); t];
        let todo: Vec<&CandRef> = cands.iter().filter(|(i, g, cs)| *i < t && *g < cs.len().min(MAXK)).collect();
        if todo.is_empty() { return out; }
        let (h, _) = self.hidden(feats, words);
        let d = self.d; let sq = (d as f32).sqrt();
        for (i, _, cs) in todo {
            let i = *i;
            let hp = linear_w(&h[i * d..(i + 1) * d], 1, d, &self.proj_w, &self.proj_b, d);
            let mut e: Vec<f32> = Vec::with_capacity(cs.len().min(MAXK));
            for (_, keys) in cs.iter().take(MAXK) {
                let mut ce = vec![0f32; d]; self.readings_emb(keys, &mut ce);
                e.push(-ce.iter().zip(&hp).map(|(a, b)| a * b).sum::<f32>() / sq);
            }
            let m = e.iter().map(|x| -x).fold(f32::NEG_INFINITY, f32::max);
            let z: f32 = e.iter().map(|x| (-x - m).exp()).sum();
            let best = e.iter().enumerate().min_by(|a, b| a.1.partial_cmp(b.1).unwrap()).map(|(k, _)| k);
            out[i] = (1.0 - (-e[0] - m).exp() / z, best);
        }
        out
    }
}

/// one candidate set as borrowed strings: (word index, gold index, [(form, reading keys)])
pub type CandRef<'a> = (usize, usize, Vec<(&'a str, Vec<&'a str>)>);

pub fn open_lines(path: &str) -> Box<dyn std::io::BufRead> {
    let f = std::fs::File::open(path).expect("input");
    if path.ends_with(".gz") { Box::new(std::io::BufReader::new(flate2::read::GzDecoder::new(f))) } else { Box::new(std::io::BufReader::new(f)) }
}
