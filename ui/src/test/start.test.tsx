// Start page B (UX brief note 4; dev/active/ux-improvements/
// start-page-plan.md): the command bar imports or finds a build, New build
// (B-3), Import from clipboard, Ctrl N, dropped files, and asking before
// any of them replaces unsaved work (UX-01).
import { act, fireEvent, screen, waitFor, within } from '@testing-library/react'
import { describe, expect, it } from 'vitest'

import { expectAccessible, renderApp } from './render'

const recent = (id: string, name: string, extra = {}) => ({
  id,
  path: `C:\\builds\\${name}.json`,
  name,
  at: Math.floor(Date.now() / 1000) - 60,
  class: null,
  ascendancy: null,
  level: null,
  'main-skill': null,
  ...extra,
})

describe('start page', () => {
  it('starts a new build from the button and from Ctrl N', async () => {
    const { user, api } = await renderApp()
    await user.click(screen.getByRole('button', { name: 'New build' }))
    expect(api.executed).toEqual(['new-build'])
    await screen.findByRole('heading', { name: 'Dashboard', level: 1 })

    // From the start page, Ctrl N does the same.
    await user.click(screen.getByRole('button', { name: 'Close build' }))
    await screen.findByRole('heading', { name: 'What do you want to do?' })
    await user.keyboard('{Control>}n{/Control}')
    await screen.findByRole('heading', { name: 'Dashboard', level: 1 })
    expect(api.executed.filter((c) => c === 'new-build')).toHaveLength(2)
  })

  it('asks before a new build replaces unsaved changes', async () => {
    const { user, api } = await renderApp()
    await user.type(
      screen.getByLabelText('Paste a share code or build XML'),
      '<PathOfBuilding/>',
    )
    await user.click(screen.getByRole('button', { name: 'Import build' }))
    await screen.findByRole('button', { name: 'Open the dashboard' })
    // An import is unsaved.
    await user.keyboard('{Control>}n{/Control}')
    const dialog = await screen.findByRole('dialog', {
      name: 'Replace this build?',
    })
    await expectAccessible(dialog)
    await user.click(within(dialog).getByRole('button', { name: 'Cancel' }))
    expect(api.executed).not.toContain('new-build')
    await user.keyboard('{Control>}n{/Control}')
    await user.click(
      await screen.findByRole('button', { name: 'Go on without saving' }),
    )
    await waitFor(() => expect(api.executed).toContain('new-build'))
  })

  it('finds recent builds by what is typed, and imports what is pasted', async () => {
    const { user, api } = await renderApp({
      recent: [
        recent('a', 'witch', { ascendancy: 'Elementalist', level: 92 }),
        recent('b', 'smite', { class: 'Templar', 'main-skill': 'Smite' }),
      ],
    })
    const bar = screen.getByLabelText('Paste a share code or build XML')
    await user.type(bar, 'smi')
    expect(screen.getByText('1 of 2')).toBeTruthy()
    expect(screen.queryByRole('button', { name: 'witch' })).toBeNull()
    expect(screen.getByRole('button', { name: 'smite' })).toBeTruthy()
    // By what the build is, too.
    await user.clear(bar)
    await user.type(bar, 'element')
    expect(screen.getByRole('button', { name: 'witch' })).toBeTruthy()
    expect(screen.queryByRole('button', { name: 'smite' })).toBeNull()

    // A build pasted into the empty bar imports at once.
    await user.clear(bar)
    await user.click(bar)
    await user.paste('<PathOfBuilding><Build/></PathOfBuilding>')
    await screen.findByRole('button', { name: 'Open the dashboard' })
    expect(api.executed).toContainEqual({
      'import-text': { text: '<PathOfBuilding><Build/></PathOfBuilding>' },
    })
  })

  it('imports text pasted on the page outside a field (Ctrl V)', async () => {
    const { api } = await renderApp()
    // jsdom has no DataTransfer: the event's clipboard data as a stub.
    fireEvent.paste(document.body, {
      clipboardData: { getData: () => '<PathOfBuilding/>' },
    })
    await screen.findByRole('button', { name: 'Open the dashboard' })
    expect(api.executed).toContainEqual({
      'import-text': { text: '<PathOfBuilding/>' },
    })
  })

  it('opens a dropped file by its kind and refuses others', async () => {
    const { api } = await renderApp()
    act(() => api.drop({ state: 'over' }))
    expect(screen.getByText(/Drop a build file to open it/)).toBeTruthy()
    act(() => api.drop({ state: 'left' }))
    expect(screen.queryByText(/Drop a build file to open it/)).toBeNull()

    act(() => api.drop({ state: 'dropped', kind: 'other', name: 'a.txt' }))
    expect((await screen.findByRole('alert')).textContent).toContain(
      'not a build file',
    )
    act(() => api.drop({ state: 'dropped', kind: 'pob', name: 'b.xml' }))
    await screen.findByRole('button', { name: 'Open the dashboard' })
  })

  it('asks about the latest drop when a second file is dropped', async () => {
    const { user, api } = await renderApp()
    await user.type(
      screen.getByLabelText('Paste a share code or build XML'),
      '<PathOfBuilding/>',
    )
    await user.click(screen.getByRole('button', { name: 'Import build' }))
    await screen.findByRole('button', { name: 'Open the dashboard' })
    // Unsaved: the first drop asks.
    act(() => api.drop({ state: 'dropped', kind: 'pob', name: 'a.xml' }))
    await screen.findByRole('dialog', { name: 'Replace this build?' })
    // A second drop replaces the question; the answer opens it.
    act(() => api.drop({ state: 'dropped', kind: 'build', name: 'b.json' }))
    await user.click(
      await screen.findByRole('button', { name: 'Go on without saving' }),
    )
    await screen.findByRole('heading', { name: 'Dashboard', level: 1 })
    expect(screen.queryByRole('dialog')).toBeNull()
  })
})
