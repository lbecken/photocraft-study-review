"""syntax.* hand-made cases: documented examples, highlight rules on tricky tokens, bracket
examples, mode rules, size limit, refusal order, read-only / typing group.
usage: python3 -I syntax_edge.py"""
import sys, os, json
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from k2 import *


def S(text, cmds, label, show=True):
    rs = doc_session(text, cmds, path="a.rs")
    if show:
        print(f"--- {label}  text={json.dumps(text, ensure_ascii=False)[:120]}")
        for (c, p), r in zip(cmds, rs[1:]):
            out = r.get("result") if r["ok"] else r["error"]
            pp = {k: v for k, v in p.items() if k != "doc"}
            if c == "syntax.highlight" and r["ok"]:
                out = " ".join(f"{text[s['start']:s['end']]!r}:{s['kind']}" for s in out["spans"])
            print(f"  {c} {json.dumps(pp)} -> {out if isinstance(out, str) else json.dumps(out, ensure_ascii=False)[:400]}")
    return rs


H = lambda **k: ("syntax.highlight", dict(doc=1, **k))
TR = ("syntax.tree", {"doc": 1})
MB = lambda o: ("syntax.match-bracket", {"doc": 1, "offset": o})
S("fn f() {}", [TR, TR, ("doc.insert", dict(doc=1, offset=8, text=" ")), TR], "tree example + modes")
S("fn f() { let s: u8 = 1; } // c", [H(), H(start=10, end=17), MB(7), MB(24), MB(30), MB(31), MB(3)], "highlight/bracket examples")
S("fn f() { g(1; }", [MB(10), TR], "missing paren")
S("fn a() { (", [MB(9), TR], "unclosed")
S(") ( ( ) ] [", [MB(i) for i in range(11)], "bracket list")
S('fn m() { Self::new(); s.f(); a::b::<u8>(); g::<T>(); x.y::<Z>(); let h = k::<u8>; m!(q); r#fn(); union(); }',
  [H()], "callees")
S("let a = b'x'; let b = c\"s\"; let c = br#\"r\"#; let d = '\\n'; let e = \"a{}\\n\"; let f = 'a; let g = 1_000u32 + 2.5e3;",
  [H()], "literals")
S("/// doc\n//! inner\n/* block /* nested */ */\nmacro_rules! m { () => {} }\nunion U { a: u8 }\nlet union = 1;\nimpl X { default fn f() {} }\nfn g(&mut self, s: &'static str) -> Self { crate::x; super::y; self.z; true }",
  [H()], "keywords, comments")
S("async fn f() { x.await; gen; try { } yield; dyn Tr; auto; safe; raw; }", [H()], "contextual words")
S("fn f() {\r\n  let x = 1;\r\n}", [H(start=8, end=9), H(start=9, end=10), H(start=9, end=9), H(start=0, end=0), MB(7), MB(8), MB(9)], "CRLF ends")
S("", [H(), TR, MB(0), MB(1)], "empty doc")
S("fn f() {}", [H(start=5, end=2), H(start=10), H(end=10), H(start=None), ("syntax.highlight", {"doc": 9, "start": 5, "end": 2}),
               ("syntax.tree", {"doc": 1, "x": 1}), MB(10), ("syntax.match-bracket", {"doc": 1}), ("syntax.match-bracket", {"doc": 1, "offset": -1})], "refusals")
# read-only and typing group
S("fn f() {}", [("doc.insert", dict(doc=1, offset=0, text="a")), TR, H(), MB(5), ("doc.insert", dict(doc=1, offset=1, text="b")),
               ("doc.undo", {"doc": 1}), ("doc.text", {"doc": 1}), ("cursor.get", {"doc": 1})], "read-only, typing group stays open")
# mode rules
S("fn f() {}", [TR, ("doc.insert", dict(doc=1, offset=9, text=" fn")), TR, ("doc.insert", dict(doc=1, offset=12, text=" g() {}")), TR,
               ("doc.insert", dict(doc=1, offset=0, text=" ")), TR, ("doc.undo", {"doc": 1}), TR,
               ("doc.undo", {"doc": 1}), ("doc.undo", {"doc": 1}), TR, ("nav.outline", {"doc": 1}), TR,
               ("doc.insert", dict(doc=1, offset=0, text="x")), ("syntax.highlight", {"doc": 1, "start": 99}), ("doc.delete", dict(doc=1, start=0, end=1)), TR], "modes")
# size limit
big = "a;" * (1048576 // 2)
rs = doc_session(big, [("doc.info", {"doc": 1}), TR, H(start=0, end=4), MB(1), ("nav.outline", {"doc": 1}),
                       ("doc.insert", dict(doc=1, offset=0, text="b")), TR, H(start=0, end=4), MB(1), ("nav.outline", {"doc": 1}),
                       ("nav.goto-symbol", {"doc": 1, "index": 0}), ("syntax.highlight", {"doc": 1, "start": 5, "end": 2}),
                       ("syntax.highlight", {"doc": 9}), ("syntax.match-bracket", {"doc": 1, "offset": 99999999}),
                       ("doc.delete", dict(doc=1, start=0, end=1)), TR, ("doc.insert", dict(doc=1, offset=0, text="b")),
                       ("doc.delete", dict(doc=1, start=0, end=1)), TR], path="big.rs")
print("--- size limit (1,048,576 chars of `a;`)")
for r in rs[1:]:
    o = r.get("result") if r["ok"] else r["error"]
    if isinstance(o, dict) and "sexp" in o: o = {k: v for k, v in o.items() if k != "sexp"}
    if isinstance(o, dict) and "spans" in o: o = {"spans": o["spans"][:3]}
    if isinstance(o, dict) and "items" in o: o = {"items": len(o["items"])}
    print("  ", r["id"], json.dumps(o)[:250])
