//! T-032 acceptance tests through the public API of `kiln-control` (AC-2, AC-6, AC-3): the
//! protocol `kind` of a `rejected`/`internal` error is `reason.as_str()` whatever the message
//! says, a panic reason reports the fixed panic message, the error object carries no `reason`,
//! and the generated kind list in `describe()` / `docs/commands.json` equals the reasons and
//! covers the table of `docs/control-protocol.md`. Written from the task specification alone by
//! the test author.
//!
//! On base this file does not compile (`RejectReason` does not exist): weak evidence by design.
//! The generated list also fails on base by assertion in `apps/kiln-cli/tests/acceptance_t032_cli.rs`.
#![cfg(test)]

use std::collections::BTreeSet;
use std::path::PathBuf;

use kiln_commands::{CommandError, InternalReason, RejectReason, standard_registry};
use kiln_control::command_error_json;
use serde_json::{Value, json};

/// Messages that, read as text, would each suggest a different kind (or none).
const MESSAGES: &[&str] = &[
    "zz unrelated",
    "no open document 1",
    "",
    "invalid relative path \"../x\": path has a '..' segment",
    "document 1 has unsaved changes; save it or close with discard",
    "cannot open a.txt: not found",
    "cannot open d: is a directory, not a file",
    "cannot save a.txt: I/O error: simulated write failure",
    "cannot open bad.bin: invalid utf-8 at byte offset 0",
    "char index 99 is out of bounds (buffer has 3 chars)",
    "range start 2 is after range end 1",
    "the command panicked",
    "command `doc.open` panicked",
    "result of `doc.open` violates its schema: x",
    "result of `doc.open` does not serialize: x",
];

fn params_variants() -> Vec<Value> {
    vec![
        json!({}),
        json!({"path": "a.txt"}),
        json!({"path": "x: not found"}),
        json!({"doc": 1}),
    ]
}

/// AC-2: for every reason and every message, the error object is exactly
/// `{"code", "kind": reason.as_str(), "message"}`; no `reason` key. AC-6: a `Panic` reason
/// reports `` command `<name>` panicked `` whatever its own message.
#[test]
fn kinds_follow_reasons_not_messages() {
    // Pinned (g), the worked example.
    assert_eq!(
        command_error_json(
            "doc.open",
            &json!({}),
            &CommandError::rejected(RejectReason::NotFound, "no open document 1"),
        ),
        json!({"code": "rejected", "kind": "not_found", "message": "no open document 1"})
    );
    assert_eq!(
        command_error_json(
            "doc.text",
            &json!({"doc": 1}),
            &CommandError::rejected(RejectReason::Other, "no open document 1"),
        )["kind"],
        "other"
    );
    for command in ["doc.open", "zz.custom"] {
        for params in params_variants() {
            for &reason in RejectReason::ALL {
                for &m in MESSAGES {
                    assert_eq!(
                        command_error_json(command, &params, &CommandError::rejected(reason, m)),
                        json!({"code": "rejected", "kind": reason.as_str(), "message": m}),
                        "{command} {params} {reason:?} {m:?}"
                    );
                }
            }
            for &reason in InternalReason::ALL {
                for &m in MESSAGES {
                    let e = CommandError::Internal {
                        reason,
                        message: m.to_owned(),
                    };
                    let message = if reason == InternalReason::Panic {
                        format!("command `{command}` panicked")
                    } else {
                        m.to_owned()
                    };
                    assert_eq!(
                        command_error_json(command, &params, &e),
                        json!({"code": "internal", "kind": reason.as_str(), "message": message}),
                        "{command} {params} {reason:?} {m:?}"
                    );
                }
            }
        }
    }
    // The size cap still applies, and changes neither code nor kind.
    let long = "z".repeat(300);
    for &reason in RejectReason::ALL {
        let v = command_error_json(
            "doc.open",
            &json!({}),
            &CommandError::rejected(reason, &long),
        );
        assert_eq!(v["code"], "rejected");
        assert_eq!(v["kind"], reason.as_str());
        assert_eq!(v["message"].as_str().map(|s| s.chars().count()), Some(200));
        assert!(v.get("reason").is_none(), "{v}");
    }
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

fn repo_file(rel: &str) -> String {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel);
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("read {}: {e}", p.display()))
}

/// AC-3: `describe()["error_kinds"]` is `ALL` (kinds, descriptions) in order; it covers every
/// `rejected`/`internal` row of the protocol table under the same code and the named kinds; the
/// committed `docs/commands.json` equals the dump.
#[test]
fn generated_kind_list_matches_reasons_and_table() {
    let d = standard_registry().expect("registry").describe();
    let entries = |code: &str| -> Vec<(String, String)> {
        d["error_kinds"][code]
            .as_array()
            .unwrap_or_else(|| panic!("error_kinds.{code} is not a list: {d:#}"))
            .iter()
            .map(|e| {
                let keys: BTreeSet<&str> = e
                    .as_object()
                    .expect("entry object")
                    .keys()
                    .map(String::as_str)
                    .collect();
                assert_eq!(keys, BTreeSet::from(["kind", "when"]), "{e}");
                (
                    e["kind"].as_str().expect("kind").to_owned(),
                    e["when"].as_str().expect("when").to_owned(),
                )
            })
            .collect()
    };
    let rejected = entries("rejected");
    let internal = entries("internal");
    let want_rejected: Vec<(String, String)> = RejectReason::ALL
        .iter()
        .map(|r| (r.as_str().to_owned(), r.description().to_owned()))
        .collect();
    let want_internal: Vec<(String, String)> = InternalReason::ALL
        .iter()
        .map(|r| (r.as_str().to_owned(), r.description().to_owned()))
        .collect();
    assert_eq!(rejected, want_rejected);
    assert_eq!(internal, want_internal);
    let keys: BTreeSet<&str> = d["error_kinds"]
        .as_object()
        .expect("object")
        .keys()
        .map(String::as_str)
        .collect();
    assert_eq!(keys, BTreeSet::from(["internal", "rejected"]));

    let rejected_kinds: BTreeSet<&str> = rejected.iter().map(|(k, _)| k.as_str()).collect();
    let internal_kinds: BTreeSet<&str> = internal.iter().map(|(k, _)| k.as_str()).collect();
    let rows = table_rows(&repo_file("docs/control-protocol.md"));
    assert_eq!(rows.len(), 23, "the table keeps its 23 rows: {rows:?}");
    let mut checked = 0;
    for (code, kind) in &rows {
        let set = match code.as_str() {
            "rejected" => &rejected_kinds,
            "internal" => &internal_kinds,
            _ => continue,
        };
        assert!(
            set.contains(kind.as_str()),
            "table row {code}/{kind} not in error_kinds"
        );
        checked += 1;
    }
    assert_eq!(checked, 13, "10 rejected and 3 internal rows");
    for kind in [
        "invalid_path",
        "no_such_document",
        "unsaved_changes",
        "not_found",
        "not_a_file",
        "decode_error",
        "out_of_range",
        "inverted_range",
    ] {
        assert!(rejected_kinds.contains(kind), "{kind}");
    }
    for kind in ["panic", "result_schema", "other"] {
        assert!(internal_kinds.contains(kind), "{kind}");
    }

    let committed = repo_file("docs/commands.json");
    let parsed: Value = serde_json::from_str(&committed).expect("docs/commands.json is JSON");
    assert_eq!(parsed, d, "docs/commands.json is stale");
    assert_eq!(
        committed,
        format!("{}\n", serde_json::to_string_pretty(&d).expect("pretty")),
        "docs/commands.json is not the byte-exact dump"
    );
}
