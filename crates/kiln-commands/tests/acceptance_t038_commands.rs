//! T-038 AC-2 and AC-3 through the Rust API of `kiln-commands`: the execute check and the
//! describe of registries built by the test, and the audit of test registries. Written by the
//! test author from `tasks/T-038-error-kinds-match-commands.md` and ruling j1 (D-136), which
//! pins the public declaration API (`Refusal`, `Witness`, `RefusalExample`, `SeedFile`, and the
//! provided `Command::refusals` / `Command::refusal_examples`; default `None` = no declaration,
//! `Some(vec![])` = declares that the command never refuses).
//!
//! Every test command refuses with the kind its params name (or succeeds with `"none"`), so
//! one command type serves every case; what differs is its declaration and its refusal
//! examples. Each audit case is a registry of one command with one problem (or none), and the
//! test asserts the number of problem lines and that each names its command (ruling D-130):
//! never the wording of a line or of a message (golden rule 9).
//!
//! On base this file does not compile (the declaration API does not exist yet): weak evidence
//! by design (ruling j1). The version of this file at `667d77a`, which used no new API, failed
//! on base by assertion in both tests (no `refusals` in `describe()`; no audit line for a
//! command without a declaration).
//!
//! `examples()` of the test commands uses the `Example` literal of the merged test files
//! (`acceptance_t032.rs`), with empty `files` and `setup`.

#![cfg(test)]

use kiln_commands::{
    Command, CommandError, Context, Example, InternalReason, Refusal, RefusalExample, Registry,
    RejectReason, SeedFile, standard_registry,
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

/// A refusal example of `kind` whose params ask the command to refuse with `asks` (a kind or
/// `"none"`), on an empty context.
fn refusal_example(kind: RejectReason, asks: &str) -> RefusalExample {
    RefusalExample {
        description: "test refusal example",
        initial_text: "",
        files: vec![],
        setup: vec![],
        params: json!({ "kind": asks }),
        kind,
    }
}

/// The sound refusal example of `kind`: it asks for exactly that kind.
fn witness(kind: RejectReason) -> RefusalExample {
    refusal_example(kind, kind.as_str())
}

/// A test command type `$ty` named `$name`, with the declaration `$refusals` and the refusal
/// examples `$examples`; every one refuses as its params name.
macro_rules! test_command {
    ($ty:ident, $name:literal, $refusals:expr, $examples:expr) => {
        struct $ty;
        impl Command for $ty {
            const NAME: &'static str = $name;
            const DOC: &'static str = "Refuses with the kind its params name.";
            const MUTATING: bool = false;
            type Params = KindParams;
            type Output = u64;
            fn examples() -> Vec<Example> {
                succeeding_example()
            }
            fn refusals() -> Option<Vec<Refusal>> {
                $refusals
            }
            fn refusal_examples() -> Vec<RefusalExample> {
                $examples
            }
            fn run(_: &mut Context, p: KindParams) -> Result<u64, CommandError> {
                refuse_as_named(&p)
            }
        }
    };
}

use RejectReason::{DecodeError, IoError, NotFound};

/// No declaration at all: only today's items (the provided defaults).
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

// AC-2's command: declares `not_found` (witnessed).
test_command!(
    DeclaresNotFound,
    "t38.declared",
    Some(vec![Refusal::example(NotFound)]),
    vec![witness(NotFound)]
);
// Declares that it never refuses.
test_command!(DeclaresNothing, "t38.never", Some(vec![]), vec![]);
// Declares a *disk* kind only (no example needed).
test_command!(
    DeclaresDisk,
    "t38.disk",
    Some(vec![Refusal::disk(IoError)]),
    vec![]
);

// AC-3's audit cases.
test_command!(
    Sound,
    "t38.sound",
    Some(vec![Refusal::example(NotFound)]),
    vec![witness(NotFound)]
);
test_command!(
    SoundWithDisk,
    "t38.sound-disk",
    Some(vec![Refusal::example(NotFound), Refusal::disk(IoError)]),
    vec![witness(NotFound)]
);
test_command!(
    NoWitness,
    "t38.no-witness",
    Some(vec![Refusal::example(NotFound)]),
    vec![]
);
test_command!(
    ExampleSucceeds,
    "t38.succeeds",
    Some(vec![Refusal::example(NotFound)]),
    vec![refusal_example(NotFound, "none")]
);
test_command!(
    ExampleOtherKind,
    "t38.other-kind",
    Some(vec![Refusal::example(NotFound), Refusal::disk(DecodeError)]),
    vec![refusal_example(NotFound, "decode_error")]
);
test_command!(
    ExampleBadParams,
    "t38.bad-params",
    Some(vec![Refusal::example(NotFound)]),
    vec![RefusalExample {
        params: json!({"kind": "not_found", "extra": 1}),
        ..witness(NotFound)
    }]
);
test_command!(
    ExampleSetupFails,
    "t38.setup-fails",
    Some(vec![Refusal::example(NotFound)]),
    vec![RefusalExample {
        setup: vec![("t38.setup-fails", json!({"kind": "not_found"}))],
        ..witness(NotFound)
    }]
);
test_command!(
    ExampleBadPath,
    "t38.bad-path",
    Some(vec![Refusal::example(NotFound)]),
    vec![RefusalExample {
        files: vec![SeedFile::text("../x.txt", "x")],
        ..witness(NotFound)
    }]
);
test_command!(
    ExampleBadHex,
    "t38.bad-hex",
    Some(vec![Refusal::example(NotFound)]),
    vec![RefusalExample {
        files: vec![SeedFile::hex("x.bin", "abc")],
        ..witness(NotFound)
    }]
);
test_command!(
    ExampleUndeclared,
    "t38.undeclared",
    Some(vec![Refusal::example(NotFound)]),
    vec![witness(NotFound), witness(DecodeError)]
);
test_command!(
    ExampleOfDisk,
    "t38.of-disk",
    Some(vec![Refusal::example(NotFound), Refusal::disk(IoError)]),
    vec![witness(NotFound), witness(IoError)]
);

fn one<C: Command + 'static>() -> Registry {
    let mut r = Registry::new();
    r.register::<C>()
        .unwrap_or_else(|e| panic!("{} registers: {e}", C::NAME));
    r
}

fn kinds_of(list: &Value) -> Vec<&str> {
    list.as_array()
        .unwrap_or_else(|| panic!("a list of kinds: {list}"))
        .iter()
        .map(|k| k.as_str().unwrap_or_else(|| panic!("a kind is a string: {k}")))
        .collect()
}

fn entry<'a>(d: &'a Value, name: &str) -> &'a Value {
    d["commands"]
        .as_array()
        .expect("commands is an array")
        .iter()
        .find(|c| c["name"] == json!(name))
        .unwrap_or_else(|| panic!("{name} is described: {d}"))
}

/// AC-2. A registry with `t38.declared` (declares `not_found`) and `t38.alpha` (no
/// declaration). `t38.declared` refusing `not_found` → `Rejected { NotFound }`; refusing
/// `decode_error` → `Internal { Other }`; nothing logged in either case. `t38.alpha` refusing
/// `decode_error` → `Rejected { DecodeError }` (unchecked). `describe()`: `t38.declared`'s
/// `refusals` are `["not_found"]`, `t38.alpha`'s `[]`, and `error_kinds.rejected` lists
/// `not_found` and no other kind; `internal` is unchanged. Also: an empty declaration is
/// checked (`t38.never` refusing → `internal`), and a declared *disk* kind passes
/// (`t38.disk` refusing `io_error` → `Rejected { IoError }`).
#[test]
fn undeclared_refusal_is_internal() {
    let mut r = Registry::new();
    r.register::<DeclaresNotFound>().expect("t38.declared registers");
    r.register::<Alpha>().expect("t38.alpha registers");

    let run = |name: &str, kind: &str| {
        let mut ctx = Context::new();
        let got = r.execute(&mut ctx, name, json!({ "kind": kind }));
        assert!(
            ctx.log().is_empty(),
            "{name} refusing {kind}: a refusal is not logged: {:?}",
            ctx.log()
        );
        got
    };

    let got = run("t38.declared", "not_found");
    assert!(
        matches!(got, Err(CommandError::Rejected { reason: NotFound, .. })),
        "a declared refusal is returned unchanged (Interface 2): {got:?}"
    );
    let got = run("t38.declared", "decode_error");
    assert!(
        matches!(
            got,
            Err(CommandError::Internal {
                reason: InternalReason::Other,
                ..
            })
        ),
        "a refusal of an undeclared kind is internal/other (Interface 2): {got:?}"
    );
    let got = run("t38.alpha", "decode_error");
    assert!(
        matches!(
            got,
            Err(CommandError::Rejected {
                reason: DecodeError,
                ..
            })
        ),
        "a command without a declaration is not checked (Interface 2): {got:?}"
    );

    // Sanity: the same command succeeds and is logged when asked to, so the context above is
    // the one execute logs into.
    let mut ctx = Context::new();
    assert_eq!(
        r.execute(&mut ctx, "t38.declared", json!({"kind": "none"})),
        Ok(json!(0))
    );
    assert_eq!(ctx.log().len(), 1);

    let d = r.describe();
    assert_eq!(
        kinds_of(&entry(&d, "t38.declared")["refusals"]),
        ["not_found"],
        "t38.declared publishes its declared kind (Interface 1)"
    );
    assert_eq!(
        kinds_of(&entry(&d, "t38.alpha")["refusals"]),
        Vec::<&str>::new(),
        "a command without a declaration publishes an empty `refusals` (Interface 1)"
    );
    let rejected: Vec<&Value> = d["error_kinds"]["rejected"]
        .as_array()
        .expect("error_kinds.rejected is an array")
        .iter()
        .map(|e| &e["kind"])
        .collect();
    assert_eq!(
        rejected,
        [&json!("not_found")],
        "error_kinds.rejected lists not_found and no other kind (AC-2)"
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

    // An empty list is a declaration, and it is checked.
    let never = one::<DeclaresNothing>();
    let mut ctx = Context::new();
    let got = never.execute(&mut ctx, "t38.never", json!({"kind": "not_found"}));
    assert!(
        matches!(
            got,
            Err(CommandError::Internal {
                reason: InternalReason::Other,
                ..
            })
        ),
        "Some(vec![]) declares no kind, so any refusal is internal/other: {got:?}"
    );
    assert!(ctx.log().is_empty());

    // A declared *disk* kind is declared: it passes.
    let disk = one::<DeclaresDisk>();
    let mut ctx = Context::new();
    let got = disk.execute(&mut ctx, "t38.disk", json!({"kind": "io_error"}));
    assert!(
        matches!(got, Err(CommandError::Rejected { reason: IoError, .. })),
        "a declared *disk* refusal is returned unchanged: {got:?}"
    );
    assert!(ctx.log().is_empty());
    let d = disk.describe();
    assert_eq!(
        kinds_of(&entry(&d, "t38.disk")["refusals"]),
        ["io_error"],
        "a *disk* kind is published in `refusals` like any declared kind"
    );
    let rejected: Vec<&Value> = d["error_kinds"]["rejected"]
        .as_array()
        .expect("error_kinds.rejected is an array")
        .iter()
        .map(|e| &e["kind"])
        .collect();
    assert_eq!(
        rejected,
        [&json!("io_error")],
        "a declared *disk* kind is published in error_kinds"
    );
}

/// The audit of `r` has exactly `n` problem lines, each naming `name`.
fn audit_case(case: &str, r: &Registry, name: &str, n: usize) {
    let problems = r.audit();
    assert_eq!(
        problems.len(),
        n,
        "{case}: want {n} problem line(s) for {name}: {problems:#?}"
    );
    for p in &problems {
        assert!(
            p.contains(name),
            "{case}: the problem line names its command {name}: {p}"
        );
    }
}

/// AC-3. Test registries, one problem each, `audit()` returning exactly one line that names the
/// command: (i) no declaration; (ii) a declared *example* kind without a refusal example; (iii)
/// a refusal example that succeeds, one that refuses with another (declared) kind, one whose
/// params fail the schema, one whose setup fails, one whose `files` path is invalid, one whose
/// `hex` has an odd number of digits; (iv) a refusal example of an undeclared kind; (v) a
/// refusal example of a kind marked *disk*. No problem: a sound declaration with its witness, a
/// *disk* kind without an example, an empty declaration. With two undeclared commands, one
/// line each in name order (Pinned (c)). The standard registry's audit is empty (Interface 3;
/// `kiln-cli commands --audit` exiting 0 is asserted in `acceptance_t038_cli.rs`).
#[test]
fn audit_requires_a_witness_for_every_declared_kind() {
    let standard = standard_registry().expect("standard registry builds");
    let problems = standard.audit();
    assert!(
        problems.is_empty(),
        "the standard registry's audit is empty (Interface 3): {problems:#?}"
    );

    // No problem.
    audit_case("sound", &one::<Sound>(), "t38.sound", 0);
    audit_case(
        "disk kind without an example",
        &one::<SoundWithDisk>(),
        "t38.sound-disk",
        0,
    );
    audit_case(
        "an empty list is a declaration",
        &one::<DeclaresNothing>(),
        "t38.never",
        0,
    );
    audit_case(
        "disk kind only",
        &one::<DeclaresDisk>(),
        "t38.disk",
        0,
    );

    // One problem each.
    audit_case("(i) no declaration", &one::<Alpha>(), "t38.alpha", 1);
    audit_case(
        "(ii) example kind without a refusal example",
        &one::<NoWitness>(),
        "t38.no-witness",
        1,
    );
    audit_case(
        "(iii) refusal example succeeds",
        &one::<ExampleSucceeds>(),
        "t38.succeeds",
        1,
    );
    audit_case(
        "(iii) refusal example refuses with another kind",
        &one::<ExampleOtherKind>(),
        "t38.other-kind",
        1,
    );
    audit_case(
        "(iii) refusal example params fail the schema",
        &one::<ExampleBadParams>(),
        "t38.bad-params",
        1,
    );
    audit_case(
        "(iii) refusal example setup fails",
        &one::<ExampleSetupFails>(),
        "t38.setup-fails",
        1,
    );
    audit_case(
        "(iii) refusal example files path invalid",
        &one::<ExampleBadPath>(),
        "t38.bad-path",
        1,
    );
    audit_case(
        "(iii) refusal example hex odd",
        &one::<ExampleBadHex>(),
        "t38.bad-hex",
        1,
    );
    audit_case(
        "(iv) refusal example of an undeclared kind",
        &one::<ExampleUndeclared>(),
        "t38.undeclared",
        1,
    );
    audit_case(
        "(v) refusal example of a disk kind",
        &one::<ExampleOfDisk>(),
        "t38.of-disk",
        1,
    );

    // Two commands without a declaration: one line each, in name order.
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
