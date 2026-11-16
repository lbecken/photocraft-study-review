"""Refusals through the panel and tabs: unreadable directory/file and a read-only file with the DAC
capabilities dropped (uid 0 without CAP_DAC_OVERRIDE / CAP_DAC_READ_SEARCH, via capsh); a file
replaced by a directory or by a link that leaves the root after it was opened (watcher thread, as
panel_race.py), then a UI save. env: KILN_BIN, KILN_CLI, KILN_WORK."""
import json, os, sys, threading, time, shutil, subprocess
sys.path.insert(0, os.path.dirname(__file__))
import kl
from k2 import Session, find, one, rows, tabs_view, message, title, tree_items

ED = {"role": "multiline_text_input", "label": "Editor"}


def tv(root):
    return [(t[0], t[1], "M" if t[2] else "", "D" if t[4] else "") for t in tabs_view(root)]


def nodac(with_locked):
    print("== DAC capabilities dropped; unreadable directory present:", with_locked)
    d = kl.fresh("nodac")
    kl.write_files(d, {"ro.txt": "read only\n", "noread.txt": "secret\n", "ok.txt": "ok\n"})
    os.makedirs(os.path.join(d, "locked"))
    if with_locked:
        open(os.path.join(d, "locked", "x.txt"), "w").write("x\n")
    os.chmod(os.path.join(d, "ro.txt"), 0o444)
    if with_locked:
        os.chmod(os.path.join(d, "locked"), 0o000)
    os.chmod(os.path.join(d, "noread.txt"), 0o000)
    reqs = [{"id": "p", "ui": "click", "target": {"role": "button", "label": "Project"}}, {"id": "s0", "ui": "snapshot"},
            {"id": "lk", "ui": "click", "target": {"role": "tree_item", "label": "locked"}}, {"id": "s1", "ui": "snapshot"},
            {"id": "nr", "ui": "click", "target": {"role": "tree_item", "label": "noread.txt"}}, {"id": "s2", "ui": "snapshot"},
            {"id": "ro", "ui": "click", "target": {"role": "tree_item", "label": "ro.txt"}},
            {"id": "f", "ui": "focus", "target": ED}, {"id": "t", "ui": "type", "text": "Z"},
            {"id": "sv", "ui": "key", "key": "S", "modifiers": ["ctrl"]}, {"id": "s3", "ui": "snapshot"},
            {"id": "cl", "ui": "click", "target": {"role": "button", "label": "Close ro.txt"}}, {"id": "s4", "ui": "snapshot"},
            {"id": "ok", "ui": "click", "target": {"role": "tree_item", "label": "ok.txt"}}, {"id": "s5", "ui": "snapshot"},
            {"id": "l", "command": "doc.list"}]
    sdir = kl.fresh("nodac-s")
    sp = os.path.join(sdir, "s.jsonl")
    open(sp, "w").write(kl.jl(reqs))
    logp = os.path.join(sdir, "log.jsonl")
    cmd = f"exec '{kl.BIN}' script '{sp}' --root '{d}' --log '{logp}'"
    p = subprocess.run(["timeout", "120", "capsh", "--drop=cap_dac_override,cap_dac_read_search", "--", "-c", cmd], capture_output=True)
    by = {}
    for l in p.stdout.decode().splitlines():
        r = json.loads(l)
        by.setdefault(r["id"], r)
    for k, lab in (("s0", "after expand"), ("s1", "click locked"), ("s2", "click noread.txt"), ("s3", "ro.txt typed + ctrl+S"),
                   ("s4", "Close ro.txt"), ("s5", "open ok.txt")):
        root = by[k]["result"]["root"]
        print(f"  {lab}: items={[(n.get('label'), n.get('toggled')) for n in tree_items(root)]} tabs={tv(root)} msg={message(root)!r}")
    print("  rc", p.returncode, "ro.txt on disk:", open(os.path.join(d, "ro.txt"), "rb").read(), "doc.list", by["l"]["result"])
    print("  log:", [l.strip() for l in open(logp) if '"project.tree"' in l or '"doc.open"' in l or '"doc.save"' in l or '"doc.close"' in l])
    os.chmod(os.path.join(d, "locked"), 0o755)
    os.chmod(os.path.join(d, "noread.txt"), 0o644)


def race(name, mutate):
    d = kl.fresh("rsave-" + name)
    outside = kl.fresh("rsave-out")
    kl.write_files(outside, {"o.txt": "outside\n"})
    kl.write_files(d, {"sentinel": "0", "a.txt": "a\n", "in.txt": "in\n"})
    sp = os.path.join(d, "sentinel")
    reqs = [{"id": "so", "command": "doc.open", "params": {"path": "sentinel"}}, {"id": "f0", "ui": "frames", "count": 1},
            {"id": "p", "ui": "click", "target": {"role": "button", "label": "Project"}},
            {"id": "o", "ui": "click", "target": {"role": "tree_item", "label": "a.txt"}},
            {"id": "f", "ui": "focus", "target": ED}, {"id": "t", "ui": "type", "text": "Z"},
            {"id": "si", "command": "doc.insert", "params": {"doc": 1, "offset": 0, "text": "1"}},
            {"id": "ss", "command": "doc.save", "params": {"doc": 1}}] + \
        [{"id": f"w{i}", "ui": "frames", "count": 600} for i in range(10)] + \
        [{"id": "f2", "ui": "focus", "target": ED}, {"id": "sv", "ui": "key", "key": "S", "modifiers": ["ctrl"]}, {"id": "s", "ui": "snapshot"},
         {"id": "cl", "ui": "click", "target": {"role": "button", "label": "Close a.txt"}}, {"id": "s2", "ui": "snapshot"},
         {"id": "l", "command": "doc.list"}]
    done = {}

    def watch():
        t0 = time.time()
        while time.time() - t0 < 120:
            if open(sp).read() != "0":
                mutate(d, outside)
                done["at"] = time.time() - t0
                return
            time.sleep(0.0005)
    th = threading.Thread(target=watch)
    th.start()
    s = Session(reqs, root=d, name="rsave-" + name, t=300)
    th.join()
    print(f"== {name}: mutated after {done.get('at')}s; save -> Message {message(s.snap('s'))!r}; tabs {tv(s.snap('s'))}; "
          f"after Close: tabs {tv(s.snap('s2'))} msg {message(s.snap('s2'))!r}")
    print("   outside o.txt:", open(os.path.join(outside, "o.txt"), "rb").read(), "| in.txt:", open(os.path.join(d, "in.txt"), "rb").read(),
          "| a.txt is", "link" if os.path.islink(os.path.join(d, "a.txt")) else ("dir" if os.path.isdir(os.path.join(d, "a.txt")) else "file"),
          "| doc.save logged for doc 2:", any(c == "doc.save" and p.get("doc") == 2 for c, p in s.logcmds()))


def to_dir(d, o):
    os.remove(os.path.join(d, "a.txt"))
    os.makedirs(os.path.join(d, "a.txt"))


def to_outside_link(d, o):
    os.remove(os.path.join(d, "a.txt"))
    os.symlink(os.path.join(o, "o.txt"), os.path.join(d, "a.txt"))


def to_inside_link(d, o):
    os.remove(os.path.join(d, "a.txt"))
    os.symlink("in.txt", os.path.join(d, "a.txt"))


def removed(d, o):
    os.remove(os.path.join(d, "a.txt"))


nodac(True)
nodac(False)
if "--nodac-only" in sys.argv:
    sys.exit(0)
race("replaced-by-dir", to_dir)
race("replaced-by-outside-link", to_outside_link)
race("replaced-by-inside-link", to_inside_link)
race("removed", removed)
