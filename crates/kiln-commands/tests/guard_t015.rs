//! T-015 regression guards (D-010, D-021), Rust API: `Context` now delegates its document map
//! and id counter to `Docs`, and that must not change behaviour. These pass on base by design.
//! Per D-043 they pin behaviour that must not change, not listings meant to grow: the command
//! list is only required to keep today's commands (the `schema` gate step compares the whole
//! list at the join).
#![cfg(test)]

use kiln_commands::{CommandError, Context, Registry, standard_registry};
use kiln_workspace::RelPath;
use serde_json::{Value, json};

fn reg() -> Registry {
    standard_registry().expect("standard registry builds")
}

fn ok(r: &Registry, ctx: &mut Context, name: &str, params: Value) -> Value {
    r.execute(ctx, name, params.clone())
        .unwrap_or_else(|e| panic!("{name} {params} failed: {e:?}"))
}

fn refused(r: &Registry, ctx: &mut Context, name: &str, params: Value) -> String {
    match r.execute(ctx, name, params.clone()) {
        Err(CommandError::Rejected { message, .. }) => message,
        other => panic!("{name} {params}: expected a refusal, got {other:?}"),
    }
}

const BASE_COMMANDS: &[&str] = &[
    "buffer.insert",
    "buffer.text",
    "doc.close",
    "doc.delete",
    "doc.info",
    "doc.insert",
    "doc.list",
    "doc.open",
    "doc.redo",
    "doc.replace",
    "doc.save",
    "doc.text",
    "doc.undo",
];

/// Every command of the base is still registered, under its name, and `names()` stays sorted
/// and free of duplicates.
#[test]
fn base_commands_are_still_registered() {
    let names = reg().names();
    for name in BASE_COMMANDS {
        assert!(names.contains(name), "{name} missing from {names:?}");
    }
    let mut sorted = names.clone();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(names, sorted);
}

/// Ids 1, 2, 3, … in open order, never reused after a close; a re-open of an open path returns
/// its id; a failed open uses up no id; `doc.list` is in increasing id order; every command on
/// an id that is not open gives today's exact `no open document {id}` refusal and leaves the
/// session unchanged.
#[test]
fn document_ids_and_refusals_are_unchanged() {
    let r = reg();
    let mut ctx = Context::with_files(&[
        ("a.txt", b"a\n"),
        ("b.txt", b"b\n"),
        ("c.txt", b"c\n"),
        ("bad.bin", b"\x80"),
    ])
    .unwrap();
    let id = |v: Value| v["doc"].as_u64().expect("doc id");
    assert_eq!(
        id(ok(&r, &mut ctx, "doc.open", json!({"path": "a.txt"}))),
        1
    );
    assert!(
        r.execute(&mut ctx, "doc.open", json!({"path": "missing"}))
            .is_err()
    );
    assert!(
        r.execute(&mut ctx, "doc.open", json!({"path": "bad.bin"}))
            .is_err()
    );
    assert_eq!(
        id(ok(&r, &mut ctx, "doc.open", json!({"path": "b.txt"}))),
        2
    );
    let again = ok(&r, &mut ctx, "doc.open", json!({"path": "a.txt"}));
    assert_eq!(
        (id(again.clone()), again["already_open"].clone()),
        (1, json!(true))
    );
    ok(&r, &mut ctx, "doc.close", json!({"doc": 1}));
    assert_eq!(
        id(ok(&r, &mut ctx, "doc.open", json!({"path": "c.txt"}))),
        3
    );
    assert_eq!(
        id(ok(&r, &mut ctx, "doc.open", json!({"path": "a.txt"}))),
        4
    );
    let list = ok(&r, &mut ctx, "doc.list", json!({}));
    let ids: Vec<u64> = list["docs"]
        .as_array()
        .expect("docs array")
        .iter()
        .map(|d| d["doc"].as_u64().expect("doc"))
        .collect();
    assert_eq!(ids, [2, 3, 4]);
    let paths: Vec<&str> = list["docs"]
        .as_array()
        .expect("docs array")
        .iter()
        .map(|d| d["path"].as_str().expect("path"))
        .collect();
    assert_eq!(paths, ["b.txt", "c.txt", "a.txt"]);

    let log_len = ctx.log().len();
    for gone in [0u64, 1, 5, u64::MAX] {
        let msg = format!("no open document {gone}");
        for (name, params) in [
            ("doc.text", json!({"doc": gone})),
            ("doc.info", json!({"doc": gone})),
            ("doc.insert", json!({"doc": gone, "offset": 0, "text": "x"})),
            ("doc.delete", json!({"doc": gone, "start": 0, "end": 0})),
            (
                "doc.replace",
                json!({"doc": gone, "start": 0, "end": 0, "text": ""}),
            ),
            ("doc.undo", json!({"doc": gone})),
            ("doc.redo", json!({"doc": gone})),
            ("doc.save", json!({"doc": gone})),
            ("doc.close", json!({"doc": gone})),
            ("doc.close", json!({"doc": gone, "discard": true})),
        ] {
            assert_eq!(refused(&r, &mut ctx, name, params), msg, "{name} {gone}");
        }
    }
    assert_eq!(ctx.log().len(), log_len, "refusals are not logged");
    assert_eq!(ok(&r, &mut ctx, "doc.list", json!({})), list);

    // Closing the newest document does not give its id back.
    ok(&r, &mut ctx, "doc.close", json!({"doc": 4}));
    assert_eq!(
        id(ok(&r, &mut ctx, "doc.open", json!({"path": "a.txt"}))),
        5
    );

    // Save goes to the document's own file and the files of the others stay.
    ok(
        &r,
        &mut ctx,
        "doc.insert",
        json!({"doc": 3, "offset": 0, "text": "+"}),
    );
    ok(&r, &mut ctx, "doc.save", json!({"doc": 3}));
    let read = |ctx: &Context, p: &str| ctx.vfs().read(&RelPath::new(p).unwrap()).unwrap();
    assert_eq!(read(&ctx, "c.txt"), b"+c\n");
    assert_eq!(read(&ctx, "a.txt"), b"a\n");
    assert_eq!(read(&ctx, "b.txt"), b"b\n");
    // Closing a dirty document is refused with today's message and keeps it open.
    ok(
        &r,
        &mut ctx,
        "doc.insert",
        json!({"doc": 2, "offset": 0, "text": "x"}),
    );
    assert_eq!(
        refused(&r, &mut ctx, "doc.close", json!({"doc": 2})),
        "document 2 has unsaved changes; save it or close with discard"
    );
    assert_eq!(
        ok(&r, &mut ctx, "doc.text", json!({"doc": 2}))["text"],
        json!("xb\n")
    );
}
