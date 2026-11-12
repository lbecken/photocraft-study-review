//! T-003 AC-9 through the `kiln-cli` binary: `commands`, `commands --schema` and
//! `commands --audit` cover the eleven `doc.*` commands. Written from the task specification
//! alone by the test author; on the base it fails by assertion (no `doc.*` command exists).

use std::process::Command;

const DOC_COMMANDS: [&str; 11] = [
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

#[cfg(test)]
fn kiln_cli(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_kiln-cli"))
        .args(args)
        .output()
        .expect("kiln-cli runs")
}

#[test]
fn cli_lists_describes_and_audits_doc_commands() {
    let out = kiln_cli(&["commands"]);
    assert!(out.status.success(), "{out:?}");
    let listing = String::from_utf8_lossy(&out.stdout);
    let listed: Vec<&str> = listing
        .lines()
        .map(|l| l.split('\t').next().unwrap_or_default())
        .collect();
    for name in DOC_COMMANDS.iter().chain(&["buffer.insert", "buffer.text"]) {
        assert!(
            listed.contains(name),
            "`{name}` missing from `commands`: {listed:?}"
        );
    }

    let out = kiln_cli(&["commands", "--schema"]);
    assert!(out.status.success(), "{out:?}");
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).expect("json");
    let commands = v["commands"].as_array().expect("commands array");
    for name in DOC_COMMANDS {
        let c = commands
            .iter()
            .find(|c| c["name"] == name)
            .unwrap_or_else(|| panic!("`{name}` missing from --schema"));
        let read_only = matches!(name, "doc.text" | "doc.info" | "doc.list");
        assert_eq!(c["mutating"], serde_json::json!(!read_only), "`{name}`");
        let examples = c["examples"].as_array().expect("examples");
        assert!(!examples.is_empty(), "`{name}` has no example");
        for ex in examples {
            assert!(
                ex["files"].is_array() && ex["setup"].is_array(),
                "`{name}`: {ex}"
            );
        }
    }

    let out = kiln_cli(&["commands", "--audit"]);
    assert!(out.status.success(), "audit failed: {out:?}");
    let report = String::from_utf8_lossy(&out.stdout);
    assert!(
        report.contains(&format!("{} commands", commands.len())),
        "audit must cover every command: {report}"
    );
}
