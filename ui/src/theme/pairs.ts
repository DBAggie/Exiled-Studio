// Every foreground/background token pair the CSS combines (Phase 4 plan,
// decision 13), with the contrast WCAG 2.2 AA asks of it: 4.5 for text,
// 3 for the focus ring and the borders that identify a text field or a
// checkbox (non-text contrast, 1.4.11). A button is identified by its
// label, so its border (--c-border-control, the mockups') is decoration.
// theme.test.ts checks each against the values in tokens.css.

export interface Pair {
  fg: string
  bg: string
  min: number
  use: string
}

const TEXT = 4.5
const NON_TEXT = 3

const surfaces = [
  '--c-bg',
  '--c-input',
  '--c-nav',
  '--c-header',
  '--c-panel',
  '--c-raised',
  '--c-hover',
  '--c-selected',
]

const textOnAll = [
  '--c-text',
  '--c-soft',
  '--c-muted',
  '--c-link',
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

const badges = ['ok', 'info', 'warn', 'neutral', 'danger']

const blocks = ['--c-remove-fill', '--c-add-fill']

export const PAIRS: Pair[] = [
  ...textOnAll.flatMap((fg) =>
    surfaces.map((bg) => ({ fg, bg, min: TEXT, use: 'text' })),
  ),
  ...badges.map((b) => ({
    fg: `--c-${b}-text`,
    bg: `--c-${b}-fill`,
    min: TEXT,
    use: 'badge',
  })),
  ...['--c-text', '--c-muted', '--c-rarity-rare', '--c-warn-text'].flatMap(
    (fg) => blocks.map((bg) => ({ fg, bg, min: TEXT, use: 'change block' })),
  ),
  { fg: '--c-on-accent', bg: '--c-accent', min: TEXT, use: 'primary button' },
  ...surfaces.map((bg) => ({
    fg: '--c-focus',
    bg,
    min: NON_TEXT,
    use: 'focus ring',
  })),
  // Text fields and checkboxes sit on these (never on a selected or
  // hovered row).
  ...['--c-bg', '--c-input', '--c-panel', '--c-raised'].map((bg) => ({
    fg: '--c-border-strong',
    bg,
    min: NON_TEXT,
    use: 'text field and checkbox borders',
  })),
  {
    fg: '--c-accent',
    bg: '--c-panel',
    min: NON_TEXT,
    use: 'selected tab and import card border',
  },
]
