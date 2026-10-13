#!/usr/bin/env bash
# Run `cargo check -p uplink` through the gate and split the diagnostics per source file.
# Writes rust/.errors/<path with / replaced by __>.txt (one file per Rust source with errors)
# and rust/.errors/ALL.txt; prints "<count> <file>" lines plus a total.
cd "$(dirname "$(readlink -f "$0")")/.." || exit 1
rm -rf .errors && mkdir -p .errors
tools/cargo-gate check -p uplink --message-format json 2>/dev/null \
 | python3 -I -c '
import sys, json, collections, os
per = collections.defaultdict(list)
total = 0
for line in sys.stdin:
    try: m = json.loads(line)
    except Exception: continue
    if m.get("reason") != "compiler-message": continue
    msg = m["message"]
    if msg.get("level") != "error": continue
    spans = [s for s in msg.get("spans", []) if s.get("is_primary")] or msg.get("spans", [])
    f = spans[0]["file_name"] if spans else "<none>"
    per[f].append(msg.get("rendered") or msg.get("message"))
    total += 1
with open(".errors/ALL.txt", "w") as all_:
    for f, errs in sorted(per.items()):
        name = f.replace("/", "__")
        with open(os.path.join(".errors", name + ".txt"), "w") as out:
            out.write("\n".join(errs))
        all_.write("\n".join(errs) + "\n")
for f, errs in sorted(per.items(), key=lambda x: -len(x[1])):
    print(len(errs), f)
print("TOTAL", total)
'
