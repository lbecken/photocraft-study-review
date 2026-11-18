//! T-092 guards (test author), written from `tasks/T-092-mark-product-tests-for-windows.md`
//! (Interface (1), Interface (2), AC-2) alone. Every test here passes on base and must pass after
//! T-092.
//!
//! 1. AC-2: each of the 24 P rows' tests of Interface (1) (after the cut, D-273) exists as exactly
//!    one `#[test]` fn of its file and has no platform cfg attribute (`cfg` with `unix`,
//!    `windows` or a `target_*` key, or a `cfg_attr` naming one) on the fn, on the inline
//!    `mod`s around it, on the file (`#![cfg]`) or on a `mod` declaration of the file (the
//!    reading of `guard_t079.rs` AC-2 without its in-body clause: a P test may hold paired
//!    platform statements, which `no-skip` checks; a `cfg!(…)` expression is not an attribute).
//! 2. No test of the 10 files T-092 touches (the 9 test files and `disk.rs`) is ignored: the
//!    list of `#[ignore]` and `#[cfg_attr(…, ignore)]` attributes in them equals the empty list
//!    (Out of scope: "`#[ignore]`, skips and early returns are not used").
//! 3. Interface (2), "Other entries are not changed": each entry of the base's registry other
//!    than the two `disk.rs` entries T-092 rewrites still exists exactly once with the base's
//!    `(file, item, platforms, counterpart, task)`. Entries are compared one by one, never by the
//!    file's count, so a later task's new entries do not break it (D-249); a later task that
//!    changes one of these entries (T-076, T-077 resolving their pending ones) updates the list
//!    below, as it updates `acceptance_t079.rs` for the same entries. Not compared: `doc`,
//!    `quote`, `reason`, `expires` (T-100 rewrites four `reason`s).
//!
//! Only `std::fs`, `toml` and `syn` are used (the same on Linux, macOS and Windows); paths are
//! `/`-separated repository-relative strings. No process is spawned.
#![cfg(test)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use syn::visit::{self, Visit};

const REGISTRY: &str = "xtask/platform-tests.toml";
const DISK: &str = "crates/kiln-workspace/src/disk.rs";
const WS: &str = "crates/kiln-workspace";

const T002: &str = "crates/kiln-workspace/tests/acceptance_t002.rs";
const T002_R2: &str = "crates/kiln-workspace/tests/acceptance_t002_r2.rs";
const T008: &str = "crates/kiln-workspace/tests/acceptance_t008.rs";
const T015_TOOLS: &str = "crates/kiln-workspace/tests/acceptance_t015_tools.rs";
const T036_WS: &str = "crates/kiln-workspace/tests/acceptance_t036_workspace.rs";
const CONFINEMENT: &str = "crates/kiln-workspace/tests/confinement.rs";
const G008: &str = "crates/kiln-workspace/tests/guard_t008.rs";
const G036_REMOVAL: &str = "crates/kiln-workspace/tests/guard_t036_removal_workspace.rs";
const G036_WS: &str = "crates/kiln-workspace/tests/guard_t036_workspace.rs";

/// The 10 files T-092 touches in `kiln-workspace`.
const TOUCHED: [&str; 10] = [
    T002,
    T002_R2,
    T008,
    T015_TOOLS,
    T036_WS,
    CONFINEMENT,
    G008,
    G036_REMOVAL,
    G036_WS,
    DISK,
];

/// Interface (1): the 24 P rows (W1; W2-W9; W12-W14; the originals of W17, W28, W35; W18, W20,
/// W21; W22; W25, W26, W29, W31, W34).
const PORTABLE: [(&str, &str); 24] = [
    (T002, "disk_save_replaces_via_rename_without_leftovers"),
    (T002_R2, "save_leaves_regular_files_at_temp_names_untouched"),
    (T002_R2, "save_never_writes_through_symlinks_at_temp_names"),
    (
        T002_R2,
        "save_ignores_self_pointing_and_dangling_symlinks_at_temp_names",
    ),
    (
        T002_R2,
        "documents_named_like_temp_files_do_not_clobber_each_other",
    ),
    (T002_R2, "file_with_255_byte_name_opens_edits_and_saves"),
    (T002_R2, "not_found_names_the_requested_path"),
    (T002_R2, "save_onto_directory_is_not_a_file"),
    (
        T002_R2,
        "save_through_symlink_with_decoys_changes_only_the_link_target",
    ),
    (T008, "open_through_escaping_symlinks_is_refused"),
    (T008, "save_through_escaping_symlinks_touches_nothing"),
    (T008, "link_swapped_in_after_open_is_refused_at_save"),
    (T008, "outside_root_wins_over_not_found"),
    (T008, "root_swapped_between_requests_is_refused"),
    (T008, "root_removed_and_replaced_by_link_is_refused"),
    (
        T008,
        "root_removed_and_replaced_by_new_directory_is_refused",
    ),
    (T015_TOOLS, "no_tools_never_runs"),
    (CONFINEMENT, "escapes_are_refused_at_read_write_and_is_file"),
    (
        CONFINEMENT,
        "links_inside_and_excursions_back_inside_are_followed",
    ),
    (CONFINEMENT, "swaps_between_open_and_save_are_checked_again"),
    (CONFINEMENT, "walk_corner_cases"),
    (G008, "symlinks_inside_root_still_work"),
    (G036_WS, "retargeted_root_link_is_refused"),
    (G036_WS, "vanished_root_is_refused"),
];

/// The base's registry entries (main `76ccb9f`, 35 entries) without the two `disk.rs` entries
/// pending on T-092, as `(file, item, platforms, counterpart, task)`.
#[rustfmt::skip]
const BASE_OTHERS: [(&str, &str, &str, &str, Option<&str>); 33] = [
    ("xtask/src/queue.rs", "tests::basic_refusals_write_nothing", "linux", "none", None),
    ("xtask/src/worktree.rs", "tests::create_seed_list_remove", "linux", "none", None),
    ("xtask/src/publish.rs", "tests::a_failed_content_commit_in_step_four_exits_one", "unix", "none", None),
    ("xtask/tests/acceptance_t013_publish.rs", "publish_refuses_while_the_queue_lock_is_held_and_releases_it_for_the_queue", "linux", "none", None),
    ("xtask/tests/guard_t005.rs", "worktree_build_does_not_touch_main_target", "unix", "none", None),
    ("xtask/tests/acceptance_t005_r3.rs", "tampered_cached_gate_binary_does_not_decide_the_gate", "linux", "none", None),
    ("xtask/tests/acceptance_t005_r4.rs", "planted_future_dated_xtask_does_not_decide_the_gate", "linux", "none", None),
    ("xtask/src/queue.rs", "tests::main_xtask_is_rebuilt_from_base_on_every_run", "unix", "none", None),
    ("apps/kiln/tests/acceptance_t055.rs", "ac8_stdout_write_failure_exits_1", "linux", "pending", Some("T-076")),
    ("apps/kiln-cli/tests/acceptance_t004_r2.rs", "run_and_control_exit_1_when_stdout_cannot_be_written", "linux", "pending", Some("T-076")),
    ("apps/kiln-cli/tests/run.rs", "run_exits_1_when_stdout_cannot_be_written", "linux", "pending", Some("T-076")),
    ("crates/kiln-workspace/tests/acceptance_t019_workspace.rs", "disk_list_dir_reports_entries_by_their_own_kind_and_skips_bad_names_non_utf8", "linux", "none", None),
    ("crates/kiln-workspace/tests/acceptance_t019_workspace.rs", "tree_counts_skipped_names_on_disk_non_utf8", "linux", "none", None),
    ("crates/kiln-workspace/tests/project.rs", "disk_list_dir_reports_own_kinds_sorted_and_skips_odd_names_non_utf8", "linux", "none", None),
    ("crates/kiln-workspace/tests/project.rs", "documented_differences_from_git_non_utf8", "linux", "none", None),
    ("apps/kiln/tests/acceptance_t065.rs", "ac1_no_display_exits_2_with_one_line", "linux", "pending", Some("T-077")),
    ("apps/kiln/tests/acceptance_t065.rs", "ac1_no_display_every_window_form", "linux", "pending", Some("T-077")),
    ("apps/kiln/tests/acceptance_t065.rs", "ac1_wayland_only_or_unreachable_display_exits_2", "linux", "pending", Some("T-077")),
    ("apps/kiln/tests/window_t065.rs", "without_a_display_the_window_exits_2_with_one_line", "linux", "pending", Some("T-077")),
    ("apps/kiln/tests/guard_t055.rs", "kiln_without_subcommand_and_display_exits_nonzero_with_a_message", "linux", "pending", Some("T-077")),
    ("crates/kiln-workspace/src/disk.rs", "tests::symlink_at_temp_name_is_not_followed", "unix", "crates/kiln-workspace/src/disk.rs::tests::link_at_temp_name_is_not_followed_on_every_platform", None),
    ("crates/kiln-workspace/src/disk.rs", "tests::temp_file_is_created_owner_only_and_gets_target_mode_before_rename", "unix", "none", None),
    ("crates/kiln-workspace/src/disk.rs", "tests::read_only_target_is_refused_unless_root", "unix", "crates/kiln-workspace/tests/guard_t073.rs::acw2_save_of_read_only_target_is_refused_and_changes_nothing", None),
    ("crates/kiln-workspace/src/disk.rs", "tests::save_creates_temp_owner_only_for_an_existing_target", "unix", "none", None),
    ("crates/kiln-workspace/src/disk.rs", "tests::new_file_gets_default_mode", "unix", "none", None),
    ("crates/kiln-workspace/src/disk.rs", "tests::replace_swaps_inode_keeps_permissions_and_follows_symlinks", "unix", "crates/kiln-workspace/src/disk.rs::tests::save_through_a_link_on_every_platform", None),
    ("crates/kiln-workspace/src/disk.rs", "tests::root_replaced_by_a_symlink_between_requests_is_outside", "unix", "crates/kiln-workspace/src/disk.rs::tests::root_replaced_by_a_link_on_every_platform", None),
    ("crates/kiln-workspace/src/disk.rs", "tests::parent_replaced_by_a_symlink_between_requests_is_outside", "unix", "crates/kiln-workspace/tests/guard_t073.rs::acw2_root_parent_replaced_is_outside_root", None),
    ("crates/kiln-workspace/src/disk.rs", "tests::root_removed_and_replaced_by_a_link_to_a_new_directory_is_outside", "unix", "crates/kiln-workspace/src/disk.rs::tests::root_removed_on_every_platform", None),
    ("crates/kiln-workspace/src/disk.rs", "tests::root_given_through_a_link_that_is_retargeted", "unix", "crates/kiln-workspace/src/disk.rs::tests::root_given_through_a_link_on_every_platform", None),
    ("crates/kiln-workspace/src/disk.rs", "tests::link_target_with_trailing_slash_requires_a_directory", "unix", "crates/kiln-workspace/src/disk.rs::tests::save_through_a_link_on_every_platform", None),
    ("crates/kiln-workspace/src/disk.rs", "tests::check_location_is_ok_for_every_inside_location", "unix", "crates/kiln-workspace/src/disk.rs::tests::check_location_inside_on_every_platform", None),
    ("crates/kiln-workspace/src/disk.rs", "tests::check_location_refuses_outside_loops_and_a_replaced_root", "unix", "crates/kiln-workspace/src/disk.rs::tests::check_location_outside_on_every_platform", None),
];

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask has a parent directory")
        .to_path_buf()
}

// ---- the sources --------------------------------------------------------------------------

/// The Linux/macOS/Windows set a cfg predicate holds on, as bits (1 Linux, 2 macOS, 4 Windows);
/// `Err` for a predicate this reader does not know (it is then reported, never guessed).
fn platforms_of(m: &syn::Meta) -> Result<u8, String> {
    const ALL: u8 = 7;
    let text = |m: &syn::Meta| match m {
        syn::Meta::Path(p) => p.get_ident().map(ToString::to_string).unwrap_or_default(),
        syn::Meta::NameValue(nv) => format!(
            "{} = …",
            nv.path
                .get_ident()
                .map(ToString::to_string)
                .unwrap_or_default()
        ),
        syn::Meta::List(l) => format!(
            "{}(…)",
            l.path
                .get_ident()
                .map(ToString::to_string)
                .unwrap_or_default()
        ),
    };
    match m {
        syn::Meta::Path(p) if p.is_ident("test") => Ok(ALL),
        syn::Meta::Path(p) if p.is_ident("unix") => Ok(1 | 2),
        syn::Meta::Path(p) if p.is_ident("windows") => Ok(4),
        syn::Meta::NameValue(nv) => {
            let v = match &nv.value {
                syn::Expr::Lit(syn::ExprLit {
                    lit: syn::Lit::Str(s),
                    ..
                }) => s.value(),
                _ => return Err(text(m)),
            };
            let key = nv
                .path
                .get_ident()
                .map(ToString::to_string)
                .unwrap_or_default();
            match (key.as_str(), v.as_str()) {
                ("target_os", "linux") => Ok(1),
                ("target_os", "macos") => Ok(2),
                ("target_os", "windows") => Ok(4),
                ("target_family", "unix") => Ok(1 | 2),
                ("target_family", "windows") => Ok(4),
                _ => Err(format!("{key} = {v:?}")),
            }
        }
        syn::Meta::List(l) => {
            let args: Vec<syn::Meta> = nested(l);
            if l.path.is_ident("not") && args.len() == 1 {
                Ok(ALL & !platforms_of(&args[0])?)
            } else if l.path.is_ident("all") {
                args.iter().try_fold(ALL, |a, m| Ok(a & platforms_of(m)?))
            } else if l.path.is_ident("any") {
                args.iter().try_fold(0, |a, m| Ok(a | platforms_of(m)?))
            } else {
                Err(text(m))
            }
        }
        _ => Err(text(m)),
    }
}

fn nested(l: &syn::MetaList) -> Vec<syn::Meta> {
    l.parse_args_with(syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated)
        .unwrap_or_else(|e| panic!("precondition: unreadable attribute arguments: {e}"))
        .into_iter()
        .collect()
}

/// Whether a cfg predicate names a platform (`unix`, `windows`, a `target_*` key).
fn is_platform(m: &syn::Meta) -> bool {
    let named = |p: &syn::Path| {
        p.is_ident("unix")
            || p.is_ident("windows")
            || p.get_ident()
                .is_some_and(|i| i.to_string().starts_with("target_"))
    };
    match m {
        syn::Meta::Path(p) => named(p),
        syn::Meta::NameValue(nv) => named(&nv.path),
        syn::Meta::List(l) => named(&l.path) || nested(l).iter().any(is_platform),
    }
}

fn set_name(bits: Result<u8, String>) -> String {
    match bits {
        Ok(3) => "unix".to_owned(),
        Ok(4) => "windows".to_owned(),
        Ok(1) => "linux".to_owned(),
        Ok(2) => "macos".to_owned(),
        Ok(7) => "every platform".to_owned(),
        Ok(b) => format!("platform set {b:#05b}"),
        Err(t) => format!("unknown predicate {t}"),
    }
}

/// The platform cfg attributes among `attrs`, each as `<at>=<set>` (`#[cfg(p)]` by the set `p`
/// holds on; any `cfg_attr` naming a platform in its predicate or what it applies as
/// `<at>=cfg_attr`, a form no row uses).
fn platform_cfgs(attrs: &[syn::Attribute], at: &str, out: &mut Vec<String>) {
    for a in attrs {
        let syn::Meta::List(l) = &a.meta else {
            continue;
        };
        if a.path().is_ident("cfg") {
            if let Some(p) = nested(l).into_iter().next()
                && is_platform(&p)
            {
                out.push(format!("{at}={}", set_name(platforms_of(&p))));
            }
        } else if a.path().is_ident("cfg_attr") && nested(l).iter().any(is_platform) {
            out.push(format!("{at}=cfg_attr({})", l.tokens));
        }
    }
}

fn rel(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or_else(|_| panic!("{} is not under {}", path.display(), root.display()))
        .components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/")
}

/// Every `.rs` file under `crates/kiln-workspace` (the crate of every file read here).
fn crate_files(root: &Path) -> Vec<String> {
    let mut out = Vec::new();
    let mut stack = vec![root.join(WS)];
    while let Some(d) = stack.pop() {
        let entries = std::fs::read_dir(&d)
            .unwrap_or_else(|e| panic!("precondition: cannot read {}: {e}", d.display()));
        for entry in entries {
            let p = entry.expect("a directory entry").path();
            if p.is_dir() {
                if p.file_name().is_some_and(|n| n != "target") {
                    stack.push(p);
                }
            } else if p.extension().is_some_and(|x| x == "rs") {
                out.push(rel(root, &p));
            }
        }
    }
    out.sort();
    out
}

fn parse(root: &Path, file: &str) -> syn::File {
    let p = root.join(file);
    let text = std::fs::read_to_string(&p)
        .unwrap_or_else(|e| panic!("precondition: cannot read {}: {e}", p.display()));
    syn::parse_file(&text).unwrap_or_else(|e| panic!("precondition: {file} does not parse: {e}"))
}

/// The attributes of every `mod name;` declaration in the crate that may load `file` (both
/// directory conventions and `#[path]`; an over-approximation only adds findings).
fn declaration_attrs(root: &Path, file: &str) -> Vec<syn::Attribute> {
    let mut out = Vec::new();
    for f in crate_files(root) {
        let dir = f.rsplit_once('/').map_or("", |(d, _)| d).to_owned();
        let stem = f
            .rsplit('/')
            .next()
            .unwrap_or_default()
            .trim_end_matches(".rs")
            .to_owned();
        for item in parse(root, &f).items {
            let syn::Item::Mod(m) = item else { continue };
            if m.content.is_some() {
                continue;
            }
            let name = m.ident.to_string();
            let mut candidates = vec![
                format!("{dir}/{name}.rs"),
                format!("{dir}/{name}/mod.rs"),
                format!("{dir}/{stem}/{name}.rs"),
                format!("{dir}/{stem}/{name}/mod.rs"),
            ];
            for a in &m.attrs {
                if a.path().is_ident("path")
                    && let syn::Meta::NameValue(nv) = &a.meta
                    && let syn::Expr::Lit(syn::ExprLit {
                        lit: syn::Lit::Str(s),
                        ..
                    }) = &nv.value
                {
                    candidates = vec![format!("{dir}/{}", s.value().replace('\\', "/"))];
                }
            }
            if candidates.iter().any(|c| c == file) {
                out.extend(m.attrs.iter().cloned());
            }
        }
    }
    out
}

fn is_test_attr(a: &syn::Attribute) -> bool {
    a.path().segments.last().is_some_and(|s| s.ident == "test")
}

/// Every `#[test]` fn of `file` (any inline module depth) as `<mod path>::<fn>` → the platform
/// cfgs bearing on it (`file=…` for the file's `#![cfg]` or its `mod` declaration, `mod <m>=…`,
/// `fn=…`). A name defined twice is listed with `#2` appended to the second.
fn tests_of(root: &Path, file: &str) -> BTreeMap<String, Vec<String>> {
    fn walk(
        items: &[syn::Item],
        prefix: &str,
        outer: &[String],
        out: &mut BTreeMap<String, Vec<String>>,
    ) {
        for item in items {
            match item {
                syn::Item::Fn(f) if f.attrs.iter().any(is_test_attr) => {
                    let mut cfgs = outer.to_vec();
                    platform_cfgs(&f.attrs, "fn", &mut cfgs);
                    let mut name = format!("{prefix}{}", f.sig.ident);
                    if out.contains_key(&name) {
                        name.push_str("#2");
                    }
                    out.insert(name, cfgs);
                }
                syn::Item::Mod(m) => {
                    if let Some((_, inner)) = &m.content {
                        let mut cfgs = outer.to_vec();
                        platform_cfgs(&m.attrs, &format!("mod {}", m.ident), &mut cfgs);
                        walk(inner, &format!("{prefix}{}::", m.ident), &cfgs, out);
                    }
                }
                _ => {}
            }
        }
    }
    let parsed = parse(root, file);
    let mut outer = Vec::new();
    platform_cfgs(&declaration_attrs(root, file), "file", &mut outer);
    platform_cfgs(&parsed.attrs, "file", &mut outer);
    let mut out = BTreeMap::new();
    walk(&parsed.items, "", &outer, &mut out);
    out
}

// ---- the guards ---------------------------------------------------------------------------

/// AC-2: the 24 P tests exist once each and carry no platform cfg attribute. Rejects:
/// `#[cfg(unix)]` on `guard_t008.rs::symlinks_inside_root_still_work`; `#![cfg(unix)]` on
/// `confinement.rs`; a P test renamed (for example W35's original moved to
/// `vanished_root_is_refused_portable`) or written twice as `#[cfg(unix)]`/`#[cfg(windows)]`
/// twins; `#[cfg_attr(windows, cfg(any()))]` on a P test.
#[test]
fn ac2_portable_tests_exist_once_and_carry_no_platform_cfg() {
    let root = root();
    let mut problems = Vec::new();
    let mut by_file: BTreeMap<&str, BTreeMap<String, Vec<String>>> = BTreeMap::new();
    for (file, _) in PORTABLE {
        by_file.entry(file).or_insert_with(|| tests_of(&root, file));
    }
    for (file, name) in PORTABLE {
        let tests = &by_file[file];
        match tests.get(name) {
            None => problems.push(format!("{file}::{name}: no such #[test] fn")),
            Some(cfgs) if !cfgs.is_empty() => problems.push(format!(
                "{file}::{name}: behind a platform cfg: {}",
                cfgs.join(", ")
            )),
            Some(_) => {}
        }
        if tests.contains_key(&format!("{name}#2")) {
            problems.push(format!("{file}::{name}: defined more than once"));
        }
    }
    assert!(
        problems.is_empty(),
        "T-092's portable tests (Interface (1), P rows) must each exist once with no platform \
         cfg attribute (AC-2):\n{}",
        problems.join("\n")
    );
}

/// Every `ignore` attribute in a file: `#[ignore]`, `#[ignore = "…"]`, `#[cfg_attr(…, ignore)]`.
struct Ignores<'s> {
    file: &'s str,
    item: String,
    out: &'s mut Vec<String>,
}

impl<'ast> Visit<'ast> for Ignores<'_> {
    fn visit_item_fn(&mut self, f: &'ast syn::ItemFn) {
        let outer = std::mem::replace(&mut self.item, f.sig.ident.to_string());
        visit::visit_item_fn(self, f);
        self.item = outer;
    }

    fn visit_attribute(&mut self, a: &'ast syn::Attribute) {
        let ignore = a.path().is_ident("ignore")
            || (a.path().is_ident("cfg_attr")
                && matches!(&a.meta, syn::Meta::List(l) if l.tokens.clone().into_iter().any(
                    |t| matches!(t, proc_macro2::TokenTree::Ident(i) if i == "ignore")
                )));
        if ignore {
            self.out
                .push(format!("{} (in or on fn `{}`)", self.file, self.item));
        }
        visit::visit_attribute(self, a);
    }
}

/// No test of the touched files is ignored: the list of `ignore` attributes in the 10 files
/// equals the empty list. Rejects: `#[ignore]` on `acceptance_t015_tools.rs::no_tools_never_runs`;
/// `#[cfg_attr(windows, ignore)]` on any test of these files.
#[test]
fn no_test_of_the_touched_files_is_ignored() {
    let root = root();
    let mut found = Vec::new();
    for file in TOUCHED {
        let parsed = parse(&root, file);
        Ignores {
            file,
            item: String::new(),
            out: &mut found,
        }
        .visit_file(&parsed);
    }
    assert_eq!(
        found,
        Vec::<String>::new(),
        "ignore attributes in the files T-092 touches (none may be used)"
    );
}

/// Interface (2), "Other entries are not changed": each base entry other than the two `disk.rs`
/// ones T-092 rewrites still exists exactly once with its base `(file, item, platforms,
/// counterpart, task)`. Rejects: T-077's `ac1_no_display_every_window_form` given a counterpart
/// by this task; an entry of `project.rs` (T-100's) moved to `platforms = "unix"`; a base entry
/// dropped or entered twice. New entries elsewhere are not compared.
#[test]
fn registry_entries_of_other_tasks_are_as_on_base() {
    let p = root().join(REGISTRY);
    let text = std::fs::read_to_string(&p)
        .unwrap_or_else(|e| panic!("precondition: cannot read {}: {e}", p.display()));
    let table: toml::Table = text
        .parse()
        .unwrap_or_else(|e| panic!("{REGISTRY} is not TOML: {e}"));
    let tests = table
        .get("test")
        .and_then(toml::Value::as_array)
        .unwrap_or_else(|| panic!("{REGISTRY} has no array of `test` tables"));
    let get = |t: &toml::Value, k: &str| -> Option<String> {
        t.get(k).map(|v| match v.as_str() {
            Some(s) => s.to_owned(),
            None => format!("<not a string: {v}>"),
        })
    };
    let mut problems = Vec::new();
    for (file, item, platforms, counterpart, task) in BASE_OTHERS {
        let mine: Vec<&toml::Value> = tests
            .iter()
            .filter(|t| {
                get(t, "file").as_deref() == Some(file) && get(t, "item").as_deref() == Some(item)
            })
            .collect();
        let [one] = mine.as_slice() else {
            problems.push(format!(
                "{file}::{item}: expected exactly one entry, found {}",
                mine.len()
            ));
            continue;
        };
        let have = (
            get(one, "platforms"),
            get(one, "counterpart"),
            get(one, "task"),
        );
        let want = (
            Some(platforms.to_owned()),
            Some(counterpart.to_owned()),
            task.map(str::to_owned),
        );
        if have != want {
            problems.push(format!(
                "{file}::{item}: (platforms, counterpart, task) = {have:?}, base has {want:?}"
            ));
        }
    }
    assert!(
        problems.is_empty(),
        "{REGISTRY}: entries of other tasks differ from the base (Interface (2): other entries \
         are not changed):\n{}",
        problems.join("\n")
    );
}
