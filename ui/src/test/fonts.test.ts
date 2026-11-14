// The bundled fonts (SIL OFL 1.1) ship with their license texts: each copy
// in public/licenses/ is the one in its package, so updating a font
// without its license fails here.
import { readFileSync } from 'node:fs'
import { join } from 'node:path'
import { describe, expect, it } from 'vitest'

const UI = join(__dirname, '..', '..')
const FONTS = ['inter', 'space-grotesk', 'jetbrains-mono']

describe('fonts', () => {
  it.each(FONTS)('ships the license of %s', (font) => {
    const pkg = join(UI, 'node_modules', '@fontsource-variable', font)
    const shipped = readFileSync(
      join(UI, 'public', 'licenses', `${font}-OFL-1.1.txt`),
      'utf8',
    )
    expect(shipped).toBe(readFileSync(join(pkg, 'LICENSE'), 'utf8'))
    expect(shipped).toContain('SIL Open Font License, Version 1.1')
  })
})
