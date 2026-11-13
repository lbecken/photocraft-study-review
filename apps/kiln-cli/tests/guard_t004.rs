//! T-004 AC-8 (compatibility): the existing `exec` and `commands` subcommands and their exit codes
//! are unchanged by `run`/`control` and by D-023's `--root`. Regression guard: passes on the base
//! by design.
#![cfg(test)]

use std::process::Command;

fn kiln(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_kiln-cli"))
        .args(args)
        .output()
        .expect("kiln-cli runs")
}

#[test]
fn exec_buffer_commands_behave_as_before() {
    let out = kiln(&["exec", "buffer.insert", r#"{"offset":0,"text":"hi"}"#]);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert_eq!(String::from_utf8_lossy(&out.stdout), "{\"len_chars\":2}\n");

    let out = kiln(&[
        "exec",
        "buffer.insert",
        r#"{"offset":3,"text":"!"}"#,
        "--text",
        "abc",
    ]);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert_eq!(String::from_utf8_lossy(&out.stdout), "{\"len_chars\":4}\n");

    let out = kiln(&["exec", "buffer.insert", r#"{"offset":"zero"}"#]);
    assert_eq!(out.status.code(), Some(2), "{out:?}");
    assert!(out.stdout.is_empty());
    let v: serde_json::Value = serde_json::from_slice(&out.stderr).unwrap();
    assert_eq!(v["code"], "invalid_params");

    assert_eq!(kiln(&["exec", "no.such", "{}"]).status.code(), Some(2));
    let out = kiln(&["exec", "buffer.text", "{"]);
    assert_eq!(out.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&out.stderr).contains("invalid_json"));
}

#[test]
fn commands_subcommands_exit_codes_unchanged() {
    assert_eq!(kiln(&["commands"]).status.code(), Some(0));
    assert_eq!(kiln(&["commands", "--schema"]).status.code(), Some(0));
    assert_eq!(kiln(&["commands", "--audit"]).status.code(), Some(0));
    assert_eq!(kiln(&["commands", "--bogus"]).status.code(), Some(2));
}
