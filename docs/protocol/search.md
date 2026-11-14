# `search.*`: search and replace in an open document

The `search` group finds the matches of a query in one open document and replaces one or all
of them. A query is a **literal** text or a **regular expression**, case-sensitive or not, and
optionally restricted to **whole words**. The machine-readable schemas and examples are in
`docs/commands.json` (`kiln-cli commands --schema`).

Offsets are chars (Unicode scalar values), as everywhere in the protocol. Lines split on
`\r\n`, a lone `\r` and `\n`.

## Common params

| Param | Type | Default | Meaning |
|---|---|---|---|
| `doc` | integer | — | an open document's id, as in `doc.*` |
| `query` | string | — | the literal text or the regular expression |
| `regex` | bool | `false` | regex mode (else literal) |
| `case_sensitive` | bool | `true` | compare chars exactly (else by Unicode simple case folding) |
| `whole_word` | bool | `false` | keep only matches with a word boundary at both ends |

An optional param, when present, must not be `null` (`invalid_params`): omit it instead.
Unknown params are `invalid_params`.

## Matching

- **Literal mode.** The occurrences of `query` as a char sequence, scanned left to right and
  non-overlapping: after a match the scan resumes at its end (`aa` in `aaa b aa` matches
  `0..2` and `6..8`). With `case_sensitive: false`, chars compare by Unicode simple case
  folding, as the case-insensitive regex mode does (`σ` matches `Σ` and `ς`). A `\r\n` in the
  query matches only `\r\n`; a `\n` in the query also matches the `\n` of a `\r\n`. Regex
  metacharacters are plain chars in literal mode (`a*` matches the two chars `a*`).
- **Regex mode.** The `regex` crate's syntax with Unicode on, multi-line and CRLF mode:
  `^` matches at the start of the text and after every line break; `$` at the end and before
  every line break, never between the `\r` and `\n` of a CRLF; `.` matches any char except
  `\r` and `\n`. `case_sensitive: false` is the case-insensitive option. Inline flags in the
  pattern (`(?i)`, `(?-i)`, `(?s)`, `(?-m)`, …) are accepted and **override** `case_sensitive`
  and the line modes for their scope: `(?i)a` with `case_sensitive: true` matches `A`.
- **Iteration** (regex mode). Leftmost-first and non-overlapping, from char 0. After a match
  ending at `e` the search resumes at `e`; an empty match found at the previous match's end
  is discarded and the search restarts one char later. After a reported empty match at `s` it
  resumes at `s + 1`. So `a*` in `baaab` matches `0..0`, `1..4` and `5..5`.
- **Whole word** (both modes). First the matches the search reports without `whole_word`,
  then only those with a word boundary at both ends are kept. A word char is `_` or an
  alphanumeric char of any script (digits included); a match `[s, e)` has its boundaries iff
  `s` is 0 or char `s − 1` is not a word char, and `e` is the end of the text or char `e` is
  not a word char. Nothing is re-scanned after a rejected match.

The matches of one search are sorted by start, pairwise distinct in start and
non-overlapping.

Matches never overlap and are found from the start of the text; `from` applies only to
`all: false`, and there is no wrap-around.

## `search.find` (read-only)

Extra param: `limit` (integer 1 to 100,000, default 10,000). Result:
`{"matches": [{"start", "end"}, …], "total", "truncated"}`: the first `limit` matches as char
ranges, `total` the exact number of all matches, and `truncated` true iff `total > limit`.

`search.find` reports `total`, the exact number of matches, and lists at most `limit` of
them; it keeps at most `limit` ranges and the capture groups of one match at a time, however
many matches there are.

```json
{"command": "search.find", "params": {"doc": 1, "query": "Foo", "case_sensitive": false}}
// text "foo FOO fOo_x éFoo"
→ {"matches": [{"start": 0, "end": 3}, {"start": 4, "end": 7}, {"start": 8, "end": 11},
   {"start": 15, "end": 18}], "total": 4, "truncated": false}
```

With `"whole_word": true` the same query gives `0..3` and `4..7` only: `8..11` is followed by
`_` and `15..18` is preceded by `é`.

## `search.replace` (mutating)

Extra params: `replacement` (string); `all` (bool, default `false`); `from` (char offset,
default 0); `limit` (integer 1 to 100,000, default 10,000).

- `all: false` replaces the first match whose start is at or after `from` (the matches are
  found from char 0, as for `search.find`). `all: true` replaces every match; `from` is then
  ignored.
- All replacements are made on the document's current text: every match is found, and its
  replacement text computed from its captures, before anything changes. They are then one
  edit step: one `doc.undo` reverts them all.
- Result: `{"replaced", "len_chars", "ranges": [{"start", "end"}, …], "truncated"}`:
  `replaced` the exact number of replacements made, `len_chars` the length afterwards,
  `ranges` the ranges of the inserted texts in the new text, in order, for the first `limit`
  replacements, and `truncated` true iff `replaced > limit`. With `all: false` there is at
  most one range.

`search.replace` with `all: true` replaces every match, however many, as one undo step: one
`doc.undo` restores the text and one `doc.redo` re-applies it. It keeps one range and one
replacement text per match, never the capture groups of more than one match. In one
measurement, with a release build on 400,001 matches of 100 empty groups (`x` repeated
400,000 times, regex `()` repeated 100 times, replacement `-`), a replace-all, its undo step
included, grew the process's peak resident memory by about 210 bytes per match (peak of
`doc.open` then the replace-all, less that of `doc.open` alone, two runs each with
`/usr/bin/time -v`, on a 4-core machine running other builds at the same time; a measured
figure, not a bound).

The reply of `search.replace` lists the ranges of the first `limit` replacements (default
10,000) and sets `truncated` when there were more; `replaced` is the exact number of
replacements, and every match is replaced whatever `limit` is.

With no match, `search.replace` changes nothing: it adds no undo step and an open typing
group stays open.

The selection moves through each replacement by the mapping rule of `cursor.md`.

Every replacement counts in `replaced`, also one whose text equals the match. A
`search.replace` with at least one replacement that is not an empty match replaced by the
empty string ends the document's typing group and adds an undo step, even when the text does
not change (a match replaced by identical text). An empty match replaced by the empty string
is no edit: when every replacement is of this kind, nothing changes, as with no match at all
(no undo step, and an open typing group stays open). The replacements are applied, for the
selection, from the last to the first, as for `eol.normalize` (see `cursor.md`).

### The replacement text

- **Literal mode:** `replacement` is inserted verbatim; `$1` is the two chars `$1`.
- **Regex mode:** `replacement` is scanned left to right: `$$` is `$`; `${name}` (any chars up
  to the next `}`, possibly none) and `$name` (the longest non-empty run of `[0-9A-Za-z_]`)
  are references; any other `$` (including `${` with no later `}`) is literal. A name of ASCII
  digits is a group number (`$0` is the whole match), any other name a group name. A
  reference to a group of the pattern expands to the text it matched, or to nothing if it did
  not take part. **A reference to a group the pattern does not have is refused**
  (`invalid_pattern`), unlike the `regex` crate's own expansion, which expands it to nothing.
  Note that `$1a` names the group `1a`; write `${1}a` for group 1 followed by `a`.

Whether a template is refused depends only on the pattern and the template, so it is checked
before any matching: a bad template is refused even when nothing matches.

### Worked examples

| Text | Query | Replacement | Result | Text afterwards |
|---|---|---|---|---|
| `baaab` | regex `a*`, `all: true` | `-` | `replaced: 3`, `ranges` `0..1, 2..3, 4..5` | `-b-b-` |
| `a1 b2` | regex `(\w)(\d)`, `all: true` | `$2$1` | `replaced: 2` | `1a 2b` (one `doc.undo` → `a1 b2`) |
| `a1 b2` | regex `(\w)(\d)`, `all: true` | `$0-` | `replaced: 2` | `a1- b2-` |
| `a1 b2` | regex `(\w)(\d)`, `all: true` | `$3` or `$x` | `rejected` / `invalid_pattern` | `a1 b2` (unchanged) |
| `a1 b2` | literal `b`, `all: true` | `$1` | `replaced: 1` | `a1 $12` |
| `aaa b aa` | literal `aa`, `from: 1` | `x` | `replaced: 1`, `ranges` `6..7` | `aaa b x` |

## Limits

- A pattern longer than 1,000 chars (counted in chars, in both modes) is refused, before it
  is compiled.
- In regex mode a pattern with more than 100 capture groups (named or not) is refused;
  100 groups are accepted.
- A pattern the `regex` crate does not accept (syntax, its size limits, nesting deeper than
  250) is refused.

A pattern may have at most 1,000 chars and, in regex mode, at most 100 capture groups.
These limits do not bound the time a search takes: a counted repetition is short to write
but can be slow to run. Measured with a release build on texts of 1,000,000 chars
(`ab` repeated 500,000 times), `[ab]{5000}[^ab]` (15 chars, no match) took 41 to 42 s,
`[ab]{3000}` (10 chars, 333 matches) 12 s and `([ab]*)` repeated 100 times (700 chars, one match) 5.6 to 5.8 s. Kiln sets no time
limit on a search. (Elapsed time of `kiln-cli run` minus that of a `doc.open`-only script, two
runs each, on a 4-core machine running other builds at the same time; measured figures, not
bounds.)

## Refusals

Checked in this order; the first failure is reported and nothing changes (not the text, the
selection, the undo/redo history or an open typing group):

1. the params schema (`invalid_params`), which also refuses `null` for an optional param,
   unknown params, and a `limit` outside 1 to 100,000;
2. the document: `doc` must be an open document's id, else `rejected` / `no_such_document`;
3. the pattern, `rejected` / `invalid_pattern`: empty (both modes; message
   `empty search pattern`), longer than 1,000 chars (both modes; message
   `invalid pattern: the pattern has <chars> chars; the limit is 1000`), not accepted by the
   regex engine (message `invalid pattern: <the engine's message>`, not stable), or with more
   than 100 capture groups (message
   `invalid pattern: the pattern has more than 100 capture groups`);
4. `search.replace` only, the replacement template, `rejected` / `invalid_pattern`: a
   reference to a group the pattern does not have (message
   `invalid replacement: no capture group "<name>"`);
5. `search.replace` with `all: false` only: `from` beyond the end of the text is
   `rejected` / `out_of_range`. With `all: true`, `from` is ignored and not checked. A `from`
   between the `\r` and `\n` of a CRLF is not refused.

The protocol cuts every message to 200 chars (`docs/control-protocol.md`, Response).
