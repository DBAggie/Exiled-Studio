import { describe, expect, it } from 'vitest'

import { fieldLabel, readable } from './index'

describe('field labels', () => {
  it('should name a field the catalogue lacks in words, not its ID', () => {
    expect(readable('poe1.max-life-leech-rate')).toBe('Max life leech rate')
    expect(readable('poe1.cold-taken-hit-mult')).toBe(
      'Cold taken hit multiplier',
    )
    expect(fieldLabel('poe1.no-such-field-dps')).toBe('No such field DPS')
  })

  it('should keep the catalogue name when there is one', () => {
    expect(fieldLabel('poe1.total-ehp')).toBe('Effective health pool')
    expect(fieldLabel('life')).toBe('Life')
  })
})
