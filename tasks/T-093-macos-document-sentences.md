---
id: T-093
title: "State in the documents where Kiln's file and project behaviour differs on macOS, each sentence with a test that runs there"
tier: low               # METHODOLOGY v1.5 §4: documentation and tests, no behaviour change; see Q-1 (a reviewer's read)
state: queued
owner_role: planner
scorecard_items: []
depends_on: [T-079]     # merged: the macOS gate is green, so a sentence's test can be shown to run there
touches:                # one role per path (D-153): implementer (low tier: implementer's own tests)
  - docs/protocol/paths.md                  # the macOS sentences of Interface; "Unix only." replaced
  - docs/protocol/project.md                # the macOS sentences of Interface
  - apps/kiln-cli/tests/macos_t093.rs       # new: the tests a sentence needs where no existing test shows it (portable: they assert each platform's behaviour)
budget_tokens: 280000   # low tier; extended from 150,000 by D-275 (`Extend T-093 to 280,000`)
iteration: 2
found_by: planner       # split off T-072 in R-336 (the macOS sentences of the draft's touches)
origin_task: null
---

# T-093 The macOS sentences of the documents

Split off T-072 in run R-336 (main `7b86bdf`). T-072 keeps the CI evidence; this task writes, per ADR-0009 Decision 4,
one sentence per platform where behaviour differs, with evidence from that platform. The window and its keys on macOS
are T-077's (`docs/protocol/ui.md` "The window", the `command` key sentence); Windows sentences are T-073's and T-074's
(merged) and T-095's (names and case). Re-checked against main `4eb349d` after T-073, T-096, T-072 and T-074 merged:
see "Spec check (after T-074)". No gate path is in `touches`; `docs/control-protocol.md` is not in `touches` and is
not changed (its two macOS-neutral bullets are pinned by `apps/kiln-cli/tests/acceptance_t008_cli.rs:562-574`).

## The evidence rule for a macOS sentence (spec check, after T-074)
A sentence about macOS cites one of these, and says which:
- **(E1) By name, from the test list.** The test's name is listed under its binary in `test-list.txt` of a `gate-macos`
  job whose gate report is `passed: true` with `test` "… 0 failed, 0 ignored …", and a CI record under `tasks/ci-runs/`
  quotes that name from that file. The sentence names the test (`<file>::<name>`), the run id and the record. Why it
  holds: the list is `cargo test --workspace --locked --no-fail-fast -- --list` over the binaries the gate's `test` step
  (`cargo test --workspace --locked`, `xtask/src/gate.rs:110`) built and ran with no filter (`docs/development.md`
  "the test list"), so a listed name on a green job with 0 ignored ran and passed there. Agents read the record, never
  the artefact (METHODOLOGY §8.1); artefacts are kept 30 days.
- **(E2) By name, from the scorecard.** The test is listed in a `scorecard/items/*.toml` item and the `gate-macos`
  `scorecard` step of a green run reads `macos: … tests ran; N registered absent here` with no problem after it, N being
  only names registered for another platform (a failing, ignored or missing listed test is a problem there). Already
  used by `tasks/ci-runs/T-079-round-2.md` §1.
- **(E3) A failing run's output**, quoted in a record or a task file (e.g. R-285's reading of run 43765532298).
- Not evidence by name: the gate's count alone. A clause resting only on it says "(macOS: by the gate's count)".

What a listed, passing test shows on macOS is what its source **asserts on the macOS branch at that run's head**: a branch
chosen by `cfg!`/a platform constant is shown; a branch chosen by a run-time probe whose outcome the test does not
assert (e.g. `folds_case` in `apps/kiln-cli/tests/acceptance_t019_cli.rs:308`) shows only that Kiln agrees with what the
probe found, not what it found. **Unmeasured on macOS on a green run:** everything a test prints and does not assert —
`MEASURED:` lines of `guard_t073.rs`/`guard_t074.rs`, probe outcomes, OS error numbers and texts, timings — because a
green `gate-macos` uploads no `test-output.txt` (only failed gates do) and the `--show-output` steps run on
`gate-windows` only (`docs/development.md` CI section). A sentence that needs such a value marks it "unmeasured on macOS".

## Goal and the sentences
Line numbers on main `4eb349d`. Evidence column: E1/E2/E3 as above; "**record**" means the E1 quotation is still to be
written (Q-4).

| # | Where (now) | Now | On macOS | Evidence (now) |
|---|---|---|---|---|
| 1 | `docs/protocol/paths.md:156` | "- Unix only." (Residual limits), now followed by T-074's Windows bullet (`:157-159`) and contradicted by the Windows paragraphs (`:32-72`) | replace by one bullet: the rules of this page hold on Linux and macOS as written; on Windows with the differences stated in "The root" and "The rule" (T-073, T-074); names and case on Windows not yet stated (T-095) | **E2, VERIFIED**: the 28 tests of `scorecard/items/ws.path-confinement.toml` ran and passed on macOS: CI run 44358453526, `tasks/ci-runs/T-074-round-1.md` gate-macos, scorecard `macos: 64 items; 216 tests ran; 3 registered absent here` (the 3 are `ui.window`'s), no problem; head `720f1d5` equals main under `crates`, `apps` (D-271) |
| 2 | `docs/protocol/project.md:26`, `:99` | names "not valid UTF-8" are skipped and counted | the runner's file system refused to create such a name, so there `skipped` counts only names with `\` | **E3, VERIFIED**: `Os { code: 92 … "Illegal byte sequence" }` (R-285, run 43765532298; `tasks/T-079-mark-product-tests-by-platform.md:73`); the `\` half by name: `crates/kiln-workspace/tests/acceptance_t019_workspace.rs::tree_counts_skipped_names_on_disk`: **record** (E1; not in a scorecard item). Whether other macOS file systems refuse such names: unmeasured |
| 3 | `docs/protocol/project.md:120-121` | "byte order of their names; no case folding" | true; on a case-folding file system two names that differ only by case cannot both exist, so the listing holds one | **unchecked** (corrected): `acceptance_t019_cli::tree_order_and_counts` (E2, ws.open-project) passes whether or not the runner's file system folds case — its expectation follows the probe `folds_case`, whose outcome is not uploaded. Needs a test in `macos_t093.rs` that expects folding on macOS by platform (Q-5) |
| 4 | `docs/protocol/paths.md:121-122` ("The rule", re-open "(the same path text)") | a path already open is recognised by its text | on a case-folding file system `A.rs` after `a.rs` opens a second document of the same file | **unchecked**: needs a test (`macos_t093.rs`), expectation per platform (Q-5) |
| 5 | `docs/protocol/project.md:109-111` | "a path of 4,096 bytes or more cannot be listed" | the host limit is the platform's | **unchecked**; KNOWN (1,024 bytes, macOS `PATH_MAX`) is not copied (§5.7). E2 shows only that a chain beyond 4,096 bytes gives `io_error` for `nav.workspace-symbols` (`guard_t038_cli::disk_refusals_through_control`, case `symbols-deep-chain`, `control.errors`). Recommended wording without a figure for macOS: Q-3 |
| 6 | `docs/protocol/paths.md:135-136` | "more than 40 symlinks … `too many levels of symbolic links`" | the same: Kiln's own count (`crates/kiln-workspace/src/disk.rs:130` `MAX_SYMLINK_HOPS = 40`, message at `:347`), not the kernel's `ELOOP` | 40/41 boundary and message asserted by `crates/kiln-workspace/tests/confinement.rs::walk_corner_cases` (`:357-381`; Unix imports, no cfg): **record** (E1). `guard_t038_cli::disk_refusals_through_control` (E2) asserts only the kind `io_error` for a two-link loop |
| 7 | `docs/protocol/paths.md:11-12` ("The root", a root given as a symlink is that directory) | — | the temporary directory is reached through `/var` → `/private/var`; messages never name a host path, so nothing visible changes | `disk.rs::tests::failed_rename_removes_own_temp_and_keeps_target` (`:905`, after T-079's canonical-path fix) and `disk.rs::tests::root_given_through_a_link_that_is_retargeted` (`:1317`, registered `unix`): **record** (E1). Whether this is a difference to state at all: Q-3 |
| 8 | `docs/protocol/paths.md:36-38` (T-073) | root renamed and moved back "as on Linux and macOS" | — (already worded) | cites `guard_t073.rs::acw2_root_replaced_at_same_path_is_outside_root` with Windows run 44268710587 only; macOS by name: **record** (E1; T-073-round-1 has the count only) |
| 9 | `docs/protocol/paths.md:41-43` (T-073; D-251 note) | "On Linux and macOS that rename succeeds … which passed on all three in that run" | — | macOS by the gate's count only (`reviews/T-073-r2.md` nit 1). `acw2_root_parent_replaced_is_outside_root`'s non-Windows branch panics unless the rename succeeds (review r2, `:90`), so a pass by name shows the clause: **record** (E1), else "(macOS: by the gate's count)" |
| 10 | `docs/protocol/paths.md:44-47` (T-074) | root removed "as on Linux and macOS" | — | **E1, VERIFIED**: `guard_t074.rs::awa4_root_removed_while_held` (no cfg; asserts every call `OutsideRoot` on every platform) listed on gate-macos of run 44358453526 (`tasks/ci-runs/T-074-round-1.md:558`), gate `passed: true`, `test` `1183 passed, 0 failed, 0 ignored`. The sentence cites Windows runs only: add the macOS citation |
| 11 | `docs/protocol/paths.md:52-55` (T-074) | "A difference from Linux and macOS: a `..` in `--root` is resolved by its text" (so on macOS physically) | — | **E1, VERIFIED**: `awa4_dotdot_after_a_junction_in_the_root_text` (its non-Windows branch expects `k/a/b`, `guard_t074.rs:1232-1236`), same run and line. Add the macOS citation |
| 12 | `docs/protocol/paths.md:56-60` (T-074) | root through a link, retargeted, "as on Linux and macOS" | — | **E1, VERIFIED**: `awa4_root_given_through_a_directory_link_and_retargeted` and `…_junction_and_retargeted` (on Unix both plant a symbolic link, `guard_t074.rs:20-23`), same run and line. Add the macOS citation |

Rows 1-7 are R-336's (row 3 re-graded); rows 8-12 are the macOS clauses T-073 and T-074 introduced, worded already, whose
macOS evidence the documents do not name. Nothing of R-336's rows was done by T-073 or T-074: row 1's "Windows not yet
stated" is now stated (so the replacement points to it), and T-074's root-through-a-link bullet covers retargeting, not
row 7's `/var` point. `docs/protocol/project.md:81-84` and the `docs/control-protocol.md` additions (`:135-147`) are
Windows-only and claim nothing about macOS.

After this task each row's sentence or clause is in the document, worded per platform, and cites its macOS evidence by
rule E1/E2/E3, or says "(macOS: by the gate's count)" or "unmeasured on macOS"; rows 3 and 4 are written only with their
new tests' result on the CI round (AC-2).

## Pinned
- (a) Error variants, payloads, precedence: n/a — no product code changes; the new tests assert existing kinds.
- (b) Files read or written: the three of `touches`; fixtures of `macos_t093.rs` under `CARGO_TARGET_TMPDIR`, removed on drop.
- (c) Order: tests first; one CI round (Q-2 A, D-245); sentences citing the round after its record; the commit after the
  round changes only the two documents (the reviewer checks `git diff <CI head>..HEAD --name-only`).
- (d) Platform: `macos_t093.rs` has no platform cfg and uses only portable `std` (no `std::os::unix`), so it compiles for
  Windows (`gate-windows` does not build `kiln-cli`'s tests until T-098); the expected value is chosen by `cfg!`, not by
  a run-time probe (rule above). Windows branch: Q-5.
- (e) Threat model: n/a — low tier, no boundary changes. Risk: a sentence wider than its evidence; the reviewer's read (Q-1 A).
- (f) Tests and registry entries pinning what this task edits (D-249 grep, below): none needs to change; the quoted
  phrases listed there stay verbatim (whitespace runs may differ: `no-skip` compares them equal).
- (g) Worked examples: n/a (no command output is pinned; the tests' expectations are the rows' "On macOS").
- (h) Scorecard items: none.
- (i) Sentences: as the table; each carries E1/E2/E3 or a mark. Rows 1, 2 (first half), 10-12: VERIFIED as cited.
  Rows 2 (`\` half), 6-9: **to be measured by the director** — the E1 quotation in the record of Q-4. Rows 3, 4: **to be
  measured by the director** in `tasks/ci-runs/T-093-round-1.md` (the new tests listed on a green `gate-macos`). Row 5:
  "unmeasured on macOS" (Q-3).

## Acceptance (low tier: implementer's own tests, gate only)
- AC-1: the new tests assert each platform's behaviour (no platform cfg on a test; the expected value chosen by `cfg!`),
  so they need no registry entry; `cargo xtask ci` green in the worktree (Linux).
- AC-2: the CI record of the branch on `ci/candidate` (`tasks/ci-runs/T-093-round-1.md`) shows `gate-macos` green
  (`passed: true`, `test` "… 0 failed, 0 ignored") and quotes from its `test-list.txt` the names of the new tests and of
  every test a row cites as "record" (E1). This is a docs-and-tests task with a CI criterion: Q-2.
- AC-3 (reviewer's read, Q-1 A): every macOS sentence or clause in `paths.md` and `project.md` cites evidence by rule
  E1/E2/E3 that the record shows, or carries "(macOS: by the gate's count)" or "unmeasured on macOS": exactly rows 1-12;
  a "macOS" clause outside them (for example one added in "For the platform report") is a finding. Every registry quote
  and test-pinned phrase of the D-249 list still occurs (main's `no-skip` passes).

## Estimate
Low tasks with a reviewer's read: T-069 346,304 (D-179) with two extensions; plain low tasks 84,466–124,444
(retro-2s1). With one CI round: **about 150,000**, at the low ceiling, unchanged: rows 8-12 add citations only, and
row 5 without a test (Q-3) removes the case most likely to need a second round. A second CI round (a new test red on
macOS) would exceed the ceiling (INFERRED).

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

## Note from T-073's review round 2 (director, D-251)
`docs/protocol/paths.md`, the Windows paragraph of "The root" (T-073): the clause that says what Linux and macOS
do when a directory above the root is renamed rests, for macOS, on the gate's count and not on the test by name
(`reviews/T-073-r2.md`, nit 1). This task names the evidence for the macOS half (T-072 makes a green run list its
tests) or words it as "by the gate's count". → Row 9.

## Questions and rulings

### Spec check (after T-074)
Planner, read-only, against main `4eb349d` (`git diff 7de171b..HEAD -- docs/protocol docs/control-protocol.md`: commits
`f6503a8`, `84c72e2` (T-073), `afa5abd`, `0809efc` (T-074)). Verdict: **no blocker**; line numbers and row 3 corrected,
rows 8-12 added, evidence rule written; four questions below.

**D-249 (main's tests and the registry against every sentence this task would reword).** Commands:
`grep -n -B4 -A6 'doc = "docs/protocol/paths.md"\|…project.md"\|…control-protocol.md"' xtask/platform-tests.toml`;
`grep -rn 'paths.md\|project.md\|control-protocol.md' xtask/tests xtask/src apps/*/tests crates/*/tests crates/*/src`.
- Registry quotes in the two documents (each must still occur; `no-skip` `platform-sentence` fails otherwise, and the
  queue runs main's tool): `paths.md:11` "if `--root` is a symlink to a directory, that directory is the root" (row 7
  is added after it, not into it); `:19-20` "only so that no other directory can take its identity"; `:24` "the root,
  or a directory above it, was replaced, for example by a symlink to another directory"; `:25` "the session refuses
  every path with `outside_root`, at open and at save"; `:121` "A path that is already open is checked again, for where
  it leads only." and `:123` "is refused with `outside_root` when the root path no longer leads to the root" (row 4
  adds a sentence after the bullet's "(the same path text)" sentence, rewording neither); `:136` "A link whose target
  ends in `/` must lead to a directory, as the kernel requires" (row 6 is the sentence before it: keep this one whole);
  `project.md:26` "not valid UTF-8 or contain" (4 entries; row 2 adds a sentence, the bullet's phrase stays).
- Quotes T-092 will add (its spec, Interface (2)): `paths.md:112` "Directories, FIFOs, sockets and devices are
  refused", `paths.md:13-15` (the unreadable root), `project.md:80` "FIFOs, sockets and devices are listed with kind
  `other`": none is a row here; keep verbatim whichever merges first.
- Tests: `acceptance_t008_cli.rs:575-578` reads `paths.md` only for the words `outside_root` and `not_a_file`;
  `:549-574` pins two bullets of `control-protocol.md`, which this task does not touch. No `xtask/tests` file pins a
  sentence of these documents (`acceptance_t079.rs:18` leaves `doc`/`quote` to `no-skip`). The rest of the hits read
  `control-protocol.md` tables or name the documents in comments and descriptions.
- What it says about the planned text: rows 1-12 can be written without changing any quoted phrase; none does.

**D-235 (gate paths).** The task changes no gate path: `touches` holds two documents under `docs/protocol/` and one test
file under `apps/kiln-cli/tests/`; none is under `xtask/` or `.github/` or a build, lint or toolchain file. The new tests
have no platform cfg, so no registry entry; no registry quote must change (D-249 above). Confirmed: tier low,
`touches` unchanged. Should a quote have to change, `xtask/platform-tests.toml` is a gate path: the task would be high,
`touches` would gain it, the Goal and the approval row would say `gate paths`, and main's tool would have to accept the tree.

**Overlap and order.**
- **T-095** (`touches`, provisional: `paths.md`, `project.md`, `control-protocol.md`, `path.rs`, two test files) shares
  both documents with T-093 (§5.2: not in flight together), and the subject: case (rows 3, 4 for macOS; T-095 for
  Windows) and `skipped` names (row 2; T-095 if Windows-refused names are counted). **T-093 merges first** (low, one CI
  round, specified; T-095 is being specified). T-095's spec check after it re-reads the new line numbers, the "Unix
  only." replacement (T-095 removes T-074's "T-095's" clause at `paths.md:71-72` and row 1 names T-095), rows 2-4's
  wording (its Windows case sentences go beside them), and `macos_t093.rs`'s Windows branch (Q-5).
- **T-092** (kiln-workspace test files, `disk.rs`'s test module, the registry, `xtask/tests`): no path shared with
  T-093. **T-093 can run beside T-092 now.** Whichever merges second re-checks only the D-249 list: T-093 keeps T-092's
  three planned quotes verbatim; T-092 adds no document sentence. T-092's Windows tests for names that are not Unicode
  (W42, W43) concern row 2's sentence on Windows, which neither task writes.
- **T-098** (not read; D-269): `macos_t093.rs` is a new `kiln-cli` test target, so if T-093 merges first T-098's set of
  targets that must build on Windows is one larger (Pinned (d) keeps it portable).

**Log sections (D-266).** The file now has every role's section with its end-of-log marker, as the template
(`methodology/templates/task.md`): director, planner, implementer, test author (n/a at low tier, kept for the layout),
reviewer.

**Test arm.** Low tier: the implementer writes and runs its own tests (gate only); the documents have no code, so their
check is (1) the evidence each sentence cites, read by the reviewer against the records (Q-1 A, AC-3); (2) main's
`no-skip`, which fails a reworded registry quote at the gate and at the queue; (3) the director's record (AC-2). If the
director takes Q-3 B, the task is documents only, and checks (1)-(3) are its whole test arm.

Questions for the director:
- **Q-3 (planner, spec check). Rows 5, 6, 7: write a sentence or only evidence?** Row 5: A — "the bound is the host's
  path limit, unmeasured on macOS" (no figure, no test); B — a test of the macOS bound in `macos_t093.rs` (the figure is
  then whatever the round shows; a wrong guess means a second round). Rows 6, 7 state no difference (same on macOS):
  A — cite their evidence in row 1's bullet, no new sentence; B — a sentence each. **Recommendation: A for all three**
  (ADR-0009 Decision 4 asks a sentence where behaviour differs; within the ceiling).
- **Q-4 (planner, spec check). Where the E1 quotations of rows 2, 6-9 come from.** A — the director adds an addendum to
  `tasks/ci-runs/T-074-round-1.md` quoting those names from the `gate-macos` `test-list.txt` of run 44358453526 (head
  `720f1d5`, equal to main under `crates`/`apps`, D-271; artefact kept until about 2026-12-18), before dispatch, so the
  implementer cites a record that exists; B — `T-093-round-1.md` quotes them with the new tests, and the sentences cite
  that run (the last commit then follows the record). **Recommendation: A** — the existing tests' evidence does not
  wait on this task's round, and the sentences need no commit after it except rows 3 and 4.
- **Q-5 (planner, spec check). Expectations of `macos_t093.rs`.** Rows 3, 4 depend on the file system folding case. A —
  chosen by platform: Linux distinct names, macOS folded (the runner's and the default volume's behaviour; a
  case-sensitive APFS volume fails the test naming why, METHODOLOGY §8.1), Windows as macOS (INFERRED from NTFS's
  default, unrun until T-098; no Windows sentence here, T-095's); B — chosen by a probe, which shows nothing about
  macOS on a green run (rule above). **Recommendation: A**, with the Windows branch named in T-095's spec check.
- **Q-6 (planner, spec check). Row 9 if Q-4 A is not taken in time.** Word it "(macOS: by the gate's count)" as D-251
  allows. **Recommendation:** only as a fallback; prefer E1 by name.

### Rulings on the spec check after T-074 (director, D-272)
- **Q-3: A.** Row 5 says "the host's path limit, unmeasured on macOS" with no figure; rows 6 and 7 get a citation,
  not a new sentence.
- **Q-4: A.** The director has the E1 quotations for rows 2 (the `\` half) and 6 to 9 appended to
  `tasks/ci-runs/T-074-round-1.md` ("Addendum for T-093") from the macOS test list of run 44358453526, and passes
  the result to the implementer.
- **Q-5: A.** The new test's expected result is set by platform; on Windows it expects the macOS result (an
  inference, not run before T-098 makes `kiln-cli`'s test targets compile there); T-095 writes the Windows sentence.
- **Q-6:** row 9 is worded "(macOS: by the gate's count)" only if the quotation of Q-4 does not show the test.
- Order: before T-095, beside T-092.

## Log
### director
<!-- end director log -->
### planner
- 2026-11-18 · R-336 · draft · did: filed from the split of T-072 · next: director
- 2026-11-18 · spec check (run ID not given in the dispatch) · specified · did: spec check: re-checked against main `4eb349d` after T-073, T-096, T-072, T-074 (diff `7de171b..HEAD` of the protocol documents; CI records T-072-round-5, T-074-round-1, T-079-round-2; registry and tests grep). Line numbers corrected (rows 2, 3, 5 moved by 3 lines; rows 4, 6 located); row 3 re-graded VERIFIED → unchecked (its test follows a probe whose outcome is not uploaded); rows 8-12 added (T-073/T-074 macOS clauses; 10-12 VERIFIED by name from T-074-round-1:558); evidence rule E1/E2/E3 written; Pinned section added; AC-2 extended to quote names, AC-3 (reviewer's read, equality to rows 1-12) added; D-249/D-235 answered (no quote changes, no gate path, tier low, touches unchanged); order: T-093 before T-095, beside T-092; log sections per template. Estimate unchanged (about 150,000) with Q-3 A. Unsure: whether `test-list.txt`'s name count (1,182) against the gate's 1,183 hides a test (unreconciled in the record); whether the T-074-round-1 macOS list holds `guard_t073`'s names (not quoted, INFERRED from no cfg). · not run: no build, no CI, no artefact read · missing context: the dispatch run ID · next: director, Q-3 to Q-6
<!-- end planner log -->
### implementer
<!-- end implementer log -->
### test author
<!-- end test author log -->
### reviewer
<!-- end reviewer log -->
