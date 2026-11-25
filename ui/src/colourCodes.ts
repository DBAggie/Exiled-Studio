// Text an imported build carries (a socket group's label, a passive tree's
// or set's title, a note): its colour codes (`^0` to `^9`, `^xRRGGBB`) are
// drawn as colour, never printed, and a trailing `{…}` link is left out
// (maintainer 2026-11-24). The build keeps the text as written, so an
// export is unchanged; `loom_app::loadout::clean_title` is the same rule
// for the titles the application sends already cleaned.
/** The digit codes' colours, as the legacy application draws them, in
 * shades that read on the dark panels (0 and 9, black and dark grey
 * there, are the muted text colour here; 7, white, is the text's own). */
const DIGIT: Record<string, string | null> = {
  0: 'var(--c-muted)',
  1: 'var(--c-danger)',
  2: 'var(--c-ok)',
  3: 'var(--c-info)',
  4: 'var(--c-rarity-rare)',
  5: 'var(--c-tt-chaos)',
  6: 'var(--c-info-text)',
  7: null,
  8: 'var(--c-soft)',
  9: 'var(--c-muted)',
}

export interface Part {
  text: string
  /** A CSS colour, or null for the surrounding text's. */
  colour: string | null
}

const channel = (v: number) => {
  const c = v / 255
  return c <= 0.03928 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4
}

/** `#rrggbb` lightened toward white until it reads on a dark panel
 * (relative luminance 0.2, about 4.5:1 on the panel colour). */
export function readableHex(hex: string): string {
  let [r, g, b] = [0, 2, 4].map((i) => parseInt(hex.slice(i, i + 2), 16)) as [
    number,
    number,
    number,
  ]
  const lum = () =>
    0.2126 * channel(r) + 0.7152 * channel(g) + 0.0722 * channel(b)
  for (let i = 0; i < 20 && lum() < 0.2; i += 1) {
    r += (255 - r) * 0.15
    g += (255 - g) * 0.15
    b += (255 - b) * 0.15
  }
  const h = (v: number) => Math.round(v).toString(16).padStart(2, '0')
  return `#${h(r)}${h(g)}${h(b)}`
}

/** The text's runs with their colours; the trailing link removed. */
export function parts(text: string): Part[] {
  const out: Part[] = []
  let colour: string | null = null
  let run = ''
  const push = () => {
    if (run !== '') out.push({ text: run, colour })
    run = ''
  }
  for (let i = 0; i < text.length; i += 1) {
    const c = text[i]
    const next = text[i + 1]
    if (c === '^' && next !== undefined) {
      if (next >= '0' && next <= '9') {
        push()
        colour = DIGIT[next] ?? null
        i += 1
        continue
      }
      const hex = text.slice(i + 2, i + 8)
      if (next === 'x' && /^[0-9a-fA-F]{6}$/.test(hex)) {
        push()
        colour = readableHex(hex)
        i += 7
        continue
      }
    }
    run += c
  }
  push()
  // A trailing `{…}` (a link to other sets) is not part of the name.
  const last = out.at(-1)
  if (last !== undefined) {
    const trimmed = last.text.trimEnd()
    const open = trimmed.lastIndexOf('{')
    last.text =
      trimmed.endsWith('}') && open >= 0
        ? trimmed.slice(0, open).trimEnd()
        : trimmed
    if (last.text === '') out.pop()
  }
  if (out[0] !== undefined) out[0].text = out[0].text.trimStart()
  return out
}

/** The text without its codes and link: for names read aloud, option
 * lists, and anywhere colour cannot show. */
export function plain(text: string): string {
  return parts(text)
    .map((p) => p.text)
    .join('')
}

/** An empty group whose label is a divider the build's author drew
 * ("==== Main Skills ===="): its words without the signs, and its colour.
 * `null` for any other group. */
export function dividerOf(
  label: string | null,
  gems: number,
): { text: string; colour: string | null } | null {
  if (label === null || gems > 0) return null
  const whole = plain(label).trim()
  const text = whole.replace(/^[=\-_~*#\s]+|[=\-_~*#\s]+$/g, '')
  if (text === '' || !/^[=\-_~*#]{2,}|[=\-_~*#]{2,}$/.test(whole)) return null
  return {
    text,
    colour: parts(label).find((x) => x.colour !== null)?.colour ?? null,
  }
}
