"""Conversion requests from the public community builds' own specs
(docs/tree/parity.md, "Older versions"): one `tree-parity` request line per
spec saved on an older tree version, its class, ascendancy, first node as
the target, and its nodes below 65536 (cluster nodes need their jewels).
Writes <out-dir>/spec-requests-<version>.txt for each version found.

    python tools/tree/spec_requests.py <out-dir> [version ...]

Without versions: 3_25 3_26 3_27 3_28.
"""
import glob
import re
import sys


def main() -> None:
    out_dir = sys.argv[1]
    versions = sys.argv[2:] or ["3_25", "3_26", "3_27", "3_28"]
    found: dict[str, list[str]] = {}
    for path in sorted(glob.glob("tools/oracle/fixtures/community/*.xml")):
        with open(path, encoding="utf-8") as f:
            text = f.read()
        for m in re.finditer(r"<Spec\b([^>]*)>", text):
            attrs = dict(re.findall(r'(\w+)="([^"]*)"', m.group(1)))
            version = attrs.get("treeVersion")
            if version not in versions:
                continue
            nodes = sorted({int(n) for n in attrs.get("nodes", "").split(",") if n and int(n) < 65536})
            if not nodes:
                continue
            found.setdefault(version, []).append(
                f"{attrs.get('classId', '0')} {attrs.get('ascendClassId', '0')} {nodes[0]} {','.join(map(str, nodes))}"
            )
    for version, lines in sorted(found.items()):
        with open(f"{out_dir}/spec-requests-{version}.txt", "w", encoding="utf-8", newline="\n") as f:
            f.write("".join(line + "\n" for line in lines))
        print(f"{version}: {len(lines)} specs")


if __name__ == "__main__":
    main()
