---
id: T-095
title: "On Windows, refuse as invalid_path the characters Windows cannot store in a name (':' with streams and drive prefixes, <>\"|?*, control characters), and state how reserved names, trailing dots and spaces, case and short names behave"
tier: high              # METHODOLOGY v1.5 §4: the confinement boundary (the lexical check of every request path) and a security property a document states. No gate path, no dependency change; no workflow edit (T-072's general reported Windows step runs the test file, D-254).
state: draft
owner_role: planner
scorecard_items: []     # none claimed (ws.* items verified on Linux; per-platform confirmation is the evaluator's)
depends_on: [T-074, T-093]   # T-074 merged (25054fe); T-093 (specified, low): shares docs/protocol/paths.md and project.md and states the macOS case sentence this task's Windows one sits beside (Q-5)
touches:                # one role per path (D-153)
  - crates/kiln-workspace/src/path.rs        # implementer: Interface (1) under #[cfg(windows)] in RelPath::new, and its #[cfg(test)] module (one new portable unit test; the three existing test names unchanged)
  - docs/protocol/paths.md                   # implementer: the sentences of Pinned (i) for paths.md
  - docs/protocol/project.md                 # implementer: the Windows clause of `skipped` (:26, :99) and the D-270 citation (:82-83)
  - docs/control-protocol.md                 # implementer: a Windows sentence after the lexical bullet (bullet verbatim), and the D-270 citation (:144-147)
  - crates/kiln-workspace/tests/guard_t095.rs   # test author: Interface (5); passes on Unix base
budget_tokens: 1100000  # high (v1.5); estimate about 450,000
iteration: 2
found_by: planner       # cut from T-074 at its re-specification (ADR-0009 (b), R-255; D-254 ruling 1)
origin_task: null
---

# T-095 Windows: names Windows gives a meaning, case and short names

Specified in full under v1.5 against main `4eb349d` (T-074 merged at `25054fe`; `path.rs` unchanged since `72522f2`:
`git diff --stat 72522f2 4eb349d -- crates/kiln-workspace/src/path.rs` empty). Measured base: the T-074 probe, run
44290286460 at `de8106f`, record `tasks/ci-runs/T-074-probe-1.md`, letter I, on `windows-2025`, through `DiskFs` with a
`\\?\` root (PROBE B4); cited as "PROBE <id>". In that probe `RelPath::new` accepted every name tried: the test prints
`PROBE I0` only for a name it refuses, and the record has no `I0` line.

## Goal
`RelPath::new` (`crates/kiln-workspace/src/path.rs:29-57`) refuses the same forms on every platform. On Windows, names
the walk passes to the file system then behave as follows at base:
- `:` reaches stream syntax and drive prefixes. `f.txt::$DATA` reads `f.txt` under a second name (PROBE I2
  `read "f.txt::$DATA": Ok([102])`), and `check_location` answers `Ok` for it (I3). `C:x`, `C:` and `D:x` answer
  `OutsideRoot` (I1-I3), but only after `Path::join` has left the root (I10 `canonical.join("C:x"): "C:x"`), so the
  file system outside was consulted.
- `<`, `?`, `*`, `|`, `"` and U+0001 answer `Io InvalidFilename`, os error 123, for write, read and check (I1-I3): an
  `io_error` after a file-system call, where the request was invalid from the start.

After this task, on Windows:
- These characters are refused as `invalid_path` before the file system is consulted, as `\` and NUL are now.
- Names Windows can store but treats specially are stated, not refused: reserved device names, a trailing dot or
  space.
- Case and 8.3 short names are stated as second spellings of one file.

Linux and macOS are unchanged. The two citations of D-270 are corrected.

Reading list:
- `tasks/ci-runs/T-074-probe-1.md` letter I, and B4.
- `docs/protocol/paths.md` "Lexical refusals", "The rule" (re-open by path text), and its Windows paragraphs.
- `docs/protocol/project.md:22-26`, `:79-84`, `:99`; `docs/control-protocol.md` "Paths".
- `crates/kiln-workspace/src/path.rs`, and `disk.rs` `list_dir` (names not a valid `RelPath` are skipped).
- `tasks/T-093-macos-document-sentences.md` rows 2-4.
- METHODOLOGY v1.5 §5.7, §6, §8.1.

## Interface (behaviour on Windows; Linux and macOS unchanged)
### (1) Lexical refusals added on Windows
`RelPath::new(p)` returns `Err(PathError)` on Windows if `p` contains any character of exactly this set:
- `:`, `<`, `>`, `"`, `|`, `?`, `*`;
- U+0001 to U+001F.

These join the existing refusals (`\`, NUL, empty, leading `/`, empty, `.` or `..` segments). The control protocol maps
them, as now, to `rejected` / `invalid_path`. The precedence is the existing one: lexical before everything.

This covers stream names (`f.txt:s`, `f.txt::$DATA`, `f.txt:s:$DATA`), drive-relative segments anywhere (`C:x`, `C:`,
`x/C:y`) and the characters Win32 does not allow in a name. The `PathError.reason` text is not specified, and no test
pins it.

Reason for this set: it is the set of characters other than `\`, `/` and NUL that Win32 does not allow in a file name.
The rule rests on Microsoft's naming rules (read, not run). Measured samples: PROBE I1 gave os error 123 for `<`, `?`,
`*`, `|`, `"` and U+0001, and every `:` form was refused by the file system (`f.txt:s` write) or reached a stream or
another drive (I1-I3). So no file a Windows project can hold through Win32 becomes unreachable; `>` and U+0002 to U+001F
were not sampled.
### (2) Not refused (stated)
Reserved device names (`CON`, `PRN`, `AUX`, `NUL`, `COM1`…`COM9`, `COM¹`…, `LPT1`…, `CONIN$`, with or without an
extension: `aux.rs`, `con.txt`) and names ending in a dot or a space (`a.`, `b `, `c. .`) stay valid. Under the `\\?\`
paths Kiln uses they are ordinary files of the project, read and written like any other (PROBE I1-I3 `Ok` for
each, I5/I6 listed).
- A project holding `aux.rs` (common in Rust repositories made on Unix) can be opened, saved and listed by Kiln on Windows.
- Other Windows programs may not open such files: a non-`\\?\` write to `a.` made a file `a` (PROBE I11, I12).
- Kiln creates such a file when a request saves a new one under that name. This is stated, not refused (Q-2).
### (3) Case and short names (stated as limits; no code change)
On Windows the file system folds case: `F.TXT` names `f.txt`.
- At base, `read` and `check_location` of `F.TXT` answer as for `f.txt` (PROBE I7).
- A save of `F.TXT` replaces that file. Which spelling the file has afterwards is **to be measured by the test author on
  CI** (`MEASURED:` line, Interface (5)); the sentence follows the record.
- `doc.open` recognises a document already open by its path text (`paths.md` "The rule"). So `doc.open F.TXT` after
  `doc.open f.txt` opens a second document of the same file, and the later save replaces the earlier one's content
  without notice. This is the same as T-093's macOS row 4.
- 8.3 short names alias long names the same way where a volume has them. The runner's work volume has none (PROBE I8
  `NotFound`; I9: 8dot3 creation disabled there, enabled on `C:`). Whether a short name reaches the file is **to be
  measured by the test author on CI** in a fixture on `C:` (`std::env::temp_dir()`), else unchecked.
- Not refused (Q-3). A listing shows the stored spelling (PROBE I6: names as created).
### (4) Listings
Names that fail `RelPath::new` are omitted from listings and counted in `skipped`, as now (`disk.rs` `list_dir`). On
Windows that now includes names with a character of (1). Such a name cannot be created through Win32 (PROBE I1), so it
can only come from another system (for example WSL's own names); that case is unchecked. `project.md:26`, `:99` gain a
Windows clause; the quoted words "not valid UTF-8 or contain" stay (`xtask/platform-tests.toml`, four entries).
### (5) Tests
- Implementer, in `path.rs`'s `#[cfg(test)]` module: one new test with no platform cfg on the fn. Each name of (1) is
  refused exactly when `cfg!(windows)`, and each name of (2) plus `c~1.rs`, `x\u{7f}` and `é` is accepted everywhere.
  It runs on Windows in T-073's reported step (`-p kiln-workspace --lib`).
- Test author, `crates/kiln-workspace/tests/guard_t095.rs`: no platform cfg in the file (D-245); expectations by
  `cfg!(windows)`; fixtures below `CARGO_TARGET_TMPDIR`, except the short-name case. It checks:
  - The refusal set of (1) as an equality over U+0000 to U+00FF except `/` (the separator), one at a time inside a name
    `a<c>b`: refused exactly for the set of (1) plus `\` and NUL on Windows, and for `\` and NUL alone on Linux and
    macOS.
  - Every stream and drive form of (1) through `Document` open and save: `Err` before any file changes, the directory
    snapshot unchanged.
  - (2) through `DiskFs`: write, read, list and save of `aux.rs`, `CON`, `a.`, `b `.
  - (3): read and save of `F.TXT` against `f.txt`, one `MEASURED:` line for the stored spelling after the save, one for
    the short-name read on `C:`, and two `Document`s for one file, the second save winning.

  On Linux and macOS each case asserts today's behaviour (`:` and the other characters are accepted and stored as
  given).

## Pinned
- (a) Errors: no new kind. Interface (1) is `PathError` → `invalid_path`, first in precedence.
- (b) Files: none written or read beyond what requests do.
- (c) Order: Interface (1) is part of the lexical check, before the root check.
- (d) Platform: Windows x86_64 MSVC (CI `windows-2025`; work volume `D:` with 8.3 creation disabled, `C:` enabled, PROBE
  I9). The new product lines are under `#[cfg(windows)]` (a statement in `RelPath::new`), so the Unix library is
  unchanged (reviewer: IR comparison as for T-073 and T-074).
- (e) Threat model: request paths from a client (control protocol, scripts) and names inside a hostile project. (1)
  stops a request from naming a stream, another drive or an invalid name. (2) and (3) are not escapes: every such
  name resolves inside the root (PROBE I3 `Ok` for each). Their cost is a second spelling of one file. Not defended:
  two documents of one file under case or short-name spellings (Q-3).
- (f) Tests pinning what this changes:
  - `xtask/tests/guard_t079.rs` pins `path.rs`'s three test names (`:1283-1285`): kept, the new test is added.
  - `apps/kiln-cli/tests/acceptance_t008_cli.rs:563-565` pins the lexical bullet verbatim: kept, the Windows sentence
    follows it.
  - The registry quotes four `project.md` passages ("not valid UTF-8 or contain") and three `control-protocol.md`
    passages: kept (S-4, measured).
- (g) Worked examples (base: PROBE lines; after: the model "Windows: refused iff a character of (1)"):
  - `read "f.txt::$DATA"`: base `Ok([102])` (I2), after `PathError` (`invalid_path`).
  - `read "C:x"`: base `OutsideRoot` (I2), after `PathError`.
  - `write_atomic "x<y"`: base `Io InvalidFilename` (I1), after `PathError`.
  - `write_atomic "nul.rs"` (a reserved name with an extension, as `aux.rs`): base `Ok` (I1), after `Ok`.
  - `read "F.TXT"`: base `Ok([102])` (I7), after the same.
  - Prototype (scratch clone, branch `t095`, not committed): the rule of (1) and the unit test of (5). Measured there:
    `cargo test -p kiln-workspace --lib --locked path::` gives `4 passed`, and `cargo clippy -p kiln-workspace --lib
    --profile test --locked --target x86_64-pc-windows-msvc -- -D warnings` is clean.
- (h) Evaluator: none.
- (i) Document sentences, worded to be copied. Each is "measured" (citing PROBE lines) or "unchecked", and is shown on
  the task's head by `guard_t095.rs` in the CI record before it stands (reviewer).
  - `paths.md` "Lexical refusals", new paragraph: "On Windows (T-095) a path is also refused this way if it contains
    `:`, `<`, `>`, `"`, `|`, `?`, `*` or a control character (U+0001 to U+001F), the characters Windows does not allow
    in a file name: so no request names an alternate data stream (`f.txt::$DATA` is a second name of `f.txt` there)
    or another drive (`C:x`). On Linux and macOS these characters are ordinary." Evidence: PROBE I1-I3, I10 (base); the
    refusal itself is shown by `guard_t095.rs` on the runner.
  - `paths.md`, replacing "Names that Windows gives a meaning, and case, are not yet stated for Windows (T-095).":
    "Names that Windows reserves for devices (`CON`, `NUL`, `aux.rs`, …) and names ending in a dot or a space are
    ordinary files of the project for Kiln, which reaches every file through `\\?\` paths. Other Windows programs may
    not open them (T-095; CI run 44290286460)." Evidence: PROBE I1-I3, I5, I11, I12.
  - `paths.md` "The rule", after the re-open item, a Windows bullet: "On Windows names differ only by case on the
    default file system: `F.TXT` reads and saves `f.txt` (after the save the file is named <measured>), and an 8.3 short
    name, on a volume that has them, names its long one (<measured or unchecked>). A path already open is recognised
    by its text, so `doc.open` under another spelling opens a second document of the same file, and the later save
    replaces the other's content." Evidence: PROBE I7, I8, I9; the rest to be measured by `guard_t095.rs`.
    Beside T-093's macOS row 4 (Q-5).
  - `project.md:26` and `:99`, appended: "On Windows also names containing a character refused by
    `docs/protocol/paths.md` "Lexical refusals" (such names cannot be made through Windows itself; unchecked)."
  - `control-protocol.md`, after the lexical bullet: "On Windows (T-095) also a path containing `:`, `<`, `>`, `"`, `|`,
    `?`, `*` or a control character; see `docs/protocol/paths.md`."
  - D-270: `control-protocol.md:144-147` and `project.md:82-83` name CI run 44358453526
    (`tasks/ci-runs/T-074-round-1.md`, all 15 guards) for the `awa3_*` and `awa1_*` tests. `paths.md`'s "its four
    `awa1_*` tests … in CI run 44307677646" stays: four is what that run held.

## Acceptance criteria
Local (implementer, re-run by the reviewer):
- AC-1: every existing `kiln-workspace` test passes on Linux and macOS. The product change is one `#[cfg(windows)]`
  statement in `RelPath::new`. The Linux and macOS library IR equals base's after normalising.
- AC-2: `cargo clippy -p kiln-workspace --lib --profile test --locked --target x86_64-pc-windows-msvc -- -D warnings` and
  `cargo clippy -p kiln-workspace --test guard_t095 --locked --target x86_64-pc-windows-msvc -- -D warnings` are clean.
  No C stub is needed (measured, Pinned (g)).
- AC-3: `cargo xtask no-skip` prints `35 platform test(s) registered (10 pending)`, unchanged. The variant to reject is a
  `#[cfg(windows)]` on the new test fn (an unlisted platform test).

Through CI (pending criterion, §8.1). Fail-on-base evidence on Windows is the probe record (as D-254 ruling 8 for
T-074).
- AC-W1 (Interface (1)): in `guard_t095`, the character equality and every stream and drive form `ok` on Windows.
  Fails on base: no `PROBE I0` line exists for `f.txt:s`, `f.txt::$DATA`, `C:x`, `x<y`, `q?`, `s*`, `p|q`, `dq"`,
  `ctl\u{1}`, `x/C:y`. Instead there are I2 `read "f.txt::$DATA": Ok([102])`, I1 `write_atomic "C:x": Err(OutsideRoot …)`,
  and I1 `write_atomic "x<y": Err(Io … InvalidFilename …)`. Unmeasured at base: `>` and U+0002 to U+001F; the rule
  covers them by construction.
- AC-W2 (Interface (2)): `aux.rs`, `CON`, `a.`, `b ` written, read, listed and saved on Windows. Holds at base (PROBE
  I1-I3, I5, I6), so it is a guard.
- AC-W3 (Interface (3)): `F.TXT` reads and saves `f.txt`; the two `MEASURED:` lines (spelling after the save, short name
  on `C:`) are present. The reading holds at base (PROBE I7); the spelling and the short name are unmeasured, and the
  test's first run measures them.
- AC-W4: the new unit test of `path.rs` `ok` on Windows in T-073's step.

### The pending CI criterion (v1.5 §8.1), for the approval row
The record `tasks/ci-runs/T-095-round-N.md`, from the artefacts of the task's head on `ci/candidate`, shows:
- `gate-linux`, `gate-macos`, `window-linux` and `window-probe (macos-15)` green, the test lists naming `guard_t095`'s
  tests.
- In `gate-windows`: fmt, layers, ui-commands, hygiene, schema and no-skip PASS. clippy, scorecard and test FAIL only
  at test targets T-092 or T-098 own, named in the record.
- T-073's step (`workspace-tests.txt`) names the new `path.rs` unit test `ok`.
- **T-072's general step** (`windows-tests.txt`, each `crates/kiln-workspace/tests/*.rs` run with `-- --show-output`)
  shows `==== guard_t095: exit status 0`, every `guard_t095` test `ok`, and its `MEASURED:` lines.

**One CI round expected, two if a measured sentence needs rewording.** If T-092 has merged first, `gate-windows`'s
`test` step may build `kiln-workspace`'s targets, and the criterion's "FAIL only at …" is re-read against T-092's end
state (T-092 makes its 15 targets exit 0).

## Out of scope and implementer notes
- Refusing a second spelling of an open file (case, short name), which would be a change to `kiln-commands`' table of
  open documents or an exact-case check in `disk.rs`'s walk (Q-3). Refusing reserved names or trailing dots and spaces
  (Q-2). Names that are not Unicode (T-092 ruling 3, D-269: an unpaired-surrogate name is made and skipped by listings,
  `tasks/ci-runs/T-092-probe-1.md` P-A1 to P-A6). The access list (T-074 D-254 ruling 7).
- No platform cfg on a test fn; Windows-only product lines under `#[cfg(windows)]`. Do not rename `path.rs`'s tests.

## Collisions and order
- **T-092** (fresh spec check running) touches `disk.rs` (test module), eleven `kiln-workspace/tests/*.rs` files,
  `xtask/platform-tests.toml`, `xtask/tests/acceptance_t092.rs` and `guard_t092.rs`. **No path is shared with this
  task**, which avoids the registry, `disk.rs` and every existing test file. They may run side by side (§5.2). One
  semantic contact: none of T-092's rewritten tests should use a `:` or `<>"|?*` name on Windows. grep on `4eb349d`:
  `RelPath::new("…[:<>|?*]…")` in `crates`, `apps` gives no hit (S-4); T-092's re-check confirms against its own diff.
- **T-093** (specified, low, re-check running) touches `docs/protocol/paths.md` and `project.md`, both shared. Not in
  flight together. Recommended: **T-093 first** (low, about 150,000, ready sooner), and its macOS case row (row 4) and
  `skipped` rows (2, 3) are the macOS halves of this task's (3) and (4). This task then re-checks the sentences of
  Pinned (i) against T-093's text (Q-5).
- T-094 (draft; `disk.rs`, `paths.md`): after.

## Estimate
T-074 booked 920,909 (D-271): 30 product lines, seven portable tests that made links, nine registry entries, six
document places, two CI looks and one formal round. This task: one product statement and one unit test, a guard file
by an independent test author without links (names only), five document places, no registry, no gate path, one or two
CI rounds: **about 450,000** (ceiling 1,100,000). Arm: high → independent test author.

## Questions and rulings
- **Q-1 (scope of the refusal).** A: Windows only. A Linux or macOS file named `a:b.rs` stays openable there; on Windows
  such a file cannot exist (Win32), so a project moved there loses nothing Kiln could have opened. B: every platform
  (one rule for portable projects; but a Unix file named `a:b` becomes unopenable and is skipped in listings, a Linux
  behaviour change touching `ws.*` tests). **Rec. A.**
- **Q-2 (reserved names, trailing dot or space).** A: allowed and stated (2). They are ordinary files under `\\?\` (PROBE
  I), and refusing them makes `aux.rs` in a real project unopenable. B: refused as `invalid_path` on Windows. **Rec. A.**
- **Q-3 (case and short names).** A: stated as limits (3), like T-093's macOS row 4. B: on Windows the walk refuses a
  spelling other than the stored one (`not_found`); that is `disk.rs`, shared with T-092, so in series and about
  +300,000. C: the open-documents table compares paths case-insensitively on Windows (`kiln-commands`, the same file's
  identity is still missed for short names). **Rec. A**, with a draft for B if the owner wants the stronger rule.
- **Q-4 (fail on base).** Windows behaviour passes on Unix base, so the test author's file holds only guards (as T-073
  y1, T-074) and no `acceptance_t095.rs` exists. Fail-on-base is the probe record (AC-W1). Whether `cargo xtask join`
  accepts a join with zero acceptance targets is not checked here (T-073 and T-074 each had one). A: as written; the
  director confirms the join tool. B: an `acceptance_t095.rs` reading `path.rs` as text for the `#[cfg(windows)]` rule
  (brittle, golden rule 9). **Rec. A.**
- **Q-5 (order with T-093).** A: T-093 first, then this task, beside T-092. B: this task first and T-093 re-checked.
  **Rec. A.**

### Spec check (planner, R-386, by runs in the scratch clone `/home/user/kiln-plan-T074`, branch `t095` = main `4eb349d` + the prototype of Pinned (g) and draft sentences of (i) in the three documents; not committed)
- **S-1 Shared with tasks in flight:** none by path (Collisions). No trait, type or public signature changes
  (`RelPath::new` keeps its signature; `PathError` unchanged).
- **S-2 Main's tool on the candidate (D-235):** no file under `xtask/` changes, so main's tool is the candidate's. No
  gate rule changes, so the tree uses none. Run on the prototype tree (`cargo xtask <step>` in the clone):
  - `PASS  no-skip: … 206 files scanned, 0 allowlisted skip(s), 35 platform test(s) registered (10 pending)` (the new
    unit test carries no platform cfg; the `#[cfg(windows)]` statement is in product code, not a test);
  - `PASS  hygiene: reviews: 64 of 76 … ok; session-paths: 716 tracked file(s) clean`;
  - `PASS  scorecard: 64 items: 47 verified, 0 implemented, 0 failing, 17 planned.`;
  - `PASS  layers: 11 crates, 14 internal edges, layer table holds`;
  - `PASS  ui-commands: self-test ok; 14 kiln-ui file(s) scanned, 5 allowlisted path(s)`;
  - `PASS  schema: 30 commands; audit ok; docs/commands.json current`.

  The real candidate adds `guard_t095.rs` (no platform cfg): `no-skip` must stay at the same count. The builders
  repeat the six steps with main's tool before the join.
- **S-3 Criteria of absence or limit:** (1) is an equality over the stated set. The variant to reject is `>` accepted
  (unsampled at base), and also `CON` refused (a set wider than (1)).
- **S-4 Main's tests naming each path (D-249), run on the prototype** (`grep -rlE '<path>' --include=*.rs crates apps
  xtask` on `4eb349d`, then the hits run):
  - `path.rs` → `xtask/tests/guard_t079.rs` (pins the three `path.rs` test names): **6 passed**.
  - `xtask/tests/acceptance_t079.rs` (registry; T-079's 20, untouched): **2 passed**.
  - `xtask/tests/acceptance_t074.rs` (registry; T-074's nine, untouched): **2 passed**.
  - `kiln-workspace --lib` (the prototype's new unit test and the three old `path.rs` tests among them): **59 passed**.
    The command was `cargo test -q --locked -p xtask --test guard_t079 --test acceptance_t079 --test acceptance_t074 -p
    kiln-cli --test acceptance_t008_cli -p kiln-workspace --lib`, 0 failed in every binary.
  - `paths.md`, `control-protocol.md` → `apps/kiln-cli/tests/acceptance_t008_cli.rs` (`:549-578`: the lexical bullet and
    the save bullet verbatim, `outside_root`/`not_a_file` present, no claim that links are followed outside), with the
    prototype's Windows sentence appended to the lexical bullet: **9 passed**.
  - The registry's quoted sentences (a script over `xtask/platform-tests.toml` and the three documents, whitespace
    squashed): all 7 `paths.md`, 4 `project.md` ("not valid UTF-8 or contain") and 3 `control-protocol.md` quotes
    present after the prototype edits (and `no-skip` PASS above, which checks them).
  - `project.md` → five files, comments or `project.rs`'s own docs. No main test names `guard_t095.rs`.
  - Searched for names this rule would refuse in existing tests: `grep -rn 'RelPath::new("[^"]*[:<>|?*]' --include=*.rs
    crates apps` gives no hit. JSON `"path"` strings in other crates' tests are not searched; they run on Windows only
    after T-092/T-098, whose re-checks cover them.
- **S-5 Unmeasured and marked:** `>` and U+0002 to U+001F at base; the spelling after a save under another case; a
  short name on `C:`; names with refused characters from another system. None decides a choice, so there is no
  probe; the test author's first CI run measures the two `MEASURED:` items.

## Note from T-074's review (director, D-270)
`docs/control-protocol.md:145` cites "`awa3_*`" and `docs/protocol/project.md:82` cites "`awa1_*`" for run
44307677646, which did not yet hold the two tests added later (`reviews/T-074-r2.md`, the nit). This task, which
edits the same documents, names run 44358453526 (all 15 guards; `tasks/ci-runs/T-074-round-1.md`) there.
(Done in Pinned (i), last item.)

### Rulings (director, D-272)
- **1: Windows only.** The lexical refusals apply on Windows; a Unix file named `a:b` stays openable. The documents
  say that a project moved to Windows cannot reach such a name there.
- **2: allow.** Reserved device names and names ending in a dot or a space are allowed and stated (they are ordinary
  files under the paths Kiln uses; a project with `aux.rs` stays usable).
- **3: limits.** Case and 8.3 short names are stated limits with the two CI measurements, not a stricter check on
  the walk.
- **4: one acceptance test is required.** `cargo xtask join` refuses a task with no acceptance test
  (`xtask/src/join.rs:259`), and the methodology wants one that fails on base. The test author writes
  `crates/kiln-workspace/tests/acceptance_t095.rs`: the criteria that can fail on base on every platform, which are
  the document statements this task must add (the Windows names paragraph; the two citations of D-270 naming run
  44358453526), read from the repository's files, as T-073's manifest test was (ruling y1 of T-073). The runtime
  tests stay guards in `guard_t095.rs`. The planner adds the file to `touches` and the criterion to the spec at the
  spec check.
- **5: T-093 first.** `depends_on` T-074 and T-093; this task's sentences are re-checked beside T-093's when it
  has merged. T-092 runs beside both (no shared path).

## Log
### director
<!-- end director log -->
### planner
- 2026-11-18 · draft · did: cut from T-074 at its re-specification under v1.5 · next: director (T-074 Q-1)
- 2026-11-18 · R-386 · draft · did: full specification under v1.5 against main `4eb349d`. Read: T-074's merged
  `disk.rs`, `paths.md`, `project.md`, `control-protocol.md`, `guard_t074.rs`, the records `T-074-round-1.md`,
  `T-092-probe-1.md`, D-256 to D-271, and T-092's and T-093's specs. Ran in the scratch clone (branch `t095` from the
  primary's main): the prototype of Pinned (g) (Windows clippy clean, `path::` 4 passed); main's six `xtask` steps PASS
  (S-2); main's pinning tests 6/2/2/9/59 passed (S-4); the quote script (S-4). Prototype reverted, `target/` deleted.
  No probe: no choice hangs on unmeasured behaviour; two items are to be measured by the test author on CI (Interface
  (3)). Tier high (confinement boundary; no gate path). Pinned (h) n/a. Not run: anything on Windows; the join tool
  with zero acceptance targets (Q-4) · next: director (Q-1 to Q-5), fresh spec check
<!-- end planner log -->
### implementer
<!-- end implementer log -->
### test author
<!-- end test author log -->
### reviewer
<!-- end reviewer log -->
