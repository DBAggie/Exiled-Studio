// Shared components. Colour is never the only signal: every badge has a
// text label (docs/ui/UI-DESIGN-BRIEF.md, section 2).
import { useEffect, useRef, type ReactNode } from 'react'

import type { FieldResult, LineView } from './api/types'
import { errorText, t, tDynamic, type MessageKey } from './i18n'
import { actions } from './store'
import { useAppDispatch, useAppSelector } from './util'

/** A screen's heading; focused when the screen opens, so keyboard and
 * screen-reader users land on it. */
export function ScreenHeading({
  children,
  className,
  id,
}: {
  children: ReactNode
  className?: string
  id?: string
}) {
  const ref = useRef<HTMLHeadingElement>(null)
  useEffect(() => {
    ref.current?.focus()
  }, [])
  return (
    <h1 tabIndex={-1} ref={ref} className={className} id={id}>
      {children}
    </h1>
  )
}

export type Tone = 'ok' | 'info' | 'warn' | 'neutral' | 'danger'

export function Badge({
  tone,
  children,
  className,
}: {
  tone: Tone
  children: ReactNode
  className?: string
}) {
  return (
    <span className={`badge badge-${tone}${className ? ` ${className}` : ''}`}>
      {children}
    </span>
  )
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

/** A field's state badge. */
export function FieldBadge({ field }: { field: FieldResult }) {
  return <StateBadge state={field.state} />
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
    <div role="alert" className="panel alert">
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

// Icons: the project's own line drawings (never game art; ADR-000).
// Decorative: each sits next to a text label or inside a control with an
// accessible name.
type IconName =
  | 'logo'
  | 'undo'
  | 'redo'
  | 'chevron'
  | 'download'
  | 'close'
  | 'copy'
  | 'folder'

const PATHS: Record<IconName, ReactNode> = {
  logo: (
    <>
      <path d="M3 5l18 14M3 19L21 5" />
      <path d="M3 12h4M17 12h4" />
    </>
  ),
  undo: (
    <>
      <path d="M9 14L4 9l5-5" />
      <path d="M4 9h10a6 6 0 010 12h-3" />
    </>
  ),
  redo: (
    <>
      <path d="M15 14l5-5-5-5" />
      <path d="M20 9H10a6 6 0 000 12h3" />
    </>
  ),
  chevron: <path d="M6 9l6 6 6-6" />,
  download: (
    <>
      <path d="M12 3v12M7 10l5 5 5-5" />
      <path d="M5 20h14" />
    </>
  ),
  close: <path d="M6 6l12 12M18 6L6 18" />,
  copy: (
    <>
      <rect x="9" y="9" width="11" height="11" rx="2" />
      <path d="M5 15V5a2 2 0 012-2h8" />
    </>
  ),
  folder: (
    <path d="M3 7a2 2 0 012-2h4l2 2h8a2 2 0 012 2v8a2 2 0 01-2 2H5a2 2 0 01-2-2z" />
  ),
}

export function Icon({ name, size = 18 }: { name: IconName; size?: number }) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth={2}
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
      focusable="false"
    >
      {PATHS[name]}
    </svg>
  )
}

/** The app's mark and name. */
export function Brand() {
  return (
    <span className="brand">
      <Icon name="logo" size={22} />
      {t('app.name')}
    </span>
  )
}
