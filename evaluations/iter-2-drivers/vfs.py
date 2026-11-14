"""Memory (--memory-seed) vs disk (--root) agreement on random scripts.
usage: python3 -I vfs.py <seed> <ncases> <steps>"""
import sys, os, random, json, shutil, subprocess
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from k import KILN, WORK

PATHS = ["a.txt", "src/main.rs", "src/lib/x.rs", "d/e/f.txt", "bom.txt", "u16.txt", "bad.txt", "cr.txt"]
PROBES = ["a.txt/x", "src", "src/lib", "missing.txt", "src/missing/y.rs", "d/e", "../a.txt", "a//b",
          "A.TXT", "src/main.rs/", "nope/x", "d/e/f.txt/g"]
TXT = ["a", "é", "😀", "\r\n", "\n", "\r", "x", "中"]


def main():
    seed, n, steps = map(int, sys.argv[1:4])
    rnd = random.Random(seed)
    base = os.path.join(WORK, f"vfs-{seed}")
    if os.path.isdir(base):
        shutil.rmtree(base)
    os.makedirs(base)
    same = diff = 0
    first = None
    for case in range(n):
        files = []
        for p in rnd.sample(PATHS, rnd.randint(1, len(PATHS))):
            if p == "bom.txt":
                files.append({"path": p, "hex": "efbbbf" + "".join(rnd.choice(TXT) for _ in range(5)).encode().hex()})
            elif p == "u16.txt":
                files.append({"path": p, "hex": rnd.choice(["fffe", "feff"]) + ("68006900" if True else "")})
            elif p == "bad.txt":
                files.append({"path": p, "hex": rnd.choice(["ff", "c3", "6162ed a080".replace(" ", "")])})
            else:
                files.append({"path": p, "text": "".join(rnd.choice(TXT) for _ in range(rnd.randint(0, 12)))})
        man = os.path.join(base, f"m{case}.json")
        json.dump({"files": files}, open(man, "w"))
        root = os.path.join(base, f"d{case}")
        for f in files:
            fp = os.path.join(root, f["path"])
            os.makedirs(os.path.dirname(fp), exist_ok=True)
            open(fp, "wb").write(f["text"].encode() if "text" in f else bytes.fromhex(f["hex"]))
        os.makedirs(root, exist_ok=True)
        lines = []
        allp = PATHS + PROBES
        for i in range(steps):
            op = rnd.choice(["open", "open", "insert", "insert", "save", "save", "close", "text", "info",
                             "list", "undo", "normalize", "position", "line", "cursor"])
            d = rnd.randint(1, 6)
            if op == "open":
                r = {"command": "doc.open", "params": {"path": rnd.choice(allp)}}
            elif op == "insert":
                r = {"command": "doc.insert", "params": {"doc": d, "offset": rnd.randint(0, 6), "text": rnd.choice(TXT) * rnd.randint(1, 3)}}
            elif op == "save":
                r = {"command": "doc.save", "params": {"doc": d}}
            elif op == "close":
                r = {"command": "doc.close", "params": {"doc": d, "discard": rnd.random() < 0.5}}
            elif op == "text":
                r = {"command": "doc.text", "params": {"doc": d}}
            elif op == "info":
                r = {"command": "doc.info", "params": {"doc": d}}
            elif op == "list":
                r = {"command": "doc.list", "params": {}}
            elif op == "undo":
                r = {"command": rnd.choice(["doc.undo", "doc.redo"]), "params": {"doc": d}}
            elif op == "normalize":
                r = {"command": "eol.normalize", "params": {"doc": d, "to": rnd.choice(["lf", "crlf", "cr"])}}
            elif op == "position":
                r = {"command": "doc.position", "params": {"doc": d, "at": {"byte": rnd.randint(0, 8)}}}
            elif op == "line":
                r = {"command": "doc.line", "params": {"doc": d, "line": rnd.randint(0, 3)}}
            else:
                r = {"command": "cursor.move", "params": {"doc": d, "by": "word", "forward": True}}
            r["id"] = i
            lines.append(json.dumps(r))
        # epilogue: close everything, reopen every file, info + text
        for d in range(1, 40):
            lines.append(json.dumps({"command": "doc.close", "params": {"doc": d, "discard": True}}))
        for p in PATHS:
            lines.append(json.dumps({"command": "doc.open", "params": {"path": p}}))
        for d in range(1, 60):
            lines.append(json.dumps({"command": "doc.info", "params": {"doc": d}}))
            lines.append(json.dumps({"command": "doc.text", "params": {"doc": d}}))
        sc = os.path.join(base, f"s{case}.jsonl")
        open(sc, "w").write("\n".join(lines) + "\n")
        a = subprocess.run(["timeout", "60", KILN, "run", sc, "--memory-seed", man], capture_output=True)
        b = subprocess.run(["timeout", "60", KILN, "run", sc, "--root", root], capture_output=True)
        if a.stdout == b.stdout and a.returncode == b.returncode:
            same += 1
        else:
            diff += 1
            if first is None:
                la, lb = a.stdout.decode().splitlines(), b.stdout.decode().splitlines()
                for i, (x, y) in enumerate(zip(la, lb)):
                    if x != y:
                        first = (case, i, lines[i] if i < len(lines) else None, x, y)
                        break
                else:
                    first = (case, "len/rc", len(la), len(lb), a.returncode, b.returncode, a.stderr.decode()[:300])
    print(f"seed={seed} cases={n} steps={steps} identical={same} different={diff}")
    if first:
        print(json.dumps(first, ensure_ascii=False))


main()
