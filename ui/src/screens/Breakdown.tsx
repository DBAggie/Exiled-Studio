// Screen 4 (docs/ui, Breakdown): why a field has its value. The steps the
// value is built from, as an outline in words; the sources of the step the
// user picks, named (the item and its line, the passive, the gem, the
// configuration input) and grouped by what they change; what is not
// counted; and the whole provenance graph for those who want every node.
import { useEffect, useMemo, useState } from 'react'

import type {
  BuildView,
  CalcResultDocumentV1,
  ItemView,
  Node,
} from '../api/types'
import { FieldBadge, ScreenHeading } from '../components'
import { OFFENCE, fieldLabel, formatNumber, t, tOr } from '../i18n'
import { actions, query } from '../store'
import {
  fieldValue,
  itemName,
  reasonText,
  useAppDispatch,
  useAppSelector,
} from '../util'

type Graph = CalcResultDocumentV1['result']['graph']
type Contribution = Extract<
  Node['kind'],
  { contribution: unknown }
>['contribution']

/** What the labels need besides the graph. */
interface Context {
  graph: Graph
  build: BuildView
  /** The field each field node is the result of. */
  fieldOf: Map<number, string>
  /** Items read for their line texts, by ID. */
  lines: Record<number, ItemView>
}

/** A name from an ID: `poe1.weapon.attack-speed` → "Weapon attack
 * speed"; a trailing min or max as "(minimum)", "(maximum)". */
function humanize(id: string): string {
  const parts = id
    .replace(/^poe1\./, '')
    .replace(/^step\./, '')
    .split('.')
  const last = parts.at(-1)
  const bound =
    last === 'min'
      ? t('breakdown.minimum')
      : last === 'max'
        ? t('breakdown.maximum')
        : null
  if (bound !== null) parts.pop()
  const text = parts.join(' ').replace(/[-_]/g, ' ')
  const out = text.charAt(0).toUpperCase() + text.slice(1)
  return bound === null ? out : `${out} (${bound})`
}

/** A configuration input's name: `multiplierRage` → "Rage",
 * `useFrenzyCharges` → "Frenzy charges". */
function configName(input: string): string {
  const key = `config.${input}`
  const bare = input.replace(
    /^(use|condition|multiplier|override|enemy)(?=[A-Z])/,
    '',
  )
  const words = bare.replace(/([a-z0-9])([A-Z])/g, '$1 $2').toLowerCase()
  return tOr(key, words.charAt(0).toUpperCase() + words.slice(1))
}

function quantityText(q: string): string {
  return tOr(`quantity.${q}`, humanize(q))
}

/** PoB's `{range:…}`-style prefixes left out of a line's text. */
function plainLine(text: string): string {
  return text.replace(/\{[^}]*\}/g, '').trim()
}

/** A source as a name and what it is ("Passive tree", the item's line). */
function sourceInfo(
  source: Contribution['source'],
  ctx: Context,
): { name: string; detail: string | null; line?: boolean } {
  if (source === 'character')
    return { name: t('breakdown.source.character'), detail: null }
  if (source === 'enemy')
    return { name: t('breakdown.source.enemy'), detail: null }
  if (typeof source !== 'object' || source === null) {
    return { name: String(source), detail: null }
  }
  const [kind, raw] = Object.entries(source)[0] ?? ['', {}]
  const p = (raw ?? {}) as Record<string, string | number>
  switch (kind) {
    case 'item':
    case 'item-base': {
      const id = Number(p.item)
      const item = ctx.build.items.find((i) => i.id === id)
      const name = item === undefined ? `#${id}` : itemName(item)
      const slot =
        item?.slot ??
        (item?.['tree-socket'] != null ? t('items.treeSocket') : null)
      const full = slot == null ? name : `${slot} · ${name}`
      if (kind === 'item-base') {
        return { name: full, detail: t('breakdown.source.itemBase') }
      }
      const text = ctx.lines[id]?.lines[Number(p.line)]?.text
      return {
        name: full,
        detail:
          text === undefined
            ? t('breakdown.source.itemLine', { line: Number(p.line) + 1 })
            : plainLine(text),
        line: text !== undefined,
      }
    }
    case 'passive': {
      const node = Number(p.node)
      const named = ctx.build['passive-names'].find((n) => n.node === node)
      return {
        name: named?.name ?? t('breakdown.source.passiveN', { node }),
        detail: t('breakdown.source.passive'),
      }
    }
    case 'gem': {
      const group = ctx.build['skill-groups'][Number(p.group)]
      const gem = group?.gems[Number(p.gem)]?.name
      const where =
        [group?.label, group?.slot].find((x) => x != null && x !== '') ?? null
      return {
        name:
          gem ??
          t('breakdown.source.gemN', {
            gem: Number(p.gem) + 1,
            group: Number(p.group) + 1,
          }),
        detail:
          where === null
            ? t('breakdown.source.gem')
            : t('breakdown.source.gemIn', { group: where }),
      }
    }
    case 'config':
      return {
        name: configName(String(p.input)),
        detail: t('breakdown.source.config'),
      }
    case 'rule':
      return {
        name: tOr(`rule.${p.rule}`, humanize(String(p.rule))),
        detail: t('breakdown.source.rule'),
      }
    case 'custom-modifier':
    case 'config-input-line':
      return {
        name: t('breakdown.source.custom'),
        detail: t('breakdown.source.itemLine', { line: Number(p.line) + 1 }),
      }
    default:
      return { name: humanize(kind), detail: null }
  }
}

/** A contribution's effect in the game's words: "+19", "+10% increased",
 * "36% more". */
function effectText(c: Contribution, value: Node['value']): string {
  if (typeof value !== 'number') return '—'
  const n = formatNumber(Math.abs(value))
  switch (c.op) {
    case 'base':
      return value < 0 ? `−${n}` : `+${n}`
    case 'increased':
      return value < 0
        ? t('breakdown.effect.reduced', { n })
        : t('breakdown.effect.increased', { n })
    case 'more':
      return value < 0
        ? t('breakdown.effect.less', { n })
        : t('breakdown.effect.more', { n })
    case 'flag':
      return t('breakdown.effect.flag')
    default:
      return formatNumber(value)
  }
}

const OPERATOR: Record<string, string> = {
  sum: '+',
  product: '×',
  'percent-factor': '×',
  difference: '−',
  quotient: '÷',
}

const isContribution = (n: Node | undefined) =>
  n !== undefined && typeof n.kind === 'object' && 'contribution' in n.kind
const isConstant = (n: Node | undefined, label?: string) =>
  n !== undefined &&
  typeof n.kind === 'object' &&
  'constant' in n.kind &&
  (label === undefined || n.kind.constant.label === label)

/** Below any node of a single part (a rounding, a product of one term):
 * the node whose parts explain it. */
function explained(graph: Graph, index: number): number {
  let i = index
  for (let guard = 0; guard < 16; guard += 1) {
    const kids = graph[i]?.children ?? []
    if (kids.length !== 1 || isContribution(graph[kids[0]])) break
    i = kids[0]
  }
  return i
}

/** A step's parts and how they combine. `A × B ÷ 100` reads as "A × B%":
 * the last part is then a percentage. */
function partsOf(
  graph: Graph,
  index: number,
): { op: string | null; parts: number[]; percentLast: boolean } {
  const node = graph[index]
  const kids = node?.children ?? []
  const first = graph[kids[0] ?? -1]
  if (
    node?.kind === 'quotient' &&
    kids.length === 2 &&
    isConstant(graph[kids[1]], 'hundred') &&
    first?.kind === 'product' &&
    (first.children?.length ?? 0) >= 2
  ) {
    return { op: '×', parts: first.children ?? [], percentLast: true }
  }
  const op =
    typeof node?.kind === 'string' ? (OPERATOR[node.kind] ?? null) : null
  return { op, parts: kids, percentLast: false }
}

/** A step's name: the field it is the result of, its term in words, a
 * constant's name. */
function stepName(index: number, ctx: Context, top: boolean): string {
  const n = ctx.graph[index]
  if (n === undefined) return ''
  const field = ctx.fieldOf.get(index)
  if (field !== undefined && !top) return fieldLabel(field)
  if (typeof n.kind === 'object') {
    if ('constant' in n.kind) {
      const label = n.kind.constant.label
      return tOr(`constant.${label}`, humanize(label))
    }
    return quantityText(n.kind.contribution.quantity)
  }
  const term = (n.term ?? n.kind).replace(/^step\./, '')
  if (term === 'result' && field !== undefined) return fieldLabel(field)
  return tOr(`term.${term}`, tOr(`kind.${n.kind}`, humanize(term)))
}

/** Every contribution under a node, once each. */
function contributions(graph: Graph, index: number): number[] {
  const out: number[] = []
  const seen = new Set<number>()
  const walk = (i: number) => {
    if (seen.has(i)) return
    seen.add(i)
    const n = graph[i]
    if (n === undefined) return
    if (isContribution(n)) out.push(i)
    for (const c of n.children ?? []) walk(c)
  }
  walk(index)
  return out
}

function valueText(n: Node | undefined, percent: boolean): string {
  if (typeof n?.value !== 'number') return '—'
  return percent ? `${formatNumber(n.value)}%` : formatNumber(n.value)
}

function Step({
  index,
  ctx,
  op,
  percent,
  depth,
  selected,
  onSelect,
  counts,
}: {
  index: number
  ctx: Context
  op: string | null
  percent: boolean
  depth: number
  selected: number
  onSelect: (i: number) => void
  counts: (i: number) => number
}) {
  const shown = explained(ctx.graph, index)
  const { op: childOp, parts, percentLast } = partsOf(ctx.graph, shown)
  const steps = parts.filter((p) => !isContribution(ctx.graph[p]))
  const [open, setOpen] = useState(depth === 0)
  const sources = counts(index)
  const name = stepName(index, ctx, depth === 0)
  return (
    <li>
      <div className={`step-row${selected === index ? ' selected' : ''}`}>
        {steps.length > 0 ? (
          <button
            type="button"
            className="icon small step-toggle"
            aria-expanded={open}
            aria-label={
              open
                ? t('breakdown.collapse', { step: name })
                : t('breakdown.expand', { step: name })
            }
            onClick={() => setOpen(!open)}
          >
            {open ? '▾' : '▸'}
          </button>
        ) : (
          <span className="step-toggle" />
        )}
        <span className="step-op" aria-hidden="true">
          {op ?? ''}
        </span>
        {sources > 0 ? (
          <button
            type="button"
            className="link step-name"
            aria-pressed={selected === index}
            onClick={() => onSelect(index)}
          >
            {name}
          </button>
        ) : (
          <span className="step-name">{name}</span>
        )}
        <span className="num step-value">
          {valueText(ctx.graph[index], percent)}
        </span>
        <span className="muted small-text step-count">
          {sources > 0 &&
            (sources === 1
              ? t('breakdown.sourceCount1')
              : t('breakdown.sourceCount', { n: sources }))}
        </span>
      </div>
      {open && steps.length > 0 && (
        <ol className="steps">
          {steps.map((p, i) => (
            <Step
              key={p}
              index={p}
              ctx={ctx}
              op={i === 0 ? null : childOp}
              percent={percentLast && p === parts.at(-1)}
              depth={depth + 1}
              selected={selected}
              onSelect={onSelect}
              counts={counts}
            />
          ))}
        </ol>
      )}
    </li>
  )
}

interface Row {
  name: string
  detail: string | null
  /** Whether detail is an item line's text. */
  line: boolean
  effect: string
  /** How many steps it enters (once per damage type, say). */
  times: number
}

/** The sources under a step, grouped by what they change; identical
 * rows once, with how many steps they enter. */
function groupSources(ctx: Context, index: number): [string, Row[]][] {
  const groups = new Map<string, Map<string, Row>>()
  const order: Record<string, number> = {
    base: 0,
    increased: 1,
    more: 2,
    flag: 3,
  }
  const sorted = contributions(ctx.graph, index)
    .map((i) => ctx.graph[i])
    .filter((n): n is Node => n !== undefined)
    .sort((a, b) => {
      const ca = (a.kind as { contribution: Contribution }).contribution
      const cb = (b.kind as { contribution: Contribution }).contribution
      return (order[ca.op] ?? 9) - (order[cb.op] ?? 9)
    })
  for (const n of sorted) {
    const c = (n.kind as { contribution: Contribution }).contribution
    const quantity = quantityText(c.quantity)
    const { name, detail, line = false } = sourceInfo(c.source, ctx)
    const effect = effectText(c, n.value)
    const key = `${name}\u0000${detail ?? ''}\u0000${effect}`
    const rows = groups.get(quantity) ?? new Map<string, Row>()
    const row = rows.get(key)
    if (row === undefined) {
      rows.set(key, { name, detail, line, effect, times: 1 })
    } else {
      row.times += 1
    }
    groups.set(quantity, rows)
  }
  return [...groups].map(([q, rows]) => [q, [...rows.values()]])
}

function Sources({
  ctx,
  index,
  top,
  onReset,
}: {
  ctx: Context
  index: number
  top: number
  onReset: () => void
}) {
  const groups = groupSources(ctx, index)
  if (groups.length === 0) return null
  return (
    <section className="panel table-panel" aria-labelledby="bs-h">
      <div className="panel-head">
        <h2 id="bs-h">
          {index === top
            ? t('breakdown.sources')
            : t('breakdown.sourcesOf', { step: stepName(index, ctx, false) })}
        </h2>
        {index !== top && (
          <button type="button" className="link" onClick={onReset}>
            {t('breakdown.sourcesAll')}
          </button>
        )}
      </div>
      <table className="sources">
        <thead>
          <tr>
            <th scope="col">{t('breakdown.colSource')}</th>
            <th scope="col" className="right">
              {t('breakdown.colEffect')}
            </th>
          </tr>
        </thead>
        {groups.map(([quantity, rows]) => (
          <tbody key={quantity}>
            <tr className="group">
              <th scope="rowgroup" colSpan={2}>
                {quantity}
              </th>
            </tr>
            {rows.map((r, i) => (
              <tr key={i}>
                <th scope="row">
                  <span className="source-name">{r.name}</span>
                  {r.detail !== null && (
                    <span className={r.line ? 'sub source-detail' : 'sub'}>
                      {r.detail}
                    </span>
                  )}
                </th>
                <td className="right">
                  <span className="num">{r.effect}</span>
                  {r.times > 1 && (
                    <span className="sub">
                      {t('breakdown.inSteps', { n: r.times })}
                    </span>
                  )}
                </td>
              </tr>
            ))}
          </tbody>
        ))}
      </table>
    </section>
  )
}

/** The whole graph from the field down, every node, for checking. */
function Tree({
  index,
  ctx,
  seen,
}: {
  index: number
  ctx: Context
  seen: Set<number>
}) {
  const node = ctx.graph[index]
  if (node === undefined) return null
  const repeated = seen.has(index)
  seen.add(index)
  const children = repeated ? [] : (node.children ?? [])
  let text: string
  if (typeof node.kind === 'object' && 'contribution' in node.kind) {
    const c = node.kind.contribution
    const s = sourceInfo(c.source, ctx)
    text = `${quantityText(c.quantity)} ${effectText(c, node.value)} · ${s.name}`
  } else {
    text = `${stepName(index, ctx, false)} = ${valueText(node, false)}`
  }
  return (
    <li>
      {text}
      {children.length > 0 && (
        <ul>
          {children.map((c) => (
            <Tree key={c} index={c} ctx={ctx} seen={seen} />
          ))}
        </ul>
      )}
    </li>
  )
}

type Calc = [number, CalcResultDocumentV1] | null

/** A field's graph and node. */
function rootOf(calc: Calc, field: string | null) {
  const f = calc?.[1].result.fields.find((x) => x.field === field)
  return {
    graph: calc?.[1].result.graph,
    root: typeof f?.node === 'number' ? f.node : null,
  }
}

/** The IDs of the items whose lines feed the field. */
function itemsUnder(calc: Calc, field: string | null): number[] {
  const { graph, root } = rootOf(calc, field)
  if (graph === undefined || root === null) return []
  const ids = new Set<number>()
  for (const i of contributions(graph, root)) {
    const k = graph[i]?.kind
    if (typeof k === 'object' && 'contribution' in k) {
      const s = k.contribution.source
      if (typeof s === 'object' && s !== null && 'item' in s) {
        ids.add(Number((s as { item: { item: number } }).item.item))
      }
    }
  }
  return [...ids]
}

/** How many sources each node under the root has. */
function sourceCounts(calc: Calc, field: string | null): Map<number, number> {
  const { graph, root } = rootOf(calc, field)
  const out = new Map<number, number>()
  if (graph === undefined || root === null) return out
  const visit = (i: number) => {
    if (out.has(i)) return
    out.set(i, contributions(graph, i).length)
    for (const c of graph[i]?.children ?? []) visit(c)
  }
  visit(root)
  return out
}

export function Breakdown() {
  const dispatch = useAppDispatch()
  const field = useAppSelector((s) => s.ui.field)
  const calc = useAppSelector((s) => s.session.calc)
  const build = useAppSelector((s) => s.session.snapshot?.build)
  const [selected, setSelected] = useState<number | null>(null)
  const [lines, setLines] = useState<Record<number, ItemView>>({})
  const f = calc?.[1].result.fields.find((x) => x.field === field)
  const graph = calc?.[1].result.graph
  const fieldNode = typeof f?.node === 'number' ? f.node : null

  // The items whose lines the sources name, read once each.
  const itemIds = useMemo(() => itemsUnder(calc, field), [calc, field])

  useEffect(() => {
    let live = true
    for (const id of itemIds) {
      void dispatch(query((api) => api.item(id))).then((v) => {
        if (live && v !== null) setLines((l) => ({ ...l, [id]: v }))
      })
    }
    return () => {
      live = false
    }
  }, [dispatch, itemIds])

  const countMap = useMemo(() => sourceCounts(calc, field), [calc, field])

  const counts = (i: number) => countMap.get(i) ?? 0

  if (
    calc === null ||
    f === undefined ||
    field === null ||
    build == null ||
    graph === undefined
  ) {
    return null
  }
  const ctx: Context = {
    graph,
    build,
    fieldOf: new Map(
      calc[1].result.fields
        .filter((x) => typeof x.node === 'number')
        .map((x) => [x.node as number, x.field]),
    ),
    lines,
  }
  const reasons = [
    ...(f.unsupported ?? []),
    ...(f.indeterminate ?? []),
    ...(f.reason != null ? [f.reason] : []),
  ]
  const top = fieldNode
  const pick = selected ?? top
  return (
    <>
      <nav className="crumbs" aria-label={t('breakdown.crumbs')}>
        <button
          type="button"
          className="link"
          aria-label={t('breakdown.back')}
          onClick={() => dispatch(actions.go('dashboard'))}
        >
          {t('nav.dashboard')}
        </button>
        <span aria-hidden="true">›</span>
        <span>
          {OFFENCE.has(field) ? t('dashboard.offence') : t('dashboard.defence')}
        </span>
        <span aria-hidden="true">›</span>
        <span aria-current="page">{fieldLabel(field)}</span>
      </nav>
      <div className="page-head">
        <ScreenHeading>
          {typeof f.value === 'number'
            ? t('breakdown.title', {
                field: fieldLabel(field),
                value: fieldValue(f),
              })
            : t('breakdown.titleNoValue', { field: fieldLabel(field) })}
        </ScreenHeading>
        <FieldBadge field={f} />
      </div>

      <div className="columns">
        {top !== null && (
          <section className="panel" aria-labelledby="bf-h">
            <div className="panel-head">
              <h2 id="bf-h">{t('breakdown.factors')}</h2>
              <span className="muted small-text">
                {t('breakdown.factorsHint')}
              </span>
            </div>
            <ol className="steps root">
              <Step
                index={top}
                ctx={ctx}
                op={null}
                percent={false}
                depth={0}
                selected={pick ?? -1}
                onSelect={setSelected}
                counts={counts}
              />
            </ol>
          </section>
        )}
        {reasons.length > 0 && (
          <section className="panel aside" aria-labelledby="br-h">
            <div className="panel-head">
              <h2 id="br-h">{t('breakdown.reasons')}</h2>
            </div>
            <ul>
              {reasons.map((r, i) => (
                <li key={i}>{reasonText(r)}</li>
              ))}
            </ul>
            <button
              type="button"
              className="link"
              onClick={() => dispatch(actions.go('warnings'))}
            >
              {t('dashboard.viewWarnings')}
            </button>
          </section>
        )}
      </div>

      {top !== null && pick !== null && (
        <Sources
          ctx={ctx}
          index={pick}
          top={top}
          onReset={() => setSelected(null)}
        />
      )}

      {top !== null && (
        <section className="panel" aria-labelledby="bt-h">
          <div className="panel-head">
            <h2 id="bt-h">{t('breakdown.tree')}</h2>
          </div>
          <details>
            <summary>{t('breakdown.treeShow')}</summary>
            <ul className="tree">
              <Tree index={top} ctx={ctx} seen={new Set()} />
            </ul>
          </details>
        </section>
      )}
    </>
  )
}
