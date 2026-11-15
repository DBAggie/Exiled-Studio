// B-2 item evidence (E5): an item card's values as the game's tooltip
// prints them, from the calculation (`ItemCard.values`): a weapon's damage
// per type, critical strike chance, and attacks per second; a shield's
// chance to block; the defences. When the card has values, they take the
// place of the defences written on the item.
import type { ItemCard } from './api/types'
import { t } from './i18n'

export interface ValueLine {
  name: string
  value: string
  /** Each part's damage type, for the game's colours (elemental ranges). */
  parts?: { text: string; kind: string }[]
}

/** Written properties the values replace. */
const REPLACED = new Set([
  'Armour',
  'Evasion Rating',
  'Evasion',
  'Energy Shield',
])

const DEFENCES = [
  ['armour', 'values.armour'],
  ['evasion', 'values.evasion'],
  ['energy-shield', 'values.energyShield'],
] as const

/** The card's value lines in the game's order; none without values. */
export function valueLines(c: ItemCard): ValueLine[] {
  const v = c.values
  if (v == null) return []
  const out: ValueLine[] = []
  const range = (min: number, max: number) => `${min}-${max}`
  const of = (type: string) => v.damage.filter((d) => d['damage-type'] === type)
  for (const d of of('physical')) {
    out.push({ name: t('values.physical'), value: range(d.min, d.max) })
  }
  const elemental = ['fire', 'cold', 'lightning'].flatMap((type) =>
    of(type).map((d) => ({ text: range(d.min, d.max), kind: type })),
  )
  if (elemental.length > 0) {
    out.push({
      name: t('values.elemental'),
      value: elemental.map((p) => p.text).join(', '),
      parts: elemental,
    })
  }
  for (const d of of('chaos')) {
    out.push({
      name: t('values.chaos'),
      value: range(d.min, d.max),
      parts: [{ text: range(d.min, d.max), kind: 'chaos' }],
    })
  }
  if (v['crit-chance'] != null) {
    out.push({
      name: t('values.crit'),
      value: `${v['crit-chance'].toFixed(2)}%`,
    })
  }
  if (v['attack-rate'] != null) {
    out.push({ name: t('values.rate'), value: v['attack-rate'].toFixed(2) })
  }
  if (v.block != null) {
    out.push({ name: t('values.block'), value: `${v.block}%` })
  }
  for (const [defence, key] of DEFENCES) {
    for (const d of v.defences.filter((x) => x.defence === defence)) {
      out.push({ name: t(key), value: String(d.value) })
    }
  }
  return out
}

/** The written properties the card still shows next to its values. */
export function writtenProperties(c: ItemCard): ItemCard['properties'] {
  return c.values == null
    ? c.properties
    : c.properties.filter((p) => !REPLACED.has(p.name))
}
