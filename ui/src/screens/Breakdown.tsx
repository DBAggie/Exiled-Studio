// Screen 4 (docs/ui, Breakdown): why a field has its value. The factors
// the field's node combines, every contribution under it with its source,
// what is not counted, and the whole provenance graph from the field down.
import type { CalcResultDocumentV1, ItemSummary, Node } from '../api/types'
import { FieldBadge, ScreenHeading } from '../components'
import { OFFENCE, fieldLabel, formatNumber, t, type MessageKey } from '../i18n'
import { actions } from '../store'
import {
  fieldValue,
  itemName,
  reasonText,
  useAppDispatch,
  useAppSelector,
} from '../util'

type Graph = CalcResultDocumentV1['result']['graph']

function nodeValue(n: Node): string {
  return typeof n.value === 'number' ? formatNumber(n.value) : '—'
}

function sourceText(source: unknown, items: ItemSummary[]): string {
  if (source === 'character') return t('breakdown.source.character')
  if (source === 'enemy') return t('breakdown.source.enemy')
  if (typeof source !== 'object' || source === null) return ''
  const [kind, d] = Object.entries(source)[0] ?? ['', {}]
  const p = (d ?? {}) as Record<string, string | number>
  const keys: Record<string, MessageKey> = {
    item: 'breakdown.source.item',
    'item-base': 'breakdown.source.itemBase',
    passive: 'breakdown.source.passive',
    gem: 'breakdown.source.gem',
    config: 'breakdown.source.config',
    rule: 'breakdown.source.rule',
    'custom-modifier': 'breakdown.source.custom',
    'config-input-line': 'breakdown.source.custom',
  }
  const key = keys[kind]
  if (key === undefined) return kind
  if (kind === 'item' || kind === 'item-base') {
    // The item by name; an item line's index is shown from 1.
    const item = items.find((i) => i.id === Number(p.item))
    const name = item === undefined ? `#${p.item}` : itemName(item)
    return t(key, { ...p, item: name, line: Number(p.line) + 1 })
  }
  return t(key, p)
}

/** A term or an operation as a label ("base", "more", ...). */
function termLabel(n: Node): string {
  const raw = n.term ?? (typeof n.kind === 'string' ? n.kind : '')
  const text = raw.replace(/^step\./, '').replace(/[-_.]/g, ' ')
  return text.charAt(0).toUpperCase() + text.slice(1)
}

function label(n: Node, items: ItemSummary[]): string {
  const kind = n.kind
  if (typeof kind === 'string') {
    return n.term ? `${termLabel(n)} (${kind})` : kind
  }
  if ('constant' in kind) {
    return kind.constant.label
  }
  const c = kind.contribution
  return `${c.op} ${c.quantity} · ${sourceText(c.source, items)}`
}

const OPERATOR: Record<string, string> = {
  sum: '+',
  product: '×',
  'percent-factor': '×',
  difference: '−',
  quotient: '÷',
}

/** The node whose parts explain the field: below any node of a single
 * part (a rounding, a product of one term). */
function explained(graph: Graph, index: number): Node | undefined {
  let node = graph[index]
  for (let guard = 0; node !== undefined && guard < 16; guard += 1) {
    const kids = node.children ?? []
    if (kids.length !== 1) break
    node = graph[kids[0]]
  }
  return node
}

/** A factor's label: a constant's own, a contribution's source, else its
 * term. */
function factorLabel(n: Node, items: ItemSummary[]): string {
  if (typeof n.kind === 'object') {
    if ('constant' in n.kind) return n.kind.constant.label
    return sourceText(n.kind.contribution.source, items)
  }
  return termLabel(n)
}

function Factors({
  graph,
  node,
  items,
}: {
  graph: Graph
  node: Node
  items: ItemSummary[]
}) {
  const kids = node.children ?? []
  const op = typeof node.kind === 'string' ? OPERATOR[node.kind] : undefined
  if (kids.length < 2 || op === undefined) return null
  return (
    <section className="panel" aria-labelledby="bf-h">
      <div className="panel-head">
        <h2 id="bf-h">{t('breakdown.factors')}</h2>
      </div>
      <ol className="factors">
        {kids.map((k, i) => {
          const n = graph[k]
          if (n === undefined) return null
          return (
            <li key={k} className="row">
              {i > 0 && (
                <span className="factor-op" aria-hidden="true">
                  {op}
                </span>
              )}
              <span className="factor">
                <span className="label">{factorLabel(n, items)}</span>
                <span className="value">{nodeValue(n)}</span>
              </span>
            </li>
          )
        })}
      </ol>
    </section>
  )
}

/** Every contribution under a node, once each, in graph order. */
function contributions(graph: Graph, index: number): Node[] {
  const out: Node[] = []
  const seen = new Set<number>()
  const walk = (i: number) => {
    if (seen.has(i)) return
    seen.add(i)
    const n = graph[i]
    if (n === undefined) return
    if (typeof n.kind === 'object' && 'contribution' in n.kind) out.push(n)
    for (const c of n.children ?? []) walk(c)
  }
  walk(index)
  return out
}

function Tree({
  graph,
  index,
  seen,
  items,
}: {
  graph: Graph
  index: number
  seen: Set<number>
  items: ItemSummary[]
}) {
  const node = graph[index]
  if (node === undefined) return null
  const repeated = seen.has(index)
  seen.add(index)
  const children = repeated ? [] : (node.children ?? [])
  return (
    <li>
      <span className="num">{nodeValue(node)}</span> — {label(node, items)}
      {children.length > 0 && (
        <ul>
          {children.map((c) => (
            <Tree key={c} graph={graph} index={c} seen={seen} items={items} />
          ))}
        </ul>
      )}
    </li>
  )
}

export function Breakdown() {
  const dispatch = useAppDispatch()
  const field = useAppSelector((s) => s.ui.field)
  const calc = useAppSelector((s) => s.session.calc)
  const items = useAppSelector((s) => s.session.snapshot?.build?.items ?? [])
  const f = calc?.[1].result.fields.find((x) => x.field === field)
  if (calc === null || f === undefined || field === null) {
    return null
  }
  const graph = calc[1].result.graph
  const reasons = [
    ...(f.unsupported ?? []),
    ...(f.indeterminate ?? []),
    ...(f.reason != null ? [f.reason] : []),
  ]
  const top = typeof f.node === 'number' ? explained(graph, f.node) : undefined
  const sources = typeof f.node === 'number' ? contributions(graph, f.node) : []
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

      {top !== undefined && <Factors graph={graph} node={top} items={items} />}

      <div className="columns">
        {sources.length > 0 && (
          <section className="panel table-panel" aria-labelledby="bs-h">
            <div className="panel-head">
              <h2 id="bs-h">{t('breakdown.sources')}</h2>
            </div>
            <table>
              <thead>
                <tr>
                  <th scope="col">{t('breakdown.colSource')}</th>
                  <th scope="col">{t('breakdown.colEffect')}</th>
                  <th scope="col" className="right">
                    {t('breakdown.colValue')}
                  </th>
                </tr>
              </thead>
              <tbody>
                {sources.map((n, i) => {
                  if (typeof n.kind !== 'object' || !('contribution' in n.kind))
                    return null
                  const c = n.kind.contribution
                  return (
                    <tr key={i}>
                      <th scope="row">{sourceText(c.source, items)}</th>
                      <td>
                        <span className="stat-id">
                          {c.op} {c.quantity}
                        </span>
                      </td>
                      <td className="right num">{nodeValue(n)}</td>
                    </tr>
                  )
                })}
              </tbody>
            </table>
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

      {typeof f.node === 'number' && (
        <section className="panel" aria-labelledby="bt-h">
          <div className="panel-head">
            <h2 id="bt-h">{t('breakdown.tree')}</h2>
          </div>
          <details>
            <summary>{t('breakdown.treeShow')}</summary>
            <ul className="tree">
              <Tree
                graph={graph}
                index={f.node}
                seen={new Set()}
                items={items}
              />
            </ul>
          </details>
        </section>
      )}
    </>
  )
}
