"""Boundary and hostile cases for the editor view and the shell, through `kiln script`.

usage: edges.py <section>...   (sections: start size limit empty enter save many title refuse
                                 scroll keys determinism settle shell plain wide logfile)
Prints one block per case: what was sent (abridged) and what was observed.
"""
import json, os, sys, subprocess, time
sys.path.insert(0, os.path.dirname(__file__))
import kl

COL = {"plain": "#dcdcdc", "keyword": "#569cd6", "type": "#4ec9b0", "function": "#dcdcaa",
       "string": "#ce9178", "number": "#b5cea8", "comment": "#6a9955"}


def S(files, paths, reqs, size=None, memory=False, log=True, extra=(), root=None, t=600):
    if root is None and not memory:
        root = kl.fresh("edge"); kl.write_files(root, files)
    lg = (root or kl.fresh("memlog")) + ".log" if log else None
    t0 = time.time()
    rc, out, err, sp = kl.kscript(reqs, root=root, paths=paths, size=size, memory=memory, log=lg, extra=extra, t=t)
    dt = time.time() - t0
    res = {}
    order = []
    for line in out.decode().splitlines():
        r = json.loads(line)
        res[r.get("id")] = r
        order.append(r)
    logl = [json.loads(l) for l in open(lg)] if lg and os.path.exists(lg) else None
    return dict(rc=rc, res=res, order=order, err=err.decode("utf-8", "replace"), log=logl, root=root, dt=dt, out=out)


def node(r, label=None, role=None):
    return kl.one(r, role=role, label=label)


def brief(n):
    if n is None:
        return None
    m = {k: v for k, v in n.items() if k not in ("children",)}
    if "rows" in m:
        m["rows"] = kl.rows_text(n)[:6]
    return m


def docs(r):
    d = node(r, label="Documents")
    return [c.get("value") for c in d["children"]] if d else None


def show(_title, **kv):
    print("==", _title)
    for k, v in kv.items():
        print("   ", k, "=", json.dumps(v, ensure_ascii=False, default=str)[:700])


SNAP = {"id": "snap", "ui": "snapshot"}
F1 = {"id": "f1", "ui": "frames", "count": 1}


def sec_start():
    files = {"a.rs": "fn a() {}\n", "b.txt": "bee\n", "d/x.txt": "x"}
    r = S(files, ["a.rs", "missing.rs", "b.txt"], [SNAP, {"id": 1, "command": "doc.list"}])
    s = r["res"]["snap"]
    show("start paths a.rs missing.rs b.txt", rc=r["rc"], docs=docs(s), title=brief(node(s, "Title")),
         message=brief(node(s, "Message")), log=r["log"], err=r["err"], list=r["res"][1])
    r = S(files, ["a.rs", "missing.rs"], [SNAP])
    s = r["res"]["snap"]
    show("start paths a.rs missing.rs (refused last)", rc=r["rc"], title=brief(node(s, "Title")), message=brief(node(s, "Message")), log=r["log"])
    for bad in ["d", "../x", "/etc/passwd", "a.rs/x", "", "d/"]:
        r = S(files, [bad], [SNAP])
        s = r["res"].get("snap")
        show(f"start path {bad!r}", rc=r["rc"], message=brief(node(s, "Message")) if s else None, editor=node(s, "Editor") is not None if s else None, log=r["log"], err=r["err"][:200])
    r = S(files, ["a.rs", "a.rs"], [SNAP, {"id": 1, "command": "doc.list"}])
    show("start path twice", rc=r["rc"], docs=docs(r["res"]["snap"]), log=r["log"], list=r["res"][1])
    r = S({}, ["a.rs"], [SNAP], memory=True)
    s = r["res"].get("snap")
    show("--memory with start path", rc=r["rc"], message=brief(node(s, "Message")) if s else None, log=r["log"], err=r["err"][:200])
    # current document rule
    reqs = [{"id": 1, "command": "doc.open", "params": {"path": "b.txt"}}, F1, {"id": "s1", "ui": "snapshot"},
            {"id": 2, "command": "doc.open", "params": {"path": "a.rs"}}, F1, {"id": "s2", "ui": "snapshot"},
            {"id": 3, "command": "doc.close", "params": {"doc": 3}}, F1, {"id": "s3", "ui": "snapshot"},
            {"id": 4, "command": "doc.close", "params": {"doc": 1}}, F1, {"id": "s4", "ui": "snapshot"},
            {"id": 5, "command": "doc.close", "params": {"doc": 2}}, F1, {"id": "s5", "ui": "snapshot"}]
    r = S(files, ["d/x.txt"], reqs)
    for k in ("s1", "s2", "s3", "s4", "s5"):
        s = r["res"][k]
        show(f"current-doc {k}", title=(node(s, "Title") or {}).get("value"), editor=(node(s, "Editor") or {}).get("value"), docs=docs(s))
    show("current-doc responses", r=[r["res"][i] for i in (1, 2, 3, 4, 5)])


def sec_size():
    files = {"a.rs": "fn a() {}\n" * 3}
    for sz in ["199x800", "200x199", "200x200", "8192x8192", "8193x200", "1280X800", "0x0", "abc", "-1x5",
               "+1280x800", "01280x800", "1280x800x", " 1280x800", "1280x", "x800", "1280×800", "99999999999999999999x200"]:
        rc, out, err, _ = kl.kscript([SNAP], root=kl.fresh("sz"), size=sz)
        print("== size", repr(sz), "rc", rc, "stdout_bytes", len(out), "stderr_first", err.decode()[:160].splitlines()[:1])
    rc, out, err, _ = kl.kscript([SNAP], root=kl.fresh("sz"), extra=["--size", "300x300", "--size", "300x300"])
    print("== size twice rc", rc, err.decode().splitlines()[:1])


def sec_limit():
    n = 1048576
    body = "fn f() {}\n"
    t = (body * (n // len(body) + 1))[:n]
    for name, text in [("at.rs", t), ("over.rs", t + "x")]:
        r = S({name: text}, [name], [SNAP, {"id": "h", "command": "syntax.highlight", "params": {"doc": 1, "end": 20}}], t=900)
        ed = node(r["res"]["snap"], "Editor")
        colours = sorted({run["color"] for row in ed.get("rows", []) for run in row})
        show(f"size {name} len={len(text)}", rc=r["rc"], dt=round(r["dt"], 2), rows=len(ed.get("rows", [])), colours=colours,
             first=ed.get("rows", [[None]])[0][:3], hl=r["res"]["h"], message=brief(node(r["res"]["snap"], "Message")))


def sec_empty():
    for name, text in [("e.rs", ""), ("e.txt", ""), ("nl.rs", "\n"), ("crlf.rs", "\r\n"), ("cr.txt", "\r"), ("sp.rs", "   "), ("tab.rs", "\t\t")]:
        reqs = [{"id": "s0", "ui": "snapshot"}, {"id": "f", "ui": "focus", "target": {"label": "Editor"}},
                {"id": "k", "ui": "key", "key": "Enter"}, {"id": "t", "ui": "type", "text": "é"},
                {"id": "b", "ui": "key", "key": "Backspace"}, {"id": "b2", "ui": "key", "key": "Backspace"}, {"id": "b3", "ui": "key", "key": "Backspace"},
                {"id": "d", "ui": "key", "key": "Delete"}, {"id": "s1", "ui": "snapshot"},
                {"id": 1, "command": "doc.text", "params": {"doc": 1}}]
        r = S({name: text}, [name], reqs)
        show(f"empty-ish {name!r} {text!r}", rc=r["rc"], s0=brief(node(r["res"]["s0"], "Editor")), s1=brief(node(r["res"]["s1"], "Editor")),
             text=r["res"][1].get("result"), mut=[l for l in r["log"] if l["command"] in ("doc.insert", "doc.delete", "doc.replace")],
             bad=[x for x in r["order"] if not x.get("ok")])


def sec_enter():
    cases = [("lf.rs", "a\nb\n", 1), ("crlf.rs", "a\r\nb\r\n", 1), ("cr.rs", "a\rb\r", 1), ("mixed.rs", "a\r\nb\nc\r\n", 1),
             ("tie.rs", "a\r\nb\nc", 1), ("tie2.rs", "a\rb\r\nc", 1), ("none.rs", "ab", 1), ("crjoin.rs", "a\rb\n", 3),
             ("lfafterlonecr.rs", "a\rb\nc\nd\n", 2)]
    for name, text, col in cases:
        reqs = [{"id": "c", "ui": "click", "target": {"label": "Editor"}, "at": {"row": 0, "col": min(col, 1)}}]
        if name == "crjoin.rs":
            reqs = [{"id": "c", "ui": "click", "target": {"label": "Editor"}, "at": {"row": 1, "col": 1}}]
        if name == "lfafterlonecr.rs":
            reqs = [{"id": "c", "ui": "click", "target": {"label": "Editor"}, "at": {"row": 0, "col": 1}},
                    {"id": "r", "ui": "key", "key": "ArrowRight"}]
        reqs += [{"id": "k", "ui": "key", "key": "Enter"}, SNAP, {"id": 1, "command": "doc.text", "params": {"doc": 1}},
                 {"id": 2, "command": "cursor.get", "params": {"doc": 1}}]
        r = S({name: text}, [name], reqs)
        show(f"enter {name} {text!r}", text=r["res"][1]["result"]["text"], cursor=r["res"][2]["result"],
             mut=[l for l in r["log"] if l["command"] in ("doc.insert", "doc.replace", "cursor.set")],
             message=brief(node(r["res"]["snap"], "Message")), bad=[x for x in r["order"] if not x.get("ok")])


def nodac(argv, t=120):
    return subprocess.run(["timeout", str(t), "capsh", "--drop=cap_dac_override,cap_dac_read_search", "--"] +
                          ["-c", " ".join("'" + a.replace("'", "'\\''") + "'" for a in argv)], capture_output=True)


def sec_save():
    base = [{"id": "f", "ui": "focus", "target": {"label": "Editor"}}, {"id": "t", "ui": "type", "text": "Z"},
            {"id": "sv", "ui": "key", "key": "S", "modifiers": ["ctrl"]}, SNAP,
            {"id": 1, "command": "doc.info", "params": {"doc": 1}}]
    # 1 read-only file, without DAC capabilities
    root = kl.fresh("ro"); kl.write_files(root, {"a.txt": "abc\n"}); os.chmod(os.path.join(root, "a.txt"), 0o444)
    sp = os.path.join(kl.fresh("rosc"), "s.jsonl"); open(sp, "w").write(kl.jl(base))
    p = nodac([kl.BIN, "script", sp, "--root", root, "a.txt"])
    rr = {json.loads(l).get("id"): json.loads(l) for l in p.stdout.decode().splitlines()}
    show("save read-only file (0444) without DAC caps", rc=p.returncode, sv=rr.get("sv"), message=brief(node(rr["snap"], "Message")),
         title=(node(rr["snap"], "Title") or {}).get("value"), info=rr[1], file=open(os.path.join(root, "a.txt"), "rb").read(), err=p.stderr[:200])
    # read-only directory (no new temp file possible), without DAC caps
    root = kl.fresh("rod"); kl.write_files(root, {"d/a.txt": "abc\n"}); os.chmod(os.path.join(root, "d"), 0o555)
    open(sp, "w").write(kl.jl(base))
    p = nodac([kl.BIN, "script", sp, "--root", root, "d/a.txt"])
    rr = {json.loads(l).get("id"): json.loads(l) for l in p.stdout.decode().splitlines()}
    show("save in read-only dir (0555) without DAC caps", rc=p.returncode, message=brief(node(rr["snap"], "Message")), file=open(os.path.join(root, "d/a.txt"), "rb").read(), listing=sorted(os.listdir(os.path.join(root, "d"))))
    os.chmod(os.path.join(root, "d"), 0o755)
    # 2 as uid 0 (with caps): read-only file
    root = kl.fresh("ro0"); kl.write_files(root, {"a.txt": "abc\n"}); os.chmod(os.path.join(root, "a.txt"), 0o444)
    r = S(None, ["a.txt"], base, root=root)
    show("save read-only file (0444) as uid 0 with caps", rc=r["rc"], message=brief(node(r["res"]["snap"], "Message")), file=open(os.path.join(root, "a.txt"), "rb").read(), mode=oct(os.stat(os.path.join(root, "a.txt")).st_mode))
    # 3 the file replaced by a directory / a link after opening: via a control request in the middle we cannot
    #   change the disk; use a script that pauses? Not possible; instead: the 100 temp names.
    root = kl.fresh("tmp100"); kl.write_files(root, {"a.txt": "abc\n"})
    for i in range(100):
        open(os.path.join(root, f".kiln-tmp-{i}"), "w").write("t")
    reqs = base + [{"id": "rm", "command": "doc.text", "params": {"doc": 1}}]
    r = S(None, ["a.txt"], reqs, root=root)
    show("save with all 100 temp names taken", rc=r["rc"], message=brief(node(r["res"]["snap"], "Message")), file=open(os.path.join(root, "a.txt"), "rb").read(), log=[l["command"] for l in r["log"] if l["command"] == "doc.save"])
    # 4 a refused save, then a successful one clears Message: dir replaced -> use 99 names + then a control save?
    root = kl.fresh("tmp99"); kl.write_files(root, {"a.txt": "abc\n"})
    for i in range(100):
        open(os.path.join(root, f".kiln-tmp-{i}"), "w").write("t")
    # first save refused; then the script opens other doc? Message is cleared by a successful UI open or save.
    reqs = base + [{"id": "p", "ui": "focus", "target": {"label": "Path"}}, {"id": "pt", "ui": "type", "text": "b.txt"},
                   {"id": "po", "ui": "click", "target": {"label": "Open"}}, {"id": "snap2", "ui": "snapshot"}]
    kl.write_files(root, {"b.txt": "bbb"})
    r = S(None, ["a.txt"], reqs, root=root)
    show("refused save then UI open", m1=brief(node(r["res"]["snap"], "Message")), m2=brief(node(r["res"]["snap2"], "Message")), title=(node(r["res"]["snap2"], "Title") or {}).get("value"))


def sec_many():
    files = {f"f{i:03d}.rs": f"fn f{i}() {{}}\n" for i in range(150)}
    reqs = [{"id": i, "command": "doc.open", "params": {"path": f"f{i:03d}.rs"}} for i in range(150)] + [F1, SNAP]
    r = S(files, [], reqs)
    s = r["res"]["snap"]
    d = node(s, "Documents")
    off = [c.get("offscreen") for c in d["children"]]
    show("150 open documents", rc=r["rc"], entries=len(d["children"]), first=d["children"][0].get("value"), last=d["children"][-1].get("value"),
         offscreen_count=sum(1 for o in off if o), title=(node(s, "Title") or {}).get("value"), scroll_bars=len(kl.find(s, role="scroll_bar")))
    reqs2 = reqs[:-1] + [{"id": "sc", "ui": "scroll", "target": {"label": "Documents"}, "direction": "down", "steps": 100}, SNAP,
                         {"id": "ck", "ui": "click", "target": {"value": "f149.rs", "role": "label"}}, {"id": "s2", "ui": "snapshot"},
                         {"id": "ck0", "ui": "click", "target": {"value": "f000.rs"}}]
    r = S(files, [], reqs2)
    s = r["res"]["snap"]
    d = node(s, "Documents")
    vis = [c.get("value") for c in d["children"] if not c.get("offscreen")]
    show("150 docs scrolled 100 steps", sc=r["res"]["sc"], visible_first=vis[:1], visible_last=vis[-1:], nvis=len(vis),
         click_entry=r["res"]["ck"], title_after_click=(node(r["res"]["s2"], "Title") or {}).get("value"), click_offscreen=r["res"]["ck0"])


def sec_title():
    for p in ["x — modified — Kiln.rs", "modified.rs", "a b/ç 😀.txt", "rs", "a.rs.txt", "a.RS", "dir.rs/b.txt", ".rs"]:
        r = S({p: "fn x() {}\n"}, [p], [SNAP, {"id": "f", "ui": "focus", "target": {"label": "Editor"}}, {"id": "t", "ui": "type", "text": "q"}, {"id": "s2", "ui": "snapshot"}])
        s = r["res"]["snap"]
        ed = node(s, "Editor")
        show(f"title {p!r}", t0=(node(s, "Title") or {}).get("value"), t1=(node(r["res"]["s2"], "Title") or {}).get("value"),
             colours=sorted({run["color"] for row in ed.get("rows", []) for run in row}), docs=docs(s))


def sec_refuse():
    files = {"a.rs": "fn a() {}\nlet b = 2;\n"}
    cases = [
        {"ui": "snapshot", "x": 1}, {"ui": "frames"}, {"ui": "frames", "count": 0}, {"ui": "frames", "count": 601},
        {"ui": "frames", "count": 1.0}, {"ui": "frames", "count": "1"}, {"ui": "click"}, {"ui": "click", "target": {}},
        {"ui": "click", "target": {"labl": "Open"}}, {"ui": "click", "target": {"label": 3}},
        {"ui": "click", "target": {"label": "Open"}, "button": "middle"}, {"ui": "click", "target": {"label": "Open"}, "modifiers": ["super"]},
        {"ui": "click", "target": {"label": "Editor"}, "at": {"row": -1, "col": 0}}, {"ui": "click", "target": {"label": "Editor"}, "at": {"row": 0}},
        {"ui": "key", "key": "a"}, {"ui": "key", "key": "F13"}, {"ui": "key", "key": "Enter", "modifiers": "ctrl"},
        {"ui": "scroll", "target": {"label": "Editor"}, "direction": "sideways"}, {"ui": "scroll", "target": {"label": "Editor"}, "steps": 0},
        {"ui": "scroll", "target": {"label": "Editor"}, "steps": 101, "direction": "down"}, {"ui": "type"}, {"ui": "type", "text": 5},
        {"ui": "Snapshot"}, {"ui": "drag"}, {"ui": 3}, {"ui": None},
        {"ui": "click", "target": {"label": "Nope"}}, {"ui": "click", "target": {"role": "label"}},
        {"ui": "click", "target": {"label_contains": "o"}}, {"ui": "focus", "target": {"label": "Open"}},
        {"ui": "type", "text": "x"},
        {"ui": "click", "target": {"label": "Editor"}, "at": {"row": 2, "col": 0}}, {"ui": "click", "target": {"label": "Editor"}, "at": {"row": 0, "col": 10}},
        {"ui": "click", "target": {"label": "Open"}, "at": {"row": 0, "col": 5}}, {"ui": "click", "target": {"label": "Path"}, "at": {"row": 0, "col": 0}},
        {"ui": "scroll", "target": {"label": "Path"}, "direction": "down"},
        {"ui": "scroll", "target": {"label": "Title"}, "direction": "down"},
        {"ui": "focus", "target": {"label": "Title"}},
        {"ui": "frames", "count": 1, "target": {"label": "Editor"}},
        {"ui": "snapshot", "id": [1]},
        {"ui": "click", "target": {"label": "Open", "role": "button", "value_contains": ""}},
    ]
    reqs = [{"id": "s0", "ui": "snapshot"}]
    for i, c in enumerate(cases):
        c = dict(c)
        if "id" not in c:
            c["id"] = f"r{i}"
        reqs.append(c)
    reqs += [{"id": "s1", "ui": "snapshot"}, {"id": "t", "command": "doc.text", "params": {"doc": 1}}]
    r = S(files, ["a.rs"], reqs)
    for x in r["order"]:
        if x.get("id") in ("s0", "s1"):
            continue
        print("  ", json.dumps(x, ensure_ascii=False)[:300])
    print("   frames s0", r["res"]["s0"]["result"]["frames"], "s1", r["res"]["s1"]["result"]["frames"])
    print("   log", [l["command"] for l in r["log"]])


def sec_scroll():
    text = "".join(f"line{i:03d} fn f{i}() {{ let x = {i}; }}\n" for i in range(200))
    reqs = [{"id": "s0", "ui": "snapshot"}, {"id": "sc", "ui": "scroll", "target": {"label": "Editor"}, "direction": "down", "steps": 3},
            {"id": "s1", "ui": "snapshot"}, {"id": "c1", "command": "cursor.get", "params": {"doc": 1}},
            {"id": "sc2", "ui": "scroll", "target": {"label": "Editor"}, "direction": "down", "steps": 100}, {"id": "s2", "ui": "snapshot"},
            {"id": "sc3", "ui": "scroll", "target": {"label": "Editor"}, "direction": "up", "steps": 1}, {"id": "s3", "ui": "snapshot"},
            {"id": "scl", "ui": "scroll", "target": {"label": "Editor"}, "direction": "right", "steps": 1}, {"id": "s3b", "ui": "snapshot"},
            {"id": "sc4", "ui": "scroll", "target": {"label": "Editor"}, "direction": "up", "steps": 100},
            {"id": "f", "ui": "focus", "target": {"label": "Editor"}}]
    reqs += [{"id": f"dn{i}", "ui": "key", "key": "ArrowDown"} for i in range(60)]
    reqs += [{"id": "s4", "ui": "snapshot"}, {"id": "c4", "command": "cursor.get", "params": {"doc": 1}},
             {"id": "pg", "ui": "key", "key": "PageDown"}, {"id": "c5", "command": "cursor.get", "params": {"doc": 1}},
             {"id": "gl", "command": "cursor.set", "params": {"doc": 1, "anchor": 9000}}, F1, {"id": "s5", "ui": "snapshot"},
             {"id": "ins", "command": "doc.insert", "params": {"doc": 1, "offset": 0, "text": "X"}},
             {"id": "fa", "ui": "frames", "count": 1}, {"id": "fb", "ui": "frames", "count": 1}, {"id": "s6", "ui": "snapshot"},
             {"id": "t", "command": "doc.text", "params": {"doc": 1}}]
    r = S({"big.rs": text}, ["big.rs"], reqs)
    lines = text.split("\n")
    for k in ("s0", "s1", "s2", "s3", "s3b", "s4", "s5", "s6"):
        ed = node(r["res"][k], "Editor")
        rt = kl.rows_text(ed)
        idx = [lines.index(x) if x in lines else ("X?" + x[:12]) for x in rt]
        consecutive = all(isinstance(a, int) and isinstance(b, int) and b == a + 1 for a, b in zip(idx, idx[1:]))
        print(f"== scroll {k}: rows={len(rt)} first_line={idx[:1]} last_line={idx[-1:]} consecutive={consecutive} sel={ed.get('selection')} value_ok={ed.get('value') in (text, 'X' + text)}")
    for k in ("sc", "sc2", "sc3", "scl", "sc4", "pg", "fa", "fb", "c1", "c4", "c5", "gl"):
        print("  ", k, json.dumps(r["res"][k])[:200])
    print("   log mutating", [l for l in r["log"] if l["command"] in ("cursor.set", "cursor.move", "doc.insert")][:3], "count", sum(1 for l in r["log"] if l["command"] == "cursor.move"))


def sec_keys():
    text = "fn ab_c(x: u8) {}\r\nlet y = 2;\n"
    keys = [("Tab", []), ("Escape", []), ("PageUp", []), ("PageDown", []), ("F1", []), ("Space", []), ("A", []),
            ("A", ["ctrl"]), ("C", ["ctrl"]), ("V", ["ctrl"]), ("X", ["ctrl"]), ("Z", ["alt"]), ("Y", ["ctrl"]),
            ("Z", ["ctrl", "shift"]), ("Z", ["command"]), ("S", ["command"]), ("Home", ["ctrl"]), ("End", ["ctrl"]),
            ("ArrowUp", ["ctrl"]), ("Backspace", ["ctrl"]), ("Delete", ["ctrl"]), ("Enter", ["shift"]), ("Enter", ["ctrl"]),
            ("ArrowRight", ["alt"]), ("Q", ["ctrl"]), ("W", ["ctrl"])]
    for k, mods in keys:
        reqs = [{"id": "c", "ui": "click", "target": {"label": "Editor"}, "at": {"row": 0, "col": 5}},
                {"id": "sel", "ui": "key", "key": "ArrowRight", "modifiers": ["shift"]},
                {"id": "t", "ui": "type", "text": "Q"}, {"id": "k", "ui": "key", "key": k, "modifiers": mods}, SNAP,
                {"id": 1, "command": "doc.text", "params": {"doc": 1}}, {"id": 2, "command": "cursor.get", "params": {"doc": 1}}]
        r = S({"a.rs": text}, ["a.rs"], reqs)
        pre = r["log"]
        mut = [(l["command"], l["params"]) for l in pre if l["command"] in ("doc.insert", "doc.delete", "doc.replace", "cursor.set", "cursor.move", "doc.undo", "doc.redo", "doc.save")]
        after_q = mut[mut.index(("doc.replace", {"doc": 1, "start": 5, "end": 6, "text": "Q"})) + 2:] if ("doc.replace", {"doc": 1, "start": 5, "end": 6, "text": "Q"}) in mut else mut
        print("== key", k, mods, "resp", json.dumps(r["res"]["k"].get("result") or r["res"]["k"].get("error")), "text", json.dumps(r["res"][1]["result"]["text"]),
              "sel", (r["res"][2]["result"]["anchor"], r["res"][2]["result"]["head"]), "dispatched_after_typing", after_q,
              "message", (node(r["res"]["snap"], "Message") or {}).get("value"))


def sec_determinism():
    files = {"src/a.rs": "fn main() {\n    let x = \"é😀\";\n}\r\n", "b.txt": "x\ty\n"}
    reqs = [{"id": "f", "ui": "focus", "target": {"label": "Path"}}, {"id": "t", "ui": "type", "text": "b.txt"},
            {"id": "k", "ui": "key", "key": "Enter"}, {"id": "c", "ui": "click", "target": {"label": "Editor"}, "at": {"row": 0, "col": 2}},
            {"id": "y", "ui": "type", "text": "Ω"}, {"id": "s", "ui": "key", "key": "S", "modifiers": ["ctrl"]}, SNAP,
            {"id": 1, "command": "doc.list"}]
    outs = []
    for depth in ("a", "a/b/c/d/e"):
        base = kl.fresh("det")
        root = os.path.join(base, depth)
        os.makedirs(root)
        kl.write_files(root, files)
        lg = os.path.join(base, "log.jsonl")
        rc, out, err, _ = kl.kscript(reqs, root=root, paths=["src/a.rs"], log=lg)
        outs.append((rc, out, open(lg, "rb").read(), kl.read_tree(root)))
    # under load: 4 busy processes
    busy = [subprocess.Popen(["timeout", "60", "python3", "-c", "while True: pass"]) for _ in range(4)]
    base = kl.fresh("detload"); root = os.path.join(base, "r"); os.makedirs(root); kl.write_files(root, files)
    lg = os.path.join(base, "log.jsonl")
    rc, out, err, _ = kl.kscript(reqs, root=root, paths=["src/a.rs"], log=lg)
    for b in busy:
        b.kill()
    outs.append((rc, out, open(lg, "rb").read(), kl.read_tree(root)))
    print("== determinism: rcs", [o[0] for o in outs], "stdout equal", len({o[1] for o in outs}) == 1, "log equal", len({o[2] for o in outs}) == 1,
          "files equal", all(o[3] == outs[0][3] for o in outs), "stdout md5", kl.md5(outs[0][1]), "lines", outs[0][1].count(b"\n"))


def sec_settle():
    files = {"a.rs": "fn a() {}\n", "b.rs": "x"}
    reqs = [{"id": "s0", "ui": "snapshot"}, {"id": "f0", "ui": "frames", "count": 1}, {"id": "s1", "ui": "snapshot"},
            {"id": 1, "command": "doc.list"}, {"id": "f1", "ui": "frames", "count": 1}, {"id": "f2", "ui": "frames", "count": 1},
            {"id": 2, "command": "doc.open", "params": {"path": "nope.rs"}}, {"id": "f3", "ui": "frames", "count": 1},
            {"id": 3, "command": "doc.open", "params": {"path": "b.rs"}}, {"id": "s2", "ui": "snapshot"}, {"id": "f4", "ui": "frames", "count": 1},
            {"id": "f5", "ui": "frames", "count": 1}, {"id": "s3", "ui": "snapshot"}, {"id": "f6", "ui": "frames", "count": 600},
            {"id": "s4", "ui": "snapshot"}]
    r = S(files, ["a.rs"], reqs)
    for x in r["order"]:
        print("  ", json.dumps(x)[:160] if "root" not in json.dumps(x)[:400] else (x["id"], "frames", x["result"]["frames"], "title", (node(x, "Title") or {}).get("value")))
    print("   log", [l["command"] for l in r["log"]])
    # control-only script, no start paths: log equals kiln-cli run's log
    reqs = [{"id": 1, "command": "doc.open", "params": {"path": "a.rs"}}, {"id": 2, "command": "doc.insert", "params": {"doc": 1, "offset": 0, "text": "Y"}}]
    r = S(files, [], reqs)
    root2 = kl.fresh("cl"); kl.write_files(root2, files)
    rc, out, err = kl.cli_run(kl.jl(reqs), root2, log=root2 + ".log")
    print("== control-only script: kiln stdout == kiln-cli run stdout", r["out"] == out, "logs equal", open(r["root"] + ".log").read() == open(root2 + ".log").read(), "log", r["log"])


def sec_shell():
    files = {"src/a.rs": "fn a() {}\n", "b.txt": "b", "d/e.txt": "e"}
    reqs = [{"id": "s0", "ui": "snapshot"}, {"id": "c0", "ui": "click", "target": {"label": "Open"}}, {"id": "s1", "ui": "snapshot"},
            {"id": "f", "ui": "focus", "target": {"label": "Path"}}, {"id": "t", "ui": "type", "text": "src/missing.rs"},
            {"id": "c1", "ui": "click", "target": {"label": "Open"}}, {"id": "s2", "ui": "snapshot"},
            {"id": "f2", "ui": "focus", "target": {"label": "Path"}}, {"id": "ka", "ui": "key", "key": "A", "modifiers": ["ctrl"]},
            {"id": "t2", "ui": "type", "text": "src/a.rs"}, {"id": "k2", "ui": "key", "key": "Enter"}, {"id": "s3", "ui": "snapshot"},
            {"id": "k3", "ui": "key", "key": "Enter"}, {"id": "s4", "ui": "snapshot"},
            {"id": "f3", "ui": "focus", "target": {"label": "Path"}}, {"id": "ka2", "ui": "key", "key": "A", "modifiers": ["ctrl"]},
            {"id": "t3", "ui": "type", "text": "d"}, {"id": "c3", "ui": "click", "target": {"label": "Open"}}, {"id": "s5", "ui": "snapshot"},
            {"id": "ka3", "ui": "key", "key": "A", "modifiers": ["ctrl"]}, {"id": "t4", "ui": "type", "text": "../x"},
            {"id": "c4", "ui": "click", "target": {"label": "Open"}}, {"id": "s6", "ui": "snapshot"},
            {"id": "c5", "ui": "click", "target": {"label": "Open"}, "button": "secondary"}, {"id": "s7", "ui": "snapshot"},
            {"id": "c6", "ui": "click", "target": {"label": "Message"}}, {"id": "c7", "ui": "click", "target": {"label": "Title"}},
            {"id": "c8", "ui": "click", "target": {"label": "Documents"}}, {"id": "s8", "ui": "snapshot"}]
    r = S(files, [], reqs)
    for k in ("s0", "s1", "s2", "s3", "s4", "s5", "s6", "s7", "s8"):
        s = r["res"][k]
        print(f"== shell {k}: path={(node(s, 'Path') or {}).get('value')!r} focused={[n.get('label') for n, _ in kl.walk(s['result']['root']) if n.get('focused')]} msg={(node(s, 'Message') or {}).get('value')!r} docs={docs(s)} title={(node(s, 'Title') or {}).get('value')!r}")
    for k in ("c0", "c1", "k2", "k3", "c3", "c4", "c5", "c6", "c7", "c8"):
        print("  ", k, json.dumps(r["res"][k])[:200])
    print("   log", [(l["command"], l["params"]) for l in r["log"] if l["command"] == "doc.open"])
    # roles/labels of every node in a full snapshot
    s = r["res"]["s8"]
    print("   all nodes:", [(n.get("role"), n.get("label")) for n, _ in kl.walk(s["result"]["root"])])


def sec_plain():
    src = "fn main() { let s = \"x\"; // c\n  struct S; 0x1f }\n"
    for p in ["a.txt", "a.rs", "a.rs.bak", "Makefile", "a.rs~"]:
        r = S({p: src}, [p], [SNAP])
        ed = node(r["res"]["snap"], "Editor")
        print("== plain", p, sorted({run["color"] for row in ed.get("rows", []) for run in row}))
    # syntax errors
    src = "fn ( { let = \"unterminated\n/* open comment\nstruct 9 }}} \u0000 \x7f\n"
    r = S({"err.rs": src}, ["err.rs"], [SNAP, {"id": "h", "command": "syntax.highlight", "params": {"doc": 1}}])
    ed = node(r["res"]["snap"], "Editor")
    kinds = ["?"] * len(src)
    for sp in r["res"]["h"]["result"]["spans"]:
        for q in range(sp["start"], sp["end"]):
            kinds[q] = sp["kind"]
    probs = []
    lines = src.split("\n")
    off = 0
    starts = []
    for ln in lines:
        starts.append(off); off += len(ln) + 1
    nonempty = [(s, ln) for s, ln in zip(starts, lines) if ln]
    for row, (s, ln) in zip(ed["rows"], nonempty):
        chars = [(c, run["color"]) for run in row for c in run["text"]]
        for i, (c, col) in enumerate(chars):
            if not c.isspace() and col != COL[kinds[s + i]]:
                probs.append((s + i, c, col, kinds[s + i]))
    print("== syntax errors file: rows", kl.rows_text(ed), "colour mismatches", probs)


def sec_wide():
    text = "éäx 中文字 😀👍🏽 ab‍cd ﬁ fi\tz\n"
    for col in range(0, len(text.split("\n")[0]) + 1):
        r = S({"w.txt": text}, ["w.txt"], [{"id": "c", "ui": "click", "target": {"label": "Editor"}, "at": {"row": 0, "col": col}},
                                           {"id": 1, "command": "cursor.get", "params": {"doc": 1}}])
        res = r["res"]["c"]
        head = r["res"][1]["result"]["head"] if res.get("ok") else None
        print(f"  col {col:2d} char {text[col]!r:10} -> head {head} {'' if head == col else 'DIFF'} {'' if res.get('ok') else res}")


def sec_logfile():
    files = {"a.rs": "x"}
    sd = kl.fresh("lf"); sp = os.path.join(sd, "s.jsonl"); open(sp, "w").write(kl.jl([SNAP]))
    root = kl.fresh("lfr"); kl.write_files(root, files)
    for lg in [sd, "/nonexistent/dir/log", os.path.join(sd, "ok.log")]:
        rc, out, err = kl.run([kl.BIN, "script", sp, "--root", root, "--log", lg, "a.rs"])
        print("== log", lg.replace(kl.WORK, "$WORK"), "rc", rc, "stdout", len(out), "stderr", err.decode()[:200].splitlines()[:1])
    for argv in [["script"], ["script", sp, sp], ["script", sp, "--memory", "--root", root], ["script", "/nonexistent.jsonl"],
                 ["script", sp, "--root", "/nonexistent"], ["script", sp, "--root", sp], ["--log", "x"], ["--memory"], ["--size", "300x300"],
                 ["script", sp, "--bogus"], ["--root"], ["script", sp, "--root", root, "--root", root], ["-h"], ["script", "-h"], ["script", sp, "--help"],
                 ["--script"], ["--script", "/nonexistent.jsonl"], ["--root", "/nonexistent"], ["--root", "/nonexistent", "--script", "/nonexistent.jsonl"],
                 ["script", sp, "--root", root, "missing.rs", "a.rs"], ["script", sp, "--root", root, "--", "a.rs"], ["script", "--root", root, sp]]:
        env = dict(os.environ); env.pop("DISPLAY", None); env.pop("WAYLAND_DISPLAY", None)
        p = subprocess.run(["timeout", "30", kl.BIN] + argv, capture_output=True, env=env)
        print("== argv", [a.replace(kl.WORK, "$WORK") for a in argv], "rc", p.returncode, "stdout", len(p.stdout),
              "stderr", [l[:160] for l in p.stderr.decode().replace(kl.WORK, "$WORK").splitlines()[:2]])


SECTIONS = {k[4:]: v for k, v in globals().items() if k.startswith("sec_")}
for s in sys.argv[1:]:
    print("######", s)
    SECTIONS[s]()
