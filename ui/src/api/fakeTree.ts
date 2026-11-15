// The fake API's passive tree: a small synthetic tree (two classes, an
// ascendancy pair, a mastery, a jewel socket, a node the editor keeps but
// does not edit) and the editor's rules as loom-tree has them, so UI
// tests run the tree screen without the application.
import type { TreeLayout, TreeNodeView, TreePreview, TreeView } from './types'

/** A spec as the fake keeps it. */
export interface FakeSpec {
  class: number
  ascendancy: number
  nodes: number[]
  masteries: { node: number; effect: number }[]
  /** The jewels in the spec's sockets. */
  jewels: { node: number; item: number }[]
}

const node = (
  id: number,
  name: string,
  kind: TreeNodeView['kind'],
  x: number,
  y: number,
  links: number[],
  lines: string[] = [],
  extra: Partial<TreeNodeView> = {},
): TreeNodeView => ({
  id,
  name,
  kind,
  group: 0,
  orbit: 0,
  position: { x, y },
  ascendancy: null,
  'mastery-group': null,
  modelled: true,
  links: links.filter((l) => l > id),
  lines,
  ...extra,
})

const NODES: TreeNodeView[] = [
  node(1, 'Witch', 'class-start', 0, 0, [10, 20, 50, 60]),
  node(2, 'Marauder', 'class-start', 0, 800, [30]),
  node(10, 'Intelligence', 'normal', 150, 0, [1, 11], ['+10 to Intelligence']),
  node(
    11,
    'Cold Damage',
    'normal',
    300,
    0,
    [10, 12],
    ['12% increased Cold Damage'],
  ),
  node(
    12,
    'Heart of Frost',
    'notable',
    450,
    0,
    [11, 13, 21, 40],
    ['30% increased Cold Damage', '10% increased Area of Effect'],
  ),
  node(
    13,
    'Glass Mind',
    'keystone',
    600,
    0,
    [12],
    ['Your maximum Energy Shield is doubled'],
    { flavour: 'A made-up keystone line.\nA second one.' },
  ),
  node(
    20,
    'Energy Shield',
    'normal',
    0,
    150,
    [1, 21],
    ['8% increased maximum Energy Shield'],
  ),
  node(21, 'Jewel Socket', 'jewel-socket', 150, 300, [12, 20, 30]),
  node(30, 'Life', 'normal', 0, 600, [2, 21], ['6% increased maximum Life']),
  node(40, 'Cold Mastery', 'mastery', 450, -120, [12], [], {
    'mastery-group': 'cold',
  }),
  node(50, 'Elementalist', 'ascendancy-start', -600, -600, [1, 51], [], {
    ascendancy: 'Elementalist',
  }),
  node(
    51,
    'Shaper of Winter',
    'notable',
    -480,
    -600,
    [50],
    ['Enemies Chilled by you take 10% increased Damage'],
    { ascendancy: 'Elementalist' },
  ),
  node(60, 'Occultist', 'ascendancy-start', 600, -600, [1, 61], [], {
    ascendancy: 'Occultist',
  }),
  node(
    61,
    'Frozen Heart',
    'notable',
    720,
    -600,
    [60],
    ['+20% to Cold Resistance'],
    {
      ascendancy: 'Occultist',
    },
  ),
  node(70, 'Bloodline Passive', 'normal', -700, 300, [], ['Kept as read'], {
    modelled: false,
    ascendancy: 'Bloodline',
  }),
]

export const FAKE_LAYOUT: TreeLayout = {
  'tree-version': '3_29',
  placed: true,
  'orbit-radii': [0, 82, 162, 335, 493, 662, 846],
  groups: [{ x: 0, y: 0 }],
  nodes: NODES,
  classes: [
    { number: 1, name: 'Marauder', start: 2, ascendancies: [] },
    {
      number: 3,
      name: 'Witch',
      start: 1,
      ascendancies: [
        {
          number: 1,
          id: 'Elementalist',
          name: 'Elementalist',
          start: 50,
          flavour: 'A made-up quote.',
          'flavour-colour': '200,180,120',
        },
        { number: 2, id: 'Occultist', name: 'Occultist', start: 60 },
      ],
    },
  ],
  masteries: [
    {
      id: 'cold',
      effects: [
        { effect: 4242, lines: ['20% increased Cold Damage'] },
        { effect: 4243, lines: ['Cold Exposure on Hit'] },
      ],
    },
  ],
}

export const freshSpec = (): FakeSpec => ({
  class: 3,
  ascendancy: 0,
  nodes: [1, 10, 70],
  masteries: [],
  jewels: [],
})

const byId = new Map(NODES.map((n) => [n.id, n]))
const neighbours = new Map<number, number[]>()
for (const n of NODES) {
  for (const m of n.links) {
    neighbours.set(n.id, [...(neighbours.get(n.id) ?? []), m])
    neighbours.set(m, [...(neighbours.get(m) ?? []), n.id])
  }
}
const near = (id: number) =>
  [...(neighbours.get(id) ?? [])].sort((a, b) => a - b)
const isStart = (id: number) =>
  ['class-start', 'ascendancy-start'].includes(byId.get(id)?.kind ?? '')

/** A refused edit's reason (the application's `APP_TREE`). */
export class TreeRefused extends Error {}

function starts(spec: FakeSpec): number[] {
  const c = FAKE_LAYOUT.classes.find((x) => x.number === spec.class)
  if (c === undefined) return []
  const a = c.ascendancies.find((x) => x.number === spec.ascendancy)
  return a?.start != null ? [c.start, a.start] : [c.start]
}

function ascendancyId(spec: FakeSpec): string | null {
  const c = FAKE_LAYOUT.classes.find((x) => x.number === spec.class)
  return c?.ascendancies.find((x) => x.number === spec.ascendancy)?.id ?? null
}

const kept = (id: number) => byId.get(id)?.modelled !== true

function connected(spec: FakeSpec): Set<number> {
  const seen = new Set<number>()
  const queue: number[] = []
  for (const s of starts(spec)) {
    if (spec.nodes.includes(s)) {
      seen.add(s)
      queue.push(s)
    }
  }
  for (const n of spec.nodes) {
    if (kept(n) && !seen.has(n)) {
      seen.add(n)
      queue.push(n)
    }
  }
  while (queue.length > 0) {
    const a = queue.shift() as number
    for (const b of near(a)) {
      const nb = byId.get(b)
      if (
        nb === undefined ||
        seen.has(b) ||
        !spec.nodes.includes(b) ||
        isStart(b) ||
        nb.kind === 'mastery' ||
        nb.ascendancy !== byId.get(a)?.ascendancy
      )
        continue
      seen.add(b)
      queue.push(b)
    }
  }
  for (const n of spec.nodes) {
    if (byId.get(n)?.kind === 'mastery' && near(n).some((m) => seen.has(m)))
      seen.add(n)
  }
  return seen
}

function prune(spec: FakeSpec): FakeSpec {
  const keep = connected(spec)
  return {
    ...spec,
    nodes: spec.nodes.filter((n) => keep.has(n)),
    masteries: spec.masteries.filter((m) => keep.has(m.node)),
  }
}

/** The shortest path to `target`, smallest node numbers first on ties. */
export function pathTo(spec: FakeSpec, target: number): number[] {
  const t = byId.get(target)
  if (t === undefined || !t.modelled || isStart(target) || t.kind === 'mastery')
    throw new TreeRefused('not allocatable')
  if (spec.nodes.includes(target)) throw new TreeRefused('allocated')
  const asc = ascendancyId(spec)
  const enterable = (id: number) => {
    const n = byId.get(id)
    return (
      n !== undefined &&
      n.modelled &&
      !isStart(id) &&
      n.kind !== 'mastery' &&
      !spec.nodes.includes(id) &&
      (n.ascendancy == null || n.ascendancy === asc)
    )
  }
  if (!enterable(target)) throw new TreeRefused('unreachable')
  const dist = new Map([[target, 0]])
  const queue = [target]
  while (queue.length > 0) {
    const a = queue.shift() as number
    for (const b of near(a)) {
      if (
        !dist.has(b) &&
        enterable(b) &&
        byId.get(b)?.ascendancy === byId.get(a)?.ascendancy
      ) {
        dist.set(b, (dist.get(a) as number) + 1)
        queue.push(b)
      }
    }
  }
  let best: [number, number] | null = null
  for (const a of connected(spec)) {
    for (const b of near(a)) {
      const d = dist.get(b)
      if (
        d === undefined ||
        byId.get(b)?.ascendancy !== byId.get(a)?.ascendancy
      )
        continue
      if (best === null || d < best[0] || (d === best[0] && b < best[1]))
        best = [d, b]
    }
  }
  if (best === null) throw new TreeRefused('unreachable')
  const out = [best[1]]
  let [d, at] = best
  while (d > 0) {
    const next = near(at).find((b) => dist.get(b) === d - 1)
    if (next === undefined) break
    out.push(next)
    at = next
    d -= 1
  }
  return out
}

const sorted = (xs: number[]) => [...new Set(xs)].sort((a, b) => a - b)

export function allocate(spec: FakeSpec, target: number): FakeSpec {
  return { ...spec, nodes: sorted([...spec.nodes, ...pathTo(spec, target)]) }
}

export function deallocate(spec: FakeSpec, target: number): FakeSpec {
  if (isStart(target) || kept(target) || !spec.nodes.includes(target))
    throw new TreeRefused('not deallocatable')
  return prune({
    ...spec,
    nodes: spec.nodes.filter((n) => n !== target),
    masteries: spec.masteries.filter((m) => m.node !== target),
  })
}

export function setMastery(
  spec: FakeSpec,
  target: number,
  effect: number,
): FakeSpec {
  const n = byId.get(target)
  const group = FAKE_LAYOUT.masteries.find((g) => g.id === n?.['mastery-group'])
  if (n?.kind !== 'mastery' || !group?.effects.some((e) => e.effect === effect))
    throw new TreeRefused('not a mastery effect')
  if (spec.masteries.some((m) => m.node !== target && m.effect === effect))
    throw new TreeRefused('taken')
  const next = {
    ...spec,
    nodes: sorted([...spec.nodes, target]),
    masteries: [
      ...spec.masteries.filter((m) => m.node !== target),
      { node: target, effect },
    ],
  }
  if (!connected(next).has(target)) throw new TreeRefused('unreachable')
  return next
}

export function selectClass(spec: FakeSpec, number: number): FakeSpec {
  if (!FAKE_LAYOUT.classes.some((c) => c.number === number))
    throw new TreeRefused('no class')
  const old = starts(spec)
  const next = {
    ...spec,
    class: number,
    ascendancy: 0,
    nodes: spec.nodes.filter((n) => !old.includes(n)),
  }
  return prune({ ...next, nodes: sorted([...next.nodes, ...starts(next)]) })
}

export function selectAscendancy(spec: FakeSpec, number: number): FakeSpec {
  const c = FAKE_LAYOUT.classes.find((x) => x.number === spec.class)
  if (number !== 0 && !c?.ascendancies.some((a) => a.number === number))
    throw new TreeRefused('no ascendancy')
  const old = starts(spec).slice(1)
  const next = {
    ...spec,
    ascendancy: number,
    nodes: spec.nodes.filter((n) => !old.includes(n)),
  }
  return prune({ ...next, nodes: sorted([...next.nodes, ...starts(next)]) })
}

/** Sockets jewel `item` (`null` empties it) in allocated socket `target`;
 * the caller checks that the item is a jewel of the build. */
export function socketJewel(
  spec: FakeSpec,
  target: number,
  item: number | null,
): FakeSpec {
  if (byId.get(target)?.kind !== 'jewel-socket')
    throw new TreeRefused('not a jewel socket')
  if (!spec.nodes.includes(target)) throw new TreeRefused('not allocated')
  const others = spec.jewels.filter((j) => j.node !== target)
  return {
    ...spec,
    jewels: item === null ? others : [...others, { node: target, item }],
  }
}

export function reset(spec: FakeSpec): FakeSpec {
  return { ...spec, nodes: starts(spec), masteries: [] }
}

export function view(spec: FakeSpec, id: number): TreeView {
  const counted = spec.nodes.filter((n) => !isStart(n))
  return {
    spec: id,
    class: spec.class,
    ascendancy: spec.ascendancy,
    nodes: sorted(spec.nodes),
    masteries: spec.masteries,
    jewels: [...spec.jewels].sort((a, b) => a.node - b.node),
    kept: spec.nodes.filter(kept),
    points: counted.filter((n) => byId.get(n)?.ascendancy == null || kept(n))
      .length,
    'ascendancy-points': counted.filter(
      (n) => byId.get(n)?.ascendancy != null && !kept(n),
    ).length,
  }
}

export function preview(
  spec: FakeSpec,
  target: number,
  life: number,
): TreePreview {
  const allocated = spec.nodes.includes(target)
  const after = allocated ? deallocate(spec, target) : allocate(spec, target)
  const path = allocated ? [] : pathTo(spec, target)
  const removed = spec.nodes.filter((n) => !after.nodes.includes(n))
  const gain =
    (after.nodes.includes(30) ? 6 : 0) - (spec.nodes.includes(30) ? 6 : 0)
  return {
    node: target,
    path,
    removed,
    fields: [
      {
        field: 'life',
        before: life,
        after: life + gain,
        'state-before': 'complete',
        'state-after': 'complete',
      },
    ],
  }
}
