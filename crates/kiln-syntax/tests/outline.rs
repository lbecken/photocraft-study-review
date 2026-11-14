//! Implementer's integration tests for `kiln_syntax::outline` (T-023), through the public API:
//! the worked examples of the task's Pinned (g), the item rules on small private sources, and
//! deep nesting on the default test-thread stack.
#![cfg(test)]

use kiln_syntax::SyntaxTree;
use kiln_syntax::outline::{ItemKind, OutlineItem, outline};

fn items(text: &str) -> Vec<OutlineItem> {
    outline(&SyntaxTree::parse(text).unwrap())
}

fn it(
    kind: ItemKind,
    name: &str,
    (start, end): (usize, usize),
    (name_start, name_end): (usize, usize),
    depth: usize,
) -> OutlineItem {
    OutlineItem {
        kind,
        name: name.to_owned(),
        start,
        end,
        name_start,
        name_end,
        depth,
    }
}

use ItemKind::{Enum, Fn, Impl, Mod, Struct, Trait};

#[test]
fn pinned_examples_1_to_5() {
    assert_eq!(
        items("impl<T> Display for Foo<T> { fn fmt(&self) {} }\nmod m;\ntrait T { fn x(&self); }"),
        vec![
            it(Impl, "Display for Foo<T>", (0, 47), (20, 26), 0),
            it(Fn, "fmt", (29, 45), (32, 35), 1),
            it(Mod, "m", (48, 54), (52, 53), 0),
            it(Trait, "T", (55, 79), (61, 62), 0),
            it(Fn, "x", (65, 77), (68, 69), 1),
        ]
    );
    let broken = SyntaxTree::parse("fn a() {}\nfn b( {\nfn c() {}\nstruct D;\n").unwrap();
    assert!(broken.sexp().starts_with("(ERROR@0..38"));
    assert_eq!(
        outline(&broken),
        vec![
            it(Fn, "a", (0, 9), (3, 4), 0),
            it(Fn, "c", (18, 27), (21, 22), 0),
            it(Struct, "D", (28, 37), (35, 36), 0),
        ]
    );
    assert_eq!(
        items("/// doc\n#[derive(Debug)]\npub struct S;\n#[cfg(test)]\nmod tests { fn t() {} }"),
        vec![
            it(Struct, "S", (25, 38), (36, 37), 0),
            it(Mod, "tests", (52, 75), (56, 61), 0),
            it(Fn, "t", (64, 73), (67, 68), 1),
        ]
    );
    assert_eq!(
        items(
            "extern \"C\" { fn c(); }\nm! { fn inner() {} }\nfn outer() { fn nested() {} struct N; }\nconst _: () = { struct Q; };"
        ),
        vec![it(Fn, "outer", (44, 83), (47, 52), 0)]
    );
    assert_eq!(
        items("impl  <T>  Foo < T ,\n U > {}\nimpl !Send for X {}"),
        vec![
            it(Impl, "Foo < T , U >", (0, 28), (11, 25), 0),
            it(Impl, "Send for X", (29, 48), (44, 45), 0),
        ]
    );
}

#[test]
fn not_listed_inside_static_values_and_blocks() {
    assert_eq!(
        items("static S: () = { fn g() {} };\nfn f() { { struct Inner; } }\nenum E { A }"),
        vec![
            it(Fn, "f", (30, 58), (33, 34), 0),
            it(Enum, "E", (59, 71), (64, 65), 0),
        ]
    );
}

#[test]
fn items_inside_items_that_are_not_containers_are_not_listed() {
    // A fn inside an enum's or struct's body cannot occur; a trait's fn with a body is listed,
    // but nothing in that body.
    assert_eq!(
        items("trait T { fn d() { fn hidden() {} } }"),
        vec![
            it(Trait, "T", (0, 37), (6, 7), 0),
            it(Fn, "d", (10, 35), (13, 14), 1),
        ]
    );
}

#[test]
fn preorder_is_the_order_of_start_and_names_slice_out() {
    let text = "mod a { fn b() {} mod c { struct D; } impl E for F { fn g() {} } }\ntrait H {}\n";
    let tree = SyntaxTree::parse(text).unwrap();
    let out = outline(&tree);
    assert_eq!(out.len(), 7);
    let chars: Vec<char> = text.chars().collect();
    for w in out.windows(2) {
        assert!(w[0].start < w[1].start);
    }
    for i in &out {
        let slice: String = chars[i.name_start..i.name_end].iter().collect();
        if i.kind == Impl {
            assert_eq!(i.name, "E for F");
            assert_eq!(slice, "F");
        } else {
            assert_eq!(slice, i.name);
        }
    }
    let depths: Vec<usize> = out.iter().map(|i| i.depth).collect();
    assert_eq!(depths, vec![0, 1, 1, 2, 1, 2, 0]);
}

#[test]
fn deep_nesting_on_the_default_stack() {
    const N: usize = 20_000;
    let text = format!("{}fn f() {{}}{}", "mod m {".repeat(N), "}".repeat(N));
    let out = items(&text);
    assert_eq!(out.len(), N + 1);
    assert_eq!(
        out[N],
        it(Fn, "f", (7 * N, 7 * N + 9), (7 * N + 3, 7 * N + 4), N)
    );
    assert_eq!(out[N - 1].depth, N - 1);
}
