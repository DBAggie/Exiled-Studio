// The UI revamp's follow-up tasks
// (dev/active/ux-improvements/ui-revamp-follow-ups.md).
import { screen, waitFor, within } from '@testing-library/react'
import { describe, expect, it } from 'vitest'

import { expectAccessible, renderApp } from './render'

const now = Math.floor(Date.now() / 1000)
const copy = (id: string, name: string) => ({
  id,
  'saved-at': now - 120,
  name,
  path: null,
  origin: null,
})

describe('Discard all for unsaved work (RV-05)', () => {
  it('should ask, then remove every copy', async () => {
    const { user, api } = await renderApp({
      recovery: [copy('r1', 'one'), copy('r2', 'two'), copy('r3', 'three')],
    })
    await user.click(screen.getByRole('button', { name: 'Discard all' }))
    const dialog = await screen.findByRole('dialog', {
      name: 'Discard all unsaved work?',
    })
    expect(within(dialog).getByText(/The 3 copies listed here/)).toBeTruthy()
    await expectAccessible()
    // Cancel keeps them.
    await user.click(within(dialog).getByRole('button', { name: 'Cancel' }))
    expect(await api.recovery()).toHaveLength(3)
    await user.click(screen.getByRole('button', { name: 'Discard all' }))
    await user.click(
      await screen.findByRole('button', { name: 'Discard all 3' }),
    )
    await waitFor(() =>
      expect(
        screen.queryByRole('heading', { name: 'Unsaved work' }),
      ).toBeNull(),
    )
    expect(await api.recovery()).toHaveLength(0)
  })

  it('should not be offered for a single copy', async () => {
    await renderApp({ recovery: [copy('r1', 'one')] })
    expect(screen.getByRole('button', { name: 'Discard one' })).toBeTruthy()
    expect(screen.queryByRole('button', { name: 'Discard all' })).toBeNull()
  })
})

describe('a card two rows tall (RV-03)', () => {
  it('should be the default Defence table, with two cards beside it', async () => {
    const { user } = await renderApp()
    await user.click(screen.getByRole('button', { name: 'Open a file…' }))
    await screen.findByRole('heading', { name: 'Dashboard', level: 1 })
    const tall = document.querySelectorAll('.dash-cell.dash-tall')
    expect(tall).toHaveLength(1)
    const cells = [...document.querySelectorAll('.dash-grid > .dash-cell')]
    const i = cells.indexOf(tall[0]!)
    expect(tall[0]!.className).toContain('dash-two-thirds')
    expect(cells[i + 1]!.className).toContain('dash-third')
    expect(cells[i + 2]!.className).toContain('dash-third')
  })
})
