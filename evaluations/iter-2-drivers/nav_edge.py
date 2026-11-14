"""nav.* hand cases (examples of nav.md, impl names, refusals) and a random goto-line model.
usage: python3 -I nav_edge.py <seed> <texts>"""
import sys, os, json, random
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from k2 import *

O = ("nav.outline", {"doc": 1})


def show(text, cmds, label):
    rs = doc_session(text, cmds, path="a.rs")
    print(f"--- {label}  text={json.dumps(text)[:140]}")
    for (c, p), r in zip(cmds, rs[1:]):
        out = r.get("result") if r["ok"] else r["error"]
        if c == "nav.outline" and r["ok"]:
            out = "; ".join(f"{i['kind']} {i['name']!r} {i['start']}..{i['end']} {i['name_start']}..{i['name_end']} {i['depth']}" for i in out["items"])
        print(f"  {c} {json.dumps({k: v for k, v in p.items() if k != 'doc'})} -> {out if isinstance(out, str) else json.dumps(out)[:300]}")
    return rs


show("impl<T> Display for Foo<T> { fn fmt(&self) {} }\nmod m;\ntrait T { fn x(&self); }", [O, ("nav.goto-symbol", {"doc": 1, "index": 1}), ("nav.goto-symbol", {"doc": 1, "index": 5})], "ex1")
show("fn a() {}\nfn b( {\nfn c() {}\nstruct D;\n", [O], "ex2")
show("/// doc\n#[derive(Debug)]\npub struct S;\n#[cfg(test)]\nmod tests { fn t() {} }", [O], "ex3")
show("extern \"C\" { fn c(); }\nm! { fn inner() {} }\nfn outer() { fn nested() {} struct N; }\nconst _: () = { struct Q; };", [O], "ex4")
show("impl  <T>  Foo < T ,\n U > {}\nimpl !Send for X {}", [O], "ex5")
show("mod m { fn a() { } } fn b() { mod n { fn c() {} }", [O], "error recovery example")
show("unsafe impl<T> Send for Vec /* c */ <T> {}\nimpl dyn Tr {}\nimpl<'a> Tr<'a,\tB> for &'a [u8] {}\nimpl crate::a::B for (u8, u16) {}\nimpl ! Sync for Y {}",
     [O], "impl names")
show("pub(crate) enum E { A }\nunion U { a: u8 }\nmacro_rules! m { () => {} }\ntype T = u8;\nstatic S: u8 = 0;\nmod a { mod b { mod c { fn d() {} } } }\ntrait X { fn y() {} type Z; }\nimpl Q { fn r() { fn s() {} } }",
     [O], "kinds and depth")
show("fn é😀() {}\r\nstruct 名;", [O, ("nav.goto-symbol", {"doc": 1, "index": 1})], "non-ASCII names, CRLF")
show("fn f() {}", [("nav.outline", {"doc": 9}), ("nav.outline", {"doc": 1, "x": 1}), ("nav.goto-symbol", {"doc": 1}), ("nav.goto-symbol", {"doc": 1, "index": -1}),
               ("nav.goto-symbol", {"doc": 9, "index": 99}), ("nav.goto-symbol", {"doc": 1, "index": 18446744073709551615}),
               ("nav.goto-line", {"doc": 1}), ("nav.goto-line", {"doc": 1, "line": 0, "col": None}), ("nav.goto-line", {"doc": 9, "line": 0}),
               ("nav.goto-line", {"doc": 1, "line": 18446744073709551615, "col": 18446744073709551615}), ("nav.goto-line", {"doc": 1, "line": -1})], "refusals")
show("ab\r\ncdef\n", [("nav.goto-line", {"doc": 1, "line": 1, "col": 9}), ("nav.goto-line", {"doc": 1, "line": 8}), ("nav.goto-line", {"doc": 1, "line": 0, "col": 1})], "goto-line example")
show("xyz", [("doc.insert", {"doc": 1, "offset": 0, "text": "a"}), ("nav.goto-line", {"doc": 1, "line": 0, "col": 1}), ("doc.insert", {"doc": 1, "offset": 1, "text": "b"}),
             ("doc.undo", {"doc": 1}), ("doc.text", {"doc": 1}), ("doc.info", {"doc": 1})], "goto-line ends typing group")
show("fn f() {}", [("doc.insert", {"doc": 1, "offset": 9, "text": "a"}), ("nav.goto-symbol", {"doc": 1, "index": 3}), ("doc.insert", {"doc": 1, "offset": 10, "text": "b"}),
                ("doc.undo", {"doc": 1}), ("doc.text", {"doc": 1}), ("nav.goto-symbol", {"doc": 1, "index": 0}), ("doc.insert", {"doc": 1, "offset": 9, "text": "c"}), ("doc.undo", {"doc": 1}), ("doc.text", {"doc": 1})], "goto-symbol typing group")

# random goto-line model
seed_, n = int(sys.argv[1]), int(sys.argv[2])
rnd = random.Random(seed_)
A = ["a", "é", "😀", "\r", "\n", "\r\n", " ", "\r\r", "\n\r"]
checks = bad = 0
for _ in range(n):
    t = "".join(rnd.choice(A) for _ in range(rnd.randint(0, 15)))
    # naive lines
    lines = []; i = 0; st_ = 0
    while i < len(t):
        if t[i] == "\r" and t[i + 1:i + 2] == "\n": lines.append((st_, i)); i += 2; st_ = i
        elif t[i] in "\r\n": lines.append((st_, i)); i += 1; st_ = i
        else: i += 1
    lines.append((st_, len(t)))
    cmds = []; exp = []
    for _ in range(12):
        ln = rnd.randint(0, len(lines) + 2); col = rnd.choice([None, rnd.randint(0, 6)])
        p = {"doc": 1, "line": ln}
        if col is not None: p["col"] = col
        cmds.append(("nav.goto-line", p))
        L = min(ln, len(lines) - 1); s, e = lines[L]; c = min(col or 0, e - s)
        exp.append({"doc": 1, "anchor": s + c, "head": s + c, "start": s + c, "end": s + c, "line": L, "col": c})
    rs = doc_session(t, cmds, path="g.txt")
    for r, x in zip(rs[1:], exp):
        checks += 1
        if not r["ok"] or r["result"] != x:
            bad += 1
            if bad < 5: print("GOTO-LINE MISMATCH", json.dumps(t), r, x)
print(f"goto-line seed={seed_} texts={n} checks={checks} mismatches={bad}")
