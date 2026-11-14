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
      add: {
        /**
         * The text.
         */
        text: string
      }
    }

/**
 * Every top-level type of the API, so one schema names them all. Never
 * sent.
 */
export interface ApplicationAPIV1 {
  /**
   * A command.
   */
  command:
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
 * One item.
 */
export interface ItemView {
  /**
   * The base, if written.
   */
  base?: string | null
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
   * The numbers in the text, in order, for roll edits.
   */
  numbers: NumberSpan[]
  /**
   * The modifier list it belongs to (`implicit`, `explicit`, ...);
   * modifier lines only.
   */
  section?: string | null
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
   * When it was last opened or saved, in seconds since the Unix
   * epoch.
   */
  at: number
  /**
   * The ID the application issued for it (the UI opens a recent
   * build by it and never sends a path).
   */
  id: string
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
   * The data version the build is pinned to.
   */
  'data-version'?: DataVersionView | null
  /**
   * `poe1` or `poe2`.
   */
  game: string
  /**
   * The items, in build order.
   */
  items: ItemSummary[]
  /**
   * The character level, if set.
   */
  level?: number | null
  lines: LineCounts
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
   * Lines with warnings: kept, shown, not calculated (ADR-015).
   */
  warnings: WarningView[]
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
   * The name, if any.
   */
  name?: string | null
  /**
   * `normal`, `magic`, `rare`, `unique`, `relic`, if read.
   */
  rarity?: string | null
  /**
   * The slot it is in, in the active item set, if any.
   */
  slot?: string | null
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
 * A line with a warning.
 */
export interface WarningView {
  /**
   * The stable code.
   */
  code: string
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
   * The item it changed.
   */
  item: number
  /**
   * What it was.
   */
  kind: 'item-edit' | 'request-applied'
  /**
   * The change request it applied, when an MCP client asked for it.
   */
  request?: number | null
}
