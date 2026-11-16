// Screen 3 (docs/ui, Workspace) and screen 9 (Customize dashboard, B-5;
// UX audit 2026-11-15, section D): the user's dashboards, each a list of
// components on a six-column grid (`../dashboards`). Fields come from the
// calc result document (never a fixed list), each with its state; a field
// without a value says why, never a blank or a zero. A row is one button:
// label, value, and badge on one line; it opens the stat's breakdown.
// A dashboard is short by default: lists show their first rows, charts
// their plot, and the rest is one click away.
import { useState } from 'react'

import type { BuildView, FieldResult } from '../api/types'
import { Delta, Sparkline } from '../charts'
import { Badge, FieldBadge, Icon, ScreenHeading } from '../components'
import {
  CATALOG,
  CATEGORIES,
  type Category,
  type ComponentKind,
  type DashComponent,
  type DashboardDef,
  MAX_COMPONENTS,
  MAX_DASHBOARDS,
  WIDTHS,
  type Width,
  moved,
  newId,
} from '../dashboards'
import { OFFENCE, fieldLabel, t } from '../i18n'
import { type Mark, actions, markOf, setPrefs } from '../store'
import {
  AllStatsPanel,
  AttributesPanel,
  ComparePanel,
  CurvePanel,
  HistoryPanel,
  Panel,
  PoolsPanel,
  ProgressionPanel,
  ResistancesPanel,
  TimelinePanel,
} from './DashPanels'
import { Inventory } from './Inventory'
import {
  fieldStateText,
  fieldValue,
  numberOf,
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

/** The rows a stat list shows before "Show all" (a list only a little
 * longer shows whole). */
const SHORT = 6

/** The stats a player reads first, in that order; the rest follow in the
 * calculation's order. */
const FIRST = [
  'hit-dps',
  'average-damage',
  'hit-speed',
  'crit-chance',
  'crit-multiplier',
  'hit-chance',
  'life',
  'energy-shield',
  'mana',
  'armour',
  'evasion',
  'block-chance',
  'resistance.fire',
  'resistance.cold',
  'resistance.lightning',
  'resistance.chaos',
]

/** `fields` with the headline stats first. */
function headlineFirst(fields: FieldResult[]): FieldResult[] {
  const rank = (f: FieldResult) => {
    const i = FIRST.indexOf(f.field)
    return i < 0 ? FIRST.length : i
  }
  return fields
    .map((f, i) => ({ f, i }))
    .sort((a, b) => rank(a.f) - rank(b.f) || a.i - b.i)
    .map((x) => x.f)
}

function FieldPanel({
  id,
  title,
  fields,
  hint,
  baseline,
}: {
  id: string
  title: string
  fields: FieldResult[]
  hint?: string
  baseline: Mark | null
}) {
  const dispatch = useAppDispatch()
  const [all, setAll] = useState(false)
  const ordered = headlineFirst(fields)
  const long = ordered.length > SHORT + 2
  const shown = all || !long ? ordered : ordered.slice(0, SHORT)
  return (
    <Panel id={id} title={title} hint={hint}>
      <ul className="stat-list">
        {shown.map((f) => {
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
                  {baseline !== null && (
                    <Delta
                      now={numberOf(f)}
                      before={baseline.fields[f.field]}
                    />
                  )}
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
      {long && (
        <button
          type="button"
          className="link show-all"
          aria-expanded={all}
          onClick={() => setAll(!all)}
        >
          {all
            ? t('dashboard.showFewer')
            : t('dashboard.showAll', { n: fields.length })}
        </button>
      )}
    </Panel>
  )
}

/** The warnings strip: the lines kept but not calculated. */
function WarningsPanel({ build, id }: { build: BuildView; id: string }) {
  const dispatch = useAppDispatch()
  const { used, unused } = splitWarnings(build)
  return (
    <Panel
      id={id}
      title={t('nav.warnings')}
      action={
        <button
          type="button"
          className="link"
          onClick={() => dispatch(actions.go('warnings'))}
        >
          {t('dashboard.viewWarnings')}
        </button>
      }
    >
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
    </Panel>
  )
}

/** The main skill's socket group, its gems with supports marked, and the
 * build's other skills in a line each. */
function MainSkillPanel({ build, id }: { build: BuildView; id: string }) {
  const dispatch = useAppDispatch()
  const group = build['skill-groups'].find((g) => g.main)
  const others = build['skill-groups'].filter(
    (g) => !g.main && g.enabled && g.gems.length > 0,
  )
  const gemName = (g: { name?: string | null }, i: number) =>
    g.name ?? t('skills.gemN', { n: i + 1 })
  return (
    <Panel
      id={id}
      title={t('dashboard.mainSkillTitle')}
      action={
        <button
          type="button"
          className="link"
          onClick={() => dispatch(actions.go('skills'))}
        >
          {t('dashboard.openSkills')}
        </button>
      }
    >
      {group === undefined ? (
        <p className="muted small-text">{t('dashboard.noMainSkill')}</p>
      ) : (
        <ul className="main-skill-gems">
          {group.gems.map((g, i) => (
            <li key={i} className={g.enabled ? undefined : 'muted'}>
              <span
                className={`gem-mark gem-${g.colour ?? 'unknown'}${g.support === true ? ' support' : ''}`}
                aria-hidden="true"
              />
              {g.main ? (
                <strong>{gemName(g, i)}</strong>
              ) : (
                <span>{gemName(g, i)}</span>
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
      {others.length > 0 && (
        <>
          <h3 className="eyebrow other-skills">{t('dashboard.otherSkills')}</h3>
          <ul className="main-skill-gems">
            {others.slice(0, 8).map((g, gi) => {
              const actives = g.gems.filter((x) => x.support !== true)
              const supports = g.gems.length - actives.length
              const first = actives[0] ?? g.gems[0]!
              return (
                <li key={gi}>
                  <span
                    className={`gem-mark gem-${first.colour ?? 'unknown'}`}
                    aria-hidden="true"
                  />
                  <span>
                    {(actives.length > 0 ? actives : [first])
                      .map((x, i) => gemName(x, i))
                      .join(', ')}
                  </span>{' '}
                  {supports > 0 && (
                    <span className="muted small-text">
                      {supports === 1
                        ? t('dashboard.supports1')
                        : t('dashboard.supportsN', { n: supports })}
                    </span>
                  )}
                </li>
              )
            })}
          </ul>
        </>
      )}
    </Panel>
  )
}

/** One calculated number, large (a stat tile), with its change against
 * the pinned numbers and its trend over this session's changes. */
function StatTile({
  field,
  id,
  baseline,
}: {
  field: FieldResult | undefined
  id: string
  baseline: Mark | null
}) {
  const dispatch = useAppDispatch()
  const trail = useAppSelector((s) => s.ui.trail)
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
  const trend = trail
    .map((m) => m.fields[field.field])
    .filter((v): v is number => typeof v === 'number')
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
        <span className="stat-tile-line">
          <span className="stat-tile-value">{fieldValue(field)}</span>
          <Sparkline values={trend} />
        </span>
        {baseline !== null && (
          <Delta now={numberOf(field)} before={baseline.fields[field.field]} />
        )}
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
  baseline,
  onChange,
}: {
  c: DashComponent
  build: BuildView
  fields: FieldResult[]
  calculated: boolean
  baseline: Mark | null
  onChange: (patch: Partial<DashComponent>) => void
}) {
  const id = `dc-${c.id}`
  // Without a calculation the page says so once (above the components).
  switch (c.kind) {
    case 'warnings':
      return <WarningsPanel build={build} id={id} />
    case 'main-skill':
      return <MainSkillPanel build={build} id={id} />
    case 'equipment':
      return <Inventory />
    default:
  }
  if (!calculated) return null
  switch (c.kind) {
    case 'offence':
      return (
        <FieldPanel
          id={id}
          title={t('dashboard.offence')}
          fields={fields.filter((f) => OFFENCE.has(f.field))}
          hint={t('dashboard.hint')}
          baseline={baseline}
        />
      )
    case 'defence':
      return (
        <FieldPanel
          id={id}
          title={t('dashboard.defence')}
          fields={fields.filter((f) => !OFFENCE.has(f.field))}
          baseline={baseline}
        />
      )
    case 'stat':
      return (
        <StatTile
          field={fields.find((f) => f.field === c.field)}
          id={id}
          baseline={baseline}
        />
      )
    case 'attributes':
      return <AttributesPanel id={id} fields={fields} baseline={baseline} />
    case 'resistances':
      return <ResistancesPanel id={id} fields={fields} baseline={baseline} />
    case 'pools':
      return <PoolsPanel id={id} fields={fields} baseline={baseline} />
    case 'all-stats':
      return <AllStatsPanel id={id} fields={fields} baseline={baseline} />
    case 'compare':
      return <ComparePanel id={id} fields={fields} baseline={baseline} />
    case 'history':
      return <HistoryPanel id={id} c={c} fields={fields} onChange={onChange} />
    case 'progression':
      return (
        <ProgressionPanel id={id} c={c} fields={fields} onChange={onChange} />
      )
    case 'curve':
      return (
        <CurvePanel
          id={id}
          c={c}
          build={build}
          fields={fields}
          onChange={onChange}
        />
      )
    case 'timeline':
      return <TimelinePanel id={id} c={c} fields={fields} onChange={onChange} />
    default:
      return null
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
        className="icon"
        aria-label={t('dashboard.moveUp')}
        title={t('dashboard.moveUp')}
        disabled={index === 0}
        onClick={() => onChange((l) => moved(l, index, -1))}
      >
        <Icon name="arrow-left" size={16} />
      </button>
      <button
        type="button"
        className="icon"
        aria-label={t('dashboard.moveDown')}
        title={t('dashboard.moveDown')}
        disabled={index === count - 1}
        onClick={() => onChange((l) => moved(l, index, 1))}
      >
        <Icon name="arrow-right" size={16} />
      </button>
      <select
        aria-label={t('dashboard.width')}
        title={t('dashboard.width')}
        value={c.width}
        onChange={(e) =>
          onChange((l) =>
            l.map((x) =>
              x.id === c.id ? { ...x, width: e.target.value as Width } : x,
            ),
          )
        }
      >
        {WIDTHS.map((w) => (
          <option key={w} value={w}>
            {t(`dashboard.width.${w}`)}
          </option>
        ))}
      </select>
      <button
        type="button"
        className="icon"
        aria-label={t('dashboard.remove')}
        title={t('dashboard.remove')}
        onClick={() => onChange((l) => l.filter((x) => x.id !== c.id))}
      >
        <Icon name="close" size={16} />
      </button>
    </div>
  )
}

/** The component library: the components by group with a search; each
 * summary once per dashboard, a chart or a stat tile as often as wanted. */
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
  const [search, setSearch] = useState('')
  const [category, setCategory] = useState<Category | 'all'>('all')
  const has = (kind: ComponentKind, f?: string) =>
    dash.components.some((c) => c.kind === kind && c.field === f)
  const full = dash.components.length >= MAX_COMPONENTS
  const free = fields.filter((f) => !has('stat', f.field))
  const words = search.trim().toLowerCase()
  const entries = CATALOG.filter(
    (e) =>
      (category === 'all' || e.category === category) &&
      (words === '' ||
        `${t(`dashboard.kind.${e.kind}`)} ${t(`dashboard.kindHelp.${e.kind}`)}`
          .toLowerCase()
          .includes(words)),
  )
  return (
    <section className="panel" aria-labelledby="dash-library">
      <div className="panel-head">
        <h2 id="dash-library">{t('dashboard.library')}</h2>
        <span className="spacer" />
        <input
          type="search"
          className="panel-search"
          value={search}
          placeholder={t('dashboard.librarySearch')}
          aria-label={t('dashboard.librarySearch')}
          onChange={(e) => setSearch(e.target.value)}
        />
      </div>
      <div
        className="segmented"
        role="group"
        aria-label={t('dashboard.categories')}
      >
        {(['all', ...CATEGORIES] as const).map((c) => (
          <button
            type="button"
            key={c}
            aria-pressed={category === c}
            onClick={() => setCategory(c)}
          >
            {t(`dashboard.category.${c}`)}
          </button>
        ))}
      </div>
      {entries.length === 0 ? (
        <p className="muted small-text">{t('dashboard.libraryNone')}</p>
      ) : (
        <ul className="dash-library">
          {entries.map((e) => (
            <li key={e.kind}>
              <span className="dash-library-text">
                <strong>{t(`dashboard.kind.${e.kind}`)}</strong>
                <span className="muted small-text">
                  {t(`dashboard.kindHelp.${e.kind}`)}
                </span>
              </span>
              <button
                type="button"
                disabled={full || (e.many !== true && has(e.kind))}
                aria-label={t('dashboard.add', {
                  name: t(`dashboard.kind.${e.kind}`),
                })}
                onClick={() => onAdd(e.kind)}
              >
                <Icon name="plus" size={16} />
                {t('dashboard.addShort')}
              </button>
            </li>
          ))}
        </ul>
      )}
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

/** Pins the numbers as they are, to compare what comes after against
 * (UX audit D-03); with numbers pinned, says against what and lets go. */
function BaselineControl() {
  const dispatch = useAppDispatch()
  const calc = useAppSelector((s) => s.session.calc)
  const baseline = useAppSelector((s) => s.ui.baseline)
  const pin = () => {
    if (calc !== null) {
      dispatch(actions.baseline(markOf(calc, t('compare.pinnedNow'))))
    }
  }
  if (baseline === null) {
    return (
      <button
        type="button"
        disabled={calc === null}
        title={t('compare.pinHelp')}
        onClick={pin}
      >
        <Icon name="pin" size={16} />
        {t('compare.pin')}
      </button>
    )
  }
  return (
    <span
      className="baseline-control"
      role="group"
      aria-label={t('compare.label')}
    >
      <Badge tone="info">{t('compare.on')}</Badge>
      <button type="button" title={t('compare.pinHelp')} onClick={pin}>
        {t('compare.repin')}
      </button>
      <button type="button" onClick={() => dispatch(actions.baseline(null))}>
        {t('compare.clear')}
      </button>
    </span>
  )
}

export function Dashboard() {
  const dispatch = useAppDispatch()
  const snapshot = useAppSelector((s) => s.session.snapshot)
  const calc = useAppSelector((s) => s.session.calc)
  const prefs = useAppSelector((s) => s.ui.prefs)
  const baseline = useAppSelector((s) => s.ui.baseline)
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
        {character.length > 0 && (
          <p className="page-sub">{character.join(' · ')}</p>
        )}
        <span className="spacer" />
        {version != null && (
          <Badge tone="neutral">
            <span>{dataLabel}</span>
          </Badge>
        )}
      </div>
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
              className="icon"
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
              <Icon name="plus" size={16} />
            </button>
          )}
        </div>
        <span className="spacer" />
        <BaselineControl />
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
          the keyboard focus). */}
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
              baseline={baseline}
              onChange={(patch) =>
                change((l) =>
                  l.map((x) => (x.id === c.id ? { ...x, ...patch } : x)),
                )
              }
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
                width:
                  kind === 'stat'
                    ? 'third'
                    : (CATALOG.find((e) => e.kind === kind)?.width ?? 'half'),
              },
            ])
          }
        />
      )}
    </>
  )
}
