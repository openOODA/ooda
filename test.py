import re, sys
fn_start = re.compile(
    r"^(void|int|long long|OoStr|OoSList|OoIList|OoResS|OoResV) "
    r"([A-Za-z_][A-Za-z0-9_]*)\s*\(.*\)\s*\{\s*$"
)
for line in sys.stdin:
    if fn_start.match(line.rstrip('\n')):
        print("MATCH:", line.rstrip())
