// Keyboard use (ADR-008; Phase 4 plan, decision 13): everything reachable
// by Tab, activated by Enter or Space, focus moved to each screen's
// heading, the menu driven by arrow keys and closed by Escape.
import { screen, waitFor } from '@testing-library/react'
import { describe, expect, it } from 'vitest'

import { renderApp } from './render'

describe('keyboard', () => {
  it('imports, navigates, and uses the export menu without a mouse', async () => {
    const { user } = await renderApp()
    // The start screen's heading has focus; Tab reaches the import field.
    expect(document.activeElement?.textContent).toBe('Exiled Studio')
    await user.tab()
    expect(document.activeElement).toBe(
      screen.getByLabelText('Share code or XML'),
    )
    await user.keyboard('<PathOfBuilding/>')
    await user.tab()
    expect(document.activeElement).toBe(
      screen.getByRole('button', { name: 'Import' }),
    )
    await user.keyboard('{Enter}')
    const review = await screen.findByRole('heading', { name: 'Import result' })
    expect(document.activeElement).toBe(review)

    // Tab to the continue button and press Space.
    const go = screen.getByRole('button', { name: 'Open the dashboard' })
    while (document.activeElement !== go) {
      await user.tab()
    }
    await user.keyboard(' ')
    const dashboard = await screen.findByRole('heading', {
      name: 'Dashboard',
      level: 1,
    })
    expect(document.activeElement).toBe(dashboard)

    // The side menu marks the current screen.
    const nav = screen.getByRole('navigation', { name: 'Workspace' })
    const current = nav.querySelector('[aria-current="page"]')
    expect(current?.textContent).toBe('Dashboard')

    // The export menu: Enter opens it, arrows move, Escape closes and
    // returns focus to the trigger.
    const trigger = screen.getByRole('button', { name: /Export/ })
    trigger.focus()
    await user.keyboard('{Enter}')
    const menu = await screen.findByRole('menu')
    await waitFor(() =>
      expect(menu.contains(document.activeElement)).toBe(true),
    )
    await user.keyboard('{ArrowDown}')
    expect(document.activeElement?.textContent).toBe(
      'Path of Building file (.xml)…',
    )
    const poe2 = screen.getByRole('menuitem', { name: /Path of Exile 2/ })
    expect(poe2.getAttribute('aria-disabled')).toBe('true')
    await user.keyboard('{Escape}')
    await waitFor(() => expect(screen.queryByRole('menu')).toBeNull())
    expect(document.activeElement).toBe(trigger)
  })

  it('announces the calculation politely', async () => {
    const { user } = await renderApp()
    const live = screen.getByRole('status')
    expect(live.getAttribute('aria-live')).toBe('polite')
    await user.click(screen.getByRole('button', { name: 'Open a .xml file…' }))
    await waitFor(() => expect(live.textContent).toBe('Calculation finished.'))
  })
})
