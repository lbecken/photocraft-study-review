//! T-004 / D-028 ruling 1: pins today's path behaviour on a real disk workspace — lexical `..`
//! segments and absolute paths are refused by `doc.open`, even when the named file exists outside
//! the root, and nothing outside the root changes. Symlink confinement is not tested here (task
//! T-008). Regression guard: passes on the base by design.
#![cfg(test)]

use std::path::{Path, PathBuf};

use kiln_commands::{CommandError, Context, standard_registry};
use kiln_workspace::DiskFs;
use serde_json::json;

fn fresh_dir(name: &str) -> PathBuf {
    let d = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("guard_t004_commands")
        .join(format!("{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

#[test]
fn lexical_dotdot_and_absolute_paths_are_refused_on_disk() {
    let work = fresh_dir("escape");
    let root = work.join("root");
    std::fs::create_dir_all(root.join("sub")).unwrap();
    std::fs::write(root.join("a.txt"), b"inside\n").unwrap();
    let outside = work.join("outside.txt");
    std::fs::write(&outside, b"outside\n").unwrap();

    let registry = standard_registry().unwrap();
    let mut ctx = Context::with_vfs(Box::new(DiskFs::new(&root).unwrap()));
    let abs = outside.to_str().unwrap().to_owned();
    for p in [
        "../outside.txt",
        "sub/../../outside.txt",
        "./../outside.txt",
        abs.as_str(),
    ] {
        let r = registry.execute(&mut ctx, "doc.open", json!({ "path": p }));
        assert!(
            matches!(r, Err(CommandError::Rejected { .. })),
            "`{p}` must be refused: {r:?}"
        );
    }
    assert!(ctx.log().is_empty(), "refused requests are not logged");
    // Positive control: a path inside the root opens as the first document.
    let r = registry
        .execute(&mut ctx, "doc.open", json!({"path": "a.txt"}))
        .unwrap();
    assert_eq!(r["doc"], json!(1), "{r}");
    assert_eq!(std::fs::read(&outside).unwrap(), b"outside\n");
}
