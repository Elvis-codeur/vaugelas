# Releasing (for the owner)

A release has two parts on GitHub: the **data release** (`data-2`: model, lexicon, resources; changes only when the
model or the data changes) and the **program release** (`v0.2.0`: one archive per platform, plus a copy of the data
files so friends find everything on one page). Both are made as drafts; nothing is public until you publish.

## v0.2.0 (model H1)

The data release `data-2` already exists as a **draft** with `model.frcm.gz`, `lexicon.tsv.gz`, `resources.tar.gz`
and `SHA256SUMS` (sha256 of the compressed files, the uncompressed files and every file in the resources archive;
they match `crates/vaugelas-cli/src/data.rs`).

1. Merge the pull request into `main` (CI must be green, including the four-platform build of the release workflow).
2. Publish the data release (the program downloads from it once the repository is public):
   ```
   gh release edit data-2 --repo Elvis-codeur/vaugelas --draft=false
   ```
3. Tag the merged `main` and push the tag; the release workflow builds the four platforms and makes the draft
   `v0.2.0`, with the data files and the CHANGELOG section as notes:
   ```
   git switch main && git pull
   git tag -a v0.2.0 -m "Vaugelas 0.2.0 (model H1)"
   git push origin v0.2.0
   ```
4. Check the draft: 4 archives (`vaugelas-v0.2.0-linux-x86_64.tar.gz`, `…-windows-x86_64.zip`,
   `…-macos-x86_64.tar.gz`, `…-macos-arm64.tar.gz`) with their `.sha256`, the three data files and
   `data-SHA256SUMS.txt`. Try one archive: unpack it, put the three data files next to the program, run
   `vaugelas check "Il a manger toute la tarte."`.
5. Publish it: `gh release edit v0.2.0 --repo Elvis-codeur/vaugelas --draft=false`.

## A later release

- Program only: bump `version` in `Cargo.toml`, add a `## [x.y.z] - date` section to `CHANGELOG.md` (the workflow
  refuses a tag without one), merge, tag `vx.y.z`.
- New model or data: make a new data set (`data-3`): compress the files (`gzip -9n`; the resources archive with
  `tar --sort=name --owner=0 --group=0 --numeric-owner --mtime=...` so it is reproducible), write `SHA256SUMS`,
  create the draft data release, then update `DATA_SET`, `DATA_URL` and the sha256 tables in
  `crates/vaugelas-cli/src/data.rs`. A model exported with an `infer` block in its header (name, `frc_ignore`,
  `threshold`) needs no code change for its settings; otherwise add it to `crates/vaugelas/src/model/profile.rs`.
  Re-run `tools/parity.sh` against fr-check-infer.
