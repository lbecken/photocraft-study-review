"""Random sessions: edits, undo/redo, eol.normalize, cursor.set/move/get, compared with a naive
Python model of docs/protocol/cursor.md and docs/protocol/eol.md after every request.
usage: python3 -I selection.py <seed> <nsessions> <steps>"""
import sys, os, random, json, shutil
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from k import Ctl, WORK

ALPHA = ["a", "b", "é", "😀", "中", "1", "_", " ", "-", ".", "\r", "\n", "\r\n", "\r\r", "\n\r"]


def breaks(t):
    out = []  # (pos, kind)
    i = 0
    while i < len(t):
        if t[i] == "\r" and i + 1 < len(t) and t[i + 1] == "\n":
            out.append((i, "crlf")); i += 2
        elif t[i] == "\r":
            out.append((i, "cr")); i += 1
        elif t[i] == "\n":
            out.append((i, "lf")); i += 1
        else:
            i += 1
    return out


def lines(t):
    res, s = [], 0
    for p, k in breaks(t):
        res.append((s, p)); s = p + (2 if k == "crlf" else 1)
    res.append((s, len(t)))
    return res


def counts(t):
    c = {"lf": 0, "crlf": 0, "cr": 0}
    for _, k in breaks(t):
        c[k] += 1
    kinds = [k for k in ("lf", "crlf", "cr") if c[k]]
    dom = "none" if not kinds else max(("lf", "crlf", "cr"), key=lambda k: (c[k], -["lf", "crlf", "cr"].index(k)))
    return c, dom, len(kinds) > 1


def inside_crlf(t, p):
    return 0 < p < len(t) and t[p - 1] == "\r" and t[p] == "\n"


def isword(ch):
    return ch == "_" or ch.isalnum()


class Model:
    def __init__(self, t):
        self.t, self.a, self.h = t, 0, 0
        self.undo, self.redo = [], []
        self.group = None  # open typing group (index into undo) and its end offset

    def mapp(self, p, s, e, n):
        if p < s:
            return p
        if p > e:
            return p + n - (e - s)
        if p == s and e > s:
            return s
        return s + n

    def apply(self, s, old, new):
        assert self.t[s:s + len(old)] == old, (self.t, s, old)
        self.t = self.t[:s] + new + self.t[s + len(old):]
        e = s + len(old)
        res = []
        for p in (self.a, self.h):
            q = self.mapp(p, s, e, len(new))
            if inside_crlf(self.t, q):
                q -= 1
            res.append(q)
        self.a, self.h = res

    def step(self, fwd, inv):
        for s, o, n in fwd:
            self.apply(s, o, n)
        self.undo.append([fwd, inv]); self.redo = []

    def edit(self, s, e, new, typed=False):
        old = self.t[s:e]
        if old == "" and new == "":
            return
        if typed and self.group is not None and self.group[1] == s:
            fwd, inv = self.undo[-1]
            self.apply(s, "", new)
            fwd.append((s, "", new)); inv.insert(0, (s, new, ""))
            self.group = (len(self.undo) - 1, s + 1)
            self.redo = []
            return
        self.step([(s, old, new)], [(s, new, old)])
        self.group = (len(self.undo) - 1, s + 1) if typed else None

    def do_undo(self):
        self.group = None
        if not self.undo:
            return False
        fwd, inv = self.undo.pop()
        for s, o, n in inv:
            self.apply(s, o, n)
        self.redo.append([fwd, inv])
        return True

    def do_redo(self):
        if not self.redo:
            return False  # observed: a no-op redo leaves the typing group open
        self.group = None
        fwd, inv = self.redo.pop()
        for s, o, n in fwd:
            self.apply(s, o, n)
        self.undo.append([fwd, inv])
        return True

    def normalize(self, to):
        c, dom, _ = counts(self.t)
        if to is None:
            to = dom
        rep = {"lf": "\n", "crlf": "\r\n", "cr": "\r"}
        bs = [(p, k) for p, k in breaks(self.t) if to != "none" and k != to]
        if not bs:
            return 0
        self.group = None
        fwd = [(p, rep[k], rep[to]) for p, k in sorted(bs, reverse=True)]
        # inverse edits in final coordinates, highest first
        inv, shift = [], 0
        tmp = []
        for p, k in sorted(bs):
            tmp.append((p + shift, rep[to], rep[k]))
            shift += len(rep[to]) - len(rep[k])
        inv = list(reversed(tmp))
        self.step(fwd, inv)
        return len(bs)

    def move(self, by, forward, extend):
        t, h = self.t, self.h
        if by == "char":
            if forward:
                h = h if h == len(t) else (h + 2 if t[h:h + 2] == "\r\n" else h + 1)
            else:
                h = 0 if h == 0 else (h - 2 if h >= 2 and t[h - 2:h] == "\r\n" else h - 1)
        elif by == "word":
            if forward:
                while h < len(t) and not isword(t[h]): h += 1
                while h < len(t) and isword(t[h]): h += 1
            else:
                while h > 0 and not isword(t[h - 1]): h -= 1
                while h > 0 and isword(t[h - 1]): h -= 1
        else:
            ls = lines(t)
            li = next(i for i, (s, e) in enumerate(ls) if s <= h <= e)
            col = h - ls[li][0]
            if forward:
                h = len(t) if li == len(ls) - 1 else ls[li + 1][0] + min(col, ls[li + 1][1] - ls[li + 1][0])
            else:
                h = 0 if li == 0 else ls[li - 1][0] + min(col, ls[li - 1][1] - ls[li - 1][0])
        self.h = h
        if not extend:
            self.a = h
        self.group = None

    def sel(self, doc):
        ls = lines(self.t)
        li = next(i for i, (s, e) in enumerate(ls) if s <= self.h <= e)
        return {"doc": doc, "anchor": self.a, "head": self.h, "start": min(self.a, self.h),
                "end": max(self.a, self.h), "line": li, "col": self.h - ls[li][0]}


def rtext(rnd, lo, hi):
    return "".join(rnd.choice(ALPHA) for _ in range(rnd.randint(lo, hi)))


def main():
    seed, nsess, steps = map(int, sys.argv[1:4])
    rnd = random.Random(seed)
    root = os.path.join(WORK, f"sel-{seed}")
    if os.path.isdir(root):
        shutil.rmtree(root)
    os.makedirs(root)
    for k in range(nsess):
        with open(os.path.join(root, f"s{k}.txt"), "w", newline="") as f:
            f.write(rtext(rnd, 0, 20))
    c = Ctl(root)
    cases = mism = 0
    msgs = []
    opcount = {}
    for k in range(nsess):
        init = open(os.path.join(root, f"s{k}.txt"), newline="").read()
        d = c.ok("doc.open", path=f"s{k}.txt")["doc"]
        m = Model(init)
        for st in range(steps):
            t = m.t
            op = rnd.choice(["type", "type", "type", "insert", "delete", "replace", "undo", "redo",
                             "set", "set", "move", "move", "move", "normalize", "badset"])
            opcount[op] = opcount.get(op, 0) + 1
            exp_err = None
            if op == "type":
                off = m.group[1] if (m.group and rnd.random() < 0.7) else rnd.randint(0, len(t))
                ch = rnd.choice(["a", "é", "😀", "_", " ", "x"])
                r = c.req("doc.insert", doc=d, offset=off, text=ch)
                m.edit(off, off, ch, typed=True)
            elif op == "insert":
                off = rnd.randint(0, len(t))
                s = rtext(rnd, 0, 4)
                if len(s) == 1 and s not in "\r\n":
                    s += "a"
                r = c.req("doc.insert", doc=d, offset=off, text=s)
                m.edit(off, off, s) if s else None
            elif op in ("delete", "replace"):
                a = rnd.randint(0, len(t)); b = rnd.randint(a, min(len(t), a + 4))
                if op == "delete":
                    r = c.req("doc.delete", doc=d, start=a, end=b)
                    if a != b:
                        m.edit(a, b, "")
                else:
                    s = rtext(rnd, 0, 3)
                    r = c.req("doc.replace", doc=d, start=a, end=b, text=s)
                    if t[a:b] != "" or s != "":
                        m.edit(a, b, s)
            elif op == "undo":
                r = c.req("doc.undo", doc=d); m.do_undo()
            elif op == "redo":
                r = c.req("doc.redo", doc=d); m.do_redo()
            elif op == "set":
                valid = [p for p in range(len(t) + 1) if not inside_crlf(t, p)]
                a = rnd.choice(valid)
                if rnd.random() < 0.3:
                    r = c.req("cursor.set", doc=d, anchor=a); m.a = m.h = a
                else:
                    h = rnd.choice(valid)
                    r = c.req("cursor.set", doc=d, anchor=a, head=h); m.a, m.h = a, h
                m.group = None
            elif op == "badset":
                bad = [p for p in range(len(t) + 1) if inside_crlf(t, p)] + [len(t) + 1, len(t) + 7]
                a = rnd.choice(bad)
                good = rnd.choice([p for p in range(len(t) + 1) if not inside_crlf(t, p)])
                if rnd.random() < 0.5:
                    r = c.req("cursor.set", doc=d, anchor=a, head=good); which, bp = "anchor", a
                else:
                    r = c.req("cursor.set", doc=d, anchor=good, head=a); which, bp = "head", a
                if bp > len(t):
                    exp_err = ("out_of_range", f"invalid {which}: index {bp} is out of bounds (length {len(t)})")
                else:
                    exp_err = ("not_a_boundary", f"invalid {which}: char index {bp} is between the \\r and \\n of a CRLF")
            elif op == "move":
                by = rnd.choice(["char", "word", "line"]); fw = rnd.random() < 0.5
                ext = rnd.choice([None, False, True])
                p = {"doc": d, "by": by, "forward": fw}
                if ext is not None:
                    p["extend"] = ext
                r = c.req("cursor.move", **p); m.move(by, fw, bool(ext))
            elif op == "normalize":
                to = rnd.choice([None, "lf", "crlf", "cr"])
                p = {"doc": d} if to is None else {"doc": d, "to": to}
                r = c.req("eol.normalize", **p)
                ch = m.normalize(to)
                cnt, dom, mixed = counts(m.t)
                cases += 1
                exp = {"changed": ch, "len_chars": len(m.t), "line_ending": dom, "mixed_line_endings": mixed}
                if not r["ok"] or r["result"] != exp:
                    mism += 1; msgs.append(("normalize-result", repr(t), to, r, exp))
            cases += 1
            if exp_err:
                if r["ok"] or (r["error"]["kind"], r["error"]["message"]) != exp_err:
                    mism += 1; msgs.append(("refusal", op, repr(t), r, exp_err))
            elif not r["ok"]:
                mism += 1; msgs.append(("unexpected-error", op, repr(t), r))
            # compare text, selection and counts
            got_t = c.ok("doc.text", doc=d)["text"]
            got_s = c.ok("cursor.get", doc=d)
            cnt, dom, mixed = counts(m.t)
            got_c = c.ok("eol.counts", doc=d)
            cases += 3
            if got_t != m.t:
                mism += 1; msgs.append(("text", seed, k, st, op, repr(t), repr(got_t), repr(m.t)))
                m.t = got_t
            if got_s != m.sel(d):
                mism += 1; msgs.append(("sel", seed, k, st, op, repr(t), r, got_s, m.sel(d)))
                m.a, m.h = got_s["anchor"], got_s["head"]
            if got_c != dict(cnt, line_ending=dom, mixed_line_endings=mixed):
                mism += 1; msgs.append(("counts", repr(m.t), got_c))
        c.ok("doc.close", doc=d, discard=True)
    rc = c.close()
    print(f"seed={seed} sessions={nsess} steps={steps} checks={cases} mismatches={mism} control_rc={rc}")
    print("ops", json.dumps(opcount, sort_keys=True))
    for x in msgs[:20]:
        print(json.dumps(x, ensure_ascii=False))


main()
