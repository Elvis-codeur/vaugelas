//! CPU inference of the paradigm-energy model (training/ebm_model.py `EBM`, encoder = training/featmodel.py `Net`).
//! Weights come from training/export_ebm.py (`model.frcm`). The math mirrors PyTorch eval mode exactly:
//! reading embedding = sum of its atom embeddings; word input = mean over present readings + sum of extra-atom
//! embeddings; + learned position; nn.TransformerEncoder (norm_first, ReLU FFN, no dropout); final LayerNorm;
//! energy of candidate c at word i = -(cand_emb(c) . proj(h_i)) / sqrt(d); score = 1 - softmax(-E)[shown form].
//!
//! Ported from fr-check-infer (frc-infer, commit cc7d94e).
//!
//! The input encoding (atoms per reading, extra slots, candidates per set) comes from the model header (`enc`), see
//! `featmodel`. The records come from the v14 data path (`record`, `cands`, `resources`), so only v14-encoded models
//! are accepted; the inference settings that belong to a model (fr-check lints it reads, threshold) are in `profile`.
pub mod cands;
pub mod drops;
pub mod ebm_data;
pub mod featmodel;
pub mod pipeline;
pub mod profile;
pub mod record;
pub mod resources;
use featmodel::{encode, Enc, Encoded, Vocab};
use profile::Profile;
use serde_json::Value;
use std::io::Read;

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

thread_local! { static SCRATCH: std::cell::RefCell<Vec<f32>> = const { std::cell::RefCell::new(Vec::new()) }; }

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
    pub vocab: Vocab,
    /// the input encoding the model was trained with
    pub enc: Enc,
    /// inference settings that belong to this model (lint inputs, default threshold)
    pub profile: Profile,
    emb: Vec<f32>, pos: Vec<f32>, layers: Vec<Layer>, norm_w: Vec<f32>, norm_b: Vec<f32>,
    proj_w: Wt, proj_b: Vec<f32>,
}

fn err<T>(m: &str) -> Result<T, String> { Err(m.to_string()) }

/// The JSON header of a model.frcm file, without reading the weights.
pub fn read_header(path: &str) -> Result<Value, String> {
    let mut f = std::fs::File::open(path).map_err(|e| format!("{path}: {e}"))?;
    let mut magic = [0u8; 14];
    f.read_exact(&mut magic).map_err(|_| format!("{path}: not a model.frcm file"))?;
    if &magic[..6] != b"FRCM1\n" { return Err(format!("{path}: not a model.frcm file")); }
    let hl = u64::from_le_bytes(magic[6..14].try_into().map_err(|_| "truncated header")?) as usize;
    let mut head = Vec::new();
    f.take(hl as u64).read_to_end(&mut head).map_err(|e| format!("{path}: {e}"))?;
    if head.len() < hl { return Err(format!("{path}: truncated model.frcm header")); }
    serde_json::from_slice(&head).map_err(|e| format!("{path}: {e}"))
}

impl Model {
    pub fn load(path: &str) -> Result<Model, String> { Self::load_opts(path, false) }

    /// int8 = true: every Linear of the encoder and the energy projection stored as int8 + per-row scales
    pub fn load_opts(path: &str, int8: bool) -> Result<Model, String> {
        let mut f = std::fs::File::open(path).map_err(|e| format!("{path}: {e}"))?;
        let mut buf = Vec::new(); f.read_to_end(&mut buf).map_err(|e| format!("{path}: {e}"))?;
        if buf.len() < 14 || &buf[..6] != b"FRCM1\n" { return err("not a model.frcm file"); }
        let hl = u64::from_le_bytes(buf[6..14].try_into().map_err(|_| "truncated header")?) as usize;
        if buf.len() < 14 + hl { return err("truncated model.frcm header"); }
        let head: Value = serde_json::from_slice(&buf[14..14 + hl]).map_err(|e| e.to_string())?;
        let data = &buf[14 + hl..];
        let tens = head["tensors"].as_object().ok_or("no tensors")?;
        let shape = |name: &str| -> Result<Vec<usize>, String> {
            let t = tens.get(name).ok_or(format!("missing tensor {name}"))?;
            t["shape"].as_array().ok_or(format!("{name}: no shape"))?.iter().map(|x| x.as_u64().map(|v| v as usize).ok_or(format!("{name}: bad shape"))).collect()
        };
        let get = |name: &str| -> Result<Vec<f32>, String> {
            let n: usize = shape(name)?.iter().product();
            let off = tens[name]["offset"].as_u64().ok_or(format!("{name}: no offset"))? as usize;
            let b = data.get(off..off + 4 * n).ok_or(format!("{name}: data out of range"))?;
            Ok(b.as_chunks::<4>().0.iter().map(|c| f32::from_le_bytes(*c)).collect())
        };
        let cfg = &head["cfg"];
        let num = |k: &str| cfg.get(k).and_then(Value::as_u64).map(|v| v as usize).ok_or(format!("model cfg: {k} missing"));
        let d = num("d")?; let nl = num("layers")?;
        let mut heads = cfg.get("heads").and_then(|h| h.as_u64()).unwrap_or(4) as usize;
        if d % heads != 0 { heads = 4; }
        let itos: Vec<String> = head["vocab"].as_array().ok_or("no vocab")?.iter().map(|s| s.as_str().map(String::from).ok_or("bad vocab entry")).collect::<Result<_, _>>()?;
        let enc = Enc::from_header(&head)?;
        if !enc.v14 { return err("this model has no v14 encoding in its header (a round-3 model such as ebm_ds?): Vaugelas needs a v14 model"); }
        let profile = Profile::from_header(&head)?;
        let mut layers = Vec::new();
        for l in 0..nl {
            let p = |s: &str| format!("net.body.layers.{l}.{s}");
            let ff = shape(&p("linear1.bias"))?[0];
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
        let l1b = layers.first().map_or(2 * d, |l| l.l1_b.len());
        let emb = get("net.emb.weight")?;
        if emb[..d].iter().any(|x| *x != 0.0) { return err("embedding row 0 (padding) is not zero"); }
        Ok(Model { d, heads, ff: l1b, vocab: Vocab::new(itos), enc, profile,
            emb, pos: get("net.pos.weight")?, layers,
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

    fn add_row(&self, out: &mut [f32], id: u32) {
        let r = &self.emb[id as usize * self.d..(id as usize + 1) * self.d];
        for (o, x) in out.iter_mut().zip(r) { *o += x; }
    }
    /// Net.embed / EBM.cand_emb for one word: mean over present readings (first atom id > 0) of the sum of their
    /// atom embeddings (padding slots add row 0, which is zero: checked at load)
    fn readings_emb(&self, reads: &[Vec<u32>], out: &mut [f32]) {
        let mut n = 0usize; let mut acc = vec![0f32; self.d];
        for ids in reads {
            if ids.first().is_none_or(|&i| i == 0) { continue; }
            let mut r = vec![0f32; self.d];
            for &i in ids { self.add_row(&mut r, i); }
            for (a, x) in acc.iter_mut().zip(&r) { *a += x; }
            n += 1;
        }
        let div = n.max(1) as f32;
        for (o, a) in out.iter_mut().zip(&acc) { *o = a / div; }
    }
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
    /// encoder output h (T x d) for one encoded sentence (EBM.hidden)
    pub fn hidden(&self, x: &Encoded) -> (Vec<f32>, usize) {
        let d = self.d; let t = x.t;
        let mut h = vec![0f32; t * d];
        for i in 0..t {
            let row = &mut h[i * d..(i + 1) * d];
            self.readings_emb(&x.reads[i], row);
            for &id in &x.extras[i] { self.add_row(row, id); }
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
                    let mut z = 0f32; for x in sc.iter_mut() { *x = (*x - mx).exp(); z += *x; }
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

    /// scores from parsed parts, encoded with the model's own encoding (see `featmodel::encode`)
    pub fn score_parts<S: AsRef<str>>(&self, feats: &[Vec<S>], words: &[S], cands: &[featmodel::SetRef<S>]) -> Vec<(f64, Option<usize>)> {
        self.score(&encode(&self.enc, &self.vocab, words, feats, cands))
    }

    /// ebm_model.token_scores for one encoded sentence: per word (score, index of the lowest-energy candidate when
    /// the word has a set). score = 1 - softmax(-E)[candidate 0 = the shown form], computed like Python's
    /// `1.0 - row[0].item()` (fp32 probability, the subtraction in double); words without a set score 0.
    pub fn score(&self, x: &Encoded) -> Vec<(f64, Option<usize>)> {
        let mut out = vec![(0f64, None); x.t];
        for (i, e, best) in self.set_energies(x) {
            let m = e.iter().map(|x| -x).fold(f32::NEG_INFINITY, f32::max);
            let z: f32 = e.iter().map(|x| (-x - m).exp()).sum();
            let p0 = (-e[0] - m).exp() / z;
            out[i] = (1.0 - p0 as f64, best);
        }
        out
    }

    /// The raw energies of every candidate set (lower is better), one entry per word that has a set. Used by the
    /// data-preparation experiment for the margin `E(written) - softmin(E(alternatives))`; `score` is the same
    /// computation folded into `1 - p(candidate 0)`.
    pub fn set_energies(&self, x: &Encoded) -> Vec<(usize, Vec<f32>, Option<usize>)> {
        if x.sets.is_empty() { return Vec::new(); }
        let (h, _) = self.hidden(x);
        let d = self.d; let sq = (d as f32).sqrt();
        let mut out = Vec::with_capacity(x.sets.len());
        for (i, cs) in &x.sets {
            let i = *i;
            let hp = linear_w(&h[i * d..(i + 1) * d], 1, d, &self.proj_w, &self.proj_b, d);
            let mut e: Vec<f32> = Vec::with_capacity(cs.len());
            for reads in cs {
                let mut ce = vec![0f32; d]; self.readings_emb(reads, &mut ce);
                e.push(-ce.iter().zip(&hp).map(|(a, b)| a * b).sum::<f32>() / sq);
            }
            let best = e.iter().enumerate().min_by(|a, b| a.1.total_cmp(b.1)).map(|(k, _)| k);
            out.push((i, e, best));
        }
        out
    }

    /// The number of candidates the encoder keeps per set (`cfg.maxk`). Raising it lets the same weights score a
    /// widened candidate set (the transformer does not depend on it; only the per-set padding does).
    pub fn set_maxk(&mut self, maxk: usize) { self.enc.maxk = maxk; }
}
