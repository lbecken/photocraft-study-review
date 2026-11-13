---
id: T-011
title: "Make the --memory file system usable: seed it with files, or drop the option"
tier: medium
state: draft
owner_role: planner
scorecard_items: [ws.vfs]
depends_on: []
touches:
  - apps/kiln-cli/src/main.rs
  - docs/control-protocol.md
budget_tokens: 400000
iteration: 0
found_by: evaluator
origin_task: T-002
---

# T-011 Make the --memory file system usable: seed it with files, or drop the option

## Goal
A user (or the evaluator) can run the same script against the in-memory file system and
against a disk root holding the same files, and compare responses and resulting files. Today
the `--memory` option of `kiln-cli run` / `exec` is close to inert: the in-memory file system
is always empty and no command creates a file, so every `doc.open` fails with `not_found`.
That makes the scorecard item `ws.vfs` ("in-memory implementation for deterministic tests
and a disk implementation", test names `kiln_workspace::acceptance_t002::*`, hence
`origin_task: T-002`; the CLI option itself presumably comes from T-004) impossible to
confirm from outside beyond the trivial empty case.

## Observed behaviour (evaluation R-050, main d3978dbf)
Script `vfs.jsonl` (open `a.txt`, open `../x`, open `dir`, `doc.list`, `buffer.insert`,
`buffer.text`, open `""`) run with `--memory` and with `--root <empty dir>`: identical
output, exit 3 both. Every file-system request in the `--memory` run is `not_found` or
`invalid_path`; there is no way to put a file into it. `kiln-cli control` does not accept
`--memory` at all (usage error, exit 2), which matches the docs.

## Steps to reproduce
1. `printf '{"command":"doc.open","params":{"path":"a.txt"}}\n' > s.jsonl`
2. `kiln-cli run s.jsonl --memory` → `rejected` / `not_found`. No option or command can make
   it succeed.

## Interface
Planner's choice, e.g. `--memory-from <dir>` (copy a directory into memory at start, print
or write the final in-memory files at the end), or remove `--memory` from the user-facing CLI
if it exists only for tests.

## Acceptance criteria
- AC-1: For a fixture directory and a script that opens, edits and saves files, the responses
  with the in-memory file system equal those with a disk root holding the same files.
  Oracle: byte comparison of stdout.
- AC-2: The resulting in-memory files can be inspected and equal the disk files. Oracle:
  byte comparison.

## Out of scope
Path confinement (T-008).

## Questions

## Log
### director
<!-- end director log -->
### planner
<!-- end planner log -->
### implementer
<!-- end implementer log -->
### test author
<!-- end test author log -->
### reviewer
<!-- end reviewer log -->
