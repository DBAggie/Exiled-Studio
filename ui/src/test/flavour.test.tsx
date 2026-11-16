// B-2: a unique's flavour text on the game-style tooltip, after the flags
// (as PoB's tooltip has it), a default-colour note in the label colour,
// and a gap where the game leaves one.
import { render, screen } from '@testing-library/react'
import { describe, expect, it } from 'vitest'

import type { ItemView } from '../generated/app-api.v1'
import { TooltipBody } from '../screens/Inventory'

function unique(flavour: ItemView['card']['flavour']): ItemView {
  return {
    id: 1,
    name: 'Test Flavoured',
    base: 'Plain Test Ring',
    rarity: 'unique',
    slot: null,
    lines: [],
    card: {
      'item-class': null,
      quality: null,
      'item-level': null,
      'level-req': null,
      sockets: [],
      properties: [],
      influences: [],
      flags: ['Corrupted'],
      variants: [],
      'selected-variant': null,
      'variant-eras': [],
      'current-variant': null,
      affixes: [],
      requirements: null,
      flavour,
    },
  }
}

describe('flavour text (B-2)', () => {
  it('shows a unique’s flavour text after the flags, with its note and gap', () => {
    const { container } = render(
      <TooltipBody
        item={unique([
          { text: 'A made-up line.', 'default-colour': false },
          { text: '', 'default-colour': false },
          { text: 'A made-up note', 'default-colour': true },
        ])}
        details={false}
      />,
    )
    const line = screen.getByText('A made-up line.')
    expect(line.className).toBe('tt-flavour')
    expect(screen.getByText('A made-up note').className).toBe('tt-flag')
    expect(container.querySelectorAll('.tt-gap')).toHaveLength(1)
    const blocks = [...container.querySelectorAll('.tt-block')]
    const flags = blocks.findIndex((b) => b.textContent === 'Corrupted')
    expect(blocks.indexOf(line.parentElement!)).toBe(flags + 1)
  })

  it('shows no flavour block without flavour text', () => {
    const { container } = render(
      <TooltipBody item={unique(undefined)} details={false} />,
    )
    expect(container.querySelector('.tt-flavour')).toBeNull()
    expect(container.querySelectorAll('.tt-block')).toHaveLength(1)
  })
})
