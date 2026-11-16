// The user's dashboards (UX brief note 5, B-5; docs/ui UI-DESIGN-BRIEF.md
// screens 3 and 9): up to four, each a list of components laid out two to a
// row, each half or full width. Kept in the UI's preferences document
// (`ui_prefs`, beside the application's settings), never in a build.

/** What a component shows. */
export type ComponentKind =
  'offence' | 'defence' | 'warnings' | 'equipment' | 'main-skill' | 'stat'

/** Every kind but `stat`, in the library's order. */
export const PANEL_KINDS: Exclude<ComponentKind, 'stat'>[] = [
  'offence',
  'defence',
  'main-skill',
  'equipment',
  'warnings',
]

export interface DashComponent {
  id: string
  kind: ComponentKind
  /** For `stat`: the calc result field it shows. */
  field?: string
  /** Half (one of two in a row) or full width. */
  width: 'half' | 'full'
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
}

/** The brief's limit. */
export const MAX_DASHBOARDS = 4
/** Enough for any layout; keeps the preferences document small. */
export const MAX_COMPONENTS = 24

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

/** The dashboards a user starts with: the brief's Overview (offence and
 * defence, main skill and equipment, warnings), and Defence. */
export function defaultPrefs(): Prefs {
  return {
    version: 1,
    active: 'overview',
    dashboards: [
      {
        id: 'overview',
        name: 'Overview',
        components: [
          { id: 'c1', kind: 'offence', width: 'half' },
          { id: 'c2', kind: 'defence', width: 'half' },
          { id: 'c3', kind: 'main-skill', width: 'half' },
          { id: 'c4', kind: 'equipment', width: 'half' },
          { id: 'c5', kind: 'warnings', width: 'full' },
        ],
      },
      {
        id: 'defence',
        name: 'Defence',
        components: [
          { id: 'd1', kind: 'stat', field: 'life', width: 'half' },
          { id: 'd2', kind: 'stat', field: 'energy-shield', width: 'half' },
          { id: 'd3', kind: 'defence', width: 'full' },
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
    (c.width === 'half' || c.width === 'full') &&
    (c.kind !== 'stat' || typeof c.field === 'string')
  )
}

/** The preferences in `text`, or the defaults when it is missing or not a
 * document this version reads (a preference, never an error). */
export function readPrefs(text: string | null): Prefs {
  if (text === null) return defaultPrefs()
  try {
    const raw = JSON.parse(text) as Record<string, unknown>
    const list = (raw.dashboards as unknown[] | undefined) ?? []
    const dashboards: DashboardDef[] = list
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
    if (raw.version !== 1 || dashboards.length === 0) return defaultPrefs()
    const active =
      typeof raw.active === 'string' &&
      dashboards.some((d) => d.id === raw.active)
        ? raw.active
        : dashboards[0]!.id
    return { version: 1, dashboards, active }
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
