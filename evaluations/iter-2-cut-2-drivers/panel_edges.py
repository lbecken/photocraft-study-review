"""Project panel (ui.project-panel), deterministic edge cases. usage: panel_edges.py <case>...
env: KILN_BIN, KILN_CLI, KILN_WORK. Prints one block per case."""
import json, os, sys, time, stat, subprocess
sys.path.insert(0, os.path.dirname(__file__))
import kl, k2
from k2 import Session, find, one, maybe, rows, tree_items, message, title

PROJ = {"role": "button", "label": "Project"}


def ck(cond, what):
    print(("  OK   " if cond else "  FAIL ") + what)
    return cond


def click(i, **t):
    return {"id": i, "ui": "click", "target": t}


def n_of(root, lab):
    return one(root, role="tree_item", label=lab)


def items(root):
    return [(n.get("label"), n.get("value"), n.get("toggled"), rows(n)) for n in tree_items(root)]


def ptree(root, path=None):
    p = {} if path is None else {"path": path}
    rc, out, err = kl.run([kl.CLI, "exec", "project.tree", json.dumps(p), "--root", root], t=120)
    return json.loads(out) if rc == 0 else json.loads(err)


def case_example():
    print("== example of ui-project.md")
    d = kl.fresh("pex")
    kl.write_files(d, {".gitignore": "*.log\n", "a.log": "x", "target/t": "t", "src/a.rs": "fn main() {}\n",
                       "dir with space/é.txt": "e", "b.txt": "b"})
    s = Session([{"id": "s0", "ui": "snapshot"}, click("c", **PROJ), {"id": "s1", "ui": "snapshot"},
                 click("d", role="tree_item", label="src"), {"id": "s2", "ui": "snapshot"},
                 click("o", role="tree_item", label="src/a.rs"), {"id": "s", "ui": "snapshot"}], root=d, name="pex")
    r0 = s.snap("s0")
    ck(one(r0, **PROJ).get("toggled") is False and rows(one(r0, **PROJ)) == ["Project"], "collapsed toggle, row 'Project'")
    ck(not find(r0, role="tree") and not tree_items(r0), "collapsed: no tree, no items")
    r1 = s.snap("s1")
    print("  after c:", [(a, c) for a, b, c, _ in items(r1)])
    ck([a for a, *_ in items(r1)] == [".gitignore", "b.txt", "dir with space", "src"], "root entries")
    ck(one(r1, role="tree", label="Project files") is not None, "tree Project files")
    r2 = s.snap("s2")
    print("  after d:", [(a, b, c) for a, b, c, _ in items(r2)])
    r = s.snap("s")
    print("  title:", title(r))
    print("  log:", s.logcmds())
    print("  rc", s.rc)


def case_empty():
    print("== empty root, and root with only ignored entries")
    for files in ({}, {".gitignore": "*\n", "x": "1"}, {"target/a": "1", ".git/HEAD": "x"}):
        d = kl.fresh("pempty")
        kl.write_files(d, files)
        s = Session([click("c", **PROJ), {"id": "s", "ui": "snapshot"}, click("c2", **PROJ), {"id": "s2", "ui": "snapshot"}],
                    root=d, name="pempty")
        r = s.snap("s")
        t = find(r, role="tree", label="Project files")
        print(" ", sorted(files), "rc", s.rc, "tree nodes", len(t), "items", items(r), "toggle", one(r, **PROJ).get("toggled"),
              "| after collapse toggle", one(s.snap("s2"), **PROJ).get("toggled"), "tree", len(find(s.snap("s2"), role="tree")))
        print("   log", s.logcmds())


NAMES = ["a", "a b", "a-", "a.rs", "ab", "é́x.txt", "‍", "x‍y.txt", "ﬁle.txt", "fi",
         "中文字.txt", "😀👍🏽.txt", "שלום.txt", "مرحبا.txt", "٣٣ ab.txt", "new\nline.txt", "tab\there", "Open", "Editor",
         ".hidden", " lead", "trail ", "UPPER", "upper", "#", "%41", "~", " nbsp", "w" * 240 + ".txt", "z̈"]


def case_names():
    print("== hostile names at the root (files), a directory 'a' with 'a/b', ignored entries")
    d = kl.fresh("pnames")
    files = {n: f"content of {n!r}\n" for n in NAMES if n != "a"}
    files["a/b"] = "ab\n"
    files["a/a b"] = "aab\n"
    files[".gitignore"] = "*.log\nign/\n"
    files["x.log"] = "l"
    files["ign/f"] = "f"
    files[".git/config"] = "c"
    files["target/x"] = "t"
    files["sub/target/y"] = "y"
    kl.write_files(d, files)
    tr = ptree(d)
    one_level = [e for e in tr["entries"] if "/" not in e["path"]]
    reqs = [click("c", **PROJ), {"id": "s", "ui": "snapshot"}, click("a", role="tree_item", label="a"),
            {"id": "sa", "ui": "snapshot"}]
    s = Session(reqs, root=d, name="pnames")
    r = s.snap("s")
    it = items(r)
    ck([a for a, *_ in it] == [e["path"] for e in one_level], f"root items = project.tree one level ({len(one_level)})")
    for (lab, val, tog, rw), e in zip(it, one_level):
        exp_tog = (False if e["kind"] == "dir" else None)
        bad = []
        if val != lab.split("/")[-1]:
            bad.append("value")
        if tog != exp_tog:
            bad.append(f"toggled {tog}")
        if rw != [val]:
            bad.append(f"rows {rw!r} offscreen={n_of(r, lab).get('offscreen')}")
        if bad:
            print("   ", repr(lab), e["kind"], bad)
    s3 = Session([click("c", **PROJ), {"id": "sc", "ui": "scroll", "target": {"role": "tree", "label": "Project files"},
                  "direction": "down", "steps": 5}, {"id": "s", "ui": "snapshot"}], root=d, name="pnscroll")
    for lab, val, tog, rw in items(s3.snap("s")):
        if rw != [val]:
            print("    scrolled 5:", repr(lab), "rows", rw, "offscreen", n_of(s3.snap("s"), lab).get("offscreen"))
    ita = items(s.snap("sa"))
    print("  after expanding a:", [a for a, *_ in ita][:6])
    # click every file, one session per file
    nok = 0
    for e in one_level:
        if e["kind"] != "file":
            continue
        for steps in range(0, 6):
            pre = [{"id": "sc", "ui": "scroll", "target": {"role": "tree", "label": "Project files"}, "direction": "down",
                    "steps": steps}] if steps else []
            s2 = Session([click("c", **PROJ)] + pre + [click("o", role="tree_item", label=e["path"]), {"id": "s", "ui": "snapshot"},
                          {"id": "l", "command": "doc.list"}, {"id": "t", "command": "doc.text", "params": {"doc": 1}}],
                         root=d, name="pnclick")
            if s2["o"]["ok"]:
                break
        if steps:
            print("   ", repr(e["path"]), "needed", steps, "scroll steps")
        r2 = s2.snap("s")
        ok = (s2["o"]["ok"] and s2["l"]["result"]["docs"][0]["path"] == e["path"]
              and (title(r2) or "").startswith(e["path"]) and s2["t"]["result"]["text"] == files[e["path"]]
              and message(r2) in ("", None)
              and ("doc.open", {"path": e["path"]}) in s2.logcmds())
        if not ok:
            print("   click FAIL", repr(e["path"]), s2["o"], title(r2), message(r2), s2["l"])
        nok += ok
    print(f"  clicked {sum(1 for e in one_level if e['kind']=='file')} files, {nok} opened with doc.open of their label")


def case_links():
    print("== links, special files, a directory link inside, unreadable dir (caps not dropped: uid 0)")
    outside = kl.fresh("poutside")
    kl.write_files(outside, {"o.txt": "outside\n", "od/f.txt": "of\n"})
    d = kl.fresh("plinks")
    kl.write_files(d, {"in.txt": "inside\n", "dir/f.txt": "f\n"})
    os.symlink(os.path.join(outside, "o.txt"), os.path.join(d, "l_out_file"))
    os.symlink(os.path.join(outside, "od"), os.path.join(d, "l_out_dir"))
    os.symlink("in.txt", os.path.join(d, "l_in_file"))
    os.symlink("dir", os.path.join(d, "l_in_dir"))
    os.symlink("nothing", os.path.join(d, "l_dangling"))
    os.symlink("..", os.path.join(d, "l_up"))
    os.mkfifo(os.path.join(d, "fifo"))
    tr = ptree(d)
    print("  project.tree:", [(e["path"], e["kind"]) for e in tr["entries"]])
    for e in tr["entries"]:
        if "/" in e["path"]:
            continue
        t0 = time.time()
        s = Session([click("c", **PROJ), {"id": "s0", "ui": "snapshot"}, click("o", role="tree_item", label=e["path"]),
                     {"id": "s", "ui": "snapshot"}, {"id": "l", "command": "doc.list"}], root=d, name="plclick", t=60)
        r = s.snap("s")
        n0 = one(s.snap("s0"), role="tree_item", label=e["path"])
        print(f"  {e['path']:12} {e['kind']:8} toggled={n0.get('toggled')} click->{s['o'].get('ok')} docs={[x['path'] for x in s["l"]["result"]["docs"]]} "
              f"msg={message(r)!r} expanded_items={[a for a, *_ in items(r) if '/' in a]} log={[c for c in s.logcmds() if c[0] in ('doc.open', 'project.tree')]} {time.time()-t0:.1f}s rc={s.rc}")


def case_collapse():
    print("== collapse forgets descendants; Project collapse forgets all; re-expand reads afresh")
    d = kl.fresh("pcoll")
    kl.write_files(d, {"a/b/c/f.txt": "1", "a/b/g.txt": "2", "a/h.txt": "3", "z.txt": "4"})
    reqs = [click("1", **PROJ), click("2", role="tree_item", label="a"), click("3", role="tree_item", label="a/b"),
            click("4", role="tree_item", label="a/b/c"), {"id": "s1", "ui": "snapshot"},
            click("5", role="tree_item", label="a"), {"id": "s2", "ui": "snapshot"},
            click("6", role="tree_item", label="a"), {"id": "s3", "ui": "snapshot"},
            click("7", role="tree_item", label="a/b"), click("8", **PROJ), {"id": "s4", "ui": "snapshot"},
            click("9", **PROJ), {"id": "s5", "ui": "snapshot"}, {"id": "fr", "ui": "frames", "count": 100}]
    s = Session(reqs, root=d, name="pcoll")
    for k in ("s1", "s2", "s3", "s4", "s5"):
        print(" ", k, [(a, c) for a, b, c, _ in items(s.snap(k))])
    print("  log:", s.logcmds())
    print("  idle frames:", s["fr"])


def case_variants():
    print("== click variants on entries; focus on an entry; keys after an entry click")
    d = kl.fresh("pvar")
    kl.write_files(d, {"a.txt": "a\n", "b.txt": "b\n", "c.txt": "c\n", "d.txt": "d\n", "dir/x.txt": "x", "dir2/y.txt": "y"})
    reqs = [click("p", **PROJ),
            {"id": "sec", "ui": "click", "target": {"role": "tree_item", "label": "a.txt"}, "button": "secondary"},
            {"id": "mod", "ui": "click", "target": {"role": "tree_item", "label": "b.txt"}, "modifiers": ["ctrl", "shift"]},
            {"id": "at", "ui": "click", "target": {"role": "tree_item", "label": "c.txt"}, "at": {"row": 0, "col": 2}},
            {"id": "atend", "ui": "click", "target": {"role": "tree_item", "label": "d.txt"}, "at": {"row": 0, "col": 5}},
            {"id": "atbad", "ui": "click", "target": {"role": "tree_item", "label": "d.txt"}, "at": {"row": 0, "col": 6}},
            {"id": "atrow", "ui": "click", "target": {"role": "tree_item", "label": "d.txt"}, "at": {"row": 1, "col": 0}},
            {"id": "dsec", "ui": "click", "target": {"role": "tree_item", "label": "dir"}, "button": "secondary"},
            {"id": "dat", "ui": "click", "target": {"role": "tree_item", "label": "dir2"}, "at": {"row": 0, "col": 0}},
            {"id": "s", "ui": "snapshot"},
            {"id": "foc", "ui": "focus", "target": {"role": "tree_item", "label": "a.txt"}},
            {"id": "foct", "ui": "focus", "target": {"role": "tree", "label": "Project files"}},
            {"id": "focp", "ui": "focus", "target": PROJ},
            click("o2", role="tree_item", label="a.txt"),
            {"id": "k", "ui": "key", "key": "Delete"},
            {"id": "ty", "ui": "type", "text": "Z"},
            {"id": "t", "command": "doc.text", "params": {"doc": 1}},
            {"id": "sc", "ui": "scroll", "target": {"role": "tree", "label": "Project files"}, "direction": "down"},
            {"id": "sc2", "ui": "scroll", "target": {"role": "tree_item", "label": "a.txt"}, "direction": "up", "steps": 3},
            {"id": "l", "command": "doc.list"}, {"id": "s2", "ui": "snapshot"}]
    s = Session(reqs, root=d, name="pvar")
    for k in ("sec", "mod", "at", "atend", "atbad", "atrow", "dsec", "dat", "foc", "foct", "focp", "o2", "k", "ty", "t", "sc", "sc2", "l"):
        print(f"  {k}: {json.dumps(s[k], ensure_ascii=False)[:220]}")
    print("  items:", [(a, c) for a, b, c, _ in items(s.snap("s"))])
    print("  focused:", [n.get("label") or n.get("role") for n in k2.nodes(s.snap("s2")) if n.get("focused")])
    print("  log:", [c for c in s.logcmds() if c[0] in ("doc.open", "project.tree", "doc.insert", "doc.delete")])


def case_ambig():
    print("== labels equal to shell labels and to tab labels (P6)")
    d = kl.fresh("pamb")
    kl.write_files(d, {"Open": "o", "Editor": "e", "Title": "t", "Message": "m", "Project": "p", "Tabs": "x", "Path": "y",
                       "Line": "l", "Documents": "d"})
    reqs = [click("p", **PROJ)]
    for lab in ("Open", "Editor", "Title", "Message", "Project", "Tabs", "Path", "Line", "Documents"):
        reqs.append({"id": "L" + lab, "ui": "click", "target": {"label": lab}})
    reqs += [click("o", role="tree_item", label="Editor"), {"id": "s", "ui": "snapshot"}]
    for lab in ("Editor", "Title", "Project", "Tabs", "Line", "Documents"):
        reqs.append({"id": "M" + lab, "ui": "click", "target": {"label": lab}})
    reqs.append({"id": "tab", "ui": "click", "target": {"label": "Editor", "role": "tab"}})
    s = Session(reqs, root=d, name="pamb")
    for r in s.resp:
        if r.get("id") not in ("s",):
            print(" ", r.get("id"), r.get("ok"), (r.get("error") or {}).get("kind"), (r.get("error") or {}).get("message", "")[:80])


def case_trunc():
    print("== truncated listings (root and nested), default limit 50,000")
    d = kl.fresh("ptrunc")
    os.makedirs(os.path.join(d, "a", "b"))
    for i in range(50001):
        open(os.path.join(d, "a", "b", f"f{i:05}"), "w").close()
    kl.write_files(d, {"0first.txt": "x", "z.txt": "z", "a/zz.txt": "zz"})
    t0 = time.time()
    s = Session([click("p", **PROJ), {"id": "s1", "ui": "snapshot"}, {"id": "t", "command": "project.tree"}], root=d, name="ptrunc")
    print(f"  root expand: {time.time()-t0:.1f}s rc {s.rc} truncated={s['t']['result']['truncated']} n={len(s['t']['result']['entries'])}")
    print("  shown:", [(a, b, c) for a, b, c, _ in items(s.snap("s1"))])
    t0 = time.time()
    s = Session([click("p", **PROJ), click("a", role="tree_item", label="a"), {"id": "s1", "ui": "snapshot"}],
                root=d, name="ptrunc2", t=900)
    print(f"  a expand: {time.time()-t0:.1f}s rc {s.rc}")
    print("  shown:", [(a, b, c) for a, b, c, _ in items(s.snap("s1"))])
    if "--nested" in sys.argv:
        t0 = time.time()
        s = Session([click("p", **PROJ), click("a", role="tree_item", label="a"), click("b", role="tree_item", label="a/b"),
                     {"id": "s1", "ui": "snapshot"}], root=d, name="ptrunc3", t=1800)
        print(f"  a/b expand: {time.time()-t0:.1f}s rc {s.rc} responses {[ (r.get('id'), r.get('ok'), r.get('result',{}).get('frames')) for r in s.resp[:3]]}")
        it = items(s.snap("s1"))
        print("  shown count", len(it), "first", [x[0] for x in it[:5]], "last 6", [(a, b) for a, b, c, _ in it[-6:]])


def case_deep():
    print("== deep tree: 130 nested directories, expand each level")
    d = kl.fresh("pdeep")
    p = d
    segs = []
    for i in range(130):
        segs.append("d")
        p = os.path.join(p, "d")
    os.makedirs(p)
    open(os.path.join(p, "leaf.txt"), "w").write("leaf\n")
    reqs = [click("p", **PROJ)]
    for i in range(1, 131):
        reqs.append({"id": f"sc{i}", "ui": "scroll", "target": {"role": "tree", "label": "Project files"}, "direction": "down", "steps": 100})
        reqs.append(click(f"e{i}", role="tree_item", label="/".join(["d"] * i)))
    reqs.append({"id": "scl", "ui": "scroll", "target": {"role": "tree", "label": "Project files"}, "direction": "down", "steps": 100})
    reqs.append({"id": "tq", "command": "project.tree", "params": {"path": "/".join(["d"] * 128)}})
    reqs.append({"id": "tq2", "command": "project.tree", "params": {"path": "/".join(["d"] * 127)}})
    reqs.append(click("leaf", role="tree_item", label="/".join(["d"] * 130) + "/leaf.txt"))
    reqs += [{"id": "s", "ui": "snapshot"}, {"id": "l", "command": "doc.list"}]
    s = Session(reqs, root=d, name="pdeep", t=900)
    bad = [(r.get("id"), (r.get("error") or {}).get("kind")) for r in s.resp if not r.get("ok")]
    print("  rc", s.rc, "refused:", bad[:5], "items shown", len(items(s.snap("s"))), "msg", message(s.snap("s")))
    print("  last items:", [(a[-12:], c) for a, b, c, _ in items(s.snap("s"))[-3:]])
    print("  doc.list", s["l"]["result"]["docs"][:1] and s["l"]["result"]["docs"][0]["path"][-20:])
    print("  tree at 128:", json.dumps(s["tq"])[:300]); print("  tree at 127:", json.dumps(s["tq2"])[:300])
    pt = [p for c, p in s.logcmds() if c == "project.tree"]
    print("  project.tree reads logged:", len(pt))


CASES = {k[5:]: v for k, v in globals().items() if k.startswith("case_")}
if __name__ == "__main__":
    for c in sys.argv[1:]:
        if c.startswith("--"):
            continue
        CASES[c]()
