"""syntax.parse on the pinned corpus (fixture data, read per the director's ruling for R-222):
check MANIFEST.toml entries (bytes, \n count, FNV-1a 64) against the files, copy the files into a
fixture root, open each through `kiln-cli run` and ask `syntax.tree`; report has_error per set;
optionally run rustc with the recorded command (arg `rustc`).
usage: python3 -I corpus.py [rustc]"""
import sys, os, json, shutil, subprocess, tomllib
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from k2 import *

C = BASE + "/clone/crates/kiln-syntax/tests/corpus"
man = tomllib.load(open(C + "/MANIFEST.toml", "rb"))["file"]
run_rec = tomllib.load(open(C + "/corpus-rustc-run.toml", "rb"))
rec = {f["path"]: f for f in run_rec["file"]}


def fnv(b):
    h = 0xcbf29ce484222325
    for x in b:
        h ^= x; h = (h * 0x100000001b3) & 0xFFFFFFFFFFFFFFFF
    return f"{h:016x}"


on_disk = sorted(os.path.relpath(os.path.join(dp, f), C) for dp, dn, fn in os.walk(C) for f in fn
                 if f not in ("MANIFEST.toml", "corpus-rustc-run.toml"))
print("files on disk", len(on_disk), "manifest entries", len(man), "not in manifest", sorted(set(on_disk) - {m["path"] for m in man}),
      "missing", sorted({m["path"] for m in man} - set(on_disk)))
root = fresh("corpus")
bad_meta = []
for m in man:
    b = open(os.path.join(C, m["path"]), "rb").read()
    if (len(b), b.count(b"\n"), fnv(b)) != (m.get("bytes"), m.get("lines"), m.get("fnv1a64")):
        bad_meta.append((m["path"], len(b), b.count(b"\n"), fnv(b)))
    write(root, m["path"], b)
print("manifest mismatches (bytes/lines/fnv)", bad_meta)
lines = []
for i, m in enumerate(man):
    lines += [req("doc.open", id=f"o{i}", path=m["path"]), req("syntax.tree", id=f"t{i}", doc=i + 1)]
rc, rs, err = runj(lines, root=root, timeout=900)
by = {r["id"]: r for r in rs}
res = {}
for i, m in enumerate(man):
    o = by[f"o{i}"]; t = by[f"t{i}"]
    if not o["ok"] or not t["ok"]:
        res[m["path"]] = ("refused", (o.get("error") or t.get("error")))
    else:
        tr = t["result"]
        res[m["path"]] = (tr["has_error"], len(tr["errors"]), o["result"]["len_chars"], o["result"]["line_ending"], tr["errors"][:2])
for s in ("valid", "gap", "invalid"):
    ms = [m for m in man if m["set"] == s]
    err = [m["path"] for m in ms if res[m["path"]][0] is True]
    print(f"set={s} files={len(ms)} has_error={len(err)} without_error={len(ms) - len(err)} refused={sum(1 for m in ms if res[m['path']][0] == 'refused')}")
    for m in ms:
        print("   ", m["path"], json.dumps(res[m["path"]]), "rustc-recorded-exit", rec.get(m["path"], {}).get("exit"),
              "fnv-in-run-record-matches", rec.get(m["path"], {}).get("fnv1a64") == m.get("fnv1a64") if m["path"] in rec else None)
print("rustc record: command", run_rec.get("command"), "| toolchain", run_rec.get("toolchain"), "| entries", len(rec),
      "| exits", sorted({(f["exit"]) for f in rec.values()}))
if len(sys.argv) > 1 and sys.argv[1] == "rustc":
    out = fresh("corpus-rustc-out"); src = fresh("corpus-rustc-src")
    for m in man:
        dst = os.path.join(src, m["path"].replace("/", "__").replace(".rs.txt", ".rs"))
        shutil.copyfile(os.path.join(C, m["path"]), dst)
        p = subprocess.run(["timeout", "120", "rustc", "--edition", "2024", "--crate-type", "lib", "--emit", "metadata", "--crate-name", "corpus",
                            "--cap-lints", "allow", "--out-dir", out, dst], capture_output=True, cwd=src)
        first = next((l for l in p.stderr.decode().splitlines() if l.startswith("error")), "")
        print("  rustc", m["set"], m["path"], "exit", p.returncode, "recorded", rec.get(m["path"], {}).get("exit"), first[:150])
