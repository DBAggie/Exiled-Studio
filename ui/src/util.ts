// Hooks and helpers shared by the screens (kept out of component files).
import { useDispatch, useSelector } from 'react-redux'

import type { FieldResult } from './api/types'
import { formatNumber, tDynamic } from './i18n'
import type { Dispatch, RootState } from './store'

export const useAppDispatch = () => useDispatch<Dispatch>()
export const useAppSelector = <T>(f: (s: RootState) => T) => useSelector(f)

/** A field's value as text; a dash when it has none. */
export function fieldValue(f: FieldResult): string {
  return typeof f.value === 'number' ? formatNumber(f.value) : '—'
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
