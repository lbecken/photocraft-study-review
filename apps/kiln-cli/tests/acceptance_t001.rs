//! T-001 AC-13 (compatibility): the existing `buffer.insert` path through the `kiln-cli`
//! binary is unchanged by the new buffer. Regression guard: it passes on the base by design.

use std::process::Command;

#[test]
fn cli_exec_buffer_insert_still_prints_len_chars() {
    let out = Command::new(env!("CARGO_BIN_EXE_kiln-cli"))
        .args([
            "exec",
            "buffer.insert",
            r#"{"offset":3,"text":"!"}"#,
            "--text",
            "abc",
        ])
        .output()
        .unwrap();
    assert!(out.status.success(), "{out:?}");
    assert_eq!(String::from_utf8_lossy(&out.stdout), "{\"len_chars\":4}\n");
}
