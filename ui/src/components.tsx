// Shared components. Colour is never the only signal: every badge has a
// text label (docs/ui/UI-DESIGN-BRIEF.md, section 2).
import { useEffect, useRef, type ReactNode } from 'react'

import type { LineView } from './api/types'
import { errorText, t, tDynamic, type MessageKey } from './i18n'
import { actions } from './store'
import { useAppDispatch, useAppSelector } from './util'

/** A screen's heading; focused when the screen opens, so keyboard and
 * screen-reader users land on it. */
export function ScreenHeading({ children }: { children: ReactNode }) {
  const ref = useRef<HTMLHeadingElement>(null)
  useEffect(() => {
    ref.current?.focus()
  }, [])
  return (
    <h1 tabIndex={-1} ref={ref}>
      {children}
    </h1>
  )
}

type Tone = 'ok' | 'info' | 'warn' | 'neutral' | 'danger'

export function Badge({ tone, children }: { tone: Tone; children: ReactNode }) {
  return <span className={`badge badge-${tone}`}>{children}</span>
}

const STATE_TONE: Record<string, Tone> = {
  complete: 'ok',
  partial: 'warn',
  unsupported: 'neutral',
  indeterminate: 'warn',
}

/** A calculation state (Exact, Partial, Not yet, Cannot settle). */
export function StateBadge({ state }: { state: string }) {
  return (
    <Badge tone={STATE_TONE[state] ?? 'neutral'}>
      {tDynamic(`state.${state}`, 'state.unsupported')}
    </Badge>
  )
}

const LINE_TONE: Record<LineView['status'], Tone> = {
  matched: 'ok',
  adapted: 'info',
  warning: 'warn',
  excluded: 'neutral',
  continues: 'neutral',
  other: 'neutral',
}

export function LineBadge({ status }: { status: LineView['status'] }) {
  if (status === 'other') {
    return null
  }
  return (
    <Badge tone={LINE_TONE[status]}>
      {t(`status.${status}` as MessageKey)}
    </Badge>
  )
}

/** The latest error, until dismissed (role alert: announced at once). */
export function ErrorAlert() {
  const error = useAppSelector((s) => s.ui.error)
  const dispatch = useAppDispatch()
  if (error === null) {
    return null
  }
  return (
    <div role="alert" className="panel">
      <p className="error">
        <strong>{t('error.title')}: </strong>
        {errorText(error)} <code>{error}</code>
      </p>
      <button type="button" onClick={() => dispatch(actions.error(null))}>
        {t('error.dismiss')}
      </button>
    </div>
  )
}

/** Polite announcements (the calculation's progress). */
export function LiveRegion() {
  const text = useAppSelector((s) => s.ui.announcement)
  return (
    <div role="status" aria-live="polite" className="visually-hidden">
      {text}
    </div>
  )
}
