// Logs and diagnostics in the UI (Phase 5, P-f; ADR-014): the About
// screen previews the diagnostic package file by file and saves it only
// when asked; a crash record from an earlier run is shown on the start
// screen until the user dismisses it.
import { screen, waitFor, within } from '@testing-library/react'
import { describe, expect, it } from 'vitest'

import { expectAccessible, renderApp } from './render'

const crash = {
  id: '1700000000-42-0',
  at: 1_700_000_000,
  version: '0.0.0',
  thread: 'loom-calc',
  message: 'a test panic',
  location: null,
}

describe('diagnostics', () => {
  it('previews the package, then saves it where the user picks', async () => {
    const { user, api } = await renderApp()
    await user.click(screen.getByRole('button', { name: 'About' }))
    expect(
      screen.queryByRole('button', { name: 'Save the diagnostic package…' }),
    ).toBeNull()
    await user.click(
      screen.getByRole('button', { name: 'Preview the diagnostic package' }),
    )
    const files = await screen.findByRole('list', {
      name: 'Files in the package',
    })
    expect(within(files).getByText('about.json')).toBeTruthy()
    expect(within(files).getByText('logs/loom.log')).toBeTruthy()
    expect(api.diagnosticsExported).toBe(0)
    await expectAccessible()
    await user.click(
      screen.getByRole('button', { name: 'Save the diagnostic package…' }),
    )
    await screen.findByText('Diagnostic package saved.')
    expect(api.diagnosticsExported).toBe(1)
  })

  it('shows a crash record on the start screen until it is dismissed', async () => {
    const { user, api } = await renderApp({ crashes: [crash] })
    await screen.findByRole('heading', {
      name: 'An internal error was recorded',
    })
    expect(screen.getByText(/a test panic/)).toBeTruthy()
    await expectAccessible()
    await user.click(screen.getByRole('button', { name: 'Dismiss the record' }))
    await waitFor(() =>
      expect(
        screen.queryByRole('heading', {
          name: 'An internal error was recorded',
        }),
      ).toBeNull(),
    )
    expect(api.crashList).toEqual([])
  })
})
