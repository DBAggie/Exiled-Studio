#!/usr/bin/env bash
# ADR-012 evidence rule: a PR body (read from stdin) must have content under each
# evidence heading of .github/pull_request_template.md. HTML comments, including
# multi-line ones, and blank lines do not count as content.
# Exit 0 when every section is filled, 1 otherwise (with a GitHub error annotation per gap).
set -euo pipefail

sections=("Tests" "Parity / divergence" "Coverage touched" "Handoff")

# Normalise line endings and strip all HTML comments (non-greedy, across lines).
body=$(tr -d '\r' | perl -0pe 's/<!--.*?-->//gs')

missing=0
for section in "${sections[@]}"; do
  # Text under the heading, up to the next "## ", without blank lines.
  content=$(printf '%s\n' "$body" \
    | awk -v h="## $section" '{sub(/[[:space:]]+$/, "")} $0==h{f=1;next} /^## /{f=0} f' \
    | grep -v '^[[:space:]]*$' || true)
  if [ -z "$content" ]; then
    echo "::error::PR description has no content under '## $section' (see .github/pull_request_template.md)"
    missing=1
  fi
done
exit "$missing"
