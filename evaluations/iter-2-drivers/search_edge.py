"""search.* hand-made cases: worked examples, limits, inline flags, anchors and CRLF, literal CRLF
rules, refusal order, typing group / undo-step rules, null params, read-only and logging.
usage: python3 -I search_edge.py"""
import sys, os, json
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from k2 import *


def S(text, cmds, label):
    lines = [req("doc.open", id=0, path="t.txt")] + [req(c, id=i + 1, **p) for i, (c, p) in enumerate(cmds)]
    rc, rs, err = mem(lines, [("t.txt", text)], name="se.jsonl")
    print(f"--- {label}  text={json.dumps(text, ensure_ascii=False)}")
    for (c, p), r in zip(cmds, rs[1:]):
        pp = {k: (v if not isinstance(v, str) or len(v) < 60 else v[:20] + f"...({len(v)} chars)") for k, v in p.items() if k != "doc"}
        out = r.get("result") if r["ok"] else r["error"]
        print(f"  {c} {json.dumps(pp, ensure_ascii=False)} -> {json.dumps(out, ensure_ascii=False)[:300]}")
    return rs


F = "search.find"; R = "search.replace"; T = ("doc.text", {"doc": 1}); U = ("doc.undo", {"doc": 1}); I = ("doc.info", {"doc": 1})
# worked examples of search.md
S("baaab", [(R, dict(doc=1, query="a*", regex=True, all=True, replacement="-")), T], "ex1")
S("a1 b2", [(R, dict(doc=1, query=r"(\w)(\d)", regex=True, all=True, replacement="$2$1")), T, U, T], "ex2")
S("a1 b2", [(R, dict(doc=1, query=r"(\w)(\d)", regex=True, all=True, replacement="$0-")), T], "ex3")
S("a1 b2", [(R, dict(doc=1, query=r"(\w)(\d)", regex=True, all=True, replacement=r)) for r in ["$3", "$x", "${1}a", "$1a", "${}", "${1", "$$1", "$"]] + [T], "ex4 + templates")
S("a1 b2", [(R, dict(doc=1, query="b", all=True, replacement="$1")), T], "ex5")
S("aaa b aa", [(R, dict(doc=1, query="aa", **{"from": 1}, replacement="x")), T], "ex6")
S("aa", [(F, dict(doc=1, query="a", regex=False, replacement="x"))], "replacement param on find")
S("aaa b aa", [(F, dict(doc=1, query="aa"))], "literal non-overlap")
S("baaab", [(F, dict(doc=1, query="a*", regex=True))], "a* iteration")
S("foo FOO fOo_x éFoo", [(F, dict(doc=1, query="Foo", case_sensitive=False)), (F, dict(doc=1, query="Foo", case_sensitive=False, whole_word=True))], "find example")
S("σ Σ ς ß ẞ ss K k K ı I i İ", [(F, dict(doc=1, query=q, case_sensitive=False)) for q in ["σ", "ß", "ss", "k", "i", "İ", "ı"]]
  + [(F, dict(doc=1, query=q, case_sensitive=False, regex=True)) for q in ["σ", "ß", "k", "i", "[a-z]"]], "case folding")
# anchors and CRLF
S("a\r\nb\rc\nd", [(F, dict(doc=1, query=q, regex=True)) for q in ["^", "$", ".", r"\r\n", "(?s).", "(?-m)^", "(?-m)$", "(?-R)$", "^$", r"\R"]], "anchors")
S("a\r\nb\n", [(F, dict(doc=1, query=q)) for q in ["\r\n", "\n", "\r", "b\n"]], "literal CRLF")
S("aA", [(F, dict(doc=1, query=q, regex=True, case_sensitive=cs)) for q, cs in [("(?i)a", True), ("(?-i)a", False), ("a(?i)a", True), ("(?i:a)A", False)]], "inline flags")
# whole word
S("ab a_b a1 é a-b ab", [(F, dict(doc=1, query=q, whole_word=True, regex=rg)) for q, rg in [("a", False), ("ab", False), ("b", False), (r"a\w*", True), ("", False), ("a|ab", True)]], "whole word")
S("aab ab", [(F, dict(doc=1, query="ab", whole_word=True))], "no re-scan after rejected match")
S("x́ x٠ ٠x", [(F, dict(doc=1, query="x", whole_word=True))], "combining mark / Arabic digit next to match")
# limits
S("ab", [(F, dict(doc=1, query="a" * 1000)), (F, dict(doc=1, query="a" * 1001)), (F, dict(doc=1, query="é" * 1001)),
          (F, dict(doc=1, query="a" * 1001, regex=True)), (F, dict(doc=1, query="()" * 100, regex=True)),
          (F, dict(doc=1, query="()" * 101, regex=True)), (F, dict(doc=1, query="(?P<n>a)" + "()" * 100, regex=True)),
          (F, dict(doc=1, query="(?:" * 250 + "a" + ")" * 250, regex=True)), (F, dict(doc=1, query="(?:" * 251 + "a" + ")" * 251, regex=True)),
          (F, dict(doc=1, query="a{1000}{1000}", regex=True)), (F, dict(doc=1, query=r"\p{Nope}", regex=True)),
          (F, dict(doc=1, query="(", regex=True)), (F, dict(doc=1, query="", regex=True)), (F, dict(doc=1, query="")),
          (F, dict(doc=1, query=r"\w{1000}", regex=True)), (F, dict(doc=1, query=r"(?:\w{100}){100}", regex=True))], "limits")
# params
S("ab", [(F, dict(doc=1, query="a", limit=0)), (F, dict(doc=1, query="a", limit=100001)), (F, dict(doc=1, query="a", limit=100000)),
          (F, dict(doc=1, query="a", regex=None)), (F, dict(doc=1, query="a", whole_word=None)), (F, dict(doc=1, query="a", case_sensitive=None)),
          (F, dict(doc=1, query="a", zz=1)), (F, dict(doc=1, query="a", all=True)), (R, dict(doc=1, query="a", replacement=None)),
          (R, dict(doc=1, query="a", replacement="b", **{"from": None})), (R, dict(doc=1, query="a")),
          (R, dict(doc=1, query="a", replacement="b", **{"from": -1}))], "params")
# refusal order
S("ab", [(F, dict(doc=9, query="")), (R, dict(doc=9, query="(", regex=True, replacement="$9")),
          (R, dict(doc=1, query="", regex=True, replacement="$9")), (R, dict(doc=1, query="(", regex=True, replacement="$9")),
          (R, dict(doc=1, query="()", regex=True, replacement="$9", **{"from": 99})), (R, dict(doc=1, query="a", replacement="b", **{"from": 99})),
          (R, dict(doc=1, query="zzz", regex=True, replacement="$9")), (R, dict(doc=1, query="a", replacement="b", all=True, **{"from": 99})), T], "refusal order")
S("a\r\nb", [(R, dict(doc=1, query="b", replacement="X", **{"from": 2})), T, (R, dict(doc=1, query="$", regex=True, replacement="E", **{"from": 2})), T], "from between CR and LF")
# typing group / undo steps
S("xyz", [("doc.insert", dict(doc=1, offset=0, text="a")), ("doc.insert", dict(doc=1, offset=1, text="b")),
          (R, dict(doc=1, query="q", replacement="Q", all=True)), ("doc.insert", dict(doc=1, offset=2, text="c")), U, T], "no match keeps typing group")
S("xyz", [("doc.insert", dict(doc=1, offset=0, text="a")), (R, dict(doc=1, query="q*", regex=True, replacement="", all=True)),
          ("doc.insert", dict(doc=1, offset=1, text="b")), U, T, I], "empty matches -> empty: no edit")
S("xyz", [("doc.insert", dict(doc=1, offset=0, text="a")), (R, dict(doc=1, query="x", replacement="x", all=True)),
          ("doc.insert", dict(doc=1, offset=1, text="b")), U, T, U, T, I, U, T, I], "identical replacement is a step")
S("xyz", [(R, dict(doc=1, query="q*", regex=True, replacement="", all=False)), I, (R, dict(doc=1, query="q*", regex=True, replacement="-", all=False, **{"from": 3})), T, I], "single empty match")
# selection through replace
S("ab ab ab", [("cursor.set", dict(doc=1, anchor=3, head=5)), (R, dict(doc=1, query="ab", replacement="XYZ", all=True)), ("cursor.get", dict(doc=1)),
               U, ("cursor.get", dict(doc=1))], "selection maps")
S("a\nb", [("cursor.set", dict(doc=1, anchor=1, head=2)), (R, dict(doc=1, query="a", replacement="x\r", all=True)), ("cursor.get", dict(doc=1)), T], "CRLF snap")
S("abc", [("cursor.set", dict(doc=1, anchor=1)), (F, dict(doc=1, query="b")), ("cursor.get", dict(doc=1)), I], "find is read-only")
