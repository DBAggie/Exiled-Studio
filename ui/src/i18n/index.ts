import { en, type MessageKey } from './en'

export type { MessageKey }

type Params = Record<string, string | number>

/** The message for `key` with `{name}` placeholders filled. */
export function t(key: MessageKey, params: Params = {}): string {
  return en[key].replace(/\{(\w+)\}/g, (whole, name: string) =>
    name in params ? String(params[name]) : whole,
  )
}

/** A message by a key built at run time, with a fallback. */
export function tDynamic(
  key: string,
  fallback: MessageKey,
  params: Params = {},
): string {
  if (key in en) {
    return t(key as MessageKey, params)
  }
  return t(fallback, params)
}

const numberFormat = new Intl.NumberFormat(undefined, {
  maximumFractionDigits: 2,
})

/** A number in the user's locale. */
export function formatNumber(value: number): string {
  return numberFormat.format(value)
}

/** The text for an error code. */
export function errorText(code: string): string {
  return tDynamic(`error.${code}`, 'error.generic', { code })
}

/** A field's label (its name when the catalog has none). */
export function fieldLabel(field: string): string {
  const key = `field.${field}`
  return key in en ? en[key as MessageKey] : field
}

/** Offence fields; the rest are defence. */
export const OFFENCE = new Set([
  'average-damage',
  'speed',
  'hit-speed',
  'hit-chance',
  'crit-chance',
  'crit-multiplier',
  'hit-dps',
])
