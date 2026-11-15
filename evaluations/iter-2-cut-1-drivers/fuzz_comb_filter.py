"""Re-run fuzz.py sessions of the `comb` alphabet with the colour check skipped for rows whose first
char is a combining mark (unicodedata.combining != 0); counts what remains. usage: <seed> <sessions> <steps>"""
import sys, os, unicodedata
sys.path.insert(0, os.path.dirname(__file__))
import fuzz
orig = fuzz.check_colours
skipped = [0]
def cc(rows, line_start, line_text, kinds, plain_only):
    if line_text and unicodedata.combining(line_text[0]):
        skipped[0] += 1
        return []
    return orig(rows, line_start, line_text, kinds, plain_only)
fuzz.check_colours = cc
sys.argv = ["fuzz.py", sys.argv[1], sys.argv[2], sys.argv[3], "comb"]
fuzz.main()
print("rows skipped (first char a combining mark):", skipped[0])
