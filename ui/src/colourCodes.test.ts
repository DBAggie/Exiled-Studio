import { describe, expect, it } from 'vitest'

import { parts, plain, readableHex } from './colourCodes'

describe('imported text', () => {
  it('should leave colour codes and the trailing link out of plain text', () => {
    expect(plain('6L ^25-Frostbearers (Mortar projectile = 5x count)')).toBe(
      '6L 5-Frostbearers (Mortar projectile = 5x count)',
    )
    expect(plain('^2Early lvl 92 ^5+1 Spectre {2}')).toBe(
      'Early lvl 92 +1 Spectre',
    )
    expect(plain('^xFF0000Red Maps{6}')).toBe('Red Maps')
    expect(plain('^6^4---------------------')).toBe('---------------------')
  })

  it('should keep a caret that is not a code, and braces inside the text', () => {
    expect(plain('2^n and ^xZZ')).toBe('2^n and ^xZZ')
    expect(plain('Use {this} first')).toBe('Use {this} first')
  })

  it('should give each run its colour', () => {
    expect(parts('6L ^25-Frost ^7plain')).toEqual([
      { text: '6L ', colour: null },
      { text: '5-Frost ', colour: 'var(--c-ok)' },
      { text: 'plain', colour: null },
    ])
  })

  it('should lighten a colour too dark to read on a dark panel', () => {
    expect(readableHex('ffffff')).toBe('#ffffff')
    const dark = readableHex('000080')
    expect(dark).not.toBe('#000080')
    expect(parts('^x000080navy')[0]?.colour).toBe(dark)
  })
})
