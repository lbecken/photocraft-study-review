//! # kiln-cli — headless application (thin binary)
//!
//! Everything the UI can do, the CLI can do (CHARTER §3 decision 1): it exposes the command
//! registry of `kiln-commands`.
//!
//! ```text
//! kiln-cli commands              list command names and docs
//! kiln-cli commands --schema     print every command with params/result schemas and examples
//! kiln-cli commands --audit      check every command's examples against its schemas
//! kiln-cli exec <name> <json> [--text <initial buffer text>]
//! ```
//!
//! Exit codes: 0 success, 1 audit problems or internal error, 2 usage or command error.

use std::io::Write;
use std::process::ExitCode;

use kiln_commands::{Context, standard_registry};
use serde_json::Value;

const USAGE: &str = "usage:\n  kiln-cli commands [--schema | --audit]\n  kiln-cli exec <name> <json-params> [--text <initial-text>]";

/// Result of one CLI invocation, separated from process I/O so it can be unit-tested.
#[derive(Debug, Default, PartialEq, Eq)]
struct Outcome {
    code: u8,
    stdout: String,
    stderr: String,
}

impl Outcome {
    fn ok(stdout: String) -> Self {
        Self {
            code: 0,
            stdout,
            stderr: String::new(),
        }
    }
    fn err(code: u8, stderr: String) -> Self {
        Self {
            code,
            stdout: String::new(),
            stderr,
        }
    }
}

fn pretty(v: &Value) -> String {
    serde_json::to_string_pretty(v).unwrap_or_else(|_| v.to_string())
}

fn run(args: &[String]) -> Outcome {
    let registry = match standard_registry() {
        Ok(r) => r,
        Err(e) => return Outcome::err(1, format!("registry failed to build: {e}\n")),
    };
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    match args.as_slice() {
        ["commands"] => {
            let mut out = String::new();
            for name in registry.names() {
                let doc = registry.doc(name).unwrap_or_default();
                out.push_str(&format!("{name}\t{doc}\n"));
            }
            Outcome::ok(out)
        }
        ["commands", "--schema"] => Outcome::ok(format!("{}\n", pretty(&registry.describe()))),
        ["commands", "--audit"] => {
            let problems = registry.audit();
            if problems.is_empty() {
                Outcome::ok(format!(
                    "audit ok: {} commands, all examples validate\n",
                    registry.names().len()
                ))
            } else {
                Outcome::err(1, problems.iter().map(|p| format!("{p}\n")).collect())
            }
        }
        ["exec", name, params, rest @ ..] => {
            let text = match rest {
                [] => "",
                ["--text", t] => t,
                _ => return Outcome::err(2, format!("{USAGE}\n")),
            };
            let params: Value = match serde_json::from_str(params) {
                Ok(v) => v,
                Err(e) => {
                    return Outcome::err(
                        2,
                        format!(
                            "{}\n",
                            serde_json::json!({"code": "invalid_json", "message": e.to_string()})
                        ),
                    );
                }
            };
            let mut ctx = Context::with_text(text);
            match registry.execute(&mut ctx, name, params) {
                Ok(result) => Outcome::ok(format!("{result}\n")),
                Err(e) => Outcome::err(2, format!("{}\n", e.to_json())),
            }
        }
        _ => Outcome::err(2, format!("{USAGE}\n")),
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let outcome = run(&args);
    // Write failures (closed pipe) cannot be reported anywhere useful; the exit code stands.
    let _ = std::io::stdout().write_all(outcome.stdout.as_bytes());
    let _ = std::io::stderr().write_all(outcome.stderr.as_bytes());
    ExitCode::from(outcome.code)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn call(args: &[&str]) -> Outcome {
        run(&args.iter().map(|s| s.to_string()).collect::<Vec<_>>())
    }

    #[test]
    fn exec_insert_reports_new_length() {
        let out = call(&["exec", "buffer.insert", r#"{"offset":0,"text":"hi"}"#]);
        assert_eq!(out, Outcome::ok("{\"len_chars\":2}\n".into()));
    }

    #[test]
    fn exec_text_with_initial_text() {
        let out = call(&["exec", "buffer.text", "{}", "--text", "abc"]);
        assert_eq!(out.code, 0);
        let v: Value = serde_json::from_str(&out.stdout).unwrap();
        assert_eq!(v["text"], "abc");
    }

    #[test]
    fn exec_invalid_params_exits_2_with_structured_error() {
        let out = call(&["exec", "buffer.insert", r#"{"offset":"zero"}"#]);
        assert_eq!(out.code, 2);
        let v: Value = serde_json::from_str(&out.stderr).unwrap();
        assert_eq!(v["code"], "invalid_params");
    }

    #[test]
    fn exec_unknown_command_and_bad_json() {
        assert_eq!(call(&["exec", "no.such", "{}"]).code, 2);
        let out = call(&["exec", "buffer.text", "{"]);
        assert_eq!(out.code, 2);
        assert!(out.stderr.contains("invalid_json"));
    }

    #[test]
    fn schema_lists_every_command() {
        let out = call(&["commands", "--schema"]);
        let v: Value = serde_json::from_str(&out.stdout).unwrap();
        let names: Vec<&str> = v["commands"]
            .as_array()
            .unwrap()
            .iter()
            .map(|c| c["name"].as_str().unwrap())
            .collect();
        assert_eq!(names, ["buffer.insert", "buffer.text"]);
    }

    #[test]
    fn audit_passes() {
        assert_eq!(call(&["commands", "--audit"]).code, 0);
    }

    #[test]
    fn usage_on_unknown_subcommand() {
        assert_eq!(call(&["frobnicate"]).code, 2);
    }
}
