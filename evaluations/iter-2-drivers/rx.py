"""Independent model of search.* (docs/protocol/search.md): a backtracking regex interpreter over
a generated AST (rendered to `regex`-crate syntax), literal scan, simple case folding (from the
regex-syntax crate's published table, a third-party file), whole-word filter, replacement
templates."""
import os, re, glob, random

# --- simple case folding orbits, parsed from regex-syntax's generated table (third-party data)
_tab = glob.glob(os.path.expanduser("~/.cargo/registry/src/*/regex-syntax-0.8.11/src/unicode_tables/case_folding_simple.rs"))[0]
ORBIT = {}


def _ch(t):
    mm = re.fullmatch(r"\\u\{([0-9a-fA-F]+)\}", t)
    if mm: return chr(int(mm.group(1), 16))
    if t.startswith("\\"): return {"\\'": "'", "\\\\": "\\"}.get(t, t)
    return t


for line in open(_tab, encoding="utf-8"):
    m = re.match(r"\s*\('(.+?)', &\[(.*)\]\),\s*$", line)
    if m:
        c = _ch(m.group(1))
        others = [_ch(o) for o in re.findall(r"'(.+?)'", m.group(2))]
        ORBIT[c] = set(others) | {c}


def orb(c):
    return ORBIT.get(c, {c})


def ceq(a, b, ci):
    return a == b or (ci and b in orb(a))


def is_word(c):
    return c == "_" or c.isalnum()


# --- AST
def render(n):
    t = n[0]
    if t == "lit":
        c = n[1]
        return "\\" + c if c in ".^$*+?()[]{}|\\-" else ("\\r" if c == "\r" else "\\n" if c == "\n" else c)
    if t == "any": return "."
    if t == "cls":
        body = "".join((esc(a) if a == b else esc(a) + "-" + esc(b)) for a, b in n[2])
        return "[" + ("^" if n[1] else "") + body + "]"
    if t == "cat": return "".join(render(x) for x in n[1])
    if t == "alt": return "(?:" + "|".join(render(x) for x in n[1]) + ")"
    if t == "grp": return "(" + render(n[1]) + ")"
    if t == "rep":
        body = render(n[1])
        if n[1][0] in ("cat", "rep"): body = "(?:" + body + ")"
        mn, mx, greedy = n[2], n[3], n[4]
        if (mn, mx) == (0, None): q = "*"
        elif (mn, mx) == (1, None): q = "+"
        elif (mn, mx) == (0, 1): q = "?"
        elif mx is None: q = "{%d,}" % mn
        elif mn == mx: q = "{%d}" % mn
        else: q = "{%d,%d}" % (mn, mx)
        return body + q + ("" if greedy else "?")
    if t == "bol": return "^"
    if t == "eol": return "$"
    raise ValueError(n)


def esc(c):
    if c in "[]^-\\&~": return "\\" + c
    if c == "\r": return "\\r"
    if c == "\n": return "\\n"
    return c


def ngroups(n):
    t = n[0]
    if t in ("cat", "alt"): return sum(ngroups(x) for x in n[1])
    if t == "grp": return 1 + ngroups(n[1])
    if t == "rep": return ngroups(n[1])
    return 0


def number(n, ctr):
    """assign group numbers in order of opening paren: returns a new AST with ('grp', k, body)"""
    t = n[0]
    if t in ("cat", "alt"): return (t, [number(x, ctr) for x in n[1]])
    if t == "grp":
        ctr[0] += 1; k = ctr[0]
        return ("g", k, number(n[1], ctr))
    if t == "rep": return ("rep", number(n[1], ctr), n[2], n[3], n[4])
    return n


def cls_has(items, c):
    return any(a <= c <= b for a, b in items)


def m(n, s, i, caps, ci, k):
    """match node n at i; call continuation k(i, caps) -> result or None"""
    t = n[0]
    L = len(s)
    if t == "lit":
        return k(i + 1, caps) if i < L and ceq(n[1], s[i], ci) else None
    if t == "any":
        return k(i + 1, caps) if i < L and s[i] not in "\r\n" else None
    if t == "cls":
        if i >= L: return None
        cs = orb(s[i]) if ci else {s[i]}
        hit = any(cls_has(n[2], c) for c in cs)
        return k(i + 1, caps) if hit != n[1] else None
    if t == "cat":
        def go(j, idx, cp):
            if idx == len(n[1]): return k(j, cp)
            return m(n[1][idx], s, j, cp, ci, lambda jj, cc: go(jj, idx + 1, cc))
        return go(i, 0, caps)
    if t == "alt":
        for x in n[1]:
            r = m(x, s, i, caps, ci, k)
            if r is not None: return r
        return None
    if t == "g":
        def after(j, cp):
            cp2 = dict(cp); cp2[n[1]] = (i, j)
            return k(j, cp2)
        return m(n[2], s, i, caps, ci, after)
    if t == "rep":
        body, mn, mx, greedy = n[1], n[2], n[3], n[4]
        def loop(j, cnt, cp):
            can_more = mx is None or cnt < mx
            def more():
                if not can_more: return None
                return m(body, s, j, cp, ci, lambda jj, cc: loop(jj, cnt + 1, cc) if jj > j else None)
            if cnt < mn:
                return more()
            if greedy:
                r = more()
                return r if r is not None else k(j, cp)
            r = k(j, cp)
            return r if r is not None else more()
        return loop(i, 0, caps)
    if t == "bol":
        ok = i == 0 or s[i - 1] == "\n" or (s[i - 1] == "\r" and (i == L or s[i] != "\n"))
        return k(i, caps) if ok else None
    if t == "eol":
        ok = i == L or s[i] == "\r" or (s[i] == "\n" and (i == 0 or s[i - 1] != "\r"))
        return k(i, caps) if ok else None
    raise ValueError(t)


def regex_at(ast, s, i, ci):
    return m(ast, s, i, {}, ci, lambda j, cp: (j, cp))


def regex_iter(ast, s, ci):
    """docs/protocol/search.md Iteration: leftmost-first, non-overlapping; empty match at prev end discarded."""
    out = []
    pos = 0; last_end = None
    L = len(s)
    while pos <= L:
        found = None
        for st in range(pos, L + 1):
            r = regex_at(ast, s, st, ci)
            if r is not None:
                found = (st, r[0], r[1]); break
        if not found: break
        st, en, cp = found
        if st == en and last_end is not None and st == last_end:
            # discard, restart one char later
            pos = st + 1
            continue
        out.append(found)
        last_end = en
        pos = en if en > st else st + 1
    return out


def literal_iter(q, s, ci):
    out = []; i = 0; n = len(q)
    while i + n <= len(s):
        if all(ceq(q[j], s[i + j], ci) for j in range(n)):
            out.append((i, i + n, {})); i += n
        else:
            i += 1
    return out


def whole(ms, s):
    return [x for x in ms if (x[0] == 0 or not is_word(s[x[0] - 1])) and (x[1] == len(s) or not is_word(s[x[1]]))]


def parse_template(tpl):
    """-> list of ('lit', str) | ('ref', name)"""
    out = []; i = 0
    while i < len(tpl):
        c = tpl[i]
        if c != "$":
            out.append(("lit", c)); i += 1; continue
        if tpl[i + 1:i + 2] == "$":
            out.append(("lit", "$")); i += 2; continue
        if tpl[i + 1:i + 2] == "{":
            j = tpl.find("}", i + 2)
            if j < 0:
                out.append(("lit", "$")); i += 1; continue
            out.append(("ref", tpl[i + 2:j])); i = j + 1; continue
        mm = re.match(r"[0-9A-Za-z_]+", tpl[i + 1:])
        if mm:
            out.append(("ref", mm.group(0))); i += 1 + len(mm.group(0)); continue
        out.append(("lit", "$")); i += 1
    return out


def template_ok(parts, ng):
    for p in parts:
        if p[0] == "ref":
            nm = p[1]
            if not (nm.isascii() and nm.isdigit() and int(nm) <= ng):
                return False, nm
    return True, None


def expand(parts, s, st, en, cp):
    r = []
    for p in parts:
        if p[0] == "lit": r.append(p[1])
        else:
            k = int(p[1])
            if k == 0: r.append(s[st:en])
            elif k in cp: r.append(s[cp[k][0]:cp[k][1]])
    return "".join(r)
