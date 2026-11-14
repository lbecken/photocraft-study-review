//! T-038 AC-6 (guard): the two refusal pairs of `nav.workspace-symbols` that only a race can
//! produce (`not_found`, `not_a_file`: a `.gitignore` removed, or a listed directory replaced by
//! a file, between the listing and the read) keep their kind through
//! `standard_registry().execute`; `project.tree {}` walks the same way and gives the same
//! kinds. The race is played by a test VFS over a `MemoryFs` that fails one call (ruling Q-5,
//! D-128: these pairs are declared *disk* and witnessed here). Written by the test author from
//! `tasks/T-038-error-kinds-match-commands.md` alone.
//!
//! Each case first runs on the same files with nothing failing and must succeed (so the
//! failing call is on the walk's path and the refusal is caused by it), and the failing call is
//! counted to show that it was reached. Passes on base; after T-038 it fails if the execute
//! check turns one of these declared refusals into `internal`. No message text is asserted.

#![cfg(test)]

use std::cell::Cell;
use std::rc::Rc;

use kiln_commands::{CommandError, Context, Registry, RejectReason, standard_registry};
use kiln_workspace::{DirListing, MemoryFs, RelPath, Vfs, VfsError};
use serde_json::{Value, json};

fn p(s: &str) -> RelPath {
    RelPath::new(s).expect("valid fixture path")
}

/// Which call the test VFS fails.
#[derive(Clone, Debug)]
enum Fail {
    /// Nothing fails (the control run).
    Nothing,
    /// `read` of this path fails with `NotFound` (the file was removed after the listing).
    ReadNotFound(RelPath),
    /// `list_dir` of this directory fails with `NotAFile` (replaced by a file after the
    /// listing of its parent).
    ListNotAFile(RelPath),
}

/// A `MemoryFs` whose one call named by `fail` fails; `hits` counts the failed calls.
#[derive(Debug)]
struct RaceFs {
    inner: MemoryFs,
    fail: Fail,
    hits: Rc<Cell<usize>>,
}

impl Vfs for RaceFs {
    fn read(&self, path: &RelPath) -> Result<Vec<u8>, VfsError> {
        if let Fail::ReadNotFound(at) = &self.fail
            && at == path
        {
            self.hits.set(self.hits.get() + 1);
            return Err(VfsError::NotFound { path: path.clone() });
        }
        self.inner.read(path)
    }
    fn write_atomic(&mut self, path: &RelPath, bytes: &[u8]) -> Result<(), VfsError> {
        self.inner.write_atomic(path, bytes)
    }
    fn is_file(&self, path: &RelPath) -> bool {
        self.inner.is_file(path)
    }
    fn list_dir(&self, dir: Option<&RelPath>) -> Result<DirListing, VfsError> {
        if let (Fail::ListNotAFile(at), Some(dir)) = (&self.fail, dir)
            && at == dir
        {
            self.hits.set(self.hits.get() + 1);
            return Err(VfsError::NotAFile { path: dir.clone() });
        }
        self.inner.list_dir(dir)
    }
    fn check_location(&self, path: &RelPath) -> Result<(), VfsError> {
        self.inner.check_location(path)
    }
}

/// The project: a root `.gitignore`, a `.rs` file at the root and one in `sub/`.
fn context(fail: Fail, hits: &Rc<Cell<usize>>) -> Context {
    let mut inner = MemoryFs::new();
    inner.insert(&p(".gitignore"), b"*.log\n");
    inner.insert(&p("a.rs"), b"fn alpha() {}\n");
    inner.insert(&p("sub/b.rs"), b"fn beta() {}\n");
    inner.insert(&p("sub/x.log"), b"ignored\n");
    Context::with_vfs(Box::new(RaceFs {
        inner,
        fail,
        hits: Rc::clone(hits),
    }))
}

fn reg() -> Registry {
    standard_registry().expect("standard registry builds")
}

/// One case: the control run succeeds with nothing failing, then the run with `fail` refuses
/// with `want` (code `rejected`, never `internal`), the failing call was reached, and nothing
/// is logged.
fn case(command: &str, params: Value, fail: Fail, want: RejectReason) {
    let what = format!("{command} {params} with {fail:?}");
    let r = reg();

    let hits = Rc::new(Cell::new(0));
    let mut ctx = context(Fail::Nothing, &hits);
    let control = r.execute(&mut ctx, command, params.clone());
    assert!(
        control.is_ok(),
        "precondition: {command} {params} succeeds when nothing fails: {control:?}"
    );

    let hits = Rc::new(Cell::new(0));
    let mut ctx = context(fail, &hits);
    let got = r.execute(&mut ctx, command, params);
    assert!(
        hits.get() >= 1,
        "precondition: {what}: the failing call was never made, so the case does not test the \
         race"
    );
    match &got {
        Err(CommandError::Rejected { reason, .. }) if *reason == want => {}
        other => panic!(
            "{what}: want rejected/{} (declared *disk*, Q-5), got {other:?}",
            want.as_str()
        ),
    }
    assert!(ctx.log().is_empty(), "{what}: a refusal is not logged");
}

/// AC-6: `nav.workspace-symbols {query: "a"}` and `project.tree {}` with the root `.gitignore`
/// read failing `NotFound` → `rejected`/`not_found`; with `list_dir` of `sub` failing
/// `NotAFile` → `rejected`/`not_a_file`.
#[test]
fn race_refusals_keep_their_kind() {
    let gitignore_gone = Fail::ReadNotFound(p(".gitignore"));
    let sub_replaced = Fail::ListNotAFile(p("sub"));
    for (command, params) in [
        ("nav.workspace-symbols", json!({"query": "a"})),
        ("project.tree", json!({})),
    ] {
        case(
            command,
            params.clone(),
            gitignore_gone.clone(),
            RejectReason::NotFound,
        );
        case(
            command,
            params,
            sub_replaced.clone(),
            RejectReason::NotAFile,
        );
    }
}
