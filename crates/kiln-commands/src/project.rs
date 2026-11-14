//! The `project.*` command group: `project.tree` (T-019), the project listed as a tree.
//!
//! The project is the session root. The walk is `kiln_workspace::project::tree`; its rules
//! are in `docs/protocol/project.md`.

use kiln_workspace::project::{self as walk, ProjectTree};
use kiln_workspace::{EntryKind, RelPath, VfsError};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::reason::RejectReason;
use crate::registry::{
    Command, CommandError, Context, Example, Refusal, RefusalExample, Registry, RegistryError,
    SeedFile,
};

/// Register the `project.*` commands.
pub(crate) fn register(registry: &mut Registry) -> Result<(), RegistryError> {
    registry.register::<Tree>()?;
    Ok(())
}

/// Deserialize an optional parameter that is present: `null` is not a value for it (an absent
/// parameter is `None` through `#[serde(default)]`). Ruling x1 (D-084).
fn present<'de, D, T>(d: D) -> Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    T::deserialize(d).map(Some)
}

/// Drop the `"default": null` that `#[serde(default)]` adds to the schema of `path`: an absent
/// `path` is the root, and `null` is not a value of the bare type.
fn no_default(schema: &mut schemars::Schema) {
    schema.remove("default");
}

/// Default of `limit`.
const DEFAULT_LIMIT: u64 = 50_000;

fn default_limit() -> u64 {
    DEFAULT_LIMIT
}

/// Params of `project.tree`.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct TreeParams {
    /// Root-relative path of the directory to list (`/`-separated; no `.`, `..` or empty
    /// segments). Omit it for the root; `null` is refused.
    #[serde(default, deserialize_with = "present")]
    #[schemars(with = "String", transform = no_default)]
    path: Option<String>,
    /// The most entries to list (1 to 1,000,000; default 50,000). Ignored entries do not count.
    #[serde(default = "default_limit")]
    #[schemars(range(min = 1, max = 1_000_000))]
    limit: u64,
}

/// Kind of a listed entry (its own kind: a symlink is never followed).
#[derive(Clone, Copy, Debug, Serialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub(crate) enum KindName {
    /// A regular file.
    File,
    /// A directory.
    Dir,
    /// A symbolic link, whatever it points to; never followed.
    Symlink,
    /// A FIFO, socket or device; never opened.
    Other,
}

/// One listed entry.
#[derive(Debug, Serialize, JsonSchema)]
pub(crate) struct TreeEntryOut {
    /// The entry's kind.
    kind: KindName,
    /// Root-relative path; it begins with the requested `path` as given.
    path: String,
}

/// Result of `project.tree`.
#[derive(Debug, Serialize, JsonSchema)]
pub(crate) struct TreeResult {
    /// Entries in preorder; each directory's entries in byte order of their names.
    entries: Vec<TreeEntryOut>,
    /// Entries not listed (and not descended) because they are the directory whose path is
    /// exactly `target`, are named `.git`, or are ignored by a `.gitignore`.
    ignored: usize,
    /// Names not listed because they are not UTF-8 or not a valid path segment.
    skipped: usize,
    /// Directories listed and not descended into because their path has 128 or more segments.
    too_deep: usize,
    /// True iff an entry beyond `limit` exists.
    truncated: bool,
}

fn kind_name(kind: EntryKind) -> KindName {
    match kind {
        EntryKind::File => KindName::File,
        EntryKind::Dir => KindName::Dir,
        EntryKind::Symlink => KindName::Symlink,
        EntryKind::Other => KindName::Other,
    }
}

fn result(tree: ProjectTree) -> TreeResult {
    TreeResult {
        entries: tree
            .entries
            .into_iter()
            .map(|e| TreeEntryOut {
                kind: kind_name(e.kind),
                path: e.path.to_string(),
            })
            .collect(),
        ignored: tree.ignored,
        skipped: tree.skipped,
        too_deep: tree.too_deep,
        truncated: tree.truncated,
    }
}

/// The refusal for a failed walk; the message names a root-relative path only (ruling Q-4).
fn refusal(e: &VfsError) -> CommandError {
    let message = match e {
        VfsError::NotAFile { path } => format!("cannot list {path}: not a directory"),
        _ => format!("cannot list the project: {e}"),
    };
    CommandError::rejected(e.into(), message)
}

pub(crate) struct Tree;

impl Command for Tree {
    const NAME: &'static str = "project.tree";
    const DOC: &'static str = "List the project (or the directory `path`) as a tree, in \
        preorder with each directory's entries in byte order of their names. Honours the \
        project's `.gitignore` files, skips the directory whose path is exactly `target` and \
        every `.git`, lists symlinks and special files without following or opening them, and \
        lists a directory whose path has 128 or more segments without descending into it. Rules: docs/protocol/project.md.";
    const MUTATING: bool = false;
    type Params = TreeParams;
    type Output = TreeResult;

    fn examples() -> Vec<Example> {
        vec![
            Example {
                description: "list a project with a .gitignore and a target directory",
                initial_text: "",
                files: vec![
                    (".gitignore", "*.log\n/target\n"),
                    ("B.txt", ""),
                    ("a.rs", ""),
                    ("a/b.rs", ""),
                    ("x.log", ""),
                    ("target/debug/k", ""),
                    ("src/lib.rs", ""),
                ],
                setup: vec![],
                params: json!({}),
                result: json!({"entries": [
                        {"kind": "file", "path": ".gitignore"},
                        {"kind": "file", "path": "B.txt"},
                        {"kind": "dir", "path": "a"},
                        {"kind": "file", "path": "a/b.rs"},
                        {"kind": "file", "path": "a.rs"},
                        {"kind": "dir", "path": "src"},
                        {"kind": "file", "path": "src/lib.rs"}],
                    "ignored": 2, "skipped": 0, "too_deep": 0, "truncated": false}),
            },
            Example {
                description: "list one directory, at most one entry",
                initial_text: "",
                files: vec![("src/a.rs", ""), ("src/b.rs", "")],
                setup: vec![],
                params: json!({"path": "src", "limit": 1}),
                result: json!({"entries": [{"kind": "file", "path": "src/a.rs"}],
                    "ignored": 0, "skipped": 0, "too_deep": 0, "truncated": true}),
            },
        ]
    }

    fn refusals() -> Option<Vec<Refusal>> {
        Some(vec![
            Refusal::example(RejectReason::InvalidPath),
            Refusal::example(RejectReason::NotFound),
            Refusal::example(RejectReason::NotAFile),
            Refusal::disk(RejectReason::IoError),
            Refusal::disk(RejectReason::OutsideRoot),
        ])
    }

    fn refusal_examples() -> Vec<RefusalExample> {
        vec![
            RefusalExample {
                description: "a path that leaves the root",
                initial_text: "",
                files: vec![],
                setup: vec![],
                params: json!({"path": "../x"}),
                kind: RejectReason::InvalidPath,
            },
            RefusalExample {
                description: "a missing directory",
                initial_text: "",
                files: vec![],
                setup: vec![],
                params: json!({"path": "missing"}),
                kind: RejectReason::NotFound,
            },
            RefusalExample {
                description: "a file where a directory is needed",
                initial_text: "",
                files: vec![SeedFile::text("f.txt", "x")],
                setup: vec![],
                params: json!({"path": "f.txt"}),
                kind: RejectReason::NotAFile,
            },
        ]
    }

    fn run(ctx: &mut Context, p: TreeParams) -> Result<TreeResult, CommandError> {
        let under = p
            .path
            .as_deref()
            .map(RelPath::new)
            .transpose()
            .map_err(|e| CommandError::rejected(RejectReason::InvalidPath, e.to_string()))?;
        // The schema bounds `limit` to 1..=1,000,000, which fits every `usize` Kiln runs on.
        let limit = usize::try_from(p.limit).unwrap_or(usize::MAX);
        walk::tree(ctx.vfs(), under.as_ref(), limit)
            .map(result)
            .map_err(|e| refusal(&e))
    }
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    use crate::registry::Context;

    fn run(files: &[(&str, &str)], params: Value) -> Result<Value, Value> {
        let registry = crate::standard_registry().unwrap();
        let mut fs = kiln_workspace::MemoryFs::new();
        for (path, content) in files {
            fs.insert(
                &kiln_workspace::RelPath::new(path).unwrap(),
                content.as_bytes(),
            );
        }
        let mut ctx = Context::with_vfs(Box::new(fs));
        let out = registry
            .execute(&mut ctx, "project.tree", params)
            .map_err(|e| e.to_json());
        if out.is_err() {
            assert!(ctx.log().is_empty());
        }
        out
    }

    fn kind(e: &Value) -> (&str, &str) {
        (
            e["code"].as_str().unwrap_or(""),
            e["reason"].as_str().unwrap_or(""),
        )
    }

    #[test]
    fn refusals_carry_existing_kinds() {
        let files = [("a.rs", ""), ("d/x", "")];
        assert_eq!(
            kind(&run(&files, json!({"path": "a.rs"})).unwrap_err()),
            ("rejected", "not_a_file")
        );
        assert_eq!(
            kind(&run(&files, json!({"path": "nope"})).unwrap_err()),
            ("rejected", "not_found")
        );
        assert_eq!(
            kind(&run(&files, json!({"path": "../x"})).unwrap_err()),
            ("rejected", "invalid_path")
        );
        for bad in [
            json!({"limit": 0}),
            json!({"limit": 1_000_001}),
            json!({"path": null}),
            json!({"limit": null}),
            json!({"depth": 1}),
        ] {
            assert_eq!(kind(&run(&files, bad).unwrap_err()).0, "invalid_params");
        }
        assert!(run(&files, json!({"limit": 1_000_000})).is_ok());
    }

    #[test]
    fn listing_is_read_only_and_logged_as_such() {
        let registry = crate::standard_registry().unwrap();
        let mut fs = kiln_workspace::MemoryFs::new();
        fs.insert(&kiln_workspace::RelPath::new("d/x").unwrap(), b"");
        let mut ctx = Context::with_vfs(Box::new(fs));
        let out = registry
            .execute(&mut ctx, "project.tree", json!({"path": "d"}))
            .unwrap();
        assert_eq!(out["entries"], json!([{"kind": "file", "path": "d/x"}]));
        assert_eq!(ctx.log().len(), 1);
        assert!(!ctx.log()[0].mutating);
    }
}
