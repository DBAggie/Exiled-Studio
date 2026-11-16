// Contrast of every token pair the CSS combines (WCAG 2.2 AA: 4.5 for
// text, 3 for the focus ring and control borders), and no colour outside
// the tokens (Phase 4 plan, decision 13; review finding 9).
import { readFileSync, readdirSync } from 'node:fs'
import { join } from 'node:path'
import { describe, expect, it } from 'vitest'

import { PAIRS } from '../theme/pairs'

const SRC = join(__dirname, '..')

function tokens(): Map<string, string> {
  const css = readFileSync(join(SRC, 'theme', 'tokens.css'), 'utf8')
  const out = new Map<string, string>()
  for (const m of css.matchAll(/(--c-[\w-]+):\s*(#[0-9a-fA-F]{6})/g)) {
    out.set(m[1], m[2])
  }
  return out
}

function luminance(hex: string): number {
  const channel = (i: number) => {
    const c = parseInt(hex.slice(i, i + 2), 16) / 255
    return c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4
  }
  return 0.2126 * channel(1) + 0.7152 * channel(3) + 0.0722 * channel(5)
}

export function contrast(a: string, b: string): number {
  const [hi, lo] = [luminance(a), luminance(b)].sort((x, y) => y - x)
  return (hi + 0.05) / (lo + 0.05)
}

function cssFiles(dir: string): string[] {
  return readdirSync(dir, { withFileTypes: true }).flatMap((e) =>
    e.isDirectory()
      ? cssFiles(join(dir, e.name))
      : e.name.endsWith('.css')
        ? [join(dir, e.name)]
        : [],
  )
}

describe('theme', () => {
  it('meets the contrast each token pair needs', () => {
    const t = tokens()
    const failures = PAIRS.flatMap((p) => {
      const fg = t.get(p.fg)
      const bg = t.get(p.bg)
      if (fg === undefined || bg === undefined) {
        return [`${p.fg} on ${p.bg}: a token is missing`]
      }
      const ratio = contrast(fg, bg)
      return ratio >= p.min
        ? []
        : [`${p.fg} on ${p.bg} (${p.use}): ${ratio.toFixed(2)} < ${p.min}`]
    })
    expect(failures).toEqual([])
  })

  it('writes colours only through the tokens', () => {
    const offenders = cssFiles(SRC)
      .filter((f) => !f.endsWith('tokens.css'))
      .flatMap((f) =>
        readFileSync(f, 'utf8')
          .split('\n')
          .filter((line) => /#[0-9a-fA-F]{3,8}\b|rgba?\(|hsla?\(/.test(line))
          .map((line) => `${f}: ${line.trim()}`),
      )
    expect(offenders).toEqual([])
  })

  it('computes contrast as WCAG does', () => {
    expect(contrast('#000000', '#ffffff')).toBeCloseTo(21, 5)
    expect(contrast('#777777', '#ffffff')).toBeCloseTo(4.48, 2)
  })
})
