// Screen 4 (docs/ui): why a field has its value: the provenance graph from
// the field's node down, each contribution with its source, and what is
// not counted.
import type { CalcResultDocumentV1, Node } from '../api/types'
import { ScreenHeading, StateBadge } from '../components'
import { fieldLabel, formatNumber, t, type MessageKey } from '../i18n'
import { actions } from '../store'
import { fieldValue, reasonText, useAppDispatch, useAppSelector } from '../util'

type Graph = CalcResultDocumentV1['result']['graph']

function nodeValue(n: Node): string {
  return typeof n.value === 'number' ? formatNumber(n.value) : '—'
}

function sourceText(source: unknown): string {
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
  // An item line's index is shown from 1.
  const params = kind === 'item' ? { ...p, line: Number(p.line) + 1 } : p
  return t(key, params)
}

function label(n: Node): string {
  const kind = n.kind
  if (typeof kind === 'string') {
    return n.term ? `${n.term} (${kind})` : kind
  }
  if ('constant' in kind) {
    return kind.constant.label
  }
  const c = kind.contribution
  return `${c.op} ${c.quantity} · ${sourceText(c.source)}`
}

function Tree({
  graph,
  index,
  seen,
}: {
  graph: Graph
  index: number
  seen: Set<number>
}) {
  const node = graph[index]
  if (node === undefined) return null
  const repeated = seen.has(index)
  seen.add(index)
  const children = repeated ? [] : (node.children ?? [])
  return (
    <li>
      <span className="mono">{nodeValue(node)}</span> — {label(node)}
      {children.length > 0 && (
        <ul>
          {children.map((c) => (
            <Tree key={c} graph={graph} index={c} seen={seen} />
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
  const f = calc?.[1].result.fields.find((x) => x.field === field)
  if (calc === null || f === undefined || field === null) {
    return null
  }
  const reasons = [
    ...(f.unsupported ?? []),
    ...(f.indeterminate ?? []),
    ...(f.reason != null ? [f.reason] : []),
  ]
  return (
    <section>
      <ScreenHeading>
        {typeof f.value === 'number'
          ? t('breakdown.title', {
              field: fieldLabel(field),
              value: fieldValue(f),
            })
          : t('breakdown.titleNoValue', { field: fieldLabel(field) })}
      </ScreenHeading>
      <p>
        <StateBadge state={f.state} />
      </p>
      {typeof f.node === 'number' && (
        <section className="panel" aria-labelledby="bt-h">
          <h2 id="bt-h">{t('breakdown.tree')}</h2>
          <ul className="tree">
            <Tree
              graph={calc[1].result.graph}
              index={f.node}
              seen={new Set()}
            />
          </ul>
        </section>
      )}
      {reasons.length > 0 && (
        <section className="panel" aria-labelledby="br-h">
          <h2 id="br-h">{t('breakdown.reasons')}</h2>
          <ul>
            {reasons.map((r, i) => (
              <li key={i}>{reasonText(r)}</li>
            ))}
          </ul>
        </section>
      )}
      <button type="button" onClick={() => dispatch(actions.go('dashboard'))}>
        {t('breakdown.back')}
      </button>
    </section>
  )
}
