"""Exhaustive doc.line / doc.position comparison with a naive Python model on random texts.
usage: python3 -I positions.py <seed> <ntexts>"""
import sys, os, random, json, shutil
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from k import Ctl, WORK

ALPHA = ["a", "b", "é", "😀", "中", "\r", "\n", "\r\n", "_", " ", " ", "\x85", "\x0b", "\x0c"]


def model(t):
    lines = []  # (start, end, ending)
    i, s = 0, 0
    while i < len(t):
        if t[i] == "\r" and i + 1 < len(t) and t[i + 1] == "\n":
            lines.append((s, i, "crlf")); i += 2; s = i
        elif t[i] == "\r":
            lines.append((s, i, "cr")); i += 1; s = i
        elif t[i] == "\n":
            lines.append((s, i, "lf")); i += 1; s = i
        else:
            i += 1
    lines.append((s, len(t), "none"))
    b = [0]; u = [0]
    for ch in t:
        b.append(b[-1] + len(ch.encode()))
        u.append(u[-1] + len(ch.encode("utf-16-le")) // 2)
    return lines, b, u


def expect_char(t, lines, b, u, c):
    if c > len(t):
        return ("out_of_range", f"index {c} is out of bounds (length {len(t)})")
    if 0 < c < len(t) and t[c - 1] == "\r" and t[c] == "\n":
        return ("not_a_boundary", f"char index {c} is between the \\r and \\n of a CRLF")
    for li, (s, e, _) in enumerate(lines):
        if s <= c <= e:
            return {"char": c, "byte": b[c], "utf16": u[c], "line": li, "col": c - s}
    raise AssertionError


def main():
    seed, n = int(sys.argv[1]), int(sys.argv[2])
    rnd = random.Random(seed)
    root = os.path.join(WORK, f"pos-{seed}")
    shutil.rmtree(root, ignore_errors=True) if root.startswith(WORK + "/pos-") else None
    os.makedirs(root)
    texts = []
    for k in range(n):
        t = "".join(rnd.choice(ALPHA) for _ in range(rnd.randint(0, 14)))
        texts.append(t)
        with open(os.path.join(root, f"t{k}.txt"), "w", newline="") as f:
            f.write(t)
    c = Ctl(root)
    cases = mism = 0
    msgs = []
    for k, t in enumerate(texts):
        r = c.ok("doc.open", path=f"t{k}.txt")
        d = r["doc"]
        lines, b, u = model(t)
        info = c.ok("doc.info", doc=d)
        cases += 1
        if info["len_lines"] != len(lines):
            mism += 1; msgs.append(("len_lines", repr(t), info["len_lines"], len(lines)))
        # every line + one past the end
        for li in range(len(lines) + 2):
            cases += 1
            r = c.req("doc.line", doc=d, line=li)
            if li < len(lines):
                s, e, end = lines[li]
                exp = {"line": li, "text": t[s:e], "start": s, "end": e, "ending": end}
                if not r["ok"] or r["result"] != exp:
                    mism += 1; msgs.append(("line", repr(t), li, r, exp))
            else:
                em = f"invalid line: line {li} is out of bounds (buffer has {len(lines)} lines)"
                if r["ok"] or r["error"]["kind"] != "out_of_range" or r["error"]["message"] != em:
                    mism += 1; msgs.append(("line-oob", repr(t), li, r, em))
        # every char
        valid = []
        for ci in range(len(t) + 3):
            cases += 1
            r = c.req("doc.position", doc=d, at={"char": ci})
            exp = expect_char(t, lines, b, u, ci)
            if isinstance(exp, dict):
                valid.append(exp)
                if not r["ok"] or r["result"] != exp:
                    mism += 1; msgs.append(("char", repr(t), ci, r, exp))
            else:
                if r["ok"] or r["error"]["kind"] != exp[0] or r["error"]["message"] != "invalid position: " + exp[1]:
                    mism += 1; msgs.append(("char-ref", repr(t), ci, r, exp))
        # every byte
        for bi in range(b[-1] + 3):
            cases += 1
            r = c.req("doc.position", doc=d, at={"byte": bi})
            if bi > b[-1]:
                exp = ("out_of_range", f"index {bi} is out of bounds (length {b[-1]})")
            elif bi not in b:
                exp = ("not_a_boundary", f"byte offset {bi} is inside a multi-byte char")
            else:
                exp = expect_char(t, lines, b, u, b.index(bi))
            if isinstance(exp, dict):
                if not r["ok"] or r["result"] != exp:
                    mism += 1; msgs.append(("byte", repr(t), bi, r, exp))
            elif r["ok"] or r["error"]["kind"] != exp[0] or r["error"]["message"] != "invalid position: " + exp[1]:
                mism += 1; msgs.append(("byte-ref", repr(t), bi, r, exp))
        for ui in range(u[-1] + 3):
            cases += 1
            r = c.req("doc.position", doc=d, at={"utf16": ui})
            if ui > u[-1]:
                exp = ("out_of_range", f"index {ui} is out of bounds (length {u[-1]})")
            elif ui not in u:
                exp = ("not_a_boundary", f"UTF-16 offset {ui} is inside a surrogate pair")
            else:
                exp = expect_char(t, lines, b, u, u.index(ui))
            if isinstance(exp, dict):
                if not r["ok"] or r["result"] != exp:
                    mism += 1; msgs.append(("utf16", repr(t), ui, r, exp))
            elif r["ok"] or r["error"]["kind"] != exp[0] or r["error"]["message"] != "invalid position: " + exp[1]:
                mism += 1; msgs.append(("utf16-ref", repr(t), ui, r, exp))
        for li in range(len(lines) + 1):
            maxc = lines[li][1] - lines[li][0] if li < len(lines) else 0
            for co in range(maxc + 2):
                cases += 1
                r = c.req("doc.position", doc=d, at={"line_col": {"line": li, "col": co}})
                if li >= len(lines):
                    exp = ("out_of_range", f"line {li} is out of bounds (buffer has {len(lines)} lines)")
                elif co > maxc:
                    exp = ("out_of_range", f"column {co} is out of bounds for line {li} ({maxc} chars)")
                else:
                    exp = expect_char(t, lines, b, u, lines[li][0] + co)
                if isinstance(exp, dict):
                    if not r["ok"] or r["result"] != exp:
                        mism += 1; msgs.append(("lc", repr(t), li, co, r, exp))
                elif r["ok"] or r["error"]["kind"] != exp[0] or r["error"]["message"] != "invalid position: " + exp[1]:
                    mism += 1; msgs.append(("lc-ref", repr(t), li, co, r, exp))
        # round trip: every returned form gives the same five numbers
        for v in valid:
            for form in ({"byte": v["byte"]}, {"utf16": v["utf16"]},
                         {"line_col": {"line": v["line"], "col": v["col"]}}):
                cases += 1
                r = c.req("doc.position", doc=d, at=form)
                if not r["ok"] or r["result"] != v:
                    mism += 1; msgs.append(("rt", repr(t), form, r, v))
        # read-only: text and history unchanged
        info2 = c.ok("doc.info", doc=d)
        cases += 1
        if info2 != info:
            mism += 1; msgs.append(("info-changed", info, info2))
    rc = c.close()
    print(f"seed={seed} texts={n} cases={cases} mismatches={mism} control_rc={rc}")
    for m in msgs[:25]:
        print(json.dumps(m, ensure_ascii=False))


main()
