#!/bin/bash
# Model parity with the research implementation on the hand-written sentences of tests/fixtures/sentences.tsv:
# regenerates tests/fixtures/sentences.frc-infer-v14.jsonl with fr-check-infer's frc-infer (the reference, itself
# checked against PyTorch), then runs the parity test, which compares every per-word score.
#
# usage: tools/parity.sh FRC_INFER DATA_DIR
#   FRC_INFER  the frc-infer binary (fr-check-infer: cargo build --release -p frc-infer)
#   DATA_DIR   an installed Vaugelas data directory (vaugelas download-model --data DATA_DIR)
# The fr-check lints the model does not read are taken from the model itself (`vaugelas info`).
set -euo pipefail
[ $# -eq 2 ] || { sed -n '2,9p' "$0"; exit 2; }
frc_infer=$1; data=$2
root=$(cd "$(dirname "$0")/.." && pwd)
fx="$root/tests/fixtures"
tmp=$(mktemp -d); trap 'rm -rf "$tmp"' EXIT
cargo build --release --locked -p vaugelas-cli
ignore=$("${CARGO_TARGET_DIR:-$root/target}/release/vaugelas" info --data "$data" | sed -n 's/^  fr-check lints not read by the model: //p')
[ -n "$ignore" ] || { echo "cannot read the model's lint list from vaugelas info" >&2; exit 1; }
# one {"id", "text"} line per sentence (the sentences hold no tab, backslash or double quote)
awk -F'\t' 'NR > 1 { printf "{\"id\": \"%d\", \"text\": \"%s\"}\n", NR - 1, $2 }' "$fx/sentences.tsv" > "$tmp/in.jsonl"
"$frc_infer" v14 "$tmp/in.jsonl" "$fx/sentences.frc-infer-v14.jsonl" --model "$data/model.frcm" --frc-ignore "$ignore" \
  --lexicon "$data/lexicon.tsv" --resources "$data/resources" --frc-lex "$data/lex"
VAUGELAS_DATA="$data" cargo test --release --locked -p vaugelas --test parity_model -- --nocapture 2>&1 | grep -E '^parity:|test result|panicked'
