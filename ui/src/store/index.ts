// The UI's read projections (ADR-004: Redux Toolkit holds projections;
// Rust is authoritative). Nothing here changes a build: commands go to
// the application and the store takes the snapshot it then reports.
import {
  configureStore,
  createSlice,
  type PayloadAction,
} from '@reduxjs/toolkit'

import {
  ApiFailure,
  type CalcResultDocumentV1,
  type Command,
  type LoomApi,
  type McpPermissions,
  type McpStatus,
  type Outcome,
  type RecentEntry,
  type RecoveryEntry,
  type Snapshot,
} from '../api/types'
import { t } from '../i18n'
import { importFlow, importActions } from './importFlow'

export type Screen =
  | 'start'
  | 'import-review'
  | 'dashboard'
  | 'breakdown'
  | 'items'
  | 'item'
  | 'warnings'
  | 'ai'
  | 'about'

interface SessionState {
  snapshot: Snapshot | null
  calc: [number, CalcResultDocumentV1] | null
}

const session = createSlice({
  name: 'session',
  initialState: { snapshot: null, calc: null } as SessionState,
  reducers: {
    snapshot(state, action: PayloadAction<Snapshot>) {
      state.snapshot = action.payload
    },
    calc(state, action: PayloadAction<[number, CalcResultDocumentV1] | null>) {
      state.calc = action.payload
    },
  },
})

interface UiState {
  screen: Screen
  field: string | null
  item: number | null
  /** Polite announcements (aria-live). */
  announcement: string
  /** The latest error code, shown in an alert until dismissed. */
  error: string | null
  /** A share code to show. */
  shareCode: string | null
}

const ui = createSlice({
  name: 'ui',
  initialState: {
    screen: 'start',
    field: null,
    item: null,
    announcement: '',
    error: null,
    shareCode: null,
  } as UiState,
  reducers: {
    go(state, action: PayloadAction<Screen>) {
      state.screen = action.payload
    },
    breakdown(state, action: PayloadAction<string>) {
      state.field = action.payload
      state.screen = 'breakdown'
    },
    editItem(state, action: PayloadAction<number>) {
      state.item = action.payload
      state.screen = 'item'
    },
    announce(state, action: PayloadAction<string>) {
      state.announcement = action.payload
    },
    error(state, action: PayloadAction<string | null>) {
      state.error = action.payload
    },
    shareCode(state, action: PayloadAction<string | null>) {
      state.shareCode = action.payload
    },
  },
})

interface ListsState {
  recent: RecentEntry[]
  recovery: RecoveryEntry[]
  mcp: McpStatus | null
}

const lists = createSlice({
  name: 'lists',
  initialState: { recent: [], recovery: [], mcp: null } as ListsState,
  reducers: {
    recent(state, action: PayloadAction<RecentEntry[]>) {
      state.recent = action.payload
    },
    recovery(state, action: PayloadAction<RecoveryEntry[]>) {
      state.recovery = action.payload
    },
    mcp(state, action: PayloadAction<McpStatus>) {
      state.mcp = action.payload
    },
  },
})

export const actions = {
  ...session.actions,
  ...ui.actions,
  ...lists.actions,
  ...importActions,
}

export function makeStore(api: LoomApi) {
  return configureStore({
    reducer: {
      session: session.reducer,
      ui: ui.reducer,
      lists: lists.reducer,
      importFlow: importFlow.reducer,
    },
    middleware: (getDefault) =>
      getDefault({ thunk: { extraArgument: api }, serializableCheck: false }),
  })
}

export type Store = ReturnType<typeof makeStore>
export type RootState = ReturnType<Store['getState']>
export type Dispatch = Store['dispatch']
type Thunk<T = void> = (
  dispatch: Dispatch,
  getState: () => RootState,
  api: LoomApi,
) => Promise<T>

function codeOf(e: unknown): string {
  return e instanceof ApiFailure ? e.code : 'APP_UNKNOWN'
}

/** Reads the snapshot (and the calculation, when it changed). */
export const refresh = (): Thunk => async (dispatch, getState, api) => {
  const snapshot = await api.snapshot()
  dispatch(actions.snapshot(snapshot))
  const held = getState().session.calc
  if (held === null || held[0] !== snapshot.calc.version) {
    dispatch(actions.calc(await api.calcDocument()))
  }
}

export const refreshLists = (): Thunk => async (dispatch, _getState, api) => {
  dispatch(actions.recent(await api.recent()))
  dispatch(actions.recovery(await api.recovery()))
  dispatch(actions.mcp(await api.mcpStatus()))
}

/** Runs a call; on failure shows its code and returns null. */
export const attempt =
  <T>(call: (api: LoomApi) => Promise<T>): Thunk<T | null> =>
  async (dispatch, _getState, api) => {
    try {
      const answer = await call(api)
      dispatch(actions.error(null))
      await dispatch(refresh())
      return answer
    } catch (e) {
      dispatch(actions.error(codeOf(e)))
      return null
    }
  }

export const run = (command: Command): Thunk<Outcome | null> =>
  attempt((api) => api.execute(command))

/** The import workflow (state machine in importFlow.ts). */
export const importBuild =
  (call: (api: LoomApi) => Promise<Outcome>): Thunk =>
  async (dispatch, _getState, api) => {
    dispatch(actions.importStarted())
    try {
      const outcome = await call(api)
      if (outcome === 'nothing') {
        dispatch(actions.importCancelled())
        return
      }
      await dispatch(refresh())
      dispatch(actions.importDone())
      dispatch(actions.go('import-review'))
    } catch (e) {
      dispatch(actions.importFailed(codeOf(e)))
    }
  }

export const openBuild =
  (call: (api: LoomApi) => Promise<Outcome>): Thunk =>
  async (dispatch) => {
    const outcome = await dispatch(attempt(call))
    if (outcome !== null && outcome !== 'nothing') {
      dispatch(actions.go('dashboard'))
      await dispatch(refreshLists())
    }
  }

export const setPermissions =
  (permissions: McpPermissions): Thunk =>
  async (dispatch, _getState, api) => {
    dispatch(actions.mcp(await api.setPermissions(permissions)))
  }

/** Follows the application's events (ADR-004): every event is followed by
 * a snapshot query, so a missed event cannot leave the UI behind. */
export async function startSync(store: Store, api: LoomApi) {
  const stop = await api.onEvent((event) => {
    const kind = event.kind
    if (typeof kind === 'object' && 'calc-finished' in kind) {
      store.dispatch(actions.announce(t('calc.announceDone')))
    } else if (kind === 'calc-started') {
      store.dispatch(actions.announce(t('calc.announceRunning')))
    }
    void store.dispatch(refresh())
    if (
      kind === 'mcp-changed' ||
      (typeof kind === 'object' && 'saved' in kind)
    ) {
      void store.dispatch(refreshLists())
    }
  })
  await store.dispatch(refresh())
  await store.dispatch(refreshLists())
  return stop
}
