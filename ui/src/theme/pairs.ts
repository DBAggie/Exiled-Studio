// Every foreground/background token pair the CSS combines (Phase 4 plan,
// decision 13), with the contrast WCAG 2.2 AA asks of it: 4.5 for text,
// 3 for the focus ring and control borders (non-text contrast, 1.4.11).
// theme.test.ts checks each against the values in tokens.css.

export interface Pair {
  fg: string
  bg: string
  min: number
  use: string
}

const TEXT = 4.5
const NON_TEXT = 3

const backgrounds = ['--c-bg', '--c-panel', '--c-raised']

const textOnAll = [
  '--c-text',
  '--c-muted',
  '--c-ok',
  '--c-info',
  '--c-warn',
  '--c-neutral',
  '--c-danger',
  '--c-accent',
  '--c-rarity-normal',
  '--c-rarity-magic',
  '--c-rarity-rare',
  '--c-rarity-unique',
]

export const PAIRS: Pair[] = [
  ...textOnAll.flatMap((fg) =>
    backgrounds.map((bg) => ({ fg, bg, min: TEXT, use: 'text and badges' })),
  ),
  { fg: '--c-on-accent', bg: '--c-accent', min: TEXT, use: 'primary button' },
  ...backgrounds.map((bg) => ({
    fg: '--c-focus',
    bg,
    min: NON_TEXT,
    use: 'focus ring',
  })),
  ...backgrounds.map((bg) => ({
    fg: '--c-border-strong',
    bg,
    min: NON_TEXT,
    use: 'control borders',
  })),
]
