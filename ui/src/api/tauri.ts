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
  type LoomApi,
} from './types'

export const COMMANDS: readonly string[] = commands
export const EVENT = 'loom://event'

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
  preview: (edit) => call('preview', { edit }),
  execute: (command) => call('execute', { command }),
  recent: () => call('recent'),
  recovery: () => call('recovery'),
  mcpStatus: () => call('mcp_status'),
  setPermissions: (permissions) => call('set_permissions', { permissions }),
  mcpStart: () => call('mcp_start'),
  mcpStop: () => call('mcp_stop'),
  importFile: () => call('import_file'),
  openFile: () => call('open_file'),
  saveAs: () => call('save_as'),
  exportXml: () => call('export_xml'),
  chooseBundle: () => call('choose_bundle'),
  onEvent: (listener) =>
    listen<ApiEvent>(EVENT, (e) => {
      listener(e.payload)
    }),
}

/** Whether the page runs inside the Tauri shell. */
export function inTauri(): boolean {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window
}
