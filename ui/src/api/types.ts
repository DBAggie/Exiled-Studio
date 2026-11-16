// The API's types, from the generated bindings (ADR-002): the app API v1
// and the calc result document v1. Named here for the rest of the UI.
import type {
  ApiError,
  ApplicationAPIV1,
  BuildView,
  CalcStatus,
  ChangeRequest,
  ConfigInputView,
  ConfigView,
  CrashRecord,
  DataFeedStatus,
  UpdateOffer,
  UpdateStatus,
  QuarantinedBundle,
  CustomBlockView,
  CustomLineView,
  DataVersionView,
  DiagnosticFile,
  FieldDelta,
  GemCatalogEntry,
  GemView,
  HistoryEntry,
  ItemCard,
  ItemEdit,
  ItemEditPreview,
  ItemSummary,
  ItemView,
  LineOp,
  LineView,
  LoadoutView,
  McpPermissions,
  McpStatus,
  RecentEntry,
  RecoveryEntry,
  SetView,
  SkillGroupView,
  Snapshot,
  TreeClassView,
  TreeLayout,
  TreeNodeView,
  TreeChange,
  TreePreview,
  TreeSprites,
  TreeView,
  WarningView,
} from '../generated/app-api.v1'
import type {
  CalcResultDocumentV1,
  FieldResult,
  Node,
} from '../generated/calc-result.v1'

export type {
  ApiError,
  BuildView,
  CalcResultDocumentV1,
  CalcStatus,
  ChangeRequest,
  ConfigInputView,
  ConfigView,
  CrashRecord,
  CustomBlockView,
  CustomLineView,
  DataVersionView,
  DiagnosticFile,
  FieldDelta,
  FieldResult,
  GemCatalogEntry,
  GemView,
  HistoryEntry,
  ItemCard,
  ItemEdit,
  ItemEditPreview,
  ItemSummary,
  ItemView,
  LineOp,
  LineView,
  LoadoutView,
  McpPermissions,
  McpStatus,
  Node,
  RecentEntry,
  RecoveryEntry,
  SetView,
  SkillGroupView,
  Snapshot,
  TreeClassView,
  TreeLayout,
  TreeNodeView,
  TreeChange,
  TreePreview,
  TreeSprites,
  TreeView,
  UpdateOffer,
  UpdateStatus,
  WarningView,
  QuarantinedBundle,
  DataFeedStatus,
}

/** The what-if query (app API v1, additive): the calculated fields at each
 * stage of the build, or with one input swept over a range. It never
 * changes the build. */
export interface WhatIfRequest {
  vary:
    | 'stages'
    | { 'gem-level': { group: number; gem: number; from: number; to: number } }
    | { 'character-level': { from: number; to: number; step: number } }
    | {
        'config-number': {
          name: string
          from: number
          to: number
          steps: number
        }
      }
}

/** One point of a what-if series: the fields with the input at `x`. */
export interface WhatIfPoint {
  /** The stage's title, or the swept value as text. */
  label: string
  /** The swept value; for stages, the index from 0. */
  x: number
  /** Whether this point is the build as it is now. */
  current: boolean
  fields: { field: string; value?: number | null; state: string }[]
}

export interface WhatIfSeries {
  points: WhatIfPoint[]
}

export type Command = ApplicationAPIV1['command']
export type UpdateMode = UpdateStatus['mode']
export type Outcome = ApplicationAPIV1['outcome']
export type ApiEvent = ApplicationAPIV1['event']

/** A failed call: the error's stable code (ADR-014). */
export class ApiFailure extends Error {
  readonly code: string
  constructor(error: ApiError) {
    super(`${error.code}: ${error.message}`)
    this.code = error.code
  }
}

/** A file over the window or dropped on it (the shell keeps its path;
 * `openDropped` opens it). */
export type DropNotice =
  | { state: 'over' }
  | { state: 'left' }
  | { state: 'dropped'; kind: 'pob' | 'build' | 'other'; name: string }

/** The calls the UI makes (ADR-004: commands and queries; events). */
export interface LoomApi {
  snapshot(): Promise<Snapshot>
  calcDocument(): Promise<[number, CalcResultDocumentV1] | null>
  item(id: number): Promise<ItemView>
  /** The gems the game data offers; for supports, whether each fits
   * socket group `group` of the active skill set. */
  gems(group?: number): Promise<GemCatalogEntry[]>
  preview(edit: ItemEdit): Promise<ItemEditPreview>
  execute(command: Command): Promise<Outcome>
  recent(): Promise<RecentEntry[]>
  recovery(): Promise<RecoveryEntry[]>
  /** Crash records, newest first (Phase 5, P-f). */
  crashes(): Promise<CrashRecord[]>
  /** Deletes a crash record the user has seen. */
  dismissCrash(id: string): Promise<void>
  /** The diagnostic package for the user to preview. */
  diagnostics(): Promise<DiagnosticFile[]>
  /** Writes the previewed package where the user picks; `nothing` when
   * the user cancels. */
  exportDiagnostics(): Promise<Outcome>
  mcpStatus(): Promise<McpStatus>
  setPermissions(permissions: McpPermissions): Promise<McpStatus>
  mcpStart(): Promise<McpStatus>
  mcpStop(): Promise<McpStatus>
  /** A new MCP token, kept; a running server takes it at once. */
  mcpRotate(): Promise<McpStatus>
  importFile(): Promise<Outcome>
  openFile(): Promise<Outcome>
  saveAs(): Promise<Outcome>
  exportXml(): Promise<Outcome>
  chooseBundle(): Promise<Outcome>
  /** Installs a data bundle pack (`.loombundle`) the user picks (Phase
   * 5, P-e); `nothing` when cancelled or already installed. */
  installBundle(): Promise<Outcome>
  /** Checks the project's signed data feed (`UPDATE_OFFLINE` in strict
   * offline mode; `FEED_*`). */
  checkDataFeed(): Promise<DataFeedStatus>
  /** Downloads a pack the last feed lists and installs it. */
  downloadData(id: string): Promise<Outcome>
  /** Opens the last dropped file: imports a PoB build, opens a build
   * document; `UI_DROP` for anything else. */
  openDropped(): Promise<Outcome>
  /** The loaded data's passive tree (static while the data is loaded). */
  treeLayout(): Promise<TreeLayout>
  /** The loaded data's tree sprites (static while the data is loaded);
   * `APP_NO_DATA` when the bundle has none. */
  treeSprites(): Promise<TreeSprites>
  /** What clicking node `node` of spec `spec` would do: allocate it and
   * its path, or deallocate it and its orphans; never committed. */
  previewTree(spec: number, node: number): Promise<TreePreview>
  /** The calculated fields at each stage, or with one input swept; never
   * a change to the build (`APP_WHAT_IF` for a range it refuses). */
  whatIf(request: WhatIfRequest): Promise<WhatIfSeries>
  /** The UI's own preferences document (the user's dashboards), `null`
   * before one is saved. */
  uiPrefs(): Promise<string | null>
  /** Saves the UI's preferences (`UI_PREFS` when not JSON or too big). */
  setUiPrefs(text: string): Promise<void>
  /** The first screen has rendered (a packaged smoke run quits on it;
   * otherwise nothing happens). */
  ready(): Promise<void>
  /** Application updates: mode, package, last check (Phase 5, P-d). */
  updateStatus(): Promise<UpdateStatus>
  setUpdateMode(mode: UpdateMode): Promise<UpdateStatus>
  /** Checks now (`UPDATE_OFFLINE`, `UPDATE_UNAVAILABLE`, `UPDATE_BUSY`). */
  checkUpdates(): Promise<UpdateStatus>
  /** Installs the offered update and restarts; the user approved it.
   * Refused for a package that only notifies (`UPDATE_NOTIFY_ONLY`). */
  installUpdate(): Promise<void>
  /** The check at start reports here. */
  onUpdates(listener: (status: UpdateStatus) => void): Promise<() => void>
  onEvent(listener: (event: ApiEvent) => void): Promise<() => void>
  onDrop(listener: (notice: DropNotice) => void): Promise<() => void>
}

/** A config input's value: on or off, a number, or text. */
export type ConfigValueView = ConfigInputView['value']
/** The league a build is planned for. */
export type League = BuildView['league']
