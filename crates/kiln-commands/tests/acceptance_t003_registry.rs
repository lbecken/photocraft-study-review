//! Acceptance tests for T-003 that need no file-backed context, written from the task
//! specification alone by the test author.
//!
//! They use only the registry API that exists on the base (`standard_registry`, `execute`,
//! `describe`, `audit`, `Context::new`), so this target compiles on the base and fails there
//! by assertion (the `doc.*` commands are unknown). File-backed criteria are in
//! `acceptance_t003.rs`.

use kiln_commands::{Context, Registry, standard_registry};
use serde_json::{Value, json};
use support::*;

#[cfg(test)]
mod support {
    use super::*;

    pub const DOC_COMMANDS: [&str; 11] = [
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

    /// Declared `mutating` flag per command (Interface: read-only queries are false).
    pub fn expected_mutating(name: &str) -> bool {
        !matches!(name, "doc.text" | "doc.info" | "doc.list" | "buffer.text")
    }

    /// Schema-valid params for each `doc.*` command.
    pub fn valid_params(name: &str) -> Value {
        match name {
            "doc.open" => json!({"path": "a.txt"}),
            "doc.close" => json!({"doc": 1, "discard": true}),
            "doc.insert" => json!({"doc": 1, "offset": 0, "text": "x"}),
            "doc.delete" => json!({"doc": 1, "start": 0, "end": 0}),
            "doc.replace" => json!({"doc": 1, "start": 0, "end": 0, "text": "x"}),
            "doc.list" => json!({}),
            _ => json!({"doc": 1}),
        }
    }

    pub fn reg() -> Registry {
        standard_registry().expect("standard registry builds")
    }

    pub fn code(r: &Registry, ctx: &mut Context, name: &str, params: Value) -> String {
        match r.execute(ctx, name, params.clone()) {
            Ok(v) => panic!("{name} {params} succeeded with {v}, expected an error"),
            Err(e) => e.to_json()["code"]
                .as_str()
                .unwrap_or_else(|| panic!("error without code: {e:?}"))
                .to_owned(),
        }
    }

    pub fn list(r: &Registry, ctx: &mut Context) -> Value {
        r.execute(ctx, "doc.list", json!({}))
            .unwrap_or_else(|e| panic!("doc.list failed: {e:?}"))
    }

    pub fn command<'a>(described: &'a Value, name: &str) -> &'a Value {
        described["commands"]
            .as_array()
            .expect("commands array")
            .iter()
            .find(|c| c["name"] == name)
            .unwrap_or_else(|| panic!("`{name}` is not described"))
    }

    /// The `files`/`setup` checks of AC-9 for one described command.
    pub fn check_examples(c: &Value, doc_group: bool) {
        let name = c["name"].as_str().expect("name");
        let examples = c["examples"].as_array().expect("examples array");
        assert!(!examples.is_empty(), "`{name}` has no example");
        for (i, ex) in examples.iter().enumerate() {
            let files = ex["files"]
                .as_array()
                .unwrap_or_else(|| panic!("`{name}` example {i}: `files` missing: {ex}"));
            let setup = ex["setup"]
                .as_array()
                .unwrap_or_else(|| panic!("`{name}` example {i}: `setup` missing: {ex}"));
            for f in files {
                let pair = f.as_array().expect("files entry is a pair");
                assert_eq!(pair.len(), 2, "`{name}` example {i}: {f}");
                assert!(
                    pair[0].is_string() && pair[1].is_string(),
                    "`{name}` example {i}: {f}"
                );
            }
            for s in setup {
                let pair = s.as_array().expect("setup entry is a pair");
                assert_eq!(pair.len(), 2, "`{name}` example {i}: {s}");
                assert!(
                    pair[0].is_string() && pair[1].is_object(),
                    "`{name}` example {i}: {s}"
                );
            }
            for key in ["description", "initial_text", "params", "result"] {
                assert!(
                    ex.get(key).is_some(),
                    "`{name}` example {i}: `{key}` missing"
                );
            }
            if !doc_group {
                assert!(
                    files.is_empty() && setup.is_empty(),
                    "`{name}` example {i}: {ex}"
                );
            }
        }
    }
}

// AC-9 (schema and examples)

#[test]
fn doc_commands_are_described_and_audited() {
    let r = reg();
    let names = r.names();
    for name in DOC_COMMANDS {
        assert!(
            names.contains(&name),
            "`{name}` is not registered: {names:?}"
        );
        assert!(
            r.doc(name).is_some_and(|d| !d.trim().is_empty()),
            "`{name}` has no doc"
        );
    }
    let described = r.describe();
    for name in DOC_COMMANDS {
        let c = command(&described, name);
        assert!(c["params_schema"].is_object(), "`{name}` params_schema");
        assert!(c["result_schema"].is_object(), "`{name}` result_schema");
        assert_eq!(
            c["params_schema"]["additionalProperties"], false,
            "`{name}` params must reject unknown fields"
        );
        assert_eq!(
            c["mutating"],
            json!(expected_mutating(name)),
            "`{name}` mutating flag"
        );
        check_examples(c, true);
    }
    for name in ["buffer.insert", "buffer.text"] {
        let c = command(&described, name);
        assert_eq!(
            c["mutating"],
            json!(expected_mutating(name)),
            "`{name}` mutating flag"
        );
        check_examples(c, false);
    }
    // Every described command has a boolean flag.
    for c in described["commands"].as_array().expect("commands") {
        assert!(
            c["mutating"].is_boolean(),
            "{}: mutating is not a boolean",
            c["name"]
        );
    }
    assert_eq!(r.audit(), Vec::<String>::new());
}

#[test]
fn docs_commands_json_has_doc_commands_with_fixtures() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../docs/commands.json");
    let text = std::fs::read_to_string(path).expect("docs/commands.json is readable");
    let doc: Value = serde_json::from_str(&text).expect("docs/commands.json is JSON");
    for name in DOC_COMMANDS {
        let c = command(&doc, name);
        assert_eq!(
            c["mutating"],
            json!(expected_mutating(name)),
            "`{name}` in commands.json"
        );
        check_examples(c, true);
    }
    for name in ["buffer.insert", "buffer.text"] {
        check_examples(command(&doc, name), false);
    }
    assert_eq!(doc, reg().describe(), "docs/commands.json is stale");
}

// AC-6 (errors), on a context without files

#[test]
fn doc_params_schema_violations_are_invalid_params() {
    let r = reg();
    let mut ctx = Context::new();
    for name in DOC_COMMANDS {
        let mut bad = valid_params(name);
        bad["unexpected"] = json!(1);
        assert_eq!(
            code(&r, &mut ctx, name, bad.clone()),
            "invalid_params",
            "{name} {bad}"
        );
        for not_object in [json!(null), json!([]), json!("x"), json!(1)] {
            assert_eq!(
                code(&r, &mut ctx, name, not_object.clone()),
                "invalid_params",
                "{name} {not_object}"
            );
        }
    }
    for (name, params) in [
        ("doc.insert", json!({"doc": 1, "offset": 0})),
        ("doc.insert", json!({"doc": "1", "offset": 0, "text": "x"})),
        ("doc.insert", json!({"doc": -1, "offset": 0, "text": "x"})),
        ("doc.insert", json!({"doc": 1.5, "offset": 0, "text": "x"})),
        ("doc.insert", json!({"doc": 1, "offset": -3, "text": "x"})),
        ("doc.delete", json!({"doc": 1, "start": 0})),
        ("doc.delete", json!({"doc": 1, "end": 0})),
        ("doc.replace", json!({"doc": 1, "start": 0, "end": 0})),
        (
            "doc.replace",
            json!({"doc": 1, "start": 0, "end": 0, "text": null}),
        ),
        ("doc.open", json!({})),
        ("doc.open", json!({"path": ["a.txt"]})),
        ("doc.close", json!({})),
        ("doc.close", json!({"doc": 1, "discard": 1})),
        ("doc.save", json!({"doc": null})),
        ("doc.text", json!({})),
        ("doc.info", json!({"doc": true})),
        ("doc.undo", json!({"doc": "one"})),
        ("doc.redo", json!({})),
    ] {
        assert_eq!(
            code(&r, &mut ctx, name, params.clone()),
            "invalid_params",
            "{name} {params}"
        );
    }
    assert_eq!(list(&r, &mut ctx), json!({"docs": []}));
}

#[test]
fn doc_commands_on_a_context_without_files_are_rejected() {
    let r = reg();
    let mut ctx = Context::new();
    assert_eq!(list(&r, &mut ctx), json!({"docs": []}));
    for path in [
        "a.txt",
        "missing/deep.rs",
        "../x",
        "",
        "/abs",
        "a/./b",
        "a\\b",
        "a//b",
    ] {
        assert_eq!(
            code(&r, &mut ctx, "doc.open", json!({"path": path})),
            "rejected",
            "{path:?}"
        );
        assert_eq!(list(&r, &mut ctx), json!({"docs": []}), "{path:?}");
    }
    for name in DOC_COMMANDS {
        if name == "doc.open" || name == "doc.list" {
            continue;
        }
        for doc in [0u64, 1, 2, u64::MAX] {
            let mut params = valid_params(name);
            params["doc"] = json!(doc);
            assert_eq!(
                code(&r, &mut ctx, name, params.clone()),
                "rejected",
                "{name} {params}"
            );
        }
    }
    assert_eq!(list(&r, &mut ctx), json!({"docs": []}));
    // The scratch buffer is untouched by doc.* failures.
    assert_eq!(ctx.buffer().text(), "");
}
