// The tree screen's cluster jewel helpers (passive tree plan, T5b): the
// spec's cluster nodes drawn over the layout, and which socket takes which
// cluster jewel (PoB's IsItemValidForSlot).
import { describe, expect, it } from 'vitest'

import type { TreeLayout, TreeNodeView, TreeView } from '../api/types'
import {
  clusterSize,
  fits,
  linksOf,
  nodeToward,
  pointsOf,
  withClusterNodes,
} from '../screens/treeLayout'

const node = (id: number, name: string): TreeNodeView => ({
  id,
  name,
  kind: 'normal',
  group: 0,
  orbit: 0,
  position: { x: id, y: 0 },
  modelled: true,
  'cluster-area': false,
  links: [],
  lines: [],
})

const layout: TreeLayout = {
  placed: true,
  'orbit-radii': [0],
  groups: [{ x: 0, y: 0 }],
  nodes: [node(1, 'One'), node(5001, 'Proxy socket')],
  classes: [],
  masteries: [],
}

const view = (added: TreeNodeView[]): TreeView => ({
  spec: 1,
  class: 1,
  ascendancy: 0,
  nodes: [],
  masteries: [],
  kept: [],
  points: 0,
  'ascendancy-points': 0,
  'cluster-nodes': added,
})

describe('cluster jewel nodes', () => {
  it('replace the layout node of their ID or are added, in ID order', () => {
    const merged = withClusterNodes(
      layout,
      view([node(65537, 'Generated'), node(5001, 'Medium socket')]),
    )
    expect(merged).not.toBe(layout)
    if (merged === null || merged === 'none') throw new Error('no layout')
    expect(merged.nodes.map((n) => [n.id, n.name])).toEqual([
      [1, 'One'],
      [5001, 'Medium socket'],
      [65537, 'Generated'],
    ])
  })

  it('leave the layout as it is without any', () => {
    expect(withClusterNodes(layout, view([]))).toBe(layout)
    expect(withClusterNodes(layout, null)).toBe(layout)
    expect(withClusterNodes('none', view([node(9, 'x')]))).toBe('none')
  })

  it('fit a socket of their size or larger', () => {
    expect(clusterSize('Large Cluster Jewel')).toBe('large')
    expect(clusterSize('Medium Test Cluster Jewel')).toBe('medium')
    expect(clusterSize('Small Cluster Jewel')).toBe('small')
    expect(clusterSize('Cobalt Jewel')).toBeNull()
    expect(fits('small', 'large')).toBe(true)
    expect(fits('medium', 'medium')).toBe(true)
    expect(fits('large', 'medium')).toBe(false)
  })
})

describe('the passive points nodes cost (RV-04)', () => {
  it('counts one each, but none for a start or an ascendancy node', () => {
    const l: TreeLayout = {
      ...layout,
      nodes: [
        node(1, 'One'),
        { ...node(2, 'WITCH'), kind: 'class-start' },
        { ...node(3, 'Mistress of Sacrifice'), ascendancy: 'Necromancer' },
        { ...node(4, 'Necromancer'), kind: 'ascendancy-start' },
      ],
    }
    expect(pointsOf([1, 2, 3, 4], l, view([]))).toBe(1)
    // A cluster jewel's node, and one the layout lacks, each cost a point.
    expect(pointsOf([1, 65537, 999], l, view([node(65537, 'Added')]))).toBe(3)
    expect(pointsOf([], l, view([]))).toBe(0)
  })
})

describe('the arrow keys on the tree (RV-20)', () => {
  const at = (id: number, x: number, y: number, links: number[] = []) => ({
    ...node(id, `n${id}`),
    position: { x, y },
    links,
  })
  const l: TreeLayout = {
    ...layout,
    nodes: [
      at(1, 0, 0, [2, 3, 5]),
      at(2, 100, 10),
      at(3, 0, -100, [4]),
      at(4, 100, -100),
      { ...at(5, -100, 0), kind: 'mastery' },
      { ...at(6, 0, 100, [1]), 'cluster-area': true },
      { ...at(7, 0, 200, [1]), ascendancy: 'Necromancer' },
    ],
  }
  const links = linksOf(l)

  it('links both ways, without masteries and empty cluster areas', () => {
    expect(links.get(1)).toEqual([2, 3])
    expect(links.get(2)).toEqual([1])
    expect(links.get(4)).toEqual([3])
    expect(links.get(5)).toBeUndefined()
    expect(links.get(6)).toBeUndefined()
    // The tree and an ascendancy are not linked for the keys.
    expect(links.get(7)).toBeUndefined()
  })

  it('move to the linked node most nearly that way, or stay', () => {
    expect(nodeToward(l, links, 1, 1, 0)).toBe(2)
    expect(nodeToward(l, links, 1, 0, -1)).toBe(3)
    // Nothing is linked below or to the left.
    expect(nodeToward(l, links, 1, 0, 1)).toBeNull()
    expect(nodeToward(l, links, 1, -1, 0)).toBeNull()
    // 4 is up and right of 1 but not linked to it.
    expect(nodeToward(l, links, 3, 1, 0)).toBe(4)
    expect(nodeToward(l, links, 99, 1, 0)).toBeNull()
  })
})
