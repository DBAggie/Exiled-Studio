#!/usr/bin/env bash
# Instruction counts of loom-bench's scenarios under callgrind (Phase 5, P7;
# docs/phase-5/performance.md), against tools/bench/baseline.json:
#
#   bash tools/bench/count.sh            # fails on a count 10% over its baseline
#   bash tools/bench/count.sh --update   # writes the counts as the baseline
#
# Linux with valgrind (CI's `bench` job). Only the measured functions are
# counted (--toggle-collect), so reading the inputs is not. Counts are
# stable across runners, unlike time; the baseline is CI's own (ubuntu-24.04
# image, the pinned toolchain), so update it from a CI run's log when a
# change is meant to cost more, and say so in the PR.
set -euo pipefail
root=$(cd "$(dirname "$0")/../.." && pwd)
cd "$root"
update=${1:-}
cargo build --release --locked -p loom-bench
out=$(mktemp -d)
trap 'rm -rf "$out"' EXIT
: > "$out/counts"
for scenario in validate load import calculate allocate; do
  valgrind --tool=callgrind --toggle-collect='*measured_*' \
    --callgrind-out-file="$out/$scenario.out" \
    target/release/loom-bench run "$scenario" > /dev/null 2> "$out/$scenario.log" \
    || { cat "$out/$scenario.log"; exit 1; }
  count=$(awk '/^(summary|totals):/ { print $2; exit }' "$out/$scenario.out")
  if [ -z "$count" ] || [ "$count" = 0 ]; then
    echo "$scenario: no instructions counted (did --toggle-collect match?)" >&2
    exit 1
  fi
  echo "$scenario $count" >> "$out/counts"
done
python3 - "$out/counts" tools/bench/baseline.json "$update" <<'PY'
import json, os, sys
counts_file, baseline_file, update = sys.argv[1], sys.argv[2], sys.argv[3] == "--update"
counts = {}
for line in open(counts_file):
    name, n = line.split()
    counts[name] = int(n)
doc = json.load(open(baseline_file))
base = doc.get("instructions", {})
if update:
    doc["instructions"] = counts
    with open(baseline_file, "w") as f:
        json.dump(doc, f, indent=2)
        f.write("\n")
    print(json.dumps(counts, indent=2))
    sys.exit(0)
failed = False
print(f"{'scenario':<10} {'instructions':>15} {'baseline':>15} {'change':>8}")
for name, n in counts.items():
    b = base.get(name)
    if b is None:
        print(f"{name:<10} {n:>15,} {'none':>15} {'':>8}")
        continue
    change = (n - b) / b * 100
    mark = ""
    if n > b * 1.10:
        failed, mark = True, "  over the 10% limit"
    elif n < b * 0.90:
        mark = "  more than 10% under: update the baseline"
    print(f"{name:<10} {n:>15,} {b:>15,} {change:>+7.1f}%{mark}")
missing = [n for n in counts if n not in base]
if missing:
    print("No baseline for " + ", ".join(missing) + ". Counts to commit:")
    print(json.dumps(counts, indent=2))
    # In CI a scenario without a baseline checks nothing: fail until the
    # counts above are committed.
    if os.environ.get("CI"):
        failed = True
sys.exit(1 if failed else 0)
PY
