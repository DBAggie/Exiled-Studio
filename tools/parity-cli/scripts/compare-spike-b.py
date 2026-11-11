"""Compare two resolve-corpus outputs on the Phase 2 plan's step 2 field list.

Usage: python -I tools/parity-cli/scripts/compare-spike-b.py <base-dir> <new-dir>
Exit 0 when every field is equal; prints each field's verdict.
Also reports whether report.json is byte-identical (informational).
"""
import json
import sys
from pathlib import Path

FIELDS = [
    "non_unique", "unique", "rate", "resolved_kinds", "adapted", "ambiguous",
    "categories", "mod_backed", "items", "items_by_rarity", "items_by_scope",
    "line_kinds",
]


def main() -> int:
    base, new = (Path(a) for a in sys.argv[1:3])
    a = json.loads((base / "summary.json").read_text(encoding="utf-8"))
    b = json.loads((new / "summary.json").read_text(encoding="utf-8"))
    bad = 0
    for f in FIELDS:
        if f not in a:
            print(f"{f}: MISSING in base")
            bad += 1
            continue
        same = a.get(f) == b.get(f)
        print(f"{f}: {'equal' if same else 'DIFFERENT'}")
        bad += not same
    others = sorted(k for k in set(a) | set(b) if k not in FIELDS and a.get(k) != b.get(k))
    print("other summary keys that differ:", others)
    rb = (base / "report.json").read_bytes() == (new / "report.json").read_bytes()
    print("report.json byte-identical:", rb)
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
