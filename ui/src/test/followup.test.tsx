// The UX audit's follow-up (docs/ui/ux-audit.md, "Follow-up"): dashboards
// a build keeps for itself (layout only, a typed section of the build),
// snapshots kept by name on this computer (never in the build), the
// comparison with another build file, dragging components, a stage
// compared with another on the tree and the equipment pages, and the
// Skills page as a table.
import { fireEvent, screen, waitFor, within } from '@testing-library/react'
import { describe, expect, it } from 'vitest'

import { dropTarget, dropped } from '../dashboards'
import { nodeDiff, slotDiff } from '../stageDiff'
import { readBuildDashboards, writeBuildDashboards } from '../buildDashboards'
import { keptKey, readKept, readKeptList, writeKept } from '../kept'
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

/** The dashboards the build holds now. */
async function stored(api: { snapshot(): Promise<unknown> }) {
  const snapshot = (await api.snapshot()) as {
    build: { dashboards?: unknown } | null
  }
  return readBuildDashboards(snapshot.build?.dashboards)
}

describe("the build's dashboards section", () => {
  it('reads none, junk, and an empty set as no dashboards', () => {
    expect(readBuildDashboards(null)).toBeUndefined()
    expect(readBuildDashboards('text')).toBeUndefined()
    expect(readBuildDashboards({ active: 'a', list: [] })).toBeUndefined()
  })

  it('keeps what it knows, drops what it does not, and writes layout only', () => {
    const own = readBuildDashboards({
      active: 'gone',
      list: [
        {
          id: 'a',
          name: 'Mine',
          components: [
            { id: 'c1', kind: 'offence', width: 'half' },
            { id: 'c2', kind: 'hologram', width: 'half' },
            { id: 'c3', kind: 'timeline', width: 'half', target: 1234.6 },
            {
              id: 'c4',
              kind: 'stat',
              width: 'third',
              field: 'life',
              break: true,
            },
          ],
        },
      ],
    })
    expect(own?.active).toBe('a')
    // A row break is kept.
    expect(own?.list[0]?.components[2]?.break).toBe(true)
    expect(own?.list[0]?.components.map((c) => c.kind)).toEqual([
      'offence',
      'timeline',
      'stat',
    ])
    const written = writeBuildDashboards(own!)
    // Only what a kind uses, a whole number for a target, and no number
    // of the build.
    expect(written.list[0]!.components).toEqual([
      { id: 'c1', kind: 'offence', width: 'half' },
      { id: 'c3', kind: 'timeline', width: 'half', target: 1235 },
      {
        id: 'c4',
        kind: 'stat',
        width: 'third',
        field: 'life',
        break: true,
      },
    ])
    expect(readBuildDashboards(written)?.list[0]?.name).toBe('Mine')
  })
})

describe('snapshots kept on this computer', () => {
  it('keeps them under the build file, the most recent builds only', () => {
    window.localStorage.clear()
    expect(keptKey({ path: 'a.json', origin: 'a.xml' })).toBe('a.json')
    expect(keptKey({ path: null, origin: 'a.xml' })).toBe('a.xml')
    expect(keptKey({})).toBeNull()
    const one = [
      { id: 's1', name: 'Before', fields: { life: 100, mana: null } },
    ]
    writeKept('a.json', one)
    expect(readKept('a.json')).toEqual(one)
    expect(readKept('b.json')).toEqual([])
    for (let i = 0; i < 20; i += 1) writeKept(`other${i}.json`, one)
    expect(readKept('a.json')).toEqual([])
    writeKept('other0.json', [])
    expect(readKept('other0.json')).toEqual([])
    // What is not a snapshot is dropped; junk in storage is nothing kept.
    expect(
      readKeptList([
        ...one,
        { id: 's2', name: 'Bad', fields: { life: 'many' } },
      ]),
    ).toEqual(one)
    window.localStorage.setItem('loom.kept.v1', 'not json')
    expect(readKept('other1.json')).toEqual([])
    window.localStorage.clear()
  })
})

describe('snapshots of the numbers', () => {
  it('keeps the numbers by name, compares with them, and deletes them', async () => {
    window.localStorage.clear()
    const { user, api } = await openDashboard()
    const version = (await api.snapshot()).version
    await user.click(screen.getByRole('button', { name: /^Snapshots/ }))
    expect(screen.getByText('None kept yet.')).toBeTruthy()
    await user.type(screen.getByLabelText('Snapshot name'), 'Leveling gear')
    await user.click(
      screen.getByRole('button', { name: 'Keep current numbers' }),
    )
    await screen.findByRole('button', { name: 'Compare with Leveling gear' })
    // It is kept on this computer under the build's file, and is no
    // change to the build.
    const key = keptKey(await api.snapshot())
    expect(key).not.toBeNull()
    expect(readKept(key!).map((s) => s.name)).toEqual(['Leveling gear'])
    expect((await api.snapshot()).version).toBe(version)
    expect(JSON.stringify(await api.snapshot())).not.toContain('Leveling gear')
    await user.click(
      screen.getByRole('button', { name: 'Compare with Leveling gear' }),
    )
    await user.keyboard('{Escape}')
    const comparison = screen.getByRole('group', { name: 'Comparison' })
    expect(within(comparison).getByText('with Leveling gear')).toBeTruthy()
    // Another stage moves the numbers away from the kept ones.
    await user.click(
      within(screen.getByRole('group', { name: 'Stage' })).getByRole('button', {
        name: 'Endgame',
      }),
    )
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
    await waitFor(() => expect(readKept(key!)).toEqual([]))
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
      expect((await stored(api))?.list.map((d) => d.name)).toEqual([
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
      expect((await stored(api))?.list[0]?.name).toBe('Boss check'),
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
    await waitFor(async () => expect(await stored(api)).toBeUndefined())
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
    // Between rows, or under the last: after the row above (the card
    // goes under it). Above the first row: at the start.
    expect(dropTarget(cells, 50, 60)).toEqual({ id: 'b', after: true })
    expect(dropTarget(cells, 50, 500)).toEqual({ id: 'c', after: true })
    expect(dropTarget(cells, 150, -20)).toEqual({ id: 'a', after: false })
    expect(dropTarget([], 0, 0)).toBeNull()
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
    // The whole bar is the handle; Shift with the arrows resizes.
    const cell = container.querySelector(`[data-cell="${start[0]}"]`)!
    const width = () =>
      /dash-(quarter|third|half|two-thirds|full)/.exec(cell.className)?.[1]
    // A figure starts a quarter wide (four lead the page), the narrowest.
    expect(width()).toBe('quarter')
    await user.keyboard('{Shift>}{ArrowRight}{/Shift}')
    await waitFor(() => expect(width()).toBe('third'))
    await user.keyboard('{Shift>}{ArrowLeft}{ArrowLeft}{/Shift}')
    await waitFor(() => expect(width()).toBe('quarter'))
    // The card's menu starts a new row with it: the room beside the card
    // before it stays empty. Kept with the layout.
    const newRow = async () => {
      screen.getByRole('button', { name: /Hit DPS: card menu$/ }).focus()
      await user.keyboard('{Enter}')
      const item = await screen.findByRole('menuitemcheckbox', {
        name: 'Start a new row',
      })
      item.focus()
      await user.keyboard('{Enter}')
      await waitFor(() => expect(screen.queryByRole('menu')).toBeNull())
    }
    const kept = () =>
      (
        JSON.parse(api.prefs ?? '{}') as {
          dashboards: { components: { id: string; break?: boolean }[] }[]
        }
      ).dashboards[0]!.components.find((c) => c.id === start[0])?.break
    await newRow()
    await waitFor(() => expect(cell.className).toContain('dash-new-row'))
    expect(kept()).toBe(true)
    await newRow()
    await waitFor(() => expect(cell.className).not.toContain('dash-new-row'))
    expect(kept()).toBeUndefined()
    const bar = screen.getByRole('group', { name: /Hit DPS: arrange$/ })
    fireEvent.pointerDown(bar, { button: 0, pointerId: 2 })
    fireEvent.pointerMove(bar, { clientX: 280, clientY: 400, pointerId: 2 })
    expect(container.querySelector('.dash-cell.drop-after')).not.toBeNull()
    fireEvent.pointerCancel(bar, { pointerId: 2 })
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
    await user.click(within(view).getByRole('button', { name: 'Groups' }))
    expect(screen.queryByRole('table', { name: 'Socket groups' })).toBeNull()
    expect(JSON.parse(api.prefs ?? '{}').skillsTable).toBeUndefined()
  })
})
