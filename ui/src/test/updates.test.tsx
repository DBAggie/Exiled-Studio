// Application updates in the UI (Phase 5, P-d; docs/phase-5/updates.md):
// the setting with its privacy note, a check on request, the prompt that
// installs only on the user's click, and a package that only notifies.
import { screen, waitFor } from '@testing-library/react'
import { describe, expect, it } from 'vitest'

import { expectAccessible, renderApp } from './render'

const offer = {
  version: '0.2.0',
  notes: 'Faster trees.',
  date: '2026-11-15T12:00:00Z',
  url: 'https://github.com/DBAggie/Exiled-Studio/releases/tag/v0.2.0',
}

async function openAbout(user: Awaited<ReturnType<typeof renderApp>>['user']) {
  await user.click(screen.getByRole('button', { name: 'About' }))
  await screen.findByRole('heading', { name: 'Updates' })
}

describe('updates', () => {
  it('shows the mode with its privacy note and remembers a change', async () => {
    const { user, api } = await renderApp()
    await openAbout(user)
    expect(screen.getByText(/GitHub sees your IP address/)).toBeTruthy()
    expect(
      (
        screen.getByRole('radio', {
          name: 'Check at start',
        }) as HTMLInputElement
      ).checked,
    ).toBe(true)
    await expectAccessible()
    await user.click(
      screen.getByRole('radio', { name: 'Never connect (strict offline)' }),
    )
    await waitFor(() => expect(api.updates.mode).toBe('strict-offline'))
    expect(screen.queryByRole('button', { name: 'Check now' })).toBeNull()
    expect(
      screen.getByText('Strict offline: Exiled Studio never connects.'),
    ).toBeTruthy()
  })

  it('checks on request and installs only when the user clicks', async () => {
    const { user, api } = await renderApp({ nextOffer: offer })
    await openAbout(user)
    await user.click(screen.getByRole('button', { name: 'Check now' }))
    const prompt = await screen.findByRole('region', {
      name: 'Update available',
    })
    expect(prompt.textContent).toContain('Version 0.2.0 is available')
    expect(api.installs).toBe(0)
    await expectAccessible()
    const installs = screen.getAllByRole('button', {
      name: 'Install and restart',
    })
    await user.click(installs[0])
    expect(api.installs).toBe(1)
  })

  it('reports the newest version when the check finds none', async () => {
    const { user } = await renderApp({ updates: { current: '0.1.0' } })
    await openAbout(user)
    await user.click(screen.getByRole('button', { name: 'Check now' }))
    await screen.findByText('You have the newest version (0.1.0).')
  })

  it('puts the prompt off with Later', async () => {
    const { user, api } = await renderApp({ nextOffer: offer })
    api.emitUpdates()
    const prompt = await screen.findByRole('region', {
      name: 'Update available',
    })
    expect(prompt).toBeTruthy()
    await user.click(screen.getByRole('button', { name: 'Later' }))
    await waitFor(() =>
      expect(
        screen.queryByRole('region', { name: 'Update available' }),
      ).toBeNull(),
    )
    expect(api.installs).toBe(0)
  })

  it('a package that only notifies shows the release page, no install', async () => {
    const { api } = await renderApp({
      nextOffer: offer,
      updates: { artifact: 'deb', installs: false },
    })
    api.emitUpdates()
    const prompt = await screen.findByRole('region', {
      name: 'Update available',
    })
    expect(prompt.textContent).toContain(offer.url)
    expect(
      screen.queryByRole('button', { name: 'Install and restart' }),
    ).toBeNull()
  })

  it('a build without an update key says so and offers no check', async () => {
    const { user } = await renderApp({ updates: { available: false } })
    await openAbout(user)
    expect(screen.getByText(/carries no update key/)).toBeTruthy()
    expect(screen.queryByRole('button', { name: 'Check now' })).toBeNull()
  })
})
