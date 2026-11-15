// B-2 item evidence (E5): an item card's values from the calculation, as
// the game's tooltip prints them: a weapon's damage per type (elements in
// their colours), crit to two places, attacks per second; a shield's
// block and defences, which take the place of the written defence lines;
// a note when the values may leave lines out.
import { render, screen } from '@testing-library/react'
import { describe, expect, it } from 'vitest'

import { valueLines } from '../cardValues'
import type { ItemView } from '../generated/app-api.v1'
import { TooltipBody } from '../screens/Inventory'

function item(card: Partial<ItemView['card']>): ItemView {
  return {
    id: 1,
    name: 'Test Weapon',
    base: 'Test Base',
    rarity: 'rare',
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
      flags: [],
      variants: [],
      'selected-variant': null,
      'variant-eras': [],
      'current-variant': null,
      affixes: [],
      requirements: null,
      ...card,
    },
  }
}

describe('card values (B-2 item evidence)', () => {
  it('prints a weapon’s values in the game’s order and form', () => {
    const weapon = item({
      values: {
        defences: [],
        damage: [
          { 'damage-type': 'physical', min: 159, max: 246 },
          { 'damage-type': 'lightning', min: 3, max: 60 },
          { 'damage-type': 'fire', min: 10, max: 20 },
        ],
        'attack-rate': 1000 / 769,
        'crit-chance': 7.6,
        partial: false,
      },
    })
    expect(valueLines(weapon.card).map((v) => `${v.name}: ${v.value}`)).toEqual(
      [
        'Physical Damage: 159-246',
        'Elemental Damage: 10-20, 3-60',
        'Critical Strike Chance: 7.60%',
        'Attacks per Second: 1.30',
      ],
    )
    render(<TooltipBody item={weapon} details={false} />)
    expect(screen.getByText('10-20').className).toBe('tt-fire')
    expect(screen.getByText('3-60').className).toBe('tt-lightning')
    expect(screen.queryByText(/may leave them out/)).toBeNull()
  })

  it('puts a shield’s computed defences in place of the written ones', () => {
    const shield = item({
      properties: [
        { name: 'Armour', value: '1198' },
        { name: 'Radius', value: 'Small' },
      ],
      values: {
        defences: [{ defence: 'armour', value: 1198 }],
        damage: [],
        block: 35,
        partial: true,
      },
    })
    const { container } = render(<TooltipBody item={shield} details={false} />)
    const text = container.textContent ?? ''
    expect(text.match(/Armour:/g)).toHaveLength(1)
    expect(text).toContain('Chance to Block: 35%')
    expect(text).toContain('Radius: Small')
    expect(screen.getByText(/may leave them out/).className).toBe('tt-note')
  })
})

describe('socket caps (B-2 item evidence)', () => {
  it('notes sockets past the base’s cap on the tooltip', () => {
    const six = item({ sockets: ['RGB', 'RGB'], 'max-sockets': 4 })
    render(<TooltipBody item={six} details={false} />)
    expect(
      screen.getByText(/More sockets than the base rolls/).textContent,
    ).toContain('at most 4')
  })

  it('says nothing within the cap or without one', () => {
    for (const card of [
      { sockets: ['RGB'], 'max-sockets': 6 },
      { sockets: ['RGBRGB'], 'max-sockets': null },
    ]) {
      const { unmount } = render(
        <TooltipBody item={item(card)} details={false} />,
      )
      expect(screen.queryByText(/More sockets than/)).toBeNull()
      unmount()
    }
  })
})
