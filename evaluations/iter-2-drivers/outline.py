"""nav.outline vs a model computed from the published sexp (docs/protocol/nav.md, The outline),
on third-party Rust files and on randomly damaged copies of them; nav.goto-symbol for every
index (selection = name range) and one past the end; read-only checks.
usage: python3 -I outline.py <seed> <files> [damaged-per-file]"""
import sys, os, json, random, glob, re
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from k2 import *
import sx

seed_, nfiles = int(sys.argv[1]), int(sys.argv[2])
ndam = int(sys.argv[3]) if len(sys.argv) > 3 else 0
rnd = random.Random(seed_)
allf = sorted(f for f in glob.glob(os.path.expanduser("~/.cargo/registry/src/*/*/**/*.rs"), recursive=True)
              if os.path.getsize(f) <= 30000)
ITEM = {"function_item": "fn", "function_signature_item": "fn", "struct_item": "struct", "enum_item": "enum",
        "trait_item": "trait", "impl_item": "impl", "mod_item": "mod"}
CONTAINER = {"source_file", "declaration_list", "mod_item", "impl_item", "trait_item", "ERROR"}


def model(text, root):
    out = []
    def rec(nd, ok_path, depth):
        for k in nd.kids:
            if k.missing: continue
            listed = k.kind in ITEM and ok_path
            if listed:
                kind = ITEM[k.kind]
                if kind == "impl":
                    ty = next((c for c in k.kids if c.field == "type"), None)
                    tr = next((c for c in k.kids if c.field == "trait"), None)
                    nm = re.sub(r"\s+", " ", text[ty.s:ty.e])
                    if tr is not None:
                        nm = re.sub(r"\s+", " ", text[tr.s:tr.e]) + " for " + nm
                    ns, ne = ty.s, ty.e
                else:
                    n = next((c for c in k.kids if c.field == "name"), None)
                    nm, ns, ne = text[n.s:n.e], n.s, n.e
                out.append({"kind": kind, "name": nm, "start": k.s, "end": k.e, "name_start": ns, "name_end": ne, "depth": depth})
            # descend: path stays ok only through container kinds
            rec(k, ok_path and k.kind in CONTAINER, depth + (1 if listed else 0))
    rec(root, True, 0)  # root is source_file
    out.sort(key=lambda x: x["start"])
    return out


SNIP = ["(", ")", "{", "}", "\"", "//", "/*", "fn x() {}", "mod q {", "impl A {", "}", "struct", "!", "#[a]", "trait T {", "\n"]
st = dict(texts=0, items=0, mismatch=0, goto_bad=0, readonly_bad=0, err_texts=0)
shown = 0
for f in rnd.sample(allf, nfiles):
    try:
        base = open(f, encoding="utf-8").read()
    except UnicodeDecodeError:
        continue
    variants = [base]
    for _ in range(ndam):
        t = base
        for _ in range(rnd.randint(1, 4)):
            o = rnd.randint(0, len(t))
            if rnd.random() < 0.5: t = t[:o] + rnd.choice(SNIP) + t[o:]
            else: t = t[:o] + t[o + rnd.randint(1, 8):]
        variants.append(t)
    for text in variants:
        st["texts"] += 1
        rs = doc_session(text, [("syntax.tree", {"doc": 1}), ("nav.outline", {"doc": 1})], path="f.rs")
        tr = rs[1]["result"]; got = rs[2]["result"]["items"]
        if tr["has_error"]: st["err_texts"] += 1
        exp = model(text, sx.parse_sexp(tr["sexp"]))
        st["items"] += len(got)
        if got != exp:
            st["mismatch"] += 1
            if shown < 6:
                shown += 1
                d = next((i for i in range(min(len(got), len(exp))) if got[i] != exp[i]), min(len(got), len(exp)))
                print("MISMATCH", f.split("/src/", 1)[1], "err" if tr["has_error"] else "", "at", d,
                      json.dumps(got[d:d + 1]), json.dumps(exp[d:d + 1]))
        # goto-symbol for every index, then one past the end; then read-only checks
        cmds = [("doc.insert", {"doc": 1, "offset": 0, "text": " "}), ("doc.delete", {"doc": 1, "start": 0, "end": 1})]
        cmds += [("nav.goto-symbol", {"doc": 1, "index": i}) for i in range(len(got))]
        cmds += [("nav.goto-symbol", {"doc": 1, "index": len(got)}), ("cursor.get", {"doc": 1}), ("nav.outline", {"doc": 1}),
                 ("doc.info", {"doc": 1}), ("syntax.tree", {"doc": 1})]
        rs2 = doc_session(text, cmds, path="f.rs")
        for i, r in enumerate(rs2[3:3 + len(got)]):
            it = got[i]
            if not r["ok"] or r["result"]["item"] != it or r["result"]["cursor"]["anchor"] != it["name_start"] or r["result"]["cursor"]["head"] != it["name_end"]:
                st["goto_bad"] += 1
        past = rs2[3 + len(got)]
        if past["ok"] or past["error"]["kind"] != "out_of_range": st["goto_bad"] += 1
        cur = rs2[4 + len(got)]["result"]
        last = got[-1] if got else None
        if last and (cur["anchor"], cur["head"]) != (last["name_start"], last["name_end"]): st["goto_bad"] += 1
        if rs2[5 + len(got)]["result"]["items"] != got: st["readonly_bad"] += 1
        info = rs2[6 + len(got)]["result"]
        if not info["can_undo"] or info["can_redo"]: st["readonly_bad"] += 1
        if rs2[7 + len(got)]["result"]["mode"] != "full": st["readonly_bad"] += 1  # nav.* never touch the syntax cache
print(f"seed={seed_} " + " ".join(f"{k}={v}" for k, v in st.items()))
