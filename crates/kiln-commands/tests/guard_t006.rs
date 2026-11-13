//! T-006 regression guards (D-010) for the D-023 spec-gap rulings on the document commands:
//! each test passes on the base by design and fails if the ruling it pins is broken. Every
//! test dispatches commands by name on an in-memory file system (`Context::with_files`).

use serde_json::{Value, json};
use support::*;

#[cfg(test)]
mod support {
    use kiln_commands::{Context, Registry, standard_registry};
    use kiln_workspace::RelPath;
    use serde_json::Value;

    pub fn reg() -> Registry {
        standard_registry().expect("standard registry builds")
    }

    pub fn ctx_files(files: &[(&str, &[u8])]) -> Context {
        Context::with_files(files).unwrap_or_else(|e| panic!("with_files({files:?}): {e:?}"))
    }

    pub fn ok(r: &Registry, ctx: &mut Context, name: &str, params: Value) -> Value {
        r.execute(ctx, name, params.clone())
            .unwrap_or_else(|e| panic!("{name} {params} failed: {e:?}"))
    }

    pub fn file(ctx: &Context, path: &str) -> Vec<u8> {
        let p = RelPath::new(path).unwrap_or_else(|e| panic!("RelPath::new({path:?}): {e:?}"));
        ctx.vfs()
            .read(&p)
            .unwrap_or_else(|e| panic!("read {path}: {e:?}"))
    }
}

/// R4 (D-023 SG-4): `MUTATING` belongs to the command, so a no-op re-open is still mutating.
#[test]
fn doc_open_is_mutating_even_for_a_reopen() {
    let r = reg();
    let described = r.describe();
    let open = described["commands"]
        .as_array()
        .expect("commands array")
        .iter()
        .find(|c| c["name"] == "doc.open")
        .expect("doc.open is described");
    assert_eq!(open["mutating"], json!(true));

    let mut ctx = ctx_files(&[("a.txt", b"hello\n")]);
    let first = ok(&r, &mut ctx, "doc.open", json!({"path": "a.txt"}));
    assert_eq!(first["already_open"], json!(false));
    let second = ok(&r, &mut ctx, "doc.open", json!({"path": "a.txt"}));
    assert_eq!(second["already_open"], json!(true));
    assert_eq!(second["doc"], first["doc"]);
    let log = ctx.log();
    assert_eq!(log.len(), 2);
    for (i, entry) in log.iter().enumerate() {
        assert_eq!(entry.name, "doc.open", "log entry {i}");
        assert!(entry.mutating, "log entry {i} of doc.open is mutating");
    }
}

/// R5 (D-023 SG-5): only a successful first open assigns an id.
#[test]
fn only_a_successful_first_open_assigns_an_id() {
    let r = reg();
    let mut ctx = ctx_files(&[
        ("a.txt", b"hello\n"),
        ("b.txt", b"b\n"),
        ("bad.txt", b"\xff"),
    ]);
    // The messages are the ones the control layer classifies as the `rejected` kinds
    // `not_found`, `decode_error` and `invalid_path` (what `kiln-cli` shows).
    let failures = [
        ("missing.txt", "cannot open missing.txt: not found"),
        (
            "bad.txt",
            "cannot open bad.txt: invalid utf-8 at byte offset 0",
        ),
        (
            "../x",
            "invalid relative path \"../x\": path has a '..' segment",
        ),
    ];
    for (path, message) in failures {
        let err = r
            .execute(&mut ctx, "doc.open", json!({ "path": path }))
            .expect_err(path);
        let got = err.to_json();
        assert_eq!(got["code"], json!("rejected"), "doc.open {path}: {got}");
        assert_eq!(got["message"], json!(message), "doc.open {path}: {got}");
    }
    assert!(ctx.log().is_empty(), "failed opens are not logged");

    let a = ok(&r, &mut ctx, "doc.open", json!({"path": "a.txt"}));
    assert_eq!(
        (a["doc"].clone(), a["already_open"].clone()),
        (json!(1), json!(false))
    );
    let again = ok(&r, &mut ctx, "doc.open", json!({"path": "a.txt"}));
    assert_eq!(
        (again["doc"].clone(), again["already_open"].clone()),
        (json!(1), json!(true))
    );
    let b = ok(&r, &mut ctx, "doc.open", json!({"path": "b.txt"}));
    assert_eq!(
        (b["doc"].clone(), b["already_open"].clone()),
        (json!(2), json!(false))
    );

    let list = ok(&r, &mut ctx, "doc.list", json!({}));
    let ids: Vec<(Value, Value)> = list["docs"]
        .as_array()
        .expect("docs array")
        .iter()
        .map(|d| (d["doc"].clone(), d["path"].clone()))
        .collect();
    assert_eq!(
        ids,
        vec![(json!(1), json!("a.txt")), (json!(2), json!("b.txt"))]
    );
}

/// R6 (D-023 SG-6): `doc.save` only ends the typing group; an available redo survives it.
#[test]
fn save_keeps_redo_available() {
    let r = reg();
    let mut ctx = ctx_files(&[("a.txt", b"x")]);
    let open = ok(&r, &mut ctx, "doc.open", json!({"path": "a.txt"}));
    assert_eq!(open["doc"], json!(1));
    ok(
        &r,
        &mut ctx,
        "doc.insert",
        json!({"doc": 1, "offset": 1, "text": "a"}),
    );
    assert_eq!(
        ok(&r, &mut ctx, "doc.undo", json!({"doc": 1}))["undone"],
        json!(true)
    );
    assert_eq!(
        ok(&r, &mut ctx, "doc.info", json!({"doc": 1}))["can_redo"],
        json!(true)
    );
    assert_eq!(
        ok(&r, &mut ctx, "doc.save", json!({"doc": 1}))["bytes_written"],
        json!(1)
    );
    let info = ok(&r, &mut ctx, "doc.info", json!({"doc": 1}));
    assert_eq!(
        (info["can_redo"].clone(), info["dirty"].clone()),
        (json!(true), json!(false))
    );
    assert_eq!(
        ok(&r, &mut ctx, "doc.redo", json!({"doc": 1})),
        json!({"redone": true, "len_chars": 2, "dirty": true})
    );
    assert_eq!(
        ok(&r, &mut ctx, "doc.text", json!({"doc": 1}))["text"],
        json!("xa")
    );
    assert_eq!(file(&ctx, "a.txt"), b"x");
}
