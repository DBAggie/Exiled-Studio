"""Every custom modifier line of PoB build files, one per line, for
pob_parse_lines.lua (ADR-015 Amendment 2; Phase 3 close-out). Original
code of this project (MIT). Maintainer-run, not in CI.

    python -I custom_lines.py <out.txt> <glob>...

Reads each file's custom modifier blocks (`CustomModifierBlock` text)
and its legacy `customMods` input (the raw attribute value: PoB's XML
reader keeps its line breaks, which an XML parser's attribute
normalization would not), splits them at line feeds as PoB does
(Classes/ConfigTab.lua 1113, 1132), and writes each distinct non-empty
line as written (pob_parse_lines.lua strips escapes and trims as PoB),
sorted.
"""
import glob
import html
import re
import sys
import xml.etree.ElementTree as ET


def main():
    lines = set()
    for pattern in sys.argv[2:]:
        for path in sorted(glob.glob(pattern)):
            try:
                root = ET.parse(path).getroot()
            except ET.ParseError:
                continue
            texts = [b.text or '' for b in root.iter('CustomModifierBlock')]
            with open(path, encoding='utf-8', errors='replace') as f:
                raw = f.read()
            for m in re.finditer(r'<Input\b[^>]*>', raw, re.S):
                tag = m.group(0)
                if 'name="customMods"' in tag:
                    s = re.search(r'string="([^"]*)"', tag, re.S)
                    if s:
                        texts.append(html.unescape(s.group(1)))
            for t in texts:
                for line in t.split('\n'):
                    if line.strip():
                        lines.add(line.rstrip('\r'))
    with open(sys.argv[1], 'w', encoding='utf-8', newline='\n') as f:
        for s in sorted(lines):
            f.write(s + '\n')
    print(len(lines))


if __name__ == '__main__':
    main()
