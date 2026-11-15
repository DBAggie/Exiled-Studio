// Hooks and helpers shared by the screens (kept out of component files).
import { useDispatch, useSelector } from 'react-redux'

import type { BuildView, FieldResult } from './api/types'
import type { Tone } from './components'
import { formatNumber, t, tDynamic } from './i18n'
import type { Dispatch, RootState } from './store'

export const useAppDispatch = () => useDispatch<Dispatch>()
export const useAppSelector = <T>(f: (s: RootState) => T) => useSelector(f)

/** Fields shown as a percentage. */
const PERCENT =
  /^(resistance\.|block-chance$|hit-chance$|crit-chance$|poe1\.spell-)/
/** Fields shown per second. */
const PER_SECOND = new Set(['speed', 'hit-speed'])

/** A number with a field's unit (75%, 2.5 / s). */
export function withUnit(field: string, value: number): string {
  // The crit multiplier is a factor (4.48); players read it as 448%.
  if (field === 'crit-multiplier') {
    return `${formatNumber(Math.round(value * 1000) / 10)}%`
  }
  // From 1,000 up, whole numbers, as PoB shows them (UX-13).
  const n = formatNumber(Math.abs(value) >= 1000 ? Math.round(value) : value)
  if (PERCENT.test(field)) return `${n}%`
  if (PER_SECOND.has(field)) return t('unit.perSecond', { n })
  return n
}

/** A field's value as text with its unit; a dash when it has none. */
export function fieldValue(f: FieldResult): string {
  return typeof f.value === 'number' ? withUnit(f.field, f.value) : '—'
}

/** A field's state in words, as its badge says it. */
export function fieldStateText(f: FieldResult): string {
  return tDynamic(`state.${f.state}`, 'state.unsupported')
}

/** A calc reason as text. */
export function reasonText(reason: unknown): string {
  if (typeof reason === 'string') {
    return tDynamic(`reason.${reason}`, 'reason.generic', { kind: reason })
  }
  if (typeof reason === 'object' && reason !== null) {
    const [kind, detail] = Object.entries(reason)[0] ?? ['', {}]
    const d = (detail ?? {}) as Record<string, unknown>
    const stat = typeof d.stat === 'string' ? d.stat : ''
    return tDynamic(`reason.${kind}`, 'reason.generic', { kind, stat })
  }
  return ''
}

const STATE_TONE: Record<string, Tone> = {
  complete: 'ok',
  partial: 'warn',
  indeterminate: 'warn',
}

/** The warnings on items the calculation reads, and on the others
 * (other item sets, jewels in inactive trees): only the first touch the
 * numbers (UX-03). */
export function splitWarnings(build: BuildView): {
  used: BuildView['warnings']
  unused: BuildView['warnings']
} {
  return {
    used: build.warnings.filter((w) => w['in-use']),
    unused: build.warnings.filter((w) => !w['in-use']),
  }
}

/** The build's status pill: the completeness of what the calculation
 * reads, and the warnings on it. */
export function buildStatus(build: BuildView): { tone: Tone; text: string } {
  const completeness = build['in-use-completeness']
  const state = tDynamic(`state.${completeness}`, 'state.unsupported')
  const n = splitWarnings(build).used.length
  return {
    tone: STATE_TONE[completeness] ?? 'neutral',
    text:
      n === 0
        ? state
        : n === 1
          ? t('header.status1', { state })
          : t('header.status', { state, n }),
  }
}

/** A history entry in words, for the undo and redo buttons (UX-23). */
export function historyText(
  entry: { kind: string; item: number },
  build: BuildView,
): string {
  const item = build.items.find((i) => i.id === entry.item)
  const title = (sets: { id: number; title?: string | null }[]) =>
    cleanTitle(sets.find((s) => s.id === entry.item)?.title) ||
    String(entry.item)
  return tDynamic(`history.${entry.kind}`, 'history.skill-edit', {
    item: item == null ? `#${entry.item}` : itemName(item),
    set: title(
      entry.kind === 'skill-set' ? build['skill-sets'] : build['item-sets'],
    ),
    stage:
      build.loadouts.find((l) => l.spec === entry.item)?.title ??
      String(entry.item),
  })
}

/** An item's name: its own, else its base, else its number. */
export function itemName(item: {
  name?: string | null
  base?: string | null
  id: number
}): string {
  return item.name ?? item.base ?? `#${item.id}`
}

/** A set's title as the user reads it: without PoB's colour codes (`^4`,
 * `^xRRGGBB`) and loadout links (`{1,2}`). */
export function cleanTitle(title: string | null | undefined): string {
  return (title ?? '')
    .replace(/\^(x[0-9a-fA-F]{6}|[0-9])/g, '')
    .replace(/\s*\{[^}]*\}\s*$/, '')
    .trim()
}

/** An input's name as a player reads it: PoB's key split into words,
 * without the `condition` and `multiplier` prefixes ("conditionFullLife"
 * is "Full life"). */
export function inputLabel(name: string): string {
  const words = name
    .replace(/^(condition|multiplier)(?=[A-Z])/, '')
    .replace(/([a-z0-9])([A-Z])/g, '$1 $2')
    .replace(/([A-Z]+)([A-Z][a-z])/g, '$1 $2')
    .toLowerCase()
  return words.charAt(0).toUpperCase() + words.slice(1)
}

/** Game data as players name it (UX-14): the game and its patch, from the
 * passive tree version the application knows for the data (`3_29` is
 * patch 3.29); the Steam build when the patch is not known. */
export function dataName(
  version: { game: string; 'steam-build'?: number | null },
  tree?: string | null,
): string {
  const game = version.game === 'poe1' ? 'Path of Exile' : 'Path of Exile 2'
  if (tree != null && /^\d+_\d+$/.test(tree)) {
    return t('data.patch', { game, patch: tree.replace('_', '.') })
  }
  return t('data.gameBuild', { game, build: version['steam-build'] ?? '?' })
}

type VersionLike = {
  game: string
  'schema-version'?: number
  'steam-build'?: number | null
}

/** [`dataName`] of a data version (a build's pinned one), with the patch
 * when the loaded or an installed bundle has that version. */
export function useDataName(version: VersionLike | null | undefined): string {
  const data = useAppSelector((s) => s.session.snapshot?.data)
  if (version == null) return '—'
  const same = (v: VersionLike | null | undefined) =>
    v != null &&
    v.game === version.game &&
    v['steam-build'] === version['steam-build'] &&
    v['schema-version'] === version['schema-version']
  const tree = same(data?.['data-version'])
    ? data?.['tree-version']
    : data?.installed?.find((b) => same(b['data-version']))?.['tree-version']
  return dataName(version, tree)
}
