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
  const n = formatNumber(value)
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

/** The build's status pill: its completeness, and its warnings. */
export function buildStatus(build: BuildView): { tone: Tone; text: string } {
  const state = tDynamic(`state.${build.completeness}`, 'state.unsupported')
  const n = build.warnings.length
  return {
    tone: STATE_TONE[build.completeness] ?? 'neutral',
    text:
      n === 0
        ? state
        : n === 1
          ? t('header.status1', { state })
          : t('header.status', { state, n }),
  }
}

/** An item's name: its own, else its base, else its number. */
export function itemName(item: {
  name?: string | null
  base?: string | null
  id: number
}): string {
  return item.name ?? item.base ?? `#${item.id}`
}
