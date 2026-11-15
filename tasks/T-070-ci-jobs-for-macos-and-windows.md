---
id: T-070
title: "Run the gate in CI on macOS and Windows as tracking jobs, with line endings untouched on checkout, the reports kept, a window probe on each platform, and the Linux window procedure under Xvfb"
tier: high              # ADR-0002 "a gate path": .github/ (a new workflow) and .gitattributes is read by every checkout the gate runs in; the approval row must contain `gate paths`
state: draft
owner_role: planner
scorecard_items: []     # no item: it produces evidence, it changes no behaviour of Kiln
depends_on: [T-069]     # the existing job must be green first, or nothing can be read from CI (ADR-0009 Context)
touches:                # provisional, fixed at specification; one role per path (D-153)
  - .github/workflows/platforms.yml      # GATE PATH: implementer (new; `ci.yml` is not edited: T-041 owns it)
  - .gitattributes                       # implementer (new): every tracked file checked out byte for byte
  - xtask/tests/acceptance_t070.rs       # GATE PATH: test author (the workflow's structure, read as a file)
  - docs/development.md                  # implementer: the "CI" section only (T-064 also edits this file: sequence them)
budget_tokens: 1100000  # high (v1.3)
iteration: 2
found_by: planner       # ADR-0009 (R-255)
origin_task: null
---

# T-070 Run the gate in CI on macOS and Windows as tracking jobs

Draft by the planner of ADR-0009 (run R-255); fuller than the other drafts because every later platform
task depends on what its first runs show. Interface, Pinned and acceptance criteria are completed at
specification; this task changes no Rust code of Kiln.

## Goal
Today nothing is known about Kiln on macOS or Windows beyond compiling: on this machine macOS compiles
completely and Windows compiles except 6 errors in `crates/kiln-workspace/src/disk.rs` and 198 in test
code (ADR-0009 probe a3–a5); nothing can be linked or run for those platforms here (probe a2). After
this task, every push to main that changes a path outside the ledger, every push to a branch
`ci/T-NNN`, a nightly run and a manual dispatch run, in a new workflow, one job per platform:

| Job | Runner label | What it runs |
|---|---|---|
| `gate-linux` | `ubuntu-24.04` | not in this workflow: `ci.yml` stays the Linux gate (T-041 later) |
| `gate-macos` | `macos-15` (arm64) | `cargo xtask ci --keep-going --report …` with the pinned toolchain |
| `gate-windows` | `windows-2025` (x86_64, MSVC) | the same |
| `window-linux` | `ubuntu-24.04` | builds `kiln`, starts Xvfb, puts the two libraries of ADR-0008 in place on the runner, runs T-065's window procedure (six cases) and the comparison with `kiln script` (ADR-0009 Q-3) |
| `window-probe` (macOS, Windows) | as above | builds `kiln` and starts `kiln --root <dir> --script <three-line script>` once, under a timeout; records exit status, stdout and stderr |

- The macOS and Windows jobs are **tracking jobs** (ADR-0009 Decision 3): their failure is reported and
  recorded, and is not "main red" until ADR-0009 Q-1 says so. The workflow does not hide a failure: no
  `continue-on-error`, the job is red when the gate is red; "tracking" is a rule of the record, not of
  the workflow.
- Every job uploads its gate report and the full output of the test runs (the scorecard step's test
  output included, which today shows only test names; job log of run 352) as an artefact with a stated
  retention, so that the director, a reviewer or the evaluator can read why a test failed on a platform.
- The checkout is byte-identical to the repository on all three platforms: today a checkout with
  `core.autocrlf=true` (Git for Windows' default, KNOWN) fails the gate's `schema` step and one
  `syntax.parse` test (ADR-0009 probe c1, c2, VERIFIED). After this task a clone on Windows with Git's
  defaults gets the files as committed (repository attributes), and the workflow's checkout does too.
- Ledger-only pushes start no job of this workflow (the path list is `xtask/ledger.toml`'s; T-041's drift
  test will cover both workflows later). No `concurrency` group, as `ci.yml` (every state is checked).
- Actions are pinned by commit sha, as in `ci.yml`; `permissions: contents: read`.

`docs/development.md` "CI" states the jobs, their triggers, what "tracking" means and how a result is
read (ADR-0009 Decision 3). Gate paths changed: `.github/workflows/platforms.yml`, `xtask/tests/`.

## What the first runs must report (recorded by the director in a decision row)
Per platform: how far the gate got and the first failure; wall time per job (the plan's figure 10–20 min
for macOS and Windows is a guess, ADR-0009 Consequences); disk used at the end (the runners have 14 GB,
probe d1; the debug test build was 6.3 GB on Linux, D-097: if a job runs out, T-054 comes first); the
window probe's line on macOS and Windows (a session to open a window in? OpenGL ≥ 2?). These figures
replace the estimates of ADR-0009 and decide the scope of T-072, T-073 and T-077.

## Pinned (first answers; completed at specification)
- (b) files written: the workflow file; `.gitattributes`; artefacts on GitHub only. Nothing in the repo
  is written by a job.
- (d) platform: the labels above, pinned (no `-latest`); toolchain from `rust-toolchain.toml` on all three
  (the images carry 1.99.1, the job installs 1.97.0, as `ci.yml` does today: job log of run 352).
- (e) threat model: `permissions: contents: read`; no secret is used; `pull_request` is not a trigger
  (no code from forks runs); the libraries for Xvfb come from the runner's own package mirror.
- (f) tests pinning what this changes: none known; the test author checks `xtask/tests/` for tests that
  read `.github/`.
- (h) the evaluator does not evaluate this task; the director reads the runs (ADR-0009 Decision 3).
- (i) document sentences: `docs/development.md` "CI"; no `docs/protocol/` sentence changes.

## Acceptance (sketch)
- AC-1 (test author, gate test reading the workflow as a file): the platform jobs exist with the pinned
  labels, each runs `cargo xtask ci`, none has `continue-on-error`, the trigger's path filter excludes
  exactly the ledger paths of `xtask/ledger.toml`, every `uses:` is pinned by a 40-hex sha.
- AC-2 (test author): a checkout of the task head made here with `core.autocrlf=true` has no file whose
  bytes differ from the committed blob (`git ls-files` against `git cat-file`), so `schema` passes there.
- AC-3 (director, recorded): a manual dispatch on the `ci/T-070` branch runs all jobs to completion; the
  reports are downloadable; the Linux window job passes T-065's six cases.
- AC-4 (reviewer): a deliberately failing commit on a scratch `ci/` branch turns the job red, and the
  artefact shows the failing test's message.

## Out of scope
Making macOS or Windows green (T-072 to T-076); the window on macOS and Windows (T-077); release builds
(T-078); `ci.yml` (T-041); a queue precondition reading CI results (ADR-0009 Decision 5, proposed for
T-033's scope).

## Estimate
High tasks of this stage: 887,544 (T-065), 997,800 (T-055), 1,190,305 (T-056) (D-168, D-155, D-162);
stage-1 high median 810,888. This task has little Rust (one gate test) but three platforms to iterate
on through CI, each round costing a push and 10–20 minutes of waiting: estimate **about 850,000**, with
wall time dominated by CI runs. Spec check by the drafting planner about 35,000 (D-163).

## Log
### director
<!-- end director log -->
### planner
- 2026-11-15 · R-255 · draft · did: filed from ADR-0009 · next: director
<!-- end planner log -->
### implementer
<!-- end implementer log -->
### test author
<!-- end test author log -->
### reviewer
<!-- end reviewer log -->
