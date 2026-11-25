// Screen 3 (docs/ui, Workspace) and screen 9 (Customize dashboard, B-5;
// UX audit 2026-11-15, section D): the user's dashboards, each a list of
// components on a six-column grid (`../dashboards`). Fields come from the
// calc result document (never a fixed list), each with its state; a field
// without a value says why, never a blank or a zero. A row is one button:
// label, value, and badge on one line; it opens the stat's breakdown.
// A dashboard is short by default: lists show their first rows, charts
// their plot, and the rest is one click away.
import { DropdownMenu, Popover } from 'radix-ui'
import { type KeyboardEvent, type PointerEvent, useRef, useState } from 'react'

import type { BuildView, FieldResult } from '../api/types'
import { Delta, Sparkline } from '../charts'
import { Badge, FieldBadge, Hint, Icon, ScreenHeading } from '../components'
import {
  CATALOG,
  CATEGORIES,
  type ComponentKind,
  type DashComponent,
  type DashboardDef,
  MAX_COMPONENTS,
  MAX_DASHBOARDS,
  WIDTHS,
  type Width,
  dropTarget,
  dropped,
  moved,
  newId,
  stepWidth,
} from '../dashboards'
import { OFFENCE, fieldLabel, t } from '../i18n'
import {
  type Mark,
  actions,
  attempt,
  markOf,
  setPrefs,
  setBuildDashboards,
  setKept,
} from '../store'
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
import { useBuildDashboards } from '../buildDashboards'
import { ReasonSummary } from '../ReasonDetails'
import { entriesOf } from '../reasons'
import { MAX_NAME, MAX_SNAPSHOTS, markOfKept } from '../kept'

/** A field's one reason about no particular thing, as its sentence (a
 * row says it; several reasons are behind the card's Details and the
 * breakdown). */
function loneReason(f: FieldResult): string | null {
  const entries = entriesOf(f)
  const first = entries[0]
  return entries.length === 1 && first?.target.kind === 'none'
    ? reasonText(first.code)
    : null
}

/** The rows a stat list shows before "Show all" (a list only a little
 * longer shows whole). */
const SHORT = 6

/** The stats a player reads first, in that order; the rest follow in the
 * calculation's order. */
const FIRST = [
  'hit-dps',
  'dot-dps',
  'ignite-dps',
  'poe1.burning-ground-dps',
  'bleed-dps',
  'poison-dps',
  'poe1.impale-dps',
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
  // Several numbers with the same lone reason are said once, under the
  // rows (not one sentence a row).
  const bare = new Map<string, FieldResult[]>()
  for (const f of fields) {
    const reason = numberOf(f) === null ? loneReason(f) : null
    if (reason !== null) bare.set(reason, [...(bare.get(reason) ?? []), f])
  }
  const folded = [...bare].filter(([, list]) => list.length > 2)
  const hidden = new Set(folded.flatMap(([, list]) => list))
  const ordered = headlineFirst(fields.filter((f) => !hidden.has(f)))
  const long = ordered.length > SHORT + 2
  const shown = all || !long ? ordered : ordered.slice(0, SHORT)
  return (
    <Panel id={id} title={title} hint={hint}>
      <ul className="stat-list">
        {shown.map((f) => {
          const reason = loneReason(f)
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
      {folded.map(([reason, list]) => (
        <p key={reason} className="reason-summary">
          {t('dashboard.folded', {
            n: list.length,
            reason,
            list: list.map((f) => fieldLabel(f.field)).join(', '),
          })}
        </p>
      ))}
      {long && (
        <button
          type="button"
          className="link show-all"
          aria-expanded={all}
          onClick={() => setAll(!all)}
        >
          {all
            ? t('dashboard.showFewer')
            : t('dashboard.showAll', { n: ordered.length })}
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
                <span className="line-text">{w.shown ?? w.text}</span>
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
  fields,
  skill,
  id,
  baseline,
}: {
  field: FieldResult | undefined
  /** Every field: a minion build's hit points at its minion's. */
  fields: FieldResult[]
  /** The main skill's name. */
  skill: string | null | undefined
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
  const trend = trail
    .map((m) => m.fields[field.field])
    .filter((v): v is number => typeof v === 'number')
  // A minion build's own hit: the damage is the minions' (maintainer
  // 2026-11-24).
  const minion =
    field.field === 'hit-dps' && numberOf(field) === null
      ? fields.find((f) => f.field === 'minion.hit-dps')
      : undefined
  if (minion !== undefined) {
    return (
      <section className="panel stat-tile" aria-labelledby={id}>
        <div className="stat-tile-button">
          <h2 id={id} className="small-text">
            {fieldLabel(field.field)}
          </h2>
          <span className="stat-tile-words">{t('dashboard.minionDamage')}</span>
        </div>
        <p className="reason-summary">
          {skill == null
            ? t('dashboard.minionDeals')
            : t('dashboard.minionSkill', { skill })}
          {numberOf(minion) === null && ` ${t('dashboard.minionNotYet')}`}
          {' · '}
          <button
            type="button"
            className="link"
            onClick={() => dispatch(actions.breakdown(minion.field))}
          >
            {numberOf(minion) === null
              ? t('dashboard.minionWhy')
              : t('dashboard.minionLink', { value: fieldValue(minion) })}
          </button>
        </p>
      </section>
    )
  }
  return (
    <section className="panel stat-tile" aria-labelledby={id}>
      <button
        type="button"
        className="stat-tile-button"
        onClick={() => dispatch(actions.breakdown(field.field))}
      >
        <span className="stat-tile-head">
          <h2 id={id} className="small-text">
            {fieldLabel(field.field)}
          </h2>
          {field.state !== 'complete' && <FieldBadge field={field} />}
        </span>
        <span className="stat-tile-line">
          <span className="stat-tile-value">{fieldValue(field)}</span>
          <Sparkline values={trend} />
        </span>
        {baseline !== null && (
          <Delta now={numberOf(field)} before={baseline.fields[field.field]} />
        )}
      </button>
      <ReasonSummary field={field} />
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
          fields={fields}
          skill={build['main-skill']}
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

/** A drag of a component by its handle. */
interface DragHandlers {
  down: (e: PointerEvent<HTMLElement>, id: string) => void
  move: (e: PointerEvent<HTMLElement>) => void
  up: () => void
  cancel: () => void
  key: (e: KeyboardEvent<HTMLButtonElement>, index: number) => void
}

/** A card's controls while customizing (maintainer 2026-11-24): its bar
 * is the handle (drag it anywhere on the grid; from the grip, the arrow
 * keys move the card and Shift with them resizes it), and one menu holds
 * the width, the moves to the top and the bottom, and Remove. */
function ComponentTools({
  c,
  index,
  count,
  onChange,
  drag,
}: {
  c: DashComponent
  index: number
  count: number
  onChange: (f: (list: DashComponent[]) => DashComponent[]) => void
  drag: DragHandlers
}) {
  const name = componentName(c)
  const setWidth = (width: Width) =>
    onChange((l) => l.map((x) => (x.id === c.id ? { ...x, width } : x)))
  return (
    // The shell turns off the browser's own drag and drop, so this is
    // pointer events (UX audit follow-up).
    <div
      className="dash-tools"
      role="group"
      aria-label={t('dashboard.tools', { name })}
      onPointerDown={(e) => {
        // The card's menu is drawn elsewhere but its events come up
        // through here: only a press on the bar itself starts a drag.
        if (e.currentTarget.contains(e.target as Node)) drag.down(e, c.id)
      }}
      onPointerMove={drag.move}
      onPointerUp={drag.up}
      onPointerCancel={drag.cancel}
    >
      <button
        type="button"
        className="icon quiet drag-handle"
        aria-label={t('dashboard.drag', { name })}
        title={t('dashboard.dragHelp')}
        onKeyDown={(e) => {
          const by =
            e.key === 'ArrowLeft' || e.key === 'ArrowUp'
              ? -1
              : e.key === 'ArrowRight' || e.key === 'ArrowDown'
                ? 1
                : 0
          if (e.shiftKey && by !== 0) {
            e.preventDefault()
            setWidth(stepWidth(c.width, by))
          } else {
            drag.key(e, index)
          }
        }}
      >
        <Icon name="grip" size={16} />
      </button>
      <strong className="small-text">{name}</strong>
      <span className="spacer" />
      <DropdownMenu.Root>
        <DropdownMenu.Trigger asChild>
          <button
            type="button"
            className="icon quiet"
            aria-label={t('dashboard.cardMenu', { name })}
            title={t('dashboard.cardMenu', { name })}
            // The menu's button is not the handle.
            onPointerDown={(e) => e.stopPropagation()}
          >
            <span aria-hidden="true">⋯</span>
          </button>
        </DropdownMenu.Trigger>
        <DropdownMenu.Portal>
          <DropdownMenu.Content
            className="menu-content"
            align="end"
            sideOffset={6}
          >
            <DropdownMenu.Label className="menu-label">
              {t('dashboard.width')}
            </DropdownMenu.Label>
            <DropdownMenu.RadioGroup
              value={c.width}
              onValueChange={(w) => setWidth(w as Width)}
            >
              {WIDTHS.map((w) => (
                <DropdownMenu.RadioItem
                  key={w}
                  value={w}
                  className="menu-item menu-radio"
                >
                  {t(`dashboard.width.${w}`)}
                </DropdownMenu.RadioItem>
              ))}
            </DropdownMenu.RadioGroup>
            <DropdownMenu.Separator className="menu-separator" />
            <DropdownMenu.Item
              className="menu-item"
              disabled={index === 0}
              onSelect={() => onChange((l) => moved(l, index, -l.length))}
            >
              {t('dashboard.moveTop')}
            </DropdownMenu.Item>
            <DropdownMenu.Item
              className="menu-item"
              disabled={index === count - 1}
              onSelect={() => onChange((l) => moved(l, index, l.length))}
            >
              {t('dashboard.moveBottom')}
            </DropdownMenu.Item>
            <DropdownMenu.Separator className="menu-separator" />
            <DropdownMenu.Item
              className="menu-item"
              onSelect={() => onChange((l) => l.filter((x) => x.id !== c.id))}
            >
              {t('dashboard.remove')}
            </DropdownMenu.Item>
          </DropdownMenu.Content>
        </DropdownMenu.Portal>
      </DropdownMenu.Root>
    </div>
  )
}

/** The Add menu of the edit toolbar: the components by group, each
 * summary once per dashboard, a chart or a value as often as wanted. */
function AddMenu({
  dash,
  fields,
  onAdd,
}: {
  dash: DashboardDef
  fields: FieldResult[]
  onAdd: (kind: ComponentKind, field?: string) => void
}) {
  const has = (kind: ComponentKind, f?: string) =>
    dash.components.some((c) => c.kind === kind && c.field === f)
  const full = dash.components.length >= MAX_COMPONENTS
  const free = fields.filter((f) => !has('stat', f.field))
  return (
    <DropdownMenu.Root>
      <DropdownMenu.Trigger asChild>
        <button type="button" className="primary" disabled={full}>
          <Icon name="plus" size={16} />
          {t('dashboard.addShort')}
          <Icon name="chevron" size={14} />
        </button>
      </DropdownMenu.Trigger>
      <DropdownMenu.Portal>
        <DropdownMenu.Content
          className="menu-content"
          align="start"
          sideOffset={6}
        >
          <DropdownMenu.Sub>
            <DropdownMenu.SubTrigger className="menu-item menu-sub">
              {t('dashboard.addValue')}
            </DropdownMenu.SubTrigger>
            <DropdownMenu.Portal>
              <DropdownMenu.SubContent className="menu-content menu-long">
                {free.map((f) => (
                  <DropdownMenu.Item
                    key={f.field}
                    className="menu-item"
                    onSelect={() => onAdd('stat', f.field)}
                  >
                    {fieldLabel(f.field)}
                  </DropdownMenu.Item>
                ))}
              </DropdownMenu.SubContent>
            </DropdownMenu.Portal>
          </DropdownMenu.Sub>
          {CATEGORIES.map((category) => (
            <DropdownMenu.Sub key={category}>
              <DropdownMenu.SubTrigger className="menu-item menu-sub">
                {t(`dashboard.category.${category}`)}
              </DropdownMenu.SubTrigger>
              <DropdownMenu.Portal>
                <DropdownMenu.SubContent className="menu-content">
                  {CATALOG.filter((e) => e.category === category).map((e) => (
                    <DropdownMenu.Item
                      key={e.kind}
                      className="menu-item menu-two"
                      disabled={e.many !== true && has(e.kind)}
                      aria-label={t('dashboard.add', {
                        name: t(`dashboard.kind.${e.kind}`),
                      })}
                      onSelect={() => onAdd(e.kind)}
                    >
                      <span>{t(`dashboard.kind.${e.kind}`)}</span>
                      <span className="muted small-text">
                        {t(`dashboard.kindHelp.${e.kind}`)}
                      </span>
                    </DropdownMenu.Item>
                  ))}
                </DropdownMenu.SubContent>
              </DropdownMenu.Portal>
            </DropdownMenu.Sub>
          ))}
        </DropdownMenu.Content>
      </DropdownMenu.Portal>
    </DropdownMenu.Root>
  )
}

/** A text field that changes its value when left or on Enter, not on
 * every key (a change to a build's own dashboards is a change to the
 * build). */
function CommitField({
  id,
  value,
  onCommit,
}: {
  id: string
  value: string
  onCommit: (value: string) => void
}) {
  const [draft, setDraft] = useState<string | null>(null)
  const commit = () => {
    if (draft !== null && draft !== value) onCommit(draft)
    setDraft(null)
  }
  return (
    <input
      id={id}
      value={draft ?? value}
      maxLength={40}
      onChange={(e) => setDraft(e.target.value)}
      onBlur={commit}
      onKeyDown={(e) => {
        if (e.key === 'Enter') commit()
        if (e.key === 'Escape') setDraft(null)
      }}
    />
  )
}

/** The snapshots kept for the build on this computer, by name: keep the
 * numbers as they are, compare against a kept one, or against another
 * build file. */
function Snapshots() {
  const dispatch = useAppDispatch()
  const calc = useAppSelector((s) => s.session.calc)
  const baseline = useAppSelector((s) => s.ui.baseline)
  const kept = useAppSelector((s) => s.ui.kept)
  const [name, setName] = useState('')
  const full = kept.length >= MAX_SNAPSHOTS
  const keep = () => {
    const title = name.trim()
    if (calc === null || title === '' || full) return
    const mark = markOf(calc, title)
    void dispatch(
      setKept([
        ...kept,
        {
          id: newId(
            's',
            kept.map((k) => k.id),
          ),
          name: title,
          fields: mark.fields,
        },
      ]),
    )
    setName('')
  }
  const compareFile = async () => {
    const other = await dispatch(attempt((api) => api.compareFile()))
    if (other == null) return
    dispatch(
      actions.baseline({
        version: -1,
        label: other.name === '' ? t('snapshots.otherBuild') : other.name,
        fields: Object.fromEntries(
          other.fields.map((f) => [f.field, f.value ?? null]),
        ),
      }),
    )
  }
  return (
    <Popover.Root>
      <Popover.Trigger asChild>
        <button type="button" title={t('snapshots.help')}>
          <Icon name="camera" size={16} />
          {t('snapshots.title')}
          {kept.length > 0 && <Badge tone="neutral">{kept.length}</Badge>}
        </button>
      </Popover.Trigger>
      <Popover.Portal>
        <Popover.Content
          className="popover-content snapshots"
          align="end"
          sideOffset={6}
          collisionPadding={8}
        >
          <div className="panel-head">
            <h2 className="small-text">{t('snapshots.title')}</h2>
            <Hint>{t('snapshots.help')}</Hint>
          </div>
          {kept.length === 0 ? (
            <p className="muted small-text">{t('snapshots.none')}</p>
          ) : (
            <ul className="snapshot-list">
              {kept.map((k) => {
                const on = baseline?.version === -1 && baseline.label === k.name
                return (
                  <li key={k.id}>
                    <button
                      type="button"
                      className="snapshot-name"
                      aria-pressed={on}
                      aria-label={t('snapshots.compare', { name: k.name })}
                      onClick={() =>
                        dispatch(actions.baseline(on ? null : markOfKept(k)))
                      }
                    >
                      {k.name}
                    </button>
                    <button
                      type="button"
                      className="icon quiet"
                      aria-label={t('snapshots.delete', { name: k.name })}
                      title={t('snapshots.delete', { name: k.name })}
                      onClick={() => {
                        if (on) dispatch(actions.baseline(null))
                        void dispatch(
                          setKept(kept.filter((x) => x.id !== k.id)),
                        )
                      }}
                    >
                      <Icon name="close" size={16} />
                    </button>
                  </li>
                )
              })}
            </ul>
          )}
          <form
            className="snapshot-keep"
            onSubmit={(e) => {
              e.preventDefault()
              keep()
            }}
          >
            <input
              value={name}
              maxLength={MAX_NAME}
              placeholder={t('snapshots.namePlaceholder')}
              aria-label={t('snapshots.name')}
              disabled={full}
              onChange={(e) => setName(e.target.value)}
            />
            <button
              type="submit"
              disabled={calc === null || full || name.trim() === ''}
            >
              {t('snapshots.keep')}
            </button>
          </form>
          {full && (
            <p className="muted small-text">
              {t('snapshots.full', { n: MAX_SNAPSHOTS })}
            </p>
          )}
          <button
            type="button"
            className="link"
            onClick={() => void compareFile()}
          >
            {t('snapshots.file')}
          </button>
        </Popover.Content>
      </Popover.Portal>
    </Popover.Root>
  )
}

/** Pins the numbers as they are, to compare what comes after against
 * (UX audit D-03); with numbers compared, says against what and lets go.
 * Beside it, the kept snapshots. */
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
      <>
        <button
          type="button"
          disabled={calc === null}
          title={t('compare.pinHelp')}
          onClick={pin}
        >
          <Icon name="pin" size={16} />
          {t('compare.pin')}
        </button>
        <Snapshots />
      </>
    )
  }
  return (
    <>
      <span
        className="baseline-control"
        role="group"
        aria-label={t('compare.label')}
      >
        <Badge tone="info">{t('compare.on')}</Badge>
        <span className="muted small-text baseline-with">
          {t('compare.with', { label: baseline.label })}
        </span>
        <button type="button" title={t('compare.pinHelp')} onClick={pin}>
          {t('compare.repin')}
        </button>
        <button type="button" onClick={() => dispatch(actions.baseline(null))}>
          {t('compare.clear')}
        </button>
      </span>
      <Snapshots />
    </>
  )
}

export function Dashboard() {
  const dispatch = useAppDispatch()
  const snapshot = useAppSelector((s) => s.session.snapshot)
  const calc = useAppSelector((s) => s.session.calc)
  const prefs = useAppSelector((s) => s.ui.prefs)
  const baseline = useAppSelector((s) => s.ui.baseline)
  const shown = useAppSelector((s) => s.ui.dashActive)
  const own = useBuildDashboards()
  const [editing, setEditing] = useState(false)
  const [drag, setDrag] = useState<{
    id: string
    target: { id: string; after: boolean } | null
  } | null>(null)
  const grid = useRef<HTMLDivElement>(null)
  const build = snapshot?.build
  const dataLabel = useDataName(build?.['data-version'])
  if (build === null || build === undefined) {
    return null
  }
  const fields = calc?.[1].result.fields ?? []
  const version = build['data-version']
  // The build's own dashboards when it keeps a set, else the user's.
  const list = own?.list ?? prefs.dashboards
  const activeId = own === undefined ? prefs.active : (shown ?? own.active)
  const dash = list.find((d) => d.id === activeId) ?? list[0]!
  const save = (dashboards: DashboardDef[], active = dash.id) => {
    if (own === undefined) {
      void dispatch(setPrefs({ ...prefs, dashboards, active }))
      return
    }
    // Which of a build's dashboards is shown is not a change to the
    // build; what they hold is.
    dispatch(actions.dashActive(active))
    if (dashboards !== list) {
      void dispatch(setBuildDashboards({ list: dashboards, active }))
    }
  }
  const change = (f: (list: DashComponent[]) => DashComponent[]) =>
    save(
      list.map((d) =>
        d.id === dash.id ? { ...d, components: f(d.components) } : d,
      ),
    )
  const ids = () =>
    list.flatMap((d) => [d.id, ...d.components.map((c) => c.id)])
  const forBuild = (on: boolean) => {
    if (on === (own !== undefined)) return
    dispatch(actions.dashActive(null))
    if (on) {
      void dispatch(
        setBuildDashboards({ list: prefs.dashboards, active: dash.id }),
      )
    } else {
      void dispatch(setBuildDashboards(null))
    }
  }
  const handlers: DragHandlers = {
    down(e, id) {
      if (e.button !== 0) return
      e.preventDefault()
      e.currentTarget.setPointerCapture?.(e.pointerId)
      setDrag({ id, target: null })
    },
    move(e) {
      if (drag === null || grid.current === null) return
      const cells = [
        ...grid.current.querySelectorAll<HTMLElement>('[data-cell]'),
      ].map((el) => {
        const r = el.getBoundingClientRect()
        return {
          id: el.dataset.cell ?? '',
          left: r.left,
          top: r.top,
          right: r.right,
          bottom: r.bottom,
        }
      })
      const target = dropTarget(cells, e.clientX, e.clientY)
      setDrag({
        id: drag.id,
        target: target !== null && target.id !== drag.id ? target : null,
      })
    },
    up() {
      if (drag?.target != null) {
        const { id, target } = drag
        change((l) => dropped(l, id, target.id, target.after))
      }
      setDrag(null)
    },
    cancel() {
      setDrag(null)
    },
    key(e, index) {
      if (e.key === 'Escape') setDrag(null)
      const by =
        e.key === 'ArrowLeft' || e.key === 'ArrowUp'
          ? -1
          : e.key === 'ArrowRight' || e.key === 'ArrowDown'
            ? 1
            : 0
      if (by === 0) return
      e.preventDefault()
      change((l) => moved(l, index, by))
    },
  }
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
          {list.map((d) => (
            <button
              type="button"
              key={d.id}
              aria-pressed={d.id === dash.id}
              onClick={() => save(list, d.id)}
            >
              {d.name}
            </button>
          ))}
          {list.length < MAX_DASHBOARDS && (
            <button
              type="button"
              className="icon"
              aria-label={t('dashboard.addDashboard')}
              title={t('dashboard.addDashboard')}
              onClick={() => {
                const id = newId('dash', ids())
                save(
                  [
                    ...list,
                    {
                      id,
                      name: t('dashboard.newName', { n: list.length + 1 }),
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
          {own !== undefined && (
            <Badge tone="neutral">{t('dashboard.scopeBadge')}</Badge>
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
        <div
          className="dash-edit-head"
          role="toolbar"
          aria-label={t('dashboard.editBar')}
        >
          <AddMenu
            dash={dash}
            fields={fields.filter((f) => f.value != null)}
            onAdd={(kind, field) =>
              // A new card goes to the top, where it is seen; drag it after.
              change((l) => [
                {
                  id: newId('c', ids()),
                  kind,
                  ...(field === undefined ? {} : { field }),
                  width:
                    kind === 'stat'
                      ? 'third'
                      : (CATALOG.find((e) => e.kind === kind)?.width ?? 'half'),
                },
                ...l,
              ])
            }
          />
          <label htmlFor="dash-name">{t('dashboard.name')}</label>{' '}
          <CommitField
            id="dash-name"
            key={dash.id}
            value={dash.name}
            onCommit={(name) =>
              save(list.map((d) => (d.id === dash.id ? { ...d, name } : d)))
            }
          />{' '}
          {list.length > 1 && (
            <button
              type="button"
              onClick={() => {
                const rest = list.filter((d) => d.id !== dash.id)
                save(rest, rest[0]!.id)
              }}
            >
              {t('dashboard.deleteDashboard')}
            </button>
          )}
          <span className="spacer" />
          <span className="small-text muted" id="dash-scope">
            {t('dashboard.scope')}
          </span>
          <div className="segmented" role="group" aria-labelledby="dash-scope">
            <button
              type="button"
              aria-pressed={own === undefined}
              onClick={() => forBuild(false)}
            >
              {t('dashboard.scope.user')}
            </button>
            <button
              type="button"
              aria-pressed={own !== undefined}
              onClick={() => forBuild(true)}
            >
              {t('dashboard.scope.build')}
            </button>
          </div>
          <Hint>{t('dashboard.scopeHelp')}</Hint>
          {own !== undefined && (
            <button
              type="button"
              onClick={() =>
                void dispatch(
                  setPrefs({ ...prefs, dashboards: list, active: dash.id }),
                )
              }
            >
              {t('dashboard.scopeDefault')}
            </button>
          )}
        </div>
      )}
      {editing && (
        <p className="muted small-text dash-edit-help">
          {t('dashboard.editHelp')}
        </p>
      )}
      {calc === null && <p>{t('dashboard.noCalc')}</p>}
      {dash.components.length === 0 && !editing && (
        <p className="muted">{t('dashboard.empty')}</p>
      )}
      {/* One grid keyed by component: moving one keeps its elements (and
          the keyboard focus). */}
      <div className="dash-grid" ref={grid}>
        {dash.components.map((c, index) => (
          <div
            key={c.id}
            data-cell={c.id}
            className={`dash-cell dash-${c.width}${
              drag?.id === c.id ? ' dragging' : ''
            }${
              drag?.target?.id === c.id
                ? drag.target.after
                  ? ' drop-after'
                  : ' drop-before'
                : ''
            }`}
          >
            {editing && (
              <ComponentTools
                c={c}
                index={index}
                count={dash.components.length}
                onChange={change}
                drag={handlers}
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
    </>
  )
}
