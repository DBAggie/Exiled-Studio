// The user's dashboards (UX brief note 5, B-5; docs/ui UI-DESIGN-BRIEF.md
// screens 3 and 9; UX audit 2026-11-15, section D): up to eight, each a
// list of components on a twelve-column grid, each a quarter, a third, a
// half, two thirds, or the full width. Kept in the UI's preferences document
// (`ui_prefs`, beside the application's settings); a build may keep its
// own set in its file (`../buildDashboards`): the layout only.
// An application older than a kind or a width drops that component when
// it reads the document (a preference, never an error).

/** What a component shows. */
export type ComponentKind =
  // Summaries.
  | 'offence'
  | 'defence'
  | 'warnings'
  | 'equipment'
  | 'main-skill'
  | 'stat'
  | 'attributes'
  | 'resistances'
  | 'pools'
  | 'all-stats'
  // Comparison and time (UX audit, section D).
  | 'compare'
  | 'history'
  | 'progression'
  | 'curve'
  | 'timeline'

/** A group of the component library. */
export type Category = 'summary' | 'defence' | 'charts' | 'build'
export const CATEGORIES: Category[] = ['summary', 'defence', 'charts', 'build']

/** What a scaling curve varies (the application's what-if query). */
export type CurveVary = 'gem-level' | 'character-level' | 'config-number'
export const CURVE_VARIES: CurveVary[] = [
  'gem-level',
  'character-level',
  'config-number',
]

/** A component's width on the dashboard's twelve-column grid. A quarter
 * (the revamp's lead row of four figures, 2026-11-25) is the narrowest;
 * an application older than it drops a component of that width, as it
 * does any width it does not know. */
export type Width = 'quarter' | 'third' | 'half' | 'two-thirds' | 'full'
export const WIDTHS: Width[] = [
  'quarter',
  'third',
  'half',
  'two-thirds',
  'full',
]

/** The library: every kind but `stat`, in order, with its group and the
 * width it starts with. */
export const CATALOG: {
  kind: Exclude<ComponentKind, 'stat'>
  category: Category
  width: Width
  /** May be added more than once (each charts its own stat). */
  many?: boolean
}[] = [
  { kind: 'offence', category: 'summary', width: 'half' },
  { kind: 'defence', category: 'summary', width: 'half' },
  { kind: 'all-stats', category: 'summary', width: 'full' },
  { kind: 'attributes', category: 'summary', width: 'half' },
  { kind: 'pools', category: 'defence', width: 'half' },
  { kind: 'resistances', category: 'defence', width: 'half' },
  { kind: 'compare', category: 'charts', width: 'half' },
  { kind: 'history', category: 'charts', width: 'half', many: true },
  { kind: 'progression', category: 'charts', width: 'full', many: true },
  { kind: 'curve', category: 'charts', width: 'half', many: true },
  { kind: 'timeline', category: 'charts', width: 'half' },
  { kind: 'main-skill', category: 'build', width: 'half' },
  { kind: 'equipment', category: 'build', width: 'half' },
  { kind: 'warnings', category: 'build', width: 'full' },
]

/** Every kind but `stat`, in the library's order. */
export const PANEL_KINDS = CATALOG.map((c) => c.kind)

export interface DashComponent {
  id: string
  kind: ComponentKind
  /** For `stat`, `history`, `curve`: the calc result field it shows. */
  field?: string
  /** For `progression`: the fields charted, one small chart each. */
  fields?: string[]
  /** For `curve`: what is varied. */
  vary?: CurveVary
  /** For `curve` over a config input: the input's name. */
  input?: string
  /** For `timeline`: the life to deal damage to. */
  target?: number
  width: Width
  /** A row break before it: it starts a new row, whatever room the row
   * before it has left. */
  break?: boolean
  /** Two rows tall: the cards after it stack beside it, one under
   * another, when the row has room left (RV-03). */
  tall?: boolean
}

export interface DashboardDef {
  id: string
  name: string
  components: DashComponent[]
}

export interface Prefs {
  version: 1
  dashboards: DashboardDef[]
  /** The dashboard shown. */
  active: string
  /** The side menu as a narrow rail of icons. */
  rail?: boolean
  /** The Skills page as a table, a row a socket group. */
  skillsTable?: boolean
}

/** Enough for any build's views; the tabs still fit one row. */
export const MAX_DASHBOARDS = 8
/** Enough for any layout; keeps the preferences document small. */
export const MAX_COMPONENTS = 24
/** A progression chart's small charts. */
export const MAX_FIELDS = 6

let counter = 0
/** A fresh ID, unique within the preferences. */
export function newId(prefix: string, taken: Iterable<string>): string {
  const used = new Set(taken)
  let id: string
  do {
    counter += 1
    id = `${prefix}${counter}`
  } while (used.has(id))
  return id
}

/** The dashboards a user starts with: a short Overview (headline numbers
 * first, detail one click away), Defence, and Trends (comparison and
 * time). */
export function defaultPrefs(): Prefs {
  return {
    version: 1,
    active: 'overview',
    dashboards: [
      {
        id: 'overview',
        name: 'Overview',
        components: [
          // A lead row of four figures, then a wide table beside a narrow
          // list: widths that differ, so the page has a first thing to
          // read (the revamp's dashboard layout, 2026-11-25).
          { id: 'c6', kind: 'stat', field: 'hit-dps', width: 'quarter' },
          { id: 'c7', kind: 'stat', field: 'life', width: 'quarter' },
          { id: 'c8', kind: 'stat', field: 'energy-shield', width: 'quarter' },
          { id: 'c9', kind: 'stat', field: 'mana', width: 'quarter' },
          // The tall Defence table with the main skill over the lines
          // to review beside it (RV-03).
          { id: 'c2', kind: 'defence', width: 'two-thirds', tall: true },
          { id: 'c3', kind: 'main-skill', width: 'third' },
          { id: 'c5', kind: 'warnings', width: 'third' },
          { id: 'c1', kind: 'offence', width: 'half' },
          { id: 'c4', kind: 'equipment', width: 'half' },
        ],
      },
      {
        id: 'defence',
        name: 'Defence',
        components: [
          { id: 'd1', kind: 'stat', field: 'life', width: 'half' },
          { id: 'd2', kind: 'stat', field: 'energy-shield', width: 'half' },
          { id: 'd4', kind: 'pools', width: 'half' },
          { id: 'd5', kind: 'resistances', width: 'half' },
          { id: 'd3', kind: 'defence', width: 'full' },
        ],
      },
      {
        id: 'trends',
        name: 'Trends',
        components: [
          { id: 't1', kind: 'compare', width: 'half' },
          { id: 't2', kind: 'history', field: 'hit-dps', width: 'half' },
          {
            id: 't3',
            kind: 'progression',
            fields: ['hit-dps', 'life', 'energy-shield'],
            width: 'full',
          },
          {
            id: 't4',
            kind: 'curve',
            vary: 'gem-level',
            field: 'hit-dps',
            width: 'half',
          },
          { id: 't5', kind: 'timeline', width: 'half' },
        ],
      },
    ],
  }
}

const KINDS = new Set<string>([...PANEL_KINDS, 'stat'])

function isComponent(x: unknown): x is DashComponent {
  if (typeof x !== 'object' || x === null) return false
  const c = x as Record<string, unknown>
  return (
    typeof c.id === 'string' &&
    typeof c.kind === 'string' &&
    KINDS.has(c.kind) &&
    typeof c.width === 'string' &&
    (WIDTHS as string[]).includes(c.width) &&
    (c.kind !== 'stat' || typeof c.field === 'string') &&
    (c.field === undefined || typeof c.field === 'string') &&
    (c.fields === undefined ||
      (Array.isArray(c.fields) &&
        c.fields.length <= MAX_FIELDS &&
        c.fields.every((f) => typeof f === 'string'))) &&
    (c.vary === undefined ||
      (typeof c.vary === 'string' &&
        (CURVE_VARIES as string[]).includes(c.vary))) &&
    (c.input === undefined || typeof c.input === 'string') &&
    (c.target === undefined ||
      (typeof c.target === 'number' && Number.isFinite(c.target))) &&
    (c.break === undefined || typeof c.break === 'boolean') &&
    (c.tall === undefined || typeof c.tall === 'boolean')
  )
}

/** The dashboards in `list` that this version reads (the others, and
 * components of a kind or width it does not know, are dropped). */
export function readDashboards(list: unknown): DashboardDef[] {
  return (Array.isArray(list) ? (list as unknown[]) : [])
    .filter(
      (d): d is Record<string, unknown> =>
        typeof d === 'object' &&
        d !== null &&
        typeof (d as Record<string, unknown>).id === 'string' &&
        typeof (d as Record<string, unknown>).name === 'string' &&
        Array.isArray((d as Record<string, unknown>).components),
    )
    .slice(0, MAX_DASHBOARDS)
    .map((d) => ({
      id: d.id as string,
      name: d.name as string,
      components: (d.components as unknown[])
        .filter(isComponent)
        .slice(0, MAX_COMPONENTS),
    }))
}

/** The preferences in `text`, or the defaults when it is missing or not a
 * document this version reads (a preference, never an error). */
export function readPrefs(text: string | null): Prefs {
  if (text === null) return defaultPrefs()
  try {
    const raw = JSON.parse(text) as Record<string, unknown>
    const dashboards = readDashboards(raw.dashboards)
    if (raw.version !== 1 || dashboards.length === 0) return defaultPrefs()
    const active =
      typeof raw.active === 'string' &&
      dashboards.some((d) => d.id === raw.active)
        ? raw.active
        : dashboards[0]!.id
    return {
      version: 1,
      dashboards,
      active,
      ...(raw.rail === true ? { rail: true } : {}),
      ...(raw.skillsTable === true ? { skillsTable: true } : {}),
    }
  } catch {
    return defaultPrefs()
  }
}

/** `list` with the item at `from` moved by `by` places (clamped). */
export function moved<T>(list: T[], from: number, by: number): T[] {
  const to = Math.max(0, Math.min(list.length - 1, from + by))
  if (to === from) return list
  const out = [...list]
  const [x] = out.splice(from, 1)
  out.splice(to, 0, x!)
  return out
}

/** The next width up, wrapping to the smallest (the resize control). */
export function nextWidth(w: Width): Width {
  return WIDTHS[(WIDTHS.indexOf(w) + 1) % WIDTHS.length]!
}

/** `list` with the item `id` taken out and put before or after the item
 * `target` (a drag's drop); `list` itself when that changes nothing. */
export function dropped<T extends { id: string }>(
  list: T[],
  id: string,
  target: string,
  after: boolean,
): T[] {
  const item = list.find((x) => x.id === id)
  if (item === undefined || id === target) return list
  const rest = list.filter((x) => x.id !== id)
  const at = rest.findIndex((x) => x.id === target)
  if (at < 0) return list
  rest.splice(after ? at + 1 : at, 0, item)
  return rest.every((x, i) => x === list[i]) ? list : rest
}

/** A cell's place on screen, for a drag. */
export interface CellRect {
  id: string
  left: number
  top: number
  right: number
  bottom: number
}

/** Where a drag at (`x`, `y`) would drop: the cell under the pointer (the
 * nearest in its row when the pointer is in a gap) and the side of its
 * middle the pointer is on. Between two rows or under the last, the end
 * of the row above (the card goes under it); above the first row, its
 * start. `null` only without cells. */
export function dropTarget(
  cells: CellRect[],
  x: number,
  y: number,
): { id: string; after: boolean } | null {
  if (cells.length === 0) return null
  const distance = (c: CellRect) =>
    x < c.left ? c.left - x : x > c.right ? x - c.right : 0
  const row = cells.filter((c) => y >= c.top && y <= c.bottom)
  if (row.length > 0) {
    const cell = row.reduce((a, b) => (distance(b) < distance(a) ? b : a))
    return { id: cell.id, after: x > (cell.left + cell.right) / 2 }
  }
  // The row that ends nearest above the pointer.
  const above = cells.filter((c) => c.bottom < y)
  if (above.length === 0) {
    const first = cells.reduce((a, b) =>
      b.top < a.top || (b.top === a.top && b.left < a.left) ? b : a,
    )
    return { id: first.id, after: false }
  }
  const bottom = Math.max(...above.map((c) => c.bottom))
  const last = above
    .filter((c) => c.bottom === bottom)
    .reduce((a, b) => (b.right > a.right ? b : a))
  return { id: last.id, after: true }
}

/** The width one step wider (`by` 1) or narrower (-1), stopping at the
 * ends (Shift with the arrow keys). */
export function stepWidth(w: Width, by: number): Width {
  const i = WIDTHS.indexOf(w)
  return WIDTHS[Math.max(0, Math.min(WIDTHS.length - 1, i + by))]!
}
