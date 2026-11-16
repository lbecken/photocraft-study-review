"""ui.tabs deterministic edge cases. usage: tabs_edges.py <case>...   env: KILN_BIN, KILN_CLI, KILN_WORK."""
import json, os, sys, time
sys.path.insert(0, os.path.dirname(__file__))
import kl, k2
from k2 import Session, find, one, rows, tabs_view, status, message, title, editor, nodes

ED = {"role": "multiline_text_input", "label": "Editor"}


def click(i, **t):
    return {"id": i, "ui": "click", "target": t}


def tv(root):
    return [(t[0], t[1], "M" if t[2] else "", "C" if t[3] else "", "D" if t[4] else "") for t in tabs_view(root)]


def show(s, ids):
    for i in ids:
        r = s[i]
        if "root" in r.get("result", {}):
            root = r["result"]["root"]
            print(f"  {i}: tabs={tv(root)} msg={message(root)!r} title={title(root)!r}")
        else:
            print(f"  {i}: {json.dumps(r, ensure_ascii=False)[:230]}")


def fx(files):
    d = kl.fresh("tedge")
    kl.write_files(d, files)
    return d


F = {"a.txt": "a\n", "b.txt": "b\n", "c.txt": "c\n"}


def case_zero():
    print("== 0 documents; then 1, 2; close all")
    d = fx(F)
    s = Session([{"id": "s0", "ui": "snapshot"}, {"id": "o1", "command": "doc.open", "params": {"path": "a.txt"}},
                 {"id": "f", "ui": "frames", "count": 1}, {"id": "s1", "ui": "snapshot"},
                 {"id": "o2", "command": "doc.open", "params": {"path": "b.txt"}}, {"id": "f2", "ui": "frames", "count": 1},
                 {"id": "s2", "ui": "snapshot"}, click("c1", role="button", label="Close b.txt"), {"id": "s3", "ui": "snapshot"},
                 click("c2", role="button", label="Close a.txt"), {"id": "s4", "ui": "snapshot"}], root=d, name="tzero")
    for i in ("s0", "s1", "s2", "s3", "s4"):
        root = s.snap(i)
        print(f"  {i}: tab_list={len(find(root, role='tab_list'))} tabs={tv(root)} title={title(root)!r} editor={editor(root) is not None} "
              f"status={[k for k, v in status(root).items() if v]}")
    print("  log:", s.logcmds())


def case_twice():
    print("== the same file opened twice (panel, Path/Open, control): one tab, it becomes current")
    d = fx(F)
    s = Session([click("p", role="button", label="Project"), click("1", role="tree_item", label="a.txt"),
                 click("2", role="tree_item", label="b.txt"), click("3", role="tree_item", label="a.txt"), {"id": "s1", "ui": "snapshot"},
                 {"id": "fp", "ui": "focus", "target": {"label": "Path"}}, {"id": "tp", "ui": "type", "text": "b.txt"},
                 click("op", role="button", label="Open"), {"id": "s2", "ui": "snapshot"},
                 {"id": "co", "command": "doc.open", "params": {"path": "a.txt"}}, {"id": "f", "ui": "frames", "count": 1},
                 {"id": "s3", "ui": "snapshot"}, {"id": "l", "command": "doc.list"}], root=d, name="ttwice")
    show(s, ["s1", "s2", "co", "s3", "l"])
    print("  mut log:", s.mut_log())


def case_refused():
    print("== refused close, then switch, edit, save; Discard lifecycle")
    d = fx(F)
    reqs = [click("p", role="button", label="Project"), click("o1", role="tree_item", label="a.txt"),
            click("o2", role="tree_item", label="b.txt"),
            {"id": "e1", "command": "doc.insert", "params": {"doc": 1, "offset": 0, "text": "X"}}, {"id": "f1", "ui": "frames", "count": 1},
            {"id": "s1", "ui": "snapshot"},
            click("c1", role="button", label="Close a.txt"), {"id": "s2", "ui": "snapshot"},
            click("sw", role="tab", label="a.txt"), {"id": "s3", "ui": "snapshot"},
            {"id": "fo", "ui": "focus", "target": ED}, {"id": "ty", "ui": "type", "text": "Y"}, {"id": "s4", "ui": "snapshot"},
            {"id": "sv", "ui": "key", "key": "S", "modifiers": ["ctrl"]}, {"id": "s5", "ui": "snapshot"},
            {"id": "ty2", "ui": "type", "text": "Z"}, {"id": "s6", "ui": "snapshot"},
            click("c2", role="button", label="Close a.txt"), {"id": "s7", "ui": "snapshot"},
            {"id": "fo2", "ui": "focus", "target": ED}, {"id": "un", "ui": "key", "key": "Z", "modifiers": ["ctrl"]}, {"id": "s8", "ui": "snapshot"},
            {"id": "ty3", "ui": "type", "text": "W"}, {"id": "s9", "ui": "snapshot"},
            click("c3", role="button", label="Close a.txt"), {"id": "s10", "ui": "snapshot"},
            click("dc", role="button", label="Discard a.txt"), {"id": "s11", "ui": "snapshot"},
            {"id": "l", "command": "doc.list"}]
    s = Session(reqs, root=d, name="trefused")
    show(s, ["s1", "c1", "s2", "s3", "s4", "s5", "s6", "s7", "s8", "s9", "s10", "dc", "s11", "l"])
    print("  mut log:", s.mut_log())
    print("  a.txt on disk:", open(os.path.join(d, "a.txt")).read().encode())


def case_control_behind():
    print("== control requests behind the UI's back: refused control close, control discard, control save, control undo")
    d = fx(F)
    reqs = [{"id": "o1", "command": "doc.open", "params": {"path": "a.txt"}}, {"id": "o2", "command": "doc.open", "params": {"path": "b.txt"}},
            {"id": "e1", "command": "doc.insert", "params": {"doc": 1, "offset": 0, "text": "X"}},
            {"id": "e2", "command": "doc.insert", "params": {"doc": 2, "offset": 0, "text": "X"}},
            {"id": "f", "ui": "frames", "count": 1}, {"id": "s1", "ui": "snapshot"},
            {"id": "cc", "command": "doc.close", "params": {"doc": 1}}, {"id": "f2", "ui": "frames", "count": 1}, {"id": "s2", "ui": "snapshot"},
            click("c1", role="button", label="Close a.txt"), click("c2", role="button", label="Close b.txt"), {"id": "s3", "ui": "snapshot"},
            {"id": "un", "command": "doc.undo", "params": {"doc": 1}}, {"id": "f3", "ui": "frames", "count": 1}, {"id": "s4", "ui": "snapshot"},
            {"id": "re", "command": "doc.redo", "params": {"doc": 1}}, {"id": "f4", "ui": "frames", "count": 1}, {"id": "s5", "ui": "snapshot"},
            {"id": "sv", "command": "doc.save", "params": {"doc": 2}}, {"id": "f5", "ui": "frames", "count": 1}, {"id": "s6", "ui": "snapshot"},
            click("c3", role="button", label="Close a.txt"), {"id": "s7", "ui": "snapshot"},
            {"id": "cd", "command": "doc.close", "params": {"doc": 1, "discard": True}}, {"id": "f6", "ui": "frames", "count": 1},
            {"id": "s8", "ui": "snapshot"},
            {"id": "o3", "command": "doc.open", "params": {"path": "c.txt"}}, {"id": "f7", "ui": "frames", "count": 1}, {"id": "s9", "ui": "snapshot"},
            {"id": "e3", "command": "doc.insert", "params": {"doc": 3, "offset": 0, "text": "X"}}, {"id": "f8", "ui": "frames", "count": 1},
            click("c4", role="button", label="Close c.txt"), {"id": "s10", "ui": "snapshot"},
            {"id": "o4", "command": "doc.open", "params": {"path": "a.txt"}}, {"id": "f9", "ui": "frames", "count": 1}, {"id": "s11", "ui": "snapshot"},
            ]
    s = Session(reqs, root=d, name="tbehind")
    show(s, ["s1", "cc", "s2", "s3", "s4", "s5", "s6", "s7", "s8", "s9", "s10", "o4", "s11"])


def case_variants():
    print("== click variants on tabs / Close / Discard / Documents entries; focus; current-tab click")
    d = fx(F)
    reqs = [{"id": "o1", "command": "doc.open", "params": {"path": "a.txt"}}, {"id": "o2", "command": "doc.open", "params": {"path": "b.txt"}},
            {"id": "o3", "command": "doc.open", "params": {"path": "c.txt"}}, {"id": "f", "ui": "frames", "count": 1},
            {"id": "t1", "ui": "click", "target": {"role": "tab", "label": "a.txt"}, "button": "secondary"}, {"id": "s1", "ui": "snapshot"},
            {"id": "t2", "ui": "click", "target": {"role": "tab", "label": "b.txt"}, "modifiers": ["alt", "shift"]}, {"id": "s2", "ui": "snapshot"},
            {"id": "t3", "ui": "click", "target": {"role": "tab", "label": "c.txt"}, "at": {"row": 0, "col": 3}}, {"id": "s3", "ui": "snapshot"},
            {"id": "t4", "ui": "click", "target": {"role": "label", "value": "a.txt"}, "button": "secondary"}, {"id": "s4", "ui": "snapshot"},
            {"id": "t5", "ui": "click", "target": {"role": "label", "value": "b.txt"}, "at": {"row": 0, "col": 0}}, {"id": "s5", "ui": "snapshot"},
            {"id": "fc", "ui": "focus", "target": {"role": "tab", "label": "a.txt"}},
            {"id": "fx", "ui": "focus", "target": {"role": "button", "label": "Close a.txt"}},
            {"id": "c1", "ui": "click", "target": {"role": "button", "label": "Close c.txt"}, "button": "secondary"}, {"id": "s6", "ui": "snapshot"},
            {"id": "c1at", "ui": "click", "target": {"role": "button", "label": "Close a.txt"}, "at": {"row": 0, "col": 0}},
            {"id": "e", "command": "doc.insert", "params": {"doc": 1, "offset": 0, "text": "X"}}, {"id": "f2", "ui": "frames", "count": 1},
            {"id": "c2", "ui": "click", "target": {"role": "button", "label": "Close a.txt"}, "modifiers": ["ctrl"]}, {"id": "s7", "ui": "snapshot"},
            {"id": "d1", "ui": "click", "target": {"role": "button", "label": "Discard a.txt"}, "at": {"row": 0, "col": 2}}, {"id": "s8", "ui": "snapshot"},
            {"id": "lg0", "command": "doc.list"}, {"id": "f3", "ui": "frames", "count": 5},
            {"id": "cur", "ui": "click", "target": {"role": "tab", "label": "b.txt"}},
            {"id": "f4", "ui": "frames", "count": 5},
            {"id": "s9", "ui": "snapshot"}]
    s = Session(reqs, root=d, name="tvar")
    show(s, ["t1", "s1", "t2", "s2", "t3", "s3", "t4", "s4", "t5", "s5", "fc", "fx", "c1", "s6", "c1at", "c2", "s7", "d1", "s8", "cur", "s9"])
    lg = s.logcmds()
    i = max(k for k, (c, p) in enumerate(lg) if c == "doc.list")
    print("  mut log:", s.mut_log())
    print("  log tail (after the last doc.list control):", lg[-8:])


def case_stale():
    print("== a click right after a control request that moves the layout (no frame between)")
    d = fx(F)
    reqs = [{"id": "o1", "command": "doc.open", "params": {"path": "a.txt"}}, {"id": "f", "ui": "frames", "count": 1},
            {"id": "o2", "command": "doc.open", "params": {"path": "b.txt"}},
            {"id": "t", "ui": "click", "target": {"role": "tab", "label": "a.txt"}}, {"id": "s1", "ui": "snapshot"},
            {"id": "c", "command": "doc.close", "params": {"doc": 1}},
            {"id": "t2", "ui": "click", "target": {"role": "tab", "label": "a.txt"}}, {"id": "s2", "ui": "snapshot"},
            {"id": "l", "command": "doc.list"}]
    s = Session(reqs, root=d, name="tstale")
    show(s, ["t", "s1", "c", "t2", "s2", "l"])
    print("  log:", s.logcmds())
    d = fx(F)
    reqs = [{"id": "o1", "command": "doc.open", "params": {"path": "a.txt"}},
            {"id": "p", "ui": "click", "target": {"role": "button", "label": "Project"}}, {"id": "s", "ui": "snapshot"},
            {"id": "p2", "ui": "click", "target": {"role": "button", "label": "Project"}}, {"id": "s2", "ui": "snapshot"}]
    s = Session(reqs, root=d, name="tstale2")
    reqs_f = reqs[:1] + [{"id": "f", "ui": "frames", "count": 1}] + reqs[1:3]
    sf = Session(reqs_f, root=d, name="tstale3")
    print("  with one frame between the open and the click:", sf["p"], "toggled:", one(sf.snap("s"), role="button", label="Project").get("toggled"))
    print("  Project click right after a control doc.open:", s["p"], "toggled:", one(s.snap("s"), role="button", label="Project").get("toggled"),
          "| second click toggled:", one(s.snap("s2"), role="button", label="Project").get("toggled"))
    print("  log:", s.logcmds())


def case_names():
    print("== paths with a line break, very wide, right-to-left, combining, emoji, prefix names")
    names = ["new\nline.txt", "w" * 240 + ".txt", "שלום.txt", "é́.txt", "😀👍🏽.txt", "a", "a b", "ab", "x‍y.txt"]
    d = fx({n: "t\n" for n in names})
    reqs = []
    for k, n in enumerate(names):
        reqs.append({"id": f"o{k}", "command": "doc.open", "params": {"path": n}})
    reqs += [{"id": "e", "command": "doc.insert", "params": {"doc": 2, "offset": 0, "text": "X"}}, {"id": "f", "ui": "frames", "count": 2},
             {"id": "s", "ui": "snapshot"}]
    for k, n in enumerate(names):
        reqs += [{"id": f"c{k}", "ui": "click", "target": {"role": "tab", "label": n}}, {"id": f"s{k}", "ui": "snapshot"}]
    s = Session(reqs, root=d, name="tnames")
    root = s.snap("s")
    for t in tabs_view(root):
        n = t[5]
        print(f"  {t[0][:30]!r}: rows={[r[:30] for r in rows(n)]} offscreen={n.get('offscreen')} modified={t[2]}")
    mods = [n for n in nodes(root) if n.get("value") == "modified"]
    print("  modified mark rows:", [rows(n) for n in mods])
    for k, n in enumerate(names):
        ok = s[f"c{k}"].get("ok")
        cur = [t[0] for t in tabs_view(s.snap(f"s{k}")) if t[1]]
        print(f"  click {n[:20]!r}: {ok} {(s[f'c{k}'].get('error') or {}).get('kind')} current={[c[:20] for c in cur]}")


def case_many():
    n = int(os.environ.get("TABS_N", "300"))
    print(f"== {n} open documents: reach every tab by scrolling Tabs; timing of 600 idle frames")
    d = kl.fresh("tmany")
    for i in range(n):
        open(os.path.join(d, f"f{i:03}.txt"), "w").write(f"{i}\n")
    reqs = [{"id": f"o{i}", "command": "doc.open", "params": {"path": f"f{i:03}.txt"}} for i in range(n)]
    reqs += [{"id": "f", "ui": "frames", "count": 2}, {"id": "s0", "ui": "snapshot"}]
    for k in range(1, 40):
        reqs += [{"id": f"sc{k}", "ui": "scroll", "target": {"role": "tab_list", "label": "Tabs"}, "direction": "down"},
                 {"id": f"s{k}", "ui": "snapshot"}]
    s = Session(reqs, root=d, name="tmany", t=1200)
    seen = {}
    for k in range(0, 40):
        root = s.snap(f"s{k}")
        vis = [t[0] for t in tabs_view(root) if not t[5].get("offscreen")]
        for v in vis:
            seen.setdefault(v, k)
        if k in (0, 1, 2, 39):
            print(f"  after {k} steps: visible {len(vis)} first {vis[:1]} last {vis[-1:]}  status painted: {[bool(rows(v)) for v in status(root).values() if isinstance(v, dict)]}")
    print(f"  tabs ever visible: {len(seen)} of {n}; never: {[f'f{i:03}.txt' for i in range(n) if f'f{i:03}.txt' not in seen][:5]}")
    # click a sample of tabs after scrolling to where they are visible
    sample = [f"f{i:03}.txt" for i in range(0, n, max(1, n // 12))] + [f"f{n-1:03}.txt"]
    bad = 0
    for lab in sample:
        k = seen.get(lab)
        if k is None:
            continue
        reqs2 = reqs[:n] + [{"id": "f", "ui": "frames", "count": 2}]
        if k:
            reqs2.append({"id": "sc", "ui": "scroll", "target": {"role": "tab_list", "label": "Tabs"}, "direction": "down", "steps": k})
        reqs2 += [{"id": "c", "ui": "click", "target": {"role": "tab", "label": lab}}, {"id": "s", "ui": "snapshot"}]
        s2 = Session(reqs2, root=d, name="tmanyc", t=600)
        cur = [t[0] for t in tabs_view(s2.snap("s")) if t[1]]
        if not (s2["c"].get("ok") and cur == [lab] and (title(s2.snap("s")) or "").startswith(lab)):
            bad += 1
            print("   click failed", lab, s2["c"], cur)
    print(f"  clicked {len(sample)} sampled tabs after scrolling, {bad} failed")
    # unscrolled click of an offscreen tab
    s3 = Session(reqs[:n] + [{"id": "f", "ui": "frames", "count": 2}, {"id": "c", "ui": "click", "target": {"role": "tab", "label": f"f{n-1:03}.txt"}}],
                 root=d, name="tmanyo")
    print("  click on the last tab without scrolling:", s3["c"])
    t0 = time.time()
    s4 = Session(reqs[:n] + [{"id": f"i{k}", "ui": "frames", "count": 600} for k in range(1)], root=d, name="tmanyt", t=600)
    print(f"  {n} opens + 600 idle frames: {time.time()-t0:.2f}s rc {s4.rc}")


CASES = {k[5:]: v for k, v in globals().items() if k.startswith("case_")}
if __name__ == "__main__":
    for c in sys.argv[1:]:
        CASES[c]()
