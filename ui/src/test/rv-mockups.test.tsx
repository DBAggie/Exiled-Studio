// The follow-up tasks built from the mockups approved on 2026-11-25
// (dev/active/ux-improvements/ui-revamp-follow-ups.md).
import { screen, waitFor, within } from '@testing-library/react'
import { describe, expect, it } from 'vitest'

import { dividerOf } from '../colourCodes'
import { expectAccessible, renderApp } from './render'

async function openDashboard() {
  const view = await renderApp({ data: true, demo: true })
  await view.user.click(
    screen.getByRole('button', {
      name: 'Import a Path of Building file (.xml)…',
    }),
  )
  await view.user.click(
    await screen.findByRole('button', { name: 'Open the dashboard' }),
  )
  await screen.findByRole('group', { name: 'Dashboards' })
  return view
}

describe('an author’s divider in the Skills table (RV-35)', () => {
  it('should read an empty group drawn with signs as a heading', () => {
    expect(dividerOf('^5==== Main Skills ============', 0)).toEqual({
      text: 'Main Skills',
      colour: expect.any(String) as string,
    })
    expect(dividerOf('---- Utility ----', 0)?.text).toBe('Utility')
    expect(dividerOf('== AG item mods', 0)?.text).toBe('AG item mods')
  })

  it('should leave every other group a row', () => {
    // It has gems, an ordinary label, no label, or signs alone.
    expect(dividerOf('==== Main Skills ====', 2)).toBeNull()
    expect(dividerOf('6L 5-Frostbearers', 0)).toBeNull()
    expect(dividerOf('Curse - on hit', 0)).toBeNull()
    expect(dividerOf(null, 0)).toBeNull()
    expect(dividerOf('==========', 0)).toBeNull()
  })
})

describe('Reset a dashboard to the default (RV-01)', () => {
  it('should be offered only for a changed dashboard, ask, and reset', async () => {
    const { user, api } = await openDashboard()
    await user.click(screen.getByRole('button', { name: 'Customize' }))
    // Nothing to reset yet.
    expect(
      screen.queryByRole('button', { name: 'Reset to default' }),
    ).toBeNull()
    const cards = () =>
      JSON.parse(api.prefs ?? '{}').dashboards?.[0]?.components
    await user.click(
      screen.getByRole('button', { name: /Hit DPS: card menu$/ }),
    )
    await user.click(await screen.findByRole('menuitem', { name: 'Remove' }))
    await waitFor(() => expect(cards()).toHaveLength(8))
    await user.click(screen.getByRole('button', { name: 'Reset to default' }))
    const dialog = await screen.findByRole('dialog', {
      name: 'Reset Overview to the default layout?',
    })
    await expectAccessible()
    await user.click(
      within(dialog).getByRole('button', { name: 'Reset Overview' }),
    )
    await waitFor(() => expect(cards()).toHaveLength(9))
    expect(
      screen.queryByRole('button', { name: 'Reset to default' }),
    ).toBeNull()
  })
})
