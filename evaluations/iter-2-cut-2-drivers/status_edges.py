"""ui.status-bar deterministic cases. env: KILN_BIN, KILN_CLI, KILN_WORK."""
import json, os, sys
sys.path.insert(0, os.path.dirname(__file__))
import kl, k2
from k2 import Session, find, rows, status, nodes

ED = {"role": "multiline_text_input", "label": "Editor"}


def bar(root):
    return {k: (v.get("value"), rows(v), v.get("offscreen")) if isinstance(v, dict) else v for k, v in status(root).items()}


def main():
    d = kl.fresh("sedge")
    files = {"m.txt": "ab\nhello world\r\nx\ry", "chars.txt": "é‍ﬁ\t😀x\n", "u16le.txt": b"\xff\xfe" + "a\r\nb".encode("utf-16-le"),
             "u16be.txt": b"\xfe\xff" + "a\rb\r".encode("utf-16-be"), "bom.txt": b"\xef\xbb\xbfone", "e.txt": "",
             "rtl.txt": "שלום עולם\n", "tie.txt": "a\r\nb\nc\rd"}
    kl.write_files(d, files)
    reqs = [{"id": "s0", "ui": "snapshot"}]
    for k, p in enumerate(files):
        reqs += [{"id": f"o{k}", "command": "doc.open", "params": {"path": p}}, {"id": f"f{k}", "ui": "frames", "count": 1},
                 {"id": f"s{k}", "ui": "snapshot"}, {"id": f"i{k}", "command": "doc.info", "params": {"doc": k + 1}}]
    # example of ui-status.md: Ln 2, Col 6 in m.txt (doc 1)
    reqs += [{"id": "tm", "ui": "click", "target": {"role": "tab", "label": "m.txt"}}, {"id": "fe", "ui": "focus", "target": ED},
             {"id": "k1", "ui": "key", "key": "ArrowDown"}] + [{"id": f"r{j}", "ui": "key", "key": "ArrowRight"} for j in range(5)] + \
            [{"id": "sm", "ui": "snapshot"}, {"id": "cm", "command": "cursor.get", "params": {"doc": 1}},
             {"id": "sel", "ui": "key", "key": "ArrowLeft", "modifiers": ["shift"]}, {"id": "sel2", "ui": "key", "key": "ArrowUp", "modifiers": ["shift"]},
             {"id": "ss", "ui": "snapshot"}, {"id": "cs", "command": "cursor.get", "params": {"doc": 1}},
             {"id": "tc", "ui": "click", "target": {"role": "tab", "label": "chars.txt"}}, {"id": "fe2", "ui": "focus", "target": ED},
             {"id": "end", "ui": "key", "key": "End"}, {"id": "sc", "ui": "snapshot"}, {"id": "cc", "command": "cursor.get", "params": {"doc": 2}}]
    for col in range(8):
        reqs += [{"id": f"ck{col}", "ui": "click", "target": ED, "at": {"row": 0, "col": col}}, {"id": f"sk{col}", "ui": "snapshot"},
                 {"id": f"gk{col}", "command": "cursor.get", "params": {"doc": 2}}]
    reqs += [{"id": "cs1", "command": "cursor.set", "params": {"doc": 2, "anchor": 3}}, {"id": "snf", "ui": "snapshot"},
             {"id": "f1", "ui": "frames", "count": 1}, {"id": "sf", "ui": "snapshot"},
             {"id": "tr", "ui": "click", "target": {"role": "tab", "label": "rtl.txt"}}, {"id": "fe3", "ui": "focus", "target": ED},
             {"id": "end3", "ui": "key", "key": "End"}, {"id": "sr", "ui": "snapshot"},
             {"id": "nz", "command": "eol.normalize", "params": {"doc": 8, "to": "cr"}}, {"id": "f2", "ui": "frames", "count": 1}, {"id": "sn", "ui": "snapshot"},
             {"id": "cl", "command": "doc.close", "params": {"doc": 8}}, {"id": "f3", "ui": "frames", "count": 1}, {"id": "scl", "ui": "snapshot"},
             {"id": "ao", "command": "doc.open", "params": {"path": "u16le.txt"}}, {"id": "f4", "ui": "frames", "count": 1}, {"id": "sao", "ui": "snapshot"},
             {"id": "bc", "ui": "click", "target": {"role": "label", "label": "Encoding"}}, {"id": "bcf", "ui": "focus", "target": {"role": "label", "label": "Line"}},
             {"id": "scr", "ui": "scroll", "target": {"role": "label", "label": "Line"}, "direction": "down"},
             {"id": "sbc", "ui": "snapshot"}]
    s = Session(reqs, root=d, name="sedge")
    print("no document:", bar(s.snap("s0")))
    for k, p in enumerate(files):
        i = s[f"i{k}"]["result"]
        print(f"open {p!r}: bar {bar(s.snap(f's{k}'))}  doc.info {i['encoding']} {i['line_ending']} mixed={i['mixed_line_endings']}")
    print("example (m.txt, ArrowDown, 5 x ArrowRight):", bar(s.snap("sm")), s["cm"]["result"])
    print("selection (shift+Left, shift+Up):", bar(s.snap("ss")), s["cs"]["result"])
    print("chars End:", bar(s.snap("sc")), s["cc"]["result"])
    for col in range(8):
        print(f"  click col {col}:", s[f"ck{col}"].get("ok"), bar(s.snap(f"sk{col}"))["Column"][0], "cursor.get col", s[f"gk{col}"]["result"]["col"])
    print("control cursor.set, snapshot without a frame:", bar(s.snap("snf"))["Column"], "after one frame:", bar(s.snap("sf"))["Column"])
    print("rtl End:", bar(s.snap("sr")))
    print("tie.txt normalize to cr:", bar(s.snap("sn"))["Line ending"])
    print("close current (doc 8) -> highest id 7 rtl.txt:", bar(s.snap("scl")))
    print("already_open control open of u16le.txt:", s["ao"]["result"]["already_open"], bar(s.snap("sao")))
    print("click / focus / scroll on the bar:", s["bc"], s["bcf"], s["scr"], bar(s.snap("sbc")))
    print("rc", s.rc)


main()
