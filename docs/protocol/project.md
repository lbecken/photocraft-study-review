# The project tree (`project.tree`)

The project is the session root: `--root` (or the current directory), or the in-memory file
system of `--memory` / `--memory-seed`. `project.tree` lists its directories and files as a
tree (T-019). It is read-only (`mutating: false`); a refused call leaves the session unchanged
and is not logged.

## Request and result

Params: `{"path"?: string, "limit"?: integer}`. `path` names the directory to list, relative to
the root as `docs/protocol/paths.md` defines it; without it the root is listed. `limit` is
1 to 1,000,000 (default 50,000). Unknown keys, `null` values and a `limit` outside that range
are refused with `invalid_params`.

Result:

```json
{"entries": [{"kind": "file", "path": "src/main.rs"}],
 "ignored": 0, "skipped": 0, "too_deep": 0, "truncated": false}
```

- `entries`: each listed entry with its root-relative `path` and its `kind`: `file`, `dir`,
  `symlink` or `other`.
- `ignored`: entries not listed because rule 2 or rule 3 below skips them, each counted once;
  nothing below them is visited.
- `skipped`: names not listed because they are not valid UTF-8 or contain `\`.
- `too_deep`: directories listed but not descended because of the depth bound (rule 5).
- `truncated`: true only if an entry beyond `limit` exists.

Refusals (`rejected`), all existing kinds: `invalid_path` (the `path` fails the lexical checks
of `docs/protocol/paths.md`, before the file system is consulted), `outside_root`, `not_found`,
`not_a_file` (the `path`'s location is a file or a special file, not a directory) and
`io_error`. The order for the `path` itself is `invalid_path`, `outside_root`, `not_found`,
`not_a_file`, `io_error`. An error while listing a directory the walk visits, or while reading
a `.gitignore` that is a regular file, makes the whole command fail with that error; its
message names that entry's path. Messages name root-relative paths only, never a host path,
and their wording is not stable.

## Walk rules

1. The walk starts at `path` (default: the root). `path` itself is not listed and is not
   subject to rules 2 and 3: a `path` naming an ignored directory, `target` or a `.git`
   directory is listed. Entry paths begin with `path` as given, also when `path` is reached
   through a symbolic link inside the root (`docs/protocol/paths.md`): `{"path": "self"}` with
   `self -> .` lists `self/…`.
2. Always skipped (counted in `ignored`, not descended): the directory whose path is exactly
   `target`, and every entry named `.git`, whatever its kind.
3. `.gitignore` files: a `.gitignore` in a visited directory, and the `.gitignore` of the root
   and of every directory above `path`, apply to that directory's subtree. Patterns are
   relative to the file's directory; `!` negates, a trailing `/` matches directories only,
   `**` matches any number of directories, a deeper file overrides a shallower one, and a file
   under an ignored directory cannot be re-included, as in git. A leading UTF-8 byte-order mark
   is dropped; lines that are not valid UTF-8, and lines the pattern parser refuses, are
   skipped. A `.gitignore` is read even when a pattern ignores it (it is then not listed). No
   other source is consulted: not `.git/info/exclude`, not global excludes, not `.ignore`.
   `.gitignore` files themselves are listed unless ignored.
4. Symbolic links and special files are listed with their kind and never descended or read.
5. Depth: a directory whose path has 128 segments is listed but not descended.
6. Limit: the walk stops when it would list entry `limit + 1`; `entries` then holds exactly
   `limit` entries and `truncated` is true.

For each entry the rules apply in the order 2, 3, 4, 5, 6.

Example: a root holding `.gitignore` (`*.log` and `/target`), `B.txt`, `a.rs`, `a/b.rs`,
`x.log`, `target/debug/k`, `src/lib.rs` and `src/.hidden` lists `.gitignore`, `B.txt`, `a`,
`a/b.rs`, `a.rs`, `src`, `src/.hidden`, `src/lib.rs`, with `ignored: 2` (`target` and `x.log`).

## Guarantees and limits

`project.tree` never writes, creates or changes a file.

On disk, the `path` is located as `docs/protocol/paths.md` describes: a `path` whose location
is outside the root is refused with `outside_root`, and nothing outside the root is listed or
read.

Symbolic links found in the project are listed with kind `symlink` and never followed, whatever
they point to; FIFOs, sockets and devices are listed with kind `other` and never opened, so they
cannot make the command wait. A `.gitignore` is read only when it is a regular file; a
`.gitignore` that is a symbolic link, a FIFO or a directory is listed and contributes no
patterns. A `.gitignore` whose patterns cannot be compiled contributes no patterns.

For patterns of the forms `*.x`, `/a`, `b/`, `!c`, `**/d` and `e/**`, in `.gitignore` files at
any depth, the `file` and `symlink` entries are exactly the paths
`git ls-files --cached --others --exclude-standard` lists in a fresh repository of the same
directory with no global or system configuration, apart from the root's `target` directory.

Known differences from git: `{a,b}` in a pattern matches `a` and `b` (git matches the literal
name `{a,b}`); a trailing tab is removed from a pattern (git keeps it); a directory that holds
its own `.git` is listed with its contents (git lists it as one entry `dir/`); names that are
not UTF-8 or contain `\` are not listed and are counted in `skipped` (git lists them).

`limit` bounds the number of entries listed, not the work: each directory visited is read whole
and sorted before any rule applies, and ignored entries do not count toward `limit`.

A directory whose path has 128 segments is listed but not descended, and is counted in
`too_deep`.

A directory whose location on the host has a path of 4,096 bytes or more cannot be listed, and
the command fails with `io_error`; whether that happens depends on how long the root's own host
path is.

A directory or a `.gitignore` that cannot be read (for example for lack of permission) makes
the whole command fail with `io_error`.

The listing is not a snapshot: if another process changes the tree while the command runs, the
result can mix the states before and after, and the residual limits of
`docs/protocol/paths.md` apply to each directory read.

Entries are in preorder, each directory's entries in byte order of their names; no case
folding.

In memory (`--memory`, `--memory-seed`) a directory exists only while a file lies below it, so
a directory without files is not listed, and every entry is a `file` or a `dir`.
