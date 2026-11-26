// An in-memory LoomApi for tests and for running the UI in a browser
// without the shell. It models the application's behaviour the UI relies
// on: versions, background calculation events, undo and redo, stale
// edits, warnings, change requests. Numbers are sample data.
import {
  ApiFailure,
  type BuildFields,
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
  type CrashRecord,
  type DataFeedStatus,
  type QuarantinedBundle,
  type UpdateMode,
  type UpdateOffer,
  type DataAutoStatus,
  type UpdateStatus,
  type DiagnosticFile,
  type TreeChange,
  type TreePreview,
  type PassiveLines,
  type StageView,
  type WhatIfRequest,
  type WhatIfSeries,
} from './types'
import { type DemoInput, demoDocument, demoValues } from './fakeDemo'
import * as fakeTree from './fakeTree'

export interface FakeOptions {
  /** Whether game data is loaded. */
  data?: boolean
  /** A full set of sample fields that follow the stage, the main gem's
   * level, and the enemy's level (`fakeDemo`): for running the UI in a
   * browser. Off, the fields are the small fixed set the tests read. */
  demo?: boolean
  /** A code every import fails with. */
  importFails?: string
  /** A code every calculation fails with. */
  calcFails?: string
  recent?: RecentEntry[]
  recovery?: RecoveryEntry[]
  /** Installed bundles by Steam build (B-1); the first is loaded. */
  installed?: number[]
  /** Installed bundles that do not load (Phase 5, P-e). */
  quarantined?: QuarantinedBundle[]
  /** The data is still loading at start (Phase 5, P7). */
  loading?: boolean
  /** What the next "install from a file" installs: a signed or an
   * unsigned pack of this game build. */
  nextPack?: { build: number; signed: boolean }
  /** What a data feed check finds (Phase 5, P-e). */
  feed?: DataFeedStatus
  /** The Steam build the build was made with (its pinned data). */
  pinned?: number
  /** The file picker of `compareFile` is cancelled. */
  cancelPick?: boolean
  /** Why the passive tree cannot be edited (its version or class). */
  treeLocked?: string
  /** What converting the locked tree would do (T5b): offered with
   * `treeLocked`, which conversion lifts. */
  treeConvert?: { version: string; dropped: number[] }
  /** Crash records from an earlier run (Phase 5, P-f). */
  crashes?: CrashRecord[]
  /** The update state at start (Phase 5, P-d). */
  updates?: Partial<UpdateStatus>
  /** What an update check finds. */
  nextOffer?: UpdateOffer
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
        line(2, '{variant:2}{range:0.5}(8-12)% Something else unread', {
          status: 'warning',
          code: 'LINE_NO_TEMPLATE',
          shown: '10% Something else unread',
          variants: [2],
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

/** A piece of sample gear for the browser's demo data: name, base, and
 * modifier lines (sample text for layout; nothing is calculated). */
function gear(
  id: number,
  name: string,
  base: string,
  rarity: string,
  itemClass: string,
  mods: string[],
  extra: Partial<ItemCard> = {},
): ItemView {
  return {
    id,
    name,
    base,
    rarity,
    slot: null,
    card: card({ 'item-class': itemClass, 'item-level': '84', ...extra }),
    lines: [
      line(0, `Rarity: ${rarity.toUpperCase()}`, {
        kind: 'rarity',
        section: null,
        status: 'other',
      }),
      line(1, name, { kind: 'name', section: null, status: 'other' }),
      line(2, base, { kind: 'base', section: null, status: 'other' }),
      ...mods.map((text, i) => line(3 + i, text)),
    ],
  } as ItemView
}

/** The demo's gear and the slot each piece sits in. */
function demoGear(): { items: ItemView[]; placed: Record<string, number> } {
  const flask = (id: number, name: string, base: string, mods: string[]) =>
    gear(id, name, base, 'magic', 'UtilityFlask', mods, {
      quality: '20',
      'level-req': '40',
      properties: [
        { name: 'Lasts', value: '6.00 Seconds' },
        { name: 'Consumes', value: '30 of 60 Charges on use' },
        { name: 'Currently has', value: '60 Charges' },
      ],
    } as Partial<ItemCard>)
  const items = [
    gear(101, 'Dread Crown', 'Sample Circlet', 'rare', 'Helmet', [
      '+92 to maximum Life',
      '+41% to Cold Resistance',
      '+38% to Lightning Resistance',
      '+64 to maximum Energy Shield',
    ]),
    gear(
      102,
      'Stormweave',
      'Sample Robe',
      'unique',
      'Body Armour',
      [
        '+118 to maximum Life',
        '+2 to Level of Socketed Gems',
        '35% increased Cold Damage',
        '+44% to Fire Resistance',
        '12% increased Cast Speed',
      ],
      {
        quality: '20',
        flavour: [
          { text: 'The loom remembers every thread', 'default-colour': false },
          { text: 'that the storm pulled loose.', 'default-colour': false },
        ],
      },
    ),
    gear(103, 'Grim Clutch', 'Sample Gloves', 'rare', 'Gloves', [
      '+71 to maximum Life',
      '14% increased Cast Speed',
      '+33% to Fire Resistance',
    ]),
    gear(104, 'Storm Stride', 'Sample Boots', 'rare', 'Boots', [
      '30% increased Movement Speed',
      '+66 to maximum Life',
      '+29% to Chaos Resistance',
    ]),
    gear(105, 'Rune Heart', 'Sample Amulet', 'rare', 'Amulet', [
      '+38% to Global Critical Strike Multiplier',
      '+1 to Level of all Cold Skill Gems',
      '+54 to maximum Life',
    ]),
    gear(106, 'Vortex Cord', 'Sample Belt', 'rare', 'Belt', [
      '+101 to maximum Life',
      '+40% to Cold Resistance',
      '20% increased Flask Charges gained',
    ]),
    flask(107, 'Seething Life Flask of Sealing', 'Sample Life Flask', [
      'Recovers 2080 Life over 3.00 Seconds',
      '66% reduced Amount Recovered',
      'Instant Recovery',
      'Grants Immunity to Bleeding for 17 seconds if used while Bleeding',
      'Grants Immunity to Corrupted Blood for 17 seconds if used while affected by Corrupted Blood',
    ]),
    flask(108, "Alchemist's Quicksilver Flask of the Cheetah", 'Sample Flask', [
      '40% increased Movement Speed',
      '25% increased effect',
      '33% reduced Duration',
      '14% increased Movement Speed during Effect',
      'Used when Charges reach full',
    ]),
    flask(109, "Chemist's Granite Flask of the Armadillo", 'Sample Flask', [
      '+1500 to Armour',
      '26% reduced Charges per use',
      '60% increased Armour during Effect',
      'Used when Charges reach full',
    ]),
    flask(110, "Surgeon's Diamond Flask of the Order", 'Sample Flask', [
      'Your Critical Strike Chance is Lucky',
      '35% chance to gain a Flask Charge when you deal a Critical Strike',
      '60% reduced Effect of Curses on you during Effect',
      'Used when Charges reach full',
    ]),
    flask(111, "Experimenter's Quartz Flask of the Owl", 'Sample Flask', [
      'Phasing',
      '10% chance to Suppress Spell Damage',
      '38% increased Duration',
      '55% reduced Effect of Freeze on you during Effect',
      'Used when Charges reach full',
    ]),
  ]
  return {
    items,
    placed: {
      Helmet: 101,
      'Body Armour': 102,
      Gloves: 103,
      Boots: 104,
      Amulet: 105,
      Belt: 106,
      'Flask 1': 107,
      'Flask 2': 108,
      'Flask 3': 109,
      'Flask 4': 110,
      'Flask 5': 111,
    },
  }
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
  return items
    .flatMap((item) => item.lines)
    .flatMap((l) => l.stats)
    .filter((s) => s.id === 'base_maximum_life')
    .reduce((total, s) => total + Number(s.value), 50)
}

/** An item's lines after an edit (`texts`, `null` for a removed line): a
 * line whose text did not change is kept, any other is a new life line;
 * renumbered. */
function editedLines(lines: LineView[], texts: (string | null)[]): LineView[] {
  const kept = lines.length
  return texts
    .flatMap((text, i) => {
      if (text === null) {
        return []
      }
      const old = lines[i]
      if (i < kept && old !== undefined && old.text === text) {
        return [old]
      }
      return [lifeLine(0, text)]
    })
    .map((l, index) => ({ ...l, index }))
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
          field: 'energy-shield',
          state: 'partial',
          value: 40,
          node: null,
          unsupported: [
            {
              'unresolved-line': {
                source: { item: { item: 1, line: 5 } },
                reason: 'no-template',
              },
            },
            {
              'unmapped-stat': {
                stat: 'some_internal_stat_id',
                source: { item: { item: 1, line: 4 } },
              },
            },
            { 'unmodelled-config': { input: 'conditionFullLife' } },
            { 'unmodelled-config': { input: 'conditionFullLife' } },
          ],
        },
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
  /** The build's own dashboards, as stored. */
  dashboards?: NonNullable<NonNullable<Snapshot['build']>['dashboards']> | null
  secondWeaponSet: boolean
  /** The active passive spec's tree. */
  tree: fakeTree.FakeSpec
  /** The stages (a spec each), in order. */
  stages: Stage[]
  /** Slots filled by `add-item`, over the fixed sets' (slot to item). */
  placed?: Record<string, number>
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

/** A command that is a word (`undo`). */
type WordCommand = Extract<Command, string>
/** A command that is an object with one key, its name. */
type ObjectCommand = Exclude<Command, string>
type KeysOf<T> = T extends unknown ? keyof T : never
type CommandName = KeysOf<ObjectCommand>
/** The body of the object command named `K`. */
type CommandBody<K extends CommandName> = Extract<
  ObjectCommand,
  Record<K, unknown>
>[K]
/** Handlers of object commands, by name. */
type Handlers = { [K in CommandName]?: (body: CommandBody<K>) => Outcome }

/** The application's limits on a build's dashboards, in short: one to
 * eight, each named, one of them shown. */
function dashboardsValid(
  d: NonNullable<CommandBody<'set-dashboards'>['dashboards']>,
): boolean {
  return !(
    d.list.length === 0 ||
    d.list.length > 8 ||
    d.list.some((x) => x.name === '' || x.name.length > 40) ||
    !d.list.some((x) => x.id === d.active)
  )
}

/** A pasted item's line `i`: its rarity, its name, then life lines. */
function pastedLine(i: number, text: string): LineView {
  if (i === 0) {
    return line(i, text, { kind: 'rarity', section: null, status: 'other' })
  }
  if (i === 1) {
    return line(i, text, { kind: 'name', section: null, status: 'other' })
  }
  return lifeLine(i, text)
}

/** The snapshot's open build. */
type BuildView = NonNullable<Snapshot['build']>

/** Where a build's items are. */
interface Places {
  /** The item's slot in the active set: an added item's own slot first,
   * then the set's, unless an added item took it. */
  slotOf: (id: number) => string | null
  /** The tree socket the item sits in. */
  socketOf: (id: number) => number | null
  /** Whether the item is in a slot or a socket. */
  inUse: (id: number) => boolean
}

function placesOf(current: State): Places {
  const placed = current.placed ?? {}
  const socketOf = (id: number) =>
    current.tree.jewels.find((j) => j.item === id)?.node ?? null
  const slotOf = (id: number) => {
    const own = Object.entries(placed).find(([, v]) => v === id)?.[0]
    if (own !== undefined) return own
    const fixed = slotIn(current.set, id)
    return fixed !== null && placed[fixed] === undefined ? fixed : null
  }
  const inUse = (id: number) => slotOf(id) !== null || socketOf(id) !== null
  return { slotOf, socketOf, inUse }
}

/** Every line of the items to review, with whether its item is in use. */
function lineWarnings(
  items: ItemView[],
  places: Places,
): BuildView['warnings'] {
  return items.flatMap((item) =>
    item.lines
      .filter((l) => l.status === 'warning')
      .map((l) => ({
        item: item.id,
        'item-name': item.name,
        line: l.index,
        text: l.text,
        code: l.code ?? '',
        'in-use': places.inUse(item.id),
        shown: l.shown ?? null,
        variants: l.variants,
      })),
  )
}

/** The items' modifier lines, counted by status. */
function lineCounts(items: ItemView[]): BuildView['lines'] {
  const all = items.flatMap((i) => i.lines).filter((l) => l.kind === 'modifier')
  const count = (s: LineView['status']) =>
    all.filter((l) => l.status === s).length
  return {
    modifiers: all.length,
    matched: count('matched'),
    adapted: count('adapted'),
    warnings: count('warning'),
    excluded: count('excluded'),
  }
}

/** The sample's item classes: jewels and added items by their card, the
 * sample's item 2 a wand, its others rings. */
function itemClass(i: ItemView): BuildView['items'][number]['item-class'] {
  if (i.card['item-class'] === 'Jewel') return 'Jewel'
  if (i.id > 100) return i.card['item-class']
  return i.id === 2 ? 'Wand' : 'Ring'
}

/** One item of the build's list. */
function itemSummary(i: ItemView, places: Places): BuildView['items'][number] {
  return {
    id: i.id,
    'item-class': itemClass(i),
    name: i.name,
    base: i.base,
    rarity: i.rarity,
    slot: places.slotOf(i.id),
    warnings: i.lines.filter((l) => l.status === 'warning').length,
    'in-use': places.inUse(i.id),
    sets: SETS.filter((s) => Object.values(s.slots).includes(i.id)).map(
      (s) => s.id,
    ),
    'tree-socket': places.socketOf(i.id),
    legacy:
      i.card['variant-eras'][(i.card['selected-variant'] ?? 0) - 1] ===
      'legacy',
    // The wand: Ice Nova and its support linked, one socket free.
    sockets: i.id === 2 ? ['BG', 'R'] : [],
    requirements: i.card.requirements ?? null,
  }
}

/** The item, skill, spec, and config sets, and the loadouts, each marked
 * active or not. */
function setViews(
  current: State,
): Pick<
  BuildView,
  'item-sets' | 'skill-sets' | 'specs' | 'config-sets' | 'loadouts'
> {
  return {
    'item-sets': SETS.map((s) => ({
      id: s.id,
      title: s.title,
      active: s.id === current.set,
    })),
    'skill-sets': STAGES.map((s) => ({
      id: s.skillSet,
      title: `^4${s.title} {${s.spec}}`,
      active: s.skillSet === current.skillSet,
    })),
    specs: current.stages.map((s) => ({
      id: s.spec,
      title: `${s.title} {${s.spec}}`,
      active: s.spec === current.spec,
    })),
    'config-sets': [{ id: 1, title: 'Default', active: true }],
    loadouts: current.stages.map((s) => ({
      spec: s.spec,
      title: s.title,
      'item-set': s.set,
      'skill-set': s.skillSet,
      'config-set': 1,
      active:
        s.spec === current.spec &&
        s.set === current.set &&
        s.skillSet === current.skillSet,
    })),
  }
}

/** The active skill set's groups, the main group and gem marked. */
function skillGroups(current: State): BuildView['skill-groups'] {
  return (current.skills[current.skillSet] ?? []).map((g, i) => ({
    ...g,
    main: i === current.main,
    gems: g.gems.map((x, j) => ({
      ...x,
      main: i === current.main && j === current.mainGem,
    })),
  }))
}

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
  /** The file the build was imported from. */
  private origin: string | null = null
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
    this.crashList = options.crashes ?? []
    this.updates = { ...this.updates, ...options.updates }
    this.nextOffer = options.nextOffer ?? null
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
      const doc = sampleDocument(this.current.items)
      this.calc = [
        version,
        this.options.demo === true
          ? demoDocument(doc, this.demoInput(this.current))
          : doc,
      ]
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
    return Promise.resolve({
      version: this.version,
      build: this.current === null ? null : this.buildView(this.current),
      calc: this.calcState,
      history: this.historyView(),
      dirty: this.dirty,
      path: this.path,
      origin: this.origin,
      data: this.dataView(),
      notices: [],
    })
  }

  /** The open build, as the snapshot shows it. */
  private buildView(current: State): BuildView {
    const places = placesOf(current)
    const warnings = lineWarnings(current.items, places)
    return {
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
      lines: lineCounts(current.items),
      items: current.items.map((i) => itemSummary(i, places)),
      warnings,
      ...setViews(current),
      'skill-groups': skillGroups(current),
      'passive-names': [{ node: 420, name: 'Heart of Ice' }],
      tree: this.options.data === true ? this.treeView(current) : null,
      config: current.config,
      league: current.league,
      dashboards: current.dashboards ?? null,
      'second-weapon-set': current.secondWeaponSet,
    }
  }

  /** The active spec's tree, locked or offered a conversion while it has
   * no version. */
  private treeView(current: State): BuildView['tree'] {
    const unversioned = current.tree.version === undefined
    return {
      ...fakeTree.view(current.tree, current.spec),
      locked: unversioned ? (this.options.treeLocked ?? null) : null,
      convert: unversioned ? (this.options.treeConvert ?? null) : null,
    }
  }

  private historyView(): Snapshot['history'] {
    return {
      'can-undo': this.undoStack.length > 0,
      'can-redo': this.redoStack.length > 0,
      undo:
        this.undoStack.length > 0 && this.current !== null
          ? (this.made.get(this.current) ?? null)
          : null,
      redo: this.made.get(this.redoStack.at(-1) as State) ?? null,
    }
  }

  /** The loaded game data and the installed bundles. */
  private dataView(): Snapshot['data'] {
    const data = this.options.data === true
    const installed = this.options.installed ?? []
    return {
      bundle: data ? '/data/bundle' : null,
      'data-version': data
        ? {
            game: 'poe1',
            'schema-version': 3,
            'steam-build': this.activeBuild,
          }
        : null,
      // The tree version is known for the real bundle's build only.
      'tree-version': data && this.activeBuild === 26530219 ? '3_29' : null,
      error: null,
      installed: installed.map((b) => this.installedView(b)),
      quarantined: structuredClone(this.options.quarantined ?? []),
      loading: this.options.loading ?? false,
      active:
        data && installed.includes(this.activeBuild)
          ? this.bundleId(this.activeBuild)
          : null,
      notice: null,
      pinned: this.pinnedBundle(installed),
    }
  }

  /** An installed bundle of Steam build `b`. */
  private installedView(
    b: number,
  ): NonNullable<Snapshot['data']['installed']>[number] {
    const accepted = this.acceptedBuilds.includes(b)
    return {
      id: this.bundleId(b),
      'data-version': {
        game: 'poe1',
        'schema-version': 4,
        'steam-build': b,
      },
      'tree-version': b === 26530219 ? '3_29' : null,
      'signed-by': accepted ? null : 'ed25519:6e846054cb5c7102',
      accepted,
    }
  }

  /** The installed bundle the open build is pinned to, when it is not the
   * loaded one. */
  private pinnedBundle(installed: number[]): string | null {
    const pinned = this.options.pinned
    return this.current !== null &&
      pinned !== undefined &&
      pinned !== this.activeBuild &&
      installed.includes(pinned)
      ? this.bundleId(pinned)
      : null
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
    if (edit.expect.some((e) => item.lines[e.line]?.text !== e.text)) {
      this.fail('APP_STALE_EDIT')
    }
    const texts: (string | null)[] = item.lines.map((l) => l.text)
    for (const op of edit.ops) {
      this.applyOp(item, texts, op)
    }
    item.lines = editedLines(item.lines, texts)
    return { ...this.current, items }
  }

  /** One operation of an edit: a line's new text in `texts` (`null` when
   * removed), or the variant selected on `item`. */
  private applyOp(
    item: ItemView,
    texts: (string | null)[],
    op: ItemEdit['ops'][number],
  ) {
    if ('replace' in op) {
      texts[op.replace.line] = op.replace.text
    } else if ('remove' in op) {
      texts[op.remove.line] = null
    } else if ('add' in op) {
      texts.push(op.add.text)
    } else if ('select-variant' in op) {
      this.selectVariant(item, op['select-variant'].variant)
    }
  }

  /** Selects variant `v` of `item`: its card and each variant line's
   * status. */
  private selectVariant(item: ItemView, v: number) {
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
    if (typeof command === 'string') return this.words[command]()
    return (
      this.dispatch(this.commands, command) ??
      this.treeCommand(command) ??
      'nothing'
    )
  }

  /** Runs `command` with its handler in `table`; `null` when the table has
   * none for it. */
  private dispatch(table: Handlers, command: ObjectCommand): Outcome | null {
    const name = Object.keys(command)[0] as CommandName
    if (!Object.hasOwn(table, name)) return null
    // The table pairs each name with its own body's handler.
    const handler = table[name] as (body: unknown) => Outcome
    return handler((command as Record<string, unknown>)[name])
  }

  /** The commands that are words. */
  private readonly words: Record<WordCommand, () => Outcome> = {
    undo: () => this.step(this.undoStack, this.redoStack),
    redo: () => this.step(this.redoStack, this.undoStack),
    save: () => this.saveFile(),
    'export-share-code': () => ({
      text: { version: this.version, text: 'eNrFAKESHARECODE' },
    }),
    close: () => this.resetTo(null),
    'new-build': () => {
      if (this.options.data !== true) this.fail('APP_NO_DATA')
      return this.resetTo({ ...fresh(), items: [], skills: {} })
    },
  }

  /** The object commands but the tree's, by name; a name not here does
   * nothing. */
  private readonly commands: Handlers = {
    'accept-bundle': (c) => this.acceptBundle(c),
    'verify-bundle': (c) => {
      this.verified.push(c.id)
      return this.done()
    },
    'use-bundle': (c) => this.useBundle(c.id),
    'import-text': (c) => this.importText(c.text),
    'edit-item': (c) => this.editItem(c),
    'add-item': (c) => this.addItem(c),
    'use-item-set': (c) => this.useItemSet(c.id),
    'use-loadout': (c) => this.useLoadout(c.spec),
    'use-skill-set': (c) => this.useSkillSet(c.id),
    'edit-gem': (c) => this.editGem(c),
    'edit-socket-group': (c) => this.editSocketGroup(c),
    'add-gem': (c) => this.addGem(c),
    'remove-gem': (c) => this.removeGem(c),
    'add-socket-group': (c) => this.addSocketGroup(c),
    'remove-socket-group': (c) => this.removeSocketGroup(c),
    'use-weapon-set': (c) =>
      this.commit(
        { ...this.state(), secondWeaponSet: c.second },
        { kind: 'weapon-set', item: 1 },
      ),
    'set-dashboards': (c) => this.setDashboards(c.dashboards ?? null),
    'set-league': (c) =>
      this.commit(
        { ...this.state(), league: c.league },
        { kind: 'league', item: 0 },
      ),
    'set-config-input': (c) => this.setConfigInput(c),
    'remove-config-input': (c) =>
      this.editConfig((config) => {
        const at = config.inputs.findIndex((i) => i.name === c.name)
        if (at < 0) this.fail('APP_CONFIG')
        config.inputs.splice(at, 1)
        return 0
      }),
    'edit-custom-block': (c) =>
      this.editConfig((config) => {
        const block = config.blocks[c.block]
        if (block === undefined) this.fail('APP_CONFIG')
        const next = fakeBlock(c.text ?? block.text, c.enabled ?? block.enabled)
        config.blocks[c.block] = { ...next, title: block.title }
        return c.block
      }),
    'add-custom-block': (c) =>
      this.editConfig((config) => {
        config.blocks.push(fakeBlock(c.text))
        return config.blocks.length - 1
      }),
    'remove-custom-block': (c) =>
      this.editConfig((config) => {
        if (config.blocks[c.block] === undefined) this.fail('APP_CONFIG')
        config.blocks.splice(c.block, 1)
        return c.block
      }),
    'open-recent': (c) => this.openRecent(c.id),
    restore: (c) => {
      this.recoveryList = this.recoveryList.filter((r) => r.id !== c.id)
      this.dirty = true
      this.changed(this.fresh())
      return this.done()
    },
    'forget-recent': (c) => {
      if (!this.recentList.some((r) => r.id === c.id))
        this.fail('APP_NOT_FOUND')
      this.recentList = this.recentList.filter((r) => r.id !== c.id)
      return 'nothing'
    },
    discard: (c) => {
      this.recoveryList = this.recoveryList.filter((r) => r.id !== c.id)
      return 'nothing'
    },
    approve: (c) => this.approve(c.id),
    decline: (c) => {
      this.request(c.id).status = 'declined'
      this.emit('mcp-changed')
      return 'nothing'
    },
  }

  /** The open build; `APP_NO_BUILD` without one. */
  private state(): State {
    if (this.current === null) this.fail('APP_NO_BUILD')
    return this.current
  }

  /** Undo or redo: the last state of `from` becomes current, the current
   * one goes onto `to`. */
  private step(from: State[], to: State[]): Outcome {
    const state = from.pop()
    if (state === undefined || this.current === null) {
      return 'nothing'
    }
    to.push(this.current)
    this.dirty = true
    this.changed(state)
    return this.done()
  }

  private saveFile(): Outcome {
    if (this.path === null) {
      this.fail('APP_NO_PATH')
    }
    this.dirty = false
    this.version += 1
    this.emit({ saved: { path: this.path } })
    return this.done()
  }

  /** Opens `state` (`null`: closes the build) with no history, no file,
   * and nothing unsaved. */
  private resetTo(state: State | null): Outcome {
    this.undoStack = []
    this.redoStack = []
    this.path = null
    this.origin = null
    this.dirty = false
    this.changed(state)
    return this.done()
  }

  private acceptBundle(c: CommandBody<'accept-bundle'>): Outcome {
    const { id, 'manifest-sha256': hash } = c
    const q = (this.options.quarantined ?? []).find((b) => b.id === id)
    if (q === undefined) this.fail('APP_NO_BUNDLE')
    if (!q.acceptable || q['manifest-sha256'] !== hash) {
      this.fail('APP_QUARANTINED')
    }
    const build = q['data-version']['steam-build'] ?? 0
    this.options.quarantined = (this.options.quarantined ?? []).filter(
      (b) => b.id !== id,
    )
    this.options.installed = [...(this.options.installed ?? []), build]
    this.acceptedBuilds.push(build)
    return this.done()
  }

  private useBundle(id: string): Outcome {
    const build = (this.options.installed ?? []).find(
      (b) => this.bundleId(b) === id,
    )
    if (build === undefined) this.fail('APP_NO_BUNDLE')
    this.activeBuild = build
    this.options.data = true
    this.version += 1
    this.emit('build-changed')
    return this.done()
  }

  /** An item edit: applied at once (no check that it changes anything). */
  private editItem(edit: ItemEdit): Outcome {
    const next = this.applyEdit(edit)
    if (this.current !== null) {
      this.undoStack.push(this.current)
    }
    this.redoStack = []
    this.dirty = true
    this.changed(next)
    return this.done()
  }

  /** A pasted item, added (and placed in `slot`, if given). */
  private addItem(c: CommandBody<'add-item'>): Outcome {
    const current = this.state()
    const { text, slot } = c
    const lines = text
      .split('\n')
      .map((l) => l.trim())
      .filter(
        (l) => l !== '' && l !== '--------' && !l.startsWith('Item Class:'),
      )
    const rarity = lines[0]?.match(/^Rarity: (\w+)$/)?.[1]?.toLowerCase()
    if (
      rarity === undefined ||
      !['normal', 'magic', 'rare', 'unique'].includes(rarity)
    ) {
      this.fail('APP_ADD_ITEM')
    }
    const named = rarity === 'rare' || rarity === 'unique'
    const id = Math.max(0, ...current.items.map((i) => i.id)) + 1
    const item: ItemView = {
      id,
      name: lines[1] ?? null,
      base: named ? (lines[2] ?? null) : null,
      rarity,
      slot: slot ?? null,
      card: card(),
      lines: lines.map((l, i) => pastedLine(i, l)),
    }
    const placed =
      slot != null ? { ...current.placed, [slot]: id } : current.placed
    return this.commit(
      { ...current, items: [...current.items, item], placed },
      { kind: 'item-added', item: id },
    )
  }

  private useItemSet(id: number): Outcome {
    const current = this.state()
    if (!SETS.some((s) => s.id === id)) {
      this.fail('APP_ITEM_SET')
    }
    return this.commit({ ...current, set: id }, { kind: 'item-set', item: id })
  }

  private useLoadout(spec: number): Outcome {
    const current = this.state()
    const stage = current.stages.find((s) => s.spec === spec)
    if (stage === undefined) this.fail('APP_LOADOUT')
    return this.commit(
      {
        ...current,
        spec: stage.spec,
        set: stage.set,
        skillSet: stage.skillSet,
      },
      { kind: 'loadout', item: stage.spec },
    )
  }

  private useSkillSet(id: number): Outcome {
    const current = this.state()
    if (current.skills[id] === undefined) this.fail('APP_SKILL')
    return this.commit(
      { ...current, skillSet: id },
      { kind: 'skill-set', item: id },
    )
  }

  /** Commits what `f` changes in a copy of the skills (and of the other
   * fields it returns), as an edit of the group it names. */
  private editSkills(
    f: (
      skills: Record<number, Group[]>,
      current: State,
    ) => { item: number; also?: Partial<State> },
  ): Outcome {
    const current = this.state()
    const skills = structuredClone(current.skills)
    const { item, also } = f(skills, current)
    return this.commit(
      { ...current, skills, ...also },
      { kind: 'skill-edit', item },
    )
  }

  private editGem(e: CommandBody<'edit-gem'>): Outcome {
    return this.editSkills((skills, current) => {
      const g = skills[current.skillSet]?.[e.group]?.gems[e.gem]
      if (g === undefined || e.level === 0) this.fail('APP_SKILL')
      if (e.level != null) g.level = e.level
      if (e.quality != null) g.quality = e.quality
      if (e.enabled != null) g.enabled = e.enabled
      return { item: e.group }
    })
  }

  private editSocketGroup(e: CommandBody<'edit-socket-group'>): Outcome {
    return this.editSkills((skills, current) => {
      const g = skills[current.skillSet]?.[e.group]
      if (g === undefined) this.fail('APP_SKILL')
      if (e.enabled != null) g.enabled = e.enabled
      if (e.slot != null) g.slot = e.slot === '' ? null : e.slot
      if (e.label != null) g.label = e.label === '' ? null : e.label
      return {
        item: e.group,
        also: {
          main: e.main === true ? e.group : current.main,
          mainGem: e['main-gem'] ?? current.mainGem,
        },
      }
    })
  }

  private addGem(e: CommandBody<'add-gem'>): Outcome {
    return this.editSkills((skills, current) => {
      const g = skills[current.skillSet]?.[e.group]
      const entry = GEM_CATALOG.find((x) => x.id === e.gem)
      if (g === undefined || entry === undefined) this.fail('APP_SKILL')
      const at = Math.min(e.at ?? g.gems.length, g.gems.length)
      g.gems.splice(
        at,
        0,
        gem(entry.name, 20, entry.colour ?? 'white', entry.support, null, []),
      )
      return { item: e.group }
    })
  }

  private removeGem(e: CommandBody<'remove-gem'>): Outcome {
    return this.editSkills((skills, current) => {
      const g = skills[current.skillSet]?.[e.group]
      if (g === undefined || g.gems[e.gem] === undefined) this.fail('APP_SKILL')
      g.gems.splice(e.gem, 1)
      return { item: e.group }
    })
  }

  private addSocketGroup(e: CommandBody<'add-socket-group'>): Outcome {
    return this.editSkills((skills, current) => {
      const set = skills[current.skillSet] ?? []
      set.push({
        label: e.label ?? null,
        slot: e.slot ?? null,
        enabled: true,
        gems: [],
      })
      skills[current.skillSet] = set
      return { item: set.length - 1 }
    })
  }

  private removeSocketGroup(e: CommandBody<'remove-socket-group'>): Outcome {
    return this.editSkills((skills, current) => {
      const set = skills[current.skillSet]
      if (set?.[e.group] === undefined) this.fail('APP_SKILL')
      set.splice(e.group, 1)
      const main = current.main
      return {
        item: e.group,
        also: {
          main: main > e.group ? main - 1 : Math.min(main, set.length - 1),
        },
      }
    })
  }

  private setDashboards(
    dashboards: CommandBody<'set-dashboards'>['dashboards'] | null,
  ): Outcome {
    const current = this.state()
    if (dashboards != null && !dashboardsValid(dashboards)) {
      this.fail('APP_DASHBOARDS')
    }
    if (
      JSON.stringify(dashboards) === JSON.stringify(current.dashboards ?? null)
    ) {
      return 'nothing'
    }
    return this.commit(
      { ...current, dashboards },
      { kind: 'dashboards', item: 0 },
    )
  }

  /** Commits what `f` changes in a copy of the config, as a config edit of
   * the item it returns. */
  private editConfig(f: (config: ConfigView) => number): Outcome {
    const current = this.state()
    const config = structuredClone(current.config)
    const item = f(config)
    return this.commit({ ...current, config }, { kind: 'config', item })
  }

  private setConfigInput(e: CommandBody<'set-config-input'>): Outcome {
    this.state()
    if (e.name.trim() === '' || e.name === 'customMods') this.fail('APP_CONFIG')
    return this.editConfig((config) => {
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
      return 0
    })
  }

  private openRecent(id: string): Outcome {
    if (!this.recentList.some((r) => r.id === id)) {
      this.fail('APP_NOT_FOUND')
    }
    this.path = 'recent.json'
    this.origin = null
    this.dirty = false
    this.changed(this.fresh())
    return this.done()
  }

  /** Applies an MCP change request: a tree change, or an item edit. */
  private approve(id: number): Outcome {
    const r = this.request(id)
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

  /** A passive tree command, or `null` for another command. */
  private treeCommand(command: Command): Outcome | null {
    if (typeof command !== 'object') return null
    return this.dispatch(this.treeCommands, command)
  }

  /** The passive tree's commands, by name. */
  private readonly treeCommands: Handlers = {
    'allocate-node': (c) =>
      this.editTree(c.spec, (s) => fakeTree.allocate(s, c.node)),
    'deallocate-node': (c) =>
      this.editTree(c.spec, (s) => fakeTree.deallocate(s, c.node)),
    'set-mastery-effect': (c) =>
      this.editTree(c.spec, (s) => fakeTree.setMastery(s, c.node, c.effect)),
    'select-class': (c) =>
      this.editTree(c.spec, (s) => fakeTree.selectClass(s, c.class)),
    'select-ascendancy': (c) =>
      this.editTree(c.spec, (s) => fakeTree.selectAscendancy(s, c.ascendancy)),
    'new-spec': (c) => this.newSpec(c),
    'rename-spec': (c) => this.renameSpec(c),
    'delete-spec': (c) => this.deleteSpec(c),
    'socket-jewel': (c) => this.socketJewel(c),
    'convert-tree-version': (c) => this.convertTree(c.spec),
    'reset-tree': (c) => this.editTree(c.spec, (s) => fakeTree.reset(s)),
  }

  /** Commits `f`'s change to the tree of spec `spec`, the active one. */
  private editTree(
    spec: number,
    f: (s: fakeTree.FakeSpec) => fakeTree.FakeSpec,
  ): Outcome {
    if (this.current === null) this.fail('APP_NO_BUILD')
    if (spec !== this.current.spec) this.fail('APP_TREE')
    if (
      this.options.treeLocked !== undefined &&
      this.current.tree.version === undefined
    )
      this.fail('APP_TREE')
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

  /** Commits the stages `next`, with `active` the active spec. */
  private commitSpecs(
    next: Stage[],
    item: number,
    active = this.current?.spec ?? 1,
  ): Outcome {
    if (this.current === null) this.fail('APP_NO_BUILD')
    return this.commit(
      { ...this.current, stages: next, spec: active },
      { kind: 'tree-specs', item },
    )
  }

  /** A spec's title, trimmed; `APP_TREE` when empty. */
  private specTitle(title: string): string {
    if (title.trim() === '') this.fail('APP_TREE')
    return title.trim()
  }

  private newSpec(c: CommandBody<'new-spec'>): Outcome {
    if (this.current === null) this.fail('APP_NO_BUILD')
    const stages = this.current.stages
    const from =
      c.copy == null ? undefined : stages.find((s) => s.spec === c.copy)
    if (c.copy != null && from === undefined) this.fail('APP_TREE')
    const spec = stages.length + 1
    return this.commitSpecs(
      [
        ...stages,
        {
          spec,
          title: this.specTitle(c.title),
          set: from?.set ?? this.current.set,
          skillSet: from?.skillSet ?? this.current.skillSet,
        },
      ],
      spec,
    )
  }

  private renameSpec(c: CommandBody<'rename-spec'>): Outcome {
    if (this.current === null) this.fail('APP_NO_BUILD')
    if (!this.current.stages.some((s) => s.spec === c.spec))
      this.fail('APP_TREE')
    const title = this.specTitle(c.title)
    return this.commitSpecs(
      this.current.stages.map((s) => (s.spec === c.spec ? { ...s, title } : s)),
      c.spec,
    )
  }

  private deleteSpec(c: CommandBody<'delete-spec'>): Outcome {
    if (this.current === null) this.fail('APP_NO_BUILD')
    const stages = this.current.stages
    if (stages.length <= 1 || !stages.some((s) => s.spec === c.spec))
      this.fail('APP_TREE')
    const active = this.current.spec
    return this.commitSpecs(
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

  private socketJewel(c: CommandBody<'socket-jewel'>): Outcome {
    const item = c.item ?? null
    if (
      item !== null &&
      this.current?.items.find((i) => i.id === item)?.card['item-class'] !==
        'Jewel'
    )
      this.fail('APP_TREE')
    return this.editTree(c.spec, (s) => fakeTree.socketJewel(s, c.node, item))
  }

  private convertTree(spec: number): Outcome {
    const to = this.options.treeConvert
    if (this.current === null) this.fail('APP_NO_BUILD')
    if (
      spec !== this.current.spec ||
      to === undefined ||
      this.current.tree.version !== undefined
    )
      this.fail('APP_TREE')
    const dropped = new Set(to.dropped)
    return this.commit(
      {
        ...this.current,
        tree: {
          ...this.current.tree,
          version: to.version,
          nodes: this.current.tree.nodes.filter((n) => !dropped.has(n)),
        },
      },
      { kind: 'tree-specs', item: spec },
    )
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

  previewTree(spec: number, node: number, fields = true): Promise<TreePreview> {
    try {
      if (this.current === null) this.fail('APP_NO_BUILD')
      if (spec !== this.current.spec) this.fail('APP_TREE')
      const p = fakeTree.preview(
        this.current.tree,
        node,
        life(this.current.items),
      )
      return Promise.resolve(fields ? p : { ...p, fields: [] })
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
    this.origin = 'build.xml'
    this.dirty = true
    this.changed(this.fresh())
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

  /** Crash records (tests set them). */
  crashList: CrashRecord[] = []

  /** How many diagnostic packages were written. */
  diagnosticsExported = 0

  crashes() {
    return Promise.resolve(structuredClone(this.crashList))
  }

  dismissCrash(id: string): Promise<void> {
    const before = this.crashList.length
    this.crashList = this.crashList.filter((c) => c.id !== id)
    return this.crashList.length < before
      ? Promise.resolve()
      : Promise.reject(
          new ApiFailure({ code: 'APP_NOT_FOUND', message: 'APP_NOT_FOUND' }),
        )
  }

  diagnostics(): Promise<DiagnosticFile[]> {
    const about = JSON.stringify({ application: 'Exiled Studio' }, null, 2)
    const log = '{"level":"INFO","message":"start"}\n'
    return Promise.resolve([
      { name: 'about.json', bytes: about.length, text: about },
      { name: 'logs/loom.log', bytes: log.length, text: log },
      ...this.crashList.map((c) => {
        const text = JSON.stringify(c, null, 2)
        return { name: `crashes/${c.id}.json`, bytes: text.length, text }
      }),
    ])
  }

  ready(): Promise<void> {
    return Promise.resolve()
  }

  /** Application updates (tests set the offer and the package). */
  updates: UpdateStatus = {
    mode: 'automatic-check',
    current: '0.0.0',
    artifact: 'nsis',
    installs: true,
    available: true,
    checking: false,
    checked: false,
    offer: null,
    error: null,
  }

  /** What the next check finds. */
  nextOffer: UpdateOffer | null = null

  /** How many installs were started. */
  installs = 0

  private updateListeners: ((s: UpdateStatus) => void)[] = []

  updateStatus(): Promise<UpdateStatus> {
    return Promise.resolve(structuredClone(this.updates))
  }

  setUpdateMode(mode: UpdateMode): Promise<UpdateStatus> {
    this.updates.mode = mode
    if (mode === 'strict-offline') this.updates.offer = null
    return this.updateStatus()
  }

  checkUpdates(): Promise<UpdateStatus> {
    if (this.updates.mode === 'strict-offline') {
      return Promise.reject(
        new ApiFailure({ code: 'UPDATE_OFFLINE', message: 'UPDATE_OFFLINE' }),
      )
    }
    this.updates.checked = true
    this.updates.offer = structuredClone(this.nextOffer)
    return this.updateStatus()
  }

  installUpdate(): Promise<void> {
    const code =
      this.updates.mode === 'strict-offline'
        ? 'UPDATE_OFFLINE'
        : !this.updates.installs
          ? 'UPDATE_NOTIFY_ONLY'
          : this.updates.offer === null
            ? 'UPDATE_NONE'
            : null
    if (code !== null) {
      return Promise.reject(new ApiFailure({ code, message: code }))
    }
    this.installs += 1
    return Promise.resolve()
  }

  /** The automatic game data's state (tests set it). */
  dataAuto: DataAutoStatus = { state: 'idle' }
  /** How often the user asked for the data. */
  fetches = 0
  private dataListeners: ((s: DataAutoStatus) => void)[] = []

  dataAutoStatus(): Promise<DataAutoStatus> {
    return Promise.resolve(structuredClone(this.dataAuto))
  }

  /** As the shell: refused offline; else the feed's packs are installed. */
  async fetchData(): Promise<DataAutoStatus> {
    this.fetches += 1
    if (this.updates.mode === 'strict-offline') {
      this.dataAuto = {
        state: 'failed',
        error: { code: 'UPDATE_OFFLINE', message: 'UPDATE_OFFLINE' },
      }
    } else {
      for (const offer of this.options.feed?.offers ?? []) {
        await this.downloadData(offer.id)
      }
      // Without data loaded, what was installed is taken at once.
      if (this.options.data === false) {
        this.options.data = true
        this.version += 1
        this.emit('build-changed')
      }
      this.dataAuto = { state: 'done' }
    }
    this.emitDataAuto()
    return structuredClone(this.dataAuto)
  }

  onDataAuto(listener: (s: DataAutoStatus) => void): Promise<() => void> {
    this.dataListeners.push(listener)
    return Promise.resolve(() => {
      this.dataListeners = this.dataListeners.filter((l) => l !== listener)
    })
  }

  /** The automatic game data reports (tests). */
  emitDataAuto(status?: DataAutoStatus) {
    if (status !== undefined) this.dataAuto = status
    for (const l of this.dataListeners) l(structuredClone(this.dataAuto))
  }

  onUpdates(listener: (s: UpdateStatus) => void): Promise<() => void> {
    this.updateListeners.push(listener)
    return Promise.resolve(() => {
      this.updateListeners = this.updateListeners.filter((l) => l !== listener)
    })
  }

  /** The check at start reports (tests). */
  emitUpdates() {
    this.updates.checked = true
    this.updates.offer = structuredClone(this.nextOffer)
    const status = structuredClone(this.updates)
    this.updateListeners.forEach((l) => {
      l(status)
    })
  }

  exportDiagnostics(): Promise<Outcome> {
    this.diagnosticsExported += 1
    return Promise.resolve({ done: { version: this.version } } as Outcome)
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
    this.changed(this.fresh())
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

  /** A fresh sample build; with the demo data, in full gear. */
  private fresh(): State {
    const state = fresh()
    if (this.options.demo !== true) return state
    const extra = demoGear()
    return {
      ...state,
      items: [...state.items, ...extra.items],
      placed: extra.placed,
    }
  }

  /** What the sample numbers follow (`fakeDemo`). */
  private demoInput(state: State): DemoInput {
    const group = state.skills[state.skillSet]?.[state.main]
    const enemy = state.config.inputs.find((i) => i.name === 'enemyLevel')
    return {
      life: life(state.items),
      stage: Math.max(
        0,
        state.stages.findIndex((s) => s.spec === state.spec),
      ),
      gemLevel: group?.gems[state.mainGem]?.level ?? 1,
      level: 90,
      enemyLevel:
        enemy != null &&
        typeof enemy.value === 'object' &&
        'number' in enemy.value
          ? enemy.value.number
          : 83,
    }
  }

  /** Another build file's numbers (the sample's, with a stronger
   * character); `null` when the fake's picker is told to cancel. */
  compareFile(): Promise<BuildFields | null> {
    if (this.options.data !== true) this.fail('APP_NO_DATA')
    if (this.options.cancelPick === true) return Promise.resolve(null)
    const base: DemoInput =
      this.current === null
        ? { life: 50, stage: 0, gemLevel: 20, level: 90, enemyLevel: 83 }
        : this.demoInput(this.current)
    return Promise.resolve({
      name: 'Other build',
      fields: demoValues({
        ...base,
        level: Math.min(100, base.level + 5),
        gemLevel: base.gemLevel + 1,
      }).map(([field, value]) => ({ field, value, state: 'complete' })),
    })
  }

  /** A stage's tree and equipment: the active stage's as they are;
   * another stage's item set, and the tree with the fixed difference
   * the sample build has between its stages. */
  /** The sample passive's lines, each with the stat it prints. */
  passiveLines(nodes: number[]): Promise<PassiveLines[]> {
    return Promise.resolve(
      nodes
        .filter((node) => node === 420)
        .map((node) => ({
          node,
          name: 'Heart of Ice',
          lines: [
            {
              text: '10% increased Cold Damage',
              stats: ['cold_damage_+%'],
            },
            {
              text: '5% chance to Freeze',
              stats: ['base_chance_to_freeze_%'],
            },
          ],
        })),
    )
  }

  stageView(spec: number): Promise<StageView> {
    const state = this.current
    if (state === null) this.fail('APP_NO_BUILD')
    const stage = state.stages.find((s) => s.spec === spec)
    if (stage === undefined) this.fail('APP_LOADOUT')
    const active = spec === state.spec
    const placed = active ? (state.placed ?? {}) : {}
    const slots = {
      ...(SETS.find((s) => s.id === (active ? state.set : stage.set))?.slots ??
        {}),
      ...placed,
    }
    const nodes = active
      ? state.tree.nodes
      : [...new Set([...state.tree.nodes.filter((n) => n !== 20), 10, 11, 12])]
    return Promise.resolve({
      spec,
      title: stage.title,
      nodes: [...nodes].sort((a, b) => a - b),
      slots: Object.entries(slots).map(([slot, item]) => ({ slot, item })),
    })
  }

  /** The what-if query over the sample numbers: the fields at each stage,
   * or with one input swept (never a change to the build). */
  whatIf(request: WhatIfRequest): Promise<WhatIfSeries> {
    const state = this.current
    if (state === null) this.fail('APP_NO_BUILD')
    const now = this.demoInput(state)
    const point = (
      label: string,
      x: number,
      current: boolean,
      input: DemoInput,
    ) => ({
      label,
      x,
      current,
      fields: demoValues(input).map(([field, value]) => ({
        field,
        value,
        state: 'complete',
      })),
    })
    const vary = request.vary
    const span = (from: number, to: number, step: number) => {
      const n = Math.floor((to - from) / step) + 1
      if (!(step > 0) || from > to || n > 41) this.fail('APP_WHAT_IF')
      return Array.from({ length: n }, (_, i) => from + i * step)
    }
    if (vary === 'stages') {
      return Promise.resolve({
        points: state.stages.map((s, i) =>
          point(s.title, i, i === now.stage, { ...now, stage: i }),
        ),
      })
    }
    if ('gem-level' in vary) {
      const v = vary['gem-level']
      const gem = state.skills[state.skillSet]?.[v.group]?.gems[v.gem]
      if (gem === undefined) this.fail('APP_WHAT_IF')
      const main = v.group === state.main && v.gem === state.mainGem
      return Promise.resolve({
        points: span(v.from, v.to, 1).map((level) =>
          point(
            String(level),
            level,
            level === gem.level,
            main ? { ...now, gemLevel: level } : now,
          ),
        ),
      })
    }
    if ('character-level' in vary) {
      const v = vary['character-level']
      return Promise.resolve({
        points: span(v.from, v.to, v.step).map((level) =>
          point(String(level), level, level === now.level, { ...now, level }),
        ),
      })
    }
    const v = vary['config-number']
    const input = state.config.inputs.find((i) => i.name === v.name)
    if (
      input === undefined ||
      typeof input.value !== 'object' ||
      !('number' in input.value) ||
      v.steps < 2 ||
      v.steps > 41 ||
      v.from > v.to
    ) {
      this.fail('APP_WHAT_IF')
    }
    const held = input.value.number
    return Promise.resolve({
      points: Array.from({ length: v.steps }, (_, i) => {
        const x = v.from + ((v.to - v.from) * i) / (v.steps - 1)
        return point(
          String(Math.round(x * 100) / 100),
          x,
          x === held,
          v.name === 'enemyLevel' ? { ...now, enemyLevel: x } : now,
        )
      }),
    })
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

  /** Builds the user accepted from quarantine. */
  acceptedBuilds: number[] = []

  /** Bundles the user verified. */
  verified: string[] = []

  checkDataFeed(): Promise<DataFeedStatus> {
    if (this.updates.mode === 'strict-offline') {
      return Promise.reject(
        new ApiFailure({ code: 'UPDATE_OFFLINE', message: 'UPDATE_OFFLINE' }),
      )
    }
    const feed = this.options.feed ?? { sequence: 1, offers: [] }
    return Promise.resolve(structuredClone(feed))
  }

  /** Packs downloaded from the feed. */
  downloads: string[] = []

  downloadData(id: string): Promise<Outcome> {
    const offer = this.options.feed?.offers.find((o) => o.id === id)
    if (offer === undefined) {
      return Promise.reject(
        new ApiFailure({ code: 'APP_NOT_FOUND', message: 'APP_NOT_FOUND' }),
      )
    }
    const build = Number(/\/(\d+)-s/.exec(id)?.[1] ?? '0')
    this.options.installed = [...(this.options.installed ?? []), build]
    offer.installed = true
    this.downloads.push(id)
    this.version += 1
    this.emit('build-changed')
    return Promise.resolve(this.done())
  }

  installBundle(): Promise<Outcome> {
    const pack = this.options.nextPack
    if (pack === undefined) return Promise.resolve('nothing' as Outcome)
    if (pack.signed) {
      this.options.installed = [...(this.options.installed ?? []), pack.build]
    } else {
      this.options.quarantined = [
        ...(this.options.quarantined ?? []),
        {
          id: this.bundleId(pack.build),
          'data-version': {
            game: 'poe1',
            'schema-version': 4,
            'steam-build': pack.build,
          },
          reason: 'unsigned',
          acceptable: true,
          'manifest-sha256': 'ab'.repeat(32),
          'key-id': null,
          files: 41,
          bytes: 123456,
        },
      ]
    }
    this.version += 1
    this.emit('build-changed')
    return Promise.resolve(this.done())
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
