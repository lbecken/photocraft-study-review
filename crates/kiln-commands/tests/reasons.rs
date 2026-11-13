//! T-032 implementer tests: every refusal carries its structured reason where it is raised, and
//! `describe()` publishes the reasons as `error_kinds` (ADR-0004).
#![cfg(test)]

use kiln_commands::{
    CommandError, Context, InternalReason, Registry, RejectReason, standard_registry,
};
use kiln_workspace::{MemoryFs, RelPath};
use serde_json::{Value, json};

fn reg() -> Registry {
    standard_registry().expect("standard registry builds")
}

fn ok(r: &Registry, ctx: &mut Context, name: &str, params: Value) -> Value {
    r.execute(ctx, name, params.clone())
        .unwrap_or_else(|e| panic!("{name} {params} failed: {e:?}"))
}

/// The reason of the refusal of `name` `params`, after checking that `to_json` carries it.
fn reason(r: &Registry, ctx: &mut Context, name: &str, params: Value) -> RejectReason {
    match r.execute(ctx, name, params.clone()) {
        Err(e @ CommandError::Rejected { reason, .. }) => {
            let j = e.to_json();
            assert_eq!(j["code"], "rejected", "{name} {params}");
            assert_eq!(j["reason"], reason.as_str(), "{name} {params}");
            reason
        }
        other => panic!("{name} {params}: expected a refusal, got {other:?}"),
    }
}

#[test]
fn each_refusal_site_sets_its_reason() {
    let r = reg();
    let mut fs = MemoryFs::new();
    fs.insert(&RelPath::new("a.txt").unwrap(), b"abc\n");
    fs.insert(&RelPath::new("d/x").unwrap(), b"x");
    fs.insert(&RelPath::new("bad.txt").unwrap(), b"\xff");
    let mut ctx = Context::with_vfs(Box::new(fs));

    let cases: Vec<(&str, Value, RejectReason)> = vec![
        (
            "doc.open",
            json!({"path": "../x"}),
            RejectReason::InvalidPath,
        ),
        (
            "doc.open",
            json!({"path": "missing"}),
            RejectReason::NotFound,
        ),
        ("doc.open", json!({"path": "d"}), RejectReason::NotAFile),
        (
            "doc.open",
            json!({"path": "bad.txt"}),
            RejectReason::DecodeError,
        ),
        ("doc.text", json!({"doc": 9}), RejectReason::NoSuchDocument),
        ("doc.close", json!({"doc": 9}), RejectReason::NoSuchDocument),
        (
            "buffer.insert",
            json!({"offset": 99, "text": "x"}),
            RejectReason::OutOfRange,
        ),
    ];
    for (name, params, want) in cases {
        assert_eq!(
            reason(&r, &mut ctx, name, params.clone()),
            want,
            "{name} {params}"
        );
    }

    let doc = ok(&r, &mut ctx, "doc.open", json!({"path": "a.txt"}))["doc"].clone();
    let cases: Vec<(&str, Value, RejectReason)> = vec![
        (
            "doc.insert",
            json!({"doc": doc, "offset": 99, "text": "x"}),
            RejectReason::OutOfRange,
        ),
        (
            "doc.delete",
            json!({"doc": doc, "start": 2, "end": 1}),
            RejectReason::InvertedRange,
        ),
        (
            "doc.replace",
            json!({"doc": doc, "start": 0, "end": 99, "text": "x"}),
            RejectReason::OutOfRange,
        ),
    ];
    for (name, params, want) in cases {
        assert_eq!(
            reason(&r, &mut ctx, name, params.clone()),
            want,
            "{name} {params}"
        );
    }
    ok(
        &r,
        &mut ctx,
        "doc.insert",
        json!({"doc": doc, "offset": 0, "text": "x"}),
    );
    assert_eq!(
        reason(&r, &mut ctx, "doc.close", json!({"doc": doc})),
        RejectReason::UnsavedChanges
    );
}

#[test]
fn failed_save_is_an_io_error() {
    let r = reg();
    let mut fs = MemoryFs::new();
    fs.insert(&RelPath::new("a.txt").unwrap(), b"abc\n");
    fs.set_fail_writes(true);
    let mut ctx = Context::with_vfs(Box::new(fs));
    let doc = ok(&r, &mut ctx, "doc.open", json!({"path": "a.txt"}))["doc"].clone();
    assert_eq!(
        reason(&r, &mut ctx, "doc.save", json!({"doc": doc})),
        RejectReason::IoError
    );
}

#[test]
fn to_json_adds_reason_for_rejected_and_internal_only() {
    let e = CommandError::rejected(RejectReason::NotFound, "no open document 1");
    assert_eq!(
        e.to_json(),
        json!({"code": "rejected", "reason": "not_found", "message": "no open document 1"})
    );
    assert_eq!(e.to_string(), "no open document 1");
    let e = CommandError::Internal {
        reason: InternalReason::ResultSchema,
        message: "m".into(),
    };
    assert_eq!(
        e.to_json(),
        json!({"code": "internal", "reason": "result_schema", "message": "m"})
    );
    let e = CommandError::UnknownCommand {
        message: "u".into(),
    };
    assert_eq!(
        e.to_json(),
        json!({"code": "unknown_command", "message": "u"})
    );
    let e = CommandError::InvalidParams {
        message: "p".into(),
        details: vec!["d".into()],
    };
    assert_eq!(
        e.to_json(),
        json!({"code": "invalid_params", "message": "p", "details": ["d"]})
    );
}

fn error_kinds(described: &Value, code: &str) -> Vec<(String, String)> {
    described["error_kinds"][code]
        .as_array()
        .unwrap_or_else(|| panic!("error_kinds.{code}"))
        .iter()
        .map(|k| {
            assert_eq!(k.as_object().unwrap().len(), 2, "{k}");
            (
                k["kind"].as_str().unwrap().to_owned(),
                k["when"].as_str().unwrap().to_owned(),
            )
        })
        .collect()
}

#[test]
fn describe_publishes_every_reason_in_all_order() {
    let d = reg().describe();
    let want: Vec<(String, String)> = RejectReason::ALL
        .iter()
        .map(|r| (r.as_str().to_owned(), r.description().to_owned()))
        .collect();
    assert_eq!(error_kinds(&d, "rejected"), want);
    let want: Vec<(String, String)> = InternalReason::ALL
        .iter()
        .map(|r| (r.as_str().to_owned(), r.description().to_owned()))
        .collect();
    assert_eq!(error_kinds(&d, "internal"), want);
    assert_eq!(d["error_kinds"].as_object().unwrap().len(), 2);
    assert!(d["commands"].is_array());

    let committed: Value = serde_json::from_str(
        &std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../docs/commands.json"
        ))
        .expect("read docs/commands.json"),
    )
    .expect("parse docs/commands.json");
    assert_eq!(
        committed["error_kinds"], d["error_kinds"],
        "regenerate docs/commands.json"
    );
}

/// Every `rejected`/`internal` row of the table in `docs/control-protocol.md` is a published
/// kind of `error_kinds` with a non-empty description, and the table keeps its 13 rows. The
/// "When" cells are not compared with the descriptions: the document says `error_kinds`
/// governs where they differ and later tasks do not edit table cells (T-008 ruling v3, D-069);
/// the exact descriptions are pinned by the T-032 acceptance tests.
#[test]
fn table_rows_are_published_kinds_with_their_when_text() {
    let doc = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../docs/control-protocol.md"
    ))
    .expect("read docs/control-protocol.md");
    let d = reg().describe();
    let mut rows = 0;
    for line in doc.lines() {
        let cells: Vec<&str> = line.split('|').map(str::trim).collect();
        if cells.len() < 5 {
            continue;
        }
        let code = cells[1].trim_matches('`');
        if code != "rejected" && code != "internal" {
            continue;
        }
        rows += 1;
        let kind = cells[2].trim_matches('`');
        let listed = error_kinds(&d, code);
        let found = listed.iter().find(|(k, _)| k == kind);
        let (_, desc) = found.unwrap_or_else(|| panic!("{code}/{kind} not in error_kinds"));
        assert!(!desc.trim().is_empty(), "{code}/{kind}: empty description");
    }
    assert_eq!(
        rows, 13,
        "the table keeps its 10 rejected and 3 internal rows"
    );
}
