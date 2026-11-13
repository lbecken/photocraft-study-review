//! T-032 acceptance tests through the built `kiln-cli` binary and the committed documents
//! (AC-3, AC-7). Written from the task specification alone by the test author. This file uses
//! no new API: it compiles on base and fails there by assertion (`commands --schema` has no
//! `error_kinds`; `docs/control-protocol.md` still has the two old sentences).
//!
//! Every spawned process is killed after [`TIMEOUT`] (D-028 (8)).
#![cfg(test)]

use std::collections::BTreeSet;
use std::io::Read;
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use serde_json::Value;

const TIMEOUT: Duration = Duration::from_secs(60);

/// Run `kiln-cli args` (empty stdin); kill it after [`TIMEOUT`].
fn kiln(args: &[&str]) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_kiln-cli"))
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn kiln-cli");
    let drain = |r: Box<dyn Read + Send>| {
        thread::spawn(move || {
            let mut r = r;
            let mut buf = Vec::new();
            let _ = r.read_to_end(&mut buf);
            buf
        })
    };
    let out = drain(Box::new(child.stdout.take().expect("stdout")));
    let err = drain(Box::new(child.stderr.take().expect("stderr")));
    let deadline = Instant::now() + TIMEOUT;
    let status = loop {
        if let Some(status) = child.try_wait().expect("try_wait") {
            break status;
        }
        if Instant::now() > deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!("kiln-cli {args:?} did not finish within {TIMEOUT:?}");
        }
        thread::sleep(Duration::from_millis(5));
    };
    Output {
        status,
        stdout: out.join().expect("stdout reader"),
        stderr: err.join().expect("stderr reader"),
    }
}

fn repo_file(rel: &str) -> String {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel);
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("read {}: {e}", p.display()))
}

/// Pinned kinds and descriptions, in declaration order (Interface, "Reasons"): today's kinds
/// with today's "When" cells (`invalid_path` reworded), then the eleven new kinds.
const REJECTED: &[(&str, &str)] = &[
    (
        "invalid_path",
        "the path is not a valid root-relative path (see Paths in `docs/control-protocol.md`)",
    ),
    ("no_such_document", "no open document has this id"),
    (
        "unsaved_changes",
        "`doc.close` of a modified document without `discard`",
    ),
    (
        "not_found",
        "the file (or, for a save, its directory) does not exist",
    ),
    ("not_a_file", "the path names a directory"),
    (
        "io_error",
        "any other file-system failure (permissions, name too long, …)",
    ),
    ("decode_error", "the file is not valid UTF-8 / UTF-16 text"),
    ("out_of_range", "an offset beyond the end of the text"),
    ("inverted_range", "a range whose start is after its end"),
    ("other", "any other refusal"),
    (
        "invalid_pattern",
        "the search pattern is empty or is not a valid regular expression",
    ),
    (
        "stale_preview",
        "a file of a replace preview changed or was removed since the preview",
    ),
    ("no_such_preview", "no stored replace preview has this id"),
    (
        "tool_unavailable",
        "external tools are off for this session (the default)",
    ),
    (
        "tool_not_found",
        "the external tool's program was not found",
    ),
    (
        "tool_timeout",
        "the external tool ran longer than its limit and was stopped",
    ),
    (
        "tool_failed",
        "the external tool failed or gave no usable result",
    ),
    (
        "no_recording",
        "the replayed tool recording has no output for this request",
    ),
    ("format_error", "rustfmt could not format the text"),
    (
        "changed_on_disk",
        "the file changed or was removed on disk since it was read",
    ),
    ("invalid_keymap", "the keymap file or chord is not valid"),
];

const INTERNAL: &[(&str, &str)] = &[
    ("panic", "a command panicked (a bug in Kiln)"),
    (
        "result_schema",
        "a command's result violates its own schema (a bug in Kiln)",
    ),
    ("other", "any other internal error"),
];

/// `true` iff `want` occurs in `all` in this order (later tasks may add kinds elsewhere).
fn is_subsequence<T: PartialEq>(want: &[T], all: &[T]) -> bool {
    let mut it = all.iter();
    want.iter().all(|w| it.any(|a| a == w))
}

/// The `(code, kind)` pair of every row of the table in "Error codes and kinds".
fn table_rows(doc: &str) -> Vec<(String, String)> {
    let start = doc
        .find("### Error codes and kinds")
        .expect("section Error codes and kinds");
    doc[start..]
        .lines()
        .skip_while(|l| !l.starts_with('|'))
        .take_while(|l| l.starts_with('|'))
        .filter_map(|l| {
            let cells: Vec<&str> = l.split('|').map(str::trim).collect();
            let code = cells.get(1)?.strip_prefix('`')?.strip_suffix('`')?;
            let kind = cells.get(2)?.strip_prefix('`')?.strip_suffix('`')?;
            Some((code.to_owned(), kind.to_owned()))
        })
        .collect()
}

/// AC-3 through the CLI: `kiln-cli commands --schema` publishes `error_kinds` with the pinned
/// kinds and descriptions in order, covering every `rejected`/`internal` table row under its
/// code; the printed bytes are the committed `docs/commands.json`.
#[test]
fn schema_dump_publishes_the_generated_kind_list() {
    let out = kiln(&["commands", "--schema"]);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert!(out.stderr.is_empty(), "{out:?}");
    let stdout = String::from_utf8(out.stdout).expect("UTF-8");
    let dump: Value = serde_json::from_str(&stdout).expect("JSON");
    let top: BTreeSet<&str> = dump
        .as_object()
        .expect("object")
        .keys()
        .map(String::as_str)
        .collect();
    assert_eq!(top, BTreeSet::from(["commands", "error_kinds"]));
    let kinds = &dump["error_kinds"];
    let groups: BTreeSet<&str> = kinds
        .as_object()
        .unwrap_or_else(|| panic!("no error_kinds object: {kinds}"))
        .keys()
        .map(String::as_str)
        .collect();
    assert_eq!(groups, BTreeSet::from(["internal", "rejected"]));
    let entries = |code: &str| -> Vec<(String, String)> {
        kinds[code]
            .as_array()
            .unwrap_or_else(|| panic!("error_kinds.{code} is not a list"))
            .iter()
            .map(|e| {
                let keys: BTreeSet<&str> = e
                    .as_object()
                    .expect("entry object")
                    .keys()
                    .map(String::as_str)
                    .collect();
                assert_eq!(keys, BTreeSet::from(["kind", "when"]), "{e}");
                let kind = e["kind"].as_str().expect("kind string").to_owned();
                let when = e["when"].as_str().expect("when string").to_owned();
                assert!(!when.trim().is_empty(), "{e}");
                (kind, when)
            })
            .collect()
    };
    let rejected = entries("rejected");
    let internal = entries("internal");
    for (list, pinned) in [(&rejected, REJECTED), (&internal, INTERNAL)] {
        let pinned: Vec<(String, String)> = pinned
            .iter()
            .map(|(k, w)| ((*k).to_owned(), (*w).to_owned()))
            .collect();
        assert!(
            is_subsequence(&pinned, list),
            "pinned kinds missing, reworded or out of order:\n{list:#?}"
        );
        let unique: BTreeSet<&str> = list.iter().map(|(k, _)| k.as_str()).collect();
        assert_eq!(unique.len(), list.len(), "duplicate kind: {list:?}");
    }
    let rows = table_rows(&repo_file("docs/control-protocol.md"));
    assert_eq!(rows.len(), 23, "{rows:?}");
    for (code, kind) in &rows {
        let list = match code.as_str() {
            "rejected" => &rejected,
            "internal" => &internal,
            _ => continue,
        };
        assert!(
            list.iter().any(|(k, _)| k == kind),
            "table row {code}/{kind} is not in error_kinds.{code}"
        );
    }
    assert_eq!(
        stdout,
        repo_file("docs/commands.json"),
        "docs/commands.json differs from `kiln-cli commands --schema`"
    );
}

/// `s` with every run of whitespace replaced by one space.
fn squash(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

const NEW_PARAGRAPH: &str = "The `kind` of a `rejected` or `internal` error comes from a \
    structured reason set where the error is raised; no message text is consulted. The \
    request-level rows of this table (codes `invalid_json`, `invalid_request`, \
    `unknown_command`, `invalid_params`) and the `rejected`/`internal` kinds in `error_kinds` \
    of `docs/commands.json` (`kiln-cli commands --schema`, generated from code) are the \
    specified set; no other pair is produced. The `rejected` and `internal` rows above are the \
    kinds of iteration 0; kinds added since are listed only in `error_kinds`. Where a row's \
    'When' text differs from `error_kinds`, `error_kinds` governs; later tasks do not edit \
    table cells.";

/// AC-7: between the table and the next paragraph, `docs/control-protocol.md` holds exactly the
/// two replacement sentences (any line wrapping); the two old sentences are gone. (That nothing
/// else in the file changed is guarded in `guard_t032_cli.rs`.)
#[test]
fn protocol_doc_states_kinds_come_from_reasons() {
    let doc = repo_file("docs/control-protocol.md");
    let table_end = "| `internal` | `other` | any other internal error | echoed |\n";
    let start = doc.find(table_end).expect("last table row") + table_end.len();
    let end = doc[start..]
        .find("A request that fails validation")
        .expect("next paragraph")
        + start;
    assert_eq!(squash(&doc[start..end]), squash(NEW_PARAGRAPH));
    let all = squash(&doc);
    assert!(
        !all.contains("is currently derived by `kiln-control`"),
        "old sentence 1 left"
    );
    assert!(
        !all.contains("This table is the specified set of `(code, kind)` pairs"),
        "old sentence 2 left"
    );
}
