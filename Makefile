# Developer entry points (the crate build is plain cargo; these are the measured checks).
BIN ?= target/release/vaugelas
DATAPREP ?= target/release/vaugelas-dataprep

.PHONY: build test check-eval eval-corpus sample dataprep parity lexpairs product-pairs pairs cands

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

# --- data preparation: Rust only (the Python generators are archived, see PARITY.md) -------------------------

dataprep:
	cargo build --release -p vaugelas-dataprep

# The parity evidence: generators byte-for-byte; record path equivalent with the two named differences.
#   PARITY_CARRIERS=<datasets/cleantext/books_dev_texts.jsonl>  (corpus, not in the repo)
#   VAUGELAS_DATA=<dir with model.frcm, lexicon.tsv, resources/, lex/>
#   PYTHON=<archive/python-2026-11-12>  (re-derive the Python side live)
parity: dataprep
	python3 tools/parity.py --binary $(DATAPREP) \
		$(if $(PARITY_CARRIERS),--carriers "$(PARITY_CARRIERS)") \
		$(if $(VAUGELAS_DATA),--data "$(VAUGELAS_DATA)") \
		$(if $(PYTHON),--python-dir "$(PYTHON)") \
		$(if $(OUT),--out "$(OUT)")

# The archived generators' entry points, now Rust.
#   make lexpairs OUT=data/lexpairs PER=40 SEED=20261112 [CONTEXTS=16]
lexpairs: dataprep
	@test -n "$(OUT)" || { echo "set OUT=<output directory>"; exit 2; }
	$(DATAPREP) lexpairs --out "$(OUT)" --per-subtype $(or $(PER),40) --seed $(or $(SEED),20261112) \
		$(if $(CONTEXTS),--contexts $(CONTEXTS))

#   make product-pairs OUT=data/product_pairs SRC=<books_dev_texts.jsonl> N=1989
product-pairs: dataprep
	@test -n "$(OUT)" -a -n "$(SRC)" || { echo "set OUT=<dir> and SRC=<clean-text jsonl>"; exit 2; }
	$(DATAPREP) product-pairs --out "$(OUT)" --src "$(SRC)" --n $(or $(N),1989) --seed $(or $(SEED),20261112)

#   make pairs ITEMS=<items.jsonl> OUT=<records.jsonl[.gz]> VAUGELAS_DATA=<dir>
pairs: dataprep
	@test -n "$(ITEMS)" -a -n "$(OUT)" -a -n "$(VAUGELAS_DATA)" || { echo "set ITEMS, OUT and VAUGELAS_DATA"; exit 2; }
	$(DATAPREP) pairs --items "$(ITEMS)" --out "$(OUT)" --data "$(VAUGELAS_DATA)"

#   make cands TEXTS=<texts.jsonl> OUT=<records.jsonl> VAUGELAS_DATA=<dir>
cands: dataprep
	@test -n "$(TEXTS)" -a -n "$(OUT)" -a -n "$(VAUGELAS_DATA)" || { echo "set TEXTS, OUT and VAUGELAS_DATA"; exit 2; }
	$(DATAPREP) cands -f "$(TEXTS)" --out "$(OUT)" --data "$(VAUGELAS_DATA)"
