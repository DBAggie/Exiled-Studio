// Installing game data (Phase 5, P-e; docs/phase-5/data-bundles.md): a
// signed pack installs and is labelled signed; an unsigned one waits in
// quarantine, its fingerprint shown, until the user accepts it; a bundle
// the user verifies is checked file by file.
import { screen, waitFor, within } from '@testing-library/react'
import { describe, expect, it } from 'vitest'

import { expectAccessible, renderApp } from './render'

async function openAbout(user: Awaited<ReturnType<typeof renderApp>>['user']) {
  await user.click(screen.getByRole('button', { name: 'About' }))
  await screen.findByRole('heading', { name: 'Game data' })
}

describe('installing game data', () => {
  it('installs a signed pack and labels it signed', async () => {
    const { user } = await renderApp({
      nextPack: { build: 26530219, signed: true },
    })
    await openAbout(user)
    await user.click(
      screen.getByRole('button', { name: 'Install game data from a file…' }),
    )
    await screen.findByText('Game data installed.')
    expect(screen.getByText('Signed by the project')).toBeTruthy()
    await expectAccessible()
  })

  it('quarantines an unsigned pack until the user accepts its fingerprint', async () => {
    const { user, api } = await renderApp({
      nextPack: { build: 100, signed: false },
    })
    await openAbout(user)
    await user.click(
      screen.getByRole('button', { name: 'Install game data from a file…' }),
    )
    const region = await screen.findByRole('region', {
      name: 'Installed, not loaded',
    })
    expect(within(region).getByText('Not signed.')).toBeTruthy()
    expect(within(region).getByText(new RegExp('ab'.repeat(32)))).toBeTruthy()
    await expectAccessible()
    await user.click(
      within(region).getByRole('button', {
        name: 'Load poe1/100-s4 anyway: I trust this data',
      }),
    )
    await waitFor(() =>
      expect(
        screen.queryByRole('region', { name: 'Installed, not loaded' }),
      ).toBeNull(),
    )
    expect(api.acceptedBuilds).toEqual([100])
    expect(
      screen.getByText('Accepted by you (not signed by the project)'),
    ).toBeTruthy()
  })

  it('a bundle with a bad signature cannot be accepted', async () => {
    const { user } = await renderApp({
      quarantined: [
        {
          id: 'poe1/7-s4',
          'data-version': {
            game: 'poe1',
            'schema-version': 4,
            'steam-build': 7,
          },
          reason: 'bad-signature',
          acceptable: false,
          'manifest-sha256': 'cd'.repeat(32),
          'key-id': null,
          files: 3,
          bytes: 10,
        },
      ],
    })
    await openAbout(user)
    const region = screen.getByRole('region', { name: 'Installed, not loaded' })
    expect(within(region).getByText(/does not match/)).toBeTruthy()
    expect(within(region).queryByRole('button')).toBeNull()
  })

  it('verifies an installed bundle', async () => {
    const { user, api } = await renderApp({ installed: [26530219] })
    await openAbout(user)
    await user.click(
      screen.getByRole('button', { name: 'Verify poe1/26530219-s4' }),
    )
    await screen.findByText(
      'Verified: every file of poe1/26530219-s4 matches its signed list.',
    )
    expect(api.verified).toEqual(['poe1/26530219-s4'])
  })

  it('lists the feed game data and installs one on request', async () => {
    const { user, api } = await renderApp({
      installed: [26530219],
      feed: {
        sequence: 4,
        offers: [
          { id: 'poe1/26600000-s4', bytes: 2048, installed: false },
          { id: 'poe1/26530219-s4', bytes: 1024, installed: true },
        ],
      },
    })
    await openAbout(user)
    await user.click(
      screen.getByRole('button', { name: 'Check for new game data' }),
    )
    const list = await screen.findByRole('list', {
      name: 'Game data the project publishes',
    })
    expect(within(list).getByText('Installed')).toBeTruthy()
    await expectAccessible()
    await user.click(
      within(list).getByRole('button', {
        name: 'Download and install poe1/26600000-s4',
      }),
    )
    await screen.findByText('Game data installed.')
    expect(api.downloads).toEqual(['poe1/26600000-s4'])
  })

  it('strict offline offers no feed check', async () => {
    const { user } = await renderApp({ updates: { mode: 'strict-offline' } })
    await openAbout(user)
    expect(
      screen.queryByRole('button', { name: 'Check for new game data' }),
    ).toBeNull()
    expect(screen.getByText(/does not look for new game data/)).toBeTruthy()
  })
})
