// The Tauri implementation of LoomApi. Command names come from the
// committed list (commands.json), which a Rust test compares with the
// shell's handlers (Phase 4 plan, decision 9).
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'

import commands from './commands.json'
import {
  ApiFailure,
  type ApiError,
  type ApiEvent,
  type CalcResultDocumentV1,
  type DropNotice,
  type LoomApi,
  type DataAutoStatus,
  type UpdateStatus,
} from './types'

export const COMMANDS: readonly string[] = commands
export const EVENT = 'loom://event'
export const DROP_EVENT = 'loom://drop'

/** The update check's channel (Rust `updates::UPDATE_EVENT`). */
export const UPDATE_EVENT = 'loom://updates'
/** The automatic game data's channel (Rust `data_updates::DATA_EVENT`). */
export const DATA_EVENT = 'loom://data-auto'

type Name = (typeof commands)[number]

function isApiError(e: unknown): e is ApiError {
  return (
    typeof e === 'object' &&
    e !== null &&
    'code' in e &&
    typeof (e as { code: unknown }).code === 'string'
  )
}

async function call<T>(name: Name, args: Record<string, unknown> = {}) {
  if (!COMMANDS.includes(name)) {
    throw new Error(`unknown command ${name}`)
  }
  try {
    return await invoke<T>(name, args)
  } catch (e) {
    if (isApiError(e)) {
      throw new ApiFailure(e)
    }
    throw e
  }
}

export const tauriApi: LoomApi = {
  snapshot: () => call('snapshot'),
  calcDocument: async () => {
    const answer = await call<[number, string] | null>('calc_document')
    return answer === null
      ? null
      : [answer[0], JSON.parse(answer[1]) as CalcResultDocumentV1]
  },
  item: (id) => call('item', { id }),
  gems: (group) => call('gems', { group: group ?? null }),
  preview: (edit) => call('preview', { edit }),
  execute: (command) => call('execute', { command }),
  recent: () => call('recent'),
  recovery: () => call('recovery'),
  mcpStatus: () => call('mcp_status'),
  setPermissions: (permissions) => call('set_permissions', { permissions }),
  mcpStart: () => call('mcp_start'),
  mcpStop: () => call('mcp_stop'),
  mcpRotate: () => call('mcp_rotate'),
  importFile: () => call('import_file'),
  openFile: () => call('open_file'),
  saveAs: () => call('save_as'),
  exportXml: () => call('export_xml'),
  chooseBundle: () => call('choose_bundle'),
  installBundle: () => call('install_bundle'),
  checkDataFeed: () => call('check_data_feed'),
  downloadData: (id) => call('download_data', { id }),
  openDropped: () => call('open_dropped'),
  treeLayout: () => call('tree_layout'),
  treeSprites: () => call('tree_sprites'),
  crashes: () => call('crashes'),
  dismissCrash: (id) => call('dismiss_crash', { id }),
  diagnostics: () => call('diagnostics'),
  exportDiagnostics: () => call('export_diagnostics'),
  ready: () => call('app_ready'),
  updateStatus: () => call('update_status'),
  setUpdateMode: (mode) => call('set_update_mode', { mode }),
  checkUpdates: () => call('check_updates'),
  installUpdate: () => call('install_update'),
  onUpdates: (listener) =>
    listen<UpdateStatus>(UPDATE_EVENT, (e) => {
      listener(e.payload)
    }),
  dataAutoStatus: () => call('data_auto_status'),
  fetchData: () => call('fetch_data'),
  onDataAuto: (listener) =>
    listen<DataAutoStatus>(DATA_EVENT, (e) => {
      listener(e.payload)
    }),
  previewTree: (spec, node, fields = true) =>
    call('preview_tree', { spec, node, fields }),
  whatIf: (request) => call('what_if', { request }),
  compareFile: () => call('compare_file'),
  stageView: (spec) => call('stage_view', { spec }),
  passiveLines: (nodes) => call('passive_lines', { nodes }),
  uiPrefs: () => call('ui_prefs'),
  setUiPrefs: (text) => call('set_ui_prefs', { text }),
  onEvent: (listener) =>
    listen<ApiEvent>(EVENT, (e) => {
      listener(e.payload)
    }),
  onDrop: (listener) =>
    listen<DropNotice>(DROP_EVENT, (e) => {
      listener(e.payload)
    }),
}

/** Whether the page runs inside the Tauri shell. */
export function inTauri(): boolean {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window
}
