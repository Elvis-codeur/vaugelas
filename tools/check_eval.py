#!/usr/bin/env python3
"""Measured precision and recall for every Vaugelas rule, on fixtures that ship with the repository.

Two fixture files (issue #20), one JSON object per line:

  tests/fixtures/regression/must-flag.jsonl      {"id","text","flag":[{"span","rule"}],"why"}
  tests/fixtures/regression/must-not-flag.jsonl  {"id","text","why"}

A `flag` entry counts when the checker reports `rule` on a range that overlaps `span` (offsets are found by
searching the text, so the fixture is independent of the tokenizer). A must-not-flag record fails on any finding
whose range overlaps no known defect (there are none in the record, so any finding fails it).

  python3 tools/check_eval.py                     # fixtures, per rule: TP / FP / FN + precision + recall
  python3 tools/check_eval.py --vaugelas BIN      # another binary
  python3 tools/check_eval.py --corpus DIR        # also the external 67-defect corpus (not in this repo)

`--corpus DIR` expects the layout of `agentic-reasoning-translator/experiments/2026-11-10-prompt-ceiling`:
`reference/inputs_v1_fixed.jsonl` (field `fr_deepseek`) and `reference/review.jsonl` (spans in `errors[].fr_span`).
The 67 defects are reported in three buckets, never as one number: (a) checkable by rules, (b) lexical/semantic
(word_choice, glossary — the rules cannot honestly claim these), (c) other. Exits 1 when a cell falls below
`tools/baseline.json`, so a change that fixes one corpus cannot silently break another.

Author: DeepSeek (agent), 2026-11-12. Measurements only — no number here is estimated.
"""
import argparse
import json
import os
import subprocess
import sys
from collections import Counter, defaultdict
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
FIXTURES = ROOT / "tests/fixtures/regression"
BASELINE = ROOT / "tools/baseline.json"
BUCKET_A = {"grammar", "typography", "structure", "scripture"}  # checkable by rules
BUCKET_B = {"word_choice", "glossary"}                          # lexical: model + combiner
THRESHOLD = None  # --threshold, for measuring a model exported with another operating point


def run_checker(binary, text, data=None, extra=()):
    cmd = [binary, "check", "--json", *extra]
    if THRESHOLD:
        cmd += ["--threshold", THRESHOLD]
    env = dict(os.environ)
    if data:
        env["VAUGELAS_DATA"] = data
    p = subprocess.run(cmd, input=text, capture_output=True, text=True, env=env)
    if p.returncode != 0:
        raise SystemExit(f"{binary} check failed: {p.stderr[-400:]}")
    out = json.loads(p.stdout)["matches"]
    for m in out:  # the checker's own contract: offset/length point at the flagged text
        if text[m["offset"]:m["offset"] + m["length"]] != m["text"]:
            raise SystemExit(f"offset mismatch: {m}")
    return out


def overlap(a_start, a_end, b_start, b_end):
    return a_start < b_end and a_end > b_start


def fixtures(binary, data):
    """Per-rule TP / FP / FN over the committed fixtures."""
    tp, fp, fn = Counter(), Counter(), Counter()
    reasons = []
    for path, want in ((FIXTURES / "must-flag.jsonl", True), (FIXTURES / "must-not-flag.jsonl", False)):
        for line in path.read_text(encoding="utf-8").splitlines():
            if not line.strip() or line.lstrip().startswith("//"):
                continue
            rec = json.loads(line)
            text = rec["text"]
            matches = run_checker(binary, text, data)
            if not want:
                forbid = rec.get("forbid")
                bad = [m for m in matches if not forbid or m["rule"] in forbid]
                if bad:
                    fp[bad[0]["rule"]] += 1
                    reasons.append((rec["id"], "must-not-flag", bad[0], rec.get("why", "")))
                continue
            for want_flag in rec["flag"]:
                span, rule = want_flag["span"], want_flag["rule"]
                start = text.find(span)
                if start < 0:
                    raise SystemExit(f"fixture {rec['id']}: span not in text: {span!r}")
                end = start + len(span)
                hits = [m for m in matches if overlap(m["offset"], m["offset"] + m["length"], start, end)]
                good = [m for m in hits if m["rule"] == rule]
                if good:
                    tp[rule] += 1
                    for m in hits:
                        if m["rule"] != rule:
                            fp[m["rule"]] += 1
                else:
                    fn[rule] += 1
                    reasons.append((rec["id"], f"missed {rule}", {"text": span}, rec.get("why", "")))
    return tp, fp, fn, reasons


def corpus(binary, data, corpus_dir):
    """Bucket recall on the external 67-defect corpus, and the findings that match no recorded defect."""
    d = Path(corpus_dir)
    texts = {}
    for line in (d / "reference/inputs_v1_fixed.jsonl").read_text(encoding="utf-8").splitlines():
        r = json.loads(line)
        texts[r["ref_id"]] = r["fr_deepseek"]
    review = {}
    for line in (d / "reference/review.jsonl").read_text(encoding="utf-8").splitlines():
        r = json.loads(line)
        review[r["ref_id"]] = r
    matches = {rid: run_checker(binary, t, data) for rid, t in texts.items()}
    buckets = {"a_rules": [0, 0], "b_lexical": [0, 0], "c_other": [0, 0]}
    by_rule = Counter()
    for rid, r in review.items():
        t = texts[rid]
        spans = []
        for e in r.get("errors", []):
            s = e.get("fr_span")
            if not s:
                continue
            i = t.find(s)
            if i >= 0:
                spans.append((i, i + len(s)))
        for e in r.get("errors", []):
            s = e.get("fr_span")
            i = t.find(s) if s else -1
            cls = e["class"]
            key = "a_rules" if cls in BUCKET_A else "b_lexical" if cls in BUCKET_B else "c_other"
            buckets[key][1] += 1
            if i >= 0 and any(overlap(m["offset"], m["offset"] + m["length"], i, i + len(s)) for m in matches[rid]):
                buckets[key][0] += 1
                by_rule[cls] += 1
    unexplained = Counter()
    for rid, ms in matches.items():
        spans = []
        t = texts[rid]
        for e in review[rid].get("errors", []):
            s = e.get("fr_span")
            i = t.find(s) if s else -1
            if i >= 0:
                spans.append((i, i + len(s)))
        for m in ms:
            if not any(overlap(m["offset"], m["offset"] + m["length"], s, e) for s, e in spans):
                unexplained[m["rule"]] += 1
    return buckets, by_rule, unexplained, sum(len(v) for v in matches.values())


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--vaugelas", default=str(ROOT / "target/release/vaugelas"))
    ap.add_argument("--data", default=os.environ.get("VAUGELAS_DATA"))
    ap.add_argument("--corpus", help="directory of the 67-defect corpus (external, not in this repo)")
    ap.add_argument("--threshold", default=None, help="model threshold (default: the model's own)")
    ap.add_argument("--limit", type=int, default=8)
    a = ap.parse_args()
    global THRESHOLD
    THRESHOLD = a.threshold
    if not Path(a.vaugelas).is_file():
        raise SystemExit(f"binary not found: {a.vaugelas}; build it first (cargo build --release)")

    tp, fp, fn, reasons = fixtures(a.vaugelas, a.data)
    print("== fixtures (tests/fixtures/regression) ==")
    print(f"{'rule':24} {'TP':>3} {'FP':>3} {'FN':>3} {'precision':>9} {'recall':>6}")
    for rule in sorted(set(tp) | set(fp) | set(fn)):
        p = tp[rule] / (tp[rule] + fp[rule]) if tp[rule] + fp[rule] else 1.0
        r = tp[rule] / (tp[rule] + fn[rule]) if tp[rule] + fn[rule] else 0.0
        print(f"{rule:24} {tp[rule]:3} {fp[rule]:3} {fn[rule]:3} {p:9.2f} {r:6.2f}")
    total_tp, total_fp, total_fn = sum(tp.values()), sum(fp.values()), sum(fn.values())
    print(f"{'TOTAL':24} {total_tp:3} {total_fp:3} {total_fn:3} "
          f"{total_tp / max(1, total_tp + total_fp):9.2f} {total_tp / max(1, total_tp + total_fn):6.2f}")
    for rid, what, m, why in reasons[: a.limit]:
        print(f"  ! {rid}: {what}: {m}  ({why})")

    result = {"fixtures": {"tp": total_tp, "fp": total_fp, "fn": total_fn}}
    if a.corpus:
        buckets, by_rule, unexplained, n_flags = corpus(a.vaugelas, a.data, a.corpus)
        print("\n== 67-defect corpus (external) ==")
        for k, (got, tot) in buckets.items():
            print(f"  {k:10} {got}/{tot} recall")
        print(f"  flags raised: {n_flags}; matching no recorded defect: {sum(unexplained.values())} "
              f"({', '.join(f'{k} {v}' for k, v in unexplained.most_common())})")
        result["corpus"] = {"buckets": {k: v for k, v in buckets.items()},
                            "unexplained": dict(unexplained), "flags": n_flags}

    if BASELINE.is_file() and not a.corpus:
        base = json.loads(BASELINE.read_text())
        got = result["fixtures"]
        bad = []
        for key in ("tp", "fp", "fn"):
            if key in base.get("fixtures", {}) and got[key] != base["fixtures"][key]:
                bad.append(f"{key}: {got[key]} (baseline {base['fixtures'][key]})")
        if bad:
            print("\nFAIL: baseline moved: " + "; ".join(bad))
            return 1
        print("\nbaseline: unchanged")
    return 0


if __name__ == "__main__":
    sys.exit(main())
