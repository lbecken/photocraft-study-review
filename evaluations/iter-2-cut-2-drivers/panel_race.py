"""Project panel: the tree changes on disk between an expand and a click (P4).

The script expands, then saves a sentinel document (control requests) and runs 6,000 idle frames;
a watcher thread changes the tree as soon as the sentinel's content changes on disk, i.e. after
the expand and before the click. A `project.tree` control request after the frames proves the
change happened before the click. env: KILN_BIN, KILN_CLI, KILN_WORK.
"""
import json, os, sys, threading, time, shutil
sys.path.insert(0, os.path.dirname(__file__))
import kl
from k2 import Session, find, one, rows, tree_items, message, title

PROJ = {"role": "button", "label": "Project"}


def click(i, **t):
    return {"id": i, "ui": "click", "target": t}


def items(root):
    return [(n.get("label"), n.get("toggled")) for n in tree_items(root)]


def race(name, files, pre, mutate, post):
    d = kl.fresh("prace-" + name)
    kl.write_files(d, files)
    kl.write_files(d, {"sentinel": "0"})
    sp = os.path.join(d, "sentinel")
    reqs = [{"id": "so", "command": "doc.open", "params": {"path": "sentinel"}}, {"id": "f1", "ui": "frames", "count": 1}] + pre + [
        {"id": "sb", "ui": "snapshot"},
        {"id": "si", "command": "doc.insert", "params": {"doc": 1, "offset": 0, "text": "1"}},
        {"id": "ss", "command": "doc.save", "params": {"doc": 1}}] + \
        [{"id": f"w{i}", "ui": "frames", "count": 600} for i in range(10)] + \
        [{"id": "tq", "command": "project.tree"}] + post + [{"id": "sa", "ui": "snapshot"}, {"id": "dl", "command": "doc.list"}]
    done = {}

    def watch():
        t0 = time.time()
        while time.time() - t0 < 120:
            try:
                if open(sp).read() != "0":
                    mutate(d)
                    done["at"] = time.time() - t0
                    return
            except FileNotFoundError:
                pass
            time.sleep(0.0005)

    th = threading.Thread(target=watch)
    th.start()
    s = Session(reqs, root=d, name="prace-" + name, t=300)
    th.join()
    print(f"== {name}: mutated after {done.get('at', 'NEVER')}")
    print("   before:", items(s.snap("sb")))
    print("   project.tree after frames:", [(e["path"], e["kind"]) for e in s["tq"]["result"]["entries"]])
    for r in post:
        print("  ", r["id"], json.dumps(s[r["id"]], ensure_ascii=False)[:200])
        if r.get("ui") == "click":
            pass
    print("   after:", items(s.snap("sa")), "| Message:", repr(message(s.snap("sa"))), "| Title:", title(s.snap("sa")))
    print("   docs:", [x["path"] for x in s["dl"]["result"]["docs"]])
    print("   log (tree/open):", [c for c in s.logcmds() if c[0] in ("project.tree", "doc.open")])
    return s


def rm_file(rel):
    return lambda d: os.remove(os.path.join(d, rel))


def dir_to_file(rel):
    def f(d):
        shutil.rmtree(os.path.join(d, rel))
        open(os.path.join(d, rel), "w").write("now a file\n")
    return f


def file_to_dir(rel):
    def f(d):
        os.remove(os.path.join(d, rel))
        os.makedirs(os.path.join(d, rel, "inner"))
    return f


def file_to_outside_link(rel, target):
    def f(d):
        os.remove(os.path.join(d, rel))
        os.symlink(target, os.path.join(d, rel))
    return f


def add_file(rel):
    return lambda d: open(os.path.join(d, rel), "w").write("new\n")


if __name__ == "__main__":
    outside = kl.fresh("prace-outside")
    kl.write_files(outside, {"secret.txt": "secret\n"})
    race("vanish", {"v.txt": "v\n", "k.txt": "k\n"}, [click("p", **PROJ)], rm_file("v.txt"),
         [click("c", role="tree_item", label="v.txt")])
    race("dir-to-file-expand", {"dd/x.txt": "x\n"}, [click("p", **PROJ)], dir_to_file("dd"),
         [click("c", role="tree_item", label="dd")])
    race("dir-to-file-expanded", {"dd/x.txt": "x\n"}, [click("p", **PROJ), click("e", role="tree_item", label="dd")],
         dir_to_file("dd"), [click("c", role="tree_item", label="dd/x.txt"), click("c2", role="tree_item", label="dd"),
                             click("c3", role="tree_item", label="dd"), {"id": "s3", "ui": "snapshot"}])
    race("file-to-dir", {"ff": "f\n"}, [click("p", **PROJ)], file_to_dir("ff"),
         [click("c", role="tree_item", label="ff")])
    race("file-to-outside-link", {"ff.txt": "f\n"}, [click("p", **PROJ)],
         file_to_outside_link("ff.txt", os.path.join(outside, "secret.txt")), [click("c", role="tree_item", label="ff.txt")])
    race("appear", {"a.txt": "a\n"}, [click("p", **PROJ)], add_file("b.txt"),
         [{"id": "s1", "ui": "snapshot"}, click("c", **PROJ), click("c2", **PROJ)])
    race("vanish-dir-expanded", {"dd/x.txt": "x\n"}, [click("p", **PROJ)], lambda d: shutil.rmtree(os.path.join(d, "dd")),
         [click("c", role="tree_item", label="dd")])
