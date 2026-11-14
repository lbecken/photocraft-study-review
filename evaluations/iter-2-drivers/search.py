"""search.find / search.replace against the independent model in rx.py.
Random texts, random literal queries and random regex ASTs; random case_sensitive, whole_word,
limit, all/from, replacement templates. After each replace: text, replaced, ranges, truncated,
len_chars; then one doc.undo must restore the text and doc.redo re-apply it; then undo again.
usage: python3 -I search.py <seed> <texts> <queries-per-text>"""
import sys, os, json, random
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from k2 import *
import rx

seed_, ntexts, nq = int(sys.argv[1]), int(sys.argv[2]), int(sys.argv[3])
rnd = random.Random(seed_)
ALPHA = ["a", "a", "b", "A", "B", "é", "É", "σ", "Σ", "ς", "k", "K", "K", "s", "S", "ſ", "ß", "ẞ", "_", "1",
         " ", " ", "\r", "\n", "\r\n", ".", "-", "$", "(", "*", "中"]
CLSCH = ["a", "b", "A", "é", "σ", "k", "s", "_", "1", " ", "\r", "\n", ".", "-", "$", "ß"]


def gen_text():
    return "".join(rnd.choice(ALPHA) for _ in range(rnd.randint(0, 30)))


def gen_ast(d=0):
    r = rnd.random()
    if d > 3 or r < 0.35:
        c = rnd.random()
        if c < 0.6: return ("lit", rnd.choice(CLSCH))
        if c < 0.75: return ("any",)
        items = []
        for _ in range(rnd.randint(1, 3)):
            a = rnd.choice(CLSCH)
            if rnd.random() < 0.3:
                b = chr(ord(a) + rnd.randint(0, 3)); items.append((a, b))
            else:
                items.append((a, a))
        return ("cls", rnd.random() < 0.3, items)
    if r < 0.55: return ("cat", [gen_ast(d + 1) for _ in range(rnd.randint(2, 3))])
    if r < 0.65: return ("alt", [gen_ast(d + 1) for _ in range(rnd.randint(2, 3))] + ([("cat", [])] if rnd.random() < 0.2 else []))
    if r < 0.8: return ("grp", gen_ast(d + 1))
    if r < 0.95:
        body = gen_ast(d + 1)
        if nullable(body): body = ("lit", rnd.choice(CLSCH))
        mn, mx = rnd.choice([(0, None), (1, None), (0, 1), (2, None), (1, 3), (2, 2), (0, 2)])
        return ("rep", body, mn, mx, rnd.random() < 0.7)
    return rnd.choice([("bol",), ("eol",)])


def nullable(n):
    t = n[0]
    if t in ("lit", "any", "cls"): return False
    if t == "cat": return all(nullable(x) for x in n[1])
    if t == "alt": return any(nullable(x) for x in n[1])
    if t == "grp": return nullable(n[1])
    if t == "rep": return n[2] == 0 or nullable(n[1])
    return True


def gen_tpl(ng):
    parts = []
    for _ in range(rnd.randint(0, 4)):
        c = rnd.random()
        if c < 0.3: parts.append(rnd.choice(["x", "-", "é", "\r\n", "", "{", "}"]))
        elif c < 0.55: parts.append("$" + str(rnd.randint(0, ng + (1 if rnd.random() < 0.15 else 0))))
        elif c < 0.7: parts.append("${" + str(rnd.randint(0, ng)) + "}")
        elif c < 0.8: parts.append("$$")
        elif c < 0.85: parts.append("${")
        elif c < 0.9: parts.append("$" + str(rnd.randint(0, ng)) + "a")
        elif c < 0.95: parts.append("$x")
        else: parts.append("$")
    return "".join(parts)


checks = mism = 0; shown = 0
for ti in range(ntexts):
    text = gen_text()
    lines = [req("doc.open", id="open", path="t.txt")]
    expect = []  # (id, kind, expected)
    for qi in range(nq):
        regex = rnd.random() < 0.6
        ci = rnd.random() < 0.4
        ww = rnd.random() < 0.25
        if regex:
            ast = gen_ast()
            pat = rx.render(ast)
            num = rx.number(ast, [0])
            ng = rx.ngroups(ast)
            ms = rx.regex_iter(num, text, ci)
        else:
            if text and rnd.random() < 0.7:
                a = rnd.randint(0, len(text) - 1); pat = text[a:a + rnd.randint(1, 3)]
                if ci: pat = "".join(rnd.choice(sorted(rx.orb(c))) for c in pat)
            else:
                pat = "".join(rnd.choice(ALPHA) for _ in range(rnd.randint(1, 2)))
            ms = rx.literal_iter(pat, text, ci); ng = 0
        if ww: ms = rx.whole(ms, text)
        p = {"doc": 1, "query": pat}
        if regex: p["regex"] = True
        if ci or rnd.random() < 0.2: p["case_sensitive"] = not ci
        if ww or rnd.random() < 0.1: p["whole_word"] = ww
        if rnd.random() < 0.5:
            # find
            lim = rnd.choice([None, 1, 2, 5])
            if lim: p["limit"] = lim
            L = lim or 10000
            exp = {"matches": [{"start": a, "end": b} for a, b, _ in ms[:L]], "total": len(ms), "truncated": len(ms) > L}
            rid = f"f{qi}"
            lines.append({"id": rid, "command": "search.find", "params": p})
            expect.append((rid, "find", exp, pat, p))
        else:
            tpl = gen_tpl(ng) if regex else rnd.choice(["", "x", "$1", "Zé", "$$"])
            allm = rnd.random() < 0.5
            p["replacement"] = tpl
            frm = None
            if allm: p["all"] = True
            if not allm or rnd.random() < 0.3:
                frm = rnd.randint(0, len(text) + (1 if rnd.random() < 0.1 else 0)); p["from"] = frm
            lim = rnd.choice([None, None, 1, 2])
            if lim: p["limit"] = lim
            L = lim or 10000
            parts = rx.parse_template(tpl) if regex else [("lit", tpl)]
            okt, bad = rx.template_ok(parts, ng) if regex else (True, None)
            if not okt:
                exp = ("error", "invalid_pattern")
            elif not allm and frm is not None and frm > len(text):
                exp = ("error", "out_of_range")
            else:
                sel = ms if allm else [x for x in ms if x[0] >= (frm or 0)][:1]
                reps = [(a, b, rx.expand(parts, text, a, b, cp) if regex else tpl) for a, b, cp in sel]
                new = []; pos = 0; ranges = []; shift = 0
                for a, b, r in reps:
                    new.append(text[pos:a]); st = a + shift; new.append(r); ranges.append({"start": st, "end": st + len(r)})
                    shift += len(r) - (b - a); pos = b
                new.append(text[pos:]); newt = "".join(new)
                exp = ({"replaced": len(reps), "len_chars": len(newt), "ranges": ranges[:L], "truncated": len(reps) > L}, newt,
                       any(not (a == b and r == "") for a, b, r in reps))
            rid = f"r{qi}"
            lines.append({"id": rid, "command": "search.replace", "params": p})
            lines.append({"id": rid + "t", "command": "doc.text", "params": {"doc": 1}})
            lines.append({"id": rid + "u", "command": "doc.undo", "params": {"doc": 1}})
            lines.append({"id": rid + "ut", "command": "doc.text", "params": {"doc": 1}})
            lines.append({"id": rid + "r", "command": "doc.redo", "params": {"doc": 1}})
            lines.append({"id": rid + "rt", "command": "doc.text", "params": {"doc": 1}})
            lines.append({"id": rid + "u2", "command": "doc.undo", "params": {"doc": 1}})
            lines.append({"id": rid + "i", "command": "doc.info", "params": {"doc": 1}})
            expect.append((rid, "replace", exp, pat, p))
    rc, rs, err = mem(lines, [("t.txt", text)], name=f"search-{seed_}.jsonl")
    by = {r["id"]: r for r in rs}

    def bad(msg, got, exp, pat, p):
        global mism, shown
        mism += 1
        if shown < 12:
            shown += 1
            print("MISMATCH", msg, "text=", json.dumps(text), "params=", json.dumps(p, ensure_ascii=False))
            print("   got", json.dumps(got, ensure_ascii=False)[:400]); print("   exp", json.dumps(exp, ensure_ascii=False)[:400])

    for rid, kind, exp, pat, p in expect:
        checks += 1
        r = by[rid]
        if kind == "find":
            if not r["ok"] or r["result"] != exp: bad("find", r, exp, pat, p)
            continue
        if isinstance(exp, tuple) and exp[0] == "error":
            if r["ok"] or r["error"]["kind"] != exp[1]: bad("refusal", r, exp, pat, p)
            if by[rid + "t"]["result"]["text"] != text: bad("text-after-refusal", by[rid + "t"], text, pat, p)
            continue
        res, newt, edit = exp
        if not r["ok"] or r["result"] != res: bad("replace", r, res, pat, p); continue
        if by[rid + "t"]["result"]["text"] != newt: bad("replace-text", by[rid + "t"]["result"], newt, pat, p)
        undone = by[rid + "u"]["result"]["undone"]
        if undone != edit: bad("undo-step", by[rid + "u"], edit, pat, p)
        if by[rid + "ut"]["result"]["text"] != text: bad("undo-text", by[rid + "ut"]["result"], text, pat, p)
        if edit and by[rid + "rt"]["result"]["text"] != newt: bad("redo-text", by[rid + "rt"]["result"], newt, pat, p)
        if by[rid + "i"]["result"]["can_undo"] is not False: bad("history", by[rid + "i"]["result"], "can_undo false", pat, p)
print(f"seed={seed_} texts={ntexts} checks={checks} mismatches={mism}")
