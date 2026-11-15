// Screen 3 (docs/ui, Workspace) and screen 9 (Customize dashboard, B-5):
// the user's dashboards, up to four, each a list of components two to a
// row, each half or full width (`../dashboards`). Fields come from the calc
// result document (never a fixed list), each with its state; a field
// without a value says why, never a blank or a zero. A row is one button:
// label, value, and badge on one line.
import { useState } from 'react'

import type { BuildView, FieldResult } from '../api/types'
import { Badge, FieldBadge, ScreenHeading } from '../components'
import {
  type ComponentKind,
  type DashComponent,
  type DashboardDef,
  MAX_COMPONENTS,
  MAX_DASHBOARDS,
  PANEL_KINDS,
  moved,
  newId,
} from '../dashboards'
import { OFFENCE, fieldLabel, t } from '../i18n'
import { actions, setPrefs } from '../store'
import { Inventory } from './Inventory'
import {
  fieldStateText,
  fieldValue,
  reasonText,
  splitWarnings,
  useAppDispatch,
  useAppSelector,
  useDataName,
} from '../util'

function why(f: FieldResult): string | null {
  if (f.state === 'unsupported' && f.reason != null) {
    return reasonText(f.reason)
  }
  if (f.state === 'indeterminate' && f.indeterminate !== undefined) {
    return f.indeterminate.map(reasonText).join(' ')
  }
  if (f.state === 'partial' && f.unsupported !== undefined) {
    return f.unsupported.map(reasonText).join(' ')
  }
  return null
}

function FieldPanel({
  id,
  title,
  fields,
  hint,
}: {
  id: string
  title: string
  fields: FieldResult[]
  hint?: string
}) {
  const dispatch = useAppDispatch()
  return (
    <section className="panel" aria-labelledby={id}>
      <div className="panel-head">
        <h2 id={id}>{title}</h2>
        {hint !== undefined && <span className="muted small-text">{hint}</span>}
      </div>
      <ul className="stat-list">
        {fields.map((f) => {
          const reason = why(f)
          const negative =
            f.field.startsWith('resistance.') &&
            typeof f.value === 'number' &&
            f.value < 0
          return (
            <li key={f.field}>
              <button
                type="button"
                className="stat-row"
                aria-label={t('dashboard.row', {
                  field: fieldLabel(f.field),
                  value: fieldValue(f),
                  state: fieldStateText(f),
                  reason: reason === null ? '' : ` ${reason}`,
                })}
                onClick={() => dispatch(actions.breakdown(f.field))}
              >
                <span className="stat-label">
                  <span>{fieldLabel(f.field)}</span>
                  {reason !== null && <span className="reason">{reason}</span>}
                </span>
                <span className="stat-end">
                  <span className={`stat-value${negative ? ' negative' : ''}`}>
                    {fieldValue(f)}
                  </span>
                  {/* Only the exceptions are marked (UX-12). */}
                  {f.state !== 'complete' && <FieldBadge field={f} />}
                </span>
              </button>
            </li>
          )
        })}
      </ul>
    </section>
  )
}

/** The warnings strip: the lines kept but not calculated. */
function WarningsPanel({ build, id }: { build: BuildView; id: string }) {
  const dispatch = useAppDispatch()
  const { used, unused } = splitWarnings(build)
  return (
    <section className="panel" aria-labelledby={id}>
      <div className="panel-head">
        <h2 id={id}>{t('nav.warnings')}</h2>
        <button
          type="button"
          className="link"
          onClick={() => dispatch(actions.go('warnings'))}
        >
          {t('dashboard.viewWarnings')}
        </button>
      </div>
      {build.warnings.length === 0 ? (
        <p className="muted small-text">{t('warnings.none')}</p>
      ) : (
        <>
          <p className="muted small-text">
            {used.length === 0
              ? t('dashboard.warningsUnusedOnly', { n: unused.length })
              : used.length === 1
                ? t('dashboard.warnings1')
                : t('dashboard.warnings', { n: used.length })}
            {used.length > 0 &&
              unused.length > 0 &&
              ` ${t('dashboard.warningsUnused', { n: unused.length })}`}
          </p>
          <ul className="warning-strip">
            {used.slice(0, 6).map((w) => (
              <li key={`${w.item}-${w.line}`}>
                <span className="mono" title={w.text}>
                  {w.text}
                </span>
                <span className="muted">{w['item-name'] ?? `#${w.item}`}</span>
              </li>
            ))}
          </ul>
        </>
      )}
    </section>
  )
}

/** The main skill's socket group: its gems, supports marked. */
function MainSkillPanel({ build, id }: { build: BuildView; id: string }) {
  const dispatch = useAppDispatch()
  const group = build['skill-groups'].find((g) => g.main)
  return (
    <section className="panel" aria-labelledby={id}>
      <div className="panel-head">
        <h2 id={id}>{t('dashboard.mainSkillTitle')}</h2>
        <button
          type="button"
          className="link"
          onClick={() => dispatch(actions.go('skills'))}
        >
          {t('dashboard.openSkills')}
        </button>
      </div>
      {group === undefined ? (
        <p className="muted small-text">{t('dashboard.noMainSkill')}</p>
      ) : (
        <ul className="main-skill-gems">
          {group.gems.map((g, i) => (
            <li key={i} className={g.enabled ? undefined : 'muted'}>
              {g.main ? (
                <strong>{g.name ?? t('skills.gemN', { n: i + 1 })}</strong>
              ) : (
                <span>{g.name ?? t('skills.gemN', { n: i + 1 })}</span>
              )}{' '}
              <span className="muted small-text">
                {g.support === true
                  ? t('dashboard.support')
                  : t('dashboard.active')}{' '}
                · {t('dashboard.gemLevel', { level: g.level })}
              </span>
            </li>
          ))}
        </ul>
      )}
    </section>
  )
}

/** One calculated number, large (a stat tile). */
function StatTile({
  field,
  id,
}: {
  field: FieldResult | undefined
  id: string
}) {
  const dispatch = useAppDispatch()
  if (field === undefined) {
    return (
      <section className="panel stat-tile" aria-labelledby={id}>
        <h2 id={id} className="small-text muted">
          {t('dashboard.statMissing')}
        </h2>
      </section>
    )
  }
  const reason = why(field)
  return (
    <section className="panel stat-tile" aria-labelledby={id}>
      <button
        type="button"
        className="stat-tile-button"
        onClick={() => dispatch(actions.breakdown(field.field))}
      >
        <h2 id={id} className="small-text">
          {fieldLabel(field.field)}
        </h2>
        <span className="stat-tile-value">{fieldValue(field)}</span>
        {field.state !== 'complete' && <FieldBadge field={field} />}
        {reason !== null && <span className="reason">{reason}</span>}
      </button>
    </section>
  )
}

/** One component of a dashboard. */
function Component({
  c,
  build,
  fields,
  calculated,
}: {
  c: DashComponent
  build: BuildView
  fields: FieldResult[]
  calculated: boolean
}) {
  const id = `dc-${c.id}`
  // Without a calculation the page says so once (above the components).
  switch (c.kind) {
    case 'offence':
      return calculated ? (
        <FieldPanel
          id={id}
          title={t('dashboard.offence')}
          fields={fields.filter((f) => OFFENCE.has(f.field))}
          hint={t('dashboard.hint')}
        />
      ) : null
    case 'defence':
      return calculated ? (
        <FieldPanel
          id={id}
          title={t('dashboard.defence')}
          fields={fields.filter((f) => !OFFENCE.has(f.field))}
        />
      ) : null
    case 'warnings':
      return <WarningsPanel build={build} id={id} />
    case 'main-skill':
      return <MainSkillPanel build={build} id={id} />
    case 'equipment':
      return <Inventory />
    case 'stat':
      return calculated ? (
        <StatTile field={fields.find((f) => f.field === c.field)} id={id} />
      ) : null
  }
}

/** The name of a component, for the customize controls. */
function componentName(c: DashComponent): string {
  return c.kind === 'stat'
    ? t('dashboard.kind.stat', { field: fieldLabel(c.field ?? '') })
    : t(`dashboard.kind.${c.kind}`)
}

/** A component's controls while customizing: move, resize, remove. */
function ComponentTools({
  c,
  index,
  count,
  onChange,
}: {
  c: DashComponent
  index: number
  count: number
  onChange: (f: (list: DashComponent[]) => DashComponent[]) => void
}) {
  const name = componentName(c)
  return (
    <div
      className="dash-tools"
      role="group"
      aria-label={t('dashboard.tools', { name })}
    >
      <strong className="small-text">{name}</strong>
      <span className="spacer" />
      <button
        type="button"
        disabled={index === 0}
        onClick={() => onChange((l) => moved(l, index, -1))}
      >
        {t('dashboard.moveUp')}
      </button>
      <button
        type="button"
        disabled={index === count - 1}
        onClick={() => onChange((l) => moved(l, index, 1))}
      >
        {t('dashboard.moveDown')}
      </button>
      <button
        type="button"
        onClick={() =>
          onChange((l) =>
            l.map((x) =>
              x.id === c.id
                ? { ...x, width: x.width === 'half' ? 'full' : 'half' }
                : x,
            ),
          )
        }
      >
        {c.width === 'half' ? t('dashboard.makeFull') : t('dashboard.makeHalf')}
      </button>
      <button
        type="button"
        onClick={() => onChange((l) => l.filter((x) => x.id !== c.id))}
      >
        {t('dashboard.remove')}
      </button>
    </div>
  )
}

/** The component library: each panel once per dashboard, a stat tile for
 * any calculated field. */
function Library({
  dash,
  fields,
  onAdd,
}: {
  dash: DashboardDef
  fields: FieldResult[]
  onAdd: (kind: ComponentKind, field?: string) => void
}) {
  const [field, setField] = useState('')
  const has = (kind: ComponentKind, f?: string) =>
    dash.components.some((c) => c.kind === kind && c.field === f)
  const full = dash.components.length >= MAX_COMPONENTS
  const free = fields.filter((f) => !has('stat', f.field))
  return (
    <section className="panel" aria-labelledby="dash-library">
      <div className="panel-head">
        <h2 id="dash-library">{t('dashboard.library')}</h2>
      </div>
      <ul className="dash-library">
        {PANEL_KINDS.map((k) => (
          <li key={k}>
            <button
              type="button"
              disabled={full || has(k)}
              onClick={() => onAdd(k)}
            >
              {t('dashboard.add', { name: t(`dashboard.kind.${k}`) })}
            </button>
            <span className="muted small-text">
              {' '}
              {t(`dashboard.kindHelp.${k}`)}
            </span>
          </li>
        ))}
      </ul>
      <div className="dash-stat-add">
        <label htmlFor="dash-stat">{t('dashboard.statField')}</label>{' '}
        <select
          id="dash-stat"
          value={field}
          onChange={(e) => setField(e.target.value)}
        >
          <option value="">{t('dashboard.statChoose')}</option>
          {free.map((f) => (
            <option key={f.field} value={f.field}>
              {fieldLabel(f.field)}
            </option>
          ))}
        </select>{' '}
        <button
          type="button"
          disabled={full || field === ''}
          onClick={() => {
            onAdd('stat', field)
            setField('')
          }}
        >
          {t('dashboard.addStat')}
        </button>
      </div>
    </section>
  )
}

export function Dashboard() {
  const dispatch = useAppDispatch()
  const snapshot = useAppSelector((s) => s.session.snapshot)
  const calc = useAppSelector((s) => s.session.calc)
  const prefs = useAppSelector((s) => s.ui.prefs)
  const [editing, setEditing] = useState(false)
  const build = snapshot?.build
  const dataLabel = useDataName(build?.['data-version'])
  if (build === null || build === undefined) {
    return null
  }
  const fields = calc?.[1].result.fields ?? []
  const version = build['data-version']
  const dash =
    prefs.dashboards.find((d) => d.id === prefs.active) ?? prefs.dashboards[0]!
  const save = (dashboards: DashboardDef[], active = prefs.active) =>
    void dispatch(setPrefs({ ...prefs, dashboards, active }))
  const change = (f: (list: DashComponent[]) => DashComponent[]) =>
    save(
      prefs.dashboards.map((d) =>
        d.id === dash.id ? { ...d, components: f(d.components) } : d,
      ),
    )
  const ids = () =>
    prefs.dashboards.flatMap((d) => [d.id, ...d.components.map((c) => c.id)])
  const character = [
    build.class != null && build.level != null
      ? t('dashboard.character', {
          class: build.ascendancy ?? build.class,
          level: build.level,
        })
      : null,
    build['main-skill'] != null
      ? t('dashboard.mainSkill', { skill: build['main-skill'] })
      : null,
  ].filter((x) => x !== null)
  return (
    <>
      <div className="page-head">
        <ScreenHeading>{t('dashboard.title')}</ScreenHeading>
        <span className="spacer" />
        {version != null && (
          <Badge tone="neutral">
            <span>{dataLabel}</span>
          </Badge>
        )}
      </div>
      {character.length > 0 && <p className="lead">{character.join(' · ')}</p>}
      <div className="dash-bar">
        <div
          className="dash-tabs"
          role="group"
          aria-label={t('dashboard.dashboards')}
        >
          {prefs.dashboards.map((d) => (
            <button
              type="button"
              key={d.id}
              aria-pressed={d.id === dash.id}
              onClick={() => save(prefs.dashboards, d.id)}
            >
              {d.name}
            </button>
          ))}
          {prefs.dashboards.length < MAX_DASHBOARDS && (
            <button
              type="button"
              aria-label={t('dashboard.addDashboard')}
              title={t('dashboard.addDashboard')}
              onClick={() => {
                const id = newId('dash', ids())
                save(
                  [
                    ...prefs.dashboards,
                    {
                      id,
                      name: t('dashboard.newName', {
                        n: prefs.dashboards.length + 1,
                      }),
                      components: [],
                    },
                  ],
                  id,
                )
                setEditing(true)
              }}
            >
              +
            </button>
          )}
        </div>
        <span className="spacer" />
        <button
          type="button"
          aria-pressed={editing}
          onClick={() => setEditing(!editing)}
        >
          {editing ? t('dashboard.done') : t('dashboard.customize')}
        </button>
      </div>
      {editing && (
        <div className="dash-edit-head">
          <label htmlFor="dash-name">{t('dashboard.name')}</label>{' '}
          <input
            id="dash-name"
            value={dash.name}
            maxLength={40}
            onChange={(e) =>
              save(
                prefs.dashboards.map((d) =>
                  d.id === dash.id ? { ...d, name: e.target.value } : d,
                ),
              )
            }
          />{' '}
          {prefs.dashboards.length > 1 && (
            <button
              type="button"
              onClick={() => {
                const rest = prefs.dashboards.filter((d) => d.id !== dash.id)
                save(rest, rest[0]!.id)
              }}
            >
              {t('dashboard.deleteDashboard')}
            </button>
          )}
        </div>
      )}
      {calc === null && <p>{t('dashboard.noCalc')}</p>}
      {dash.components.length === 0 && !editing && (
        <p className="muted">{t('dashboard.empty')}</p>
      )}
      {/* One grid keyed by component: moving one keeps its elements (and
          the keyboard focus); half-width components share a row, a full
          one takes its own. */}
      <div className="dash-grid">
        {dash.components.map((c, index) => (
          <div key={c.id} className={`dash-cell dash-${c.width}`}>
            {editing && (
              <ComponentTools
                c={c}
                index={index}
                count={dash.components.length}
                onChange={change}
              />
            )}
            <Component
              c={c}
              build={build}
              fields={fields}
              calculated={calc !== null}
            />
          </div>
        ))}
      </div>
      {editing && (
        <Library
          dash={dash}
          fields={fields.filter((f) => f.value != null)}
          onAdd={(kind, field) =>
            change((l) => [
              ...l,
              {
                id: newId('c', ids()),
                kind,
                ...(field === undefined ? {} : { field }),
                width: kind === 'warnings' ? 'full' : 'half',
              },
            ])
          }
        />
      )}
    </>
  )
}
