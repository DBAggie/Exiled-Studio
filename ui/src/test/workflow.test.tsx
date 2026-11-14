// The UI workflow (Phase 4 plan, decision 14 c): import, review,
// dashboard with states and reasons, breakdown, item edit with preview,
// background recalculation announced, undo, save, export.
import { screen, waitFor, within } from '@testing-library/react'
import { describe, expect, it } from 'vitest'

import { renderApp } from './render'

/** A dashboard row: one button named "<label>: <value>, <state>." */
const row = (label: string) =>
  screen.getByRole('button', { name: new RegExp(`^${label}:`) })

describe('workflow', () => {
  it('imports, inspects, edits, recalculates, undoes, saves, and exports', async () => {
    const { user, api } = await renderApp()

    // Import from pasted text.
    await user.type(
      screen.getByLabelText('Paste a share code or build XML'),
      '<PathOfBuilding/>',
    )
    await user.click(screen.getByRole('button', { name: 'Import build' }))
    expect(document.activeElement).toBe(
      await screen.findByRole('heading', { name: 'Imported build', level: 1 }),
    )
    expect(screen.getByText('Nothing like this')).toBeTruthy()
    expect(screen.getByText('Lines to review')).toBeTruthy()

    // The dashboard: every field with its state; one without a value says why.
    await user.click(screen.getByRole('button', { name: 'Open the dashboard' }))
    await screen.findByRole('heading', { name: 'Dashboard', level: 1 })
    await waitFor(() =>
      expect(screen.getByRole('status').textContent).toBe(
        'Calculation finished.',
      ),
    )
    // Label, value, and badge are one row (one button).
    const life = row('Life')
    expect(within(life).getByText('70')).toBeTruthy()
    expect(within(life).getByText('Exact')).toBeTruthy()
    const dps = row('Hit DPS')
    expect(within(dps).getByText('Not yet')).toBeTruthy()
    expect(within(dps).getByText('—')).toBeTruthy()
    expect(
      within(dps).getByText('Not calculated by this version yet.'),
    ).toBeTruthy()
    expect(within(row('Average hit')).getByText('Cannot settle')).toBeTruthy()

    // The breakdown of life: the item by name, its line from 1.
    await user.click(row('Life'))
    await screen.findByRole('heading', { name: 'Why is Life 70?', level: 1 })
    const sources = screen.getByRole('table')
    expect(within(sources).getByText('Loom Test Band, line 4')).toBeTruthy()
    await user.click(
      screen.getByRole('button', { name: 'Back to the dashboard' }),
    )

    // Edit an item: preview, then apply.
    await user.click(screen.getByRole('button', { name: 'Equipment' }))
    await user.click(
      await screen.findByRole('button', { name: 'Edit Loom Test Band' }),
    )
    const input = await screen.findByLabelText('Line 4 text')
    await user.clear(input)
    await user.type(input, '+40 to maximum Life')
    expect(screen.getByText('Edited')).toBeTruthy()
    await user.click(screen.getByRole('button', { name: 'Preview effect' }))
    const impact = (
      await screen.findByRole('heading', { name: 'Impact of this edit' })
    ).closest('section') as HTMLElement
    await waitFor(() => expect(within(impact).getByText('70 →')).toBeTruthy())
    expect(within(impact).getByText('95')).toBeTruthy()
    expect(within(impact).getByText('+25')).toBeTruthy()
    await user.click(screen.getByRole('button', { name: 'Apply' }))
    expect(
      await screen.findByText('Edit applied. Undo is available.'),
    ).toBeTruthy()
    await waitFor(() => expect(screen.getByText('Up to date')).toBeTruthy())
    expect(
      api.executed.some((c) => typeof c === 'object' && 'edit-item' in c),
    ).toBe(true)

    // Undo restores the earlier calculation.
    await user.click(screen.getByRole('button', { name: 'Undo' }))
    await user.click(screen.getByRole('button', { name: 'Dashboard' }))
    await waitFor(() =>
      expect(within(row('Life')).getByText('70')).toBeTruthy(),
    )
    expect(
      (screen.getByRole('button', { name: 'Redo' }) as HTMLButtonElement)
        .disabled,
    ).toBe(false)

    // Save: the first save asks where (save as), then the build is clean.
    expect(screen.getByText('Unsaved changes')).toBeTruthy()
    await user.click(screen.getByRole('button', { name: 'Save' }))
    await waitFor(() =>
      expect(screen.queryByText('Unsaved changes')).toBeNull(),
    )

    // Export a share code.
    await user.click(screen.getByRole('button', { name: /Export/ }))
    await user.click(
      await screen.findByRole('menuitem', {
        name: 'Path of Building share code',
      }),
    )
    const dialog = await screen.findByRole('dialog', { name: 'Share code' })
    expect(
      (within(dialog).getByRole('textbox') as HTMLTextAreaElement).value,
    ).toBe('eNrFAKESHARECODE')
    await user.keyboard('{Escape}')
    await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull())
  })

  it('approves an AI change request as one undoable step', async () => {
    const { user, api } = await renderApp()
    await user.click(screen.getByRole('button', { name: 'Open a .xml file…' }))
    await user.click(
      await screen.findByRole('button', { name: 'Open the dashboard' }),
    )
    await user.click(screen.getByRole('button', { name: 'AI connection' }))
    await user.click(
      await screen.findByRole('button', { name: 'Start the server' }),
    )
    expect(
      await screen.findByText(/Running at http:\/\/127\.0\.0\.1/),
    ).toBeTruthy()
    // The command to copy names the user scope, so the server shows in
    // every folder.
    expect(screen.getByText(/claude mcp add --scope user/)).toBeTruthy()
    await user.click(screen.getByLabelText('Allow change requests'))
    api.addRequest({
      item: 1,
      ops: [{ replace: { line: 3, text: '+30 to maximum Life' } }],
      expect: [{ line: 3, text: '+15 to maximum Life' }],
    })
    // The pending count shows in the side menu.
    expect(
      await screen.findByRole('button', {
        name: /AI connection 1 waiting for you/,
      }),
    ).toBeTruthy()
    // The edit reads as words with the item named, never as JSON.
    const request = (
      await screen.findByRole('heading', {
        name: 'test-client wants to change Loom Test Band',
      })
    ).closest('article') as HTMLElement
    expect(within(request).getByText('Remove from Loom Test Band')).toBeTruthy()
    expect(within(request).getByText('+15 to maximum Life')).toBeTruthy()
    expect(within(request).getByText('Add to Loom Test Band')).toBeTruthy()
    expect(within(request).getByText('+30 to maximum Life')).toBeTruthy()
    expect(within(request).queryByText(/"replace"/)).toBeNull()
    // The effect, calculated by the app, with the dashboard's labels.
    expect(within(request).getByText('Life 70 → 85')).toBeTruthy()
    await user.click(
      within(request).getByRole('button', { name: 'Approve request 1' }),
    )
    expect(await within(request).findByText('Applied')).toBeTruthy()
    expect(
      (screen.getByRole('button', { name: 'Undo' }) as HTMLButtonElement)
        .disabled,
    ).toBe(false)
  })
})
