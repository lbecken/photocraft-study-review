"""Models for syntax.* built only from the published S-expression (`syntax.tree`.sexp) plus the
text: a parser of the sexp format of docs/protocol/syntax.md, a highlight model of rules 1-7, and
a bracket-matching model. Anonymous leaves are not in the sexp, so the model infers them: a char
not covered by a named token node is part of an anonymous leaf."""
import re

KEYWORDS = set("""as async await break const continue crate dyn else enum extern false fn for gen if impl in let
loop match mod move mut pub ref return self Self static struct super trait true type unsafe use where while yield
abstract become box do final macro override priv typeof unsized virtual try union default safe raw auto""".split())
TOKEN_KINDS = {"line_comment", "block_comment", "string_literal", "raw_string_literal", "char_literal",
               "string_content", "escape_sequence", "doc_comment", "identifier", "type_identifier",
               "field_identifier", "primitive_type", "integer_literal", "float_literal", "shorthand_field_identifier",
               "metavariable", "label", "lifetime", "self", "crate", "super", "mutable_specifier",
               "inner_doc_comment_marker", "outer_doc_comment_marker", "fragment_specifier"}


class Node:
    __slots__ = ("kind", "s", "e", "field", "kids", "missing", "parent")

    def __init__(self, kind, s, e, field=None, missing=False):
        self.kind, self.s, self.e, self.field, self.kids, self.missing, self.parent = kind, s, e, field, [], missing, None


def parse_sexp(t):
    i = 0; n = len(t)
    stack = [Node("ROOT", 0, 10 ** 12)]
    pending_field = None
    while i < n:
        c = t[i]
        if c == " ":
            i += 1; continue
        if c == ")":
            stack.pop(); i += 1; continue
        if c == "(":
            i += 1
            if t.startswith("MISSING ", i):
                i += 8
                if t[i] == '"':
                    j = t.index('"@', i + 1); kind = t[i:j + 1]; i = j + 1
                else:
                    j = t.index("@", i); kind = t[i:j]; i = j
                m = re.compile(r"@(\d+)\.\.(\d+)").match(t, i)
                nd = Node(kind, int(m.group(1)), int(m.group(2)), pending_field, True)
                i = m.end()
            else:
                m = re.compile(r"([A-Za-z_][A-Za-z_0-9]*)@(\d+)\.\.(\d+)").match(t, i)
                nd = Node(m.group(1), int(m.group(2)), int(m.group(3)), pending_field)
                i = m.end()
            pending_field = None
            nd.parent = stack[-1]; stack[-1].kids.append(nd); stack.append(nd)
            continue
        m = re.compile(r"([a-z_][a-z_0-9]*):").match(t, i)
        if m:
            pending_field = m.group(1); i = m.end(); continue
        raise ValueError(f"bad sexp at {i}: {t[i:i+40]!r}")
    return stack[0].kids[0]


def walk(nd):
    yield nd
    for k in nd.kids:
        yield from walk(k)


def innermost(root, p):
    """innermost non-missing named node whose [s,e) contains p"""
    nd = root
    while True:
        for k in nd.kids:
            if not k.missing and k.s <= p < k.e:
                nd = k; break
        else:
            return nd


def paint(root, L):
    arr = [root] * L
    def rec(nd):
        if not nd.missing:
            for q in range(max(nd.s, 0), min(nd.e, L)):
                arr[q] = nd
        for k in nd.kids:
            rec(k)
    rec(root)
    return arr


def ancestors(nd):
    while nd is not None and nd.kind != "ROOT":
        yield nd
        nd = nd.parent


def callee_name(nd):
    """resolve a call_expression's `function` field (or generic_function's) to the identifier node"""
    if nd is None: return None
    if nd.kind == "identifier": return nd
    if nd.kind == "scoped_identifier":
        return next((k for k in nd.kids if k.field == "name"), None)
    if nd.kind == "field_expression":
        return next((k for k in nd.kids if k.field == "field"), None)
    if nd.kind == "generic_function":
        return callee_name(next((k for k in nd.kids if k.field == "function"), None))
    return None


def highlight_model(text, root):
    L = len(text)
    kinds = ["plain"] * L
    fn_ids = set()
    for nd in walk(root):
        if nd.kind in ("function_item", "function_signature_item"):
            for k in nd.kids:
                if k.field == "name" and k.kind == "identifier": fn_ids.add((k.s, k.e))
        if nd.kind == "call_expression":
            f = next((k for k in nd.kids if k.field == "function"), None)
            c = callee_name(f)
            if c is not None and c.kind in ("identifier", "field_identifier"): fn_ids.add((c.s, c.e))
    P = paint(root, L)
    def innermost(_r, q): return P[q]
    for p in range(L):
        inn = P[p]
        anc = [a.kind for a in ancestors(inn)]
        if any(k in ("line_comment", "block_comment") for k in anc):
            kinds[p] = "comment"; continue
        if any(k in ("string_literal", "raw_string_literal", "char_literal") for k in anc):
            kinds[p] = "string"; continue
        if any(k in ("integer_literal", "float_literal") for k in anc):
            kinds[p] = "number"; continue
        if inn.kind in ("self", "crate", "super", "mutable_specifier"):
            kinds[p] = "keyword"; continue
        if inn.kind not in TOKEN_KINDS:
            # an anonymous leaf (or white space): keyword if its identifier-like run is a keyword
            if text[p].isalnum() or text[p] == "_" or text[p] == "!":
                a = p
                while a > 0 and (text[a - 1].isalnum() or text[a - 1] == "_") and innermost(root, a - 1) is inn: a -= 1
                b = p + 1 if text[p] != "!" else p
                while b < L and (text[b].isalnum() or text[b] == "_") and innermost(root, b) is inn: b += 1
                w = text[a:b]
                if text[p] == "!":
                    # part of `macro_rules!`
                    a2 = p
                    while a2 > 0 and (text[a2 - 1].isalnum() or text[a2 - 1] == "_"): a2 -= 1
                    if text[a2:p] == "macro_rules" and innermost(root, a2) is inn:
                        kinds[p] = "keyword"
                    continue
                if w in KEYWORDS or (w == "macro_rules" and text[b:b + 1] == "!"):
                    kinds[p] = "keyword"; continue
            continue
        if inn.kind in ("type_identifier", "primitive_type"):
            kinds[p] = "type"; continue
        if (inn.s, inn.e) in fn_ids and inn.kind in ("identifier", "field_identifier"):
            kinds[p] = "function"; continue
    return kinds


def spans_to_kinds(spans, start, end):
    out = []
    for sp in spans:
        out.extend([sp["kind"]] * (sp["end"] - sp["start"]))
    return out


def check_partition(spans, start, end):
    if start == end:
        return spans == []
    if not spans or spans[0]["start"] != start or spans[-1]["end"] != end:
        return False
    for a, b in zip(spans, spans[1:]):
        if a["end"] != b["start"] or a["kind"] == b["kind"]:
            return False
    return all(s["start"] < s["end"] for s in spans)


def kinds_to_spans(kinds, start, end):
    out = []
    for p in range(start, end):
        k = kinds[p]
        if out and out[-1]["kind"] == k and out[-1]["end"] == p:
            out[-1]["end"] = p + 1
        else:
            out.append({"start": p, "end": p + 1, "kind": k})
    return out


def brackets_model(text, root):
    """offset -> (open, close) for matched brackets"""
    pairs = {"(": ")", "[": "]", "{": "}"}
    closers = {v: k for k, v in pairs.items()}
    groups = {}
    P = paint(root, len(text))
    def innermost(_r, q): return P[q]
    for p, ch in enumerate(text):
        if ch in pairs or ch in closers:
            inn = innermost(root, p)
            if any(a.kind in ("line_comment", "block_comment", "string_literal", "raw_string_literal", "char_literal")
                   for a in ancestors(inn)):
                continue
            if inn.kind in TOKEN_KINDS:
                continue
            groups.setdefault(id(inn), []).append((p, ch))
    res = {}
    for lst in groups.values():
        for o, c in pairs.items():
            st = []
            for p, ch in lst:
                if ch == o: st.append(p)
                elif ch == c and st:
                    q = st.pop(); res[q] = (q, p); res[p] = (q, p)
    return res
