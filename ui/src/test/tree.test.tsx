// The passive tree screen (passive tree plan, T6) over the fake API's
// synthetic tree: the list view does what the canvas does (ADR-008):
// allocate along a path, remove with what it leaves unconnected, choose a
// mastery effect, change the ascendancy and the class (asking first), reset;
// each one undoable change; a node kept from the import stays.
import { screen, waitFor, within } from '@testing-library/react'
import { describe, expect, it } from 'vitest'

import { expectAccessible, renderApp } from './render'

async function openTree(options: Parameters<typeof renderApp>[0] = {}) {
  const ctx = await renderApp(options)
  await ctx.user.click(screen.getByRole('button', { name: 'Open a file…' }))
  await screen.findByRole('heading', { name: 'Dashboard', level: 1 })
  await ctx.user.click(screen.getByRole('button', { name: 'Passive tree' }))
  await screen.findByRole('heading', { name: 'Passive tree', level: 1 })
  // jsdom draws nothing: the list view.
  await ctx.user.click(await screen.findByRole('button', { name: 'List' }))
  return ctx
}

const points = () => screen.getByText(/^Passive points:/).textContent

describe('passive tree', () => {
  it('allocates, removes, and undoes from the list view', async () => {
    const { user, api } = await openTree()
    expect(points()).toBe('Passive points: 2 · Ascendancy points: 0')
    expect(
      screen.getByText(/^Allocated nodes kept as imported: 1\./),
    ).toBeTruthy()
    await expectAccessible()

    await user.click(screen.getByRole('button', { name: 'Keystones' }))
    await user.click(
      screen.getByRole('button', { name: 'Allocate Glass Mind' }),
    )
    await waitFor(() =>
      expect(points()).toBe('Passive points: 5 · Ascendancy points: 0'),
    )
    expect(api.executed).toContainEqual({
      'allocate-node': { spec: 1, node: 13 },
    })

    await user.click(screen.getByRole('button', { name: 'Allocated' }))
    await user.click(screen.getByRole('button', { name: 'Remove Cold Damage' }))
    // 11 leaves 12 and 13 unconnected; the kept node stays.
    await waitFor(() =>
      expect(points()).toBe('Passive points: 2 · Ascendancy points: 0'),
    )
    expect(screen.getByText('Bloodline Passive')).toBeTruthy()

    await user.click(screen.getByRole('button', { name: /^Undo/ }))
    await waitFor(() =>
      expect(points()).toBe('Passive points: 5 · Ascendancy points: 0'),
    )
  })

  it('is one Tab stop, says what a button changes, and asks with Escape to cancel', async () => {
    const { user } = await openTree()
    await user.click(screen.getByRole('button', { name: 'Keystones' }))
    const rows = screen
      .getAllByRole('button')
      .filter((b) => b.hasAttribute('data-row'))
    expect(rows.filter((b) => b.tabIndex === 0)).toHaveLength(1)
    rows[0]?.focus()
    // The node's lines and the count of nodes the button would allocate.
    await waitFor(() =>
      expect(rows[0]?.getAttribute('aria-describedby') ?? '').toContain('told'),
    )
    const told = rows[0]?.getAttribute('aria-describedby')?.split(' ').at(-1)
    expect(document.getElementById(told ?? '')?.textContent).toMatch(
      /^Allocates (this node only|\d+ nodes)/,
    )
    if (rows.length > 1) {
      await user.keyboard('{ArrowDown}')
      expect(document.activeElement).toBe(rows[1])
    }

    const reset = screen.getByRole('button', { name: 'Reset tree' })
    await user.click(reset)
    await screen.findByRole('alertdialog')
    await user.keyboard('{Escape}')
    await waitFor(() => expect(screen.queryByRole('alertdialog')).toBeNull())
    expect(document.activeElement).toBe(reset)
  })

  it('chooses a mastery effect, the ascendancy, and the class', async () => {
    const { user } = await openTree()
    await user.click(screen.getByRole('button', { name: 'Notables' }))
    await user.click(
      screen.getByRole('button', { name: 'Allocate Heart of Frost' }),
    )
    await user.click(screen.getByRole('button', { name: 'Masteries' }))
    await user.click(
      screen.getByRole('button', { name: 'Choose an effect for Cold Mastery' }),
    )
    const dialog = await screen.findByRole('dialog', { name: 'Cold Mastery' })
    await expectAccessible(dialog)
    await user.click(
      within(dialog).getByRole('button', { name: /Cold Exposure on Hit/ }),
    )
    await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull())
    await waitFor(() =>
      expect(points()).toBe('Passive points: 5 · Ascendancy points: 0'),
    )

    await user.selectOptions(
      screen.getByLabelText('Ascendancy'),
      'Elementalist',
    )
    await user.click(screen.getByRole('button', { name: 'Ascendancy' }))
    await user.click(
      await screen.findByRole('button', { name: 'Allocate Shaper of Winter' }),
    )
    await waitFor(() =>
      expect(points()).toBe('Passive points: 5 · Ascendancy points: 1'),
    )

    // Changing class asks first; Cancel changes nothing.
    await user.selectOptions(screen.getByLabelText('Class'), 'Marauder')
    const ask = await screen.findByRole('alertdialog')
    await user.click(within(ask).getByRole('button', { name: 'Cancel' }))
    expect(screen.getByLabelText<HTMLSelectElement>('Class').value).toBe('3')
    await user.selectOptions(screen.getByLabelText('Class'), 'Marauder')
    await user.click(
      within(await screen.findByRole('alertdialog')).getByRole('button', {
        name: 'Change class',
      }),
    )
    // Nothing connects to the Marauder's start; the kept node stays.
    await waitFor(() =>
      expect(points()).toBe('Passive points: 1 · Ascendancy points: 0'),
    )

    await user.click(screen.getByRole('button', { name: 'Reset tree' }))
    await user.click(
      within(await screen.findByRole('alertdialog')).getByRole('button', {
        name: 'Reset tree',
      }),
    )
    await waitFor(() =>
      expect(points()).toBe('Passive points: 0 · Ascendancy points: 0'),
    )
  })

  it('shows a keystone flavour text and the ascendancy quote', async () => {
    const { user } = await openTree()
    await user.click(screen.getByRole('button', { name: 'Keystones' }))
    const list = screen.getByRole('list', { name: 'Nodes' })
    const text = within(list).getByText('A made-up keystone line.')
    expect(text.closest('.tree-flavour')?.textContent).toBe(
      'A made-up keystone line.A second one.',
    )
    expect(screen.queryByText('A made-up quote.')).toBeNull()
    await user.selectOptions(
      screen.getByLabelText('Ascendancy'),
      'Elementalist',
    )
    expect(await screen.findByText('A made-up quote.')).toBeTruthy()
  })

  it('shows the calculated attribute totals by the points', async () => {
    await openTree()
    expect(
      screen.getByText('Strength 20 · Dexterity 5 · Intelligence 20'),
    ).toBeTruthy()
  })

  it('finds nodes by name and stat text', async () => {
    const { user } = await openTree()
    await user.type(screen.getByLabelText('Search nodes'), 'cold damage')
    expect(screen.getByText('2 found')).toBeTruthy()
    const list = screen.getByRole('list', { name: 'Nodes' })
    expect(within(list).getByText('Heart of Frost')).toBeTruthy()
    expect(within(list).queryByText('Life')).toBeNull()
  })

  it('shows a tree it cannot edit without offering edits', async () => {
    await openTree({
      treeLocked:
        "the spec is for tree version 3_25, the loaded data's is 3_29",
    })
    expect(
      screen.getByText(
        "This passive tree is shown, not edited: the spec is for tree version 3_25, the loaded data's is 3_29.",
      ),
    ).toBeTruthy()
    expect(screen.getByLabelText<HTMLSelectElement>('Class').disabled).toBe(
      true,
    )
    expect(
      screen.getByRole<HTMLButtonElement>('button', { name: 'Reset tree' })
        .disabled,
    ).toBe(true)
    expect(screen.queryByRole('button', { name: /^Remove / })).toBeNull()
    expect(screen.getByText('Bloodline Passive')).toBeTruthy()
    await expectAccessible()
  })

  it('converts a tree of an older version after asking', async () => {
    const { user, api } = await openTree({
      treeLocked:
        "the spec is for tree version 3_25, the loaded data's is 3_29",
      treeConvert: { version: '3_29', dropped: [11] },
    })
    await user.click(screen.getByRole('button', { name: 'Convert to 3_29' }))
    const ask = await screen.findByRole('alertdialog')
    expect(ask.textContent).toContain('1 allocated nodes')
    await expectAccessible(ask)
    await user.click(
      within(ask).getByRole('button', { name: 'Convert to 3_29' }),
    )
    await waitFor(() =>
      expect(api.executed).toContainEqual({
        'convert-tree-version': { spec: 1 },
      }),
    )
    // Converted: edited, nothing left to convert.
    await waitFor(() =>
      expect(
        screen.queryByText(/^This passive tree is shown, not edited/),
      ).toBeNull(),
    )
    expect(screen.queryByRole('button', { name: 'Convert to 3_29' })).toBeNull()
    expect(screen.getByLabelText<HTMLSelectElement>('Class').disabled).toBe(
      false,
    )
    // One undoable change.
    await user.click(screen.getByRole('button', { name: /^Undo/ }))
    await screen.findByRole('button', { name: 'Convert to 3_29' })
  })

  it('adds, renames, and deletes passive trees', async () => {
    const { user, api } = await openTree()
    const bar = screen.getByRole('group', { name: 'Passive trees' })
    await user.click(within(bar).getByRole('button', { name: 'New tree' }))
    const form = screen.getByRole('form', { name: 'New tree' })
    await user.type(within(form).getByLabelText('Name'), 'Bossing')
    await user.click(within(form).getByRole('button', { name: 'Save' }))
    await waitFor(() =>
      expect(api.executed).toContainEqual({
        'new-spec': { title: 'Bossing', copy: null },
      }),
    )
    await user.click(within(bar).getByRole('button', { name: 'Rename tree' }))
    const rename = screen.getByRole('form', { name: 'Rename tree' })
    const name = within(rename).getByLabelText('Name')
    await user.clear(name)
    await user.type(name, 'Campaign')
    await user.click(within(rename).getByRole('button', { name: 'Save' }))
    await waitFor(() => expect(within(bar).getByText(/^Campaign/)).toBeTruthy())
    await user.click(within(bar).getByRole('button', { name: 'Delete tree' }))
    await user.click(
      within(await screen.findByRole('alertdialog')).getByRole('button', {
        name: 'Delete tree',
      }),
    )
    await waitFor(() =>
      expect(api.executed).toContainEqual({ 'delete-spec': { spec: 1 } }),
    )
    await expectAccessible()
  })

  it('sockets a jewel of the build, empties the socket, and undoes', async () => {
    const { user, api } = await openTree()
    await user.type(screen.getByLabelText('Search nodes'), 'jewel socket')
    await user.click(
      screen.getByRole('button', { name: 'Allocate Jewel Socket' }),
    )
    await user.click(
      await screen.findByRole('button', {
        name: 'Choose jewel for Jewel Socket',
      }),
    )
    const dialog = await screen.findByRole('dialog', { name: 'Jewel Socket' })
    await expectAccessible(dialog)
    // The build's jewels only: not its rings or wand.
    const list = within(dialog).getByRole('list', { name: 'Jewels' })
    expect(within(list).getAllByRole('button')).toHaveLength(1)
    await user.click(within(list).getByRole('button', { name: /Frost Eye/ }))
    await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull())
    expect(api.executed).toContainEqual({
      'socket-jewel': { spec: 1, node: 21, item: 4 },
    })

    await user.click(
      screen.getByRole('button', { name: 'Choose jewel for Jewel Socket' }),
    )
    const again = await screen.findByRole('dialog', { name: 'Jewel Socket' })
    expect(
      within(again)
        .getByRole('button', { name: /Frost Eye/ })
        .getAttribute('aria-pressed'),
    ).toBe('true')
    await user.click(
      within(again).getByRole('button', { name: 'Empty socket' }),
    )
    await waitFor(() =>
      expect(api.executed).toContainEqual({
        'socket-jewel': { spec: 1, node: 21 },
      }),
    )
    await user.click(screen.getByRole('button', { name: 'Undo' }))
    await user.click(
      await screen.findByRole('button', {
        name: 'Choose jewel for Jewel Socket',
      }),
    )
    const undone = await screen.findByRole('dialog', { name: 'Jewel Socket' })
    expect(
      within(undone)
        .getByRole('button', { name: /Frost Eye/ })
        .getAttribute('aria-pressed'),
    ).toBe('true')
  })

  it('shows an AI tree request by node name and applies it on approval', async () => {
    const { user, api } = await openTree()
    await user.click(screen.getByRole('button', { name: 'AI connection' }))
    await user.click(
      await screen.findByRole('button', { name: 'Start the server' }),
    )
    await user.click(await screen.findByLabelText('Allow change requests'))
    api.addTreeRequest({ 'allocate-node': { spec: 1, node: 30 } })
    const request = (
      await screen.findByRole('heading', {
        name: 'test-client wants to change the passive tree',
      })
    ).closest('article') as HTMLElement
    // The path by name, never as JSON.
    expect(within(request).getByText('Add to the passive tree')).toBeTruthy()
    for (const name of ['Energy Shield', 'Jewel Socket', 'Life'])
      expect(within(request).getByText(name)).toBeTruthy()
    expect(within(request).queryByText(/allocate-node/)).toBeNull()
    expect(within(request).getByText(/^Life \d+ → \d+$/)).toBeTruthy()
    await expectAccessible(request)
    await user.click(
      within(request).getByRole('button', { name: 'Approve request 1' }),
    )
    expect(await within(request).findByText('Applied')).toBeTruthy()
    await user.click(screen.getByRole('button', { name: 'Passive tree' }))
    await waitFor(() =>
      expect(points()).toBe('Passive points: 5 · Ascendancy points: 0'),
    )
  })
})
