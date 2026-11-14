---
iteration: 2
run: R-221                # booked to iter-2; syntax.parse re-evaluated in R-222 (same clone)
main_sha: 0704b24a72c4ba245ff90b4a40720630125a9787
interfaces: [cli, control]
confirmed_items: [syntax.parse, ws.open-project, search.literal, search.regex, search.replace, search.whole-word, syntax.incremental, syntax.highlight, syntax.brackets, syntax.outline, nav.goto-line, nav.goto-symbol, nav.workspace-symbol, ws.path-confinement, cmd.schema, control.errors, text.lines, text.positions, edit.selection, io.mixed-eol, undo.grouping, undo.undo, undo.redo, undo.roundtrip, ws.dirty-tracking, text.insert, text.delete, ws.documents, ws.vfs, cli.exec, cli.run, cmd.log-replay, control.session, control.protocol, cli.schema, cmd.registry, io.utf8, io.crlf, io.encoding, io.bom, io.save-atomic]
rejected_items: []
defects_filed: [T-050, T-051, T-052, T-053]
not_evaluated: []
---

# Evaluation, iteration 2

Of the 13 `implemented` items, all 13 are confirmed (`syntax.parse` was first not evaluated in run
R-221 because its corpus lies under `crates/`; after the director's ruling it was evaluated in run
R-222: 33/33 `valid` corpus files parse without error nodes, NEW-2 keeps a documentation remainder). All 28 items verified before this stage passed
their regression checks and are confirmed again, including the three changed in this stage
(`ws.path-confinement`, `cmd.schema`, `control.errors`). 0 rejected. 4 findings (NEW-1 … NEW-4) and
7 questions. In this clone `cargo xtask scorecard` fails (NEW-3; cause found by the director: the checkout's location). No UI exists, so
`ui-snapshot` was not used. Comparisons against independent models: 450 random trees vs git,
8,000 search/replace checks, 840 third-party files for highlight (6.78 M chars) and brackets
(315,580 checks), 6,870 incremental trees, 450 outlines, 3,600 goto-line checks; 0 mismatches
attributable to Kiln.

## Setup
- Clean clone at `0704b24a` in the director's scratch directory (`git rev-parse HEAD` →
  `0704b24a72c4ba245ff90b4a40720630125a9787`). `cargo build -p kiln-cli` (debug):
  `Finished dev profile [unoptimized + debuginfo] target(s) in 27.23s`.
- `cargo xtask scorecard` (as uid 0, and again under `capsh --drop=cap_dac_override,cap_dac_read_search`),
  exit 1 both times:
  `FAIL  scorecard: 5 scorecard problems:` — all five `ws.path-confinement` tests
  (`acceptance_t036_cli::unreadable_root_is_refused_at_start`,
  `acceptance_t036_workspace::unreadable_root_is_refused_by_new`,
  `guard_t036_removal_cli::root_made_unreadable_after_start_stays_protected_through_control`,
  `acceptance_t037_cli::reopen_whose_walk_fails_inside_is_io_error`,
  `guard_t037_cli::reopen_does_not_need_the_file`) "fails". The file it wrote says
  `64 items: 27 verified, 13 implemented, 1 failing, 23 planned.` (I saved that copy as
  `work/scorecard-generated.md` and restored the clone's `docs/scorecard.md`, which the command
  had rewritten.)
- `kiln-cli commands --audit` → `audit ok: 30 commands, all examples validate`;
  `kiln-cli commands --schema | cmp - docs/commands.json` → identical.
- `cargo build -p kiln-cli --release` (`Finished release profile [optimized] target(s) in 1m 19s`)
  only for the measured figures under "Known and already filed".
- Read only what the brief allows: docs/scorecard.md, scorecard/items/*.toml, control-protocol.md,
  docs/protocol/*.md, docs/commands.json, `--help` and `commands` output, the evaluation template,
  evaluations/iter-1.md (not iter-0.md), iter-1 drivers, METHODOLOGY §9, file names of `tasks/`
  (last T-048); in R-222, by the director's ruling, also the corpus files and manifests under
  `crates/kiln-syntax/tests/corpus/` (nothing else under `crates/`). Third-party data used as oracles: `git` 2.43.0, Python 3.13 `jsonschema` 4.26.0, the
  case-folding table of `regex-syntax 0.8.11` and third-party `.rs` files in the cargo registry
  (`~/.cargo/registry/src/*/`), none of them Kiln's.
- Drivers in `work/drivers/` (`k2.py` helpers; every process under `timeout`); fixtures under
  `work/fx/`, outputs `work/out-*.txt`. Machine: 4 cores, uid 0; permission cases under
  `drivers/nodac.sh` (uid 0 without `CAP_DAC_OVERRIDE`/`CAP_DAC_READ_SEARCH`).

## Items

### ws.open-project
- Claimed: open a directory as a project and list its files as a tree, honouring `.gitignore` and
  skipping `target/` (`docs/protocol/project.md`).
- What I did:
  - `drivers/tree.py <seed> 150` for seeds 1, 2, 3: random trees (depth ≤ 3, names from
    `a b c d e x.log y.rs A B.txt z.tmp e.rs target sub _`), `.gitignore` files in random directories
    holding 1–4 patterns of only the guaranteed forms (`*.log *.rs *.tmp /a /b /sub /target b/ d/ e/
    sub/ !c !x.log !y.rs !e.rs !a **/d **/c **/x.log e/** sub/** a/** /e/**`), random symlinks
    (`<dir>/lnk -> ..`, `abs.log -> /etc`). Compared the `file`+`symlink` entries of
    `project.tree {}` with `git ls-files -z --cached --others --exclude-standard` after `git init` in
    the same directory (`GIT_CONFIG_GLOBAL=/dev/null GIT_CONFIG_NOSYSTEM=1 HOME=/nonexistent`), minus
    the root's `target/` directory; checked preorder / byte order of names; and compared the whole
    result of `--root` with `--memory-seed` of the same files (before any symlink or `.git` existed).
  - `drivers/tree_edge.py`, `drivers/tree_edge2.py`, `drivers/tree_fuzz.py 7 600`: the documented
    differences, 30 forms outside the guarantee, `.gitignore` as symlink / FIFO / directory, kinds of
    special files, non-UTF-8 and `\` names, every rule of `path`, the depth bound, `limit`, the
    4,096-byte host path, 100,100 entries, and permission cases under `drivers/nodac.sh`
    (`capsh --drop=cap_dac_override,cap_dac_read_search`).
- What I observed:
  - Random comparison: `seed=1 cases=150 git_mismatches=0 memory_disk_differences=0`, same for seeds
    2 and 3 (450 cases, 0 mismatches, 0 order errors, 0 memory/disk differences). The first run of
    seed 1 showed 3 "mismatches" that were my model's error (I had dropped a *file* named `target`;
    Kiln lists it, as rule 2 says only the *directory* `target` is skipped).
  - Each documented difference reproduced exactly: `{a,b}` ignores `a` and `b` (git: only `{a,b}`);
    a trailing tab, VT, U+00A0 are stripped (git keeps them; a trailing space and `a\ ` agree with
    git); `f[[:digit:]]` ignores nothing (git ignores `f1`); `***/z` leaves `z` listed (git ignores
    it); a nested repository `inner` (made with `git init`) → Kiln `inner`, `inner/f`, git `inner/`;
    names `bad\xffname`, `back\slash` and directory `baddir\xfe` → not listed, `skipped: 3`.
  - Agrees with git also for: `a/b`, `f?`, `f[ab]`, `f[!c]x`, `f[a-c]`, `#a`, `\#b`, `\!x`, `a/**/b`,
    `d/` + `!d/f` (not re-included), `d/*` + `!d/f` (re-included), `*` `!*/` `!*.rs`, a deeper
    `!keep.log`, `/a` in a subdirectory, `*/x`, BOM, CRLF lines, a line of invalid UTF-8, `.gitignore`
    ignoring itself (still applied, not listed), trailing `\`, `.*` + `!.gitignore`, `A` vs `a`, `/`,
    leading space, `**` + `!keep`, `a/**/`, `é*`, a `.gitignore` that is a symlink (no patterns, as
    git ≥ 2.32).
  - Differences from git that the list does not name → NEW-1 (`[` alone, negated and `/`-holding
    bracket expressions, unbalanced and empty braces).
  - Kinds: `d` dir, `dangling`/`dlink`/`rootlink`/`target` (a symlink named `target` at the root)
    `symlink`, FIFO, `/dev/null`-like char device and a socket `other`; `d/.git` (a file) counted in
    `ignored`; no request blocked (a `.gitignore` FIFO: Kiln returned at once; `git ls-files` hung on
    it until my `timeout 10`).
  - `path`: `ign` (ignored), `target`, `.git` (when it exists) are listed; `self` (`-> .`) lists
    `self/…`, `self/self/sub` works; `out` (`-> ../outside`), `out/missing`, `rootlink` (`-> /`), a
    dangling link to `/nonexistent-dir` and `dang/x` → `outside_root`; `out/back/sub` (outside and back
    in) → `outside_root` while `doc.open out/back/sub/f` succeeds, as documented; `f`, `fifo`, `sub/x`
    (files) → `not_a_file` `cannot list f: not a directory`; `sub/f/x` → `not_found`; `missing`,
    `.git` (absent) → `not_found`; `""`, `/abs`, `sub/`, `a//b`, `../x`, `out/../te-path/sub` →
    `invalid_path`; `path: null` and an unknown key → `invalid_params`. A root `.gitignore` `sub/x`
    applies when `path` is `sub`.
  - Depth: a chain of 131 `d` directories → deepest entry has 128 segments, `too_deep: 1`, the
    127-segment `…/g` listed. With `path` = a 128-, 129- or 131-segment directory, that directory is
    descended (it is the start) and its children of 129/130/132 segments are listed; `too_deep` counts
    the next directory (Q1).
  - `limit` on 5 listable entries (+1 ignored): 1 → 1 entry `truncated: true`; 4 → 4, `true`; 5 → 5,
    `false`; 6 → 5, `false`; `0`, `1000001`, `null`, `-1`, `1.5`, `"3"` → `invalid_params`.
  - Host path of 4,115 bytes (20 segments of 200 `x`) → `io_error`, message
    `cannot list the project: xxxx…` cut at 200 chars; no host path in any message.
  - Without DAC capabilities: directory `a` mode 0300 → `io_error`
    `cannot list the project: a: I/O error: Permission denied (os error 13)`; `b/.gitignore` mode 0000
    → `io_error` naming `b/.gitignore` (and `path: "c"` beside it works); the root `.gitignore`
    unreadable with `path: "c"` → `io_error` naming `.gitignore` (root's file applies above `path`);
    a `.gitignore` inside a directory of mode 0600 (listable, not searchable) → `io_error`; a file of
    mode 0000 is listed (not read). Root of mode 0311, 0000, 0100 → exit 2
    `kiln-cli: bad --root: root `<as given>`: I/O error: Permission denied (os error 13)`; 0500 works.
  - Timing (debug): 100,100 entries with `limit: 1000000` in 1.61 s; default limit → 50,000 entries,
    `truncated: true`, 0.85 s.
- Verdict: **confirmed** (run, exec; disk and memory).
- Inert check: `path` and `limit` both change the result; `ignored`, `skipped`, `too_deep`,
  `truncated` each took non-zero values in some case above.

### search.literal
- Claimed: literal search, case-sensitive and case-insensitive, agrees with a naive scan.
- What I did: `drivers/search.py <seed> 80 25` for seeds 1–4 (320 random texts of 0–30 tokens
  from `a b A B é É σ Σ ς k K K(U+212A) s S ſ ß ẞ _ 1 ␠ \r \n \r\n . - $ ( * 中`, 25 random
  queries each, 60 % regex / 40 % literal, random `case_sensitive`, `whole_word`, `limit`; for
  replaces also `all`, `from`, a template; every request in a `--memory-seed` session). The literal
  model (`drivers/rx.py`) is a left-to-right non-overlapping scan; case-insensitive equality is "same
  orbit" in the simple-case-folding table of the third-party crate `regex-syntax 0.8.11`
  (`src/unicode_tables/case_folding_simple.rs` in the cargo registry; not Kiln source). Literal
  queries are substrings of the text (case-randomised within their orbit) or random tokens.
  Hand cases (`drivers/search_edge.py`, output `work/out-search-edge.txt`).
- What I observed: `seed=1 texts=80 checks=2000 mismatches=0`, likewise seeds 2, 3, 4: 8,000
  checks (find, replace, refusal), 0 mismatches. Hand cases: `aa` in `aaa b aa` → `0..2`, `6..8`;
  case folding on `σ Σ ς ß ẞ ss K k K ı I i İ`: `σ` → all three sigmas; `ß` → `ß`, `ẞ` (not `ss`);
  `k` → `K k K`; `i` → `I i` only; `ı` and `İ` only themselves (simple folding); `\r\n` in the query
  matches only the CRLF, `\n` also the `\n` of the CRLF; `""` → `invalid_pattern`
  `empty search pattern`; 1,000-char query accepted, 1,001 chars (also 1,001 × `é`) →
  `invalid pattern: the pattern has 1001 chars; the limit is 1000`; `limit` 0 / 100001 / `null`
  params / unknown key / `all` on `search.find` → `invalid_params`; `limit` 100000 accepted;
  `doc: 9` with an empty query → `no_such_document` first. `search.find` changed neither the
  selection nor `doc.info` (`can_undo:false`). With 1,000,000 matches of `x`: `limit:1` → 1 range,
  `total:1000000`, `truncated:true`, peak RSS 21,108 KB (debug).
- Verdict: **confirmed** (run, memory seed).
- Inert check: `case_sensitive`, `limit`, `regex` all change results.

### search.regex
- Claimed: regex search with char-offset ranges; invalid patterns reported.
- What I did: the regex part of the same runs: random ASTs (literals, `.`, classes with ranges
  and negation, concatenation, `(?:…|…)` with an empty alternative, capture groups, greedy and lazy
  `* + ? {m,} {m} {m,n}` over non-nullable bodies, `^`, `$`) rendered to `regex` syntax and matched
  by my own backtracking interpreter (leftmost-first, the iteration rule of search.md, CRLF-mode
  anchors as documented, case folding as above). Hand cases: anchors on `a\r\nb\rc\nd`, inline flags,
  limits.
- What I observed: 0 mismatches in the 8,000 checks above (the first run gave 1 mismatch caused by
  my renderer writing a repetition of a repetition without a group, `X+??`; fixed in the model).
  Anchors: `^` → 0, 3, 5, 7 and `$` → 1, 4, 6, 8 (never at 2, between `\r` and `\n`); `.` skips `\r`
  and `\n`; `(?s).` matches all 8 chars; `(?-m)^` → only 0, `(?-m)$` → only 8; `(?-R)$` → 2, 6, 8;
  `(?i)a` with `case_sensitive:true` on `aA` → 2 matches, `(?-i)a` with `false` → 1. `a*` in `baaab`
  → `0..0`, `1..4`, `5..5`. 100 groups `()`×100 accepted; 101 groups and `(?P<n>a)` + 100 groups →
  `invalid pattern: the pattern has more than 100 capture groups`; class nesting `[`×250 accepted,
  ×251 → `invalid_pattern` (the message is `invalid pattern: regex parse error:\n    [[[…` cut at
  200 chars, so the engine's reason is not visible; Q2); `a{1}`×200 accepted, ×251 refused likewise;
  `a{1000}{1000}`, `\w{1000}`, `(?:\w{100}){100}` → `invalid pattern: Compiled regex exceeds size
  limit of 10485760 bytes.`; `\p{Nope}`, `(`, `\R` → `invalid pattern: regex parse error: …`.
- Verdict: **confirmed** (run, memory seed).
- Inert check: `regex: true` changes the meaning of metacharacters (`a*` literal vs regex).

### search.replace
- Claimed: replace one or all matches as one undoable operation, with capture groups.
- What I did: in `search.py` every replace was followed by `doc.text`, `doc.undo`, `doc.text`,
  `doc.redo`, `doc.text`, `doc.undo`, `doc.info`, compared with the model (text afterwards,
  `replaced`, `ranges` in the new text, `truncated` for `limit`, `len_chars`, whether an undo step
  was added: not when every replacement is an empty match replaced by `""`; text restored by one undo;
  redo re-applies; `can_undo:false` at the end). Templates were random mixes of text, `$n`, `${n}`,
  `$$`, `${`, `$na`, `$x`, `$`, references one past the last group. Hand cases: the six worked
  examples, refusal order, typing-group rules, the selection mapping, a replace-all of 400,001 matches.
- What I observed: 0 mismatches (as above). The six worked examples reproduce exactly (`-b-b-`
  with ranges `0..1, 2..3, 4..5`; `1a 2b` and one undo → `a1 b2`; `a1- b2-`; `$3` and `$x` →
  `invalid replacement: no capture group "3"` / `"x"`; `a1 $12`; `aaa b x` with range `6..7`). `$1a`
  → `no capture group "1a"`, `${}` → `no capture group ""`, `${1}a` accepted. Order: `doc 9` before
  the pattern; a bad template is refused when nothing matches and before `from: 99`; `from: 99`
  with `all:true` is ignored; `from` between `\r` and `\n` accepted. Typing group: no match → the
  next typed char joins the open group (one undo removed `a`, `b`, `c`); empty matches by `""` →
  `replaced: 5` and no step; `x`→`x` → a step (undo sequence `axyz`, `axyz`, `xyz`). Selection
  `3..5` on `ab ab ab`, replace-all `ab`→`XYZ` → `4..7`, undo → `3..5` (my mapping-rule model gives
  the same); `a\nb` with selection `1..2`, `a`→`x\r` → `1..3` (CRLF snap). 400,001 matches of 100
  empty groups on `x`×400,000 (`limit:1`): `replaced:400001`, `ranges` 1 entry, `truncated:true`,
  undo back to 400,000 chars; peak RSS 100,476 KB vs 18,672 KB for `doc.open` alone (debug), i.e.
  about 209 bytes per match, the same order as the documented 210.
- Verdict: **confirmed** (run, memory seed).
- Inert check: `all`, `from`, `limit`, `replacement` each change the result.

### search.whole-word
- Claimed: the whole-word option restricts matches to identifier boundaries.
- What I did: `whole_word` in 25 % of the random queries (filter after matching, word char = `_`
  or alphanumeric); hand cases.
- What I observed: 0 mismatches. `ab a_b a1 é a-b ab`: `a` → only `12..13`; `ab` → `0..2`,
  `16..18`; `a\w*` (regex) → 5 matches; `a|ab` (regex) → only `12..13` (leftmost-first `a` at 0 is
  rejected and nothing is re-scanned); `aab ab` with `ab` → only `4..6`; `x` next to U+0301 (combining
  mark, not alphanumeric) is a word, next to U+0660 (Arabic-Indic digit) is not; the documented
  example `foo FOO fOo_x éFoo` → `0..3`, `4..7`.
- Verdict: **confirmed** (run, memory seed).
- Inert check: changes results (above).

### syntax.parse
- Claimed: parse Rust with tree-sitter; every file of the pinned compiling corpus parses without
  error nodes.
- What I did (run R-222, after the director's ruling that the files under
  `crates/kiln-syntax/tests/corpus/` — `rust/`, `rust-gaps/`, `rust-invalid/`, `MANIFEST.toml`,
  `corpus-rustc-run.toml` — are fixture data I may read; nothing else under `crates/` was read):
  `drivers/corpus.py rustc`: parsed `MANIFEST.toml` (45 entries: set `valid` 33 in `rust/`, set `gap`
  2 in `rust-gaps/`, set `invalid` 10 in `rust-invalid/`) and checked each entry's `bytes`, `lines`
  (`\n` count) and `fnv1a64` against the file (my own FNV-1a 64); copied the 45 files into
  `work/fx/corpus/` at their manifest paths, opened each with `doc.open` in one `kiln-cli run
  --root work/fx/corpus` session and asked `syntax.tree`; then ran the command recorded in
  `corpus-rustc-run.toml` myself (`rustc --edition 2024 --crate-type lib --emit metadata
  --crate-name corpus --cap-lints allow`, with the local `rustc 1.97.0 (2d8144b78 2026-07-07)`, the
  recorded toolchain) on a copy of each file. Before the ruling (R-221), and still reported here:
  `drivers/syntax.py <seed> <n> <maxbytes>` on third-party Rust files of the local cargo registry
  (113 crates, sources that compiled as dependencies; seed 1: 40 files ≤ 20 KB; seeds 2, 3: 400 files
  ≤ 60 KB each), each opened in a `--memory-seed` session: `syntax.tree` twice, then highlight and
  brackets (below), then `doc.info`. Hand cases (`drivers/syntax_edge.py`): the documented tree
  example, the size limit, refusal order, read-only.
- What I observed:
  - Corpus (output `work/out-corpus.txt`): 45 files on disk = 45 manifest entries, none extra or
    missing; `manifest mismatches (bytes/lines/fnv) []`; the fnv of every entry of
    `corpus-rustc-run.toml` equals the manifest's.
  - `set=valid files=33 has_error=0` — every `valid` file opens (32 `lf`, `crlf_lines.rs.txt`
    `crlf`) and gives `has_error: false`, `errors: []`. rustc: all 33 exit 0 (recorded 0).
  - `set=gap files=2 has_error=2`: `safe_foreign_fn.rs.txt` one `error` node 237..241 and
    `safe_foreign_static.rs.txt` one `error` node 171..175, each exactly the text `safe` (`pub safe
    fn abs(…)`, `pub safe static errno_like: i32;`), as the manifest's `gap` notes say
    ("tree-sitter-rust 0.24.2 does not parse the edition-2024 `safe` qualifier …"). rustc: both exit
    0 (recorded 0), i.e. they compile.
  - `set=invalid files=10 has_error=10`, each with 1–2 `error`/`missing` nodes; rustc: all 10 exit 1
    (recorded 1), first errors e.g. `expected expression, found }`, `mismatched closing delimiter: ]`,
    `unterminated double quote string`.
  - My rustc run agreed with the recorded exit code for 45 of 45 files.
  - Earlier substitute run, 840 third-party files: 10 have `has_error: true` (autocfg `src/version.rs` and `src/lib.rs`, syn-3.0.10
    `src/custom_punctuation.rs` and `src/error.rs`, anyhow `src/ensure.rs`, winnow
    `examples/arithmetic/test_parser_ast.rs`, serde_json `tests/ui/parse_expr.rs`, and zerocopy-derive
    `output_tests/expected/into_bytes_enum.repr(C).expected.rs` and `….repr(i16).expected.rs`, the
    latter drawn in both seeds, so 9 distinct files). Of those I looked at: `autocfg-1.5.1/src/version.rs`
    669..673 is `try!(` (2015-edition code), `syn-3.0.10/src/error.rs` 3912..3940 is
    `struct _Test where Error: Send + Sync;`, `anyhow` 3520 is inside a `macro_rules!` matcher
    (`$stack:tt $bail:tt (~$($fuel:t…`); the zerocopy and serde_json files are, by their paths, test
    fixtures.
  - Every first call `mode: "full"`, second `unchanged` with otherwise identical result; `doc.info`
    afterwards `dirty:false`, `can_undo:false`.
  - `fn f() {}` → exactly the documented `sexp`, `node_count: 10`; after `doc.insert` of `" "` at 8 →
    `incremental`.
  - Size limit on `a;`×524,288 (1,048,576 chars): `syntax.tree` accepted (`node_count: 1572865`),
    highlight, match-bracket, `nav.outline` accepted; after one more char all of them →
    `too_large` (`document 1 has 1048577 chars; syntax commands accept at most 1048576`, for `nav.*`
    `… nav.outline and nav.goto-symbol accept at most 1048576`); `doc: 9` → `no_such_document`
    first; `too_large` before `out_of_range` (`offset: 99999999`) and before `inverted_range`; after
    deleting the char → accepted again with `mode: "unchanged"`.
- Verdict: **confirmed** (run, disk), reading "the pinned compiling corpus" as manifest set
  `valid` (33 files, 0 with error nodes). The two `gap` files also compile and do have error nodes;
  the manifest says so, but the item's title and syntax.md do not (NEW-2, remaining part).
- Inert check: `has_error`, `errors`, `node_count` and `mode` each varied.

### syntax.incremental
- Claimed: incremental reparse after an edit yields the same tree as parsing from scratch.
- What I did: `drivers/incr.py <seed> 40 40` for seeds 2, 3, 4, 5 and `incr.py 1 10 30`
  (170 sessions): a third-party file (300–6,000 bytes) opened, then 30–40 random steps (insert of
  snippets such as `(`, `}`, `"`, `//`, `/*`, `fn x() {}`, `\r\n`, `é`, `😀`, `r#"`; delete; replace;
  undo), each followed by `syntax.tree` (and every third by `syntax.highlight`). The same script was
  run twice (once with a `doc.text` before every tree, to learn each text); every distinct text was
  then opened as its own document in a fresh session and parsed from scratch; compared `has_error`,
  `errors`, `sexp`, `node_count` and highlight spans; mode rule checked: first call `full`,
  `unchanged` iff the text equals the text of the previous successful call, never `incremental`
  for a tree with errors.
- What I observed: `seed=2 sessions=40 trees=1640 tree_diff=0 hl_diff=0 … mode_bad=0`, likewise
  seeds 3, 4, 5 (1,640 trees each) and seed 1 (310 trees): 6,870 trees, 0 differences from the fresh
  parse, 0 mode violations, 0 differences between the two runs of each script; 5,288 of the trees
  had errors. Hand cases: an edit that gives errors → `full`; the next edit that removes them →
  `incremental`; undoing back to the original text after other calls → `incremental` (not
  `unchanged`: the last successful call saw another text); a refused `syntax.highlight` between an
  insert and its deletion → next call `unchanged`; `nav.outline` in between → `unchanged`.
- Verdict: **confirmed** (run, memory seed).
- Inert check: `mode` takes all three values.

### syntax.highlight
- Claimed: highlight spans for Rust cover the text without overlap.
- What I did: in `syntax.py` (840 files, 6.78 M chars) the full highlight and 3 random
  sub-ranges per file: partition (sorted, contiguous, non-empty, adjacent kinds differ, exact cover,
  `[]` for empty ranges), clipping (sub-range = clip of the full spans), and each char's kind against
  my model of rules 1–7 (`drivers/sx.py`), computed from the published `sexp` (named nodes, fields)
  plus the text: anonymous keyword leaves are identifier-like runs not covered by a named token.
  Same on the 6,870 trees of `incr.py`. Hand cases (`out-syntax-edge.txt`).
- What I observed: `hl_kind_mismatch_files=0 partition_bad=0 clip_bad=0` on all 840 files. On the
  damaged texts of `incr.py` my model disagreed only where it cannot know the leaf: 20,518 chars
  directly inside an `ERROR` node and not covered by any named node (e.g. text after an unclosed
  `/*`, where Kiln says `plain` and my model `keyword` for `pub`), which I excluded and counted; and
  one other run (Q3). Hand cases match the rules: the documented example spans exactly; range
  `10..17` → `et`:keyword, ` s: `:plain, `u`:type; `Self::new()` → `Self` plain, `new` function;
  `-> Self` → `type` (a `type_identifier`); `s.f()`, `a::b::<u8>()`, `g::<T>()`, `x.y::<Z>()`,
  `r#fn()`, `union()` → callee `function`; `let h = k::<u8>;` → `k` plain (a `generic_function`
  that is not called); `m!(q)` → `m` plain; `b'x'`, `c"s"`, `br#"r"#`, `'\n'` single strings;
  `1_000u32`, `2.5e3` numbers; `///`, `//!`, nested `/* /* */ */` comments; `macro_rules!` keyword
  including `!`; `union U` keyword, `let union = 1` plain; `default fn` keyword; `&mut self` →
  `mut`, `self` keyword; `'static` plain; `async`, `.await`, `try`, `yield` keyword, `gen`, `dyn` (in
  `dyn Tr;` as a statement), `auto`, `safe`, `raw` plain; a range from 8 to 9 or 9 to 10 of a CRLF
  (between `\r` and `\n`) accepted; `start == end` and the empty document → `spans: []`. Refusals:
  `start: null` → `invalid_params`; `start: 10` on 9 chars → `out_of_range` `start 10 is beyond the
  end of the text (9 chars)`; `5..2` → `inverted_range`; `doc: 9` with `5..2` → `no_such_document`.
  Read-only: after `doc.insert` `a`, `syntax.tree`, `syntax.highlight`, `syntax.match-bracket`,
  `doc.insert` `b`, one `doc.undo` removed both chars; selection unchanged.
- Verdict: **confirmed** (run, memory seed).
- Inert check: `start`/`end` change the result; all 7 kinds occur.

### syntax.brackets
- Claimed: find the matching bracket for `()`, `[]`, `{}` ignoring brackets in strings and comments.
- What I did: `syntax.match-bracket` at every bracket char of the 840 files (plus 20 random
  offsets and the text length each), compared with my model (`sx.brackets_model`: bracket chars
  that are anonymous leaves — not inside a comment, string, char literal or other named token —
  grouped by their innermost named node, paired per kind by a stack); and at up to 60 bracket chars
  of every text of `incr.py`. Hand cases: the documented examples.
- What I observed: `br_checks=7064`, `139004`, `169512` with `br_mismatch=0` (315,580 checks). On
  the damaged texts 0 mismatches except brackets inside `ERROR` text not covered by a named node
  (3,814 excluded, e.g. `Ref::rea/*d(source)` after an unclosed comment: Kiln `null`, my model a
  pair). Documented examples: offset 7 and 24 → `{open 7, close 24}`; `fn f() { g(1; }` at 10 →
  `null` (the `)` is `MISSING`); `fn a() { (` at 9 → `null`; `) ( ( ) ] [` → only 4↔6; offset = length
  → `null`; offset > length → `out_of_range` `offset 31 is beyond the end of the text (30 chars)`;
  `offset: -1` → `invalid_params`; CRLF chars → `null`.
- Verdict: **confirmed** (run, memory seed).
- Inert check: returns information.

### syntax.outline
- Claimed: the outline lists `fn`, `struct`, `enum`, `trait`, `impl` and `mod` items with names and ranges.
- What I did: `drivers/outline.py 1 150 2`: 150 third-party files ≤ 30 KB, each also with 2 randomly
  damaged copies (450 texts, 217 with errors); `nav.outline` compared with a model computed from the
  published `sexp` by nav.md's rules (listed when every node strictly between the item and the root is
  `source_file`, `declaration_list`, `mod_item`, `impl_item`, `trait_item` or `ERROR`; names from the
  `name` field, for `impl` the `trait` and `type` fields with white space collapsed; depth = listed
  ancestors). `syn` is not available to me as a tool (Q4), so this is a check of the stated rules,
  not of the `syn` sentence. Hand cases: the five examples and the error-recovery example of nav.md.
- What I observed: `texts=450 items=4897 mismatch=0`. The five examples and the error-recovery
  example reproduce exactly (e.g. ex. 5 → `impl 'Foo < T , U >' 0..28 11..25 0; impl 'Send for X'
  29..48 44..45 0`). Others: `unsafe impl<T> Send for Vec /* c */ <T>` → name
  `Send for Vec /* c */ <T>` (the comment is part of "the text of its type"); `impl ! Sync for Y` →
  `Sync for Y`; `impl<'a> Tr<'a,\tB> for &'a [u8]` → `Tr<'a, B> for &'a [u8]`; `union`, `type`,
  `static`, `macro_rules!` not listed; nested `mod a { mod b { mod c { fn d` → depths 0, 1, 2, 3; a
  `fn` inside a `fn` body not listed; `pub(crate) enum E` starts at 0 (visibility included).
- Verdict: **confirmed** (run, memory seed).
- Inert check: returns information.

### nav.goto-line
- Claimed: go to line:column, clamped to the document.
- What I did: `drivers/nav_edge.py 1 300`: 300 random texts (`a é 😀 \r \n \r\n ␠ \r\r \n\r`), 12
  random `{line, col?}` each (line up to 2 past the end, col 0–6 or absent) against a naive model
  (lines split by hand; line clamped to the last, col to the content length). Hand cases.
- What I observed: `goto-line seed=1 texts=300 checks=3600 mismatches=0`. Documented example
  `ab\r\ncdef\n` reproduced (heads 8, 9, 1). `line` and `col` = 18446744073709551615 → clamped
  (`head 9`); `line: -1`, `col: null`, missing `line` → `invalid_params`; `doc: 9` →
  `no_such_document`. Typing `a`, `nav.goto-line`, typing `b` → one undo removes only `b`;
  `doc.info` unchanged otherwise.
- Verdict: **confirmed** (run, memory seed).
- Inert check: `col` and `line` change the result.

### nav.goto-symbol
- Claimed: go to a symbol of the current file chosen from the outline.
- What I did: in `outline.py`, for each of the 450 texts: `nav.goto-symbol` for every index and one
  past the end, then `cursor.get`, `nav.outline`, `doc.info`, `syntax.tree`. Hand cases.
- What I observed: `goto_bad=0 readonly_bad=0`: for every index, `item` equals outline item
  `index` and the selection is `name_start..name_end`; one past the end → `out_of_range` (`index 5
  is beyond the outline of document 1 (5 items)`); the outline after the calls is unchanged; the
  first `syntax.tree` afterwards still reports `mode: "full"` (nav.* did not touch the cache).
  Example 1 `{index: 1}` → anchor 32, head 35. `index` = u64::MAX → `out_of_range`; `doc: 9,
  index: 99` → `no_such_document`; `index: -1` / missing → `invalid_params`. A refused goto-symbol
  between two typed chars leaves the group open (one undo removed both).
- Verdict: **confirmed** (run, memory seed).
- Inert check: `index` changes the result.

### nav.workspace-symbol
- Claimed: find a symbol by name across the project's Rust files.
- What I did: `drivers/wsym.py 1 40 30`: 40 third-party files copied into `""`, `src/`, `src/a/`,
  `lib/`, `ign/` (git-ignored), `target/`, `x/target/`, some re-encoded (UTF-8 BOM, UTF-16 LE/BE BOM)
  or made invalid, plus `notes.txt`, `UPPER.RS`, a file named `.rs`, a directory `dir.rs/` with
  `inner.rs`, a symlink `link.rs`, a FIFO `pipe.rs`, `big.rs` (1,048,592 chars) and `edge.rs`
  (1,048,575 chars). Model: the `file` entries ending in `.rs` of `project.tree {limit: 1000000}`,
  each file's outline from a separate session's `nav.outline`, lowercase containment, rank (exact,
  prefix, contains), walk order, item start, `line`/`col` of `name_start`, `limit`, `truncated`,
  `skipped` in walk order. 30 queries with random `limit`. Hand cases in `ws-hand` and `ws-open`
  (under `drivers/nodac.sh`) and `ws-million`.
- What I observed: `seed=1 rsfiles=28 searched=27 skipped=[{"path": "big.rs", "reason":
  "too_large"}] queries=30 mismatches=0`. Hand: the documented example gives exactly the documented
  order (`a.rs fn foo`, `b/c.rs mod foo`, `a.rs struct FooBar`, `b/c.rs fn barfoo`) plus `u16.rs fn
  foo16`; `bad.rs` (invalid UTF-8) → skipped `decode_error`, `noread.rs` (mode 0000, no DAC) →
  `io_error`; a `.rs` symlink, `UPPER.RS` and `z.txt` not searched; `.rs` searched; a `.rs` file
  130 directories deep not searched, `too_deep: 1`. An open, edited, unsaved `a.rs` is searched with
  its current text (`foo` at `name_start` 23, line 1); an open `z.txt` is not. Open documents whose
  files were then made unreadable → still searched from the document; replaced by a link to outside
  or by a directory → not searched (no longer a `file` entry), not in `skipped`. `query: ""` →
  `invalid_pattern` `the search pattern is empty`; `limit` 0 / 10001 / `null`, missing `query` →
  `invalid_params`. 1,000,002 entries (1,000 directories of 999 files, `a.rs` first, `zz.rs` last):
  `walk_truncated: true`, `first_one` found, `last_one` not; 16.6 s for this and a
  `project.tree {limit: 1000000}` (debug), which listed exactly 1,000,000 entries, `truncated: true`.
- Verdict: **confirmed** (run, control; disk and memory).
- Inert check: `limit` changes the result; `skipped`, `walk_truncated`, `too_deep` took non-zero
  values.

### Regression checks of the 28 items verified before this stage

The iteration-1 drivers were copied to `work/drivers/` unchanged and run with
`KILN_BIN=<clone>/target/debug/kiln-cli KILN_EVAL_WORK=work/legacy`; new ones are `regress.py`,
`confine2.py`, `kinds.py`, `examples2.py` (outputs `work/out-*.txt`).

### ws.path-confinement
- Changed in this stage; several checks.
- Root at start (`confine2.py` §1, under `nodac.sh`): mode 0700 and 0500 work; 0311, 0300, 0100,
  0000 are refused by `run`, `control` and `exec` alike: exit 2, stdout empty, stderr
  `kiln-cli: bad --root: root `<root as given>`: I/O error: Permission denied (os error 13)`; a
  relative `--root r300` is named `r300`; a `--root` symlink to a 0300 directory and a root whose
  parent is not searchable are refused the same way. Mode 0400 (listable, not searchable) is
  accepted at start, and `doc.open a.txt` then fails `io_error` (consistent with "cannot open for
  reading"). The iteration-1 driver `unreadable.py` now stops at its first step because its 0300 root
  is refused at start (`kiln-cli: bad --root: …`), i.e. the documented change.
- After start (`confine2.py` §2, §4): root made 0300 → `doc.open`, edit, `doc.save`, re-open work;
  `project.tree`, `nav.workspace-symbols` → `io_error` `cannot list the project: (root): I/O error:
  Permission denied`; root made 0600 → open, re-open and save → `io_error`; a directory above the
  root made 0600 → first open, re-open, save, `project.tree` and `nav.workspace-symbols` all →
  `outside_root`, as paths.md says; restored → the save works (`Qa`).
- Re-open of an open path (`confine2.py` §3): deleted, replaced by a directory, by a FIFO, by a link
  inside, made 0000, made undecodable → `already_open: true` without reading; replaced by a link
  outside, parent replaced by a link outside → `outside_root`; a 41-link chain → `io_error`
  `too many levels of symbolic links`; a directory on the way made 0600 → `io_error`; a chain out and
  back in → `already_open: true`; root renamed away → `outside_root`, back → `already_open: true`;
  the edited text (`EDIT same`) kept; the outside file unchanged. iteration-1 `paths2.py` S12 (the
  NEW-1 / T-037 case) now gives `outside_root` and the document keeps `EDIT reopen`; `rootid.py`
  gives `outside_root` for re-opens in every replaced-root state and `already_open: true` once the
  root is back.
- `paths.py` (49 probes): 17 `outside_root`, `UNCHANGED_AFTER_OPENS True`; `paths2.py` S1–S13 as in
  iteration 1 (`FINAL secret: SECRET`, no file created outside).
- **Confirmed**, with NEW-3 (five of the item's own tests fail when `cargo xtask scorecard` runs as
  uid 0, so the generated scorecard shows the item `failing`).

### cmd.schema
- Changed in this stage.
- `examples2.py`: all 48 examples reproduced through `run` (`examples=48 reproduced=48
  result_schema_valid=48 invalid=0`, jsonschema 4.26.0), and all 57 refusal examples refuse with
  their kind (`refusal_examples=57 reproduced=57`), each identical on disk and in memory
  (`memory_vs_disk_differences=0`); every refusal example's kind is in its command's `refusals`;
  `published-not-declared [] declared-not-published []`. Commands whose declared kinds have no
  refusal example: `doc.open` (`io_error`, `outside_root`), `doc.save` (`not_found`, `not_a_file`,
  `io_error`, `outside_root`), `nav.workspace-symbols` (`not_found`, `not_a_file`, `io_error`,
  `outside_root`), `project.tree` (`io_error`, `outside_root`) — all disk-only, as the protocol says;
  but see NEW-4. `--audit` → `audit ok: 30 commands, all examples validate`. Example `files` come in
  two shapes (Q6). **Confirmed.**

### control.errors
- Changed in this stage.
- `kinds.py`: 1,634 requests (every command × values generated from its params schema, on disk
  and in memory, after opening a CRLF document): 81 distinct (command, code, kind), `internal=0`,
  `rejected kinds not declared by the command: []`. `regress.py`: hostile lines through `control`
  (bad ids, not JSON, `[1]`, `{}`, numeric command, unknown key, params array, wrong-case command,
  lone surrogate, 100,000 `[`, invalid UTF-8, a final line without `\n`, a 400-char path → `io_error`
  with a 200-char message) → 19 responses for 19 non-blank lines, each with the documented
  code/kind, blank lines unanswered, rc 0. control-protocol.md says `error_kinds` is the specified
  set and lists only kinds some command reports: matches the binary (above). **Confirmed.**

### text.lines
- `positions.py 1 200` → `cases=16554 mismatches=0`; `positions.py 5 200` → `cases=15584
  mismatches=0`. **Confirmed** (both).

### text.positions
- The same run as `text.lines` just above covers this item (its claim is checked by the same comparisons). **Confirmed.**

### edit.selection
- `selection.py 1 20 100` → `checks=8138 mismatches=0`; `selection.py 8 20 150` → `checks=12197
  mismatches=0` (each step compared: text, selection, eol counts, normalisation results, typing
  groups). **Confirmed** (all three).

### io.mixed-eol
- The same run as `edit.selection` just above covers this item (its claim is checked by the same comparisons). **Confirmed.**

### undo.grouping
- The same run as `edit.selection` just above covers this item (its claim is checked by the same comparisons). **Confirmed.**

### undo.undo
- `roundtrip.py 11 10 120` → `ok=10 mismatches=0`; `regress.py`: delete `1..3` of `abcd`, undo,
  redo, two typed chars, undo (both gone), redo, undo, undo → `dirty:false`, `can_undo:false`,
  `can_redo:true`, a further undo → `undone:false`. **Confirmed** (all four).

### undo.redo
- The same run as `undo.undo` just above covers this item (its claim is checked by the same comparisons). **Confirmed.**

### undo.roundtrip
- The same run as `undo.undo` just above covers this item (its claim is checked by the same comparisons). **Confirmed.**

### ws.dirty-tracking
- The same run as `undo.undo` just above covers this item (its claim is checked by the same comparisons). **Confirmed.**

### text.insert
- `regress.py`: insert at 5 of 4 chars → `out_of_range`; offset −1 → `invalid_params`; delete `3..1`
  → `inverted_range`; `0..99` → `out_of_range`; delete `1..3` → `removed: "bc"`. **Confirmed** (both).

### text.delete
- The same run as `text.insert` just above covers this item (its claim is checked by the same comparisons). **Confirmed.**

### ws.documents
- `regress.py`: two documents; dirty `doc.close` → `unsaved_changes`; `discard:true` closes; reopen
  gets id 3; `doc.list` sorted and per-document dirty flags. **Confirmed.**

### ws.vfs
- `vfs.py 1 100 60` → `cases=100 steps=60 identical=100 different=0`; and every example/refusal
  example identical on disk and in memory (above). **Confirmed.**

### cli.exec
- `exec buffer.insert {"offset":2,"text":"X"} --text héllo --memory` → `{"len_chars":6}` rc 0;
  offset 9 → rc 2, `{"code":"rejected","kind":"out_of_range",…}` on stderr; `nosuch` →
  `unknown_command` rc 2; `exec doc.open '{' --root /nonexistent` → `invalid_json` first; `exec
  project.tree {}` and `exec nav.workspace-symbols {"query":"a"}` → results rc 0; `exec syntax.tree
  {"doc":1}` → `no_such_document` rc 2 (each exec is a fresh session). **Confirmed.**

### cli.run
- `regress.py`: a 19-request script using the new commands (`search.replace`, `syntax.*`,
  `nav.*`, `project.tree`, `eol.normalize`, saves of CRLF, UTF-16 LE and BOM files, an invalid
  `search.find`, blank lines) run in two copies at different depths: both rc 3, stdout
  byte-identical (20 lines), files identical. Its `--log` (19 lines: the read-only `syntax.*`,
  `nav.*`, `project.tree` requests included, the refused one not) replayed on a fresh copy: rc 0,
  log byte-identical, files identical (`src/a.rs` = `fn lapha() {}\r\nstruct Beta;\r\n`, `u16.txt`
  `fffe5a00…`, `bom.txt` `efbbbf6869210a`). Edit-and-save through `control` in `confine2.py` §2
  (`a.txt` became `Xa`). **Confirmed** (all three).

### cmd.log-replay
- The same run as `cli.run` just above covers this item (its claim is checked by the same comparisons). **Confirmed.**

### control.session
- The same run as `cli.run` just above covers this item (its claim is checked by the same comparisons). **Confirmed.**

### control.protocol
- `regress.py`: ids `-9223372036854775808` and `18446744073709551615` echoed; `2^64`, `1.0` →
  `bad_id`, id `null`; `"s"` with a trailing `\r` answered; blank and white-space lines unanswered;
  final line without `\n` answered. **Confirmed.**

### cli.schema
- `commands --schema` byte-identical to `docs/commands.json`; examples as under cmd.schema.
  **Confirmed.**

### cmd.registry
- `examples2.py`: `{"doc":1,"zzz":1}` to all 30 commands → `invalid_params` 30/30; text `abc\n` and
  selection unchanged afterwards. **Confirmed.**

### io.utf8
- `regress.py`: 11 files (LF, CRLF, CR, mixed, none, empty, BOM, BOM only, UTF-16 LE with `中😀`,
  UTF-16 BE, NUL) opened and saved unedited → all byte-identical; `line_ending` `lf crlf cr lf none
  none lf none crlf lf lf`, only `mixed` mixed; `fn a() {}\n// \xff` → `decode_error` `invalid utf-8
  at byte offset 13`; `ff fe 61 00 62` → `invalid utf-16le at byte offset 4`; edited UTF-16 LE saved
  as `fffe5a006800e900…`, edited BOM file as `efbbbf6869210a`, CRLF kept. **Confirmed** (all four).

### io.crlf
- The same run as `io.utf8` just above covers this item (its claim is checked by the same comparisons). **Confirmed.**

### io.encoding
- The same run as `io.utf8` just above covers this item (its claim is checked by the same comparisons). **Confirmed.**

### io.bom
- The same run as `io.utf8` just above covers this item (its claim is checked by the same comparisons). **Confirmed.**

### io.save-atomic
- `regress.py` with strace: `openat(AT_FDCWD, "$R/.kiln-tmp-0", O_WRONLY|O_CREAT|O_EXCL|O_CLOEXEC,
  0600)`, `fsync(4)`, `rename("$R/.kiln-tmp-0", "$R/f.txt")`, `fsync(4)`; mode 0751 kept; save into
  a `chattr +i` directory → `io_error … Operation not permitted (os error 1)`, file `yx` intact,
  document dirty, no temporary file. **Confirmed.**

## Wired but inert
- Every one of the 30 commands of `kiln-cli commands` returned information or changed something
  observable in the runs above; every parameter of the 10 new commands changed a result (`path`,
  `limit` of `project.tree`; `start`/`end`; `offset`; `query`, `regex`, `case_sensitive`,
  `whole_word`, `limit`, `replacement`, `all`, `from`; `line`, `col`; `index`; workspace `limit`).
- Documented limits all enforced at their stated boundary: pattern 1,000 chars, 100 groups, regex
  nesting 250, document size 1,048,576 chars (all `syntax.*`, `nav.outline`, `nav.goto-symbol`,
  workspace `skipped`), depth 128 segments, `limit` ranges (1–1,000,000; 1–100,000; 1–10,000), the
  walk's 1,000,000 entries, a host path over 4,096 bytes.
- Documented refusals never produced: `nav.workspace-symbols` `not_found` and `not_a_file` (NEW-4).
- The selection is now also set by `nav.goto-line` / `nav.goto-symbol` and mapped through
  `search.replace`, but still no command reads it as input (iteration-1 Q2; Q7 below).

## Defects filed
Director's note, added when the file was published: the evaluator named its findings NEW-1 to NEW-4; the director filed them as T-050 (NEW-1, with Q1), T-051 (NEW-2), T-052 (NEW-3) and T-053 (NEW-4, with Q2 and Q6 and earlier follow-ups); Q5 is added to T-048. The answers to the questions are in D-141. The corpus ruling behind run R-222 is in D-141.

| Task | Item | Summary | Suspected origin task |
|---|---|---|---|
| NEW-1 | ws.open-project | `.gitignore` differences from git not in project.md's list: bracket expressions that match `/` (`[!a]`, `[/]`), a lone `[`, unbalanced and empty braces | T-019 (named in project.md) |
| NEW-2 | syntax.parse | the corpus exists only under `crates/` (resolved for this run by the director's ruling; item confirmed); remaining: the title "every file of the pinned compiling corpus" is wider than the behaviour — 2 `gap` files compile and have error nodes, and only MANIFEST.toml says so | T-020 (test names in docs/scorecard.md); T-016 (named in MANIFEST.toml) |
| NEW-3 | ws.path-confinement | `cargo xtask scorecard` fails in this clone: five of the item's tests need every directory above the checkout to be searchable by another identity (director's observation: `/tmp/claude-0` is 0700 root); undocumented precondition; the generated scorecard shows the item `failing` | T-036 / T-037 (test names in docs/scorecard.md) |
| NEW-4 | nav.workspace-symbol, cmd.schema, control.errors | `nav.workspace-symbols` publishes refusals `not_found` and `not_a_file` that no request I could make produces | T-023 (test names in docs/scorecard.md) |

### NEW-1 — undocumented differences from git in `.gitignore` matching
- Reproduction (`drivers/tree_fuzz.py 7 600`, fixture `fx/tfz`; and a direct run on `fx/tdstar`
  holding `x/a`, `xa`, `a`, `x/y/a`, `q/x/a`; git run as `git ls-files --others --exclude-standard`
  with `GIT_CONFIG_GLOBAL=/dev/null GIT_CONFIG_NOSYSTEM=1 HOME=/nonexistent`):
  - `.gitignore` = `*[!a]*a` → git lists `.gitignore a q/x/a x/a x/y/a`; `project.tree` lists only
    `.gitignore a` (the negated class matches the `/`). `x[/]a` → git lists `x/a`, Kiln ignores it.
    In the fuzz: `*[!a]**[a-b]` → Kiln also ignores `aa/b c/b/x d/a x/a x/b x/y/a`.
  - `.gitignore` = `[` with a file `[` (`tree_edge.py` case `bad-pattern`) → git lists `[`, Kiln
    ignores it (`ignored=2`).
  - Braces beyond `{a,b}`: `*}`, `{*`, `{**`, `***[a-b]}` → git ignores the file `{a}`, Kiln lists
    it (the line seems refused); `b*{}` → Kiln ignores `b ba aa/b x/b c/b/x`, git lists them; `{x,}`
    → Kiln ignores `x/…` and `c/x`; `{}b[a-b]` → Kiln ignores `ba`.
  - Result of the fuzz: `seed=7 patterns=600 differing=8`, all 8 of the kinds above.
- Expected: project.md "Differences from git found so far (this list is not complete)". None of
  these forms is in the guaranteed set (`*.x`, `/a`, `b/`, `!c`, `**/d`, `e/**`), so the guarantee
  holds; the list is narrower than the behaviour. Either list them or match git.

### NEW-2 — the corpus claim of `syntax.parse` could not be evaluated (resolved by ruling; a doc gap remains)
- Resolution: the director ruled (R-222) that the corpus files and manifests are fixture data the
  evaluator may read; with that, `syntax.parse` was evaluated and confirmed (above). What remains:
  - The item title says "every file of the pinned compiling corpus parses without error nodes".
    The corpus holds 35 files that compile with the recorded command (33 `valid` + 2 `gap`, my rustc
    run: exit 0 for all 35), and the 2 `gap` files parse with one `error` node each (on `safe`). Only
    `MANIFEST.toml` (under `crates/`) says that the claim is about set `valid` and why the gaps are
    excluded; syntax.md and the scorecard item do not mention the gap set, and nav.md names set
    `valid` only for the outline. The title is wider than the behaviour by these 2 files.
  - The corpus is still reachable only under `crates/`, so a future evaluator needs the same ruling.
- Original reproduction (R-221): the item title is "every file of the pinned compiling corpus parses without error
  nodes"; nav.md names the corpus `crates/kiln-syntax/tests/corpus/`; the brief forbids reading
  anything under `crates/` and any test. Nothing in `docs/`, `kiln-cli --help` or
  `commands --schema` lists the corpus files or exposes a command that parses them.
- Expected: METHODOLOGY §9 ("For each `implemented` item it tries the behaviour through a public
  interface"). Substitute evidence: 840 third-party files parsed, 10 with errors (see syntax.parse).
- Suggested: name set `valid` (and the documented gaps) in the item title or in syntax.md, and
  either publish the corpus location as evaluator-readable data or move it outside `crates/`.

### NEW-3 — scorecard fails as uid 0
- Reproduction: in the clean clone, `cargo xtask scorecard` (uid 0) → exit 1,
  `FAIL  scorecard: 5 scorecard problems:` followed by the five tests listed in Setup; the same
  under `capsh --drop=cap_dac_override,cap_dac_read_search -- -c 'cargo xtask scorecard'`; the file
  it writes says `64 items: 27 verified, 13 implemented, 1 failing, 23 planned.` with
  `ws.path-confinement` `failing`. The committed docs/scorecard.md says 28 verified.
- What the tests name (unreadable root refused at start, root made unreadable after start, re-open
  whose walk fails inside → `io_error`, re-open that does not need the file) holds through the binary
  under `nodac.sh` (`confine2.py` §1–§3). I did not read or run the tests individually.
- Director's observation (not mine; reported to me for R-222): the director ran one of the five tests
  in this clone. The tests re-run themselves under another identity, and that spawn fails with
  `precondition: cannot spawn the child run for mode 0311: Permission denied (os error 13)`,
  because the clone lies under `/tmp/claude-0`, which is mode 0700 and owned by root; the primary
  checkout lies under directories of mode 0755 and the same tests pass there. So the cause is the
  location of the checkout, not uid 0 as such.
- Expected: the scorecard is the gate's statement of status; it should not depend on where the
  checkout is, or this precondition of the five tests (every directory above the checkout must be
  searchable by the other identity) should be documented (I was not given docs/development.md).
  The finding stays: an undocumented precondition of five tests on the checkout's location.

### NEW-4 — `nav.workspace-symbols` refusals that never occur
- Reproduction: `kiln-cli commands --schema` → `nav.workspace-symbols` `refusals: [not_found,
  not_a_file, io_error, invalid_pattern, outside_root]`; nav.md "then an error of the walk with
  `project.tree`'s kinds (`not_found`, `not_a_file`, `outside_root`, `io_error`)". The command has no
  `path`; a root renamed/removed gives `outside_root` (paths.md, observed in `confine2.py` §4 and
  iteration-1 `rootid.py`), a root that is a file or missing is refused at start. Over all my runs
  (`kinds.py` 1,634 requests, hand cases) the command produced only `invalid_params`,
  `invalid_pattern`, `io_error` and `outside_root`. I did not try changing a directory during the
  walk, the only way I can see to reach `not_found` / `not_a_file`.
- Expected: control-protocol.md "each command's entry lists the kinds it can report as its
  `refusals`"; brief: documented refusals that never occur. If they can occur only in a race, say
  so (as paths.md does for other residual cases); otherwise drop them.

### Known and already filed (observed, not filed again)
- T-045 (no time bound on parsing): release `syntax.tree` on `#!` repeated: 16,384 chars 1.57 s,
  1.44 s; 32,768 chars 6.07 s, 6.46 s (doc: 1.10 s and 4.49 s; same order, same fourfold growth).
- T-046 (no time bound on a search): release, `ab`×500,000: `[ab]{3000}` 15.40 s, 13.57 s (doc 12 s,
  333 matches, also 333 here); `([ab]*)`×100 8.20 s, 8.06 s (doc 5.6–5.8 s). Same order.
- T-048 (no cap on spans): release, 1,048,576 chars of `1;`: the `syntax.highlight` reply held
  1,048,575 spans; peak RSS 2,124,528 KB (doc 2,073 MiB); `a;` 707,048 KB (doc 689 MiB).
- T-044 (link targets ending in `/.`) and T-047 (cost of `.gitignore` files): not exercised.

## Not evaluated
- None. (`syntax.parse` was not evaluated in R-221; after the director's ruling it was evaluated in
  R-222 and confirmed.)
- Not run, and why:
  - Changes made *during* a request (paths.md residual limits; project.md / nav.md "not a
    snapshot"): needs racing a request; not attempted.
  - An ordinary (non-root) identity: the binary and the fixtures lie below `/tmp/claude-0`
    (mode 0700, owner root) and `/root` is 0700, so a `setpriv` uid could reach neither; I ran
    permission cases as uid 0 without `CAP_DAC_OVERRIDE`/`CAP_DAC_READ_SEARCH` (`drivers/nodac.sh`,
    `CapEff 000001fffefffff9`), so owner bits apply. Group/other bits, a root owned by another user,
    and the scorecard as a non-root user were not tested.
  - `syn` as an oracle for the outline (not available as a tool here). The outline was not run on
    the corpus's `valid` set either (the ruling covered `syntax.parse`).
  - `ui-snapshot`: no UI.
  - Release-build memory figure of nav.md (338 MiB) and the 23 s workspace timing, and the 41–42 s
    search figure: not repeated (time).

## Questions for the director
- **Q1 (ws.open-project):** the depth bound counts segments of the entry's path from the root, but
  a `path` that itself has 128 or more segments is descended (it is the start, not "listed"): with
  `path` = a 131-segment directory, an entry of 132 segments is listed and `too_deep` is 0.
  Intended reading of rule 5?
- **Q2 (search.regex):** for a long refused pattern the 200-char message cap keeps only the
  engine's echo of the pattern, so the reason (`nest limit`, …) is cut off (`[`×251 →
  `invalid pattern: regex parse error:\n    [[[[…`). Messages are documented as not stable; worth
  moving the reason first?
- **Q3 (syntax.highlight):** in one damaged text, `mod` used as the separator of a macro repetition
  (`$(#[$outer_derive])mod m {*` inside `macro_rules!`) is `plain`, while the sexp shows no named
  node over it (so it is an anonymous leaf whose kind I cannot see). Is a separator token a keyword
  leaf `mod` or another kind?
- **Q4 (syntax.outline):** the outline's agreement with `syn` on corpus set `valid` cannot be
  checked by me (no `syn` tool; the corpus ruling came after the outline runs); I checked the stated structural rules instead. Enough?
- **Q5 (syntax.tree):** at the size limit the reply is one line with a 34–37 MB `sexp`
  (`a;`/`1;` repeated), and `errors` has no cap either. Does T-048 (spans) cover `syntax.tree`'s
  reply too, or is this separate?
- **Q6 (cmd.schema):** in `docs/commands.json`, `examples[].files` are `[path, text]` pairs but
  `refusal_examples[].files` are `{path, text}` objects (55 vs 27 entries). No document I may read
  describes the format. Intended?
- **Q7 (edit.selection):** the selection is now set by `nav.goto-*`, but still no command uses it
  as input (see iteration-1 Q2). Still as planned?
