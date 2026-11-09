# Sharing Vaugelas with friends (for the owner)

Both repositories stay **private**. This page says how to let a few friends in, what they need, and what to tell them.

## The two repositories in one paragraph each

**vaugelas** (this one, for everybody). *Vaugelas is a French grammar checker that runs entirely on your own computer.
It combines hand-written grammar rules with a small neural model that compares each word with the other forms it
could take (annulés / annulées, son / sont, vert / verre) and proposes the one that fits. It is strongest on
agreement, participles and moods, catches many homophones, and is weak on word choice that needs meaning. Use it from
a terminal, or run it as a local LanguageTool server and plug it into VS Code, your browser or LibreOffice. Nothing
you write leaves your machine.*

**vaugelas-research** (for the curious and the technical). *How Vaugelas was built and measured: the Python training
code, a record of every training run with its curves and scores, the chronology of the experiments (what worked, what
did not, and why), the errata of mistakes found and corrected, and the model card of each released model. Read it if
you want to know how a paradigm-energy model learns French grammar, or to rebuild the model yourself.*

## Giving access

On GitHub, for each repository: **Settings › Collaborators › Add people**, type the friend's GitHub username, send.
They get an e-mail and must accept the invitation. Notes:

- On a personal account there is no read-only role: a collaborator can push to the repository. Protect `main`
  (**Settings › Branches › Add rule**: require a pull request before merging) if you want every change to go through
  you. Friends can still open issues, which is what you want from them.
- Collaborators can see draft releases too; publish the release before telling them where to download.
- To remove someone later: same page, **Remove**.
- A friend without a GitHub account: send them the four files (program + three data files) directly instead.

## What a friend needs

- Nothing to install besides Vaugelas itself: one program (about 6 MB) and three data files (about 83 MB to
  download, about 200 MB once installed). No Python, no Java, no Internet connection after the first start.
- A computer with about 1 GB of free memory (Vaugelas uses about 650 MB while running).
- Windows 10 or 11 (x86-64), macOS 11 or later (Apple silicon or Intel), or Linux x86-64.
- A terminal, or an editor/extension that can talk to a LanguageTool server (see the README).

What to send them: the link to the repository's **Releases** page and this sentence: *"Download the file for your
system and the three data files (model.frcm.gz, lexicon.tsv.gz, resources.tar.gz), unpack the program, put the three
data files next to it, and follow the README."*

## Before the first invitation (checklist)

- [ ] Merge the pull request of the H1 switch and push the tag (see [RELEASING.md](RELEASING.md)).
- [ ] Publish the data release `data-2` and the `v0.2.0` release (both are drafts until you publish them).
- [ ] Read [RIGHTS-CHECKLIST.md](RIGHTS-CHECKLIST.md): nothing there blocks a private share with a few friends, but
      decide about `tests/fixtures/sample.txt` (held-out test sentences) first.
- [ ] Optionally protect `main` (above).
