// The dashboard's components beyond the stat lists (UX audit 2026-11-15,
// section D): attributes, pools, resistances, every stat, the comparison
// with pinned numbers, and the charts over time (this session's changes,
// the build's stages, one input swept, a fight). Each is short on the
// dashboard and opens to a larger chart with its numbers as a table.
import { Dialog, Popover } from 'radix-ui'
import { type ReactNode, useEffect, useState } from 'react'

import type {
  BuildView,
  FieldResult,
  WhatIfRequest,
  WhatIfSeries,
} from '../api/types'
import { type ChartPoint, Delta, DeltaBar, LineChart, Meter } from '../charts'
import { compactNumber } from '../chartMath'
import { FieldMark, Icon } from '../components'
import {
  CURVE_VARIES,
  type CurveVary,
  type DashComponent,
  MAX_FIELDS,
} from '../dashboards'
import { fieldLabel, formatNumber, t } from '../i18n'
import { type Mark, actions, markOf, query } from '../store'
import {
  fieldValue,
  inputLabel,
  numberOf,
  useAppDispatch,
  useAppSelector,
  withUnit,
} from '../util'

/** A component's frame: its title, what it offers (settings, the detail
 * view, a link), and its content. */
export function Panel({
  id,
  title,
  hint,
  settings,
  detail,
  action,
  className,
  children,
}: {
  id: string
  title: string
  /** A few words beside the title. */
  hint?: string
  /** The component's own choices (which stat, what range). */
  settings?: ReactNode
  /** The larger view: the chart at size and its numbers as a table. */
  detail?: () => ReactNode
  action?: ReactNode
  className?: string
  children: ReactNode
}) {
  return (
    <section
      className={`panel dash-panel${className ? ` ${className}` : ''}`}
      aria-labelledby={id}
    >
      <div className="panel-head">
        <h2 id={id}>{title}</h2>
        {hint !== undefined && <span className="muted small-text">{hint}</span>}
        <span className="spacer" />
        {action}
        {settings !== undefined && (
          <Popover.Root>
            <Popover.Trigger asChild>
              <button
                type="button"
                className="icon quiet"
                aria-label={t('panel.settings', { name: title })}
                title={t('panel.settings', { name: title })}
              >
                <Icon name="sliders" size={16} />
              </button>
            </Popover.Trigger>
            <Popover.Portal>
              <Popover.Content
                className="popover-content"
                align="end"
                sideOffset={6}
                collisionPadding={8}
              >
                {settings}
              </Popover.Content>
            </Popover.Portal>
          </Popover.Root>
        )}
        {detail !== undefined && (
          <Dialog.Root>
            <Dialog.Trigger asChild>
              <button
                type="button"
                className="icon quiet"
                aria-label={t('panel.detail', { name: title })}
                title={t('panel.detail', { name: title })}
              >
                <Icon name="expand" size={16} />
              </button>
            </Dialog.Trigger>
            <Dialog.Portal>
              <Dialog.Overlay className="dialog-overlay" />
              <Dialog.Content
                className="dialog-content dialog-wide"
                aria-describedby={undefined}
              >
                <div className="panel-head">
                  <Dialog.Title>{title}</Dialog.Title>
                  <span className="spacer" />
                  <Dialog.Close asChild>
                    <button type="button">{t('dialog.close')}</button>
                  </Dialog.Close>
                </div>
                {detail()}
              </Dialog.Content>
            </Dialog.Portal>
          </Dialog.Root>
        )}
      </div>
      {children}
    </section>
  )
}

/** A chart's numbers as a table (the detail view): the same data for
 * anyone who reads rows better than marks. */
function DataTable({
  head,
  rows,
}: {
  head: string[]
  rows: (string | ReactNode)[][]
}) {
  return (
    <div className="table-scroll">
      <table className="data-table">
        <thead>
          <tr>
            {head.map((h) => (
              <th key={h} scope="col">
                {h}
              </th>
            ))}
          </tr>
        </thead>
        <tbody>
          {rows.map((r, i) => (
            <tr key={i}>
              {r.map((cell, j) =>
                j === 0 ? (
                  <th key={j} scope="row">
                    {cell}
                  </th>
                ) : (
                  <td key={j}>{cell}</td>
                ),
              )}
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  )
}

/** A row that opens its stat's breakdown: label, a mark, the value. */
function StatLine({
  field,
  label,
  mark,
  value,
  note,
}: {
  field: FieldResult
  label?: string
  mark?: ReactNode
  value?: string
  note?: ReactNode
}) {
  const dispatch = useAppDispatch()
  return (
    <li>
      <button
        type="button"
        className="stat-row meter-row"
        onClick={() => dispatch(actions.breakdown(field.field))}
      >
        <span className="stat-label">{label ?? fieldLabel(field.field)}</span>
        {mark}
        <span className="stat-end">
          {note}
          <span className="stat-value">{value ?? fieldValue(field)}</span>
          <FieldMark field={field} />
        </span>
      </button>
    </li>
  )
}

const byName = (fields: FieldResult[]) =>
  new Map(fields.map((f) => [f.field, f] as const))

/** Strength, dexterity, and intelligence side by side, each against the
 * largest of the three. */
export function AttributesPanel({
  id,
  fields,
  baseline,
}: {
  id: string
  fields: FieldResult[]
  baseline: Mark | null
}) {
  const by = byName(fields)
  const rows = (
    [
      ['strength', 'life'],
      ['dexterity', 'good'],
      ['intelligence', 'mana'],
    ] as const
  ).flatMap(([name, tone]) => {
    const f = by.get(name)
    return f === undefined ? [] : [{ f, tone }]
  })
  const high = Math.max(1, ...rows.map((r) => numberOf(r.f) ?? 0))
  return (
    <Panel id={id} title={t('dashboard.kind.attributes')}>
      {rows.length === 0 ? (
        <p className="muted small-text">{t('dashboard.statMissing')}</p>
      ) : (
        <ul className="stat-list">
          {rows.map(({ f, tone }) => (
            <StatLine
              key={f.field}
              field={f}
              mark={<Meter value={numberOf(f) ?? 0} high={high} tone={tone} />}
              note={
                baseline !== null && (
                  <Delta now={numberOf(f)} before={baseline.fields[f.field]} />
                )
              }
            />
          ))}
        </ul>
      )}
    </Panel>
  )
}

/** The default resistance cap, marked on each bar (the game's base
 * maximum; a build's own maximum is not a field yet). */
const RESISTANCE_CAP = 75

/** The four resistances on one scale, the cap marked, with how far each
 * is from it. */
export function ResistancesPanel({
  id,
  fields,
  baseline,
}: {
  id: string
  fields: FieldResult[]
  baseline: Mark | null
}) {
  const rows = fields.filter((f) => f.field.startsWith('resistance.'))
  return (
    <Panel
      id={id}
      title={t('dashboard.kind.resistances')}
      hint={t('resist.capHint', { cap: RESISTANCE_CAP })}
    >
      {rows.length === 0 ? (
        <p className="muted small-text">{t('dashboard.statMissing')}</p>
      ) : (
        <ul className="stat-list">
          {rows.map((f) => {
            const v = numberOf(f)
            const short = v === null ? null : RESISTANCE_CAP - v
            return (
              <StatLine
                key={f.field}
                field={f}
                label={t(
                  `resist.${f.field.slice('resistance.'.length)}` as never,
                )}
                mark={
                  v !== null && (
                    <Meter
                      value={v}
                      low={-60}
                      high={90}
                      cap={RESISTANCE_CAP}
                      tone={v >= RESISTANCE_CAP ? 'good' : 'series'}
                    />
                  )
                }
                note={
                  baseline !== null ? (
                    <Delta now={v} before={baseline.fields[f.field]} />
                  ) : short !== null && short > 0 ? (
                    <span className="muted small-text">
                      {t('resist.short', { n: formatNumber(short) })}
                    </span>
                  ) : null
                }
              />
            )
          })}
        </ul>
      )}
    </Panel>
  )
}

/** Life, mana, and energy shield as bars on one scale; the part left
 * after reservation solid, the reserved part lighter behind it. */
export function PoolsPanel({
  id,
  fields,
  baseline,
}: {
  id: string
  fields: FieldResult[]
  baseline: Mark | null
}) {
  const by = byName(fields)
  const rows = (
    [
      ['life', 'life-unreserved', 'life'],
      ['energy-shield', null, 'shield'],
      ['mana', 'mana-unreserved', 'mana'],
    ] as const
  ).flatMap(([name, free, tone]) => {
    const f = by.get(name)
    if (f === undefined) return []
    return [{ f, free: free === null ? null : numberOf(by.get(free)), tone }]
  })
  const high = Math.max(1, ...rows.map((r) => numberOf(r.f) ?? 0))
  return (
    <Panel id={id} title={t('dashboard.kind.pools')}>
      {rows.length === 0 ? (
        <p className="muted small-text">{t('dashboard.statMissing')}</p>
      ) : (
        <ul className="stat-list">
          {rows.map(({ f, free, tone }) => {
            const total = numberOf(f)
            const reserved =
              total !== null && free !== null && free < total
                ? total - free
                : null
            return (
              <StatLine
                key={f.field}
                field={f}
                mark={
                  total !== null && (
                    <Meter
                      value={free ?? total}
                      behind={total}
                      high={high}
                      tone={tone}
                    />
                  )
                }
                note={
                  baseline !== null ? (
                    <Delta now={total} before={baseline.fields[f.field]} />
                  ) : reserved !== null ? (
                    <span className="muted small-text">
                      {t('pools.reserved', { n: formatNumber(reserved) })}
                    </span>
                  ) : null
                }
              />
            )
          })}
        </ul>
      )}
    </Panel>
  )
}

/** Every stat the calculation gives, with a search: the whole result in
 * one place. Short on the dashboard (it scrolls inside its frame). */
export function AllStatsPanel({
  id,
  fields,
  baseline,
}: {
  id: string
  fields: FieldResult[]
  baseline: Mark | null
}) {
  const dispatch = useAppDispatch()
  const [search, setSearch] = useState('')
  const words = search.trim().toLowerCase()
  const rows = fields.filter(
    (f) =>
      words === '' ||
      fieldLabel(f.field).toLowerCase().includes(words) ||
      f.field.includes(words),
  )
  return (
    <Panel
      id={id}
      title={t('dashboard.kind.all-stats')}
      hint={t('allStats.count', { n: fields.length })}
      action={
        <input
          type="search"
          className="panel-search"
          value={search}
          placeholder={t('allStats.search')}
          aria-label={t('allStats.search')}
          onChange={(e) => setSearch(e.target.value)}
        />
      }
    >
      {rows.length === 0 ? (
        <p className="muted small-text">{t('allStats.none')}</p>
      ) : (
        <ul className="stat-list stat-columns">
          {rows.map((f) => (
            <li key={f.field}>
              <button
                type="button"
                className="stat-row"
                onClick={() => dispatch(actions.breakdown(f.field))}
              >
                <span className="stat-label">{fieldLabel(f.field)}</span>
                <span className="stat-end">
                  {baseline !== null && (
                    <Delta
                      now={numberOf(f)}
                      before={baseline.fields[f.field]}
                    />
                  )}
                  <span className="stat-value">{fieldValue(f)}</span>
                  <FieldMark field={f} />
                </span>
              </button>
            </li>
          ))}
        </ul>
      )}
    </Panel>
  )
}

/** The numbers now against the pinned ones: each stat that changed, most
 * changed first, as a bar from the middle. */
export function ComparePanel({
  id,
  fields,
  baseline,
}: {
  id: string
  fields: FieldResult[]
  baseline: Mark | null
}) {
  const dispatch = useAppDispatch()
  const calc = useAppSelector((s) => s.session.calc)
  const pin = () => {
    if (calc !== null) {
      dispatch(actions.baseline(markOf(calc, t('compare.pinnedNow'))))
    }
  }
  if (baseline === null) {
    return (
      <Panel id={id} title={t('dashboard.kind.compare')}>
        <p className="muted small-text">{t('compare.empty')}</p>
        <button type="button" onClick={pin} disabled={calc === null}>
          {t('compare.pin')}
        </button>
      </Panel>
    )
  }
  const rows = fields
    .map((f) => {
      const now = numberOf(f)
      const before = baseline.fields[f.field] ?? null
      const change = now !== null && before !== null ? now - before : null
      const share =
        change === null || before === null
          ? 0
          : before === 0
            ? Math.sign(change)
            : Math.max(-1, Math.min(1, change / Math.abs(before)))
      return { f, now, before, change, share }
    })
    .filter((r) => r.change !== null && Math.abs(r.change) > 1e-9)
    .sort((a, b) => Math.abs(b.share) - Math.abs(a.share))
  const table = () => (
    <DataTable
      head={[
        t('table.stat'),
        t('compare.before'),
        t('compare.now'),
        t('compare.change'),
      ]}
      rows={fields.map((f) => {
        const before = baseline.fields[f.field] ?? null
        return [
          fieldLabel(f.field),
          before === null ? '—' : withUnit(f.field, before),
          fieldValue(f),
          <Delta key="d" now={numberOf(f)} before={before} />,
        ]
      })}
    />
  )
  return (
    <Panel
      id={id}
      title={t('dashboard.kind.compare')}
      hint={t('compare.against', { label: baseline.label })}
      detail={table}
    >
      {rows.length === 0 ? (
        <p className="muted small-text">{t('compare.same')}</p>
      ) : (
        <ul className="stat-list compare-list">
          {rows.slice(0, 8).map(({ f, now, before, share }) => (
            <StatLine
              key={f.field}
              field={f}
              mark={<DeltaBar share={share} />}
              note={<Delta now={now} before={before} />}
            />
          ))}
        </ul>
      )}
      {rows.length > 8 && (
        <p className="muted small-text">
          {t('compare.more', { n: rows.length - 8 })}
        </p>
      )}
    </Panel>
  )
}

/** A stat's choices, in the calculation's order. */
function FieldSelect({
  id,
  value,
  fields,
  onChange,
}: {
  id: string
  value: string
  fields: FieldResult[]
  onChange: (field: string) => void
}) {
  return (
    <select id={id} value={value} onChange={(e) => onChange(e.target.value)}>
      {!fields.some((f) => f.field === value) && (
        <option value={value}>{fieldLabel(value)}</option>
      )}
      {fields.map((f) => (
        <option key={f.field} value={f.field}>
          {fieldLabel(f.field)}
        </option>
      ))}
    </select>
  )
}

type Change = (patch: Partial<DashComponent>) => void

/** One stat after each change this session (the trail), oldest first. */
export function HistoryPanel({
  id,
  c,
  fields,
  onChange,
}: {
  id: string
  c: DashComponent
  fields: FieldResult[]
  onChange: Change
}) {
  const trail = useAppSelector((s) => s.ui.trail)
  const field = c.field ?? 'hit-dps'
  const points: ChartPoint[] = trail.map((m, i) => ({
    label: i === 0 ? t('trail.start') : String(i),
    y: m.fields[field] ?? null,
    note: m.label,
  }))
  const format = (v: number) => withUnit(field, v)
  const title = t('trend.title', { field: fieldLabel(field) })
  const chart = (height: number) => (
    <LineChart
      points={points}
      label={title}
      format={format}
      current={points.length - 1}
      height={height}
      step
    />
  )
  return (
    <Panel
      id={id}
      title={title}
      hint={t('trend.hint')}
      settings={
        <div className="settings">
          <label htmlFor={`${id}-field`}>{t('dashboard.statField')}</label>
          <FieldSelect
            id={`${id}-field`}
            value={field}
            fields={fields}
            onChange={(f) => onChange({ field: f })}
          />
        </div>
      }
      detail={() => (
        <>
          {chart(320)}
          <DataTable
            head={[t('table.step'), t('table.change'), fieldLabel(field)]}
            rows={trail.map((m, i) => [
              i === 0 ? t('trail.start') : String(i),
              m.label,
              m.fields[field] == null ? '—' : format(m.fields[field]),
            ])}
          />
        </>
      )}
    >
      {points.length < 2 ? (
        <p className="muted small-text">{t('trend.empty')}</p>
      ) : (
        chart(150)
      )}
    </Panel>
  )
}

type Loaded =
  | { state: 'loading' }
  | { state: 'failed' }
  | { state: 'done'; series: WhatIfSeries }

/** The what-if query's answer for `request`, asked again when the build
 * changes; `null` request asks nothing. */
function useWhatIf(request: WhatIfRequest | null): Loaded {
  const dispatch = useAppDispatch()
  const version = useAppSelector((s) => s.session.snapshot?.version)
  const key = request === null ? null : JSON.stringify(request)
  const [answer, setAnswer] = useState<{
    key: string
    version: number | undefined
    value: Loaded
  } | null>(null)
  useEffect(() => {
    if (key === null) return
    let live = true
    void dispatch(
      query((api) => api.whatIf(JSON.parse(key) as WhatIfRequest)),
    ).then((series) => {
      if (live) {
        setAnswer({
          key,
          version,
          value:
            series === null ? { state: 'failed' } : { state: 'done', series },
        })
      }
    })
    return () => {
      live = false
    }
  }, [dispatch, key, version])
  if (key === null) return { state: 'failed' }
  // The last answer stays up while the next one is asked (no flicker).
  return answer !== null && answer.key === key
    ? answer.value
    : { state: 'loading' }
}

const valueAt = (
  point: WhatIfSeries['points'][number],
  field: string,
): number | null => {
  const v = point.fields.find((f) => f.field === field)?.value
  return typeof v === 'number' ? v : null
}

function Waiting({ loaded }: { loaded: Loaded }) {
  return (
    <p className="muted small-text" role="status">
      {loaded.state === 'loading' ? t('chart.loading') : t('chart.failed')}
    </p>
  )
}

/** The chosen stats at each stage of the build (PoB's loadouts), one
 * small chart each on its own scale: how the character grows. */
export function ProgressionPanel({
  id,
  c,
  fields,
  onChange,
}: {
  id: string
  c: DashComponent
  fields: FieldResult[]
  onChange: Change
}) {
  const stages = useAppSelector(
    (s) => s.session.snapshot?.build?.loadouts.length ?? 0,
  )
  const loaded = useWhatIf(stages > 1 ? { vary: 'stages' } : null)
  const chosen = c.fields ?? ['hit-dps', 'life']
  const toggle = (field: string) =>
    onChange({
      fields: chosen.includes(field)
        ? chosen.filter((f) => f !== field)
        : [...chosen, field].slice(0, MAX_FIELDS),
    })
  const settings = (
    <fieldset className="settings">
      <legend>{t('progression.choose', { n: MAX_FIELDS })}</legend>
      <div className="settings-checks">
        {fields.map((f) => (
          <label key={f.field} className="check">
            <input
              type="checkbox"
              checked={chosen.includes(f.field)}
              disabled={
                !chosen.includes(f.field) && chosen.length >= MAX_FIELDS
              }
              onChange={() => toggle(f.field)}
            />
            {fieldLabel(f.field)}
          </label>
        ))}
      </div>
    </fieldset>
  )
  const title = t('dashboard.kind.progression')
  if (stages <= 1) {
    return (
      <Panel id={id} title={title} settings={settings}>
        <p className="muted small-text">{t('progression.oneStage')}</p>
      </Panel>
    )
  }
  if (loaded.state !== 'done') {
    return (
      <Panel id={id} title={title} settings={settings}>
        <Waiting loaded={loaded} />
      </Panel>
    )
  }
  const points = loaded.series.points
  const current = points.findIndex((p) => p.current)
  const chart = (field: string, height: number) => (
    <LineChart
      points={points.map((p) => ({ label: p.label, y: valueAt(p, field) }))}
      label={t('progression.chart', { field: fieldLabel(field) })}
      format={(v) => withUnit(field, v)}
      current={current < 0 ? undefined : current}
      height={height}
    />
  )
  return (
    <Panel
      id={id}
      title={title}
      hint={t('progression.hint')}
      settings={settings}
      detail={() => (
        <DataTable
          head={[t('table.stat'), ...points.map((p) => p.label)]}
          rows={fields.map((f) => [
            fieldLabel(f.field),
            ...points.map((p) => {
              const v = valueAt(p, f.field)
              return v === null ? '—' : withUnit(f.field, v)
            }),
          ])}
        />
      )}
    >
      {chosen.length === 0 ? (
        <p className="muted small-text">{t('progression.none')}</p>
      ) : (
        <div className="small-multiples">
          {chosen.map((field) => (
            <figure key={field}>
              <figcaption>{fieldLabel(field)}</figcaption>
              {chart(field, 130)}
            </figure>
          ))}
        </div>
      )}
    </Panel>
  )
}

/** The request a curve asks, from the build as it is: the main gem's
 * levels, the character's levels, or a config number around its value. */
function curveRequest(
  build: BuildView,
  vary: CurveVary,
  input: string | undefined,
): { request: WhatIfRequest; axis: string } | null {
  if (vary === 'character-level') {
    // The levels around the character's own (one point a level, so the
    // build's own level is on the curve), within the query's 41 points.
    const level = build.level ?? 1
    const from = Math.max(1, Math.min(level - 20, 60))
    return {
      request: {
        vary: {
          'character-level': { from, to: Math.min(100, from + 40), step: 1 },
        },
      },
      axis: t('curve.axis.character-level'),
    }
  }
  if (vary === 'gem-level') {
    const group = build['skill-groups'].findIndex((g) => g.main)
    const gem = build['skill-groups'][group]?.gems.findIndex((g) => g.main)
    const view = build['skill-groups'][group]?.gems[gem ?? -1]
    if (group < 0 || gem === undefined || gem < 0 || view === undefined) {
      return null
    }
    return {
      request: {
        vary: {
          'gem-level': { group, gem, from: 1, to: Math.max(21, view.level) },
        },
      },
      axis: t('curve.axis.gem-level', {
        gem: view.name ?? t('skills.gemN', { n: gem + 1 }),
      }),
    }
  }
  const view = build.config?.inputs.find(
    (i) =>
      i.name === input && typeof i.value === 'object' && 'number' in i.value,
  )
  if (view === undefined || !('number' in view.value)) return null
  const now = view.value.number
  // Levels sweep the levels around; other numbers from zero to twice
  // their value (at least ten).
  const level = /level/i.test(view.name)
  const from = level ? Math.max(1, Math.round(now) - 20) : 0
  const to = level ? Math.min(100, Math.round(now) + 20) : Math.max(10, now * 2)
  return {
    request: {
      vary: {
        'config-number': {
          name: view.name,
          from,
          to,
          steps: level ? to - from + 1 : 21,
        },
      },
    },
    axis: inputLabel(view.name),
  }
}

/** One stat as one input changes, everything else as the build has it:
 * what a gem level, a character level, or a config number is worth. */
export function CurvePanel({
  id,
  c,
  build,
  fields,
  onChange,
}: {
  id: string
  c: DashComponent
  build: BuildView
  fields: FieldResult[]
  onChange: Change
}) {
  const vary = c.vary ?? 'gem-level'
  const field = c.field ?? 'hit-dps'
  const numbers = (build.config?.inputs ?? []).filter(
    (i) => typeof i.value === 'object' && 'number' in i.value,
  )
  const input = c.input ?? numbers[0]?.name
  const made = curveRequest(build, vary, input)
  const loaded = useWhatIf(made?.request ?? null)
  const title = t('curve.title', {
    field: fieldLabel(field),
    axis: made?.axis ?? t(`curve.vary.${vary}`),
  })
  const settings = (
    <div className="settings">
      <label htmlFor={`${id}-field`}>{t('dashboard.statField')}</label>
      <FieldSelect
        id={`${id}-field`}
        value={field}
        fields={fields}
        onChange={(f) => onChange({ field: f })}
      />
      <label htmlFor={`${id}-vary`}>{t('curve.varyLabel')}</label>
      <select
        id={`${id}-vary`}
        value={vary}
        onChange={(e) => onChange({ vary: e.target.value as CurveVary })}
      >
        {CURVE_VARIES.map((v) => (
          <option key={v} value={v}>
            {t(`curve.vary.${v}`)}
          </option>
        ))}
      </select>
      {vary === 'config-number' && (
        <>
          <label htmlFor={`${id}-input`}>{t('curve.inputLabel')}</label>
          <select
            id={`${id}-input`}
            value={input ?? ''}
            onChange={(e) => onChange({ input: e.target.value })}
          >
            {numbers.map((i) => (
              <option key={i.name} value={i.name}>
                {inputLabel(i.name)}
              </option>
            ))}
          </select>
        </>
      )}
    </div>
  )
  if (made === null) {
    return (
      <Panel id={id} title={title} settings={settings}>
        <p className="muted small-text">{t(`curve.none.${vary}`)}</p>
      </Panel>
    )
  }
  if (loaded.state !== 'done') {
    return (
      <Panel id={id} title={title} settings={settings}>
        <Waiting loaded={loaded} />
      </Panel>
    )
  }
  const points = loaded.series.points
  const current = points.findIndex((p) => p.current)
  const format = (v: number) => withUnit(field, v)
  const chart = (height: number) => (
    <LineChart
      points={points.map((p) => ({ label: p.label, y: valueAt(p, field) }))}
      label={title}
      format={format}
      current={current < 0 ? undefined : current}
      height={height}
    />
  )
  return (
    <Panel
      id={id}
      title={title}
      settings={settings}
      detail={() => (
        <>
          {chart(320)}
          <DataTable
            head={[made.axis, fieldLabel(field)]}
            rows={points.map((p) => {
              const v = valueAt(p, field)
              return [
                p.current ? `${p.label} · ${t('chart.now')}` : p.label,
                v === null ? '—' : format(v),
              ]
            })}
          />
        </>
      )}
    >
      {chart(150)}
    </Panel>
  )
}

/** Lives to fight through, offered in the settings. */
const TARGETS = [1e6, 5e6, 1e7, 5e7, 1e8]
const DEFAULT_TARGET = 1e7

/** How long a fight lasts on the build's hit DPS: damage dealt over time
 * against a life to get through. An estimate: the same DPS throughout,
 * nothing ramping, the enemy as the configuration sets it. */
export function TimelinePanel({
  id,
  c,
  fields,
  onChange,
}: {
  id: string
  c: DashComponent
  fields: FieldResult[]
  onChange: Change
}) {
  const by = byName(fields)
  const dps = numberOf(by.get('hit-dps'))
  const rate = numberOf(by.get('hit-speed')) ?? numberOf(by.get('speed'))
  const target = c.target ?? DEFAULT_TARGET
  const title = t('dashboard.kind.timeline')
  const settings = (
    <div className="settings">
      <label htmlFor={`${id}-target`}>{t('timeline.target')}</label>
      <select
        id={`${id}-target`}
        value={target}
        onChange={(e) => onChange({ target: Number(e.target.value) })}
      >
        {[...new Set([...TARGETS, target])]
          .sort((a, b) => a - b)
          .map((v) => (
            <option key={v} value={v}>
              {compactNumber(v)}
            </option>
          ))}
      </select>
    </div>
  )
  if (dps === null || dps <= 0) {
    return (
      <Panel id={id} title={title} settings={settings}>
        <p className="muted small-text">{t('timeline.noDps')}</p>
      </Panel>
    )
  }
  const seconds = target / dps
  // The plot runs a little past the kill (at least five seconds), in
  // minutes once it is a long fight.
  const span = Math.max(5, seconds * 1.25)
  const inMinutes = span > 180
  const clock = (at: number) =>
    inMinutes
      ? t('timeline.minutes', { n: formatNumber(Math.round(at / 6) / 10) })
      : t('timeline.seconds', { n: formatNumber(Math.round(at * 10) / 10) })
  const steps = 24
  const points: ChartPoint[] = Array.from({ length: steps + 1 }, (_, i) => {
    const at = (span * i) / steps
    return { label: clock(at), y: dps * at }
  })
  const chart = (height: number) => (
    <LineChart
      points={points}
      label={title}
      format={(v) => compactNumber(Math.round(v))}
      target={{ y: target, label: compactNumber(target) }}
      height={height}
      zero
    />
  )
  const time =
    seconds >= 90
      ? t('timeline.minutes', { n: formatNumber(Math.round(seconds / 6) / 10) })
      : t('timeline.seconds', {
          n: formatNumber(Math.round(seconds * 10) / 10),
        })
  return (
    <Panel
      id={id}
      title={title}
      hint={t('timeline.hint')}
      settings={settings}
      detail={() => (
        <>
          {chart(320)}
          <p className="muted small-text">{t('timeline.estimate')}</p>
          <DataTable
            head={[t('timeline.time'), t('timeline.damage')]}
            rows={points.map((p) => [
              p.label,
              p.y === null ? '—' : formatNumber(Math.round(p.y)),
            ])}
          />
        </>
      )}
    >
      <p className="headline">
        <strong>{time}</strong>{' '}
        <span className="muted small-text">
          {t('timeline.toDeal', { n: compactNumber(target) })}
          {rate !== null &&
            ` · ${t('timeline.hits', { n: formatNumber(Math.ceil(seconds * rate)) })}`}
        </span>
      </p>
      {chart(130)}
    </Panel>
  )
}
