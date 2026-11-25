// Shared components. Colour is never the only signal: every badge has a
// text label (docs/ui/UI-DESIGN-BRIEF.md, section 2).
import { Tooltip } from 'radix-ui'
import { useEffect, useRef, useState, type ReactNode } from 'react'

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

const STATES = ['complete', 'partial', 'unsupported', 'indeterminate']

/** A calculation state (Exact, Partial, Not yet, Cannot settle) as a small
 * mark read by its shape, never by colour alone (the revamp's signature,
 * 2026-11-25): a filled circle, a half, a ring, a dashed ring. Its name is
 * the tooltip, and is read aloud except for Exact, which a row says in its
 * own name; `text` writes the name beside the mark. `name` says the state
 * in a page's own words (a custom modifier line is "Counted"). */
export function StateMark({
  state,
  text = false,
  name: named,
}: {
  state: string
  text?: boolean
  name?: string
}) {
  const known = STATES.includes(state) ? state : 'unsupported'
  const name = named ?? tDynamic(`state.${known}`, 'state.unsupported')
  return (
    <span className={`state-mark state-${known}`} title={name}>
      <svg aria-hidden="true" width="12" height="12" viewBox="0 0 12 12">
        {known === 'complete' ? (
          <circle cx="6" cy="6" r="5" fill="currentColor" />
        ) : (
          <circle
            cx="6"
            cy="6"
            r="4.5"
            fill="none"
            stroke="currentColor"
            strokeWidth="1.5"
            strokeDasharray={known === 'indeterminate' ? '2 2' : undefined}
          />
        )}
        {known === 'partial' && (
          <path d="M6 1.5a4.5 4.5 0 000 9z" fill="currentColor" />
        )}
      </svg>
      {text ? (
        <span className="state-mark-text">{name}</span>
      ) : (
        known !== 'complete' && <span className="visually-hidden">{name}</span>
      )}
    </span>
  )
}

/** A field's state mark. */
export function FieldMark({
  field,
  text,
}: {
  field: FieldResult
  text?: boolean
}) {
  return <StateMark state={field.state} text={text} />
}

/** What the marks mean, once a page. */
export function StateLegend() {
  return (
    <ul className="state-legend" aria-label={t('state.legend')}>
      {STATES.map((state) => (
        <li key={state}>
          <StateMark state={state} text />
        </li>
      ))}
    </ul>
  )
}

export function LineBadge({ status }: { status: LineView['status'] }) {
  if (status === 'other') {
    return null
  }
  // A small mark read by its shape, as a calculation state is (RV-06):
  // filled for a line read, half for one whose text differs, a ring for
  // one not used, a ring in the warning colour for one to review. Its
  // name is the tooltip and is read aloud.
  const name = t(`status.${status}` as MessageKey)
  return (
    <span className={`line-state line-state-${status}`} title={name}>
      <span className="line-state-dot" aria-hidden="true" />
      <span className="visually-hidden">{name}</span>
    </span>
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
        {errorText(error)}
      </p>
      <ErrorCode code={error} />
      <button type="button" onClick={() => dispatch(actions.error(null))}>
        {t('error.dismiss')}
      </button>
    </div>
  )
}

/** An error or warning code, small and apart from the words: for
 * support and bug reports, not for reading (UX-05). */
export function ErrorCode({ code }: { code: string }) {
  return <p className="error-code">{t('error.code', { code })}</p>
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
  | 'plus'
  | 'sliders'
  | 'expand'
  | 'arrow-left'
  | 'arrow-right'
  | 'pin'
  | 'grip'
  | 'camera'
  | 'info'
  | 'rail'
  | 'nav-dashboard'
  | 'nav-items'
  | 'nav-skills'
  | 'nav-tree'
  | 'nav-config'
  | 'nav-warnings'
  | 'nav-ai'
  | 'nav-about'
  | 'nav-close'

const PATHS: Record<IconName, ReactNode> = {
  sliders: (
    <>
      <path d="M4 7h10M18 7h2M4 17h4M12 17h8" />
      <circle cx="16" cy="7" r="2" />
      <circle cx="10" cy="17" r="2" />
    </>
  ),
  expand: <path d="M14 4h6v6M20 4l-7 7M10 20H4v-6M4 20l7-7" />,
  'arrow-left': <path d="M19 12H5M11 6l-6 6 6 6" />,
  'arrow-right': <path d="M5 12h14M13 6l6 6-6 6" />,
  pin: (
    <>
      <path d="M9 4h6l-1 6 3 3H7l3-3z" />
      <path d="M12 13v7" />
    </>
  ),
  grip: (
    <>
      <circle cx="9" cy="6" r="1" />
      <circle cx="15" cy="6" r="1" />
      <circle cx="9" cy="12" r="1" />
      <circle cx="15" cy="12" r="1" />
      <circle cx="9" cy="18" r="1" />
      <circle cx="15" cy="18" r="1" />
    </>
  ),
  camera: (
    <>
      <path d="M4 8h3l2-3h6l2 3h3v11H4z" />
      <circle cx="12" cy="13" r="3.5" />
    </>
  ),
  info: (
    <>
      <circle cx="12" cy="12" r="9" />
      <path d="M12 11v6M12 7.5v.5" />
    </>
  ),
  rail: (
    <>
      <rect x="3" y="4" width="18" height="16" rx="2" />
      <path d="M9 4v16" />
    </>
  ),
  'nav-dashboard': (
    <>
      <rect x="3" y="3" width="8" height="10" rx="1.5" />
      <rect x="13" y="3" width="8" height="6" rx="1.5" />
      <rect x="13" y="11" width="8" height="10" rx="1.5" />
      <rect x="3" y="15" width="8" height="6" rx="1.5" />
    </>
  ),
  'nav-items': (
    <path d="M12 3l8 3v6c0 4.5-3.2 7.8-8 9-4.8-1.200-8-4.500-8-9V6z" />
  ),
  'nav-skills': (
    <>
      <path d="M12 3l7.800 4.500v9L12 21l-7.800-4.500v-9z" />
      <path d="M12 8l3.500 2v4L12 16l-3.500-2v-4z" />
    </>
  ),
  'nav-tree': (
    <>
      <circle cx="12" cy="5" r="2" />
      <circle cx="5" cy="18" r="2" />
      <circle cx="19" cy="18" r="2" />
      <circle cx="12" cy="13" r="2" />
      <path d="M12 7v4M10.500 14.500L6.300 16.600M13.500 14.500l4.200 2.100" />
    </>
  ),
  'nav-config': (
    <>
      <path d="M4 6h9M17 6h3M4 12h3M11 12h9M4 18h11M19 18h1" />
      <circle cx="15" cy="6" r="2" />
      <circle cx="9" cy="12" r="2" />
      <circle cx="17" cy="18" r="2" />
    </>
  ),
  'nav-warnings': (
    <>
      <path d="M12 3.500l9.500 16.500h-19z" />
      <path d="M12 10v4.500M12 17v.500" />
    </>
  ),
  'nav-ai': (
    <>
      <path d="M9 3v5M15 3v5M6 8h12v3a6 6 0 01-12 0z" />
      <path d="M12 17v4" />
    </>
  ),
  'nav-about': (
    <>
      <circle cx="12" cy="12" r="9" />
      <path d="M12 11v6M12 7.500v.500" />
    </>
  ),
  'nav-close': (
    <>
      <path d="M14 4h4a2 2 0 012 2v12a2 2 0 01-2 2h-4" />
      <path d="M10 8l-4 4 4 4M6 12h10" />
    </>
  ),
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
  plus: <path d="M12 5v14M5 12h14" />,
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

export type { IconName }

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

/** An explanation kept out of the way (UX audit B-01): a small "i" beside
 * a heading that shows its text on hover, on keyboard focus, and on a
 * press; the layer is placed inside the window whatever the control's
 * place. The text is the control's description, so a screen reader reads
 * it with the control. */
export function Hint({
  children,
  label,
  text,
}: {
  children: string
  label?: string
  /** Words in place of the mark, where several notes sit together and a
   * row of marks would not say which is which (RV-31). */
  text?: string
}) {
  const [open, setOpen] = useState(false)
  return (
    <Tooltip.Provider delayDuration={150}>
      <Tooltip.Root open={open} onOpenChange={setOpen}>
        <Tooltip.Trigger asChild>
          <button
            type="button"
            className={text === undefined ? 'hint' : 'link quiet hint-text'}
            aria-label={label ?? t('hint.label')}
            aria-description={children}
            onClick={() => setOpen(true)}
          >
            {text ?? <Icon name="info" size={16} />}
          </button>
        </Tooltip.Trigger>
        <Tooltip.Portal>
          <Tooltip.Content
            className="hint-content"
            side="bottom"
            align="start"
            sideOffset={6}
            collisionPadding={8}
          >
            {children}
          </Tooltip.Content>
        </Tooltip.Portal>
      </Tooltip.Root>
    </Tooltip.Provider>
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

/** An item's sockets as its `Sockets` line gives them: link groups of
 * colour letters (`['RRG', 'B']`), drawn as circles joined by link bars.
 * `filled` marks the sockets a gem sits in. Shapes and token colours, no
 * game art (ADR-000). */
export function Sockets({
  groups,
  filled,
}: {
  groups: string[]
  filled?: ReadonlySet<number>
}) {
  if (groups.length === 0) return null
  const starts = groups.map((_, i) =>
    groups.slice(0, i).reduce((n, g) => n + g.length, 0),
  )
  const total = groups.reduce((n, g) => n + g.length, 0)
  return (
    <span
      className="sockets"
      role="img"
      aria-label={t('sockets.label', {
        n: total,
        links: groups.map((g) => g.length).join(' + '),
      })}
    >
      {groups.map((g, gi) => (
        <span className="socket-link" key={gi}>
          {[...g].map((c, ci) => (
            <span
              key={ci}
              className={`socket socket-${c.toLowerCase()}${
                filled?.has((starts[gi] ?? 0) + ci) === true ? ' filled' : ''
              }`}
            />
          ))}
        </span>
      ))}
    </span>
  )
}
