// axe-core on every screen of the slice (Phase 4 plan, decision 13).
import { screen, waitFor } from '@testing-library/react'
import { describe, it } from 'vitest'

import { expectAccessible, renderApp } from './render'

async function imported() {
  const r = await renderApp({
    recent: [{ id: 'a1', path: '/builds/witch.json', name: 'witch', at: 1 }],
    recovery: [
      { id: 'r1', 'saved-at': 1, name: 'lost', path: null, origin: null },
    ],
  })
  await expectAccessible()
  await r.user.click(screen.getByRole('button', { name: 'Open a .xml file…' }))
  await screen.findByRole('heading', { name: 'Imported build', level: 1 })
  return r
}

describe('accessibility', () => {
  it('has no violations on any screen', async () => {
    const { user } = await imported()
    await expectAccessible()

    await user.click(screen.getByRole('button', { name: 'Open the dashboard' }))
    await waitFor(() => screen.getByRole('button', { name: /^Life:/ }))
    await expectAccessible()

    await user.click(screen.getByRole('button', { name: /^Life:/ }))
    await screen.findByRole('heading', { name: /Why is Life/ })
    await expectAccessible()

    await user.click(screen.getByRole('button', { name: 'Equipment' }))
    await screen.findByRole('heading', { name: 'Equipment', level: 1 })
    await expectAccessible()

    await user.click(
      screen.getByRole('button', { name: 'Edit Loom Test Band' }),
    )
    await screen.findByLabelText('Line 4 text')
    await user.click(screen.getByRole('button', { name: 'Remove line 5' }))
    await user.click(screen.getByRole('button', { name: 'Preview effect' }))
    await screen.findByRole('heading', { name: 'Impact of this edit' })
    await expectAccessible()

    await user.click(screen.getByRole('button', { name: /Warnings/ }))
    await screen.findByRole('heading', { name: 'Warnings', level: 1 })
    await expectAccessible()

    await user.click(screen.getByRole('button', { name: 'AI connection' }))
    await user.click(
      await screen.findByRole('button', { name: 'Start the server' }),
    )
    await screen.findByText(/Running at/)
    await expectAccessible()

    await user.click(screen.getByRole('button', { name: 'About' }))
    await screen.findByRole('heading', { name: 'About Exiled Studio' })
    await expectAccessible()

    await user.click(screen.getByRole('button', { name: /Export/ }))
    await screen.findByRole('menu')
    await expectAccessible()
  })
})
