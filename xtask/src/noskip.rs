//! `cargo xtask no-skip`: no test may be ignored, skipped or compiled out, unless allowlisted in
//! `xtask/skips.toml` with a reason and an unexpired date.
//!
//! Detected (by parsing every `.rs` file with `syn`):
//! * `#[ignore]` on any item, and `#[cfg_attr(…, ignore)]`;
//! * in a test function: a call or macro whose name starts with `skip`, and a `return` inside a
//!   conditional, loop or `let … else` (an early exit that makes the test pass without running);
//!   closures and nested functions are not inspected;
//! * a `#[cfg(…)]` other than exactly `cfg(test)` on a test function, on a module that contains
//!   tests (inline, or the file it loads), or as an inner `#![cfg(…)]` of a file with tests;
//! * a doc-test fence marked `ignore`.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use anyhow::{Context as _, Result, bail};
use serde::Deserialize;
use syn::visit::{self, Visit};
use syn::{Attribute, Expr, ItemFn, ItemMod, Meta};

use crate::util::{StepResult, rel, rust_files};

/// What kind of skip was found.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Kind {
    /// `#[ignore]`.
    Ignore,
    /// `#[cfg_attr(…, ignore)]`.
    CfgAttrIgnore,
    /// A `skip*` call or macro in a test.
    SkipCall,
    /// A conditional `return` in a test.
    EarlyReturn,
    /// A test or test module behind a `cfg` other than `test`.
    CfgDisabled,
    /// A ```` ```ignore ```` doc test.
    DocIgnore,
}

impl Kind {
    /// The name used in `skips.toml`.
    pub fn name(self) -> &'static str {
        match self {
            Kind::Ignore => "ignore",
            Kind::CfgAttrIgnore => "cfg-attr-ignore",
            Kind::SkipCall => "skip-call",
            Kind::EarlyReturn => "early-return",
            Kind::CfgDisabled => "cfg-disabled",
            Kind::DocIgnore => "doc-ignore",
        }
    }
}

/// One finding.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Finding {
    /// Item path within the file (`tests::name`), or `<file>` for file-level findings.
    pub item: String,
    /// 1-based line.
    pub line: usize,
    /// Kind.
    pub kind: Kind,
}

fn path_last(path: &syn::Path) -> String {
    path.segments
        .last()
        .map(|s| s.ident.to_string())
        .unwrap_or_default()
}

fn is_test_attr(a: &Attribute) -> bool {
    path_last(a.path()) == "test" && !a.path().is_ident("cfg")
}

fn is_test_fn(attrs: &[Attribute]) -> bool {
    attrs.iter().any(is_test_attr)
}

/// A `#[cfg(...)]` that is not exactly `#[cfg(test)]`.
fn non_test_cfg(attrs: &[Attribute]) -> Option<&Attribute> {
    attrs.iter().find(|a| {
        a.path().is_ident("cfg")
            && match &a.meta {
                Meta::List(l) => l.tokens.to_string().trim() != "test",
                _ => true,
            }
    })
}

fn line_of(span: proc_macro2::Span) -> usize {
    span.start().line
}

fn contains_tests(items: &[syn::Item]) -> bool {
    struct Has(bool);
    impl<'a> Visit<'a> for Has {
        fn visit_item_fn(&mut self, f: &'a ItemFn) {
            if is_test_fn(&f.attrs) {
                self.0 = true;
            }
            visit::visit_item_fn(self, f);
        }
    }
    let mut h = Has(false);
    for i in items {
        h.visit_item(i);
    }
    h.0
}

/// Inspects a test function body.
struct Body<'f> {
    item: String,
    cond_depth: usize,
    out: &'f mut Vec<Finding>,
}

impl<'a> Visit<'a> for Body<'_> {
    fn visit_expr_closure(&mut self, _: &'a syn::ExprClosure) {}
    fn visit_item(&mut self, _: &'a syn::Item) {}
    fn visit_expr_if(&mut self, e: &'a syn::ExprIf) {
        self.cond_depth += 1;
        visit::visit_expr_if(self, e);
        self.cond_depth -= 1;
    }
    fn visit_expr_match(&mut self, e: &'a syn::ExprMatch) {
        self.cond_depth += 1;
        visit::visit_expr_match(self, e);
        self.cond_depth -= 1;
    }
    fn visit_expr_while(&mut self, e: &'a syn::ExprWhile) {
        self.cond_depth += 1;
        visit::visit_expr_while(self, e);
        self.cond_depth -= 1;
    }
    fn visit_expr_for_loop(&mut self, e: &'a syn::ExprForLoop) {
        self.cond_depth += 1;
        visit::visit_expr_for_loop(self, e);
        self.cond_depth -= 1;
    }
    fn visit_expr_loop(&mut self, e: &'a syn::ExprLoop) {
        self.cond_depth += 1;
        visit::visit_expr_loop(self, e);
        self.cond_depth -= 1;
    }
    fn visit_local(&mut self, l: &'a syn::Local) {
        if let Some(init) = &l.init {
            self.visit_expr(&init.expr);
            if let Some((_, diverge)) = &init.diverge {
                self.cond_depth += 1;
                self.visit_expr(diverge);
                self.cond_depth -= 1;
            }
        }
    }
    fn visit_expr_return(&mut self, r: &'a syn::ExprReturn) {
        if self.cond_depth > 0 {
            self.out.push(Finding {
                item: self.item.clone(),
                line: line_of(r.return_token.span),
                kind: Kind::EarlyReturn,
            });
        }
        visit::visit_expr_return(self, r);
    }
    fn visit_expr_call(&mut self, c: &'a syn::ExprCall) {
        if let Expr::Path(p) = &*c.func
            && path_last(&p.path).starts_with("skip")
        {
            self.out.push(Finding {
                item: self.item.clone(),
                line: line_of(p.path.segments[0].ident.span()),
                kind: Kind::SkipCall,
            });
        }
        visit::visit_expr_call(self, c);
    }
    fn visit_macro(&mut self, m: &'a syn::Macro) {
        if path_last(&m.path).starts_with("skip") {
            self.out.push(Finding {
                item: self.item.clone(),
                line: line_of(m.path.segments[0].ident.span()),
                kind: Kind::SkipCall,
            });
        }
    }
}

/// Walks one file's items.
struct FileScan<'f> {
    file: PathBuf,
    module: Vec<String>,
    out: &'f mut Vec<Finding>,
}

impl FileScan<'_> {
    fn name(&self, leaf: &str) -> String {
        let mut p = self.module.clone();
        p.push(leaf.to_owned());
        p.join("::")
    }

    fn check_attrs(&mut self, item: &str, attrs: &[Attribute]) {
        for a in attrs {
            if a.path().is_ident("ignore") {
                self.out.push(Finding {
                    item: item.into(),
                    line: line_of(a.pound_token.span),
                    kind: Kind::Ignore,
                });
            }
            if a.path().is_ident("cfg_attr")
                && let Meta::List(l) = &a.meta
                && l.tokens
                    .clone()
                    .into_iter()
                    .any(|t| matches!(t, proc_macro2::TokenTree::Ident(i) if i == "ignore"))
            {
                self.out.push(Finding {
                    item: item.into(),
                    line: line_of(a.pound_token.span),
                    kind: Kind::CfgAttrIgnore,
                });
            }
            if a.path().is_ident("doc")
                && let Meta::NameValue(nv) = &a.meta
                && let Expr::Lit(syn::ExprLit {
                    lit: syn::Lit::Str(s),
                    ..
                }) = &nv.value
                && s.value().lines().any(|l| {
                    let t = l.trim_start();
                    t.strip_prefix("```")
                        .is_some_and(|info| info.split([',', ' ']).any(|w| w.trim() == "ignore"))
                })
            {
                self.out.push(Finding {
                    item: item.into(),
                    line: line_of(a.pound_token.span),
                    kind: Kind::DocIgnore,
                });
            }
        }
    }

    fn module_file(&self, m: &ItemMod) -> Option<PathBuf> {
        let dir = self.file.parent()?;
        let stem = self.file.file_stem()?.to_str()?;
        // `lib.rs`/`main.rs`/`mod.rs` load children from their own directory; `foo.rs` from `foo/`.
        let base = if matches!(stem, "lib" | "main" | "mod") {
            dir.to_path_buf()
        } else {
            dir.join(stem)
        };
        let mut nested = base.clone();
        for m in &self.module {
            nested.push(m);
        }
        let name = m.ident.to_string();
        [
            nested.join(format!("{name}.rs")),
            nested.join(&name).join("mod.rs"),
        ]
        .into_iter()
        .find(|p| p.is_file())
    }
}

impl<'a> Visit<'a> for FileScan<'_> {
    fn visit_item_fn(&mut self, f: &'a ItemFn) {
        let item = self.name(&f.sig.ident.to_string());
        self.check_attrs(&item, &f.attrs);
        if is_test_fn(&f.attrs) {
            if let Some(a) = non_test_cfg(&f.attrs) {
                self.out.push(Finding {
                    item: item.clone(),
                    line: line_of(a.pound_token.span),
                    kind: Kind::CfgDisabled,
                });
            }
            let mut body = Body {
                item,
                cond_depth: 0,
                out: self.out,
            };
            for stmt in &f.block.stmts {
                body.visit_stmt(stmt);
            }
        }
        // Nested fns inside a fn are rare in tests; not descended.
    }

    fn visit_item_mod(&mut self, m: &'a ItemMod) {
        let name = m.ident.to_string();
        let item = self.name(&name);
        self.check_attrs(&item, &m.attrs);
        if let Some(a) = non_test_cfg(&m.attrs) {
            let has_tests = match &m.content {
                Some((_, items)) => contains_tests(items),
                None => self.module_file(m).is_some_and(|p| {
                    std::fs::read_to_string(p)
                        .ok()
                        .and_then(|s| syn::parse_file(&s).ok())
                        .is_some_and(|f| contains_tests(&f.items))
                }),
            };
            if has_tests {
                self.out.push(Finding {
                    item: item.clone(),
                    line: line_of(a.pound_token.span),
                    kind: Kind::CfgDisabled,
                });
            }
        }
        if m.content.is_some() {
            self.module.push(name);
            visit::visit_item_mod(self, m);
            self.module.pop();
        }
    }

    fn visit_item(&mut self, i: &'a syn::Item) {
        match i {
            syn::Item::Fn(f) => self.visit_item_fn(f),
            syn::Item::Mod(m) => self.visit_item_mod(m),
            syn::Item::Impl(imp) => {
                for ii in &imp.items {
                    if let syn::ImplItem::Fn(f) = ii {
                        self.check_attrs(&self.name(&f.sig.ident.to_string()), &f.attrs);
                    }
                }
                self.check_attrs("<impl>", &imp.attrs);
            }
            other => {
                let attrs: &[Attribute] = match other {
                    syn::Item::Struct(s) => &s.attrs,
                    syn::Item::Enum(e) => &e.attrs,
                    syn::Item::Trait(t) => &t.attrs,
                    syn::Item::Const(c) => &c.attrs,
                    syn::Item::Static(s) => &s.attrs,
                    syn::Item::Type(t) => &t.attrs,
                    syn::Item::Macro(m) => &m.attrs,
                    syn::Item::Use(u) => &u.attrs,
                    _ => &[],
                };
                self.check_attrs(&self.name("<item>"), attrs);
            }
        }
    }
}

/// Findings in one source file.
pub fn scan_file(file: &Path, source: &str) -> Result<Vec<Finding>> {
    let parsed = syn::parse_file(source).with_context(|| format!("parse {}", file.display()))?;
    let mut out = Vec::new();
    {
        let mut scan = FileScan {
            file: file.to_path_buf(),
            module: Vec::new(),
            out: &mut out,
        };
        scan.check_attrs("<file>", &parsed.attrs);
        if let Some(a) = non_test_cfg(&parsed.attrs)
            && contains_tests(&parsed.items)
        {
            let line = line_of(a.pound_token.span);
            scan.out.push(Finding {
                item: "<file>".into(),
                line,
                kind: Kind::CfgDisabled,
            });
        }
        for item in &parsed.items {
            scan.visit_item(item);
        }
    }
    out.sort();
    out.dedup();
    Ok(out)
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SkipFile {
    #[serde(default)]
    skip: Vec<SkipEntry>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SkipEntry {
    file: String,
    item: String,
    kind: Option<String>,
    reason: String,
    expires: String,
}

fn valid_date(d: &str) -> bool {
    d.len() == 10 && crate::timefmt::parse_utc(d).is_ok()
}

/// Apply the allowlist: returns problems (unlisted findings, expired or unused or malformed
/// entries). `findings` are (file relative to root, finding).
pub fn apply_allowlist(
    findings: &[(String, Finding)],
    entries_toml: &str,
    today: &str,
) -> Result<Vec<String>> {
    let file: SkipFile = toml::from_str(entries_toml).context("parse xtask/skips.toml")?;
    let mut problems = Vec::new();
    let mut used = BTreeSet::new();
    for (i, e) in file.skip.iter().enumerate() {
        if e.reason.trim().is_empty() {
            problems.push(format!(
                "skips.toml entry {} ({}): empty reason",
                i + 1,
                e.item
            ));
        }
        if !valid_date(&e.expires) {
            problems.push(format!(
                "skips.toml entry {} ({}): expires `{}` is not YYYY-MM-DD",
                i + 1,
                e.item,
                e.expires
            ));
        } else if e.expires.as_str() < today {
            problems.push(format!(
                "skips.toml entry {} ({} in {}): expired on {} (today is {today})",
                i + 1,
                e.item,
                e.file,
                e.expires
            ));
        }
    }
    for (path, f) in findings {
        let hit = file.skip.iter().position(|e| {
            e.file == *path
                && e.item == f.item
                && e.kind.as_deref().is_none_or(|k| k == f.kind.name())
        });
        match hit {
            Some(i) => {
                used.insert(i);
            }
            None => problems.push(format!(
                "{path}:{}: `{}` is a skipped test ({}); remove the skip or list it in xtask/skips.toml with a reason and expiry",
                f.line,
                f.item,
                f.kind.name()
            )),
        }
    }
    for (i, e) in file.skip.iter().enumerate() {
        if !used.contains(&i) {
            problems.push(format!(
                "skips.toml entry {} ({} in {}) matches nothing; remove it",
                i + 1,
                e.item,
                e.file
            ));
        }
    }
    Ok(problems)
}

/// Directories scanned, relative to the root.
const SCANNED: &[&str] = &["crates", "apps", "xtask"];
/// Excluded: the self-test fixtures.
const EXCLUDED: &[&str] = &["xtask/fixtures"];

/// Run the self-test, scan the workspace and apply the allowlist.
pub fn check(root: &Path, today: &str) -> Result<StepResult> {
    if !valid_date(today) {
        bail!("today `{today}` is not YYYY-MM-DD");
    }
    let fx = root.join("xtask/fixtures/no-skip");
    let bad_path = fx.join("violations.rs");
    let bad = scan_file(&bad_path, &std::fs::read_to_string(&bad_path)?)?;
    let kinds: BTreeSet<Kind> = bad.iter().map(|f| f.kind).collect();
    let all = BTreeSet::from([
        Kind::Ignore,
        Kind::CfgAttrIgnore,
        Kind::SkipCall,
        Kind::EarlyReturn,
        Kind::CfgDisabled,
        Kind::DocIgnore,
    ]);
    if kinds != all || bad.len() != 9 {
        bail!("self-test failed: violations fixture gave {bad:?}");
    }
    let clean_path = fx.join("clean.rs");
    let clean = scan_file(&clean_path, &std::fs::read_to_string(&clean_path)?)?;
    if !clean.is_empty() {
        bail!("self-test failed: clean fixture gave {clean:?}");
    }

    let mut findings = Vec::new();
    let mut files = 0;
    for dir in SCANNED {
        for f in rust_files(root, &root.join(dir), EXCLUDED)? {
            files += 1;
            let src = std::fs::read_to_string(&f)?;
            for finding in scan_file(&f, &src)? {
                findings.push((rel(root, &f), finding));
            }
        }
    }
    if files == 0 {
        bail!("no Rust files found to scan");
    }
    let skips =
        std::fs::read_to_string(root.join("xtask/skips.toml")).context("read xtask/skips.toml")?;
    let problems = apply_allowlist(&findings, &skips, today)?;
    let listed = findings.len();
    Ok(StepResult::from_problems(
        format!(
            "self-test ok; {files} files scanned, {listed} allowlisted skip(s), today = {today}"
        ),
        "skip problems",
        &problems,
    ))
}

/// "Today" for expiry purposes: the commit date of HEAD, so the result is a function of the commit.
pub fn today(root: &Path) -> Result<String> {
    crate::git::head_date(root)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scan(src: &str) -> Vec<(String, Kind)> {
        scan_file(Path::new("x.rs"), src)
            .unwrap()
            .into_iter()
            .map(|f| (f.item, f.kind))
            .collect()
    }

    #[test]
    fn fixture_findings_are_exact() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let p = root.join("fixtures/no-skip/violations.rs");
        let got: Vec<(String, Kind)> = scan_file(&p, &std::fs::read_to_string(&p).unwrap())
            .unwrap()
            .into_iter()
            .map(|f| (f.item, f.kind))
            .collect();
        use Kind::*;
        let mut want = vec![
            ("documented".to_string(), DocIgnore),
            ("tests::ignored".into(), Ignore),
            ("tests::ignored_on_linux".into(), CfgAttrIgnore),
            ("tests::skips_via_helper".into(), SkipCall),
            ("tests::skips_via_helper".into(), SkipCall),
            ("tests::returns_early".into(), EarlyReturn),
            ("tests::let_else_return".into(), EarlyReturn),
            ("tests::cfg_gated_test".into(), CfgDisabled),
            ("gated".into(), CfgDisabled),
        ];
        want.sort();
        let mut got = got;
        got.sort();
        assert_eq!(got, want);
    }

    #[test]
    fn unconditional_return_and_cfg_test_are_fine() {
        assert!(
            scan("#[cfg(test)] mod t { #[test] fn a() { assert!(true); return; } }").is_empty()
        );
    }

    #[test]
    fn file_level_cfg_on_test_file_is_reported() {
        assert_eq!(
            scan("#![cfg(feature = \"x\")]\n#[test] fn a() {}"),
            [("<file>".to_string(), Kind::CfgDisabled)]
        );
    }

    #[test]
    fn allowlist_admits_listed_unexpired_entries_only() {
        let f = vec![(
            "crates/a/src/lib.rs".to_string(),
            Finding {
                item: "tests::slow".into(),
                line: 3,
                kind: Kind::Ignore,
            },
        )];
        let ok = "[[skip]]\nfile = \"crates/a/src/lib.rs\"\nitem = \"tests::slow\"\nreason = \"T-009 makes it fast\"\nexpires = \"2026-12-01\"\n";
        assert!(apply_allowlist(&f, ok, "2026-11-12").unwrap().is_empty());
        let expired = apply_allowlist(&f, ok, "2026-12-02").unwrap();
        assert!(expired[0].contains("expired"), "{expired:?}");
        let unlisted = apply_allowlist(&f, "", "2026-11-12").unwrap();
        assert!(unlisted[0].contains("skipped test"), "{unlisted:?}");
        let unused = apply_allowlist(&[], ok, "2026-11-12").unwrap();
        assert!(unused[0].contains("matches nothing"), "{unused:?}");
        let wrong_kind = ok.replace("reason", "kind = \"early-return\"\nreason");
        assert_eq!(
            apply_allowlist(&f, &wrong_kind, "2026-11-12")
                .unwrap()
                .len(),
            2
        );
        let no_reason = ok.replace("T-009 makes it fast", " ");
        assert!(apply_allowlist(&f, &no_reason, "2026-11-12").unwrap()[0].contains("empty reason"));
    }

    #[test]
    fn out_of_line_module_with_tests_behind_cfg_is_reported() {
        let d = crate::util::testutil::TempDir::new("noskip-mod");
        std::fs::create_dir_all(d.path().join("src")).unwrap();
        std::fs::write(d.path().join("src/slow.rs"), "#[test] fn t() {}").unwrap();
        let lib = d.path().join("src/lib.rs");
        let got = scan_file(&lib, "#[cfg(feature = \"slow\")] mod slow;").unwrap();
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].kind, Kind::CfgDisabled);
    }
}
