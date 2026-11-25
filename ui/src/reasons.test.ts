import { describe, expect, it } from 'vitest'

import type { FieldResult } from './api/types'
import { byGroup, codeCounts, entriesOf, readReason } from './reasons'

describe('reasons', () => {
  it('should read where a reason comes from', () => {
    expect(
      readReason({
        'unmapped-stat': {
          stat: 'life_leech_from_minion_damage_%',
          source: { passive: { node: 14603 } },
        },
      }),
    ).toEqual({
      group: 'stats',
      code: 'unmapped-stat',
      detail: 'life_leech_from_minion_damage_%',
      target: { kind: 'passive', node: 14603 },
    })
    expect(
      readReason({
        'unresolved-line': {
          source: { item: { item: 3, line: 24 } },
          reason: 'no-template',
        },
      }).target,
    ).toEqual({ kind: 'item', item: 3, line: 24 })
    expect(
      readReason({
        'unresolved-line': {
          source: { 'custom-modifier': { block: 0, line: 2 } },
          reason: 'no-template',
        },
      }).target,
    ).toEqual({ kind: 'custom', block: 0, line: 2 })
    expect(
      readReason({ 'unmodelled-config': { input: 'TotalSpectreLife' } }),
    ).toMatchObject({
      group: 'config',
      target: { kind: 'config', input: 'TotalSpectreLife' },
    })
    expect(
      readReason({
        'unmodelled-item': { item: 11, slot: 'Flask 4', what: 'active-flask' },
      }),
    ).toMatchObject({
      group: 'content',
      detail: 'active-flask',
      target: { kind: 'item', item: 11, line: null },
    })
    expect(
      readReason({
        'unmodelled-skill': {
          source: { gem: { group: 6, gem: 0 } },
          what: 'socket-colour-quality',
        },
      }).target,
    ).toEqual({ kind: 'gem', group: 6, gem: 0 })
  })

  it('should keep a reason it does not know as "other", with its code', () => {
    expect(readReason('not-computed')).toEqual({
      group: 'other',
      code: 'not-computed',
      detail: null,
      target: { kind: 'none' },
    })
    expect(readReason({ 'new-kind': { x: 1 } })).toMatchObject({
      group: 'other',
      code: 'new-kind',
    })
  })

  it('should group in the shown order and count the codes', () => {
    const field = {
      field: 'life',
      state: 'partial',
      value: 1,
      node: null,
      unsupported: [
        { 'unmodelled-config': { input: 'a' } },
        { 'scaled-line': { source: { item: { item: 1, line: 2 } } } },
        { 'unmodelled-config': { input: 'b' } },
        'unchecked-tree-version',
      ],
    } as unknown as FieldResult
    const entries = entriesOf(field)
    expect(byGroup(entries).map((g) => [g.group, g.entries.length])).toEqual([
      ['lines', 1],
      ['config', 2],
      ['other', 1],
    ])
    expect(codeCounts(entries)).toEqual([
      ['unmodelled-config', 2],
      ['scaled-line', 1],
      ['unchecked-tree-version', 1],
    ])
  })
})
