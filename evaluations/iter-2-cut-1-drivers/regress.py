"""Regression sample of verified items through kiln-cli (and one check kiln-cli control vs kiln script).

usage: regress.py <seed>
"""
import json, os, random, subprocess, sys
sys.path.insert(0, os.path.dirname(__file__))
import kl
import jsonschema

seed = int(sys.argv[1])
rng = random.Random(seed)
schema_out = subprocess.run(["timeout", "30", kl.CLI, "commands", "--schema"], capture_output=True).stdout
committed = open(os.path.join(os.environ["KILN_CLONE"], "docs/commands.json"), "rb").read()
print("== cmd.schema: schema output == docs/commands.json:", schema_out == committed)
p = subprocess.run(["timeout", "30", kl.CLI, "commands", "--audit"], capture_output=True)
print("   --audit rc", p.returncode, p.stdout.decode().strip()[:200])
d = json.loads(schema_out)


def files_of(e):
    for f in e["files"]:
        if isinstance(f, list):
            yield f[0], f[1]
        else:
            yield f["path"], (f["text"] if "text" in f else bytes.fromhex(f["hex"]))


n = ok = valid = rn = rok = 0
bad = []
for c in d["commands"]:
    for kind_, exs in (("ok", c["examples"]), ("rx", c["refusal_examples"])):
        for i, e in enumerate(exs):
            if c["name"].startswith("buffer."):
                argv = [kl.CLI, "exec", c["name"], json.dumps(e["params"]), "--text", e["initial_text"], "--memory"]
                rc, out, err = kl.run(argv)
                last = {"ok": True, "result": json.loads(out)} if rc == 0 else {"ok": False, "error": json.loads(err.decode().splitlines()[0])}
            else:
                root = kl.fresh(f"rg-{c['name']}-{i}")
                kl.write_files(root, dict(files_of(e)))
                lines = [{"id": f"s{j}", "command": s[0], "params": s[1]} for j, s in enumerate(e["setup"])]
                lines.append({"id": "x", "command": c["name"], "params": e["params"]})
                rc, out, err = kl.cli_run(kl.jl(lines), root, t=600)
                last = kl.parse(out)[-1]
            if kind_ == "ok":
                n += 1
                if last.get("result") == e["result"]:
                    ok += 1
                else:
                    bad.append((c["name"], i, last, e["result"]))
                try:
                    jsonschema.validate(last.get("result"), c["result_schema"]); valid += 1
                except Exception as ex:
                    bad.append(("schema", c["name"], str(ex)[:200]))
            else:
                rn += 1
                if (last.get("error") or {}).get("kind") == e["kind"]:
                    rok += 1
                else:
                    bad.append(("rx", c["name"], i, e["kind"], last))
print(f"   examples={n} reproduced={ok} result_schema_valid={valid}; refusal_examples={rn} reproduced={rok}; problems={bad[:5]}")

# control.errors: hostile lines through kiln-cli control
root = kl.fresh("rg-ctl"); kl.write_files(root, {"a.txt": "abc\n"})
hostile = [b'{"id":1.5,"command":"doc.list"}', b'not json', b'[1]', b'{}', b'{"command":3}', b'{"command":"doc.list","x":1}',
           b'{"command":"doc.list","params":[]}', b'{"command":"Doc.List"}', b'{"id":"\\ud800","command":"doc.list"}',
           b'[' * 100000, b'\xff\xfe', b'   ', b'', b'{"id":7,"command":"doc.open","params":{"path":"' + b'x' * 400 + b'"}}',
           b'{"id":8,"command":"doc.insert","params":{"doc":1}}', b'{"id":9,"command":"doc.close","params":{"doc":0}}',
           b'{"id":true,"command":"doc.list"}', b'{"id":-9223372036854775808,"command":"doc.list"}', b'{"id":18446744073709551616,"command":"doc.list"}',
           b'{"id":10,"command":"doc.list"}']
rc, out, err = kl.cli_control(b"\n".join(hostile), root)
resp = [json.loads(l) for l in out.decode().splitlines()]
print("== control.errors: rc", rc, "non-blank lines", sum(1 for h in hostile if h.strip()), "responses", len(resp))
for h, r in zip([h for h in hostile if h.strip()], resp):
    e = r.get("error") or {}
    print("   ", h[:50], "->", r.get("id") if not isinstance(r.get("id"), str) or len(r.get("id")) < 20 else "...", e.get("code"), e.get("kind"), len(e.get("message", "")), "ok" if r.get("ok") else "")
# same lines through kiln script vs kiln-cli control: same bytes
sp = os.path.join(kl.fresh("rg-cs"), "s.jsonl")
open(sp, "wb").write(b"\n".join(hostile) + b"\n")
root2 = kl.fresh("rg-cs-root"); kl.write_files(root2, {"a.txt": "abc\n"})
rc2, out2, err2 = kl.run([kl.BIN, "script", sp, "--root", root2])
root3 = kl.fresh("rg-cs-root3"); kl.write_files(root3, {"a.txt": "abc\n"})
rc3, out3, err3 = kl.cli_control(b"\n".join(hostile) + b"\n", root3)
print("== kiln script vs kiln-cli control on the hostile lines: stdout identical:", out2 == out3, "rcs", rc2, rc3, "lines", out2.count(b"\n"))
# a second, longer one: random control requests
reqs = [{"id": 0, "command": "doc.open", "params": {"path": "a.txt"}}]
for i in range(300):
    c = rng.choice(["doc.insert", "doc.delete", "doc.undo", "doc.redo", "cursor.move", "doc.text", "search.find", "doc.position", "eol.normalize", "doc.save"])
    if c == "doc.insert":
        prm = {"doc": 1, "offset": rng.randint(0, 30), "text": rng.choice(["x", "é😀", "\r\n", "\r", "ab\ncd"])}
    elif c == "doc.delete":
        a = rng.randint(0, 30); prm = {"doc": 1, "start": a, "end": a + rng.randint(-1, 4)}
    elif c == "cursor.move":
        prm = {"doc": 1, "by": rng.choice(["char", "word", "line"]), "forward": rng.random() < 0.5}
    elif c == "search.find":
        prm = {"doc": 1, "query": rng.choice(["x", "é", "\r\n", "ab"])}
    elif c == "doc.position":
        prm = {"doc": 1, "at": {"char": rng.randint(0, 30)}}
    elif c == "eol.normalize":
        prm = {"doc": 1, "to": rng.choice(["lf", "crlf", "cr"])}
    else:
        prm = {"doc": 1}
    reqs.append({"id": i + 1, "command": c, "params": prm})
open(sp, "w").write(kl.jl(reqs))
r4 = kl.fresh("rg4"); kl.write_files(r4, {"a.txt": "abc\n"})
r5 = kl.fresh("rg5"); kl.write_files(r5, {"a.txt": "abc\n"})
rc4, out4, _ = kl.run([kl.BIN, "script", sp, "--root", r4])
rc5, out5, _ = kl.cli_control(kl.jl(reqs), r5)
print("== random 301 control requests: kiln script vs kiln-cli control stdout identical:", out4 == out5, "files identical:", kl.read_tree(r4) == kl.read_tree(r5),
      "refused:", sum(1 for l in kl.parse(out4) if not l["ok"]))

# ws.path-confinement
root = kl.fresh("rg-conf"); kl.write_files(root, {"in.txt": "in"})
outside = kl.fresh("rg-outside"); kl.write_files(outside, {"secret.txt": "SECRET"})
os.symlink(os.path.join(outside, "secret.txt"), os.path.join(root, "link.txt"))
os.symlink(outside, os.path.join(root, "dirlink"))
reqs = [{"id": "a", "command": "doc.open", "params": {"path": "link.txt"}}, {"id": "b", "command": "doc.open", "params": {"path": "dirlink/secret.txt"}},
        {"id": "c", "command": "doc.open", "params": {"path": "../rg-outside/secret.txt"}}, {"id": "d", "command": "doc.open", "params": {"path": "in.txt"}}]
rc, out, err = kl.cli_run(kl.jl(reqs), root)
print("== ws.path-confinement:", [(r["id"], (r.get("error") or {}).get("kind") or "ok") for r in kl.parse(out)], "secret unchanged:", open(os.path.join(outside, "secret.txt")).read() == "SECRET")

# cli.run: the worked example of control-protocol.md, byte for byte
root = kl.fresh("rg-we"); kl.write_files(root, {"a.txt": "hello\n"})
script = ('{"id":1,"command":"doc.open","params":{"path":"a.txt"}}\n{"id":2,"command":"doc.insert","params":{"doc":1,"offset":5,"text":" world"}}\n'
          '{"id":3,"command":"doc.undo","params":{"doc":1}}\n{"id":4,"command":"doc.redo","params":{"doc":1}}\n'
          '{"id":"oops","command":"doc.insert","params":{"doc":1}}\n{"id":5,"command":"doc.save","params":{"doc":1}}\n')
exp_out = ('{"id":1,"ok":true,"result":{"already_open":false,"doc":1,"encoding":"utf-8","len_chars":6,"line_ending":"lf","mixed_line_endings":false,"path":"a.txt"}}\n'
           '{"id":2,"ok":true,"result":{"dirty":true,"len_chars":12}}\n{"id":3,"ok":true,"result":{"dirty":false,"len_chars":6,"undone":true}}\n'
           '{"id":4,"ok":true,"result":{"dirty":true,"len_chars":12,"redone":true}}\n'
           '{"error":{"code":"invalid_params","details":["\\"offset\\" is a required property","\\"text\\" is a required property"],"kind":"schema","message":"invalid params for `doc.insert`"},"id":"oops","ok":false}\n'
           '{"id":5,"ok":true,"result":{"bytes_written":12,"dirty":false,"doc":1}}\n')
exp_log = ('{"command":"doc.open","id":1,"params":{"path":"a.txt"}}\n{"command":"doc.insert","id":2,"params":{"doc":1,"offset":5,"text":" world"}}\n'
           '{"command":"doc.undo","id":3,"params":{"doc":1}}\n{"command":"doc.redo","id":4,"params":{"doc":1}}\n{"command":"doc.save","id":5,"params":{"doc":1}}\n')
rc, out, err = kl.cli_run(script, root, log=root + ".log")
print("== cli.run worked example: rc", rc, "stdout as documented:", out.decode() == exp_out, "file:", open(os.path.join(root, "a.txt"), "rb").read(),
      "log as documented:", open(root + ".log").read() == exp_log)

# text.positions: doc.position of every char offset of the doc-positions.md text vs a naive model
t = "a\r\nbé😀\rc\n"
root = kl.fresh("rg-pos"); kl.write_files(root, {"a.txt": t})
reqs = [{"id": "o", "command": "doc.open", "params": {"path": "a.txt"}}] + [{"id": i, "command": "doc.position", "params": {"doc": 1, "at": {"char": i}}} for i in range(len(t) + 2)]
rc, out, err = kl.cli_run(kl.jl(reqs), root)
mism = 0
for r in kl.parse(out)[1:]:
    i = r["id"]
    if i > len(t):
        mism += 0 if (not r["ok"] and r["error"]["kind"] == "out_of_range") else 1
        continue
    if 0 < i < len(t) and t[i - 1] == "\r" and t[i] == "\n":
        mism += 0 if not r["ok"] else 1
        continue
    pre = t[:i]
    nl = pre.replace("\r\n", "\n").replace("\r", "\n")
    line = nl.count("\n"); col = len(nl) - (nl.rfind("\n") + 1)
    exp = {"char": i, "byte": len(pre.encode()), "utf16": len(pre.encode("utf-16-le")) // 2, "line": line, "col": col}
    if r.get("result") != exp:
        mism += 1; print("   pos mismatch", i, r, exp)
print("== text.positions: offsets", len(t) + 2, "mismatches", mism, "(between \\r\\n and past the end: refused as expected)" )

# undo.roundtrip: random edits, undo all, redo all
mism = 0
for s in range(30):
    t0 = "".join(rng.choice(["a", "é", "\r\n", "\n", "\r", "😀", " "]) for _ in range(rng.randint(0, 20)))
    root = kl.fresh("rg-undo"); kl.write_files(root, {"a.txt": t0})
    reqs = [{"id": "o", "command": "doc.open", "params": {"path": "a.txt"}}]
    cur = t0
    for k in range(rng.randint(1, 25)):
        if rng.random() < 0.6 or not cur:
            off = rng.randint(0, len(cur)); txt = rng.choice(["x", "é😀", "\r\n", "\r", "ab\ncd"])
            reqs.append({"id": f"e{k}", "command": "doc.insert", "params": {"doc": 1, "offset": off, "text": txt}})
            cur = cur[:off] + txt + cur[off:]
        else:
            a = rng.randint(0, len(cur)); b = rng.randint(a, len(cur))
            reqs.append({"id": f"e{k}", "command": "doc.delete", "params": {"doc": 1, "start": a, "end": b}})
            cur = cur[:a] + cur[b:]
        if rng.random() < 0.3:
            reqs.append({"id": f"m{k}", "command": "cursor.move", "params": {"doc": 1, "by": "char", "forward": True}})
    reqs += [{"id": f"u{k}", "command": "doc.undo", "params": {"doc": 1}} for k in range(40)]
    reqs.append({"id": "tu", "command": "doc.text", "params": {"doc": 1}})
    reqs += [{"id": f"r{k}", "command": "doc.redo", "params": {"doc": 1}} for k in range(40)]
    reqs.append({"id": "tr", "command": "doc.text", "params": {"doc": 1}})
    rc, out, err = kl.cli_run(kl.jl(reqs), root)
    res = {r["id"]: r for r in kl.parse(out)}
    refused = [r for r in res.values() if not r["ok"]]
    if res["tu"]["result"]["text"] != t0 or res["tr"]["result"]["text"] != cur:
        mism += 1
        print("   undo mismatch", repr(t0), refused[:2])
print("== undo.roundtrip: sessions 30, mismatches", mism)

# search.literal: random literal finds vs a naive count (case-sensitive, non-overlapping from the left)
mism = 0; cases = 0
for s in range(40):
    t0 = "".join(rng.choice(["a", "b", "ab", "é", "\r\n", "😀", "A"]) for _ in range(rng.randint(0, 40)))
    root = kl.fresh("rg-find"); kl.write_files(root, {"a.txt": t0})
    qs = ["a", "ab", "aa", "é", "\r\n", "😀a", "b\r", "A"]
    reqs = [{"id": "o", "command": "doc.open", "params": {"path": "a.txt"}}] + [{"id": q, "command": "search.find", "params": {"doc": 1, "query": q}} for q in qs]
    rc, out, err = kl.cli_run(kl.jl(reqs), root)
    for r in kl.parse(out)[1:]:
        q = r["id"]; cases += 1
        exp = []; i = 0
        while True:
            j = t0.find(q, i)
            if j < 0:
                break
            exp.append({"start": j, "end": j + len(q)}); i = j + len(q)
        if r["result"]["matches"] != exp or r["result"]["total"] != len(exp):
            mism += 1; print("   find mismatch", repr(t0), repr(q), r["result"], exp)
print("== search.literal: cases", cases, "mismatches", mism)

# syntax.highlight: published example + a coverage property (spans cover every char exactly once, in order)
root = kl.fresh("rg-hl"); src = 'fn f() { let s: u8 = 1; } // c'
kl.write_files(root, {"a.rs": src, "b.rs": "struct S<T> { x: \"é😀\", y: 0x1f } /* c */\n#[derive(Debug)]\nimpl S { fn g(&self) -> i32 { 3 } }\n"})
reqs = [{"id": "o", "command": "doc.open", "params": {"path": "a.rs"}}, {"id": "h", "command": "syntax.highlight", "params": {"doc": 1}},
        {"id": "o2", "command": "doc.open", "params": {"path": "b.rs"}}, {"id": "h2", "command": "syntax.highlight", "params": {"doc": 2}},
        {"id": "l2", "command": "doc.info", "params": {"doc": 2}}]
rc, out, err = kl.cli_run(kl.jl(reqs), root)
res = {r["id"]: r for r in kl.parse(out)}
ex = [e for c in d["commands"] if c["name"] == "syntax.highlight" for e in c["examples"]][0]["result"]
sp_ = res["h2"]["result"]["spans"]
cover = sp_[0]["start"] == 0 and sp_[-1]["end"] == res["l2"]["result"]["len_chars"] and all(a["end"] == b["start"] for a, b in zip(sp_, sp_[1:]))
print("== syntax.highlight: example reproduced:", res["h"]["result"] == ex, "second file spans cover exactly once:", cover, "kinds:", sorted({s["kind"] for s in sp_}))
