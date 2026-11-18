---
id: T-093
title: "State in the documents where Kiln's file and project behaviour differs on macOS, each sentence with a test that runs there"
tier: low               # METHODOLOGY v1.5 §4: documentation and tests, no behaviour change; see Q-1 (a reviewer's read)
state: specified
owner_role: planner
scorecard_items: []
depends_on: [T-079]     # merged: the macOS gate is green, so a sentence's test can be shown to run there
touches:                # one role per path (D-153): implementer (low tier: implementer's own tests)
  - docs/protocol/paths.md                  # the macOS sentences of Interface; "Unix only." replaced
  - docs/protocol/project.md                # the macOS sentences of Interface
  - apps/kiln-cli/tests/macos_t093.rs       # new: the tests a sentence needs where no existing test shows it (portable: they assert each platform's behaviour)
budget_tokens: 150000   # low (v1.5); see Estimate
iteration: 2
found_by: planner       # split off T-072 in R-336 (the macOS sentences of the draft's touches)
origin_task: null
---

# T-093 The macOS sentences of the documents

Split off T-072 in run R-336 (main `7b86bdf`). T-072 keeps the CI evidence; this task writes, per ADR-0009 Decision 4,
one sentence per platform where behaviour differs, with evidence from that platform. The window and its keys on macOS
are T-077's (`docs/protocol/ui.md` "The window", the `command` key sentence); Windows sentences are T-073's and T-074's.

## Goal and the sentences
Today the documents describe Linux and are silent or wrong for macOS in these places (read on `7b86bdf`):
| # | Where | Now | On macOS | Evidence |
|---|---|---|---|---|
| 1 | `docs/protocol/paths.md:114` | "Unix only." | the rules hold on Linux and macOS; Windows not yet stated (T-073, T-074) | VERIFIED: `gate-macos` green with all 28 `ws.path-confinement` tests run (`tasks/ci-runs/T-079-round-2.md`: scorecard "216 tests ran; 3 registered absent here", the 3 being `ui.window`'s) |
| 2 | `docs/protocol/project.md:26`, `:96` | names "not valid UTF-8" are skipped and counted | macOS's file system refuses such a name, so `skipped` counts only names with `\` | VERIFIED: creating one failed with `Os { code: 92 … "Illegal byte sequence" }` (R-285, run 43765532298 artefact; the four tests are now split, `_non_utf8` registered `linux`) |
| 3 | `docs/protocol/project.md:117` | "byte order of their names; no case folding" | true, but two names that differ only by case cannot both exist on the default file system | VERIFIED: `tree_order_and_counts` passes on macOS with its case-folding probe (T-079 round 2, scorecard no problem) |
| 4 | `docs/protocol/paths.md` "The rule", re-open "(the same path text)" | a path already open is recognised by its text | on a case-folding file system `A.rs` after `a.rs` opens a second document of the same file | **unchecked** — needs a test (`macos_t093.rs`), asserting per platform |
| 5 | `docs/protocol/project.md:106` | "a path of 4,096 bytes or more cannot be listed" | the host limit is the platform's (KNOWN: 1,024 bytes on macOS) | **unchecked** — the figure needs a test that runs on macOS; until then the sentence names no figure for macOS |
| 6 | `docs/protocol/paths.md` "more than 40 symlinks … `too many levels of symbolic links`" | Kiln's own count (`disk.rs` `MAX_SYMLINK_HOPS`) | same on macOS (the kernel's `ELOOP` number differs, 62, but Kiln reports its own message) | VERIFIED: `disk_refusals_through_control` passes on macOS (T-079 round 2) |
| 7 | `docs/protocol/paths.md` "The root" (a root given as a symlink is that directory) | — | the temporary directory is reached through `/var` → `/private/var`; messages never name a host path, so nothing visible changes | VERIFIED by `failed_rename_removes_own_temp_and_keeps_target` passing there after T-079's canonical-path fix |
After this task each row's sentence is in the document, worded per platform, and each has a test that runs on macOS
(existing, named in the sentence's evidence, or new in `macos_t093.rs`); rows 4 and 5 are written only with their new
tests' result on a CI run.

## Acceptance (low tier: implementer's own tests, gate only)
- The new tests assert each platform's behaviour (no platform cfg on a test; the expected value chosen by platform),
  so they need no registry entry.
- The CI record of the branch on `ci/candidate` (`tasks/ci-runs/T-093-round-1.md`) shows `gate-macos` green with the
  new tests `ok` by name. This is a docs-and-tests task with a CI criterion: Q-2.

## Estimate
Low tasks with a reviewer's read: T-069 346,304 (D-179) with two extensions; plain low tasks 84,466–124,444
(retro-2s1). With one CI round: **about 150,000**, at the low ceiling.

## Questions
- **Q-1 (planner, R-336). Review.** A: low with a reviewer's read of the sentences against the CI record (as ADR-0009
  Q-7 for T-069). B: plain low. **Recommendation: A** — the sentences are platform claims, and low has no other check.
- **Q-2 (planner, R-336). A CI round for a low task.** The new tests touch no platform-sensitive path, but their point
  is macOS. A: the director pushes the head to `ci/candidate` once and records it before queueing. B: rely on the first
  `platforms.yml` run on main after the merge. **Recommendation: A** — a sentence about macOS needs its evidence before
  it reaches main.

### Rulings on spec R-336 (director, D-245)
- **Q-1: A** (a reviewer's read). **Q-2: A** (one CI round before the queue).
- Dispatched when `docs/protocol/paths.md` is free: after T-073 and T-074, beside T-092.

## Log
### director
<!-- end director log -->
### planner
- 2026-11-18 · R-336 · draft · did: filed from the split of T-072 · next: director
<!-- end planner log -->
