"""Pick the oracle fixtures a CI run needs (maintainer request, 2026-11-24:
"only run what is necessary for a PR").

The oracle job runs PoB itself on each fixture (three runs per OS, then
Windows against Linux). A fixture's PoB output changes only when the fixture
or the oracle does, never with the engine. So a pull request runs:

- the smoke set: every fixture that is not a mechanic or community fixture
  (PoB's own test builds, the synthetic fixtures, the intentional
  discrepancy), so the oracle and the cross-platform comparison are always
  exercised;
- every fixture the pull request adds, or whose manifest entry it changes
  (a changed file changes its sha256 or blob);
- every fixture when the pull request touches the oracle itself
  (FULL_PATHS) or carries the label `oracle-full`;
- every fixture when the base branch cannot be compared with (RV-26: a
  stacked pull request whose base the checkout has not fetched is fetched
  once, and if that fails too the choice is the safe one).

This script is not one of FULL_PATHS: it picks fixtures and cannot change
PoB's output for one, and a wrong pick is caught by the full run of the
push to dev. While it was, every push of a pull request that edited it ran
every fixture (about 132 minutes on Windows).

Any other event (a push to dev, experimental, or main; a manual run) runs
every fixture.

usage: python3 -I oracle-select.py <event> <base-ref> <labels-json> <out-manifest>
Prints the manifest to pass to oracle-run and writes the choice to
$GITHUB_STEP_SUMMARY when set.
"""
import json
import os
import subprocess
import sys

MANIFEST = 'tools/oracle/fixtures/fixtures.json'
SMOKE_TIERS = {'pob-test', 'synthetic', 'discrepancy'}
# Changes that can change PoB's output for every fixture.
FULL_PATHS = (
    'tools/oracle/manifest.json',
    'tools/oracle/acquire.',
    'tools/oracle/adapter/',
    'tools/oracle/schema/',
    'tools/parity-cli/src/oracle',
    'rust-toolchain.toml',
)


def git(*args):
    return subprocess.run(['git', *args], check=True, capture_output=True, text=True).stdout


def changed_since(base):
    """The paths the pull request changes against `origin/<base>`, or None
    when the base cannot be compared with, even after fetching it."""
    diff = ('diff', '--name-only', f'origin/{base}...HEAD')
    try:
        return git(*diff).split()
    except subprocess.CalledProcessError:
        pass
    try:
        git('fetch', '--no-tags', 'origin', f'+refs/heads/{base}:refs/remotes/origin/{base}')
        return git(*diff).split()
    except subprocess.CalledProcessError:
        return None


def summary(text):
    path = os.environ.get('GITHUB_STEP_SUMMARY')
    if path:
        with open(path, 'a', encoding='utf-8') as f:
            f.write(text + '\n')


def main():
    event, base, labels, out = sys.argv[1:5]
    if event != 'pull_request':
        summary(f'Oracle fixtures: all ({event}).')
        print(MANIFEST)
        return
    if 'oracle-full' in json.loads(labels or '[]'):
        summary('Oracle fixtures: all (label `oracle-full`).')
        print(MANIFEST)
        return
    changed = changed_since(base)
    if changed is None:
        summary(f'Oracle fixtures: all (the base `{base}` could not be compared with).')
        print(MANIFEST)
        return
    full = sorted({p for p in changed if p.startswith(FULL_PATHS)})
    if full:
        summary('Oracle fixtures: all (the oracle changed: ' + ', '.join(f'`{p}`' for p in full) + ').')
        print(MANIFEST)
        return
    head = json.load(open(MANIFEST, encoding='utf-8'))
    try:
        before = json.loads(git('show', f'origin/{base}:{MANIFEST}'))
    except subprocess.CalledProcessError:
        before = {'fixtures': []}
    old = {f['id']: f for f in before['fixtures']}
    smoke, new = [], []
    for f in head['fixtures']:
        if f.get('tier') in SMOKE_TIERS:
            smoke.append(f)
        elif old.get(f['id']) != f:
            new.append(f)
    picked = {'format': head['format'], 'fixtures': smoke + new}
    with open(out, 'w', encoding='utf-8', newline='\n') as fh:
        json.dump(picked, fh, indent=2)
        fh.write('\n')
    lines = [f'Oracle fixtures: {len(picked["fixtures"])} of {len(head["fixtures"])} '
             f'(smoke set {len(smoke)}, added or changed by this pull request {len(new)}).']
    lines += [f'- `{f["id"]}`' for f in new]
    summary('\n'.join(lines))
    print(out)


if __name__ == '__main__':
    main()
