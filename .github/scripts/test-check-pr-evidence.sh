#!/usr/bin/env bash
# Tests for check-pr-evidence.sh. Run: bash .github/scripts/test-check-pr-evidence.sh
set -uo pipefail

dir=$(cd "$(dirname "$0")" && pwd)
check="$dir/check-pr-evidence.sh"
template="$dir/../pull_request_template.md"
failures=0

# expect <name> <expected exit code> <body>
expect() {
  local name=$1 want=$2 body=$3 got
  printf '%s' "$body" | bash "$check" >/dev/null
  got=$?
  if [ "$got" -eq "$want" ]; then
    echo "ok   $name"
  else
    echo "FAIL $name: exit $got, want $want"
    failures=$((failures + 1))
  fi
}

filled=$'## Summary\nx\n\n## Tests\ncargo test: ok\n\n## Parity / divergence\nNone: CI only\n\n## Coverage touched\nNone: CI only\n\n## Handoff\nSUCCESSOR-HANDOFF.md\n'

expect "unedited_template_should_fail" 1 "$(cat "$template")"
expect "filled_body_should_pass" 0 "$filled"
expect "crlf_body_should_pass" 0 "${filled//$'\n'/$'\r\n'}"
expect "empty_body_should_fail" 1 ""
expect "single_empty_section_should_fail" 1 "${filled/None: CI only$'\n\n'## Coverage/$'\n'## Coverage}"
expect "multiline_comment_only_should_fail" 1 "${filled/SUCCESSOR-HANDOFF.md/<!-- line one
line two -->}"
expect "text_after_comment_should_pass" 0 "${filled/SUCCESSOR-HANDOFF.md/<!-- hint --> SUCCESSOR-HANDOFF.md}"
# Quote the pattern: an unquoted leading "#" would anchor it at the start of the string.
spaced="${filled/'## Tests'/'## Tests  '}"
[ "$spaced" != "$filled" ] || { echo "FAIL trailing_space fixture did not change"; failures=$((failures + 1)); }
expect "trailing_space_heading_should_pass" 0 "$spaced"

[ "$failures" -eq 0 ] && echo "all passed" || { echo "$failures failed"; exit 1; }
