// The UX audit's follow-up (docs/ui/ux-audit.md, "Follow-up"): dashboards
// a build keeps for itself, snapshots kept by name in the build, the
// comparison with another build file, dragging components, a stage
// compared with another on the tree and the equipment pages, and the
// Skills page as a table.
import { fireEvent, screen, waitFor, within } from '@testing-library/react'
import { describe, expect, it } from 'vitest'

import { dropTarget, dropped } from '../dashboards'
import { nodeDiff, slotDiff } from '../stageDiff'
import { readWorkspace, writeWorkspace } from '../workspace'
import { expectAccessible, renderApp } from './render'

async function openDashboard(demo = true) {
  const view = await renderApp({ data: true, demo })
  await view.user.click(
    screen.getByRole('button', {
      name: 'Import a Path of Building file (.xml)…',
    }),
  )
  await view.user.click(
    await screen.findByRole('button', { name: 'Open the dashboard' }),
  )
  await screen.findByRole('group', { name: 'Dashboards' })
  return view
}

/** The workspace document the build holds now. */
async function stored(api: { snapshot(): Promise<unknown> }) {
  const snapshot = (await api.snapshot()) as {
    build: { workspace?: string | null } | null
  }
  return readWorkspace(snapshot.build?.workspace)
}

describe('the build workspace document', () => {
  it('reads nothing, junk, and another version as the empty workspace', () => {
    const empty = { version: 1, snapshots: [] }
    expect(readWorkspace(null)).toEqual(empty)
    expect(readWorkspace('not json')).toEqual(empty)
    expect(readWorkspace('{"version":2,"snapshots":[]}')).toEqual(empty)
    // The empty workspace is not stored at all.
    expect(writeWorkspace(readWorkspace(null))).toBeNull()
  })

  it('keeps what it knows and drops what it does not', () => {
    const text = JSON.stringify({
      version: 1,
      snapshots: [
        { id: 's1', name: 'Before', fields: { life: 100, mana: null } },
        { id: 's2', name: 'Bad', fields: { life: 'many' } },
      ],
      dashboards: {
        active: 'gone',
        list: [
          {
            id: 'a',
            name: 'Mine',
            components: [
              { id: 'c1', kind: 'offence', width: 'half' },
              { id: 'c2', kind: 'hologram', width: 'half' },
            ],
          },
        ],
      },
    })
    const w = readWorkspace(text)
    expect(w.snapshots.map((s) => s.name)).toEqual(['Before'])
    expect(w.dashboards?.active).toBe('a')
    expect(w.dashboards?.list[0]?.components.map((c) => c.kind)).toEqual([
      'offence',
    ])
    expect(readWorkspace(writeWorkspace(w))).toEqual(w)
  })
})

describe('snapshots kept in the build', () => {
  it('keeps the numbers by name, compares with them, and deletes them', async () => {
    const { user, api } = await openDashboard()
    await user.click(screen.getByRole('button', { name: /^Snapshots/ }))
    expect(screen.getByText('None kept yet.')).toBeTruthy()
    await user.type(screen.getByLabelText('Snapshot name'), 'Leveling gear')
    await user.click(
      screen.getByRole('button', { name: 'Keep current numbers' }),
    )
    await screen.findByRole('button', { name: 'Compare with Leveling gear' })
    // It is in the build (an undoable change), not only on screen.
    expect((await stored(api)).snapshots.map((s) => s.name)).toEqual([
      'Leveling gear',
    ])
    expect(
      api.executed.some((c) => typeof c === 'object' && 'set-workspace' in c),
    ).toBe(true)
    await user.click(
      screen.getByRole('button', { name: 'Compare with Leveling gear' }),
    )
    await user.keyboard('{Escape}')
    const comparison = screen.getByRole('group', { name: 'Comparison' })
    expect(within(comparison).getByText('with Leveling gear')).toBeTruthy()
    // Another stage moves the numbers away from the kept ones.
    await user.selectOptions(screen.getByLabelText('Stage'), '2')
    const offence = await screen.findByRole('region', { name: /^Offence/ })
    await waitFor(() =>
      expect(
        within(offence).getByRole('button', { name: /Hit DPS/ }).textContent,
      ).toMatch(/up|down/),
    )
    // Keeping a snapshot is not a step of the trail of numbers.
    await user.click(screen.getByRole('button', { name: /^Snapshots/ }))
    await user.click(
      screen.getByRole('button', { name: 'Delete snapshot Leveling gear' }),
    )
    await waitFor(async () => expect((await stored(api)).snapshots).toEqual([]))
    expect(screen.queryByRole('group', { name: 'Comparison' })).toBeNull()
    await expectAccessible()
  })

  it('compares with another build file without opening it', async () => {
    const { user, api } = await openDashboard()
    const before = (await api.snapshot()).version
    await user.click(screen.getByRole('button', { name: /^Snapshots/ }))
    await user.click(
      screen.getByRole('button', { name: 'Compare with another build file…' }),
    )
    await user.keyboard('{Escape}')
    const comparison = await screen.findByRole('group', { name: 'Comparison' })
    expect(within(comparison).getByText('with Other build')).toBeTruthy()
    expect((await api.snapshot()).version).toBe(before)
    const offence = screen.getByRole('region', { name: /^Offence/ })
    expect(
      within(offence).getByRole('button', { name: /Hit DPS/ }).textContent,
    ).toMatch(/up|down/)
  })
})

describe('dashboards a build keeps for itself', () => {
  it('copies the user dashboards into the build and edits them there', async () => {
    const { user, api } = await openDashboard()
    await user.click(screen.getByRole('button', { name: 'Customize' }))
    const scope = screen.getByRole('group', {
      name: 'These dashboards are for',
    })
    expect(
      within(scope)
        .getByRole('button', { name: 'Every build' })
        .getAttribute('aria-pressed'),
    ).toBe('true')
    await user.click(within(scope).getByRole('button', { name: 'This build' }))
    await waitFor(async () =>
      expect((await stored(api)).dashboards?.list.map((d) => d.name)).toEqual([
        'Overview',
        'Defence',
        'Trends',
      ]),
    )
    // A change now goes to the build, and the user's own stay as they are.
    const prefsBefore = api.prefs
    const name = screen.getByLabelText('Name')
    await user.clear(name)
    await user.type(name, 'Boss check{Enter}')
    await waitFor(async () =>
      expect((await stored(api)).dashboards?.list[0]?.name).toBe('Boss check'),
    )
    expect(api.prefs).toBe(prefsBefore)
    // Changing the tab shown is not a change to the build.
    const version = (await api.snapshot()).version
    const tabs = screen.getByRole('group', { name: 'Dashboards' })
    await user.click(within(tabs).getByRole('button', { name: 'Defence' }))
    expect(
      within(tabs)
        .getByRole('button', { name: 'Defence' })
        .getAttribute('aria-pressed'),
    ).toBe('true')
    expect((await api.snapshot()).version).toBe(version)
    // Back to the user's: the build no longer keeps a set.
    await user.click(within(scope).getByRole('button', { name: 'Every build' }))
    await waitFor(async () =>
      expect((await stored(api)).dashboards).toBeUndefined(),
    )
    expect(
      within(screen.getByRole('group', { name: 'Dashboards' })).getByRole(
        'button',
        { name: 'Overview' },
      ),
    ).toBeTruthy()
  })
})

describe('arranging components by dragging', () => {
  const cells = [
    { id: 'a', left: 0, top: 0, right: 100, bottom: 50 },
    { id: 'b', left: 120, top: 0, right: 220, bottom: 50 },
    { id: 'c', left: 0, top: 70, right: 220, bottom: 120 },
  ]

  it('finds the cell and the side under the pointer', () => {
    expect(dropTarget(cells, 20, 10)).toEqual({ id: 'a', after: false })
    expect(dropTarget(cells, 90, 10)).toEqual({ id: 'a', after: true })
    // In the gap between two cells: the nearer one.
    expect(dropTarget(cells, 112, 10)).toEqual({ id: 'b', after: false })
    expect(dropTarget(cells, 200, 100)).toEqual({ id: 'c', after: true })
    // Between rows, or outside the grid: nowhere.
    expect(dropTarget(cells, 50, 60)).toBeNull()
    expect(dropTarget(cells, 50, 500)).toBeNull()
  })

  it('puts the dragged item before or after the target', () => {
    const list = ['a', 'b', 'c', 'd'].map((id) => ({ id }))
    const ids = (l: { id: string }[]) => l.map((x) => x.id).join('')
    expect(ids(dropped(list, 'a', 'c', true))).toBe('bcad')
    expect(ids(dropped(list, 'd', 'a', false))).toBe('dabc')
    // Dropping where it already is changes nothing.
    expect(dropped(list, 'b', 'a', true)).toBe(list)
    expect(dropped(list, 'b', 'b', false)).toBe(list)
    expect(dropped(list, 'x', 'a', false)).toBe(list)
  })

  it('moves a component with the pointer and with the arrow keys', async () => {
    const { user, api, container } = await openDashboard()
    await user.click(screen.getByRole('button', { name: 'Customize' }))
    const order = () =>
      [...container.querySelectorAll('[data-cell]')].map((el) =>
        el.getAttribute('data-cell'),
      )
    const start = order()
    // jsdom lays nothing out: give the cells places, one row.
    container.querySelectorAll<HTMLElement>('[data-cell]').forEach((el, i) => {
      el.getBoundingClientRect = () =>
        ({
          left: i * 100,
          right: i * 100 + 90,
          top: 0,
          bottom: 50,
        }) as DOMRect
    })
    const handle = screen.getByRole('button', { name: /^Move .*Hit DPS/ })
    fireEvent.pointerDown(handle, { button: 0, pointerId: 1 })
    fireEvent.pointerMove(handle, { clientX: 280, clientY: 10, pointerId: 1 })
    expect(container.querySelector('.dash-cell.drop-after')).not.toBeNull()
    fireEvent.pointerUp(handle, { pointerId: 1 })
    await waitFor(() =>
      expect(order()).toEqual([
        start[1],
        start[2],
        start[0],
        ...start.slice(3),
      ]),
    )
    expect(JSON.parse(api.prefs ?? '{}').dashboards[0].components[2].id).toBe(
      start[0],
    )
    // The keyboard does the same from the handle.
    screen.getByRole('button', { name: /^Move .*Hit DPS/ }).focus()
    await user.keyboard('{ArrowLeft}')
    await waitFor(() =>
      expect(order()).toEqual([
        start[1],
        start[0],
        start[2],
        ...start.slice(3),
      ]),
    )
    await expectAccessible()
  })
})

describe('a stage compared with another', () => {
  it('finds the nodes and the slots that differ', () => {
    expect(nodeDiff([1, 2, 3], [2, 3, 4, 5])).toEqual({
      added: [4, 5],
      removed: [1],
    })
    expect(
      slotDiff(
        [
          { slot: 'Ring 1', item: 1 },
          { slot: 'Weapon 1', item: 2 },
          { slot: 'Belt', item: 9 },
        ],
        [
          { slot: 'Ring 1', item: 3 },
          { slot: 'Weapon 1', item: 2 },
          { slot: 'Helmet', item: 7 },
        ],
      ),
    ).toEqual({
      rows: [
        { slot: 'Ring 1', here: 1, other: 3 },
        { slot: 'Belt', here: 9, other: null },
        { slot: 'Helmet', here: null, other: 7 },
      ],
      same: 1,
    })
  })

  it('shows the equipment and the tree against another stage, changing nothing', async () => {
    const { user, api } = await openDashboard()
    const version = (await api.snapshot()).version
    await user.click(screen.getByRole('button', { name: /^Equipment/ }))
    await user.selectOptions(
      await screen.findByLabelText('Compare with stage'),
      'Endgame',
    )
    const diff = await screen.findByRole('region', {
      name: 'Equipment against Endgame',
    })
    const row = await within(diff).findByRole('row', { name: /Ring 1/ })
    expect(within(row).getAllByRole('button').length).toBe(2)
    expect(
      within(diff).getByText('1 other slot holds the same item.'),
    ).toBeTruthy()
    // The same choice on the tree page.
    await user.click(screen.getByRole('button', { name: /^Passive tree/ }))
    expect(
      await screen.findByText(
        /Nodes only in Endgame: \d+\. Nodes only in this stage: \d+\./,
      ),
    ).toBeTruthy()
    expect(screen.getByText('Only in Endgame')).toBeTruthy()
    expect((await api.snapshot()).version).toBe(version)
    await expectAccessible()
  })
})

describe('the Skills page as a table', () => {
  it('lists a socket group a row and opens its editor', async () => {
    const { user, api } = await openDashboard()
    await user.click(screen.getByRole('button', { name: /^Skills/ }))
    const view = await screen.findByRole('group', { name: 'View' })
    await user.click(within(view).getByRole('button', { name: 'Table' }))
    const table = await screen.findByRole('table', { name: 'Socket groups' })
    expect(within(table).getAllByRole('row').length).toBeGreaterThan(1)
    expect(within(table).getByText('Ice Nova')).toBeTruthy()
    expect(JSON.parse(api.prefs ?? '{}').skillsTable).toBe(true)
    const open = within(table).getAllByRole('button', { name: /^Edit / })[0]!
    await user.click(open)
    expect(within(table).getByRole('button', { name: 'Add gem' })).toBeTruthy()
    await expectAccessible()
    await user.click(within(view).getByRole('button', { name: 'Cards' }))
    expect(screen.queryByRole('table', { name: 'Socket groups' })).toBeNull()
    expect(JSON.parse(api.prefs ?? '{}').skillsTable).toBeUndefined()
  })
})
