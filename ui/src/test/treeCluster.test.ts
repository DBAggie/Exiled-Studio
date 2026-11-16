// The tree screen's cluster jewel helpers (passive tree plan, T5b): the
// spec's cluster nodes drawn over the layout, and which socket takes which
// cluster jewel (PoB's IsItemValidForSlot).
import { describe, expect, it } from 'vitest'

import type { TreeLayout, TreeNodeView, TreeView } from '../api/types'
import { clusterSize, fits, withClusterNodes } from '../screens/treeLayout'

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
