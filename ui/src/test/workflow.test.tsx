// The UI workflow (Phase 4 plan, decision 14 c): import, review,
// dashboard with states and reasons, breakdown, item edit with preview,
// background recalculation announced, undo, save, export.
import { screen, waitFor, within } from '@testing-library/react'
import { describe, expect, it } from 'vitest'

import { renderApp } from './render'

describe('workflow', () => {
  it('imports, inspects, edits, recalculates, undoes, saves, and exports', async () => {
    const { user, api } = await renderApp()

    // Import from pasted text.
    await user.type(
      screen.getByLabelText('Share code or XML'),
      '<PathOfBuilding/>',
    )
    await user.click(screen.getByRole('button', { name: 'Import' }))
    expect(document.activeElement).toBe(
      await screen.findByRole('heading', { name: 'Import result', level: 1 }),
    )
    expect(screen.getByText('Nothing like this')).toBeTruthy()

    // The dashboard: every field with its state; one without a value says why.
    await user.click(screen.getByRole('button', { name: 'Open the dashboard' }))
    await screen.findByRole('heading', { name: 'Dashboard', level: 1 })
    await waitFor(() =>
      expect(screen.getByRole('status').textContent).toBe(
        'Calculation finished.',
      ),
    )
    const lifeRow = screen
      .getByRole('button', { name: 'Explain Life' })
      .closest('tr')
    expect(lifeRow).not.toBeNull()
    expect(within(lifeRow as HTMLElement).getByText('70')).toBeTruthy()
    expect(within(lifeRow as HTMLElement).getByText('Exact')).toBeTruthy()
    const dpsRow = screen
      .getByRole('button', { name: 'Explain Hit DPS' })
      .closest('tr') as HTMLElement
    expect(within(dpsRow).getByText('Not yet')).toBeTruthy()
    expect(within(dpsRow).getByText('—')).toBeTruthy()
    expect(
      within(dpsRow).getByText('Not calculated by this version yet.'),
    ).toBeTruthy()
    const avgRow = screen
      .getByRole('button', { name: 'Explain Average hit' })
      .closest('tr') as HTMLElement
    expect(within(avgRow).getByText('Cannot settle')).toBeTruthy()

    // The breakdown of life.
    await user.click(screen.getByRole('button', { name: 'Explain Life' }))
    await screen.findByRole('heading', { name: 'Why is Life 70?', level: 1 })
    expect(screen.getByText(/Item 1, line 4/)).toBeTruthy()
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
    const effect = await screen.findByRole('heading', {
      name: /Effect of the edit/,
    })
    const table = effect.parentElement as HTMLElement
    expect(within(table).getByText('70')).toBeTruthy()
    expect(within(table).getByText('95')).toBeTruthy()
    await user.click(screen.getByRole('button', { name: 'Apply' }))
    expect(
      await screen.findByText('Edit applied. Undo is available.'),
    ).toBeTruthy()
    await waitFor(() =>
      expect(screen.getByText('Calculation up to date')).toBeTruthy(),
    )
    expect(
      api.executed.some((c) => typeof c === 'object' && 'edit-item' in c),
    ).toBe(true)

    // Undo restores the earlier calculation.
    await user.click(screen.getByRole('button', { name: 'Undo' }))
    await user.click(screen.getByRole('button', { name: 'Dashboard' }))
    await waitFor(() => {
      const row = screen
        .getByRole('button', { name: 'Explain Life' })
        .closest('tr') as HTMLElement
      expect(within(row).getByText('70')).toBeTruthy()
    })
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
    await user.click(screen.getByLabelText('Allow change requests'))
    api.addRequest({
      item: 1,
      ops: [{ replace: { line: 3, text: '+30 to maximum Life' } }],
      expect: [{ line: 3, text: '+15 to maximum Life' }],
    })
    await user.click(
      await screen.findByRole('button', { name: 'Approve request 1' }),
    )
    expect(await screen.findByText('Applied')).toBeTruthy()
    expect(
      (screen.getByRole('button', { name: 'Undo' }) as HTMLButtonElement)
        .disabled,
    ).toBe(false)
  })
})
