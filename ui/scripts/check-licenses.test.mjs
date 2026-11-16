// The license allow-list reads SPDX expressions as pnpm reports them.
import { describe, expect, it } from 'vitest'

import { allowed } from './check-licenses.mjs'

describe('license check', () => {
  it('allows open-source licenses, fonts included', () => {
    for (const l of ['MIT', 'MPL-2.0', 'OFL-1.1', 'Apache-2.0', 'ISC']) {
      expect(allowed(l)).toBe(true)
    }
  })

  it('reads OR as either side and AND as both', () => {
    expect(allowed('Apache-2.0 OR MIT')).toBe(true)
    expect(allowed('(GPL-3.0-only OR MIT)')).toBe(true)
    expect(allowed('MIT AND GPL-3.0-only')).toBe(false)
  })

  it('reads A WITH exception as A', () => {
    expect(allowed('Apache-2.0 WITH LLVM-exception')).toBe(true)
    expect(allowed('GPL-2.0-only WITH Classpath-exception-2.0')).toBe(false)
  })

  it('drops parentheses: (A OR B) AND C reads as A OR (B AND C)', () => {
    expect(allowed('(MIT OR GPL-3.0-only) AND GPL-3.0-only')).toBe(true)
  })

  it('does not allow copyleft that would bind the app', () => {
    for (const l of ['GPL-3.0-only', 'AGPL-3.0-or-later', 'LGPL-2.1-only']) {
      expect(allowed(l)).toBe(false)
    }
  })
})
