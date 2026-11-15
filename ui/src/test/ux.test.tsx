// The High fixes of the UX walkthrough (docs/ui/ux-walkthrough.md):
// closing asks before dropping unsaved work (UX-01), the equipment list
// shows the active item set and can switch it (UX-02), warnings on items
// not in use leave the status alone (UX-03), and the breakdown reads in
// words (UX-04).
import { fireEvent, screen, waitFor, within } from '@testing-library/react'
import { describe, expect, it } from 'vitest'

import { expectAccessible, renderApp } from './render'

async function importBuild() {
  const ctx = await renderApp()
  await ctx.user.type(
    screen.getByLabelText('Paste a share code or build XML'),
    '<PathOfBuilding/>',
  )
  await ctx.user.click(screen.getByRole('button', { name: 'Import build' }))
  await ctx.user.click(
    await screen.findByRole('button', { name: 'Open the dashboard' }),
  )
  await screen.findByRole('heading', { name: 'Dashboard', level: 1 })
  return ctx
}

describe('UX High fixes', () => {
  it('asks before closing a build with unsaved changes (UX-01)', async () => {
    const { user, api } = await importBuild()
    await user.click(screen.getByRole('button', { name: 'Close build' }))
    const dialog = await screen.findByRole('dialog', {
      name: 'Close this build?',
    })
    await expectAccessible(dialog)
    // Cancel keeps the build open.
    await user.click(within(dialog).getByRole('button', { name: 'Cancel' }))
    await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull())
    expect(api.executed).not.toContain('close')
    expect(screen.getByRole('heading', { name: 'Dashboard' })).toBeTruthy()

    // Save and close saves first, then closes.
    await user.click(screen.getByRole('button', { name: 'Close build' }))
    await user.click(
      await screen.findByRole('button', { name: 'Save and close' }),
    )
    await screen.findByRole('heading', { name: 'What do you want to do?' })
    expect(api.executed).toContain('close')
  })

  it('closes without asking when nothing is unsaved, or when told to', async () => {
    const { user, api } = await renderApp()
    await user.click(screen.getByRole('button', { name: 'Open a file…' }))
    await screen.findByRole('heading', { name: 'Dashboard', level: 1 })
    await user.click(screen.getByRole('button', { name: 'Close build' }))
    await screen.findByRole('heading', { name: 'What do you want to do?' })
    expect(screen.queryByRole('dialog')).toBeNull()
    expect(api.executed).toContain('close')

    // An import is unsaved: close without saving drops it.
    await user.type(
      screen.getByLabelText('Paste a share code or build XML'),
      '<PathOfBuilding/>',
    )
    await user.click(screen.getByRole('button', { name: 'Import build' }))
    await user.click(
      await screen.findByRole('button', { name: 'Open the dashboard' }),
    )
    await user.click(screen.getByRole('button', { name: 'Close build' }))
    await user.click(
      await screen.findByRole('button', { name: 'Close without saving' }),
    )
    await screen.findByRole('heading', { name: 'What do you want to do?' })
    expect(api.executed.filter((c) => c === 'close')).toHaveLength(2)
  })

  it('counts only warnings on items in use (UX-03)', async () => {
    const { user } = await importBuild()
    // Two lines have warnings; one is on an item of another item set.
    expect(screen.getByText('Partial · 1 warning')).toBeTruthy()
    await user.click(screen.getByRole('button', { name: /^Warnings/ }))
    const used = screen.getByRole('region', { name: 'On items in use' })
    expect(within(used).getByText('Nothing like this')).toBeTruthy()
    const unused = screen.getByRole('region', { name: 'On items not in use' })
    expect(within(unused).getByText('Something else unread')).toBeTruthy()
  })

  it('shows the active item set first and switches sets (UX-02)', async () => {
    const { user, api } = await importBuild()
    await user.click(screen.getByRole('button', { name: 'Equipment' }))
    const inUse = (
      await screen.findByRole('heading', { name: 'In use' })
    ).closest('section') as HTMLElement
    expect(within(inUse).getByText('Loom Test Band')).toBeTruthy()
    expect(within(inUse).queryByText('Spare Band')).toBeNull()
    expect(screen.getByText('2 items not in use')).toBeTruthy()
    await expectAccessible()

    // Switching the set is a command, undoable like an edit.
    await user.selectOptions(screen.getByLabelText('Item set'), 'Endgame')
    await waitFor(() =>
      expect(within(inUse).getByText('Spare Band')).toBeTruthy(),
    )
    expect(within(inUse).queryByText('Loom Test Band')).toBeNull()
    expect(api.executed).toContainEqual({ 'use-item-set': { id: 2 } })
    await user.click(screen.getByRole('button', { name: 'Undo' }))
    await waitFor(() =>
      expect(within(inUse).getByText('Loom Test Band')).toBeTruthy(),
    )
  })

  it('reads the breakdown in words (UX-04)', async () => {
    const { user } = await importBuild()
    await waitFor(() =>
      expect(screen.getByRole('status').textContent).toBe(
        'Calculation finished.',
      ),
    )
    await user.click(screen.getByRole('button', { name: /^Life:/ }))
    await screen.findByRole('heading', { name: 'Why is Life 70?', level: 1 })
    // Steps in words, not IDs.
    const steps = (
      screen
        .getByRole('heading', { name: 'How it is built' })
        .closest('section') as HTMLElement
    ).textContent
    expect(steps).toContain('Base')
    expect(steps).not.toContain('step.')
    // Sources grouped by what they change; the effect as the game says it.
    const table = screen.getByRole('table')
    expect(within(table).getByRole('rowheader', { name: 'Life' })).toBeTruthy()
    expect(within(table).getByText('+20')).toBeTruthy()
    await expectAccessible()
  })
  it('has keyboard shortcuts and says what undo would undo (UX-23)', async () => {
    const { user, api, store } = await importBuild()
    await user.click(screen.getByRole('button', { name: 'Equipment' }))
    await user.selectOptions(
      await screen.findByLabelText('Item set'),
      'Endgame',
    )
    const undo = screen.getByRole('button', { name: 'Undo' })
    await waitFor(() =>
      expect(undo.getAttribute('title')).toBe(
        'Undo switch to item set Endgame (Ctrl+Z)',
      ),
    )
    await user.keyboard('{Control>}z{/Control}')
    await waitFor(() => expect(api.executed).toContain('undo'))
    await user.keyboard('{Control>}y{/Control}')
    await waitFor(() => expect(api.executed).toContain('redo'))
    await user.keyboard('{Control>}s{/Control}')
    await waitFor(() =>
      expect(store.getState().session.snapshot?.dirty).toBe(false),
    )
    // In a text field, Ctrl+Z is the field's own.
    const before = api.executed.filter((c) => c === 'undo').length
    await user.click(screen.getByRole('button', { name: 'Skills' }))
    await user.click(await screen.findByLabelText('Ice Nova level'))
    await user.keyboard('{Control>}z{/Control}')
    expect(api.executed.filter((c) => c === 'undo')).toHaveLength(before)
  })

  it('previews an item edit by itself after a pause (UX-24)', async () => {
    const { user } = await importBuild()
    await user.click(screen.getByRole('button', { name: 'Equipment' }))
    await user.click(
      await screen.findByRole('button', { name: 'Edit Loom Test Band' }),
    )
    // A line shows as the game prints it; Edit opens its PoB text (UX-07).
    await user.click(await screen.findByRole('button', { name: 'Edit line 4' }))
    const input = await screen.findByLabelText('Line 4 text')
    await user.clear(input)
    await user.type(input, '+40 to maximum Life')
    // No click on Preview: the effect comes on its own.
    expect(await screen.findByText('+25', {}, { timeout: 3000 })).toBeTruthy()
  })

  it('puts unsaved work first and keeps recent builds short (UX-19, UX-20)', async () => {
    const now = Math.floor(Date.now() / 1000)
    const { user, api } = await renderApp({
      recent: [
        {
          id: 'a1',
          path: 'C:\\builds\\witch.json',
          name: 'witch',
          at: now - 7200,
          class: 'Witch',
          ascendancy: 'Elementalist',
          level: 92,
          'main-skill': 'Ice Nova',
        },
      ],
      recovery: [
        {
          id: 'r1',
          'saved-at': now - 120,
          name: 'lost',
          path: null,
          origin: 'C:\\pob\\lost.xml',
        },
      ],
    })
    // Recovery comes before the command bar.
    const recovery = screen.getByRole('heading', {
      name: 'Unsaved work from a session that ended',
    })
    const bar = screen.getByLabelText('Paste a share code or build XML')
    expect(
      recovery.compareDocumentPosition(bar) & Node.DOCUMENT_POSITION_FOLLOWING,
    ).toBeTruthy()
    expect(screen.getByText(/Last changed 2 minutes ago/)).toBeTruthy()
    expect(screen.getByRole('button', { name: 'Restore lost' })).toBeTruthy()
    // Recent: the file name, the time in words, and a way to remove it.
    expect(screen.getByText('witch.json').getAttribute('title')).toBe(
      'C:\\builds\\witch.json',
    )
    expect(screen.getByText('2 hours ago')).toBeTruthy()
    // What the build is (UX-20).
    expect(screen.getByText('Elementalist 92 · Ice Nova')).toBeTruthy()
    await user.click(
      screen.getByRole('button', { name: 'Remove witch from recent builds' }),
    )
    expect(api.executed).toContainEqual({ 'forget-recent': { id: 'a1' } })
    expect(await screen.findByText('No recent builds.')).toBeTruthy()
    await expectAccessible()
  })

  it('adds gems from the picker and removes gems and groups', async () => {
    const { user, api } = await importBuild()
    await user.click(screen.getByRole('button', { name: 'Skills' }))
    const weapon = (
      await screen.findByRole('heading', { name: 'Weapon 1', level: 2 })
    ).closest('section') as HTMLElement
    await user.click(within(weapon).getByRole('button', { name: 'Add gem' }))
    const dialog = await screen.findByRole('dialog', {
      name: 'Add gems to Group 1',
    })
    // Skills first; supports that fit the group before the others.
    expect(
      await within(dialog).findByRole('button', { name: 'Add Ice Nova' }),
    ).toBeTruthy()
    await user.click(within(dialog).getByRole('button', { name: 'Supports' }))
    const fitting = within(dialog).getByRole('list', { name: 'Gems' })
    expect(
      within(fitting).getByRole('button', { name: 'Add Added Cold Damage' }),
    ).toBeTruthy()
    expect(
      within(fitting).queryByRole('button', {
        name: 'Add Melee Physical Damage',
      }),
    ).toBeNull()
    expect(
      within(dialog).getByText(
        'Supports that support nothing in this group (1)',
      ),
    ).toBeTruthy()
    await user.type(within(dialog).getByLabelText('Search gems'), 'cold')
    await user.click(
      within(fitting).getByRole('button', { name: 'Add Added Cold Damage' }),
    )
    expect(api.executed).toContainEqual({
      'add-gem': { group: 0, gem: 'AddedColdDamageSupport' },
    })
    expect(
      await within(dialog).findByText('Added: Added Cold Damage'),
    ).toBeTruthy()
    await expectAccessible(dialog)
    await user.click(within(dialog).getByRole('button', { name: 'Done' }))
    await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull())

    // Remove a gem, add a group to the boots, remove it again.
    await user.click(
      within(weapon).getAllByRole('button', {
        name: 'Remove Added Cold Damage',
      })[0] as HTMLElement,
    )
    expect(api.executed).toContainEqual({
      'remove-gem': { group: 0, gem: 1 },
    })
    await user.click(
      screen.getByRole('button', { name: 'Add a socket group in Boots' }),
    )
    expect(api.executed).toContainEqual({
      'add-socket-group': { slot: 'Boots', label: null },
    })
    const added = (
      await screen.findByRole('heading', { name: 'Group 3', level: 3 })
    ).closest('section') as HTMLElement
    expect(within(added).getByText('No gems yet.')).toBeTruthy()
    await user.click(within(added).getByText('Group settings'))
    await user.click(
      within(added).getByRole('button', { name: 'Remove group' }),
    )
    expect(api.executed).toContainEqual({
      'remove-socket-group': { group: 2 },
    })
    await waitFor(() =>
      expect(screen.queryByRole('heading', { name: 'Group 3' })).toBeNull(),
    )
  })

  it('switches the whole stage from the header', async () => {
    const { user, api } = await importBuild()
    const stage = screen.getByLabelText('Stage') as HTMLSelectElement
    expect(stage.selectedOptions[0]?.text).toBe('Leveling')
    await user.selectOptions(stage, 'Endgame')
    expect(api.executed).toContainEqual({ 'use-loadout': { spec: 2 } })
    // Items and skills follow the stage.
    await user.click(screen.getByRole('button', { name: 'Equipment' }))
    const inUse = (
      await screen.findByRole('heading', { name: 'In use' })
    ).closest('section') as HTMLElement
    await waitFor(() =>
      expect(within(inUse).getByText('Spare Band')).toBeTruthy(),
    )
    await user.click(screen.getByRole('button', { name: 'Skills' }))
    expect(
      await screen.findByRole('heading', { name: 'Body Armour' }),
    ).toBeTruthy()
    expect(screen.getByLabelText('Ice Nova level')).toHaveProperty(
      'value',
      '20',
    )
  })

  it('shows each slot with its item, sockets, and supports under actives', async () => {
    const { user } = await importBuild()
    await user.click(screen.getByRole('button', { name: 'Skills' }))
    const weapon = (
      await screen.findByRole('heading', { name: 'Weapon 1', level: 2 })
    ).closest('section') as HTMLElement
    expect(within(weapon).getByText('Shiny Wand')).toBeTruthy()
    // The wand's sockets: two linked, one apart.
    expect(
      within(weapon).getByRole('img', { name: '3 sockets, links 2 + 1' }),
    ).toBeTruthy()
    // The support sits under the active it supports.
    const supports = within(weapon).getByRole('list', {
      name: 'Supports for Ice Nova',
    })
    expect(within(supports).getByText('Added Cold Damage')).toBeTruthy()
    // The Boots group has no item in this sample.
    const boots = screen
      .getByRole('heading', { name: 'Boots', level: 2 })
      .closest('section') as HTMLElement
    expect(within(boots).getByText('No item in this slot')).toBeTruthy()
    await expectAccessible()

    // The equipment list draws the same sockets.
    await user.click(screen.getByRole('button', { name: 'Equipment' }))
    expect(
      await screen.findByRole('img', { name: '3 sockets, links 2 + 1' }),
    ).toBeTruthy()
  })

  it('edits gems and the main skill on the Skills screen', async () => {
    const { user, api } = await importBuild()
    await user.click(screen.getByRole('button', { name: 'Skills' }))
    await screen.findByRole('heading', { name: 'Skills', level: 1 })
    await expectAccessible()
    const weapon = screen
      .getByRole('heading', { name: 'Weapon 1' })
      .closest('section') as HTMLElement
    expect(within(weapon).getByText('Main skill group')).toBeTruthy()
    expect(within(weapon).getByText('Main skill')).toBeTruthy()

    const level = within(weapon).getByLabelText('Ice Nova level')
    await user.clear(level)
    await user.type(level, '5{Enter}')
    expect(api.executed).toContainEqual({
      'edit-gem': { group: 0, gem: 0, level: 5 },
    })
    await user.click(within(weapon).getByLabelText('Added Cold Damage enabled'))
    expect(api.executed).toContainEqual({
      'edit-gem': { group: 0, gem: 1, enabled: false },
    })

    await user.click(
      screen.getByRole('button', { name: 'Make Frostblink the main skill' }),
    )
    const movement = screen
      .getByRole('heading', { name: 'Movement' })
      .closest('section') as HTMLElement
    await waitFor(() =>
      expect(within(movement).getByText('Main skill group')).toBeTruthy(),
    )
    expect(api.executed).toContainEqual({
      'edit-socket-group': { group: 1, main: true, 'main-gem': 0 },
    })
    // Undo puts the main skill back.
    await user.click(screen.getByRole('button', { name: 'Undo' }))
    await waitFor(() =>
      expect(within(weapon).getByText('Main skill group')).toBeTruthy(),
    )
  })

  it('shows and edits the configuration and custom modifiers', async () => {
    const { user, api } = await importBuild()
    await user.click(screen.getByRole('button', { name: 'Configuration' }))
    await screen.findByRole('heading', { name: 'Configuration', level: 1 })
    // Inputs by group, named as Path of Building names them, marked by
    // what the calculation does.
    const enemy = screen
      .getByRole('heading', { name: 'Enemy', level: 3 })
      .closest('div') as HTMLElement
    expect(within(enemy).getByText('Is the enemy a Boss?')).toBeTruthy()
    expect(within(enemy).getByText(/enemyLevel · default value/)).toBeTruthy()
    const conditions = screen
      .getByRole('heading', { name: 'Conditions', level: 3 })
      .closest('div') as HTMLElement
    expect(within(conditions).getByText('Changes no number')).toBeTruthy()

    // A choice from PoB's list, a flag, and removing an input.
    await user.selectOptions(
      screen.getByLabelText('Resistance penalty'),
      'Act 5 (-30%)',
    )
    expect(api.executed).toContainEqual({
      'set-config-input': {
        name: 'resistancePenalty',
        value: { number: -30 },
      },
    })
    await user.click(screen.getByLabelText('Are you always on Full Life?'))
    expect(api.executed).toContainEqual({
      'set-config-input': {
        name: 'conditionFullLife',
        value: { boolean: false },
      },
    })
    await user.click(
      screen.getByRole('button', {
        name: 'Remove Are you always on Full Life?',
      }),
    )
    expect(api.executed).toContainEqual({
      'remove-config-input': { name: 'conditionFullLife' },
    })
    await waitFor(() =>
      expect(
        screen.queryByLabelText('Are you always on Full Life?'),
      ).toBeNull(),
    )

    // Any of PoB's options from the catalogue, found by search: a general
    // one at once, one for a skill the build lacks among all options.
    await user.click(screen.getByText('Add an option'))
    const search = screen.getByLabelText('Search options')
    await user.type(search, "avian's might")
    expect(
      screen.queryByRole('button', { name: "Add Is Avian's Might active?" }),
    ).toBeNull()
    await user.click(screen.getByLabelText('All options'))
    expect(
      screen.getByRole('button', { name: "Add Is Avian's Might active?" }),
    ).toBeTruthy()
    await user.clear(search)
    await user.type(search, 'consecrated')
    await user.click(
      screen.getByRole('button', {
        name: 'Add Are you on Consecrated Ground?',
      }),
    )
    expect(api.executed).toContainEqual({
      'set-config-input': {
        name: 'conditionOnConsecratedGround',
        value: { boolean: true },
      },
    })
    expect(
      await screen.findByLabelText('Are you on Consecrated Ground?'),
    ).toBeTruthy()

    // An input by its PoB name; it is not calculated yet.
    await user.click(
      screen.getByText('Add an input by its Path of Building name'),
    )
    await user.type(screen.getByLabelText('Name'), 'conditionOnFungalGround')
    await user.click(screen.getByRole('button', { name: 'Add' }))
    expect(api.executed).toContainEqual({
      'set-config-input': {
        name: 'conditionOnFungalGround',
        value: { boolean: true },
      },
    })
    expect(
      await screen.findByLabelText('Are you on Fungal Ground?'),
    ).toBeTruthy()

    // A custom modifier block: its lines read, a warning marked.
    const block = screen
      .getByRole('heading', { name: 'Block 1', level: 3 })
      .closest('section') as HTMLElement
    expect(within(block).getByText('Note, no effect')).toBeTruthy()
    const text = within(block).getByLabelText('Text of Block 1')
    await user.type(text, '{Enter}+7 to nothing known')
    await user.click(within(block).getByRole('button', { name: 'Apply text' }))
    expect(api.executed).toContainEqual({
      'edit-custom-block': {
        block: 0,
        text: '+15 to maximum Life\nBoss phase two\n+7 to nothing known',
      },
    })
    expect(await within(block).findByText('1 not calculated')).toBeTruthy()
    await user.click(within(block).getByLabelText('Block 1 on'))
    expect(api.executed).toContainEqual({
      'edit-custom-block': { block: 0, enabled: false },
    })
    await expectAccessible(document.body)

    // Undo names the change.
    expect(
      screen.getByRole('button', { name: 'Undo' }).getAttribute('title'),
    ).toMatch(/configuration change/)
  })

  it('shows an item card, lines as the game prints them, and variants (UX-07 to UX-09)', async () => {
    const { user, api } = await importBuild()
    await user.click(screen.getByRole('button', { name: 'Equipment' }))
    await user.click(
      await screen.findByRole('button', { name: 'Edit Loom Test Band' }),
    )
    // The card: class, quality, levels, flags; no file fields among lines.
    const facts = await screen.findByRole('list', { name: 'Item details' })
    for (const f of [
      'Rings',
      'Quality +20%',
      'Item level 84',
      'Requires level 30',
      'Corrupted',
    ]) {
      expect(within(facts).getByText(f)).toBeTruthy()
    }
    await user.click(screen.getByRole('button', { name: 'Back to equipment' }))

    // The spare ring: its variants, the lines without PoB's markup.
    await user.selectOptions(screen.getByLabelText('Item set'), 'Endgame')
    await user.click(
      await screen.findByRole('button', { name: 'Edit Spare Band' }),
    )
    expect(await screen.findByText('+15 to maximum Life')).toBeTruthy()
    expect(screen.queryByText(/\{variant:1\}/)).toBeNull()
    expect(
      screen.getByText('Variant: Pre 3.7.0', { selector: '.badge' }),
    ).toBeTruthy()
    expect(screen.getByText('Crafted')).toBeTruthy()
    // The file fields are folded away, kept as written.
    expect(screen.getByText('File fields (3)')).toBeTruthy()
    const variant = screen.getByLabelText('Item version')
    expect((variant as HTMLSelectElement).value).toBe('2')
    expect(screen.queryByRole('note')).toBeNull()
    await user.selectOptions(variant, 'Pre 3.7.0 (legacy)')
    expect(api.executed).toContainEqual({
      'edit-item': {
        item: 3,
        ops: [{ 'select-variant': { variant: 1 } }],
        expect: [],
      },
    })
    await waitFor(() =>
      expect(
        (screen.getByLabelText('Item version') as HTMLSelectElement).value,
      ).toBe('1'),
    )
    // A legacy version in a challenge league says so, with the way back.
    expect(screen.getByRole('note').textContent).toMatch(/only Standard/)
    await user.click(
      screen.getByRole('button', { name: 'Use current version (Current)' }),
    )
    await waitFor(() =>
      expect(
        (screen.getByLabelText('Item version') as HTMLSelectElement).value,
      ).toBe('2'),
    )
    expect(screen.queryByRole('note')).toBeNull()
    await user.selectOptions(screen.getByLabelText('Item version'), '1')
    // Editing a line shows its PoB text, and locks the variant until done.
    await user.click(screen.getByRole('button', { name: 'Edit line 7' }))
    expect(
      (screen.getByLabelText('Line 7 text') as HTMLInputElement).value,
    ).toBe('{variant:1}{range:0.5}+(10-20) to maximum Life')
    await user.type(screen.getByLabelText('Line 7 text'), ' ')
    expect(
      (screen.getByLabelText('Item version') as HTMLSelectElement).disabled,
    ).toBe(true)
    await expectAccessible()
    await user.click(screen.getByRole('button', { name: /Discard/ }))
    await user.click(screen.getByRole('button', { name: 'Back to equipment' }))

    // The list marks the legacy item; Standard keeps legacy versions.
    const spare = await screen.findByRole('button', { name: 'Edit Spare Band' })
    expect(within(spare).getByText('Legacy')).toBeTruthy()
    await user.selectOptions(screen.getByLabelText('League'), 'standard')
    expect(api.executed).toContainEqual({
      'set-league': { league: 'standard' },
    })
    await waitFor(() =>
      expect(
        within(
          screen.getByRole('button', { name: 'Edit Spare Band' }),
        ).queryByText('Legacy'),
      ).toBeNull(),
    )
    await user.click(screen.getByRole('button', { name: 'Edit Spare Band' }))
    expect((await screen.findByRole('note')).textContent).toMatch(
      /Standard keeps it/,
    )
  })
})

describe('the equipment panel (B-2)', () => {
  it('lays out the active set as the game does, with tooltips and the weapon swap', async () => {
    const { user, api } = await importBuild()
    await user.click(screen.getByRole('button', { name: 'Equipment' }))
    const panel = await screen.findByRole('region', { name: 'Equipped' })
    const ring = within(panel).getByRole('button', {
      name: 'Ring 1: Loom Test Band',
    })
    expect(
      within(panel).getByRole('button', { name: 'Weapon 1: Shiny Wand' }),
    ).toBeTruthy()
    expect(
      within(panel).getByRole('img', { name: 'Helmet: empty' }),
    ).toBeTruthy()

    // Hover: the game's tooltip, values only.
    await user.hover(ring)
    const tip = await screen.findByRole('tooltip')
    expect(within(tip).getByText('Loom Test Band')).toBeTruthy()
    expect(within(tip).getByText('Plain Test Ring')).toBeTruthy()
    expect(within(tip).getByText('+12 to Accuracy')).toBeTruthy()
    expect(within(tip).getByText('Corrupted')).toBeTruthy()
    expect(within(tip).queryByText(/Item Level/)).toBeNull()

    expect(tip.textContent).toContain('Requires Level 30, 10 Dex')
    expect(within(tip).queryByText(/Prefix Modifier/)).toBeNull()

    // Alt held: each roll's range, the item level, and each mod's header
    // over its lines, with its tier; a crafted mod as the game heads it.
    fireEvent.keyDown(window, { key: 'Alt' })
    expect(await within(tip).findByText('+12(10-15) to Accuracy')).toBeTruthy()
    expect(within(tip).getByText(/Item Level/)).toBeTruthy()
    const header = within(tip).getByText(
      'Prefix Modifier \u201cSteady\u201d (Tier: 2) \u2014 Attack',
    )
    expect(header.nextElementSibling?.textContent).toBe(
      '+12(10-15) to Accuracy',
    )
    const crafted = within(tip).getByText(
      'Master Crafted Suffix Modifier \u201cof Craft\u201d',
    )
    expect(crafted.nextElementSibling?.textContent).toBe('+15 to maximum Life')
    fireEvent.keyUp(window, { key: 'Alt' })
    expect(await within(tip).findByText('+12 to Accuracy')).toBeTruthy()
    await user.unhover(ring)
    await waitFor(() => expect(screen.queryByRole('tooltip')).toBeNull())

    // The weapon swap: one undoable command; set II has no weapon here.
    const swap = within(panel).getAllByRole('group', { name: 'Weapon set' })[0]!
    await user.click(within(swap).getByRole('button', { name: 'II' }))
    expect(api.executed).toContainEqual({
      'use-weapon-set': { second: true },
    })
    expect(
      await within(panel).findByRole('img', { name: 'Weapon 1: empty' }),
    ).toBeTruthy()
    await expectAccessible()
  })

  it('marks requirements the character does not meet, as the game does', async () => {
    const { user } = await importBuild()
    await user.click(screen.getByRole('button', { name: 'Equipment' }))
    const panel = await screen.findByRole('region', { name: 'Equipped' })
    // The sample character: 5 Dexterity; the ring needs 10.
    expect(
      await within(panel).findByText(
        'Loom Test Band needs 10 Dexterity (you have 5).',
      ),
    ).toBeTruthy()
    expect(
      within(panel).getByText('Strength 20 · Dexterity 5 · Intelligence 20'),
    ).toBeTruthy()
    await user.hover(
      within(panel).getByRole('button', { name: 'Ring 1: Loom Test Band' }),
    )
    const tip = await screen.findByRole('tooltip')
    const need = within(tip).getByText('10')
    expect(need.className).toBe('tt-unmet')
    await expectAccessible()
  })
})

// B-5 (UX brief note 5; docs/ui screen 9): dashboards the user adds,
// arranges, and resizes, saved as the UI's preferences.
describe('dashboards (B-5)', () => {
  it('switches, customizes, and saves dashboards', async () => {
    const { user, api } = await importBuild()
    const tabs = screen.getByRole('group', { name: 'Dashboards' })
    // The defaults: Overview (shown) and Defence.
    expect(
      within(tabs)
        .getByRole('button', { name: 'Overview' })
        .getAttribute('aria-pressed'),
    ).toBe('true')
    expect(screen.getByRole('region', { name: 'Main skill' })).toBeTruthy()
    await user.click(within(tabs).getByRole('button', { name: 'Defence' }))
    // A stat tile, and the Defence panel below it.
    expect(screen.getByRole('region', { name: 'Life' })).toBeTruthy()
    expect(screen.getByRole('region', { name: 'Defence' })).toBeTruthy()
    expect(JSON.parse(api.prefs ?? '{}').active).toBe('defence')

    // A new dashboard opens in Customize, empty; add a panel and a stat.
    await user.click(screen.getByRole('button', { name: 'Add a dashboard' }))
    const name = screen.getByLabelText('Name')
    await user.clear(name)
    await user.type(name, 'Bossing')
    await user.click(screen.getByRole('button', { name: 'Add Offence' }))
    expect(
      screen
        .getByRole('button', { name: 'Add Offence' })
        .hasAttribute('disabled'),
    ).toBe(true)
    await user.selectOptions(screen.getByLabelText('Stat'), 'life')
    await user.click(screen.getByRole('button', { name: 'Add stat' }))
    const life = screen.getByRole('group', { name: 'Stat: Life: arrange' })
    await user.click(within(life).getByRole('button', { name: 'Move up' }))
    await user.click(within(life).getByRole('button', { name: 'Full width' }))
    await expectAccessible()

    let saved = JSON.parse(api.prefs ?? '{}') as {
      dashboards: {
        name: string
        components: { kind: string; width: string }[]
      }[]
    }
    const bossing = saved.dashboards.find((d) => d.name === 'Bossing')!
    expect(bossing.components.map((c) => [c.kind, c.width])).toEqual([
      ['stat', 'full'],
      ['offence', 'half'],
    ])
    await user.click(within(life).getByRole('button', { name: 'Remove' }))
    await user.click(screen.getByRole('button', { name: 'Done' }))
    expect(within(tabs).getByRole('button', { name: 'Bossing' })).toBeTruthy()

    // Up to four dashboards; then deleting one.
    await user.click(screen.getByRole('button', { name: 'Add a dashboard' }))
    expect(screen.queryByRole('button', { name: 'Add a dashboard' })).toBeNull()
    await user.click(
      screen.getByRole('button', { name: 'Delete this dashboard' }),
    )
    saved = JSON.parse(api.prefs ?? '{}')
    expect(saved.dashboards.map((d) => d.name)).toEqual([
      'Overview',
      'Defence',
      'Bossing',
    ])
  })
})
