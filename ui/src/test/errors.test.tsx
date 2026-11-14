// Error states (Phase 4 plan, decision 13): no data, a failed import, a
// failed calculation, a stale edit. Each says what happened, as an alert,
// with its code.
import { screen, waitFor } from '@testing-library/react'
import { describe, expect, it } from 'vitest'

import { expectAccessible, renderApp } from './render'

describe('error states', () => {
  it('says when no game data is loaded, and lets the user choose it', async () => {
    const { user } = await renderApp({ data: false })
    expect(
      screen.getByText(
        /No game data is loaded\. Choose the game data folder: the folder that holds bundle\.json/,
      ),
    ).toBeTruthy()
    await user.type(
      screen.getByLabelText('Paste a share code or build XML'),
      'abc',
    )
    await user.click(screen.getByRole('button', { name: 'Import build' }))
    const alert = await screen.findByRole('alert')
    expect(alert.textContent).toContain('APP_NO_DATA')
    await expectAccessible()
    await user.click(
      screen.getByRole('button', { name: 'Choose the game data folder…' }),
    )
    await waitFor(() =>
      expect(
        screen.getByText(/Data for Path of Exile, Steam build 26530219/),
      ).toBeTruthy(),
    )
  })

  it('shows a failed import with its code', async () => {
    const { user } = await renderApp({ importFails: 'IMPORT_XML' })
    await user.type(
      screen.getByLabelText('Paste a share code or build XML'),
      '<bad',
    )
    await user.click(screen.getByRole('button', { name: 'Import build' }))
    const alert = await screen.findByRole('alert')
    expect(alert.textContent).toContain('This is not a Path of Building build')
    expect(alert.textContent).toContain('IMPORT_XML')
  })

  it('says what to do in words, the code once and apart (UX-05)', async () => {
    const { user } = await renderApp({
      importFails: 'IMPORT_SHARE_CODE_BASE64',
    })
    await user.type(
      screen.getByLabelText('Paste a share code or build XML'),
      'not a code',
    )
    await user.click(screen.getByRole('button', { name: 'Import build' }))
    const alert = await screen.findByRole('alert')
    expect(alert.textContent).toContain('This is not a share code.')
    expect(alert.textContent).toContain('then Generate')
    expect(alert.textContent?.split('IMPORT_SHARE_CODE_BASE64')).toHaveLength(2)
    expect(screen.getByText('Code IMPORT_SHARE_CODE_BASE64')).toBeTruthy()
  })

  it('says links cannot be opened offline (UX-18)', async () => {
    const { user, api } = await renderApp()
    await user.type(
      screen.getByLabelText('Paste a share code or build XML'),
      'https://pobb.in/abc',
    )
    await user.click(screen.getByRole('button', { name: 'Import build' }))
    const alert = await screen.findByRole('alert')
    expect(alert.textContent).toContain('Links can’t be opened')
    expect(alert.textContent).not.toContain('UI_LINK')
    expect(api.executed).toEqual([])
  })

  it('words a code it does not know by its layer, never the bare code', async () => {
    const { user } = await renderApp({ importFails: 'BUNDLE_HASH_MISMATCH' })
    await user.type(
      screen.getByLabelText('Paste a share code or build XML'),
      'abc',
    )
    await user.click(screen.getByRole('button', { name: 'Import build' }))
    const alert = await screen.findByRole('alert')
    expect(alert.textContent).toContain('The game data is damaged')
  })

  it('shows a failed calculation in the header, never as numbers', async () => {
    const { user } = await renderApp({ calcFails: 'APP_DATA_VERSION' })
    await user.click(screen.getByRole('button', { name: 'Open a .xml file…' }))
    await user.click(
      await screen.findByRole('button', { name: 'Open the dashboard' }),
    )
    expect(
      await screen.findByText(
        /Calculation failed: This build was saved with another version of the game data/,
      ),
    ).toBeTruthy()
    expect(screen.getByText('No calculation yet.')).toBeTruthy()
    await expectAccessible()
  })

  it('refuses a stale edit and says why', async () => {
    const { user, api } = await renderApp()
    await user.click(screen.getByRole('button', { name: 'Open a .xml file…' }))
    await user.click(
      await screen.findByRole('button', { name: 'Open the dashboard' }),
    )
    await user.click(screen.getByRole('button', { name: 'Equipment' }))
    await user.click(
      await screen.findByRole('button', { name: 'Edit Loom Test Band' }),
    )
    // A line shows as the game prints it; Edit opens its PoB text (UX-07).
    await user.click(await screen.findByRole('button', { name: 'Edit line 4' }))
    const input = await screen.findByLabelText('Line 4 text')
    await user.clear(input)
    await user.type(input, '+20 to maximum Life')
    // Meanwhile the item changes (an approved AI request, for example).
    await api.execute({
      'edit-item': {
        item: 1,
        ops: [{ replace: { line: 3, text: '+16 to maximum Life' } }],
        expect: [{ line: 3, text: '+15 to maximum Life' }],
      },
    })
    await user.click(screen.getByRole('button', { name: 'Apply' }))
    const alert = await screen.findByRole('alert')
    expect(alert.textContent).toContain(
      'The item changed since this edit was made',
    )
    expect(alert.textContent).toContain('APP_STALE_EDIT')
  })
})
