"""ui.status-bar: seeded random sessions against cursor.get / doc.info and a naive model.

usage: status_fuzz.py <seed> <sessions> <steps> [--size WxH]
env: KILN_BIN, KILN_CLI, KILN_WORK.

Each session: two documents of generated text (combining marks, a joiner, a ligature, CJK, emoji,
right-to-left text, tabs; LF/CRLF/CR/mixed/no endings) in a random encoding (utf-8, utf-8-bom,
utf-16le, utf-16be) and a random path (.txt or .rs), opened as start paths. Steps: keys (arrows with
shift/ctrl, Home/End, typed text, Enter, Backspace, Delete), clicks with `at` on Editor, tab
switches, and control requests behind the UI (nav.goto-line, eol.normalize, doc.insert). After each
step: snapshot, cursor.get, doc.info, doc.text of the current document.
Checks: the four bar values = cursor.get / doc.info (O3: registry); line/col = my naive model of
the head on doc.text; encoding = the encoding I wrote the file in (until the end, it never
changes); line ending and (mixed) = my naive count on doc.text (eol.md: dominant kind, ties LF,
then CRLF, then CR; none without breaks); each value painted on exactly one row equal to it;
bar absent with no document. Plus: log replay on a fresh copy reproduces the files (saves
included) and idle frames after every action leave the log byte-identical.
"""
import json, os, random, sys
sys.path.insert(0, os.path.dirname(__file__))
import kl, k2
from k2 import Session, find, rows, status, tabs_view, editor

ALPHA = list("abc xyz") + ["é", "́", "‍", "ﬁ", "中", "文", "😀", "👍🏽", "ש", "ל", "م", "٣", "\t", "_", "(", "1"]
EOLS = ["\n", "\r\n", "\r"]
ED = {"role": "multiline_text_input", "label": "Editor"}


def gen_text(rng):
    kind = rng.choice(["lf", "crlf", "cr", "mixed", "none", "empty"])
    if kind == "empty":
        return ""
    n = rng.randint(1, 8)
    out = []
    for i in range(n):
        out.append("".join(rng.choice(ALPHA) for _ in range(rng.randint(0, 14))))
        if i < n - 1 or rng.random() < 0.5:
            if kind == "none":
                continue
            out.append({"lf": "\n", "crlf": "\r\n", "cr": "\r"}.get(kind) or rng.choice(EOLS))
    return "".join(out)


def encode(text, enc):
    if enc == "utf-8":
        return text.encode("utf-8")
    if enc == "utf-8-bom":
        return b"\xef\xbb\xbf" + text.encode("utf-8")
    if enc == "utf-16le":
        return b"\xff\xfe" + text.encode("utf-16-le")
    return b"\xfe\xff" + text.encode("utf-16-be")


def breaks(text):
    """naive: list of (offset_after_break) and counts"""
    i, n = 0, len(text)
    starts = [0]
    c = {"lf": 0, "crlf": 0, "cr": 0}
    while i < n:
        if text[i] == "\r" and i + 1 < n and text[i + 1] == "\n":
            c["crlf"] += 1
            i += 2
            starts.append(i)
        elif text[i] == "\r":
            c["cr"] += 1
            i += 1
            starts.append(i)
        elif text[i] == "\n":
            c["lf"] += 1
            i += 1
            starts.append(i)
        else:
            i += 1
    return starts, c


def naive_le(text):
    _, c = breaks(text)
    tot = sum(c.values())
    if tot == 0:
        return "none", False
    best = max(c.values())
    for k in ("lf", "crlf", "cr"):
        if c[k] == best:
            dom = k
            break
    return dom, sum(1 for v in c.values() if v) > 1


def naive_linecol(text, head):
    starts, _ = breaks(text)
    line = max(i for i, s in enumerate(starts) if s <= head)
    return line, head - starts[line]


def session(seed, si, steps, size):
    rng = random.Random(seed * 7919 + si)
    init = kl.fresh(f"sz{seed}-{si}")
    docs = []
    for k in range(2):
        t = gen_text(rng)
        enc = rng.choice(["utf-8", "utf-8-bom", "utf-16le", "utf-16be"])
        p = f"d{k}" + rng.choice([".txt", ".rs"])
        with open(os.path.join(init, p), "wb") as f:
            f.write(encode(t, enc))
        docs.append((p, enc))
    reqs = []
    cur = 2  # last start path is current
    for st in range(steps):
        a = rng.choice(["key", "key", "key", "type", "click", "tab", "goto", "eol", "ins", "enter", "save"])
        if a in ("key", "type", "enter", "save"):
            reqs.append({"id": f"g{st}", "ui": "focus", "target": ED})
        if a == "key":
            k = rng.choice(["ArrowLeft", "ArrowRight", "ArrowUp", "ArrowDown", "Home", "End", "Backspace", "Delete"])
            mods = []
            if k.startswith("Arrow") or k in ("Home", "End"):
                if rng.random() < 0.4:
                    mods.append("shift")
            if k in ("ArrowLeft", "ArrowRight") and rng.random() < 0.3:
                mods.append("ctrl")
            reqs.append({"id": f"a{st}", "ui": "key", "key": k, "modifiers": mods})
        elif a == "type":
            reqs.append({"id": f"a{st}", "ui": "type", "text": "".join(rng.choice(ALPHA[:-3]) for _ in range(rng.randint(1, 3))).replace("\t", "t")})
        elif a == "enter":
            reqs.append({"id": f"a{st}", "ui": "key", "key": "Enter"})
        elif a == "save":
            reqs.append({"id": f"a{st}", "ui": "key", "key": "S", "modifiers": ["ctrl"]})
        elif a == "click":
            m = {"id": f"a{st}", "ui": "click", "target": ED, "at": {"row": rng.randint(0, 2), "col": rng.randint(0, 6)}}
            if rng.random() < 0.3:
                m["modifiers"] = ["shift"]
            reqs.append(m)
        elif a == "tab":
            cur = rng.choice([1, 2])
            reqs.append({"id": f"a{st}", "ui": "click", "target": {"role": "tab", "label": docs[cur - 1][0]}})
        elif a == "goto":
            reqs.append({"id": f"a{st}", "command": "nav.goto-line", "params": {"doc": cur, "line": rng.randint(0, 6), "col": rng.randint(0, 12)}})
            reqs.append({"id": f"f{st}", "ui": "frames", "count": 1})
        elif a == "eol":
            reqs.append({"id": f"a{st}", "command": "eol.normalize", "params": {"doc": cur, "to": rng.choice(["lf", "crlf", "cr"])}})
            reqs.append({"id": f"f{st}", "ui": "frames", "count": 1})
        elif a == "ins":
            reqs.append({"id": f"a{st}", "command": "doc.insert", "params": {"doc": cur, "offset": 0, "text": rng.choice(["\r", "\n", "\r\n", "x"])}})
            reqs.append({"id": f"f{st}", "ui": "frames", "count": 1})
        reqs += [{"id": f"s{st}", "ui": "snapshot"}, {"id": f"c{st}", "command": "cursor.get", "params": {"doc": cur}},
                 {"id": f"i{st}", "command": "doc.info", "params": {"doc": cur}}, {"id": f"t{st}", "command": "doc.text", "params": {"doc": cur}}]
    return init, docs, reqs, steps


def check(seed, si, steps, size):
    init, docs, reqs, steps = session(seed, si, steps, size)
    root = kl.copytree(init, "szui")
    paths = [p for p, e in docs]
    s = Session(reqs, root=root, paths=paths, size=size, name=f"sz{seed}-{si}")
    s.reqs_by_id = {r["id"]: r for r in reqs}
    mism = []
    refused_clicks = 0
    for st in range(steps):
        a = s.by.get(f"a{st}")
        if not a["ok"]:
            if a["error"].get("kind") == "not_actionable" and s.reqs_by_id.get(f"a{st}", {}).get("at") is not None:
                refused_clicks += 1
            else:
                mism.append((st, "action refused", a))
        snap = s.snap(f"s{st}")
        c, i, t = s[f"c{st}"]["result"], s[f"i{st}"]["result"], s[f"t{st}"]["result"]["text"]
        cur = c["doc"]
        sb = status(snap)
        want = {"Line": f"Ln {c['line'] + 1}", "Column": f"Col {c['col'] + 1}", "Encoding": i["encoding"],
                "Line ending": i["line_ending"] + (" (mixed)" if i["mixed_line_endings"] else "")}
        for k, v in want.items():
            n = sb[k]
            if not isinstance(n, dict) or n.get("value") != v:
                mism.append((st, "bar vs registry", k, n if not isinstance(n, dict) else n.get("value"), v))
            elif rows(n) != [v]:
                mism.append((st, "bar rows", k, rows(n), v, n.get("offscreen")))
        nl, ncol = naive_linecol(t, c["head"])
        if (nl, ncol) != (c["line"], c["col"]):
            mism.append((st, "cursor.get vs naive", (c["line"], c["col"]), (nl, ncol)))
        le, mixed = naive_le(t)
        if (le, mixed) != (i["line_ending"], i["mixed_line_endings"]):
            mism.append((st, "doc.info line ending vs naive", (i["line_ending"], i["mixed_line_endings"]), (le, mixed), repr(t)))
        if i["encoding"] != docs[cur - 1][1]:
            mism.append((st, "encoding vs file", i["encoding"], docs[cur - 1][1]))
        ed = editor(snap)
        if ed is None or ed.get("value") != t or ed.get("selection") != {"anchor": c["anchor"], "head": c["head"]}:
            mism.append((st, "editor vs registry", ed and ed.get("selection"), c))
        cur_tab = [x[0] for x in tabs_view(snap) if x[1]]
        if cur_tab != [docs[cur - 1][0]]:
            mism.append((st, "current tab", cur_tab))
    rep = kl.copytree(init, "szrep")
    rc, out, err = k2.cli_run_log(s.log, rep)
    if rc != 0 or kl.read_tree(rep) != kl.read_tree(root):
        mism.append(("replay", rc, err[:200]))
    reqs2 = []
    for r in reqs:
        reqs2.append(r)
        # not after a refused action: it ran no frame, so the frames after it would be the
        # first frames after the preceding control requests (not idle)
        if "ui" in r and r["ui"] != "snapshot" and not str(r["id"]).startswith("f") and s.by[r["id"]].get("ok"):
            reqs2.append({"id": "idle-" + r["id"], "ui": "frames", "count": 20})
    s2 = Session(reqs2, root=kl.copytree(init, "szidle"), paths=paths, size=size, name=f"szi{seed}-{si}")
    if open(s2.logp, "rb").read() != open(s.logp, "rb").read():
        mism.append(("idle frames changed the log",))
    if any(r.get("result") != {"frames": 20, "settled": True} for r in s2.resp if str(r.get("id")).startswith("idle-")):
        mism.append(("idle answers",))
    return mism, steps, refused_clicks


if __name__ == "__main__":
    seed, n, steps = int(sys.argv[1]), int(sys.argv[2]), int(sys.argv[3])
    size = sys.argv[sys.argv.index("--size") + 1] if "--size" in sys.argv else None
    import collections
    cats = collections.Counter()
    tot = mm = rc = 0
    for si in range(n):
        m, k, r = check(seed, si, steps, size)
        for x in m:
            cats[(str(x[1]) if len(x) > 1 else str(x[0]), x[2] if len(x) > 2 and isinstance(x[2], str) else "")] += 1
        tot += k
        rc += r
        mm += len(m)
        for x in m[:5]:
            print(f"seed {seed} session {si}:", json.dumps(x, ensure_ascii=False, default=str)[:400])
    print("mismatch categories:", dict(cats))
    print(f"status_fuzz seed={seed} size={size} sessions={n} steps={tot} refused_at_clicks={rc} mismatches={mm}")
