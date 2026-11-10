#!/usr/bin/env bash
# Acquires the pinned PoB oracle (Spike C) on Linux: fetches PoB, LuaJIT and
# luautf8 at the commits in manifest.json, builds LuaJIT and the lua-utf8
# module into <oracle-dir>/bin, then writes <oracle-dir>/build.json.
#
# Maintainer and CI script. It is the only oracle step that uses the network
# (git fetch); loom-parity-cli never fetches. Nothing is installed outside
# <oracle-dir>. Needs git, make, a C compiler (cc), python3 (reads the
# manifest, writes build.json), sha256sum. Check the result with
# `loom-parity-cli oracle-verify`.
#
# usage: tools/oracle/acquire.sh [<manifest> [<oracle-dir>]]
set -euo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
manifest="${1:-$here/manifest.json}"
oracle="${2:-$here/../../local/oracle}"

# Prints a manifest field (dotted path); a list prints one item per line.
field() {
  python3 -I -c '
import json, sys
value = json.load(open(sys.argv[1], encoding="utf-8"))
for key in sys.argv[2].split("."):
    value = value[key]
print("\n".join(value) if isinstance(value, list) else value)
' "$manifest" "$1"
}

# Fetches commit $2 of $1 into $3 (shallow, by SHA) and checks it out.
# A checkout already at the commit is left as it is (oracle-verify reports
# a dirty tree).
get_pinned() {
  local url="$1" commit="$2" dir="$3" head
  if [ ! -e "$dir/.git" ]; then
    mkdir -p "$dir"
    git -C "$dir" init -q
    git -C "$dir" config core.autocrlf false
    git -C "$dir" remote add origin "$url"
  fi
  git -C "$dir" remote set-url origin "$url"
  head="$(git -C "$dir" rev-parse --verify -q 'HEAD^{commit}' || true)"
  if [ "$head" != "$commit" ]; then
    git -C "$dir" fetch -q --depth 1 origin "$commit"
    git -C "$dir" -c advice.detachedHead=false checkout -q --force "$commit"
  fi
  head="$(git -C "$dir" rev-parse 'HEAD^{commit}')"
  if [ "$head" != "$commit" ]; then
    echo "$dir is at $head, expected $commit" >&2
    exit 1
  fi
}

[ "$(field format)" = 1 ] || { echo "unsupported manifest format" >&2; exit 1; }
mkdir -p "$oracle"
oracle="$(cd "$oracle" && pwd)"
luajit="$oracle/src/luajit"
utf8="$oracle/src/luautf8"
bin="$oracle/bin"

get_pinned "$(field pob.repository)" "$(field pob.commit)" "$oracle/pob"
get_pinned "$(field luajit.repository)" "$(field luajit.commit)" "$luajit"
get_pinned "$(field luautf8.repository)" "$(field luautf8.commit)" "$utf8"

# Fresh build trees at exactly the pinned sources (both are ours, under
# local/): an edited tracked file would otherwise be built and recorded as
# the pinned commit.
git -C "$luajit" -c advice.detachedHead=false checkout -q --force "$(field luajit.commit)"
git -C "$luajit" clean -q -fdx
git -C "$utf8" -c advice.detachedHead=false checkout -q --force "$(field luautf8.commit)"
git -C "$utf8" clean -q -fdx
rm -rf "$bin"
mkdir -p "$bin"

xcflags=""
while IFS= read -r define; do
  [ -n "$define" ] && xcflags="$xcflags -D$define"
done < <(field luajit.defines)
xcflags="${xcflags# }"

# The luajit executable links LuaJIT statically and exports its symbols, so
# the module needs no library to link against (as with luarocks' builtin
# build type, which luautf8's rockspec uses).
make -C "$luajit" -j"$(nproc)" XCFLAGS="$xcflags"
cc -O2 -fPIC -shared -I"$luajit/src" -o "$utf8/lua-utf8.so" "$utf8/lutf8lib.c"

cp "$luajit/src/luajit" "$bin/luajit"
cp "$utf8/lua-utf8.so" "$bin/lua-utf8.so"

python3 -I - "$manifest" "$oracle" "$(cc --version | head -n 1)" <<'EOF'
import hashlib, json, platform, sys
manifest_path, oracle, toolchain = sys.argv[1:4]
manifest = json.load(open(manifest_path, encoding="utf-8"))
files = {}
for name in ("bin/luajit", "bin/lua-utf8.so"):
    with open(f"{oracle}/{name}", "rb") as f:
        files[name] = hashlib.sha256(f.read()).hexdigest()
record = {
    "format": 1,
    "platform": "linux-" + platform.machine(),
    "luajit_commit": manifest["luajit"]["commit"],
    "luautf8_commit": manifest["luautf8"]["commit"],
    "defines": manifest["luajit"]["defines"],
    "toolchain": toolchain,
    "files": files,
}
text = json.dumps(record, indent=2) + "\n"
with open(f"{oracle}/build.json", "w", encoding="utf-8", newline="\n") as f:
    f.write(text)
sys.stdout.write(text)
EOF
