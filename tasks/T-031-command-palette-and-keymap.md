---
id: T-031
title: "Rank commands for a palette query by subsequence match on name and doc, bind key chords to commands with a validated default keymap, and load a user keymap that overrides it and reports every problem"
tier: medium            # ADR-0002 default: in-memory commands in two new groups; reads one file through the Vfs read path; no new dependency
state: draft
owner_role: planner
scorecard_items: [cmd.palette, keymap.default, keymap.custom]
depends_on: [T-018, T-023, T-025, T-026]   # the default keymap binds cursor.*, nav.*, edit.*, fmt.* (T-015 transitively)
touches:
  - crates/kiln-commands/src/palette.rs       # palette.* (slot from T-015)
  - crates/kiln-commands/src/keymap.rs        # keymap.* + KeymapState (slot from T-015)
  - crates/kiln-commands/tests/keymap.rs      # implementer's own integration tests
  - docs/protocol/palette.md                  # new
  - docs/protocol/keymap.md                   # new
  - docs/commands.json                        # generated
budget_tokens: 500000   # medium (v1.1, provisional)
iteration: 2
found_by: planner
origin_task: null
---

# T-031 Command palette and keymap

## Goal
The UI (iteration 2) will only dispatch commands (CHARTER §3 decision 1). It needs two headless
services, and after this task both are reachable through the CLI and the control protocol:
- **a palette**: `palette.query` ranks the registered commands for a typed query, by a pinned
  subsequence rule on the name and then the doc;
- **a keymap**: key chords bind to command names with static params. A default keymap is
  validated against the registry: every bound name exists, and its params are accepted. A user
  keymap file, read through the VFS, overrides the defaults per chord. Every problem in it is
  reported (unknown command, bad chord, bad params, duplicate), and a keymap with problems is
  never half-loaded.

No gate path. Arm: **odd** (the implementer writes the acceptance tests first on
`task/T-031-tests`).

Reading list: `tasks/T-015-*.md` (Ownership, `KeymapState`, reason `invalid_keymap`);
`docs/commands.json` (names, docs, params schemas); `tasks/T-018-*.md`, `tasks/T-023-*.md`,
`tasks/T-025-*.md`, `tasks/T-026-*.md` (the bound commands' params).

## Interface

### `palette.query` (read-only) `{query, limit?}` (`limit` 1..=1000, default 50)
→ `{items:[{name, doc, match, window}], total}`. The candidates are the commands of
`standard_registry()`, the same list `kiln-cli commands` prints. Matching is
case-insensitive: chars compare after `to_lowercase()`, char by char.
- `q` is a subsequence of `s` iff its chars occur in `s` in order. The `window` of a match in
  `s` is the length (in chars) of the shortest substring of `s` that contains `q` as a
  subsequence.
- `match`, best first: `exact` (the name equals `q`), `prefix` (the name starts with `q`),
  `name` (`q` is a subsequence of the name), `doc` (`q` is a subsequence of the doc only).
  Commands matching none are omitted.
- Order: by `match` rank; within `prefix` and `name`, by `window` (smaller first); then by name
  (bytes). `window` is the name window for `exact`/`prefix`/`name` and null for `doc`.
- An empty `query` lists every command with `match: "all"`, `window: null`, sorted by name.
- `total` counts all matches; `items` holds the first `limit`.

### Chords (exact grammar)
A chord is `part(+part)*`, compared case-insensitively.
- Every part but the last is a modifier: `ctrl`, `alt`, `shift` or `meta`, each at most once.
- The last part is the key: a single printable ASCII char other than `+` and space, or one of
  `enter tab escape backspace delete up down left right home end pageup pagedown space plus
  f1 … f12`.
- **Canonical form**: lowercase, modifiers in the order ctrl, alt, shift, meta, then the key.
  `Shift+Ctrl+S` → `ctrl+shift+s`.

### Bindings and validation
- A binding is `{chord, command, params}`, where `params` is a JSON object (default `{}`).
- If the command's params schema has a `doc` property, the binding must not contain `doc`; the
  UI adds the active document. A binding is **valid** iff the command is registered and
  `params`, with `"doc": 1` added when the schema has `doc`, pass the command's params schema.
- Problems, each `{index, chord, problem}`, with `problem` one of `invalid_chord`,
  `unknown_command`, `invalid_params`, `duplicate_chord` (the same canonical chord twice in one
  file). Several problems of one entry are reported in this order.

### Default keymap (exact; `source: "default"`)
| Chord | Command | Params |
|---|---|---|
| `ctrl+s` | `doc.save` | `{}` |
| `ctrl+w` | `doc.close` | `{}` |
| `ctrl+z` | `doc.undo` | `{}` |
| `ctrl+shift+z`, `ctrl+y` | `doc.redo` | `{}` |
| `enter` | `edit.newline` | `{}` |
| `tab` / `shift+tab` | `edit.indent` / `edit.outdent` | `{}` |
| `ctrl+/` | `edit.toggle-comment` | `{}` |
| `ctrl+shift+i` | `fmt.document` | `{}` |
| `ctrl+shift+o` | `nav.outline` | `{}` |
| `left` / `right` | `cursor.move` | `{"by":"char","forward":false}` / `{"by":"char","forward":true}` |
| `up` / `down` | `cursor.move` | `{"by":"line","forward":false}` / `{"by":"line","forward":true}` |
| `ctrl+left` / `ctrl+right` | `cursor.move` | `{"by":"word","forward":false}` / `{"by":"word","forward":true}` |
| `shift+` each of the six above | `cursor.move` | the same, plus `"extend":true` |

### Commands
| Command | Mutating | Params | Result |
|---|---|---|---|
| `keymap.list` | no | `{}` | `{bindings:[{chord, command, params, source}]}`: effective bindings, sorted by chord |
| `keymap.resolve` | no | `{chord}` | `{chord: <canonical>, binding: {command, params, source} \| null}` |
| `keymap.check` | no | `{path}` | `{problems:[…]}`: validates the file without loading it |
| `keymap.load` | yes | `{path}` | `{bindings: n, unbound: n}`: replaces the user layer |

- The keymap file (UTF-8 JSON, read with `Vfs::read`; the path is root-relative):
  `{"bindings":[{"chord":"ctrl+k","command":"doc.save","params":{}}, {"chord":"ctrl+w","command":null}]}`.
  `"command": null` unbinds that chord. Any other top-level or entry key is an error.
- The effective map is the defaults, overridden per canonical chord by the user layer. A
  `null` removes the chord.
- `keymap.load` succeeds only with zero problems. Otherwise → `rejected`/`invalid_keymap`,
  message `invalid keymap <path>: <n> problem(s); first: entry <i> (<chord>): <problem>`, and the
  user layer is unchanged.
- File errors: path refusals and `not_found`/`not_a_file`/`io_error`/`outside_root` (T-008) as
  for `doc.open`. Not UTF-8, not JSON, or the wrong shape → `invalid_keymap` (message names the
  JSON error). `keymap.resolve` with an invalid chord → `invalid_keymap` (message `invalid chord`).
- No new error kind: `invalid_keymap` comes from T-032.

`docs/protocol/palette.md` and `keymap.md` document all of this, with the examples.

## Pinned
- (a) Precedence: schema → path (`doc.open` order) → decode/JSON shape → per-entry problems
  (all collected) → load.
- (b) Files: one keymap file, read only, through the VFS. Nothing written. The user layer lives
  in `KeymapState` (memory) for the session.
- (c) Order: problems in entry order, and within an entry in the pinned order.
- (d) Lowercasing is per char; chords are ASCII.
- (e) Threat model: n/a beyond the Vfs read path (T-008 confinement applies).
- (f) Snapshots: none.
- (g) Worked examples (executed against a naive Python model of the rule, on a fixed candidate
  list):
  - Candidates `doc.save`, `doc.close`, `search.find`, `fmt.document`, `doc.disk-status`, query
    `sa`. Result: `doc.save` (`name`, window 2), `doc.disk-status` (`name`, 3), `search.find`
    (`name`, 3), then the doc-only matches by name. `doc.disk-status` sorts before
    `search.find` on the name tie-break.
  - `keymap.resolve {chord:"Shift+Ctrl+Z"}` → chord `ctrl+shift+z`, binding `doc.redo`.
  - A file with `{"chord":"ctrl+q","command":"no.such"}` and `{"chord":"ctrl+alt","command":"doc.save"}`
    → `keymap.check` gives `[{index:0, chord:"ctrl+q", problem:"unknown_command"}, {index:1,
    chord:"ctrl+alt", problem:"invalid_chord"}]`, and `keymap.load` → `invalid_keymap`.
- (h) Evaluator calls: `kiln-cli exec palette.query '{"query":"sa"}'` (`cmd.palette`);
  `kiln-cli exec keymap.list '{}'` and `keymap.resolve` (`keymap.default`); `kiln-cli run
  --root <dir>` with `keymap.check`, `keymap.load`, `keymap.resolve` on a user file
  (`keymap.custom`).

## Acceptance criteria
- AC-1 (`cmd.palette`, CLI): for ≥ 200 fixed-seed random queries (substrings, subsequences,
  case changes, non-matching strings, empty), `palette.query` equals a naive model of the rule.
  The model runs over the names and docs that `kiln-cli commands --schema` prints. Oracle:
  naive model over the public command list.
- AC-2 (`cmd.palette`, exact): the worked example's ordering holds for a query whose expected
  order the test derives from the live list by the naive model. Every listed command exists in
  `kiln-cli commands`. Oracle: naive model + exact property.
- AC-3 (`keymap.default`, control): `keymap.list` equals the pinned default table. For every
  default binding, executing its command through `kiln-cli run` with its params plus
  `{"doc":1}`, after a `doc.open` of a file, gives neither
  `unknown_command` nor `invalid_params`. Oracle: execution against the registry.
- AC-4 (`keymap.custom`, CLI): for random user files with valid entries, unbinds, overrides and
  each problem kind, `keymap.check` equals a naive validator. `keymap.load` succeeds iff there is
  no problem, and afterwards `keymap.list` equals the naive merge of defaults and user layer. A
  failed load leaves `keymap.list` unchanged. A second load replaces the first user layer.
  Oracle: naive model.
- AC-5 (`keymap.custom`, refusals): missing file → `not_found`; `../k.json` → `invalid_path`;
  `[1]` as content → `invalid_keymap`; the worked examples hold exactly. Oracle: exact value.

## Scorecard items and intended test names
| Item | Tests |
|---|---|
| cmd.palette | `kiln_cli::acceptance_t031::palette_matches_naive_model`, `kiln_cli::acceptance_t031::palette_order_on_live_list` |
| keymap.default | `kiln_cli::acceptance_t031::default_keymap_binds_registered_commands_with_valid_params` |
| keymap.custom | `kiln_cli::acceptance_t031::user_keymap_overrides_and_reports_problems`, `kiln_cli::acceptance_t031::keymap_refusals` |

## Out of scope and implementer notes
- Chord sequences (`ctrl+k ctrl+c`), platform-specific defaults (`cmd` on macOS), when-clauses,
  recently-used ranking, fuzzy scoring beyond the rule, a default keymap path; the UI.
- Your own tests: `crates/kiln-commands/tests/keymap.rs`, unit tests. Acceptance tests (odd arm)
  go first.

## Questions and rulings

## Log
Append to your role's section only, above its end marker. One entry per run:
`- <UTC time> · R-NNN · <state seen> · did: … · gate attempts: n (last: green|red <step>) ·
not run: … · missing context: none|… · next: …` (+ `QUESTION:` lines)

### director
<!-- end director log -->
### planner
- 2026-11-13 · R-060 · (new) · did: wrote the stub of R-056 as a full spec, as the director
  asked. Palette ranking uses a pinned subsequence rule with a naive model over the public
  command list. The default keymap is validated by executing each binding. User keymaps are
  all-or-nothing, with a full problem list from `keymap.check`. Tier medium (no ADR-0002 high
  criterion: the read goes through the existing Vfs path). Arm odd: it pairs with the even
  T-022 as the last-wave medium tasks, and the odd arm (T-021, T-023, T-025, T-031) now has as
  many tasks as the even one (T-018, T-020, T-022, T-026). It depends on the tasks whose
  commands the defaults bind, so it is last. The palette example was executed against a naive
  model. Pinned (e), (f) n/a. · gate attempts: 0 · missing context: none · next: director.
- 2026-11-13 · R-066 · draft · did: `invalid_keymap` comes from T-032 (D-041); transitively a dependency through T-018 → T-010 → T-032. · gate attempts: 0 · missing context: none · next: director.
<!-- end planner log -->
### implementer
<!-- end implementer log -->
### test author
<!-- end test author log -->
### reviewer
<!-- end reviewer log -->
