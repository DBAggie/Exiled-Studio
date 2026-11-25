// A field's reasons (what its value does not count yet) read for people:
// grouped by kind, each pointing at the item, passive, gem, or setting it
// comes from, with the calculation's codes kept apart for reports (the
// dashboard cards redesign, maintainer 2026-11-24).
import type { FieldResult, PassiveLines } from './api/types'

/** The groups, in the order they are shown. */
export const GROUPS = ['lines', 'stats', 'config', 'content', 'other'] as const
export type ReasonGroup = (typeof GROUPS)[number]

/** Where a reason comes from. */
export type ReasonTarget =
  | { kind: 'item'; item: number; line: number | null }
  | { kind: 'passive'; node: number }
  | { kind: 'gem'; group: number; gem: number }
  | { kind: 'config'; input: string }
  | { kind: 'custom'; block: number; line: number }
  | { kind: 'none' }

/** One reason, read. */
export interface ReasonEntry {
  group: ReasonGroup
  /** The calculation's code (`unmapped-stat`). */
  code: string
  /** The code's own detail (a stat ID, a `what`), for the small print. */
  detail: string | null
  target: ReasonTarget
}

const GROUP_OF: Record<string, ReasonGroup> = {
  'unresolved-line': 'lines',
  'pending-line': 'lines',
  'unread-line': 'lines',
  'excluded-line': 'lines',
  'scaled-line': 'lines',
  'unmapped-stat': 'stats',
  'unsettled-value': 'stats',
  'unhandled-kind': 'stats',
  'unmodelled-config': 'config',
  'unmodelled-condition': 'config',
  'unmodelled-choice': 'config',
  'conflicting-override': 'config',
  'unmodelled-gem': 'content',
  'unmodelled-skill': 'content',
  'unmodelled-item': 'content',
  'unmodelled-passive': 'content',
}

const num = (v: unknown): number | null => (typeof v === 'number' ? v : null)
const str = (v: unknown): string | null => (typeof v === 'string' ? v : null)
const obj = (v: unknown): Record<string, unknown> | null =>
  typeof v === 'object' && v !== null ? (v as Record<string, unknown>) : null

/** The target a reason's `source` names. */
function sourceTarget(source: unknown): ReasonTarget {
  const s = obj(source)
  if (s === null) return { kind: 'none' }
  const item = obj(s.item)
  if (item !== null && num(item.item) !== null) {
    return { kind: 'item', item: num(item.item) ?? 0, line: num(item.line) }
  }
  const base = obj(s['item-base'])
  if (base !== null && num(base.item) !== null) {
    return { kind: 'item', item: num(base.item) ?? 0, line: null }
  }
  const passive = obj(s.passive)
  if (passive !== null && num(passive.node) !== null) {
    return { kind: 'passive', node: num(passive.node) ?? 0 }
  }
  const gem = obj(s.gem)
  if (gem !== null && num(gem.group) !== null && num(gem.gem) !== null) {
    return { kind: 'gem', group: num(gem.group) ?? 0, gem: num(gem.gem) ?? 0 }
  }
  const config = obj(s.config) ?? obj(s['config-input-line'])
  if (config !== null && str(config.input) !== null) {
    return { kind: 'config', input: str(config.input) ?? '' }
  }
  const custom = obj(s['custom-modifier'])
  if (custom !== null && num(custom.block) !== null) {
    return {
      kind: 'custom',
      block: num(custom.block) ?? 0,
      line: num(custom.line) ?? 0,
    }
  }
  return { kind: 'none' }
}

/** One of the calculation's reasons, read. */
export function readReason(reason: unknown): ReasonEntry {
  if (typeof reason === 'string') {
    return {
      group: GROUP_OF[reason] ?? 'other',
      code: reason,
      detail: null,
      target: { kind: 'none' },
    }
  }
  const [code, raw] = Object.entries(obj(reason) ?? {})[0] ?? ['', null]
  const d = obj(raw) ?? {}
  let target = sourceTarget(d.source)
  if (target.kind === 'none' && num(d.item) !== null) {
    target = { kind: 'item', item: num(d.item) ?? 0, line: null }
  }
  if (target.kind === 'none' && str(d.input) !== null) {
    target = { kind: 'config', input: str(d.input) ?? '' }
  }
  return {
    group: GROUP_OF[code] ?? 'other',
    code,
    detail:
      str(d.stat) ??
      str(d.what) ??
      str(d.reason) ??
      str(d.scaling) ??
      str(d.exclusion) ??
      null,
    target,
  }
}

/** The line of a passive that prints the stat `stat`, with the passive's
 * name: an uncounted modifier of a passive is named by its own line, not
 * by the passive it is on (RV-11). `null` when the lines are not known
 * yet, or none prints the stat. */
export function passiveLine(
  passives: readonly PassiveLines[],
  node: number,
  stat: string | null,
): { text: string; passive: string } | null {
  if (stat === null) return null
  const passive = passives.find((p) => p.node === node)
  const line = passive?.lines.find((l) => l.stats.includes(stat))
  return passive === undefined || line === undefined
    ? null
    : { text: line.text, passive: passive.name }
}

/** The reasons a field carries, whatever its state. */
export function reasonsOf(f: FieldResult): unknown[] {
  const field = f as {
    reason?: unknown
    unsupported?: unknown[]
    indeterminate?: unknown[]
  }
  return [
    ...(field.reason != null ? [field.reason] : []),
    ...(field.indeterminate ?? []),
    ...(field.unsupported ?? []),
  ]
}

/** The field's reasons, read. */
export function entriesOf(f: FieldResult): ReasonEntry[] {
  return reasonsOf(f).map(readReason)
}

/** Each code with how many reasons carry it, most first: the small
 * print. */
export function codeCounts(entries: ReasonEntry[]): [string, number][] {
  const counts = new Map<string, number>()
  for (const e of entries) counts.set(e.code, (counts.get(e.code) ?? 0) + 1)
  return [...counts].sort((a, b) => b[1] - a[1] || a[0].localeCompare(b[0]))
}

/** The entries of each group that has any, in the groups' order. */
export function byGroup(
  entries: ReasonEntry[],
): { group: ReasonGroup; entries: ReasonEntry[] }[] {
  return GROUPS.map((group) => ({
    group,
    entries: entries.filter((e) => e.group === group),
  })).filter((g) => g.entries.length > 0)
}
