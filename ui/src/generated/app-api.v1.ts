// Generated from schemas/app-api.v1.schema.json by pnpm generate:schema; do not edit.

/**
 * One line operation of an item edit.
 */
export type LineOp =
  | {
      replace: {
        /**
         * The line's index.
         */
        line: number
        /**
         * The new text.
         */
        text: string
      }
    }
  | {
      remove: {
        /**
         * The line's index.
         */
        line: number
      }
    }
  | {
      'set-range': {
        /**
         * The line's index.
         */
        line: number
        /**
         * The roll, from 0 to 1.
         */
        value: number
      }
    }
  | {
      'select-variant': {
        /**
         * The variant.
         */
        variant: number
      }
    }
  | {
      add: {
        /**
         * The text.
         */
        text: string
      }
    }
/**
 * What a variant of an item is.
 */
export type VariantEra = 'legacy' | 'current' | 'choice'

/**
 * Every top-level type of the API, so one schema names them all. Never
 * sent.
 */
export interface ApplicationAPIV1 {
  /**
   * A command.
   */
  command:
    | 'new-build'
    | {
        'import-text': {
          /**
           * The text.
           */
          text: string
        }
      }
    | {
        'edit-item': ItemEdit
      }
    | {
        'use-item-set': {
          /**
           * The set's ID.
           */
          id: number
        }
      }
    | {
        'use-loadout': {
          /**
           * The passive spec's ID.
           */
          spec: number
        }
      }
    | {
        'use-skill-set': {
          /**
           * The set's ID.
           */
          id: number
        }
      }
    | {
        'edit-gem': {
          /**
           * Enables or disables the gem.
           */
          enabled?: boolean | null
          /**
           * The gem's index in the group.
           */
          gem: number
          /**
           * The socket group's index in the active skill set.
           */
          group: number
          /**
           * A new level.
           */
          level?: number | null
          /**
           * A new quality.
           */
          quality?: number | null
        }
      }
    | {
        'edit-socket-group': {
          /**
           * Enables or disables the group.
           */
          enabled?: boolean | null
          /**
           * The group's index in the active skill set.
           */
          group: number
          /**
           * Renames the group; empty for no label.
           */
          label?: string | null
          /**
           * Makes it the main skill group (`true` only).
           */
          main?: boolean | null
          /**
           * Makes this gem (by index in the group) the group's main active
           * skill; with `main`, the build's main skill.
           */
          'main-gem'?: number | null
          /**
           * Moves the group to this equipment slot (empty: no slot).
           */
          slot?: string | null
        }
      }
    | {
        'add-gem': {
          /**
           * Its place in the group (by index); the end when absent.
           */
          at?: number | null
          /**
           * The gem, by its gem effect ID (a [`GemCatalogEntry`]'s `id`).
           */
          gem: string
          /**
           * The group's index in the active skill set.
           */
          group: number
        }
      }
    | {
        'remove-gem': {
          /**
           * The gem's index in the group.
           */
          gem: number
          /**
           * The group's index in the active skill set.
           */
          group: number
        }
      }
    | {
        'add-socket-group': {
          /**
           * Its label, if any.
           */
          label?: string | null
          /**
           * The equipment slot it is socketed in, if any.
           */
          slot?: string | null
        }
      }
    | {
        'remove-socket-group': {
          /**
           * The group's index in the active skill set.
           */
          group: number
        }
      }
    | {
        'use-config-set': {
          /**
           * The set's ID.
           */
          id: number
        }
      }
    | {
        'set-config-input': {
          /**
           * The name (PoB's option key).
           */
          name: string
          /**
           * The value.
           */
          value:
            | {
                boolean: boolean
              }
            | {
                number: number
              }
            | {
                text: string
              }
        }
      }
    | {
        'remove-config-input': {
          /**
           * The name.
           */
          name: string
        }
      }
    | {
        'edit-custom-block': {
          /**
           * The block's index.
           */
          block: number
          /**
           * Whether it applies, if changed.
           */
          enabled?: boolean | null
          /**
           * The new text, if changed.
           */
          text?: string | null
        }
      }
    | {
        'add-custom-block': {
          /**
           * The text.
           */
          text: string
        }
      }
    | {
        'remove-custom-block': {
          /**
           * The block's index.
           */
          block: number
        }
      }
    | {
        'use-weapon-set': {
          /**
           * The second weapon set (`II`) when set.
           */
          second: boolean
        }
      }
    | {
        'set-league': {
          /**
           * The league.
           */
          league: 'challenge' | 'standard'
        }
      }
    | 'undo'
    | 'redo'
    | 'save'
    | {
        'open-recent': {
          /**
           * The ID.
           */
          id: string
        }
      }
    | {
        'forget-recent': {
          /**
           * The ID.
           */
          id: string
        }
      }
    | {
        restore: {
          /**
           * The snapshot's ID.
           */
          id: string
        }
      }
    | {
        discard: {
          /**
           * The snapshot's ID.
           */
          id: string
        }
      }
    | {
        approve: {
          /**
           * The request's ID.
           */
          id: number
        }
      }
    | {
        decline: {
          /**
           * The request's ID.
           */
          id: number
        }
      }
    | 'export-share-code'
    | 'close'
  error: ApiError
  event: Event
  /**
   * The gems the game data offers.
   */
  gems: GemCatalogEntry[]
  item: ItemView
  mcp: McpStatus
  /**
   * A command's answer.
   */
  outcome:
    | {
        done: {
          /**
           * The version.
           */
          version: number
        }
      }
    | {
        text: {
          /**
           * The text.
           */
          text: string
          /**
           * The version.
           */
          version: number
        }
      }
    | 'nothing'
  preview: ItemEditPreview1
  /**
   * Recent builds.
   */
  recent: RecentEntry[]
  /**
   * Recovery snapshots found at start.
   */
  recovery: RecoveryEntry[]
  snapshot: Snapshot
}
/**
 * An item edit: line operations on one item. It is refused as stale
 * (`APP_STALE_EDIT`) when the item's lines it names are not the ones
 * it expects.
 */
export interface ItemEdit {
  /**
   * The text each line the operations name had when the edit was
   * made, by line index.
   */
  expect: ExpectedLine[]
  /**
   * The item's ID.
   */
  item: number
  /**
   * The operations, in order; line indices are the item's before
   * the edit.
   */
  ops: LineOp[]
}
/**
 * A line's text an edit expects.
 */
export interface ExpectedLine {
  /**
   * The line's index.
   */
  line: number
  /**
   * Its text.
   */
  text: string
}
/**
 * An error.
 */
export interface ApiError {
  /**
   * The code of the layer that failed (`IMPORT_*`, `PERSIST_*`,
   * `APP_*`, ...).
   */
  code: string
  /**
   * Detail in English, for diagnostics.
   */
  message: string
}
/**
 * An event.
 */
export interface Event {
  /**
   * The command that caused it, if any.
   */
  correlation?: number | null
  /**
   * What happened.
   */
  kind:
    | 'build-changed'
    | {
        saved: {
          /**
           * Where.
           */
          path: string
        }
      }
    | 'calc-started'
    | {
        'calc-finished': {
          /**
           * The result's overall state.
           */
          state: string
        }
      }
    | {
        'calc-failed': {
          error: ApiError1
        }
      }
    | 'mcp-changed'
    | {
        failed: {
          error: ApiError2
        }
      }
  /**
   * The snapshot version after the event.
   */
  version: number
}
/**
 * Why.
 */
export interface ApiError1 {
  /**
   * The code of the layer that failed (`IMPORT_*`, `PERSIST_*`,
   * `APP_*`, ...).
   */
  code: string
  /**
   * Detail in English, for diagnostics.
   */
  message: string
}
/**
 * Why.
 */
export interface ApiError2 {
  /**
   * The code of the layer that failed (`IMPORT_*`, `PERSIST_*`,
   * `APP_*`, ...).
   */
  code: string
  /**
   * Detail in English, for diagnostics.
   */
  message: string
}
/**
 * A gem the game data offers, for a gem picker.
 */
export interface GemCatalogEntry {
  /**
   * Its colour (red, green, blue, or white), if known.
   */
  colour?: string | null
  /**
   * For a support, when a group was named: whether it would support
   * an active gem of that group.
   */
  fits?: boolean | null
  /**
   * The gem effect ID (the `gem` of [`Command::AddGem`]).
   */
  id: string
  /**
   * The name.
   */
  name: string
  /**
   * Whether it is a support gem.
   */
  support: boolean
  /**
   * Its tag names (Spell, Area, Cold, ...).
   */
  tags: string[]
}
/**
 * One item.
 */
export interface ItemView {
  /**
   * The base, if written.
   */
  base?: string | null
  card: ItemCard
  /**
   * The item's ID.
   */
  id: number
  /**
   * Every source line, in order.
   */
  lines: LineView[]
  /**
   * The name, if any.
   */
  name?: string | null
  /**
   * The rarity, if read.
   */
  rarity?: string | null
  /**
   * The slot it is in, if any.
   */
  slot?: string | null
}
/**
 * What an item card shows (UX walkthrough, UX-08).
 */
export interface ItemCard {
  /**
   * The item's mods and the lines each made, implicits first, then
   * prefixes and suffixes, crafted last in each (the tooltip's Alt
   * view headers): the mods PoB names, and those the lines allow
   * one reading of; lines no mod is found for are in none.
   */
  affixes: AffixView[]
  /**
   * When the selected variant is a legacy version: the variant the
   * current game makes instead, if one matches.
   */
  'current-variant'?: number | null
  /**
   * Flag lines (`Corrupted`, `Mirrored`), as written.
   */
  flags: string[]
  /**
   * The influences, by name.
   */
  influences: string[]
  /**
   * The item class, if written.
   */
  'item-class'?: string | null
  /**
   * The item level, as written.
   */
  'item-level'?: string | null
  /**
   * The level the item requires, as written.
   */
  'level-req'?: string | null
  /**
   * The defences and other base values written on the item
   * (`Energy Shield: 276`), in order.
   */
  properties: CardProperty[]
  /**
   * The quality, as written (`20`).
   */
  quality?: string | null
  /**
   * What the item requires, when known.
   */
  requirements?: Requirements | null
  /**
   * The selected variant (1-based), if the item names one.
   */
  'selected-variant'?: number | null
  /**
   * The sockets, one string of colour letters per link group.
   */
  sockets: string[]
  /**
   * What each variant is, in the order of `variants`.
   */
  'variant-eras': VariantEra[]
  /**
   * The variants' names, in order (variant 1 first).
   */
  variants: string[]
}
/**
 * One mod of an item, as the tooltip's Alt view heads its lines
 * (`Prefix Modifier "Urchin's" (Tier: 2) — Life, Defences`).
 */
export interface AffixView {
  /**
   * Whether it is crafted (`Master Crafted Prefix Modifier`).
   */
  crafted: boolean
  /**
   * Whether it was inferred from the lines (the only reading the
   * game data allows) rather than named by the item.
   */
  inferred: boolean
  /**
   * Prefix or suffix.
   */
  kind: 'implicit' | 'prefix' | 'suffix'
  /**
   * The line indices it made, in item order.
   */
  lines: number[]
  /**
   * The affix name, when the game data has one.
   */
  name?: string | null
  /**
   * Its tags as the game prints them (`Life`, `Defences`).
   */
  tags: string[]
  /**
   * Its tier on the item's base (1 is the best), when known.
   */
  tier?: number | null
}
/**
 * One base value on an item card.
 */
export interface CardProperty {
  /**
   * The name (`Energy Shield`).
   */
  name: string
  /**
   * The value, as written.
   */
  value: string
}
/**
 * An item's requirements: the level, and attributes above 0.
 */
export interface Requirements {
  /**
   * Dexterity (0: none).
   */
  dexterity: number
  /**
   * Intelligence (0: none).
   */
  intelligence: number
  /**
   * The level: as written on the item, else the base's drop level.
   */
  level?: number | null
  /**
   * Strength (0: none).
   */
  strength: number
}
/**
 * One source line of an item.
 */
export interface LineView {
  /**
   * The tolerance it needed (`case-folded`, `split-section`).
   */
  adapted?: string | null
  /**
   * The warning's code (`LINE_NO_TEMPLATE`, ...), when the status is
   * `warning`.
   */
  code?: string | null
  /**
   * The line's index among the item's lines.
   */
  index: number
  /**
   * What it is: the domain's line kind name (`modifier`, `spec`,
   * `name`, ...).
   */
  kind: string
  /**
   * The modifier's marks from its prefixes: `crafted`,
   * `fractured`, `crucible`, `scourge`, `synthesis`, `mutated`.
   */
  marks: string[]
  /**
   * The numbers in the text, in order, for roll edits.
   */
  numbers: NumberSpan[]
  /**
   * The line as the game prints it with Alt held: each roll followed
   * by its range (`+28(24-28) to maximum Life`); `None` when the
   * line has no range.
   */
  ranged?: string | null
  /**
   * The modifier list it belongs to (`implicit`, `explicit`, ...);
   * modifier lines only.
   */
  section?: string | null
  /**
   * A modifier line as the game prints it, its roll applied (no
   * markup); `None` when it is shown as written.
   */
  shown?: string | null
  /**
   * What the line resolved to: game stats, or a typed kind.
   */
  stats: StatView[]
  /**
   * Its status.
   */
  status: 'matched' | 'adapted' | 'warning' | 'excluded' | 'continues' | 'other'
  /**
   * The line as written.
   */
  text: string
  /**
   * The typed kind (`granted-skill`, ...), if it resolved to one.
   */
  typed?: string | null
  /**
   * The variants (1-based) a modifier line belongs to; empty for
   * every variant.
   */
  variants: number[]
}
/**
 * A number in a line's text: where it is, as character offsets.
 */
export interface NumberSpan {
  /**
   * The offset after the last character.
   */
  end: number
  /**
   * The first character's offset.
   */
  start: number
  /**
   * The number as written.
   */
  text: string
}
/**
 * One stat a line resolved to.
 */
export interface StatView {
  /**
   * The game's stat ID.
   */
  id: string
  /**
   * The value as text (`15`, `10 to 20`, `hidden`).
   */
  value: string
}
/**
 * The MCP server's state.
 */
export interface McpStatus {
  permissions: McpPermissions
  /**
   * Change requests, newest last.
   */
  requests: ChangeRequest[]
  /**
   * Whether it runs.
   */
  running: boolean
  /**
   * The session's bearer token, while it runs.
   */
  token?: string | null
  /**
   * Its address, while it runs (`http://127.0.0.1:<port>/mcp`).
   */
  url?: string | null
}
/**
 * What clients may do.
 */
export interface McpPermissions {
  /**
   * Clients may request changes (off by default: read-only).
   */
  requests: boolean
  /**
   * Requests apply without asking (a standing grant the user can
   * revoke).
   */
  'standing-grant': boolean
}
/**
 * A change an MCP client asked for.
 */
export interface ChangeRequest {
  /**
   * The client's name, as it gave it.
   */
  client: string
  edit: ItemEdit1
  /**
   * Why it failed, if it did.
   */
  error?: ApiError3 | null
  /**
   * Its ID, unique in the session.
   */
  id: number
  /**
   * The client's note, if any.
   */
  note?: string | null
  /**
   * Its effect, calculated when it arrived.
   */
  preview?: ItemEditPreview | null
  /**
   * Where it is.
   */
  status: 'pending' | 'applied' | 'declined' | 'failed'
}
/**
 * An item edit: line operations on one item. It is refused as stale
 * (`APP_STALE_EDIT`) when the item's lines it names are not the ones
 * it expects.
 */
export interface ItemEdit1 {
  /**
   * The text each line the operations name had when the edit was
   * made, by line index.
   */
  expect: ExpectedLine[]
  /**
   * The item's ID.
   */
  item: number
  /**
   * The operations, in order; line indices are the item's before
   * the edit.
   */
  ops: LineOp[]
}
/**
 * An error: a stable code (ADR-014) and a message for logs. The UI
 * shows text for the code, not the message.
 */
export interface ApiError3 {
  /**
   * The code of the layer that failed (`IMPORT_*`, `PERSIST_*`,
   * `APP_*`, ...).
   */
  code: string
  /**
   * Detail in English, for diagnostics.
   */
  message: string
}
/**
 * An item edit's effect, calculated by the application (never by the
 * requester).
 */
export interface ItemEditPreview {
  /**
   * Each slice field before and after.
   */
  fields: FieldDelta[]
  item: ItemView1
}
/**
 * One field before and after an edit.
 */
export interface FieldDelta {
  /**
   * Its value after, when computed and finite.
   */
  after?: number | null
  /**
   * Its value before, when computed and finite.
   */
  before?: number | null
  /**
   * The field's name.
   */
  field: string
  /**
   * Its state after.
   */
  'state-after': string
  /**
   * Its state before.
   */
  'state-before': string
}
/**
 * The item as edited, with its lines' new statuses.
 */
export interface ItemView1 {
  /**
   * The base, if written.
   */
  base?: string | null
  card: ItemCard
  /**
   * The item's ID.
   */
  id: number
  /**
   * Every source line, in order.
   */
  lines: LineView[]
  /**
   * The name, if any.
   */
  name?: string | null
  /**
   * The rarity, if read.
   */
  rarity?: string | null
  /**
   * The slot it is in, if any.
   */
  slot?: string | null
}
/**
 * An item edit's effect, calculated by the application (never by the
 * requester).
 */
export interface ItemEditPreview1 {
  /**
   * Each slice field before and after.
   */
  fields: FieldDelta[]
  item: ItemView1
}
/**
 * A recent build.
 */
export interface RecentEntry {
  /**
   * The ascendancy, if known.
   */
  ascendancy?: string | null
  /**
   * When it was last opened or saved, in seconds since the Unix
   * epoch.
   */
  at: number
  /**
   * The class, if known (UX-20; lists written before it have none).
   */
  class?: string | null
  /**
   * The ID the application issued for it (the UI opens a recent
   * build by it and never sends a path).
   */
  id: string
  /**
   * The character level, if known.
   */
  level?: number | null
  /**
   * The main skill, if known.
   */
  'main-skill'?: string | null
  /**
   * The display name.
   */
  name: string
  /**
   * The build document's path.
   */
  path: string
}
/**
 * A recovery snapshot (autosave) found at start.
 */
export interface RecoveryEntry {
  /**
   * Its ID (the file's stem).
   */
  id: string
  /**
   * The build's display name.
   */
  name: string
  /**
   * The file it was imported from, if any.
   */
  origin?: string | null
  /**
   * Where the build was saved, if it was.
   */
  path?: string | null
  /**
   * When it was written, in seconds since the Unix epoch.
   */
  'saved-at': number
}
/**
 * The snapshot query's answer.
 */
export interface Snapshot {
  /**
   * The open build, if any.
   */
  build?: BuildView | null
  calc: CalcStatus
  data: DataStatus
  /**
   * Whether the build changed since it was last saved or opened.
   */
  dirty: boolean
  history: HistoryView
  /**
   * Notices about the open build that are not errors (for example
   * `APP_RESOLUTION_CHANGED`: the build's stored resolutions differ
   * from this bundle's, so they stand and rolled values stay
   * unsettled).
   */
  notices: ApiError3[]
  /**
   * The file the build was imported from, if any; never written.
   */
  origin?: string | null
  /**
   * Where the build is saved (a build document), if it is.
   */
  path?: string | null
  /**
   * Increases with every change of the build (import, open, edit,
   * undo, redo); 0 before any build.
   */
  version: number
}
/**
 * The open build.
 */
export interface BuildView {
  /**
   * The ascendancy, if set.
   */
  ascendancy?: string | null
  /**
   * The class, if set.
   */
  class?: string | null
  /**
   * `complete` or `partial`: whether anything was not read.
   */
  completeness: string
  /**
   * The active config set's inputs and custom modifier blocks, if
   * the build has a config set.
   */
  config?: ConfigView | null
  /**
   * The config sets, in build order.
   */
  'config-sets': SetView[]
  /**
   * The data version the build is pinned to.
   */
  'data-version'?: DataVersionView | null
  /**
   * `poe1` or `poe2`.
   */
  game: string
  /**
   * `complete` or `partial` counting only what the calculation
   * reads: lines of items in use ([`ItemSummary::in_use`]) and the
   * custom modifier lines. A warning on an item of another item set
   * leaves it `complete` (UX walkthrough, UX-03).
   */
  'in-use-completeness': string
  /**
   * The item sets, in build order.
   */
  'item-sets': SetView[]
  /**
   * The items, in build order.
   */
  items: ItemSummary[]
  /**
   * The league the build is planned for (`challenge` when not
   * chosen).
   */
  league: 'challenge' | 'standard'
  /**
   * The character level, if set.
   */
  level?: number | null
  lines: LineCounts
  /**
   * The build's stages (PoB's loadouts: one per passive spec, with
   * the item, skill, and config sets linked to it), in spec order.
   */
  loadouts: LoadoutView[]
  /**
   * The main skill's gem names, if the build has a main group.
   */
  'main-skill'?: string | null
  /**
   * A display name: the file's stem, else "imported build" in the
   * UI's language (empty here).
   */
  name: string
  /**
   * The names of the active passive spec's allocated nodes, when the
   * loaded game data has them: a calc result's `passive` source
   * names its node by number.
   */
  'passive-names': NodeName[]
  /**
   * Whether the active item set uses its second weapon set (the
   * game's weapon swap, `II`).
   */
  'second-weapon-set': boolean
  /**
   * The active skill set's socket groups, in order: a calc result's
   * `gem` source names a group and a gem by their index here.
   */
  'skill-groups': SkillGroupView[]
  /**
   * The skill sets, in build order.
   */
  'skill-sets': SetView[]
  /**
   * The passive tree specs, in build order.
   */
  specs: SetView[]
  /**
   * Lines with warnings: kept, shown, not calculated (ADR-015).
   */
  warnings: WarningView[]
}
/**
 * The active config set: its inputs and custom modifier blocks.
 */
export interface ConfigView {
  /**
   * Its custom modifier blocks, in order (the index is the key).
   */
  blocks: CustomBlockView[]
  /**
   * Its inputs, in build order (the name is the key).
   */
  inputs: ConfigInputView[]
  /**
   * The set's ID.
   */
  set: number
}
/**
 * One custom modifier block.
 */
export interface CustomBlockView {
  /**
   * Whether it applies.
   */
  enabled: boolean
  /**
   * The modifier lines read from the text, in order.
   */
  lines: CustomLineView[]
  /**
   * The text as written.
   */
  text: string
  /**
   * The title, if any.
   */
  title?: string | null
}
/**
 * One line of a custom modifier block.
 */
export interface CustomLineView {
  /**
   * Its status: `warning` when it did not resolve (kept, not
   * calculated, ADR-015).
   */
  status: 'matched' | 'adapted' | 'warning' | 'excluded' | 'continues' | 'other'
  /**
   * The text.
   */
  text: string
  /**
   * The typed kind it resolved to, if any (`custom-note`: a label or
   * comment with no effect; `custom-pattern`: read as PoB reads it).
   */
  typed?: string | null
}
/**
 * One config input.
 */
export interface ConfigInputView {
  /**
   * What it does to the computed fields.
   */
  effect: 'applied' | 'no-effect' | 'unmodelled'
  /**
   * The name, as the build's format writes it (PoB's option key).
   */
  name: string
  /**
   * Whether the value is a placeholder: the value a planner shows
   * for an input left unset.
   */
  placeholder: boolean
  /**
   * The value.
   */
  value:
    | {
        boolean: boolean
      }
    | {
        number: number
      }
    | {
        text: string
      }
}
/**
 * One item set, skill set, passive spec, or config set.
 */
export interface SetView {
  /**
   * Whether the calculation uses it.
   */
  active: boolean
  /**
   * The set's ID.
   */
  id: number
  /**
   * The title as written, if any (PoB's colour codes and `{1,2}`
   * loadout links included).
   */
  title?: string | null
}
/**
 * A data version.
 */
export interface DataVersionView {
  /**
   * `poe1` or `poe2`.
   */
  game: string
  /**
   * The bundle's data schema version.
   */
  'schema-version': number
  /**
   * The Steam build it was extracted from, if known.
   */
  'steam-build'?: number | null
}
/**
 * One item in a list.
 */
export interface ItemSummary {
  /**
   * The base, if written.
   */
  base?: string | null
  /**
   * The item's ID.
   */
  id: number
  /**
   * Whether the calculation reads it: in a slot of the active item
   * set that is in use, or in an allocated jewel socket of the
   * active passive spec.
   */
  'in-use': boolean
  /**
   * Its base's item class in the game data (`Bow`, `Quiver`, `Two
   * Hand Sword`, `LifeFlask`, ...), when the base is known.
   */
  'item-class'?: string | null
  /**
   * Whether its selected variant is a legacy version of a unique
   * (`Pre 3.7.0`): only Standard has such items.
   */
  legacy: boolean
  /**
   * The name, if any.
   */
  name?: string | null
  /**
   * `normal`, `magic`, `rare`, `unique`, `relic`, if read.
   */
  rarity?: string | null
  /**
   * The item sets that hold it, by ID.
   */
  sets: number[]
  /**
   * The slot it is in, in the active item set, if any.
   */
  slot?: string | null
  /**
   * Its sockets as linked groups, each a string of colours (`R`,
   * `G`, `B`, `W`, `A` abyssal) in socket order: `R-R-G B` is
   * `["RRG", "B"]`. Empty without a `Sockets` line.
   */
  sockets: string[]
  /**
   * The allocated jewel socket of the active spec it is in, by node
   * number, if any.
   */
  'tree-socket'?: number | null
  /**
   * Its lines with warnings.
   */
  warnings: number
}
/**
 * The import's counts.
 */
export interface LineCounts {
  /**
   * Resolved after an import tolerance.
   */
  adapted: number
  /**
   * Excluded by the item (another variant, a custom line, a
   * section without effect).
   */
  excluded: number
  /**
   * Resolved to game stats exactly.
   */
  matched: number
  /**
   * Modifier lines.
   */
  modifiers: number
  /**
   * With a warning.
   */
  warnings: number
}
/**
 * A stage of the build (PoB's loadout): a passive spec and the sets
 * linked to it, switched together.
 */
export interface LoadoutView {
  /**
   * Whether the build uses this spec and every linked set.
   */
  active: boolean
  /**
   * The linked config set, if one is.
   */
  'config-set'?: number | null
  /**
   * The linked item set, if one is.
   */
  'item-set'?: number | null
  /**
   * The linked skill set, if one is.
   */
  'skill-set'?: number | null
  /**
   * The passive spec's ID (the stage's key).
   */
  spec: number
  /**
   * The title, without colour codes and links.
   */
  title: string
}
/**
 * A passive node's name.
 */
export interface NodeName {
  /**
   * Its name in the game data.
   */
  name: string
  /**
   * The node number.
   */
  node: number
}
/**
 * One socket group of the active skill set.
 */
export interface SkillGroupView {
  /**
   * Whether the group is enabled.
   */
  enabled: boolean
  /**
   * Its gems, in order.
   */
  gems: GemView[]
  /**
   * The label, if any.
   */
  label?: string | null
  /**
   * Whether it is the build's main skill group.
   */
  main: boolean
  /**
   * The slot it is socketed in, if any.
   */
  slot?: string | null
}
/**
 * One gem of a socket group.
 */
export interface GemView {
  /**
   * Its colour, when the game data knows it: `red`, `green`,
   * `blue`, or `white`.
   */
  colour?: string | null
  /**
   * Whether the gem is enabled.
   */
  enabled: boolean
  /**
   * The level.
   */
  level: number
  /**
   * Whether it is the build's main skill (in the main group, the
   * active skill the group's main skill names).
   */
  main: boolean
  /**
   * The name as written, if any.
   */
  name?: string | null
  /**
   * The quality.
   */
  quality: number
  /**
   * The socket it sits in, by index in its slot's item (PoB fills a
   * slot's sockets with its groups' gems in order); `None` when the
   * group has no slot or the item has no socket left for it.
   */
  socket?: number | null
  /**
   * Whether it is a support, when the game data knows the gem.
   */
  support?: boolean | null
  /**
   * For a support: the gems of the group (by index) it can support,
   * as the calculation matches supports to a skill.
   */
  supports: number[]
}
/**
 * A line with a warning.
 */
export interface WarningView {
  /**
   * The stable code.
   */
  code: string
  /**
   * Whether the line's item is in use ([`ItemSummary::in_use`]): a
   * warning on an item not in use does not touch the calculation.
   */
  'in-use': boolean
  /**
   * The item's ID.
   */
  item: number
  /**
   * The item's name, if any.
   */
  'item-name'?: string | null
  /**
   * The line's index.
   */
  line: number
  /**
   * The line as written.
   */
  text: string
}
/**
 * The calculation of the latest version.
 */
export interface CalcStatus {
  /**
   * Why the latest calculation failed, if it did.
   */
  error?: ApiError3 | null
  /**
   * The result's overall state (`complete`, `partial`,
   * `unsupported`, `indeterminate`), when there is one.
   */
  'result-state'?: string | null
  /**
   * Where it is.
   */
  state: 'idle' | 'running' | 'done' | 'failed'
  /**
   * The version of the result held (`result` is of it); may be
   * older than the snapshot's while a calculation runs.
   */
  version: number
}
/**
 * The game data the application calculates with.
 */
export interface DataStatus {
  /**
   * The bundle folder, if one is set.
   */
  bundle?: string | null
  /**
   * The bundle's data version, when it loaded.
   */
  'data-version'?: DataVersionView | null
  /**
   * Why the bundle did not load, if it did not (a stable code).
   */
  error?: ApiError3 | null
  /**
   * The passive tree version the application gives the engine for
   * this bundle, if known.
   */
  'tree-version'?: string | null
}
/**
 * Undo and redo.
 */
export interface HistoryView {
  /**
   * Whether there is a change to redo.
   */
  'can-redo': boolean
  /**
   * Whether there is a change to undo.
   */
  'can-undo': boolean
  /**
   * What redo would apply again.
   */
  redo?: HistoryEntry | null
  /**
   * What undo would revert.
   */
  undo?: HistoryEntry | null
}
/**
 * One change in the history (ADR-017: an addon's change is recorded
 * like the user's).
 */
export interface HistoryEntry {
  /**
   * The item it changed (for `item-set`: the item set it made
   * active).
   */
  item: number
  /**
   * What it was.
   */
  kind:
    | 'item-edit'
    | 'request-applied'
    | 'item-set'
    | 'loadout'
    | 'skill-set'
    | 'skill-edit'
    | 'config'
    | 'league'
    | 'weapon-set'
  /**
   * The change request it applied, when an MCP client asked for it.
   */
  request?: number | null
}
