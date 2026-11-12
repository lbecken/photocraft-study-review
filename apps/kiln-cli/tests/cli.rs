//! End-to-end tests of the `kiln-cli` binary through its process interface.

use std::process::Command;

fn kiln_cli(args: &[&str]) -> std::io::Result<std::process::Output> {
    Command::new(env!("CARGO_BIN_EXE_kiln-cli"))
        .args(args)
        .output()
}

#[test]
fn exec_insert_through_the_binary() {
    let out = kiln_cli(&[
        "exec",
        "buffer.insert",
        r#"{"offset":3,"text":"!"}"#,
        "--text",
        "abc",
    ])
    .unwrap();
    assert!(out.status.success(), "{out:?}");
    assert_eq!(String::from_utf8_lossy(&out.stdout), "{\"len_chars\":4}\n");
}

#[test]
fn exec_error_is_json_on_stderr_with_exit_2() {
    let out = kiln_cli(&["exec", "buffer.insert", r#"{"offset":9,"text":"!"}"#]).unwrap();
    assert_eq!(out.status.code(), Some(2));
    let err: serde_json::Value = serde_json::from_slice(&out.stderr).expect("json error");
    assert_eq!(err["code"], "rejected");
}

#[test]
fn commands_schema_through_the_binary() {
    let out = kiln_cli(&["commands", "--schema"]).unwrap();
    assert!(out.status.success());
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).expect("json");
    for c in v["commands"].as_array().expect("array") {
        assert!(c["params_schema"].is_object());
        assert!(c["result_schema"].is_object());
        assert!(!c["examples"].as_array().expect("examples").is_empty());
    }
}
