# Developer entry points (the crate build is plain cargo; these are the measured checks).
BIN ?= target/release/vaugelas

.PHONY: build test check-eval eval-corpus sample
build:
	cargo build --release

test:
	cargo test --release

# Per-rule precision and recall on the committed fixtures; fails when the committed baseline moves.
check-eval: $(BIN)
	python3 tools/check_eval.py --vaugelas $(BIN)

# The 67-defect corpus is not in this repository: pass its directory.
#   make eval-corpus CORPUS=../agentic-reasoning-translator/experiments/2026-11-10-prompt-ceiling
CORPUS ?=
eval-corpus: $(BIN)
	@test -n "$(CORPUS)" || { echo "set CORPUS=<dir of the 67-defect corpus>"; exit 2; }
	python3 tools/check_eval.py --vaugelas $(BIN) --corpus "$(CORPUS)"

# Regenerate the rule-parity fixture (deterministic) and its expected output.
sample:
	python3 tools/make_sample.py
	cargo run --release --example gen_parity
