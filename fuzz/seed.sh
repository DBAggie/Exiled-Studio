#!/usr/bin/env bash
# Seeds fuzz/corpus/<target>/ from the repository's public fixtures (Phase 5,
# P6; docs/phase-5/security.md), so each run starts from real documents:
#   bash fuzz/seed.sh
# Run from the repository root. The packs and feed are signed with the
# public test key, which the targets trust.
set -euo pipefail
corpus=fuzz/corpus
mkdir -p "$corpus"/{pob_xml,share_code,build_document,signature,install_pack,data_feed}
fixture=crates/loom-data/tests/fixtures/bundle-poe1
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

cp tools/oracle/fixtures/*.xml "$corpus/pob_xml/"
cp crates/loom-serialization/src/dto/v1/testdata/*.json "$corpus/build_document/"

release() { cargo run -q --locked -p loom-release -- "$@"; }
release pack "$fixture" --test-key --out "$corpus/install_pack/signed.loombundle" > /dev/null
release pack "$fixture" --unsigned --out "$corpus/install_pack/unsigned.loombundle" > /dev/null
release test-sign "$fixture/bundle.json" --out "$work/manifest.sig"
release feed --sequence 2 --base-url https://example.invalid/download/v0 \
  --out "$work/feed.json" "$corpus/install_pack/signed.loombundle"
release test-sign "$work/feed.json" --out "$work/feed.sig"

# Share codes of the XML fixtures, and the two-part inputs
# (<first length: u16 LE><first><second>).
python3 - "$corpus" "$fixture/bundle.json" "$work" <<'PY'
import base64, pathlib, sys, zlib
corpus, manifest, work = pathlib.Path(sys.argv[1]), pathlib.Path(sys.argv[2]), pathlib.Path(sys.argv[3])
for xml in sorted((corpus / "pob_xml").glob("*.xml")):
    code = base64.urlsafe_b64encode(zlib.compress(xml.read_bytes(), 9))
    (corpus / "share_code" / (xml.stem + ".txt")).write_bytes(code)
def pair(first, second, out):
    out.write_bytes(len(first).to_bytes(2, "little") + first + second)
pair(manifest.read_bytes(), (work / "manifest.sig").read_bytes(), corpus / "signature" / "signed")
pair((work / "feed.json").read_bytes(), (work / "feed.sig").read_bytes(), corpus / "data_feed" / "signed")
PY
for d in "$corpus"/*/; do echo "$d: $(ls "$d" | wc -l) seeds"; done
