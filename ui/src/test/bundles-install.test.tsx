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
})
