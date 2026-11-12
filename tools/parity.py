#!/usr/bin/env python3
"""Parity harness for the Rust data-preparation port (`vaugelas-dataprep`) against the archived Python generators.

What it checks
--------------
1. `lexpairs` — three datasets: the archived generator at 40 and 60 per sub-type, and the pre-12:55 revision
   (8 contexts) that produced the committed `data/lexpairs.jsonl`. Bytes and sha256 of `lexpairs.jsonl` and
   `CARD.md` against goldens produced by Python.
2. `product-pairs` — the 1,989-item batch 1, against its Python golden. Needs the carrier corpus
   (`--carriers`, or `PARITY_CARRIERS`), which is deliberately not in the repository.
3. `pairs` — the record builder: 60 items through `vaugelas-dataprep pairs`, compared with the Python
   `ebm_data2.py pairs` reference on the same v14 flag set. Three outcomes are allowed and counted separately:
   identical; `frc=FR_INVENTED` only (the Rust checker's PR #21 rule, absent from the Nov-8 `frc` binary the
   reference used); `|capr=` only (older `lm_atoms` after sentence-opening punctuation). Anything else fails.
   Needs the model data (`--data`, or `VAUGELAS_DATA`).

Live Python
-----------
`--python-dir DIR` additionally runs the archived scripts (`lexpairs.py`, `product_pairs.py`) and compares their
fresh output with Rust's, so the equivalence can be re-derived rather than only trusted from the goldens. The
`product_pairs.py` import of numpy is unused upstream; when numpy is missing the harness supplies an empty shim on
PYTHONPATH (documented, not silent).

Usage
-----
  python3 tools/parity.py [--binary target/release/vaugelas-dataprep]
                          [--carriers FILE] [--data DIR] [--python-dir DIR] [--out PARITY.md]

Exit status 0 when every check ran and passed; 2 when a check failed; 0 with SKIP lines for checks whose input
is not available (the table says so explicitly, nothing is reported as passing that did not run).
"""
import argparse
import gzip
import hashlib
import json
import os
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
GOLD = ROOT / "crates" / "vaugelas-dataprep" / "tests" / "parity"
ALGORITHM = "pyrandom-mt19937-v1"

rows = []
failures = []


def sha(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def read_maybe_gz(p: Path) -> bytes:
    data = p.read_bytes()
    return gzip.decompress(data) if p.suffix == ".gz" else data


def add(name, python_sha, rust_sha, status, note=""):
    rows.append({"name": name, "python": python_sha, "rust": rust_sha, "status": status, "note": note})
    if status.startswith("FAIL"):
        failures.append(name)


def run(cmd, **kw):
    t0 = time.time()
    p = subprocess.run([str(c) for c in cmd], capture_output=True, text=True, **kw)
    dt = time.time() - t0
    if p.returncode != 0:
        sys.stderr.write(p.stderr)
        raise SystemExit(f"command failed ({p.returncode}): {' '.join(str(c) for c in cmd)}")
    return dt, p.stderr


def compare_files(name, rust_path, golden: Path, expected_sha: str):
    rust = rust_path.read_bytes()
    want = read_maybe_gz(golden)
    if rust != want:
        add(name, sha(want), sha(rust), "FAIL", "bytes differ")
        return False
    if sha(rust) != expected_sha:
        add(name, expected_sha, sha(rust), "FAIL", "golden changed")
        return False
    add(name, expected_sha, sha(rust), "identical")
    return True


def lexpairs(binary, work, pydir=None):
    cases = [
        ("lexpairs 1,080 (40/sub-type, 16 contexts)", ["--per-subtype", "40"], "lexpairs-1080.jsonl.gz",
         "ae10a3859f167fc65510451f54b7d69966d7f83779ce14e742b147a8d5db003e",
         "8565a2a64c1f1153ed360ab961b9d7196c195fa697e8caef60124750b56bdf79"),
        ("lexpairs 1,620 (60/sub-type, 16 contexts)", ["--per-subtype", "60"], "lexpairs-1620.jsonl.gz",
         "95cc3570a3635f15db1ff16d5ecc648c8b28ab5c0c97a6c465727820c7d7b974",
         "1ab7ab7512d6759030bd97f520b25bd255b1de27ce7b70a64f365c9cfc47c361"),
        ("lexpairs 1,080 (legacy: 8 contexts, data/lexpairs.jsonl)", ["--per-subtype", "40", "--contexts", "8"],
         "lexpairs-1080-legacy8.jsonl.gz",
         "f28bf5e26ff92012919f9136e46283a3b3bcd5a4617278b2fad2040b6ad865ae",
         "5ff6321e3013c913ea3a161f3cb6ecb848f1ea52a7ffecc94cdaf1714de2d6f1"),
    ]
    for name, args, golden, dsha, csha in cases:
        out = work / (golden.replace(".jsonl.gz", ""))
        dt, _ = run([binary, "lexpairs", "--out", out, *args])
        compare_files(name, out / "lexpairs.jsonl", GOLD / golden, dsha)
        card = out / "CARD.md"
        add(name + " — CARD.md", csha, sha(card.read_bytes()), "identical" if sha(card.read_bytes()) == csha else "FAIL")
        print(f"  rust {dt:.2f}s  {name}")
        if pydir:
            if "--contexts" in args:
                add(name + " — live Python", "—", "—", "SKIP", "the legacy 8-context revision is git 3693633, not the archive")
            else:
                py = work / (golden.replace(".jsonl.gz", "") + "-py")
                per = args[args.index("--per-subtype") + 1]
                cmd = [sys.executable, pydir / "lexpairs.py", "--out", py, "--per-subtype", per]
                dpy, _ = run(cmd)
                same = (out / "lexpairs.jsonl").read_bytes() == (py / "lexpairs.jsonl").read_bytes()
                add(name + " — live Python", sha((py / "lexpairs.jsonl").read_bytes()), sha((out / "lexpairs.jsonl").read_bytes()),
                    "identical" if same else "FAIL", f"python {dpy:.2f}s")


def product(binary, work, carriers, pydir=None):
    if not carriers:
        add("product-pairs 1,989", "f4a8b93c5c5a00f062e7fab4f73eba3ac84e17739d85b238fc15292657b8276c", "—", "SKIP",
            "set PARITY_CARRIERS (corpus not in the repo)")
        return
    out = work / "product"
    dt, _ = run([binary, "product-pairs", "--out", out, "--src", carriers, "--n", "1989"])
    compare_files("product-pairs 1,989", out / "items.jsonl", GOLD / "product-items-1989.jsonl.gz",
                  "f4a8b93c5c5a00f062e7fab4f73eba3ac84e17739d85b238fc15292657b8276c")
    print(f"  rust {dt:.2f}s  product-pairs")
    if pydir:
        py = work / "product-py"
        env = dict(os.environ)
        shim = work / "numpy-shim"
        shim.mkdir(exist_ok=True)
        (shim / "numpy.py").write_text("")  # upstream imports numpy without using it
        env["PYTHONPATH"] = str(shim) + (os.pathsep + env["PYTHONPATH"] if env.get("PYTHONPATH") else "")
        dpy, _ = run([sys.executable, pydir / "product_pairs.py", "--out", py, "--n", "1989", "--src", carriers], env=env)
        same = (out / "items.jsonl").read_bytes() == (py / "items.jsonl").read_bytes()
        add("product-pairs 1,989 — live Python", sha((py / "items.jsonl").read_bytes()), sha((out / "items.jsonl").read_bytes()),
            "identical" if same else "FAIL", f"python {dpy:.2f}s")
        if not same:
            failures.append("product-pairs 1,989 — live Python")


def _strip_capr(rec):
    rec = json.loads(json.dumps(rec))
    for c in rec["cands"]:
        for cand in c[2]:
            cand[1] = [_remove_capr(r) for r in cand[1]]
    return rec


def _remove_capr(s):
    while "|capr=" in s:
        i = s.index("|capr=")
        j = i + 6
        while j < len(s) and s[j].isdigit():
            j += 1
        s = s[:i] + s[j:]
    return s


def _strip_invented(rec):
    return {**rec, "feats": [[a for a in f if a != "frc=FR_INVENTED"] for f in rec["feats"]]}


def pairs(binary, work, data):
    if not data:
        add("pairs (60 items, 119 records)", "7f33d164ed914ea2f310fef4325f016ee25a2749e4349f47821303b901dfdb00", "—", "SKIP",
            "set VAUGELAS_DATA (model data)")
        return
    out = work / "pairs60.jsonl"
    dt, _ = run([binary, "pairs", "--items", GOLD / "pairs-items60.jsonl", "--out", out, "--data", data])
    rust = [json.loads(l) for l in out.read_text(encoding="utf-8").splitlines()]
    py = [json.loads(l) for l in gzip.decompress((GOLD / "pairs-reference60.jsonl.gz").read_bytes()).decode().splitlines()]
    same = invented = capr = 0
    other = []
    for k, (a, b) in enumerate(zip(rust, py)):
        if a == b:
            same += 1
        elif _strip_invented(a) == _strip_invented(b):
            invented += 1
        elif _strip_capr(a) == _strip_capr(b):
            capr += 1
        else:
            other.append(k)
    ref_sha = "7f33d164ed914ea2f310fef4325f016ee25a2749e4349f47821303b901dfdb00"
    rust_sha = sha(out.read_bytes())
    note = f"{same} identical, {invented} FR_INVENTED-only, {capr} capr-only, {len(other)} other"
    if other or len(rust) != len(py):
        add("pairs (60 items, 119 records)", ref_sha, rust_sha, "FAIL", note + f" at {other[:5]}")
    else:
        add("pairs (60 items, 119 records)", ref_sha, rust_sha, "equivalent", note)
    print(f"  rust {dt:.2f}s  pairs ({note})")


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--binary", default=str(ROOT / "target/release/vaugelas-dataprep"))
    ap.add_argument("--carriers", default=os.environ.get("PARITY_CARRIERS"))
    ap.add_argument("--data", default=os.environ.get("VAUGELAS_DATA"))
    ap.add_argument("--python-dir", default=None, help="the archived Python directory (archive/python-2026-11-12)")
    ap.add_argument("--out", default=None, help="write the markdown table here")
    a = ap.parse_args()
    binary = Path(a.binary)
    if not binary.exists():
        raise SystemExit(f"missing {binary}: run `cargo build --release -p vaugelas-dataprep` first")
    pydir = Path(a.python_dir) if a.python_dir else None
    if pydir and not (pydir / "lexpairs.py").exists():
        raise SystemExit(f"--python-dir {pydir} does not hold lexpairs.py")

    print(f"parity: {binary}  ({ALGORITHM})")
    # all produced files go under the repository (not /tmp, which is a small quota'd tmpfs here)
    work = ROOT / ".parity"
    if work.exists():
        import shutil
        shutil.rmtree(work)
    work.mkdir()
    lexpairs(binary, work, pydir)
    product(binary, work, a.carriers, pydir)
    pairs(binary, work, a.data)

    table = ["| check | Python sha256 | Rust sha256 | result | note |", "|---|---|---|---|---|"]
    for r in rows:
        table.append(f"| {r['name']} | `{r['python'][:16]}…` | `{r['rust'][:16]}…` | {r['status']} | {r['note']} |")
    table.append("")
    table.append(f"Random-stream contract: `{ALGORITHM}`; default seed 20261112. Python references produced with "
                 f"CPython 3.12.3 (`lexpairs.py`, `product_pairs.py`, `ebm_data2.py`).")
    md = "\n".join(table)
    print()
    print(md)
    if a.out:
        Path(a.out).write_text("# Data-prep parity\n\n" + md + "\n", encoding="utf-8")
        print(f"\nwritten to {a.out}")
    if failures:
        print(f"\nFAILED: {len(failures)} check(s): {', '.join(failures)}", file=sys.stderr)
        return 2
    skipped = [r["name"] for r in rows if r["status"] == "SKIP"]
    print(f"\nall run checks passed" + (f"; skipped: {', '.join(skipped)}" if skipped else ""))
    return 0


if __name__ == "__main__":
    sys.exit(main())
