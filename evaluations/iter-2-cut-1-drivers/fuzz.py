"""Seeded random editor sessions: kiln script (UI requests) vs a naive model + a kiln-cli oracle.

usage: fuzz.py <seed> <sessions> <steps> [alphabet]
  alphabet: "ascii" (default), "uni" (adds é, 中, 😀, tab), "comb" (adds combining marks; no clicks)

For every session:
  1. a random document (LF / CRLF / CR / mixed / none / empty), path .rs or .txt;
  2. a random list of UI inputs among the editor's input table;
  3. the naive model (this file) predicts, for each input, the mutating commands the input table
     names, and text + selection after it; the predicted commands are run step by step in a
     `kiln-cli control` session on a copy of the files (the oracle; undo/redo/save state is taken
     from it, the model's text/selection for other inputs is compared with it);
  4. one `kiln script` run with, after each input: snapshot, doc.text, cursor.get, doc.info,
     syntax.highlight; `--log` written;
  5. checks: UI doc.text/cursor == oracle; snapshot Editor value/selection == doc.text/cursor.get;
     Title has `modified` iff dirty; rows == non-empty lines (prefix rule for cut lines);
     colours per char == table[kind of syntax.highlight] (.rs) or plain (.txt), blanks by the
     neighbour rule; mutating commands of the log == predicted; log replayed with `kiln-cli run`
     on a fresh copy gives the same files as the UI session; files of UI == files of oracle;
     the same script with 30 idle frames inserted after every input gives the same log.
Prints one summary line and every mismatch (first 40).
"""
import json, os, random, subprocess, sys
sys.path.insert(0, os.path.dirname(__file__))
import kl

COL = {"plain": "#dcdcdc", "keyword": "#569cd6", "type": "#4ec9b0", "function": "#dcdcaa",
       "string": "#ce9178", "number": "#b5cea8", "comment": "#6a9955"}
MUTATING = set()
for c in json.load(open(os.path.join(os.environ["KILN_CLONE"], "docs/commands.json")))["commands"]:
    if c["mutating"]:
        MUTATING.add(c["name"])


# ---------------------------------------------------------------- naive model
def is_word(ch):
    return ch == "_" or ch.isalnum()


def lines_of(t):
    """list of (start, end, ending) as doc.line reports them."""
    out = []
    s = 0
    i = 0
    n = len(t)
    while i < n:
        if t[i] == "\r" and i + 1 < n and t[i + 1] == "\n":
            out.append((s, i, "crlf")); i += 2; s = i
        elif t[i] == "\r":
            out.append((s, i, "cr")); i += 1; s = i
        elif t[i] == "\n":
            out.append((s, i, "lf")); i += 1; s = i
        else:
            i += 1
    out.append((s, n, "none"))
    return out


def line_of(t, h):
    L = lines_of(t)
    for idx, (s, e, _) in enumerate(L):
        if s <= h <= e:
            return idx, L
    raise AssertionError("position inside a break")


def dominant(t):
    c = {"lf": 0, "crlf": 0, "cr": 0}
    for (_, _, k) in lines_of(t)[:-1]:
        c[k] += 1
    if sum(c.values()) == 0:
        return "none"
    m = max(c.values())
    for k in ("lf", "crlf", "cr"):
        if c[k] == m:
            return k


def step_char(t, h, fwd):
    if fwd:
        if h >= len(t):
            return h
        return h + 2 if t[h:h + 2] == "\r\n" else h + 1
    if h <= 0:
        return 0
    return h - 2 if h >= 2 and t[h - 2:h] == "\r\n" else h - 1


def step_word(t, h, fwd):
    if fwd:
        while h < len(t) and not is_word(t[h]):
            h += 1
        while h < len(t) and is_word(t[h]):
            h += 1
        return h
    while h > 0 and not is_word(t[h - 1]):
        h -= 1
    while h > 0 and is_word(t[h - 1]):
        h -= 1
    return h


def step_line(t, h, fwd):
    idx, L = line_of(t, h)
    col = h - L[idx][0]
    if fwd:
        if idx == len(L) - 1:
            return len(t)
        s, e, _ = L[idx + 1]
    else:
        if idx == 0:
            return 0
        s, e, _ = L[idx - 1]
    return s + min(col, e - s)


def snap(t, p):
    if 0 < p < len(t) and t[p - 1] == "\r" and t[p] == "\n":
        return p - 1
    return p


def map_pos(p, s, e, n, t_after):
    if p < s:
        q = p
    elif p > e:
        q = p + n - (e - s)
    elif p == s and e > s:
        q = s
    else:
        q = s + n
    return snap(t_after, q)


class Model:
    def __init__(self, text):
        self.t = text
        self.a = 0
        self.h = 0

    def edit(self, s, e, new):
        t2 = self.t[:s] + new + self.t[e:]
        self.a = map_pos(self.a, s, e, len(new), t2)
        self.h = map_pos(self.h, s, e, len(new), t2)
        self.t = t2

    def sel(self):
        return min(self.a, self.h), max(self.a, self.h)


def predict(m, inp):
    """Apply input to model m; return list of predicted mutating (command, params) or None for
    'take state from the oracle' (undo/redo/save)."""
    k = inp["k"]
    cmds = []
    if k in ("type", "enter"):
        if k == "enter":
            le = dominant(m.t)
            text = {"lf": "\n", "crlf": "\r\n", "cr": "\r", "none": "\n"}[le]
        else:
            text = inp["text"]
        s, e = m.sel()
        want = s + len(text)
        join = k == "enter" and text == "\r" and m.t[e:e + 1] == "\n"
        if k == "enter":
            inp["_t025"] = text == "\n" and s > 0 and m.t[s - 1] == "\r"
        if s == e:
            cmds.append(("doc.insert", {"doc": 1, "offset": s, "text": text}))
            m.edit(s, s, text)
        else:
            cmds.append(("doc.replace", {"doc": 1, "start": s, "end": e, "text": text}))
            m.edit(s, e, text)
            if not join:
                cmds.append(("cursor.set", {"doc": 1, "anchor": want}))
                m.a = m.h = snap(m.t, want)
        if join:
            # caret after the whole break when the inserted ending joins the next char
            cmds.append(("cursor.set", {"doc": 1, "anchor": want + 1}))
            m.a = m.h = want + 1
        return cmds
    if k in ("bs", "del"):
        s, e = m.sel()
        if s == e:
            if k == "bs":
                if m.h == 0:
                    return []
                s, e = step_char(m.t, m.h, False), m.h
            else:
                if m.h == len(m.t):
                    return []
                s, e = m.h, step_char(m.t, m.h, True)
        cmds.append(("doc.delete", {"doc": 1, "start": s, "end": e}))
        m.edit(s, e, "")
        return cmds
    if k == "arrow":
        d = inp["dir"]
        by = "line" if d in ("up", "down") else ("word" if inp.get("ctrl") else "char")
        fwd = d in ("right", "down")
        p = {"doc": 1, "by": by, "forward": fwd}
        if inp.get("shift"):
            p["extend"] = True
        cmds.append(("cursor.move", p))
        nh = {"char": step_char, "word": step_word, "line": step_line}[by](m.t, m.h, fwd)
        m.h = nh
        if not inp.get("shift"):
            m.a = nh
        return cmds
    if k in ("home", "end"):
        idx, L = line_of(m.t, m.h)
        nh = L[idx][0] if k == "home" else L[idx][1]
        if inp.get("shift"):
            cmds.append(("cursor.set", {"doc": 1, "anchor": m.a, "head": nh}))
        else:
            cmds.append(("cursor.set", {"doc": 1, "anchor": nh, "head": nh}))
            m.a = nh
        m.h = nh
        return cmds
    if k == "click":
        L = [x for x in lines_of(m.t) if x[1] > x[0]]
        s, e, _ = L[inp["row"]]
        col = min(inp["col"], e - s)
        inp["col"] = col
        nh = s + col
        if inp.get("shift"):
            cmds.append(("cursor.set", {"doc": 1, "anchor": m.a, "head": nh}))
        else:
            cmds.append(("cursor.set", {"doc": 1, "anchor": nh, "head": nh}))
            m.a = nh
        m.h = nh
        return cmds
    if k == "undo":
        return [("doc.undo", {"doc": 1})]
    if k == "redo":
        return [("doc.redo", {"doc": 1})]
    if k == "save":
        return [("doc.save", {"doc": 1})]
    raise ValueError(k)


def ui_lines(inp):
    """UI requests of one input."""
    k = inp["k"]
    if k == "type":
        return [{"ui": "type", "text": inp["text"]}]
    if k == "enter":
        return [{"ui": "key", "key": "Enter"}]
    if k == "bs":
        return [{"ui": "key", "key": "Backspace"}]
    if k == "del":
        return [{"ui": "key", "key": "Delete"}]
    mods = []
    if inp.get("ctrl"):
        mods.append("ctrl")
    if inp.get("shift"):
        mods.append("shift")
    if k == "arrow":
        r = {"ui": "key", "key": {"left": "ArrowLeft", "right": "ArrowRight", "up": "ArrowUp", "down": "ArrowDown"}[inp["dir"]]}
    elif k == "home":
        r = {"ui": "key", "key": "Home"}
    elif k == "end":
        r = {"ui": "key", "key": "End"}
    elif k == "click":
        r = {"ui": "click", "target": {"label": "Editor"}, "at": {"row": inp["row"], "col": inp["col"]}}
    elif k == "undo":
        r = {"ui": "key", "key": "Z", "modifiers": ["ctrl"]}
        return [r]
    elif k == "redo":
        r = {"ui": "key", "key": "Y", "modifiers": ["ctrl"]} if inp.get("y") else {"ui": "key", "key": "Z", "modifiers": ["ctrl", "shift"]}
        return [r]
    elif k == "save":
        return [{"ui": "key", "key": "S", "modifiers": ["ctrl"]}]
    if mods:
        r["modifiers"] = mods
    return [r]


# ---------------------------------------------------------------- generators
ASCII = "abcxyz_019 (){};:\"'/*fnletstructu8"


def alphabet(kind):
    a = list("abcxyz_019 ") + ["fn", "let", "struct", "u8", "\"s\"", "//c", "1.5", "{", "}", "(", ")", ";",
                               " fn f() ", " let ", " struct S ", ": u8", " 42 ", " \"str\" ", " /* c */ ", " f(1) "]
    if kind in ("uni", "comb"):
        a += ["é", "中", "😀", "\t", "ß", "Ω"]
    if kind == "comb":
        a += ["é", "́", "ä"]
    return a


def gen_doc(rng, kind):
    a = alphabet(kind)
    nlines = rng.choice([0, 1, 1, 2, 3, 5, 8, 12, 20])
    eolk = rng.choice(["lf", "crlf", "cr", "mixed"])
    out = ""
    for i in range(nlines):
        ln = "".join(rng.choice(a) for _ in range(rng.choice([0, 0, 1, 3, 6, 10, 20])))
        out += ln
        if i < nlines - 1 or rng.random() < 0.6:
            out += {"lf": "\n", "crlf": "\r\n", "cr": "\r"}.get(eolk) or rng.choice(["\n", "\r\n", "\r"])
    if rng.random() < 0.05:
        out += "x" * rng.choice([170, 400])  # a long, cut line
    return out


def gen_input(rng, m, kind, clicks):
    r = rng.random()
    a = alphabet(kind)
    if r < 0.25:
        return {"k": "type", "text": "".join(rng.choice(a) for _ in range(rng.choice([1, 1, 2, 4])))}
    if r < 0.32:
        return {"k": "enter"}
    if r < 0.40:
        return {"k": "bs"}
    if r < 0.46:
        return {"k": "del"}
    if r < 0.66:
        d = rng.choice(["left", "right", "up", "down"])
        # ctrl with ArrowUp/ArrowDown is "not specified" (ui-editor.md): not generated
        return {"k": "arrow", "dir": d,
                "ctrl": d in ("left", "right") and rng.random() < 0.3, "shift": rng.random() < 0.3}
    if r < 0.72:
        return {"k": rng.choice(["home", "end"]), "shift": rng.random() < 0.3}
    if r < 0.84 and clicks:
        L = [x for x in lines_of(m.t) if x[1] > x[0]]
        if L and len(lines_of(m.t)) <= 30:
            row = rng.randrange(len(L))
            s, e, _ = L[row]
            ln = min(e - s, 150)
            return {"k": "click", "row": row, "col": rng.randint(0, ln), "shift": rng.random() < 0.3}
        return {"k": "arrow", "dir": "left"}
    if r < 0.91:
        return {"k": "undo"}
    if r < 0.97:
        return {"k": "redo", "y": rng.random() < 0.5}
    return {"k": "save"}


# ---------------------------------------------------------------- oracle
class Oracle:
    def __init__(self, root):
        self.p = subprocess.Popen(["timeout", "600", kl.CLI, "control", "--root", root],
                                  stdin=subprocess.PIPE, stdout=subprocess.PIPE)
        self.n = 0

    def req(self, cmd, params):
        self.n += 1
        self.p.stdin.write((json.dumps({"id": self.n, "command": cmd, "params": params}) + "\n").encode())
        self.p.stdin.flush()
        return json.loads(self.p.stdout.readline())

    def close(self):
        self.p.stdin.close()
        self.p.wait()


def blank(ch):
    return ch in " \t" or ch.isspace()


def check_colours(rows, line_start, line_text, kinds, plain_only):
    """rows: runs of one painted row; kinds: list of kind per char of the doc."""
    chars = []
    for run in rows:
        for ch in run["text"]:
            chars.append((ch, run["color"]))
    probs = []
    if all(blank(c) for c, _ in chars):
        for i, (c, col) in enumerate(chars):
            if col != COL["plain"]:
                probs.append(("blank-row-colour", i, c, col))
        return probs
    for i, (c, col) in enumerate(chars):
        if blank(c):
            if i > 0:
                exp = chars[i - 1][1]
            else:
                j = i
                while j < len(chars) and blank(chars[j][0]):
                    j += 1
                exp = chars[j][1]
        else:
            exp = COL["plain"] if plain_only else COL[kinds[line_start + i]]
        if col != exp:
            probs.append(("colour", line_start + i, c, col, exp))
    return probs


def session(seed, si, steps, kind):
    rng = random.Random(f"{seed}-{si}-{kind}")
    text = gen_doc(rng, kind)
    path = rng.choice(["a.rs", "a.rs", "b.txt"])
    init = kl.fresh(f"fz{seed}-{si}")
    kl.write_files(init, {path: text})
    oroot = kl.copytree(init, f"fzo{seed}-{si}")
    uroot = kl.copytree(init, f"fzu{seed}-{si}")
    idle_root = kl.copytree(init, f"fzi{seed}-{si}")
    orc = Oracle(oroot)
    o = orc.req("doc.open", {"path": path})
    assert o["ok"], o
    m = Model(text)
    clicks = kind != "comb"
    plan = []  # (input, predicted cmds, expected text, anchor, head, dirty, model_ok)
    probs = []
    for st in range(steps):
        inp = gen_input(rng, m, kind, clicks)
        before = (m.t, m.a, m.h)
        cmds = predict(m, inp)
        for (c, p) in cmds:
            r = orc.req(c, p)
            if not r["ok"]:
                probs.append(("oracle-refused", st, inp, c, p, r))
        ot = orc.req("doc.text", {"doc": 1})["result"]["text"]
        oc = orc.req("cursor.get", {"doc": 1})["result"]
        oi = orc.req("doc.info", {"doc": 1})["result"]
        if inp["k"] in ("undo", "redo", "save"):
            m.t, m.a, m.h = ot, oc["anchor"], oc["head"]
        elif (m.t, m.a, m.h) != (ot, oc["anchor"], oc["head"]):
            probs.append(("model-vs-oracle", st, inp, before, (m.t, m.a, m.h), (ot, oc["anchor"], oc["head"])))
            m.t, m.a, m.h = ot, oc["anchor"], oc["head"]
        plan.append((inp, cmds, ot, oc["anchor"], oc["head"], oi["dirty"]))
    orc.close()

    def build(idle):
        reqs = []
        if any(p[0]["k"] != "click" for p in plan) or True:
            reqs.append({"id": "focus", "ui": "focus", "target": {"label": "Editor"}})
        for st, (inp, *_rest) in enumerate(plan):
            for u in ui_lines(inp):
                u = dict(u); u["id"] = f"a{st}"
                reqs.append(u)
            if idle:
                reqs.append({"id": f"idle{st}", "ui": "frames", "count": 30})
            reqs.append({"id": f"s{st}", "ui": "snapshot"})
            reqs.append({"id": f"t{st}", "command": "doc.text", "params": {"doc": 1}})
            reqs.append({"id": f"c{st}", "command": "cursor.get", "params": {"doc": 1}})
            reqs.append({"id": f"i{st}", "command": "doc.info", "params": {"doc": 1}})
            if path.endswith(".rs"):
                reqs.append({"id": f"h{st}", "command": "syntax.highlight", "params": {"doc": 1}})
        return reqs

    ulog = uroot + ".log"
    rc, out, err, sp = kl.kscript(build(False), root=uroot, paths=[path], log=ulog, name=f"fz{seed}-{si}")
    res = {}
    for r in kl.parse(out):
        res[r["id"]] = r
    for rid, r in res.items():
        if not r.get("ok"):
            probs.append(("response-not-ok", rid, r))
        elif str(rid).startswith("a") and not r["result"].get("settled"):
            probs.append(("not-settled", rid, r))
    cases = 0
    for st, (inp, cmds, ot, oa, oh, odirty) in enumerate(plan):
        cases += 1
        t = res[f"t{st}"]["result"]["text"]
        c = res[f"c{st}"]["result"]
        info = res[f"i{st}"]["result"]
        if (t, c["anchor"], c["head"]) != (ot, oa, oh):
            if inp.get("_t025"):
                probs.append(("T-025?", st, inp))
            probs.append(("ui-vs-oracle", st, inp, (t, c["anchor"], c["head"]), (ot, oa, oh)))
        if info["dirty"] != odirty:
            probs.append(("dirty", st, inp, info["dirty"], odirty))
        snap_ = res[f"s{st}"]
        ed = kl.one(snap_, label="Editor")
        ti = kl.one(snap_, label="Title")
        if ed is None or ti is None:
            probs.append(("no-editor-or-title", st)); continue
        if ed.get("value") != t:
            probs.append(("editor-value", st, ed.get("value"), t))
        if ed.get("selection") != {"anchor": c["anchor"], "head": c["head"]}:
            probs.append(("editor-selection", st, ed.get("selection"), c))
        if not ed.get("focused"):
            probs.append(("editor-not-focused", st))
        tv = ti.get("value", "")
        exp_title = path + (" — modified — Kiln" if info["dirty"] else " — Kiln")
        if tv != exp_title:
            probs.append(("title", st, tv, exp_title))
        # rows
        L = [x for x in lines_of(t) if x[1] > x[0]]
        rt = kl.rows_text(ed)
        if len(lines_of(t)) <= 30:
            if len(rt) != len(L):
                probs.append(("row-count", st, len(rt), len(L), rt[:3]))
            else:
                kinds = None
                if path.endswith(".rs") and len(t) <= 1048576:
                    kinds = ["?"] * len(t)
                    for sp_ in res[f"h{st}"]["result"]["spans"]:
                        for q in range(sp_["start"], sp_["end"]):
                            kinds[q] = sp_["kind"]
                for row, (s, e, _), rtext in zip(ed.get("rows", []), L, rt):
                    full = t[s:e]
                    if not (full.startswith(rtext) and (rtext == full or len(rtext) >= 40)):
                        probs.append(("row-text", st, rtext, full))
                        continue
                    probs += [("colour", st) + p for p in check_colours(row, s, full, kinds, kinds is None)]
    # log: mutating commands
    log = [json.loads(l) for l in open(ulog)]
    def norm(c, p):
        p = dict(p)
        if c == "cursor.move" and p.get("extend") is False:
            del p["extend"]  # the table names only extend:true; false is the default
        return (c, p)
    got = [norm(l["command"], l["params"]) for l in log if l["command"] in MUTATING]
    exp = [("doc.open", {"path": path})] + [c for p in plan for c in p[1]]
    if got != exp:
        # first difference
        i = next((i for i, (g, e) in enumerate(zip(got, exp)) if g != e), min(len(got), len(exp)))
        probs.append(("log-mutating", i, got[max(0, i - 2):i + 3], exp[max(0, i - 2):i + 3]))
    # files UI vs oracle
    fu, fo = kl.read_tree(uroot), kl.read_tree(oroot)
    if fu != fo:
        probs.append(("files-ui-vs-oracle", {k: (fu.get(k), fo.get(k)) for k in set(fu) | set(fo) if fu.get(k) != fo.get(k)}))
    # replay
    rroot = kl.copytree(init, f"fzr{seed}-{si}")
    rrc, rout, rerr = kl.cli_run(open(ulog).read(), rroot, log=rroot + ".log")
    if rrc != 0:
        probs.append(("replay-rc", rrc, rerr[:300]))
    if kl.read_tree(rroot) != fu:
        probs.append(("replay-files",))
    if open(rroot + ".log").read() != open(ulog).read():
        probs.append(("replay-log-differs",))
    # idle frames
    ilog = idle_root + ".log"
    irc, iout, ierr, _ = kl.kscript(build(True), root=idle_root, paths=[path], log=ilog, name=f"fzi{seed}-{si}")
    if open(ilog).read() != open(ulog).read():
        probs.append(("idle-frames-changed-log", len(open(ilog).readlines()), len(log)))
    ires = {r["id"]: r for r in kl.parse(iout)}
    for st in range(len(plan)):
        r = ires.get(f"idle{st}")
        if not (r and r.get("ok") and r["result"] == {"frames": 30, "settled": True}):
            probs.append(("idle-frames-response", st, r))
    if kl.read_tree(idle_root) != fu:
        probs.append(("idle-files",))
    return cases, probs, rc, {"path": path, "text": text, "plan": [p[0] for p in plan], "script": sp}


def main():
    seed, sessions, steps = int(sys.argv[1]), int(sys.argv[2]), int(sys.argv[3])
    kind = sys.argv[4] if len(sys.argv) > 4 else "ascii"
    total = 0
    allp = []
    for si in range(sessions):
        cases, probs, rc, info = session(seed, si, steps, kind)
        total += cases
        for p in probs:
            allp.append((si, p, info["script"]))
    cats = {}
    for si, p, _ in allp:
        cats[p[0]] = cats.get(p[0], 0) + 1
    print(f"seed={seed} alphabet={kind} sessions={sessions} steps={steps} cases={total} mismatches={len(allp)} by_kind={cats}")
    firsts = {}
    for si, p, sp in allp:
        firsts.setdefault(si, (p, sp))
    print(f"sessions_with_mismatch={len(firsts)}")
    for si, (p, sp) in list(firsts.items())[:40]:
        print("  first of session", si, sp, json.dumps(p, ensure_ascii=False, default=str)[:1200])


if __name__ == "__main__":
    main()
