// An in-memory LoomApi for tests and for running the UI in a browser
// without the shell. It models the application's behaviour the UI relies
// on: versions, background calculation events, undo and redo, stale
// edits, warnings, change requests. Numbers are sample data.
import {
  ApiFailure,
  type ApiEvent,
  type CalcResultDocumentV1,
  type ChangeRequest,
  type Command,
  type ConfigView,
  type DropNotice,
  type GemCatalogEntry,
  type HistoryEntry,
  type ItemEdit,
  type ItemEditPreview,
  type GemView,
  type ItemCard,
  type ItemView,
  type LineView,
  type LoomApi,
  type McpPermissions,
  type McpStatus,
  type Outcome,
  type RecentEntry,
  type RecoveryEntry,
  type SkillGroupView,
  type Snapshot,
  type TreeLayout,
  type TreeSprites,
  type TreeChange,
  type TreePreview,
} from './types'
import * as fakeTree from './fakeTree'

export interface FakeOptions {
  /** Whether game data is loaded. */
  data?: boolean
  /** A code every import fails with. */
  importFails?: string
  /** A code every calculation fails with. */
  calcFails?: string
  recent?: RecentEntry[]
  recovery?: RecoveryEntry[]
  /** Installed bundles by Steam build (B-1); the first is loaded. */
  installed?: number[]
  /** The Steam build the build was made with (its pinned data). */
  pinned?: number
  /** Why the passive tree cannot be edited (its version or class). */
  treeLocked?: string
}

function line(
  index: number,
  text: string,
  extra: Partial<LineView> = {},
): LineView {
  const view: LineView = {
    index,
    text,
    kind: 'modifier',
    section: 'explicit',
    status: 'matched',
    code: null,
    stats: [],
    typed: null,
    adapted: null,
    numbers: [...text.matchAll(/\d+(\.\d+)?/g)].map((m) => ({
      start: m.index,
      end: m.index + m[0].length,
      text: m[0],
    })),
    variants: [],
    marks: [],
    ranged: null,
    ...extra,
  }
  // A modifier shows without PoB's markup (the app also applies the roll).
  if (!('shown' in extra)) {
    view.shown =
      view.kind === 'modifier' ? text.replace(/\{[^}]*\}/g, '') : null
  }
  return view
}

function card(extra: Partial<ItemCard> = {}): ItemCard {
  return {
    'item-class': null,
    quality: null,
    'item-level': null,
    'level-req': null,
    sockets: [],
    properties: [],
    influences: [],
    flags: [],
    variants: [],
    'selected-variant': null,
    'variant-eras': [],
    'current-variant': null,
    affixes: [],
    requirements: null,
    ...extra,
  }
}

function lifeLine(index: number, text: string): LineView {
  const m = /^\+(\d+) to maximum Life$/.exec(text)
  if (m === null) {
    return line(index, text, {
      status: 'warning',
      code: 'LINE_NO_TEMPLATE',
    })
  }
  return line(index, text, {
    stats: [{ id: 'base_maximum_life', value: m[1] }],
  })
}

function sampleItems(): ItemView[] {
  return [
    {
      id: 1,
      name: 'Loom Test Band',
      base: 'Plain Test Ring',
      rarity: 'rare',
      slot: 'Ring 1',
      card: card({
        'item-class': 'Rings',
        quality: '20',
        'item-level': '84',
        'level-req': '30',
        flags: ['Corrupted'],
        affixes: [
          {
            kind: 'prefix',
            name: 'Steady',
            tier: 2,
            crafted: false,
            inferred: false,
            tags: ['Attack'],
            lines: [4],
          },
          {
            kind: 'suffix',
            name: 'of Craft',
            tier: null,
            crafted: true,
            inferred: true,
            tags: [],
            lines: [3],
          },
        ],
        requirements: {
          level: 30,
          strength: 0,
          dexterity: 10,
          intelligence: 0,
        },
      }),
      lines: [
        line(0, 'Rarity: RARE', {
          kind: 'rarity',
          section: null,
          status: 'other',
        }),
        line(1, 'Loom Test Band', {
          kind: 'name',
          section: null,
          status: 'other',
        }),
        line(2, 'Plain Test Ring', {
          kind: 'base',
          section: null,
          status: 'other',
        }),
        lifeLine(3, '+15 to maximum Life'),
        line(4, '+12 to Accuracy', {
          ranged: '+12(10-15) to Accuracy',
          stats: [{ id: 'accuracy_rating', value: '12' }],
        }),
        line(5, 'Nothing like this', {
          status: 'warning',
          code: 'LINE_NO_TEMPLATE',
        }),
      ],
    },
    {
      id: 2,
      name: 'Shiny Wand',
      base: 'Plain Wand',
      rarity: 'magic',
      slot: 'Weapon 1',
      card: card({ sockets: ['BG', 'R'] }),
      lines: [
        line(0, 'Rarity: MAGIC', {
          kind: 'rarity',
          section: null,
          status: 'other',
        }),
        line(1, 'Shiny Wand', { kind: 'name', section: null, status: 'other' }),
        line(2, '+5 TO MAXIMUM LIFE', {
          status: 'adapted',
          adapted: 'case-folded',
          stats: [{ id: 'base_maximum_life', value: '5' }],
        }),
      ],
    },
    {
      // Only in the second item set: not in use while set 1 is active.
      id: 3,
      name: 'Spare Band',
      base: 'Plain Test Ring',
      rarity: 'rare',
      slot: null,
      card: card({
        variants: ['Pre 3.7.0', 'Current'],
        'variant-eras': ['legacy', 'current'],
        'selected-variant': 2,
      }),
      lines: [
        line(0, 'Rarity: RARE', {
          kind: 'rarity',
          section: null,
          status: 'other',
        }),
        line(1, 'Spare Band', { kind: 'name', section: null, status: 'other' }),
        line(2, 'Something else unread', {
          status: 'warning',
          code: 'LINE_NO_TEMPLATE',
        }),
        line(3, 'Variant: Pre 3.7.0', {
          kind: 'spec',
          section: null,
          status: 'other',
        }),
        line(4, 'Variant: Current', {
          kind: 'spec',
          section: null,
          status: 'other',
        }),
        line(5, 'Selected Variant: 2', {
          kind: 'spec',
          section: null,
          status: 'other',
        }),
        line(6, '{variant:1}{range:0.5}+(10-20) to maximum Life', {
          shown: '+15 to maximum Life',
          ranged: '+15(10-20) to maximum Life',
          variants: [1],
          status: 'excluded',
        }),
        line(7, '{variant:2}{crafted}+25 to maximum Life', {
          variants: [2],
          marks: ['crafted'],
        }),
      ],
    },
    {
      // A jewel in no item set: the tree's sockets take it.
      id: 4,
      name: 'Frost Eye',
      base: 'Cobalt Jewel',
      rarity: 'rare',
      slot: null,
      card: card({ 'item-class': 'Jewel' }),
      lines: [
        line(0, 'Rarity: RARE', {
          kind: 'rarity',
          section: null,
          status: 'other',
        }),
        line(1, 'Frost Eye', { kind: 'name', section: null, status: 'other' }),
        line(2, 'Cobalt Jewel', {
          kind: 'base',
          section: null,
          status: 'other',
        }),
      ],
    },
  ]
}

/** The sample build's item sets: each set's slots, by item ID. */
const SETS: { id: number; title: string; slots: Record<string, number> }[] = [
  { id: 1, title: 'Leveling', slots: { 'Ring 1': 1, 'Weapon 1': 2 } },
  { id: 2, title: 'Endgame', slots: { 'Ring 1': 3, 'Weapon 1': 2 } },
]

/** The slot `item` is in, in set `set`. */
function slotIn(set: number, item: number): string | null {
  const slots = SETS.find((s) => s.id === set)?.slots ?? {}
  return Object.entries(slots).find(([, id]) => id === item)?.[0] ?? null
}

function life(items: ItemView[]): number {
  let total = 50
  for (const item of items) {
    for (const l of item.lines) {
      for (const s of l.stats) {
        if (s.id === 'base_maximum_life') {
          total += Number(s.value)
        }
      }
    }
  }
  return total
}

/** A small calc result document: one exact field with its breakdown, a
 * partial one, an unsupported one, and one that cannot be settled. */
export function sampleDocument(items: ItemView[]): CalcResultDocumentV1 {
  const value = life(items)
  const doc = {
    format: 'loom-calc-result',
    version: 1,
    result: {
      game: 'poe1',
      state: 'partial',
      reasons: { unsupported: [] },
      graph: [
        {
          kind: { constant: { label: 'base life' } },
          value: 50,
        },
        {
          kind: {
            contribution: {
              op: 'base',
              quantity: 'life',
              source: { item: { item: 1, line: 3 } },
            },
          },
          value: value - 50,
        },
        { kind: 'sum', children: [0, 1], term: 'base', value },
        { kind: 'product', children: [2], term: 'result', value },
      ],
      fields: [
        { field: 'life', state: 'complete', node: 3, value },
        { field: 'strength', state: 'complete', node: null, value: 20 },
        { field: 'dexterity', state: 'complete', node: null, value: 5 },
        { field: 'intelligence', state: 'complete', node: null, value: 20 },
        {
          field: 'resistance.fire',
          state: 'partial',
          value: -60,
          node: null,
          unsupported: ['unchecked-tree-version'],
        },
        { field: 'hit-dps', state: 'unsupported', reason: 'not-computed' },
        {
          field: 'average-damage',
          state: 'indeterminate',
          indeterminate: [{ 'unsettled-value': { source: 'enemy' } }],
        },
      ],
    },
  }
  return doc as unknown as CalcResultDocumentV1
}

type Listener = (event: ApiEvent) => void
type DropListener = (notice: DropNotice) => void

type Group = Omit<SkillGroupView, 'main'>

const gem = (
  name: string,
  level: number,
  colour: string,
  support = false,
  socket: number | null = null,
  supports: number[] = [],
): GemView => ({
  name,
  level,
  quality: 0,
  enabled: true,
  support,
  colour,
  main: false,
  socket,
  supports,
})

/** The sample build's skill sets, by ID. */
function sampleSkills(): Record<number, Group[]> {
  return {
    1: [
      {
        label: null,
        slot: 'Weapon 1',
        enabled: true,
        gems: [
          gem('Ice Nova', 1, 'blue', false, 0),
          gem('Added Cold Damage', 1, 'green', true, 1, [0]),
        ],
      },
      {
        label: 'Movement',
        slot: 'Boots',
        enabled: true,
        gems: [gem('Frostblink', 1, 'blue')],
      },
    ],
    2: [
      {
        label: null,
        slot: 'Body Armour',
        enabled: true,
        gems: [
          gem('Ice Nova', 20, 'blue'),
          gem('Added Cold Damage', 20, 'green', true, null, [0]),
        ],
      },
    ],
  }
}

/** The sample game data's gems (a few). */
const GEM_CATALOG: GemCatalogEntry[] = [
  {
    id: 'AddedColdDamageSupport',
    name: 'Added Cold Damage',
    support: true,
    colour: 'green',
    tags: ['Support', 'Cold'],
    fits: null,
  },
  {
    id: 'Frostblink',
    name: 'Frostblink',
    support: false,
    colour: 'blue',
    tags: ['Spell', 'Movement', 'Cold'],
    fits: null,
  },
  {
    id: 'IceNova',
    name: 'Ice Nova',
    support: false,
    colour: 'blue',
    tags: ['Spell', 'Area', 'Cold'],
    fits: null,
  },
  {
    id: 'MeleePhysicalDamageSupport',
    name: 'Melee Physical Damage',
    support: true,
    colour: 'red',
    tags: ['Support', 'Melee'],
    fits: null,
  },
  {
    id: 'Smite',
    name: 'Smite',
    support: false,
    colour: 'red',
    tags: ['Attack', 'Melee'],
    fits: null,
  },
]

/** A stage: its spec, title, item set, skill set. */
interface Stage {
  spec: number
  title: string
  set: number
  skillSet: number
}

/** The sample build's stages: spec, item set, skill set. */
const STAGES: Stage[] = [
  { spec: 1, title: 'Leveling', set: 1, skillSet: 1 },
  { spec: 2, title: 'Endgame', set: 2, skillSet: 2 },
]

interface State {
  items: ItemView[]
  /** The active item set. */
  set: number
  /** The active passive spec. */
  spec: number
  /** The active skill set. */
  skillSet: number
  skills: Record<number, Group[]>
  /** The main socket group, by index in the active skill set. */
  main: number
  /** The main gem, by index in the main group. */
  mainGem: number
  /** The active config set's inputs and blocks. */
  config: ConfigView
  league: 'challenge' | 'standard'
  secondWeaponSet: boolean
  /** The active passive spec's tree. */
  tree: fakeTree.FakeSpec
  /** The stages (a spec each), in order. */
  stages: Stage[]
}

/** A custom line's status as the resolver gives it: a line with no digit
 * is a note; "to maximum Life" matches; anything else is a warning. */
const fakeLine = (
  text: string,
): ConfigView['blocks'][number]['lines'][number] =>
  /\d/.test(text)
    ? /to maximum Life$/.test(text)
      ? { text, status: 'matched', typed: null }
      : { text, status: 'warning', typed: null }
    : { text, status: 'matched', typed: 'custom-note' }

const fakeBlock = (text: string, enabled = true) => ({
  title: null,
  enabled,
  text,
  lines: text
    .split('\n')
    .map((l) => l.trim())
    .filter((l) => l !== '')
    .map(fakeLine),
})

/** The effect the engine gives an input, by name (a sample of its rule). */
const fakeEffect = (name: string): ConfigView['inputs'][number]['effect'] =>
  name.startsWith('condition')
    ? 'no-effect'
    : ['enemyIsBoss', 'usePowerCharges', 'resistancePenalty'].includes(name)
      ? 'applied'
      : 'unmodelled'

const sampleConfig = (): ConfigView => ({
  set: 1,
  inputs: [
    { name: 'resistancePenalty', value: { number: -60 }, placeholder: false },
    { name: 'enemyIsBoss', value: { text: 'Pinnacle' }, placeholder: false },
    { name: 'usePowerCharges', value: { boolean: true }, placeholder: false },
    { name: 'conditionFullLife', value: { boolean: true }, placeholder: false },
    { name: 'enemyLevel', value: { number: 83 }, placeholder: true },
  ].map((i) => ({ ...i, effect: fakeEffect(i.name) })),
  blocks: [fakeBlock('+15 to maximum Life\nBoss phase two')],
})

const fresh = (): State => ({
  items: sampleItems(),
  set: 1,
  spec: 1,
  skillSet: 1,
  skills: sampleSkills(),
  main: 0,
  mainGem: 0,
  config: sampleConfig(),
  league: 'challenge',
  secondWeaponSet: false,
  tree: fakeTree.freshSpec(),
  stages: STAGES,
})

export class FakeApi implements LoomApi {
  version = 0
  private options: FakeOptions
  private listeners: Listener[] = []
  private dropListeners: DropListener[] = []
  /** The last dropped file's kind, until opened. */
  private dropped: 'pob' | 'build' | 'other' | null = null
  private current: State | null = null
  private undoStack: State[] = []
  private redoStack: State[] = []
  /** The change that made each state, for the history's words. */
  private made = new WeakMap<State, HistoryEntry>()
  private dirty = false
  private path: string | null = null
  private calc: [number, CalcResultDocumentV1] | null = null
  private calcState: Snapshot['calc'] = {
    state: 'idle',
    version: 0,
    'result-state': null,
    error: null,
  }
  private mcp: McpStatus = {
    running: false,
    url: null,
    token: null,
    permissions: { requests: false, 'standing-grant': false },
    requests: [],
  }
  private recentList: RecentEntry[]
  private recoveryList: RecoveryEntry[]
  /** Every command executed, in order (for tests). */
  readonly executed: Command[] = []

  constructor(options: FakeOptions = {}) {
    this.options = { data: true, ...options }
    this.recentList = options.recent ?? []
    this.recoveryList = options.recovery ?? []
    this.activeBuild = options.installed?.[0] ?? 26530219
  }

  /** The loaded bundle's Steam build. */
  private activeBuild: number

  private bundleId(build: number) {
    return `poe1/${build}-s4`
  }

  private emit(kind: ApiEvent['kind']) {
    const event: ApiEvent = { version: this.version, correlation: null, kind }
    setTimeout(() => {
      for (const l of this.listeners) {
        l(event)
      }
    }, 0)
  }

  private fail(code: string): never {
    throw new ApiFailure({ code, message: code })
  }

  private changed(state: State | null) {
    this.current = state
    this.version += 1
    this.emit('build-changed')
    if (state === null) {
      this.calc = null
      this.calcState = {
        state: 'idle',
        version: 0,
        'result-state': null,
        error: null,
      }
      return
    }
    if (this.options.calcFails !== undefined) {
      const error = { code: this.options.calcFails, message: 'failed' }
      this.calcState = { ...this.calcState, state: 'failed', error }
      this.emit({ 'calc-failed': { error } })
      return
    }
    this.calcState = { ...this.calcState, state: 'running', error: null }
    this.emit('calc-started')
    const version = this.version
    setTimeout(() => {
      if (version !== this.version || this.current === null) {
        return
      }
      this.calc = [version, sampleDocument(this.current.items)]
      this.calcState = {
        state: 'done',
        version,
        'result-state': 'partial',
        error: null,
      }
      this.emit({ 'calc-finished': { state: 'partial' } })
    }, 5)
  }

  /** Makes `next` current as one undoable change; nothing when it is
   * the same. */
  private commit(next: State, entry?: HistoryEntry): Outcome {
    if (this.current === null) this.fail('APP_NO_BUILD')
    if (JSON.stringify(next) === JSON.stringify(this.current)) return 'nothing'
    if (entry !== undefined) this.made.set(next, entry)
    this.undoStack.push(this.current)
    this.redoStack = []
    this.dirty = true
    this.changed(next)
    return this.done()
  }

  private done(): Outcome {
    return { done: { version: this.version } }
  }

  snapshot(): Promise<Snapshot> {
    const items = this.current?.items ?? []
    const set = this.current?.set ?? 1
    const socketOf = (id: number) =>
      this.current?.tree.jewels.find((j) => j.item === id)?.node ?? null
    const inUse = (id: number) =>
      slotIn(set, id) !== null || socketOf(id) !== null
    const warnings = items.flatMap((item) =>
      item.lines
        .filter((l) => l.status === 'warning')
        .map((l) => ({
          item: item.id,
          'item-name': item.name,
          line: l.index,
          text: l.text,
          code: l.code ?? '',
          'in-use': inUse(item.id),
        })),
    )
    const all = items
      .flatMap((i) => i.lines)
      .filter((l) => l.kind === 'modifier')
    const count = (s: LineView['status']) =>
      all.filter((l) => l.status === s).length
    return Promise.resolve({
      version: this.version,
      build:
        this.current === null
          ? null
          : {
              name: this.path === null ? '' : 'saved',
              game: 'poe1',
              class: 'Witch',
              ascendancy: null,
              level: 90,
              'main-skill': 'Ice Nova',
              'data-version': {
                game: 'poe1',
                'schema-version': 3,
                'steam-build': 26530219,
              },
              completeness: warnings.length > 0 ? 'partial' : 'complete',
              'in-use-completeness': warnings.some((w) => w['in-use'])
                ? 'partial'
                : 'complete',
              lines: {
                modifiers: all.length,
                matched: count('matched'),
                adapted: count('adapted'),
                warnings: count('warning'),
                excluded: count('excluded'),
              },
              items: items.map((i) => ({
                id: i.id,
                'item-class':
                  i.card['item-class'] === 'Jewel'
                    ? 'Jewel'
                    : i.id === 2
                      ? 'Wand'
                      : 'Ring',
                name: i.name,
                base: i.base,
                rarity: i.rarity,
                slot: slotIn(set, i.id),
                warnings: i.lines.filter((l) => l.status === 'warning').length,
                'in-use': inUse(i.id),
                sets: SETS.filter((s) =>
                  Object.values(s.slots).includes(i.id),
                ).map((s) => s.id),
                'tree-socket': socketOf(i.id),
                legacy:
                  i.card['variant-eras'][
                    (i.card['selected-variant'] ?? 0) - 1
                  ] === 'legacy',
                // The wand: Ice Nova and its support linked, one socket free.
                sockets: i.id === 2 ? ['BG', 'R'] : [],
                requirements: i.card.requirements ?? null,
              })),
              warnings,
              'item-sets': SETS.map((s) => ({
                id: s.id,
                title: s.title,
                active: s.id === set,
              })),
              'skill-sets': STAGES.map((s) => ({
                id: s.skillSet,
                title: `^4${s.title} {${s.spec}}`,
                active: s.skillSet === this.current?.skillSet,
              })),
              specs: (this.current?.stages ?? STAGES).map((s) => ({
                id: s.spec,
                title: `${s.title} {${s.spec}}`,
                active: s.spec === this.current?.spec,
              })),
              'config-sets': [{ id: 1, title: 'Default', active: true }],
              loadouts: (this.current?.stages ?? STAGES).map((s) => ({
                spec: s.spec,
                title: s.title,
                'item-set': s.set,
                'skill-set': s.skillSet,
                'config-set': 1,
                active:
                  s.spec === this.current?.spec &&
                  s.set === set &&
                  s.skillSet === this.current?.skillSet,
              })),
              'skill-groups': (
                this.current?.skills[this.current.skillSet] ?? []
              ).map((g, i) => ({
                ...g,
                main: i === this.current?.main,
                gems: g.gems.map((x, j) => ({
                  ...x,
                  main: i === this.current?.main && j === this.current.mainGem,
                })),
              })),
              'passive-names': [{ node: 420, name: 'Heart of Ice' }],
              tree:
                this.options.data === true && this.current !== null
                  ? {
                      ...fakeTree.view(this.current.tree, this.current.spec),
                      locked: this.options.treeLocked ?? null,
                    }
                  : null,
              config: this.current?.config ?? null,
              league: this.current?.league ?? 'challenge',
              'second-weapon-set': this.current?.secondWeaponSet ?? false,
            },
      calc: this.calcState,
      history: {
        'can-undo': this.undoStack.length > 0,
        'can-redo': this.redoStack.length > 0,
        undo:
          this.undoStack.length > 0 && this.current !== null
            ? (this.made.get(this.current) ?? null)
            : null,
        redo: this.made.get(this.redoStack.at(-1) as State) ?? null,
      },
      dirty: this.dirty,
      path: this.path,
      origin: null,
      data: {
        bundle: this.options.data === true ? '/data/bundle' : null,
        'data-version':
          this.options.data === true
            ? {
                game: 'poe1',
                'schema-version': 3,
                'steam-build': this.activeBuild,
              }
            : null,
        // The tree version is known for the real bundle's build only.
        'tree-version':
          this.options.data === true && this.activeBuild === 26530219
            ? '3_29'
            : null,
        error: null,
        installed: (this.options.installed ?? []).map((b) => ({
          id: this.bundleId(b),
          'data-version': {
            game: 'poe1',
            'schema-version': 4,
            'steam-build': b,
          },
          'tree-version': b === 26530219 ? '3_29' : null,
        })),
        active:
          this.options.data === true &&
          (this.options.installed ?? []).includes(this.activeBuild)
            ? this.bundleId(this.activeBuild)
            : null,
        notice: null,
        pinned:
          this.current !== null &&
          this.options.pinned !== undefined &&
          this.options.pinned !== this.activeBuild &&
          (this.options.installed ?? []).includes(this.options.pinned)
            ? this.bundleId(this.options.pinned)
            : null,
      },
      notices: [],
    })
  }

  calcDocument() {
    return Promise.resolve(this.calc)
  }

  item(id: number): Promise<ItemView> {
    const item = this.current?.items.find((i) => i.id === id)
    if (item === undefined) {
      this.fail('APP_EDIT_ITEM')
    }
    return Promise.resolve(structuredClone(item))
  }

  private applyEdit(edit: ItemEdit): State {
    if (this.current === null) {
      this.fail('APP_NO_BUILD')
    }
    const items = structuredClone(this.current.items)
    const item = items.find((i) => i.id === edit.item)
    if (item === undefined) {
      this.fail('APP_EDIT_ITEM')
    }
    for (const e of edit.expect) {
      if (item.lines[e.line]?.text !== e.text) {
        this.fail('APP_STALE_EDIT')
      }
    }
    const texts: (string | null)[] = item.lines.map((l) => l.text)
    for (const op of edit.ops) {
      if ('replace' in op) {
        texts[op.replace.line] = op.replace.text
      } else if ('remove' in op) {
        texts[op.remove.line] = null
      } else if ('add' in op) {
        texts.push(op.add.text)
      } else if ('select-variant' in op) {
        const v = op['select-variant'].variant
        if (v < 1 || v > item.card.variants.length) this.fail('APP_EDIT_LINE')
        item.card['selected-variant'] = v
        item.card['current-variant'] =
          item.card['variant-eras'][v - 1] === 'legacy'
            ? item.card['variant-eras'].indexOf('current') + 1 || null
            : null
        item.lines = item.lines.map((l) =>
          l.variants.length === 0
            ? l
            : { ...l, status: l.variants.includes(v) ? 'matched' : 'excluded' },
        )
      }
    }
    const kept = item.lines.length
    item.lines = texts.flatMap((text, i) => {
      if (text === null) {
        return []
      }
      const old = item.lines[i]
      if (i < kept && old !== undefined && old.text === text) {
        return [old]
      }
      return [lifeLine(0, text)]
    })
    item.lines = item.lines.map((l, index) => ({ ...l, index }))
    return { ...this.current, items }
  }

  preview(edit: ItemEdit): Promise<ItemEditPreview> {
    return Promise.resolve(this.previewOf(edit))
  }

  /** The effect of an edit, calculated (the app attaches it to a change
   * request too). */
  private previewOf(edit: ItemEdit): ItemEditPreview {
    const after = this.applyEdit(edit)
    const before = life(this.current?.items ?? [])
    const item = after.items.find((i) => i.id === edit.item)
    if (item === undefined) {
      this.fail('APP_EDIT_ITEM')
    }
    return {
      item,
      fields: [
        {
          field: 'life',
          before,
          after: life(after.items),
          'state-before': 'complete',
          'state-after': 'complete',
        },
      ],
    }
  }

  execute(command: Command): Promise<Outcome> {
    this.executed.push(command)
    try {
      return Promise.resolve(this.run(command))
    } catch (e) {
      return Promise.reject(e as Error)
    }
  }

  private run(command: Command): Outcome {
    if (typeof command === 'string') {
      switch (command) {
        case 'undo': {
          const prev = this.undoStack.pop()
          if (prev === undefined || this.current === null) {
            return 'nothing'
          }
          this.redoStack.push(this.current)
          this.dirty = true
          this.changed(prev)
          return this.done()
        }
        case 'redo': {
          const next = this.redoStack.pop()
          if (next === undefined || this.current === null) {
            return 'nothing'
          }
          this.undoStack.push(this.current)
          this.dirty = true
          this.changed(next)
          return this.done()
        }
        case 'save':
          if (this.path === null) {
            this.fail('APP_NO_PATH')
          }
          this.dirty = false
          this.version += 1
          this.emit({ saved: { path: this.path } })
          return this.done()
        case 'export-share-code':
          return { text: { version: this.version, text: 'eNrFAKESHARECODE' } }
        case 'close':
          this.undoStack = []
          this.redoStack = []
          this.path = null
          this.dirty = false
          this.changed(null)
          return this.done()
      }
    }
    if (typeof command === 'object' && 'use-bundle' in command) {
      const build = (this.options.installed ?? []).find(
        (b) => this.bundleId(b) === command['use-bundle'].id,
      )
      if (build === undefined) this.fail('APP_NO_BUNDLE')
      this.activeBuild = build
      this.options.data = true
      this.version += 1
      this.emit('build-changed')
      return this.done()
    }
    if (command === 'new-build') {
      if (this.options.data !== true) this.fail('APP_NO_DATA')
      this.undoStack = []
      this.redoStack = []
      this.path = null
      this.dirty = false
      this.changed({ ...fresh(), items: [], skills: {} })
      return this.done()
    }
    if ('import-text' in command) {
      return this.importText(command['import-text'].text)
    }
    if ('edit-item' in command) {
      const next = this.applyEdit(command['edit-item'])
      if (this.current !== null) {
        this.undoStack.push(this.current)
      }
      this.redoStack = []
      this.dirty = true
      this.changed(next)
      return this.done()
    }
    if ('use-item-set' in command) {
      const id = command['use-item-set'].id
      if (this.current === null) {
        this.fail('APP_NO_BUILD')
      }
      if (!SETS.some((s) => s.id === id)) {
        this.fail('APP_ITEM_SET')
      }
      return this.commit(
        { ...this.current, set: id },
        { kind: 'item-set', item: id },
      )
    }
    if ('use-loadout' in command) {
      if (this.current === null) this.fail('APP_NO_BUILD')
      const stage = this.current.stages.find(
        (s) => s.spec === command['use-loadout'].spec,
      )
      if (stage === undefined) this.fail('APP_LOADOUT')
      return this.commit(
        {
          ...this.current,
          spec: stage.spec,
          set: stage.set,
          skillSet: stage.skillSet,
        },
        { kind: 'loadout', item: stage.spec },
      )
    }
    if ('use-skill-set' in command) {
      const id = command['use-skill-set'].id
      if (this.current === null) this.fail('APP_NO_BUILD')
      if (this.current.skills[id] === undefined) this.fail('APP_SKILL')
      return this.commit(
        { ...this.current, skillSet: id },
        { kind: 'skill-set', item: id },
      )
    }
    if ('edit-gem' in command) {
      const e = command['edit-gem']
      if (this.current === null) this.fail('APP_NO_BUILD')
      const skills = structuredClone(this.current.skills)
      const g = skills[this.current.skillSet]?.[e.group]?.gems[e.gem]
      if (g === undefined || e.level === 0) this.fail('APP_SKILL')
      if (e.level != null) g.level = e.level
      if (e.quality != null) g.quality = e.quality
      if (e.enabled != null) g.enabled = e.enabled
      return this.commit(
        { ...this.current, skills },
        { kind: 'skill-edit', item: e.group },
      )
    }
    if ('edit-socket-group' in command) {
      const e = command['edit-socket-group']
      if (this.current === null) this.fail('APP_NO_BUILD')
      const skills = structuredClone(this.current.skills)
      const g = skills[this.current.skillSet]?.[e.group]
      if (g === undefined) this.fail('APP_SKILL')
      if (e.enabled != null) g.enabled = e.enabled
      if (e.slot != null) g.slot = e.slot === '' ? null : e.slot
      if (e.label != null) g.label = e.label === '' ? null : e.label
      return this.commit(
        {
          ...this.current,
          skills,
          main: e.main === true ? e.group : this.current.main,
          mainGem: e['main-gem'] ?? this.current.mainGem,
        },
        { kind: 'skill-edit', item: e.group },
      )
    }
    if ('add-gem' in command) {
      const e = command['add-gem']
      if (this.current === null) this.fail('APP_NO_BUILD')
      const skills = structuredClone(this.current.skills)
      const g = skills[this.current.skillSet]?.[e.group]
      const entry = GEM_CATALOG.find((x) => x.id === e.gem)
      if (g === undefined || entry === undefined) this.fail('APP_SKILL')
      const at = Math.min(e.at ?? g.gems.length, g.gems.length)
      g.gems.splice(
        at,
        0,
        gem(entry.name, 20, entry.colour ?? 'white', entry.support, null, []),
      )
      return this.commit(
        { ...this.current, skills },
        { kind: 'skill-edit', item: e.group },
      )
    }
    if ('remove-gem' in command) {
      const e = command['remove-gem']
      if (this.current === null) this.fail('APP_NO_BUILD')
      const skills = structuredClone(this.current.skills)
      const g = skills[this.current.skillSet]?.[e.group]
      if (g === undefined || g.gems[e.gem] === undefined) this.fail('APP_SKILL')
      g.gems.splice(e.gem, 1)
      return this.commit(
        { ...this.current, skills },
        { kind: 'skill-edit', item: e.group },
      )
    }
    if ('add-socket-group' in command) {
      const e = command['add-socket-group']
      if (this.current === null) this.fail('APP_NO_BUILD')
      const skills = structuredClone(this.current.skills)
      const set = skills[this.current.skillSet] ?? []
      set.push({
        label: e.label ?? null,
        slot: e.slot ?? null,
        enabled: true,
        gems: [],
      })
      skills[this.current.skillSet] = set
      return this.commit(
        { ...this.current, skills },
        { kind: 'skill-edit', item: set.length - 1 },
      )
    }
    if ('remove-socket-group' in command) {
      const e = command['remove-socket-group']
      if (this.current === null) this.fail('APP_NO_BUILD')
      const skills = structuredClone(this.current.skills)
      const set = skills[this.current.skillSet]
      if (set?.[e.group] === undefined) this.fail('APP_SKILL')
      set.splice(e.group, 1)
      const main = this.current.main
      return this.commit(
        {
          ...this.current,
          skills,
          main: main > e.group ? main - 1 : Math.min(main, set.length - 1),
        },
        { kind: 'skill-edit', item: e.group },
      )
    }
    if ('use-weapon-set' in command) {
      if (this.current === null) this.fail('APP_NO_BUILD')
      const second = command['use-weapon-set'].second
      return this.commit(
        { ...this.current, secondWeaponSet: second },
        { kind: 'weapon-set', item: 1 },
      )
    }
    if ('set-league' in command) {
      if (this.current === null) this.fail('APP_NO_BUILD')
      return this.commit(
        { ...this.current, league: command['set-league'].league },
        { kind: 'league', item: 0 },
      )
    }
    if ('set-config-input' in command) {
      const e = command['set-config-input']
      if (this.current === null) this.fail('APP_NO_BUILD')
      if (e.name.trim() === '' || e.name === 'customMods')
        this.fail('APP_CONFIG')
      const config = structuredClone(this.current.config)
      const input = config.inputs.find((i) => i.name === e.name)
      if (input === undefined) {
        config.inputs.push({
          name: e.name,
          value: e.value,
          placeholder: false,
          effect: fakeEffect(e.name),
        })
      } else {
        input.value = e.value
        input.placeholder = false
      }
      return this.commit(
        { ...this.current, config },
        { kind: 'config', item: 0 },
      )
    }
    if ('remove-config-input' in command) {
      const e = command['remove-config-input']
      if (this.current === null) this.fail('APP_NO_BUILD')
      const config = structuredClone(this.current.config)
      const at = config.inputs.findIndex((i) => i.name === e.name)
      if (at < 0) this.fail('APP_CONFIG')
      config.inputs.splice(at, 1)
      return this.commit(
        { ...this.current, config },
        { kind: 'config', item: 0 },
      )
    }
    if ('edit-custom-block' in command) {
      const e = command['edit-custom-block']
      if (this.current === null) this.fail('APP_NO_BUILD')
      const config = structuredClone(this.current.config)
      const block = config.blocks[e.block]
      if (block === undefined) this.fail('APP_CONFIG')
      const next = fakeBlock(e.text ?? block.text, e.enabled ?? block.enabled)
      config.blocks[e.block] = { ...next, title: block.title }
      return this.commit(
        { ...this.current, config },
        { kind: 'config', item: e.block },
      )
    }
    if ('add-custom-block' in command) {
      if (this.current === null) this.fail('APP_NO_BUILD')
      const config = structuredClone(this.current.config)
      config.blocks.push(fakeBlock(command['add-custom-block'].text))
      return this.commit(
        { ...this.current, config },
        { kind: 'config', item: config.blocks.length - 1 },
      )
    }
    if ('remove-custom-block' in command) {
      const e = command['remove-custom-block']
      if (this.current === null) this.fail('APP_NO_BUILD')
      const config = structuredClone(this.current.config)
      if (config.blocks[e.block] === undefined) this.fail('APP_CONFIG')
      config.blocks.splice(e.block, 1)
      return this.commit(
        { ...this.current, config },
        { kind: 'config', item: e.block },
      )
    }
    const tree = this.treeCommand(command)
    if (tree !== null) return tree
    if ('open-recent' in command) {
      if (!this.recentList.some((r) => r.id === command['open-recent'].id)) {
        this.fail('APP_NOT_FOUND')
      }
      this.path = 'recent.json'
      this.dirty = false
      this.changed(fresh())
      return this.done()
    }
    if ('restore' in command) {
      this.recoveryList = this.recoveryList.filter(
        (r) => r.id !== command.restore.id,
      )
      this.dirty = true
      this.changed(fresh())
      return this.done()
    }
    if ('forget-recent' in command) {
      const id = command['forget-recent'].id
      if (!this.recentList.some((r) => r.id === id)) this.fail('APP_NOT_FOUND')
      this.recentList = this.recentList.filter((r) => r.id !== id)
      return 'nothing'
    }
    if ('discard' in command) {
      this.recoveryList = this.recoveryList.filter(
        (r) => r.id !== command.discard.id,
      )
      return 'nothing'
    }
    if ('approve' in command) {
      const r = this.request(command.approve.id)
      if (r.tree != null) {
        const outcome = this.treeCommand(r.tree as Command)
        r.status = 'applied'
        this.emit('mcp-changed')
        return outcome ?? 'nothing'
      }
      if (r.edit == null) this.fail('APP_REQUEST')
      const next = this.applyEdit(r.edit)
      if (this.current !== null) {
        this.undoStack.push(this.current)
      }
      r.status = 'applied'
      this.dirty = true
      this.changed(next)
      this.emit('mcp-changed')
      return this.done()
    }
    if ('decline' in command) {
      this.request(command.decline.id).status = 'declined'
      this.emit('mcp-changed')
      return 'nothing'
    }
    return 'nothing'
  }

  /** A passive tree command, or `null` for another command. */
  private treeCommand(command: Command): Outcome | null {
    if (typeof command !== 'object') return null
    const edit = (
      spec: number,
      f: (s: fakeTree.FakeSpec) => fakeTree.FakeSpec,
    ): Outcome => {
      if (this.current === null) this.fail('APP_NO_BUILD')
      if (spec !== this.current.spec) this.fail('APP_TREE')
      if (this.options.treeLocked !== undefined) this.fail('APP_TREE')
      let next: fakeTree.FakeSpec
      try {
        next = f(this.current.tree)
      } catch (e) {
        if (e instanceof fakeTree.TreeRefused) this.fail('APP_TREE')
        throw e
      }
      return this.commit(
        { ...this.current, tree: next },
        { kind: 'tree', item: spec },
      )
    }
    if ('allocate-node' in command) {
      const c = command['allocate-node']
      return edit(c.spec, (s) => fakeTree.allocate(s, c.node))
    }
    if ('deallocate-node' in command) {
      const c = command['deallocate-node']
      return edit(c.spec, (s) => fakeTree.deallocate(s, c.node))
    }
    if ('set-mastery-effect' in command) {
      const c = command['set-mastery-effect']
      return edit(c.spec, (s) => fakeTree.setMastery(s, c.node, c.effect))
    }
    if ('select-class' in command) {
      const c = command['select-class']
      return edit(c.spec, (s) => fakeTree.selectClass(s, c.class))
    }
    if ('select-ascendancy' in command) {
      const c = command['select-ascendancy']
      return edit(c.spec, (s) => fakeTree.selectAscendancy(s, c.ascendancy))
    }
    const specs = (
      next: Stage[],
      item: number,
      active = this.current?.spec ?? 1,
    ): Outcome => {
      if (this.current === null) this.fail('APP_NO_BUILD')
      return this.commit(
        { ...this.current, stages: next, spec: active },
        { kind: 'tree-specs', item },
      )
    }
    const named = (title: string) => {
      if (title.trim() === '') this.fail('APP_TREE')
      return title.trim()
    }
    if ('new-spec' in command) {
      const c = command['new-spec']
      if (this.current === null) this.fail('APP_NO_BUILD')
      const stages = this.current.stages
      const from =
        c.copy == null ? undefined : stages.find((s) => s.spec === c.copy)
      if (c.copy != null && from === undefined) this.fail('APP_TREE')
      const spec = stages.length + 1
      return specs(
        [
          ...stages,
          {
            spec,
            title: named(c.title),
            set: from?.set ?? this.current.set,
            skillSet: from?.skillSet ?? this.current.skillSet,
          },
        ],
        spec,
      )
    }
    if ('rename-spec' in command) {
      const c = command['rename-spec']
      if (this.current === null) this.fail('APP_NO_BUILD')
      if (!this.current.stages.some((s) => s.spec === c.spec))
        this.fail('APP_TREE')
      const title = named(c.title)
      return specs(
        this.current.stages.map((s) =>
          s.spec === c.spec ? { ...s, title } : s,
        ),
        c.spec,
      )
    }
    if ('delete-spec' in command) {
      const c = command['delete-spec']
      if (this.current === null) this.fail('APP_NO_BUILD')
      const stages = this.current.stages
      if (stages.length <= 1 || !stages.some((s) => s.spec === c.spec))
        this.fail('APP_TREE')
      const active = this.current.spec
      return specs(
        stages
          .filter((s) => s.spec !== c.spec)
          .map((s, i) => ({ ...s, spec: i + 1 })),
        c.spec,
        active === c.spec
          ? Math.max(1, c.spec - 1)
          : active > c.spec
            ? active - 1
            : active,
      )
    }
    if ('socket-jewel' in command) {
      const c = command['socket-jewel']
      const item = c.item ?? null
      if (
        item !== null &&
        this.current?.items.find((i) => i.id === item)?.card['item-class'] !==
          'Jewel'
      )
        this.fail('APP_TREE')
      return edit(c.spec, (s) => fakeTree.socketJewel(s, c.node, item))
    }
    if ('reset-tree' in command) {
      const c = command['reset-tree']
      return edit(c.spec, (s) => fakeTree.reset(s))
    }
    return null
  }

  treeLayout(): Promise<TreeLayout> {
    if (this.options.data !== true) {
      return Promise.reject(
        new ApiFailure({ code: 'APP_NO_DATA', message: 'APP_NO_DATA' }),
      )
    }
    return Promise.resolve(fakeTree.FAKE_LAYOUT)
  }

  // The fake data has no sprites: the tree is drawn in our own shapes.
  treeSprites(): Promise<TreeSprites> {
    return Promise.reject(
      new ApiFailure({ code: 'APP_NO_DATA', message: 'APP_NO_DATA' }),
    )
  }

  previewTree(spec: number, node: number): Promise<TreePreview> {
    try {
      if (this.current === null) this.fail('APP_NO_BUILD')
      if (spec !== this.current.spec) this.fail('APP_TREE')
      return Promise.resolve(
        fakeTree.preview(this.current.tree, node, life(this.current.items)),
      )
    } catch (e) {
      if (e instanceof fakeTree.TreeRefused) {
        return Promise.reject(
          new ApiFailure({ code: 'APP_TREE', message: e.message }),
        )
      }
      return Promise.reject(e as Error)
    }
  }

  private request(id: number): ChangeRequest {
    const r = this.mcp.requests.find(
      (q) => q.id === id && q.status === 'pending',
    )
    if (r === undefined) {
      this.fail('APP_REQUEST')
    }
    return r
  }

  private importText(text: string): Outcome {
    if (this.options.data !== true) {
      this.fail('APP_NO_DATA')
    }
    if (this.options.importFails !== undefined) {
      this.fail(this.options.importFails)
    }
    if (text.trim() === '') {
      this.fail('IMPORT_XML')
    }
    this.undoStack = []
    this.redoStack = []
    this.path = null
    this.dirty = true
    this.changed(fresh())
    return this.done()
  }

  recent() {
    return Promise.resolve(this.recentList)
  }

  gems(group?: number) {
    if (!this.options.data) this.fail('APP_NO_DATA')
    const groups = this.current?.skills[this.current.skillSet] ?? []
    if (group !== undefined && groups[group] === undefined) {
      this.fail('APP_SKILL')
    }
    return Promise.resolve(
      GEM_CATALOG.map((e) => ({
        ...e,
        // Cold supports fit a group with a cold spell; Melee ones do not.
        fits:
          group === undefined || !e.support
            ? null
            : e.tags.includes('Cold') &&
              (groups[group]?.gems ?? []).some((g) => g.name === 'Ice Nova'),
      })),
    )
  }

  recovery() {
    return Promise.resolve(this.recoveryList)
  }

  mcpStatus() {
    return Promise.resolve(structuredClone(this.mcp))
  }

  setPermissions(permissions: McpPermissions) {
    this.mcp.permissions = permissions
    return this.mcpStatus()
  }

  mcpStart() {
    this.mcp.running = true
    this.mcp.url = 'http://127.0.0.1:50000/mcp'
    this.mcp.token ??= 'fake-token'
    this.mcp['token-kept'] = true
    return this.mcpStatus()
  }

  mcpStop() {
    this.mcp.running = false
    this.mcp.url = null
    this.mcp.token = null
    this.mcp['token-kept'] = false
    this.mcp.permissions['standing-grant'] = false
    return this.mcpStatus()
  }

  private rotations = 0
  mcpRotate() {
    this.rotations += 1
    if (this.mcp.running) this.mcp.token = `fake-token-${this.rotations}`
    return this.mcpStatus()
  }

  /** A client's change request (for tests). */
  addRequest(edit: ItemEdit): ChangeRequest {
    const r: ChangeRequest = {
      id: this.mcp.requests.length + 1,
      client: 'test-client',
      edit,
      note: 'More life',
      preview: this.previewOf(edit),
      status: 'pending',
      error: null,
    }
    this.mcp.requests.push(r)
    this.emit('mcp-changed')
    return r
  }

  /** A client's passive tree change request (for tests). */
  addTreeRequest(change: TreeChange): ChangeRequest {
    const [key] = Object.keys(change)
    const { spec, node } = Object.values(change)[0] as {
      spec: number
      node: number
    }
    if (this.current === null || spec !== this.current.spec)
      this.fail('APP_TREE')
    const preview = fakeTree.preview(
      this.current.tree,
      node,
      life(this.current.items),
    )
    const r: ChangeRequest = {
      id: this.mcp.requests.length + 1,
      client: 'test-client',
      edit: null,
      note: 'More damage',
      preview: null,
      status: 'pending',
      error: null,
      tree: change,
      'tree-preview':
        key === 'set-mastery-effect' ? { ...preview, path: [] } : preview,
    }
    this.mcp.requests.push(r)
    this.emit('mcp-changed')
    return r
  }

  importFile() {
    return Promise.resolve(this.importText('<PathOfBuilding/>'))
  }

  openFile() {
    this.path = 'build.json'
    this.dirty = false
    this.undoStack = []
    this.redoStack = []
    this.changed(fresh())
    return Promise.resolve(this.done())
  }

  saveAs() {
    if (this.current === null) {
      this.fail('APP_NO_BUILD')
    }
    this.path = 'build.json'
    this.dirty = false
    this.version += 1
    this.emit({ saved: { path: this.path } })
    return Promise.resolve(this.done())
  }

  exportXml() {
    return Promise.resolve(this.done())
  }

  /** The saved preferences document (for tests). */
  prefs: string | null = null

  uiPrefs() {
    return Promise.resolve(this.prefs)
  }

  setUiPrefs(text: string) {
    this.prefs = text
    return Promise.resolve()
  }

  chooseBundle() {
    this.options.data = true
    this.version += 1
    this.emit('build-changed')
    return Promise.resolve(this.done())
  }

  openDropped() {
    const kind = this.dropped
    this.dropped = null
    if (kind === null) return Promise.resolve<Outcome>('nothing')
    try {
      if (kind === 'other') this.fail('UI_DROP')
      return kind === 'pob' ? this.importFile() : this.openFile()
    } catch (e) {
      return Promise.reject(e as Error)
    }
  }

  /** A test's drag and drop, as the shell reports it. */
  drop(notice: DropNotice) {
    if (notice.state === 'dropped') this.dropped = notice.kind
    for (const l of this.dropListeners) l(notice)
  }

  onDrop(listener: DropListener) {
    this.dropListeners.push(listener)
    return Promise.resolve(() => {
      this.dropListeners = this.dropListeners.filter((l) => l !== listener)
    })
  }

  onEvent(listener: Listener) {
    this.listeners.push(listener)
    return Promise.resolve(() => {
      this.listeners = this.listeners.filter((l) => l !== listener)
    })
  }
}
