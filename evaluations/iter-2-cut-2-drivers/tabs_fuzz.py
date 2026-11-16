"""Seeded random sessions for ui.tabs (and the status bar and panel opens) against a naive model.

usage: tabs_fuzz.py <seed> <sessions> <steps>
env: KILN_BIN, KILN_CLI, KILN_WORK.

Model: open documents (id -> path), the current document, the set of documents showing a
`Discard` button, the `Message` prefix, the predicted mutating commands. Dirty flags, caret and
encodings come from the registry (doc.list, cursor.get, doc.info in the same session: the
independent source of ADR-0008 O3). Each step: one action (UI or a control request behind the
UI's back), then `snapshot`, `doc.list`, and for the current document `cursor.get`, `doc.info`.
After the session: the mutating log vs the model, the log replayed with `kiln-cli run` on a fresh
copy (files equal), and the same script with 30 idle frames after every action (log byte-equal).
"""
import json, os, random, sys, shutil
sys.path.insert(0, os.path.dirname(__file__))
import kl, k2
from k2 import Session, find, one, rows, tabs_view, status, message, title, editor

FILES = {
    "a.txt": "alpha\nbeta\n",
    "b.rs": "fn main() {\n    let x = 1;\n}\n",
    "c d.txt": "space name\r\nsecond\r\n",
    "dir/é.txt": "é́ combining\rcr line\r",
    "dir/中文.txt": "中文 😀 x‍y ﬁ\n\tt\n",
    "m.txt": "a\nb\r\nc\rd",
    "u16.txt": "﻿utf16 le\nline\n".encode("utf-16-le"),
    "bom.txt": b"\xef\xbb\xbfbom\n",
    "be.txt": "﻿be\r\n".encode("utf-16-be"),
    "empty.txt": "",
}
PATHS = sorted(FILES)
PROJ = {"role": "button", "label": "Project"}
MODELLED = {"doc.open", "doc.close", "doc.save", "doc.insert", "doc.replace", "doc.undo"}


class Model:
    def __init__(self):
        self.docs = {}  # id -> path
        self.next = 1
        self.cur = None
        self.discard = set()
        self.msg = ""
        self.mut = []
        self.expanded_dir = False

    def by_path(self, p):
        for i, q in self.docs.items():
            if q == p:
                return i
        return None

    def open(self, p):
        i = self.by_path(p)
        if i is None:
            i = self.next
            self.next += 1
            self.docs[i] = p
        self.cur = i
        self.mut.append(("doc.open", {"path": p}))
        return i

    def closed(self, i):
        del self.docs[i]
        self.discard.discard(i)
        if self.cur == i:
            self.cur = max(self.docs) if self.docs else None


def gen_action(rng, m, dirty, step):
    acts = ["panel_open", "panel_open", "ctl_open"]
    if m.docs:
        acts += ["tab", "tab", "doclist", "close", "close", "type", "type", "type", "save", "undo", "ctl_insert",
                 "ctl_save", "ctl_close", "ctl_close_discard", "ctl_cursor", "key_move", "click_editor"]
    if m.discard:
        acts += ["discard", "discard"]
    return rng.choice(acts)


def session(seed, si, steps, check_idle=True):
    """Builds the script step by step; before each step the prefix is run so that predictions that
    need the registry (dirty flags, the caret) use the previous step's control answers."""
    rng = random.Random(seed * 100003 + si)
    init = kl.fresh(f"tz{seed}-{si}")
    kl.write_files(init, FILES)
    m = Model()
    reqs = [{"id": "p0", "ui": "click", "target": PROJ},
            {"id": "p1", "ui": "click", "target": {"role": "tree_item", "label": "dir"}}]
    expect = []
    for st in range(steps):
        prev = None
        if st > 0:
            prev = Session(reqs, root=kl.copytree(init, "tzrun"), name="tzpre")
            pdl = prev[f"l{st-1}"]["result"]["docs"]
            prevdirty = {d["doc"]: d["dirty"] for d in pdl}
            m.discard = {i for i in m.discard if prevdirty.get(i)}
            if expect:
                expect[-1]["model"] = (expect[-1]["model"][0], expect[-1]["model"][1], set(m.discard), expect[-1]["model"][3])
        else:
            prevdirty = {}
        a = gen_action(rng, m, None, st)
        pre = len(reqs)
        e = {"step": st, "act": a}
        if a == "panel_open":
            p = rng.choice(PATHS)
            reqs.append({"id": f"a{st}", "ui": "click", "target": {"role": "tree_item", "label": p}})
            m.open(p)
            m.msg = ""
        elif a == "ctl_open":
            p = rng.choice(PATHS)
            reqs.append({"id": f"a{st}", "command": "doc.open", "params": {"path": p}})
            reqs.append({"id": f"f{st}", "ui": "frames", "count": 1})
            m.open(p)
        elif a in ("tab", "doclist"):
            i = rng.choice(sorted(m.docs))
            if a == "tab":
                reqs.append({"id": f"a{st}", "ui": "click", "target": {"role": "tab", "label": m.docs[i]}})
            else:
                reqs.append({"id": f"a{st}", "ui": "click", "target": {"role": "label", "value": m.docs[i]}})
            m.cur = i
        elif a == "close":
            i = rng.choice(sorted(m.docs))
            reqs.append({"id": f"a{st}", "ui": "click", "target": {"role": "button", "label": "Close " + m.docs[i]}})
            if prevdirty.get(i):
                m.discard.add(i)
                m.msg = "unsaved_changes"
            else:
                m.mut.append(("doc.close", {"doc": i}))
                m.closed(i)
                m.msg = ""
        elif a == "discard":
            i = rng.choice(sorted(m.discard))
            reqs.append({"id": f"a{st}", "ui": "click", "target": {"role": "button", "label": "Discard " + m.docs[i]}})
            m.mut.append(("doc.close", {"doc": i, "discard": True}))
            m.closed(i)
            m.msg = ""
        elif a == "type":
            t = rng.choice(["x", "é", "中", "😀", "ab"])
            reqs.append({"id": f"g{st}", "ui": "focus", "target": {"role": "multiline_text_input", "label": "Editor"}})
            reqs.append({"id": f"a{st}", "ui": "type", "text": t})
            c = prev[f"c{st-1}"]["result"]
            if c["anchor"] == c["head"]:
                m.mut.append(("doc.insert", {"doc": m.cur, "offset": c["head"], "text": t}))
            else:
                m.mut.append(("doc.replace", {"doc": m.cur, "start": min(c["anchor"], c["head"]),
                                              "end": max(c["anchor"], c["head"]), "text": t}))
        elif a == "key_move":
            k = rng.choice(["ArrowLeft", "ArrowRight", "ArrowUp", "ArrowDown", "Home", "End"])
            mods = rng.choice([[], ["shift"]])
            reqs.append({"id": f"g{st}", "ui": "focus", "target": {"role": "multiline_text_input", "label": "Editor"}})
            reqs.append({"id": f"a{st}", "ui": "key", "key": k, "modifiers": mods})
        elif a == "click_editor":
            reqs.append({"id": f"a{st}", "ui": "click", "target": {"role": "multiline_text_input", "label": "Editor"}})
        elif a == "save":
            reqs.append({"id": f"g{st}", "ui": "focus", "target": {"role": "multiline_text_input", "label": "Editor"}})
            reqs.append({"id": f"a{st}", "ui": "key", "key": "S", "modifiers": ["ctrl"]})
            m.mut.append(("doc.save", {"doc": m.cur}))
            m.discard.discard(m.cur)
            m.msg = ""
        elif a == "undo":
            reqs.append({"id": f"g{st}", "ui": "focus", "target": {"role": "multiline_text_input", "label": "Editor"}})
            reqs.append({"id": f"a{st}", "ui": "key", "key": "Z", "modifiers": ["ctrl"]})
            m.mut.append(("doc.undo", {"doc": m.cur}))
            e["refresh"] = True
        elif a == "ctl_insert":
            i = rng.choice(sorted(m.docs))
            reqs.append({"id": f"a{st}", "command": "doc.insert", "params": {"doc": i, "offset": 0, "text": "Q"}})
            reqs.append({"id": f"f{st}", "ui": "frames", "count": 1})
            m.mut.append(("doc.insert", {"doc": i, "offset": 0, "text": "Q"}))
        elif a == "ctl_save":
            i = rng.choice(sorted(m.docs))
            reqs.append({"id": f"a{st}", "command": "doc.save", "params": {"doc": i}})
            reqs.append({"id": f"f{st}", "ui": "frames", "count": 1})
            m.mut.append(("doc.save", {"doc": i}))
            m.discard.discard(i)
        elif a in ("ctl_close", "ctl_close_discard"):
            i = rng.choice(sorted(m.docs))
            disc = a == "ctl_close_discard"
            reqs.append({"id": f"a{st}", "command": "doc.close", "params": {"doc": i, "discard": True} if disc else {"doc": i}})
            reqs.append({"id": f"f{st}", "ui": "frames", "count": 1})
            if disc or not prevdirty.get(i):
                m.mut.append(("doc.close", {"doc": i, "discard": True} if disc else {"doc": i}))
                m.closed(i)
            else:
                e["expect_refused"] = True
        elif a == "ctl_cursor":
            i = rng.choice(sorted(m.docs))
            reqs.append({"id": f"a{st}", "command": "nav.goto-line", "params": {"doc": i, "line": 1, "col": 2}})
            reqs.append({"id": f"f{st}", "ui": "frames", "count": 1})
        reqs.append({"id": f"s{st}", "ui": "snapshot"})
        reqs.append({"id": f"l{st}", "command": "doc.list"})
        if m.cur is not None:
            reqs.append({"id": f"c{st}", "command": "cursor.get", "params": {"doc": m.cur}})
            reqs.append({"id": f"i{st}", "command": "doc.info", "params": {"doc": m.cur}})
        e["model"] = (dict(m.docs), m.cur, set(m.discard), m.msg)
        expect.append(e)
    # final discard refresh for the last step
    last = Session(reqs, root=kl.copytree(init, "tzrun"), name="tzpre")
    dd = {d["doc"]: d["dirty"] for d in last[f"l{steps-1}"]["result"]["docs"]}
    m.discard = {i for i in m.discard if dd.get(i)}
    expect[-1]["model"] = (expect[-1]["model"][0], expect[-1]["model"][1], set(m.discard), expect[-1]["model"][3])
    return init, reqs, expect, m


def check(seed, si, steps):
    init, reqs, expect, m = session(seed, si, steps)
    root = kl.copytree(init, "tzui")
    s = Session(reqs, root=root, name=f"tz{seed}-{si}")
    mism = []
    discard = set()
    for e in expect:
        st = e["step"]
        docs, cur, mdiscard, msg = e["model"]
        a = s.by.get(f"a{st}")
        if a is None or not a.get("ok"):
            if not (e.get("expect_refused") and a and a.get("error", {}).get("kind") == "unsaved_changes"):
                mism.append((st, e["act"], "action refused", a))
        snap = s.snap(f"s{st}")
        dl = s[f"l{st}"]["result"]["docs"]
        reg = [(d["doc"], d["path"], d["dirty"]) for d in dl]
        if [x[0] for x in reg] != sorted(docs) or [x[1] for x in reg] != [docs[i] for i in sorted(docs)]:
            mism.append((st, e["act"], "model docs", sorted(docs.items()), reg))
        tv = tabs_view(snap)
        if [t[0] for t in tv] != [x[1] for x in reg]:
            mism.append((st, e["act"], "tab labels", [t[0] for t in tv], reg))
        else:
            for t, x in zip(tv, reg):
                want_tog = (x[0] == cur)
                if t[1] is not want_tog:
                    mism.append((st, e["act"], "toggled", t[0], t[1], "cur", cur))
                if t[2] != x[2]:
                    mism.append((st, e["act"], "modified", t[0], t[2], x[2]))
                if not t[3]:
                    mism.append((st, e["act"], "no close button", t[0]))
                if t[4] != (x[0] in mdiscard):
                    mism.append((st, e["act"], "discard", t[0], t[4], sorted(mdiscard)))
                if rows(t[5]) and rows(t[5]) != [x[1]]:
                    mism.append((st, e["act"], "tab rows", rows(t[5])))
        tl = find(snap, role="tab_list", label="Tabs")
        if bool(tl) != bool(reg):
            mism.append((st, e["act"], "tab_list presence", bool(tl)))
        mv = message(snap) or ""
        if (msg == "" and mv != "") or (msg and not mv.startswith(msg)):
            mism.append((st, e["act"], "message", msg, mv))
        docsnodes = find(snap, role="list", label="Documents")[0]
        if [c.get("value") for c in docsnodes["children"]] != [x[1] for x in reg]:
            mism.append((st, e["act"], "Documents", [c.get("value") for c in docsnodes["children"]]))
        tt = title(snap)
        if cur is None:
            if tt is not None or editor(snap) is not None:
                mism.append((st, e["act"], "title/editor without doc"))
        else:
            dirty = dict((x[0], x[2]) for x in reg).get(cur)
            want = docs[cur] + (" — modified — Kiln" if dirty else " — Kiln")
            if tt != want:
                mism.append((st, e["act"], "title", tt, want))
            ed = editor(snap)
        # status bar
        sb = status(snap)
        if cur is None:
            if any(v is not None for v in sb.values()):
                mism.append((st, e["act"], "status without doc", {k: (v and v.get("value")) for k, v in sb.items()}))
        else:
            if not s[f"c{st}"].get("ok") or not s[f"i{st}"].get("ok"):
                mism.append((st, e["act"], "check request refused", s[f"c{st}"], sorted(docs.items()), reg))
                continue
            c = s[f"c{st}"]["result"]
            inf = s[f"i{st}"]["result"]
            want = {"Line": f"Ln {c['line'] + 1}", "Column": f"Col {c['col'] + 1}", "Encoding": inf["encoding"],
                    "Line ending": inf["line_ending"] + (" (mixed)" if inf["mixed_line_endings"] else "")}
            for k, v in want.items():
                n = sb[k]
                if n is None or n == "AMBIG" or n.get("value") != v or (rows(n) and rows(n) != [v]):
                    mism.append((st, e["act"], "status", k, n if not isinstance(n, dict) else (n.get("value"), rows(n)), v))
            ed = editor(snap)
            if ed is not None and ed.get("selection") != {"anchor": c["anchor"], "head": c["head"]}:
                mism.append((st, e["act"], "editor selection", ed.get("selection"), c))
    mut = [x for x in s.mut_log() if x[0] in MODELLED]
    mm = [x for x in m.mut]
    if mut != mm:
        i = next((i for i, (x, y) in enumerate(zip(mut, mm)) if x != y), min(len(mut), len(mm)))
        mism.append(("log", "mutating log differs at", i, mut[i:i + 3], mm[i:i + 3], len(mut), len(mm)))
    rep = kl.copytree(init, "tzrep")
    rc, out, err = k2.cli_run_log(s.log, rep)
    if rc != 0:
        mism.append(("replay rc", rc, err[:200]))
    if kl.read_tree(rep) != kl.read_tree(root):
        mism.append(("replay files differ",))
    # idle frames
    reqs2 = []
    for r in reqs:
        reqs2.append(r)
        if "ui" in r and r["ui"] not in ("snapshot",) and not str(r["id"]).startswith("f"):
            reqs2.append({"id": "idle-" + str(r["id"]), "ui": "frames", "count": 30})
    s2 = Session(reqs2, root=kl.copytree(init, "tzidle"), name=f"tzi{seed}-{si}")
    if open(s2.logp, "rb").read() != open(s.logp, "rb").read():
        mism.append(("idle frames changed the log",))
    bad_idle = [r for r in s2.resp if str(r.get("id")).startswith("idle-") and r.get("result") != {"frames": 30, "settled": True}]
    if bad_idle:
        mism.append(("idle answers", bad_idle[:2]))
    return mism, len(expect), len(s.resp)


if __name__ == "__main__":
    seed, n, steps = int(sys.argv[1]), int(sys.argv[2]), int(sys.argv[3])
    total = 0
    tm = 0
    lines = 0
    for si in range(n):
        mm, k, nl = check(seed, si, steps)
        total += k
        lines += nl
        tm += len(mm)
        for x in mm[:6]:
            print(f"seed {seed} session {si}:", json.dumps(x, ensure_ascii=False, default=str)[:400])
        if len(mm) > 6:
            print(f"  ... {len(mm)} mismatches in session {si}")
    print(f"tabs_fuzz seed={seed} sessions={n} steps={total} response_lines={lines} mismatches={tm}")
