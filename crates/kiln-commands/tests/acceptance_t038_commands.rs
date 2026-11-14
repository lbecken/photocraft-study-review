//! T-038 AC-2 and AC-3 through the Rust API of `kiln-commands`: the execute check and the
//! describe of a registry built by the test, and the audit of test registries. Written by the
//! test author from `tasks/T-038-error-kinds-match-commands.md` alone.
//!
//! The spec leaves the declaration API to the implementer (Interface 3: "a trait item with a
//! default meaning "no declaration", a method, a builder"), so this file uses only what the
//! spec fixes: a `Command` written with today's items, as the merged test files write them, is
//! a command *without a declaration* (Interface 3; Pinned (f) 7). The parts of AC-2 and AC-3
//! that need a command *with* a declaration (a declared `not_found` passing, an undeclared
//! `decode_error` becoming `internal`/`other`, audit rules (ii)–(v), "an empty list is a
//! declaration", a *disk* kind without an example) cannot be written until the spec names that
//! API: see the QUESTION in the task log (run R-216).
//!
//! No message text and no audit wording is asserted: an audit line is identified by the command
//! name it contains (ruling D-130).

#![cfg(test)]

use kiln_commands::{
    Command, CommandError, Context, Example, Registry, RejectReason, standard_registry,
};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};

/// Params of the test commands: the kind to refuse with, or `"none"` to succeed.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct KindParams {
    /// A `RejectReason` kind, or `"none"`.
    kind: String,
}

/// One valid example (it succeeds with 0), so the existing example checks of the audit find
/// nothing to report.
fn succeeding_example() -> Vec<Example> {
    vec![Example {
        description: "succeeds",
        initial_text: "",
        files: vec![],
        setup: vec![],
        params: json!({"kind": "none"}),
        result: json!(0),
    }]
}

/// Refuse with the kind `p.kind` names (through `CommandError::rejected`), or succeed.
fn refuse_as_named(p: &KindParams) -> Result<u64, CommandError> {
    if p.kind == "none" {
        return Ok(0);
    }
    let reason = RejectReason::ALL
        .iter()
        .copied()
        .find(|r| r.as_str() == p.kind)
        .unwrap_or_else(|| panic!("test bug: {} is not a kind", p.kind));
    Err(CommandError::rejected(reason, "refused by the test"))
}

/// A command with no declaration (only today's `Command` items).
struct Alpha;

impl Command for Alpha {
    const NAME: &'static str = "t38.alpha";
    const DOC: &'static str = "Refuses with the kind its params name; declares nothing.";
    const MUTATING: bool = false;
    type Params = KindParams;
    type Output = u64;
    fn examples() -> Vec<Example> {
        succeeding_example()
    }
    fn run(_: &mut Context, p: KindParams) -> Result<u64, CommandError> {
        refuse_as_named(&p)
    }
}

/// A second command with no declaration.
struct Beta;

impl Command for Beta {
    const NAME: &'static str = "t38.beta";
    const DOC: &'static str = "Refuses with the kind its params name; declares nothing.";
    const MUTATING: bool = false;
    type Params = KindParams;
    type Output = u64;
    fn examples() -> Vec<Example> {
        succeeding_example()
    }
    fn run(_: &mut Context, p: KindParams) -> Result<u64, CommandError> {
        refuse_as_named(&p)
    }
}

fn registry_of_alpha() -> Registry {
    let mut r = Registry::new();
    r.register::<Alpha>().expect("t38.alpha registers");
    r
}

/// AC-2 (the part the spec fixes without a declaration API). A command with no declaration is
/// not checked: its `decode_error` refusal comes back as `Rejected { reason: DecodeError }`, and
/// nothing is logged. Its entry in `describe()` has `refusals`, empty (it declares no kind,
/// Interface 1), and `error_kinds.rejected` lists exactly the kinds some command's `refusals`
/// hold, so for this registry none; `error_kinds.internal` is unchanged.
/// Fails on base by assertion: `describe()` has no `refusals` and publishes all of
/// `RejectReason::ALL`.
#[test]
fn undeclared_refusal_is_internal() {
    let r = registry_of_alpha();
    let mut ctx = Context::new();
    let got = r.execute(&mut ctx, "t38.alpha", json!({"kind": "decode_error"}));
    assert!(
        matches!(
            got,
            Err(CommandError::Rejected {
                reason: RejectReason::DecodeError,
                ..
            })
        ),
        "a command without a declaration is not checked (Interface 2): {got:?}"
    );
    assert!(
        ctx.log().is_empty(),
        "a refusal is not logged: {:?}",
        ctx.log()
    );

    // Sanity: the same command succeeds and is logged when asked to, so the refusal above is
    // the command's own and the context is the one execute logs into.
    let mut ctx = Context::new();
    let ok = r.execute(&mut ctx, "t38.alpha", json!({"kind": "none"}));
    assert_eq!(ok, Ok(json!(0)));
    assert_eq!(ctx.log().len(), 1);

    let d = r.describe();
    let commands = d["commands"].as_array().expect("commands is an array");
    assert_eq!(commands.len(), 1, "{d}");
    assert_eq!(
        commands[0]["refusals"],
        json!([]),
        "a command without a declaration publishes an empty `refusals` (Interface 1): {}",
        commands[0]
    );
    let rejected = d["error_kinds"]["rejected"]
        .as_array()
        .expect("error_kinds.rejected is an array");
    let kinds: Vec<&Value> = rejected.iter().map(|e| &e["kind"]).collect();
    assert!(
        kinds.is_empty(),
        "no command of this registry declares a kind, so `error_kinds.rejected` is empty \
         (Interface 1); it lists {kinds:?}"
    );
    let internal: Vec<&Value> = d["error_kinds"]["internal"]
        .as_array()
        .expect("error_kinds.internal is an array")
        .iter()
        .map(|e| &e["kind"])
        .collect();
    assert_eq!(
        internal,
        [&json!("panic"), &json!("result_schema"), &json!("other")],
        "error_kinds.internal is unchanged (Interface 1)"
    );
}

/// AC-3 (the parts the spec fixes without a declaration API). Rule (i): a registry whose one
/// command has no declaration audits to exactly one problem line, which names that command;
/// with two such commands, two lines, one per command in name order (Pinned (c)). The standard
/// registry's audit is empty (Interface 3), as `kiln-cli commands --audit` exiting 0 shows
/// through the binary (asserted in `acceptance_t038_cli.rs`, which can run it).
/// Fails on base by assertion: today's audit reports nothing for a command without a
/// declaration.
#[test]
fn audit_requires_a_witness_for_every_declared_kind() {
    let standard = standard_registry().expect("standard registry builds");
    let problems = standard.audit();
    assert!(
        problems.is_empty(),
        "the standard registry's audit is empty (Interface 3): {problems:#?}"
    );

    let problems = registry_of_alpha().audit();
    assert_eq!(
        problems.len(),
        1,
        "rule (i): one problem for the one command without a declaration: {problems:#?}"
    );
    assert!(
        problems[0].contains("t38.alpha"),
        "the problem line names its command: {problems:#?}"
    );

    let mut two = Registry::new();
    two.register::<Beta>().expect("t38.beta registers");
    two.register::<Alpha>().expect("t38.alpha registers");
    let problems = two.audit();
    assert_eq!(
        problems.len(),
        2,
        "rule (i): one problem per command without a declaration: {problems:#?}"
    );
    assert!(
        problems[0].contains("t38.alpha") && !problems[0].contains("t38.beta"),
        "first line: t38.alpha (name order, Pinned (c)): {problems:#?}"
    );
    assert!(
        problems[1].contains("t38.beta") && !problems[1].contains("t38.alpha"),
        "second line: t38.beta (name order, Pinned (c)): {problems:#?}"
    );
}
