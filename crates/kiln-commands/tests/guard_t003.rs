//! T-003 AC-10 (compatibility): `buffer.insert` / `buffer.text` behave as before. Regression
//! guard: passes on the base by design, written by the test author from the specification.

use kiln_commands::{Context, standard_registry};
use serde_json::json;

#[test]
fn buffer_commands_behave_as_before() {
    let r = standard_registry().expect("registry");
    let mut ctx = Context::with_text("héllo");
    assert_eq!(
        r.execute(&mut ctx, "buffer.insert", json!({"offset": 2, "text": "X"}))
            .expect("insert"),
        json!({"len_chars": 6})
    );
    assert_eq!(
        r.execute(&mut ctx, "buffer.text", json!({})).expect("text"),
        json!({"text": "héXllo", "len_chars": 6})
    );
    assert_eq!(ctx.buffer().text(), "héXllo");
    let err = r
        .execute(&mut ctx, "buffer.insert", json!({"offset": 7, "text": "x"}))
        .expect_err("out of range");
    assert_eq!(err.to_json()["code"], "rejected");
    assert_eq!(ctx.buffer().text(), "héXllo");
    for bad in [
        json!({"offset": 0}),
        json!({"offset": 0, "text": "x", "doc": 1}),
        json!({"x": 1}),
    ] {
        let err = r
            .execute(&mut ctx, "buffer.insert", bad.clone())
            .expect_err("invalid");
        assert_eq!(err.to_json()["code"], "invalid_params", "{bad}");
    }
    let err = r
        .execute(&mut ctx, "buffer.text", json!({"doc": 1}))
        .expect_err("invalid");
    assert_eq!(err.to_json()["code"], "invalid_params");
    let mut empty = Context::new();
    assert_eq!(
        r.execute(&mut empty, "buffer.text", json!({}))
            .expect("text"),
        json!({"text": "", "len_chars": 0})
    );
}

#[test]
fn buffer_commands_keep_their_examples_and_schemas() {
    let r = standard_registry().expect("registry");
    let d = r.describe();
    let find = |n: &str| {
        d["commands"]
            .as_array()
            .expect("commands")
            .iter()
            .find(|c| c["name"] == n)
            .cloned()
            .expect("described")
    };
    let insert = find("buffer.insert");
    assert_eq!(
        insert["examples"][0]["params"],
        json!({"offset": 0, "text": "fn main() {}"})
    );
    assert_eq!(insert["examples"][0]["result"], json!({"len_chars": 12}));
    assert_eq!(insert["params_schema"]["additionalProperties"], false);
    let text = find("buffer.text");
    assert_eq!(
        text["examples"][0]["result"],
        json!({"text": "let x = 1;", "len_chars": 10})
    );
}
