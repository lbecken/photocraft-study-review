"""One regression check (or a few) per item verified before this stage, through the binary.
usage: python3 -I regress.py"""
import sys, os, json, subprocess, shutil, filecmp
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from k2 import *


def P(label, *a):
    print(label, *[x if isinstance(x, str) else json.dumps(x, ensure_ascii=False) for x in a])


print("=== cli.exec")
P("buffer.insert", exe("buffer.insert", {"offset": 2, "text": "X"}, "--text", "héllo", "--memory"))
P("out of range", exe("buffer.insert", {"offset": 9, "text": "X"}, "--text", "héllo", "--memory"))
P("unknown", exe("nosuch", {}, "--memory"))
P("bad json first", exe("doc.open", "{", "--root", "/nonexistent"))
r0 = fresh("rg-exec"); write(r0, "a.rs", "fn a() {}\n")
P("exec project.tree", exe("project.tree", {}, "--root", r0))
P("exec nav.workspace-symbols", exe("nav.workspace-symbols", {"query": "a"}, "--root", r0))
P("exec syntax.tree", exe("syntax.tree", {"doc": 1}, "--root", r0))
P("exec search.find", exe("search.find", {"doc": 1, "query": "a"}, "--root", r0))

print("=== cli.run / cmd.log-replay / control.session (new commands included)")
A = fresh("rg-detA"); B = fresh("rg-det/x/y/B")
for R in (A, B):
    write(R, "src/a.rs", "fn alpha() {}\r\nstruct Beta;\r\n"); write(R, ".gitignore", "*.log\n"); write(R, "x.log", "")
    write(R, "u16.txt", b"\xff\xfe" + "héllo\n".encode("utf-16-le")); write(R, "bom.txt", b"\xef\xbb\xbfhi\n")
script = [req("doc.open", id=1, path="src/a.rs"), req("search.replace", id=2, doc=1, query="(a)(l)", regex=True, replacement="$2$1", all=True),
          req("syntax.tree", id=3, doc=1), req("syntax.highlight", id=4, doc=1, start=0, end=5), req("nav.outline", id=5, doc=1),
          req("nav.goto-line", id=6, doc=1, line=1, col=3), req("nav.goto-symbol", id=7, doc=1, index=1), req("project.tree", id=8),
          req("nav.workspace-symbols", id=9, query="BETA"), req("eol.normalize", id=10, doc=1, to="lf"), req("doc.undo", id=11, doc=1),
          req("search.find", id=12, doc=1, query=""), req("doc.save", id=13, doc=1), req("doc.open", id=14, path="u16.txt"),
          req("doc.insert", id=15, doc=2, offset=0, text="Z"), req("doc.save", id=16, doc=2), req("doc.open", id=17, path="bom.txt"),
          req("doc.insert", id=18, doc=3, offset=2, text="!"), req("doc.save", id=19, doc=3), "", "   ", '{"id":"x","command":"syntax.match-bracket","params":{"doc":1,"offset":12}}']
outs = []
for R in (A, B):
    rc, out, err = run(script, root=R, extra=["--log", os.path.join(WORK, f"rg-log-{len(outs)}.jsonl")], name="rg-det.jsonl")
    outs.append((rc, out))
P("rc", outs[0][0], outs[1][0], "stdout identical", outs[0][1] == outs[1][1], "lines", len(outs[0][1].splitlines()))
d = filecmp.dircmp(A, B); P("files identical", d.diff_files == [] and d.left_only == [] and d.right_only == [], d.diff_files)
print(outs[0][1][:1500])
logA = open(os.path.join(WORK, "rg-log-0.jsonl")).read()
P("log lines", len(logA.splitlines()), "logged commands", [json.loads(l)["command"] for l in logA.splitlines()])
C = fresh("rg-detC")
write(C, "src/a.rs", "fn alpha() {}\r\nstruct Beta;\r\n"); write(C, ".gitignore", "*.log\n"); write(C, "x.log", "")
write(C, "u16.txt", b"\xff\xfe" + "héllo\n".encode("utf-16-le")); write(C, "bom.txt", b"\xef\xbb\xbfhi\n")
p = subprocess.run(["timeout", "60", KILN, "run", os.path.join(WORK, "rg-log-0.jsonl"), "--root", C, "--log", os.path.join(WORK, "rg-log-C.jsonl")], capture_output=True)
P("replay rc", p.returncode, "log identical", open(os.path.join(WORK, "rg-log-C.jsonl")).read() == logA)
d = filecmp.dircmp(A, C); P("replay files identical", d.diff_files == [] and d.left_only == [] and d.right_only == [], d.diff_files)
for n in ["src/a.rs", "u16.txt", "bom.txt"]:
    P("bytes", n, open(os.path.join(A, n), "rb").read().hex())

print("=== io.utf8 / io.crlf / io.encoding (open+save unedited is byte-identical)")
R = fresh("rg-io")
files = {"lf": b"a\nb\n", "crlf": b"a\r\nb\r\n", "cr": b"a\rb\r", "mixed": b"a\r\nb\nc\r", "none": b"abc", "empty": b"", "bom": b"\xef\xbb\xbfa\n",
         "bomonly": b"\xef\xbb\xbf", "u16le": b"\xff\xfe" + "a\r\n中😀".encode("utf-16-le"), "u16be": b"\xfe\xff" + "a\n".encode("utf-16-be"), "nul": b"a\x00b\n"}
for n, b in files.items(): write(R, n, b)
lines = []
for i, n in enumerate(files): lines += [req("doc.open", id=f"o{n}", path=n), req("doc.save", id=f"s{n}", doc=i + 1)]
lines += [req("doc.open", id="bad", path="bad"), req("doc.open", id="odd", path="odd")]
write(R, "bad", b"fn a() {}\n// \xff"); write(R, "odd", b"\xff\xfea\x00b")
rc, rs, err = runj(lines, root=R)
P("all ok", all(r["ok"] for r in rs[:-2]), "line endings", [r["result"]["line_ending"] for r in rs if r["id"] in {"o" + n for n in files}],
  "mixed", [n for r, n in zip([r for r in rs if r["id"] in {"o" + n for n in files}], files) if r["result"]["mixed_line_endings"]])
P("unchanged", all(open(os.path.join(R, n), "rb").read() == b for n, b in files.items()))
P("decode errors", rs[-2]["error"], rs[-1]["error"])

print("=== io.save-atomic")
R = fresh("rg-atomic"); write(R, "f.txt", "x\n"); os.chmod(os.path.join(R, "f.txt"), 0o751)
st = subprocess.run(["strace", "-f", "-qq", "-e", "trace=openat,rename,renameat,renameat2,fsync,unlink,unlinkat", "-o", os.path.join(WORK, "rg-strace.txt"),
                     "timeout", "60", KILN, "run", os.path.join(WORK, "rg-at.jsonl"), "--root", R], capture_output=True) if not open(os.path.join(WORK, "rg-at.jsonl"), "w").write(
    '{"id":1,"command":"doc.open","params":{"path":"f.txt"}}\n{"id":2,"command":"doc.insert","params":{"doc":1,"offset":0,"text":"y"}}\n{"id":3,"command":"doc.save","params":{"doc":1}}\n') is None else None
tr = [l.strip() for l in open(os.path.join(WORK, "rg-strace.txt")) if "kiln-tmp" in l or ("fsync" in l) or "rename" in l]
print("\n".join(l.replace(R, "$R") for l in tr[-6:]))
P("mode", oct(os.stat(os.path.join(R, "f.txt")).st_mode & 0o777), "content", open(os.path.join(R, "f.txt")).read(), "leftovers", [n for n in os.listdir(R) if "kiln-tmp" in n])
subprocess.run(["chattr", "+i", R])
rc, rs, err = runj([req("doc.open", id=1, path="f.txt"), req("doc.insert", id=2, doc=1, offset=0, text="z"), req("doc.save", id=3, doc=1), req("doc.info", id=4, doc=1)], root=R)
subprocess.run(["chattr", "-i", R])
P("immutable dir save", rs[2], "dirty", rs[3]["result"]["dirty"], "file", open(os.path.join(R, "f.txt")).read(), "leftovers", [n for n in os.listdir(R) if "kiln-tmp" in n])

print("=== text.insert / text.delete / undo / dirty / documents")
cmds = [("doc.insert", {"doc": 1, "offset": 5, "text": "x"}), ("doc.insert", {"doc": 1, "offset": -1, "text": "x"}), ("doc.delete", {"doc": 1, "start": 3, "end": 1}),
        ("doc.delete", {"doc": 1, "start": 0, "end": 99}), ("doc.delete", {"doc": 1, "start": 1, "end": 3}), ("doc.undo", {"doc": 1}), ("doc.redo", {"doc": 1}),
        ("doc.insert", {"doc": 1, "offset": 0, "text": "a"}), ("doc.insert", {"doc": 1, "offset": 1, "text": "b"}), ("doc.undo", {"doc": 1}), ("doc.redo", {"doc": 1}),
        ("doc.undo", {"doc": 1}), ("doc.undo", {"doc": 1}), ("doc.info", {"doc": 1}), ("doc.undo", {"doc": 1}), ("doc.open", {"path": "b.txt"}),
        ("doc.insert", {"doc": 2, "offset": 0, "text": "Q"}), ("doc.close", {"doc": 2}), ("doc.close", {"doc": 2, "discard": True}), ("doc.open", {"path": "b.txt"}),
        ("doc.list", {}), ("doc.text", {"doc": 1})]
lines = [req("doc.open", id=0, path="a.txt")] + [req(c, id=i + 1, **p) for i, (c, p) in enumerate(cmds)]
rc, rs, err = mem(lines, [("a.txt", "abcd"), ("b.txt", "B")])
for (c, p), r in zip(cmds, rs[1:]):
    print("  ", c, json.dumps(p), "->", json.dumps(r.get("result") if r["ok"] else r["error"]["kind"]))

print("=== control.protocol / control.errors")
lines = ['{"id":-9223372036854775808,"command":"doc.list"}', '{"id":18446744073709551615,"command":"doc.list"}', '{"id":18446744073709551616,"command":"doc.list"}',
         '{"id":1.0,"command":"doc.list"}', '{"id":"s","command":"doc.list"}\r', "", "  ", "not json", "[1]", "{}", '{"command":1}', '{"command":"doc.list","x":1}',
         '{"command":"doc.list","params":[]}', '{"command":"DOC.LIST"}', '{"command":"doc.list","id":"\\ud800"}', "[" * 100000,
         '{"id":"big","command":"search.find","params":{' + ",".join(f'"k{i}":{i}' for i in range(400)) + '}}',
         '{"id":"longpath","command":"doc.open","params":{"path":"' + "p" * 400 + '"}}',
         '{"id":"pat","command":"search.find","params":{"doc":1,"query":"(' + "x" * 900 + '","regex":true}}']
path = os.path.join(WORK, "rg-proto.jsonl")
with open(path, "wb") as f:
    for l in lines: f.write(l.encode() + b"\n")
    f.write(b'{"id":"badutf8","command":"\xff"}\n{"id":"last","command":"doc.list"}')
p = subprocess.run(["timeout", "60", KILN, "control", "--root", r0], stdin=open(path, "rb"), capture_output=True)
outl = p.stdout.decode().splitlines()
P("rc", p.returncode, "responses", len(outl), "non-blank requests", len([l for l in lines if l.strip()]) + 2)
for l in outl:
    r = json.loads(l); e = r.get("error")
    print("  ", len(l.encode()), json.dumps({"id": r["id"] if not isinstance(r["id"], str) or len(r["id"]) < 20 else r["id"][:20], "ok": r["ok"],
                                             **({"code": e["code"], "kind": e["kind"], "msglen": len(e["message"]), "details": e.get("details") if not isinstance(e.get("details"), list) else len(e["details"])} if e else {})}))
