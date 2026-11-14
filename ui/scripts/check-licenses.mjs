// JavaScript dependency licenses (Phase 4 plan, gate 5): every package
// `pnpm licenses list` reports must have an allowed license, or be a named
// exception with its reason (AGENTS.md, "Dependencies": the Rust side's
// deny.toml has the same rule). Exit 1 on anything else.
import { execFileSync } from 'node:child_process'

const ALLOWED = new Set([
  'MIT',
  'ISC',
  'Apache-2.0',
  'Apache-2.0 OR MIT',
  'MIT OR Apache-2.0',
  'BSD-2-Clause',
  'BSD-3-Clause',
  '0BSD',
  'BlueOak-1.0.0',
  'CC0-1.0',
  'Unlicense',
])

// Packages with another license, by name (a prefix covers the platform
// builds of one package). Each is a build or test tool: none is shipped in
// the application.
const EXCEPTIONS = [
  {
    prefix: 'lightningcss',
    license: 'MPL-2.0',
    reason:
      "Vite's CSS transformer (build tool, in the tree since the scaffold)",
  },
  {
    prefix: 'axe-core',
    license: 'MPL-2.0',
    reason:
      'accessibility checks in the UI tests (Phase 4); test-only. Awaiting the maintainer’s approval (Phase 4 report).',
  },
  {
    prefix: 'argparse',
    license: 'Python-2.0',
    reason:
      'via json-schema-to-typescript (generates types; build tool, in the tree since Phase 2)',
  },
  {
    prefix: 'argparse',
    license: 'PSF-2.0',
    reason: 'the same package, as pnpm reports it',
  },
]

// The pnpm that runs this script (`pnpm licenses:check` sets
// npm_execpath), called without a shell: a JavaScript entry point through
// node, or pnpm's own executable.
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
    if (ALLOWED.has(license)) continue
    const ok = EXCEPTIONS.some(
      (e) => e.license === license && p.name.startsWith(e.prefix),
    )
    if (!ok)
      failures.push(`${p.name}@${p.versions?.join(',') ?? '?'}: ${license}`)
  }
}
if (failures.length > 0) {
  console.error(`licenses not allowed:\n  ${failures.join('\n  ')}`)
  process.exit(1)
}
console.log(`licenses ok: ${count} packages`)
