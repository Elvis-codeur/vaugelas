# Releasing

A release has two parts on GitHub: the **data release** (`data-3` today: model, lexicon, resources; it changes only
when the model or the data changes) and the **program release** (`vX.Y.Z`: one archive per platform, plus a copy of
the data files so everything is on one page). Both are made as drafts and published after a check.

## A program release
1. Bump `version` in `Cargo.toml` and add a `## [x.y.z] - date` section to `CHANGELOG.md` (the workflow refuses a tag
   without one); merge into `main` with CI green.
2. Tag and push: `git tag -a vX.Y.Z -m "Vaugelas X.Y.Z" && git push origin vX.Y.Z`. The release workflow builds Linux
   x86_64, Windows x86_64, macOS x86_64 and arm64, adds the data files of the data set named in
   `crates/vaugelas-cli/src/data.rs` (`DATA_SET`, checked against its `SHA256SUMS`) and makes a draft with the
   CHANGELOG section as notes.
3. Check the draft: 4 archives with their `.sha256`, the three data files and `data-SHA256SUMS.txt`. Unpack one
   archive, put the three data files next to the program, run `vaugelas check "Il a manger toute la tarte."`.
4. Attach `DATA-LICENSES.md` and `LICENSES/*.txt`, then publish: `gh release edit vX.Y.Z --draft=false`.

## A new model or new data
Make a new data set (`data-N`): compress the files (`gzip -9n`; the resources archive with
`tar --sort=name --owner=0 --group=0 --numeric-owner --mtime=...` so it is reproducible), write `SHA256SUMS`,
create the draft data release with the licence files, then update `DATA_SET`, `DATA_URL` and the sha256 tables in
`crates/vaugelas-cli/src/data.rs`. A model exported with an `infer` block in its header (name, `frc_ignore`,
`threshold`) needs no code change for its settings; otherwise add it to `crates/vaugelas/src/model/profile.rs`.
Re-run `tools/parity.sh` against fr-check-infer, publish the data release, then make the program release.
