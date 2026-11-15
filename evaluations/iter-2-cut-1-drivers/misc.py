"""Miscellaneous checks: Path selection field, secondary click in the editor, the scroll bar node,
display variables in script mode, several start paths in the window vs headless.
env: KILN_BIN, KILN_CLI, KILN_WORK, XLIBS"""
import json, os, subprocess, sys
sys.path.insert(0, os.path.dirname(__file__))
import kl


def res_of(out):
    return {r["id"]: r for r in kl.parse(out)}


# 1 Path: selection field when focused / after typing
root = kl.fresh("m1"); kl.write_files(root, {"a.rs": "fn a() {}\nlet b = 2;\n"})
reqs = [{"id": "f", "ui": "focus", "target": {"label": "Path"}}, {"id": "s1", "ui": "snapshot"}, {"id": "t", "ui": "type", "text": "abc"},
        {"id": "s2", "ui": "snapshot"}, {"id": "l", "ui": "key", "key": "ArrowLeft", "modifiers": ["shift"]}, {"id": "s3", "ui": "snapshot"}]
rc, out, err, _ = kl.kscript(reqs, root=root, paths=["a.rs"])
r = res_of(out)
for k in ("s1", "s2", "s3"):
    n = kl.one(r[k], label="Path")
    print("== Path", k, {x: n.get(x) for x in ("value", "selection", "focused")})

# 2 secondary click in the editor; click with ctrl; click with alt
for name, extra in [("secondary", {"button": "secondary"}), ("ctrl", {"modifiers": ["ctrl"]}), ("alt", {"modifiers": ["alt"]})]:
    root = kl.fresh("m2"); kl.write_files(root, {"a.rs": "fn a() {}\nlet b = 2;\n"})
    c = {"id": "c", "ui": "click", "target": {"label": "Editor"}, "at": {"row": 1, "col": 4}}
    c.update(extra)
    reqs = [c, {"id": "s", "ui": "snapshot"}, {"id": 1, "command": "cursor.get", "params": {"doc": 1}}]
    rc, out, err, _ = kl.kscript(reqs, root=root, paths=["a.rs"], log=root + ".log")
    r = res_of(out)
    log = [json.loads(l) for l in open(root + ".log")]
    ed = kl.one(r["s"], label="Editor")
    print("== editor click", name, "resp", r["c"].get("result") or r["c"].get("error"), "cursor", (r[1]["result"]["anchor"], r[1]["result"]["head"]),
          "focused", ed.get("focused"), "mutating", [(l["command"], l["params"]) for l in log if l["command"] in ("cursor.set", "cursor.move")])

# 3 the scroll bar: one overflowing area (Documents), click the bar
files = {f"f{i:03d}.txt": "x" for i in range(40)}
root = kl.fresh("m3"); kl.write_files(root, files)
reqs = [{"id": i, "command": "doc.open", "params": {"path": f"f{i:03d}.txt"}} for i in range(40)]
reqs += [{"id": "fr", "ui": "frames", "count": 3}, {"id": "s0", "ui": "snapshot"},
         {"id": "cb", "ui": "click", "target": {"role": "scroll_bar"}}, {"id": "s1", "ui": "snapshot"}]
rc, out, err, _ = kl.kscript(reqs, root=root, log=root + ".log")
r = res_of(out)


def vis(s):
    d = kl.one(s, label="Documents")
    v = [c.get("value") for c in d["children"] if not c.get("offscreen")]
    return (v[0], v[-1], len(v))


print("== scroll bar nodes", len(kl.find(r["s0"], role="scroll_bar")), "click on it:", r["cb"].get("result") or r["cb"].get("error"), "visible before", vis(r["s0"]), "after", vis(r["s1"]))
for n in kl.find(r["s0"], role="scroll_bar"):
    print("   bar node", {k: v for k, v in n.items() if k != "children"})

# 4 display variables do not matter in script mode
root = kl.fresh("m4"); kl.write_files(root, {"a.rs": "fn a() {}\n"})
sp = os.path.join(kl.fresh("m4s"), "s.jsonl"); open(sp, "w").write(kl.jl([{"id": "s", "ui": "snapshot"}]))
outs = []
for envmod in [{}, {"DISPLAY": ":0"}, {"DISPLAY": "garbage", "WAYLAND_DISPLAY": "wayland-7"}]:
    env = dict(os.environ); env.pop("DISPLAY", None); env.pop("WAYLAND_DISPLAY", None); env.update(envmod)
    p = subprocess.run(["timeout", "60", kl.BIN, "script", sp, "--root", root, "a.rs"], capture_output=True, env=env)
    outs.append((p.returncode, p.stdout, p.stderr))
print("== script mode with DISPLAY unset / :0 / garbage+WAYLAND_DISPLAY: same stdout", len({o[1] for o in outs}) == 1, "rcs", [o[0] for o in outs], "stderr", [o[2][:80] for o in outs])

# 5 several start paths (one missing, one a directory) in the window vs headless, with and without --root
XL = os.environ["XLIBS"] + "/usr/lib/x86_64-linux-gnu"
files = {"a.rs": "fn a() {}\n", "b.txt": "b\r\n", "d/c.rs": "struct C;\n"}
reqs = [{"id": "s", "ui": "snapshot"}, {"id": 1, "command": "doc.list"}, {"id": "f", "ui": "focus", "target": {"label": "Editor"}},
        {"id": "t", "ui": "type", "text": "Ω"}, {"id": "k", "ui": "key", "key": "S", "modifiers": ["ctrl"]}, {"id": 2, "command": "doc.text", "params": {"doc": 3}},
        {"id": "bad", "ui": "click", "target": {"label": "Nope"}}]
sp = os.path.join(kl.fresh("m5s"), "s.jsonl"); open(sp, "w").write(kl.jl(reqs))
for paths in (["a.rs", "missing.rs", "d", "b.txt", "d/c.rs"], ["a.rs", "missing.rs"], []):
    w = kl.fresh("m5w"); kl.write_files(w, files)
    h = kl.fresh("m5h"); kl.write_files(h, files)
    home = kl.fresh("m5home")
    env = dict(os.environ); env.pop("WAYLAND_DISPLAY", None); env.update(LD_LIBRARY_PATH=XL, MESA_SHADER_CACHE_DISABLE="true", HOME=home)
    pw = subprocess.run(["timeout", "120", "xvfb-run", "-a", "-s", "-screen 0 1280x800x24", kl.BIN, "--script", sp, "--root", w] + paths, capture_output=True, env=env)
    ph = subprocess.run(["timeout", "120", kl.BIN, "script", sp, "--root", h] + paths, capture_output=True)
    # options after paths, window form: paths first then --root, --script
    w2 = kl.fresh("m5w2"); kl.write_files(w2, files)
    pw2 = subprocess.run(["timeout", "120", "xvfb-run", "-a", "-s", "-screen 0 1280x800x24", kl.BIN] + paths + ["--script", sp, "--root", w2], capture_output=True, env=env)
    msg = kl.one(res_of(ph.stdout)["s"], label="Message")
    print("== start paths", paths, "window rc", pw.returncode, "headless rc", ph.returncode, "stdout equal", pw.stdout == ph.stdout,
          "options-after-paths equal", pw2.stdout == ph.stdout, "files equal", kl.read_tree(w) == kl.read_tree(h) == kl.read_tree(w2),
          "message", (msg or {}).get("value"), "title", (kl.one(res_of(ph.stdout)["s"], label="Title") or {}).get("value"), "stderr", pw.stderr[:100])
