// PoB's configuration options (labels, types, choices, defaults,
// tooltips), read from the pinned PoB by tools/oracle/config-options.ps1
// into data/pob-config-options.json (PoB is MIT; its labels and tooltips
// are kept as PoB wrote them, with colour codes removed). The
// configuration page names inputs by them and adds any option from them.
import catalogue from './data/pob-config-options.json'

import type { ConfigValueView } from './api/types'

export interface ConfigChoice {
  val: string | number
  label: string
}

export interface ConfigOption {
  /** PoB's key, as builds store it. */
  var: string
  /** PoB's control: check, count, countAllowZero, integer, float, list. */
  type: string
  label: string
  /** PoB's section ("General", "Enemy Stats", ...). */
  section?: string
  /** The heading it sits under within the section, if any. */
  heading?: string
  list?: ConfigChoice[]
  /** PoB's default (a list's default value, or a check's state). */
  default?: string | number | boolean
  /** The value PoB shows when it is left unset. */
  placeholder?: number
  tooltip?: string
  /** PoB shows it only for these skills. */
  skills?: string[]
  /** PoB shows it only under conditions of the build it checks itself. */
  conditional: boolean
  legacy?: boolean
}

interface Catalogue {
  source: { tag: string; commit: string }
  options: ConfigOption[]
}

const data = catalogue as unknown as Catalogue

/** Every option, in PoB's order. */
export const OPTIONS: readonly ConfigOption[] = data.options

/** The PoB version the catalogue was read from. */
export const SOURCE_TAG = data.source.tag

const byVar = new Map(OPTIONS.map((o) => [o.var, o]))

/** The option named `name`, if PoB has one. */
export function optionOf(name: string): ConfigOption | undefined {
  return byVar.get(name)
}

/** The sections, in PoB's order. */
export const SECTIONS: readonly string[] = [
  ...new Set(OPTIONS.map((o) => o.section ?? 'Other')),
]

/** The value a new input of `o` starts with: its default, else on for a
 * check, else its placeholder or 0. */
export function initialValue(o: ConfigOption): ConfigValueView {
  if (o.type === 'check') return { boolean: true }
  if (o.type === 'list') {
    const v = o.default ?? o.list?.[0]?.val ?? ''
    return typeof v === 'number' ? { number: v } : { text: String(v) }
  }
  return { number: o.placeholder ?? 0 }
}

/** Whether PoB would show `o` for a build with these skills: shown
 * everywhere, or for one of them. Options PoB shows under conditions it
 * checks itself are not counted (they are found with "all options"). */
export function relevant(o: ConfigOption, skills: ReadonlySet<string>) {
  if (o.conditional || o.legacy) return false
  if (o.skills == null) return true
  return o.skills.some((s) => skills.has(s))
}

/** Whether `o` matches a search: its label, key, or heading. */
export function matches(o: ConfigOption, query: string) {
  const q = query.trim().toLowerCase()
  if (q === '') return true
  return [o.label, o.var, o.heading ?? ''].some((s) =>
    s.toLowerCase().includes(q),
  )
}
