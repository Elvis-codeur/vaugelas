#!/bin/bash
# Model parity on the held-out sets: new crate vs the reference scores (frc-infer golden = Python + PyTorch verified).
# usage: tools/parity.sh WORK_ROOT DATA_DIR    (WORK_ROOT = language-correction-works, holding hardloop/, datasets/, build/)
set -e
L="$1"; export VAUGELAS_DATA="$2"
for f in final2_pairs:final-2 tune_pairs:tune; do IFS=: read -r set dir <<< "$f"
  echo "== $set"
  VAUGELAS_PARITY_ITEMS="$L/hardloop/work/$dir/items.jsonl" VAUGELAS_PARITY_REF="$L/build/infer-work/e2e_ds_$set.jsonl" \
  VAUGELAS_PARITY_PY="$L/datasets/ebm6/$set.jsonl.gz" \
    cargo test --release --locked -p vaugelas --test parity_model -- --ignored --nocapture 2>&1 | grep -E '^\{|test result|differs|panicked'
done
