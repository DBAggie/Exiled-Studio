// The API's types, from the generated bindings (ADR-002): the app API v1
// and the calc result document v1. Named here for the rest of the UI.
import type {
  ApiError,
  ApplicationAPIV1,
  BuildView,
  CalcStatus,
  ChangeRequest,
  FieldDelta,
  ItemEdit,
  ItemEditPreview,
  ItemSummary,
  ItemView,
  LineOp,
  LineView,
  McpPermissions,
  McpStatus,
  RecentEntry,
  RecoveryEntry,
  Snapshot,
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
  FieldDelta,
  FieldResult,
  ItemEdit,
  ItemEditPreview,
  ItemSummary,
  ItemView,
  LineOp,
  LineView,
  McpPermissions,
  McpStatus,
  Node,
  RecentEntry,
  RecoveryEntry,
  Snapshot,
  WarningView,
}

export type Command = ApplicationAPIV1['command']
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

/** The calls the UI makes (ADR-004: commands and queries; events). */
export interface LoomApi {
  snapshot(): Promise<Snapshot>
  calcDocument(): Promise<[number, CalcResultDocumentV1] | null>
  item(id: number): Promise<ItemView>
  preview(edit: ItemEdit): Promise<ItemEditPreview>
  execute(command: Command): Promise<Outcome>
  recent(): Promise<RecentEntry[]>
  recovery(): Promise<RecoveryEntry[]>
  mcpStatus(): Promise<McpStatus>
  setPermissions(permissions: McpPermissions): Promise<McpStatus>
  mcpStart(): Promise<McpStatus>
  mcpStop(): Promise<McpStatus>
  importFile(): Promise<Outcome>
  openFile(): Promise<Outcome>
  saveAs(): Promise<Outcome>
  exportXml(): Promise<Outcome>
  chooseBundle(): Promise<Outcome>
  onEvent(listener: (event: ApiEvent) => void): Promise<() => void>
}
