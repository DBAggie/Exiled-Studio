// JavaScript dependency licenses (Phase 4 plan, gate 5): every package
// `pnpm licenses list` reports must have an open-source license on the
// allow-list, or be a named exception with its reason (AGENTS.md,
// "Dependencies"; the Rust side's deny.toml has the same rule). Exit 1 on
// anything else.
//
// The allow-list is OSI-approved licenses (maintainer decision 2026-11-13:
// "licenses that are still considered open source"), plus CC0-1.0, a
// public-domain dedication. Strong copyleft (GPL, AGPL, LGPL, and the
// like) is not on it: ADR-005 keeps GPL-derived code out of the shipped
// app, so such a package fails here and needs the maintainer.
import { execFileSync } from 'node:child_process'

export const ALLOWED = new Set([
  // Permissive.
  '0BSD',
  'Apache-2.0',
  'Artistic-2.0',
  'BlueOak-1.0.0',
  'BSD-2-Clause',
  'BSD-3-Clause',
  'BSL-1.0',
  'CC0-1.0',
  'ISC',
  'MIT',
  'MIT-0',
  'NCSA',
  'Python-2.0',
  'Unicode-3.0',
  'Unlicense',
  'UPL-1.0',
  'Zlib',
  // File-level copyleft: using an unmodified package is fine; its own
  // files stay under it.
  'EPL-2.0',
  'MPL-2.0',
  // Fonts: the SIL Open Font License lets a font be bundled with any
  // software (Inter, Space Grotesk, JetBrains Mono; the license texts
  // ship in public/licenses/).
  'OFL-1.1',
])

// Licenses that would bind the shipped app: never allowed by the list,
// reported with that reason.
const COPYLEFT = /^(A?GPL|LGPL|EUPL|OSL|SSPL|CPAL)-/

// Packages with a license outside the list, by name (a prefix covers the
// platform builds of one package), each with its reason.
const EXCEPTIONS = [
  {
    prefix: 'argparse',
    license: 'PSF-2.0',
    reason:
      'via json-schema-to-typescript (generates types; build tool, in the tree since Phase 2); the Python license as pnpm reports it',
  },
]

/** Whether an SPDX expression is allowed: `A OR B` needs one side, `A AND
 * B` both; `A WITH exception` is A (an exception only adds permissions);
 * parentheses are dropped, so `(A OR B) AND C` reads as `A OR (B AND C)`
 * (no such expression in the tree; the test pins the reading). */
export function allowed(expression) {
  const e = expression
    .replace(/[()]/g, '')
    .replace(/ WITH [^ ]+/g, '')
    .trim()
  if (/ OR /.test(e)) return e.split(/ OR /).some((x) => allowed(x))
  if (/ AND /.test(e)) return e.split(/ AND /).every((x) => allowed(x))
  return ALLOWED.has(e)
}

function main() {
  // The pnpm that runs this script (`pnpm licenses:check` sets
  // npm_execpath), called without a shell: a JavaScript entry point
  // through node, or pnpm's own executable.
  const pnpm = process.env.npm_execpath
  if (pnpm === undefined) {
    console.error('run this as: pnpm licenses:check')
    process.exit(2)
  }
  const list = ['licenses', 'list', '--json']
  const [file, args] = /\.[cm]?js$/.test(pnpm)
    ? [process.execPath, [pnpm, ...list]]
    : [pnpm, list]
  const out = execFileSync(file, args, {
    encoding: 'utf8',
    maxBuffer: 64 * 1024 * 1024,
  })
  const byLicense = JSON.parse(out)
  const failures = []
  let count = 0
  for (const [license, packages] of Object.entries(byLicense)) {
    for (const p of packages) {
      count += 1
      if (allowed(license)) continue
      const ok = EXCEPTIONS.some(
        (e) => e.license === license && p.name.startsWith(e.prefix),
      )
      if (ok) continue
      const why = COPYLEFT.test(license.replace(/[()]/g, ''))
        ? ' (copyleft: conflicts with ADR-005; ask the maintainer)'
        : ''
      failures.push(
        `${p.name}@${p.versions?.join(',') ?? '?'}: ${license}${why}`,
      )
    }
  }
  if (failures.length > 0) {
    console.error(`licenses not allowed:\n  ${failures.join('\n  ')}`)
    process.exit(1)
  }
  console.log(`licenses ok: ${count} packages`)
}

if (process.argv[1]?.endsWith('check-licenses.mjs')) main()
