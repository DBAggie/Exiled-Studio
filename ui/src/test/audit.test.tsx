// The UX audit's changes (docs/ui/ux-audit.md): hover displays that stay in
// the window, the frame that keeps the header and the side menu in place,
// the dashboards' new components, the comparison with pinned numbers, and
// the charts over time.
import { screen, waitFor, within } from '@testing-library/react'
import { describe, expect, it } from 'vitest'

import { niceTicks } from '../chartMath'
import { defaultPrefs, readPrefs } from '../dashboards'
import {
  GAP,
  MIN_SCALE,
  fitScale,
  placeBeside,
  placeByPoint,
} from '../floating'
import { expectAccessible, renderApp } from './render'

const VIEW = { width: 1280, height: 720 }

/** Whether a placed display is whole in the window. */
function inView(
  p: { left: number; top: number; scale: number },
  box: { width: number; height: number },
) {
  return (
    p.left >= GAP &&
    p.top >= GAP &&
    p.left + box.width * p.scale <= VIEW.width - GAP + 0.001 &&
    p.top + box.height * p.scale <= VIEW.height - GAP + 0.001
  )
}

describe('hover displays stay in the window (A-02)', () => {
  const box = { width: 420, height: 360 }

  it('puts a tooltip beside its item, level with it', () => {
    const anchor = { left: 300, top: 200, right: 344, bottom: 290 }
    const p = placeBeside(anchor, box, VIEW)
    expect(p).toEqual({ left: 344 + GAP, top: 200, scale: 1 })
  })

  it('moves a tooltip up for an item at the bottom of the window (a flask)', () => {
    // The reported case: the flask row at the window's lower edge.
    const anchor = { left: 500, top: 650, right: 544, bottom: 715 }
    const p = placeBeside(anchor, box, VIEW)
    expect(p.top).toBe(VIEW.height - GAP - box.height)
    expect(inView(p, box)).toBe(true)
  })

  it('flips a tooltip to the left of an item at the right edge', () => {
    const anchor = { left: 1200, top: 100, right: 1250, bottom: 150 }
    const p = placeBeside(anchor, box, VIEW)
    expect(p.left).toBe(1200 - GAP - box.width)
    expect(inView(p, box)).toBe(true)
  })

  it('shrinks a tooltip taller than the window instead of cutting it', () => {
    const tall = { width: 420, height: 1000 }
    const anchor = { left: 300, top: 400, right: 344, bottom: 490 }
    const p = placeBeside(anchor, tall, VIEW)
    expect(p.scale).toBeCloseTo((VIEW.height - 2 * GAP) / 1000, 5)
    expect(inView(p, tall)).toBe(true)
    // Never smaller than text can be read at.
    expect(fitScale({ width: 400, height: 5000 }, VIEW)).toBe(MIN_SCALE)
  })

  it('keeps a pointer card in the window in every corner', () => {
    for (const point of [
      { x: 5, y: 5 },
      { x: 1275, y: 5 },
      { x: 5, y: 715 },
      { x: 1275, y: 715 },
      { x: 640, y: 360 },
    ]) {
      expect(inView(placeByPoint(point, box, VIEW), box)).toBe(true)
    }
    // Below and right of the pointer where there is room.
    expect(placeByPoint({ x: 100, y: 100 }, box, VIEW)).toEqual({
      left: 116,
      top: 116,
      scale: 1,
    })
  })
})

describe('chart axes', () => {
  it('uses round values that span the data', () => {
    const ticks = niceTicks(322, 13795, 3)
    expect(ticks[0]).toBeLessThanOrEqual(322)
    expect(ticks[ticks.length - 1]).toBeGreaterThanOrEqual(13795)
    expect(ticks).toEqual([0, 5000, 10000, 15000])
    // A flat line still gets an axis.
    expect(niceTicks(75, 75).length).toBeGreaterThan(1)
  })
})

describe('dashboard preferences', () => {
  it('reads the new kinds and widths and drops what it does not know', () => {
    const prefs = readPrefs(
      JSON.stringify({
        version: 1,
        active: 'a',
        rail: true,
        dashboards: [
          {
            id: 'a',
            name: 'A',
            components: [
              { id: '1', kind: 'curve', vary: 'gem-level', width: 'third' },
              { id: '2', kind: 'progression', fields: ['life'], width: 'full' },
              { id: '3', kind: 'timeline', target: 5e6, width: 'two-thirds' },
              { id: '4', kind: 'from-the-future', width: 'half' },
              { id: '5', kind: 'curve', vary: 'the-moon', width: 'half' },
              { id: '6', kind: 'offence', width: 'enormous' },
            ],
          },
        ],
      }),
    )
    expect(prefs.rail).toBe(true)
    expect(prefs.dashboards[0]!.components.map((c) => c.id)).toEqual([
      '1',
      '2',
      '3',
    ])
    // The defaults read back as themselves.
    expect(readPrefs(JSON.stringify(defaultPrefs()))).toEqual(defaultPrefs())
  })
})

async function openDashboard(demo: boolean) {
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

describe('the workspace frame (A-01)', () => {
  it('collapses the side menu to a rail and remembers it', async () => {
    const { user, api, container } = await openDashboard(false)
    expect(container.querySelector('.app.nav-rail')).toBeNull()
    await user.click(screen.getByRole('button', { name: 'Collapse the menu' }))
    expect(container.querySelector('.app.nav-rail')).not.toBeNull()
    expect(JSON.parse(api.prefs ?? '{}').rail).toBe(true)
    // Every page keeps its name on the rail (for a screen reader and as
    // the button's tooltip).
    const nav = screen.getByRole('navigation', { name: 'Workspace' })
    expect(within(nav).getByRole('button', { name: /^Equipment/ })).toBeTruthy()
    await expectAccessible()
  })
})

describe('comparison and time on the dashboard (section D)', () => {
  it('pins the numbers and shows what changed after a stage switch', async () => {
    const { user } = await openDashboard(true)
    const tabs = screen.getByRole('group', { name: 'Dashboards' })
    await user.click(within(tabs).getByRole('button', { name: 'Trends' }))
    const compare = screen.getByRole('region', { name: 'Compare' })
    expect(
      within(compare).getByText(/Pin the numbers as they are/),
    ).toBeTruthy()
    await user.click(
      within(compare).getByRole('button', { name: 'Pin to compare' }),
    )
    expect(
      within(compare).getByText('Nothing has changed since you pinned.'),
    ).toBeTruthy()
    // The stage changes every number: the comparison lists them, each
    // with its direction in words (never colour alone).
    await user.click(
      within(screen.getByRole('group', { name: 'Stage' })).getByRole('button', {
        name: 'Endgame',
      }),
    )
    await waitFor(() =>
      expect(
        within(screen.getByRole('region', { name: 'Compare' })).getByRole(
          'button',
          { name: /Hit DPS/ },
        ),
      ).toBeTruthy(),
    )
    const row = within(
      screen.getByRole('region', { name: 'Compare' }),
    ).getByRole('button', { name: /Hit DPS/ })
    expect(row.textContent).toMatch(/up/)
    expect(row.textContent).toMatch(/%\)/)
    // The trail charts the stat after each change this session.
    const trend = screen.getByRole('region', {
      name: 'Hit DPS over your changes',
    })
    expect(within(trend).getByRole('group', { name: /Hit DPS/ })).toBeTruthy()
    await user.click(screen.getByRole('button', { name: 'Stop comparing' }))
    expect(
      within(screen.getByRole('region', { name: 'Compare' })).getByRole(
        'button',
        { name: 'Pin to compare' },
      ),
    ).toBeTruthy()
    await expectAccessible()
  })

  it('charts the stages, a scaling curve, and a fight, with their numbers as tables', async () => {
    const { user } = await openDashboard(true)
    const tabs = screen.getByRole('group', { name: 'Dashboards' })
    await user.click(within(tabs).getByRole('button', { name: 'Trends' }))
    const progression = screen.getByRole('region', {
      name: 'Progression by stage',
    })
    await waitFor(() =>
      expect(
        within(progression).getByRole('group', { name: 'Life by stage' }),
      ).toBeTruthy(),
    )
    const curve = await screen.findByRole('region', {
      name: 'Hit DPS by Ice Nova level',
    })
    await waitFor(() =>
      expect(
        within(curve).getByRole('group', { name: 'Hit DPS by Ice Nova level' }),
      ).toBeTruthy(),
    )
    // The arrow keys read a chart's points.
    const plot = within(curve).getByRole('group', {
      name: 'Hit DPS by Ice Nova level',
    })
    plot.focus()
    await user.keyboard('{ArrowRight}')
    expect(within(plot).getByRole('status').textContent).toMatch(/1 · now/)
    const fight = screen.getByRole('region', { name: 'Fight timeline' })
    expect(within(fight).getByText(/to deal 10M/)).toBeTruthy()
    // The detail view: the same numbers as a table.
    await user.click(
      within(progression).getByRole('button', {
        name: 'Progression by stage: details',
      }),
    )
    const dialog = await screen.findByRole('dialog')
    expect(
      within(dialog).getByRole('columnheader', { name: 'Endgame' }),
    ).toBeTruthy()
    expect(
      within(dialog).getByRole('rowheader', { name: 'Hit DPS' }),
    ).toBeTruthy()
    await expectAccessible()
  })

  it('keeps a summary short and opens the rest on request', async () => {
    const { user } = await openDashboard(true)
    const defence = screen.getByRole('region', { name: 'Defence' })
    expect(within(defence).getAllByRole('listitem')).toHaveLength(6)
    await user.click(within(defence).getByRole('button', { name: /^Show all/ }))
    expect(within(defence).getAllByRole('listitem').length).toBeGreaterThan(10)
    // The headline number comes first.
    const offence = screen.getByRole('region', { name: 'Offence' })
    expect(within(offence).getAllByRole('listitem')[0]!.textContent).toMatch(
      /^Hit DPS/,
    )
  })
})
