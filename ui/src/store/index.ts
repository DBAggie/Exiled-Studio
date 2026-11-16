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
  type DropNotice,
  type LoomApi,
  type McpPermissions,
  type McpStatus,
  type Outcome,
  type RecentEntry,
  type RecoveryEntry,
  type Snapshot,
  type UpdateMode,
  type UpdateStatus,
} from '../api/types'
import { type Prefs, defaultPrefs, readPrefs } from '../dashboards'
import { t } from '../i18n'
import { historyText } from '../util'
import { type Workspace, writeWorkspace } from '../workspace'
import { importFlow, importActions } from './importFlow'

export type Screen =
  | 'start'
  | 'import-review'
  | 'dashboard'
  | 'breakdown'
  | 'items'
  | 'item'
  | 'skills'
  | 'tree'
  | 'config'
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
      // Refreshes can answer out of order: an older snapshot never
      // replaces a newer one.
      if (
        state.snapshot === null ||
        action.payload.version >= state.snapshot.version
      ) {
        state.snapshot = action.payload
      }
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
  /** A finished action to confirm (saved, exported), until dismissed or
   * the next screen. */
  flash: string | null
  /** A file is being dragged over the window. */
  dragging: boolean
  /** An action that would replace a build with unsaved changes, waiting
   * for the user's answer (UX-01). */
  confirm: Replace | null
  /** The user's dashboards (B-5). */
  prefs: Prefs
  /** The numbers compared against (UX audit D-03): the ones pinned this
   * session, a snapshot kept in the build, or another build file's. */
  baseline: Mark | null
  /** The build's own dashboard shown (a build that keeps its own set);
   * for the session, so that changing tabs is not a change to the build. */
  dashActive: string | null
  /** The stage the tree and the equipment are compared with, by its
   * spec. */
  compareStage: number | null
  /** The numbers after each change this session, oldest first: what the
   * history charts draw. */
  trail: Mark[]
}

/** Every computed field's value at one moment of the build. */
export interface Mark {
  /** The calculation's version. */
  version: number
  /** What made these numbers, in words ("Opened", "edit of Shiny Wand"). */
  label: string
  /** Each field's value; `null` when it was not computed. */
  fields: Record<string, number | null>
}

/** The most steps a session's trail keeps. */
export const MAX_TRAIL = 60

/** A calculation's fields as a mark. */
export function markOf(
  calc: [number, CalcResultDocumentV1],
  label: string,
): Mark {
  const fields: Record<string, number | null> = {}
  for (const f of calc[1].result.fields) {
    const value = (f as { value?: unknown }).value
    fields[f.field] = typeof value === 'number' ? value : null
  }
  return { version: calc[0], label, fields }
}

/** What replaces the open build: a new one, a file the user picks, a
 * dropped file. */
export type Replace =
  | 'new-build'
  | 'open-file'
  | 'import-file'
  | { dropped: Extract<DropNotice, { state: 'dropped' }>['kind'] }

const ui = createSlice({
  name: 'ui',
  initialState: {
    screen: 'start',
    field: null,
    item: null,
    announcement: '',
    error: null,
    shareCode: null,
    flash: null,
    dragging: false,
    confirm: null,
    prefs: defaultPrefs(),
    baseline: null,
    dashActive: null,
    compareStage: null,
    trail: [],
  } as UiState,
  reducers: {
    go(state, action: PayloadAction<Screen>) {
      state.screen = action.payload
      state.flash = null
    },
    flash(state, action: PayloadAction<string | null>) {
      state.flash = action.payload
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
    dragging(state, action: PayloadAction<boolean>) {
      state.dragging = action.payload
    },
    confirm(state, action: PayloadAction<Replace | null>) {
      state.confirm = action.payload
    },
    prefs(state, action: PayloadAction<Prefs>) {
      state.prefs = action.payload
    },
    baseline(state, action: PayloadAction<Mark | null>) {
      state.baseline = action.payload
    },
    dashActive(state, action: PayloadAction<string | null>) {
      state.dashActive = action.payload
    },
    compareStage(state, action: PayloadAction<number | null>) {
      state.compareStage = action.payload
    },
    /** A new calculation's numbers join the trail (once per version). */
    tracked(state, action: PayloadAction<Mark>) {
      const last = state.trail[state.trail.length - 1]
      if (last !== undefined && last.version === action.payload.version) {
        return
      }
      state.trail.push(action.payload)
      if (state.trail.length > MAX_TRAIL) state.trail.shift()
    },
    /** Another build (or none): its numbers start over. */
    forgetNumbers(state) {
      state.baseline = null
      state.trail = []
      state.dashActive = null
      state.compareStage = null
    },
  },
})

interface ListsState {
  recent: RecentEntry[]
  recovery: RecoveryEntry[]
  mcp: McpStatus | null
  /** Application updates (Phase 5, P-d); null before the shell answers. */
  updates: UpdateStatus | null
  /** The offered version the user put off with "Later". */
  updateLater: string | null
}

const lists = createSlice({
  name: 'lists',
  initialState: {
    recent: [],
    recovery: [],
    mcp: null,
    updates: null,
    updateLater: null,
  } as ListsState,
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
    updates(state, action: PayloadAction<UpdateStatus>) {
      state.updates = action.payload
    },
    updateLater(state, action: PayloadAction<string | null>) {
      state.updateLater = action.payload
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
  const before = getState().session.snapshot
  const snapshot = await api.snapshot()
  dispatch(actions.snapshot(snapshot))
  // Another build's numbers are not this one's history: a build opened,
  // imported, or closed starts the trail and the baseline over. (A build
  // is the same while its version only grows by its own changes; the
  // application starts each build's history empty.)
  const replaced =
    snapshot.build == null ||
    before?.build == null ||
    (snapshot.history['can-undo'] !== true &&
      snapshot.history['can-redo'] !== true &&
      snapshot.version !== before.version)
  const ui = getState().ui
  if (
    replaced &&
    (ui.trail.length > 0 ||
      ui.baseline !== null ||
      ui.dashActive !== null ||
      ui.compareStage !== null)
  ) {
    dispatch(actions.forgetNumbers())
  }
  const held = getState().session.calc
  if (held === null || held[0] !== snapshot.calc.version) {
    const calc = await api.calcDocument()
    dispatch(actions.calc(calc))
    if (calc !== null && snapshot.build != null) {
      const undo = snapshot.history.undo
      const mark = markOf(
        calc,
        undo == null ? t('trail.opened') : historyText(undo, snapshot.build),
      )
      // A change to the build's dashboards or kept snapshots (done or
      // undone) moves no number: it is not a step of the trail.
      const last = getState().ui.trail.at(-1)
      const workspace =
        (undo?.kind === 'workspace' ||
          snapshot.history.redo?.kind === 'workspace') &&
        last !== undefined &&
        JSON.stringify(last.fields) === JSON.stringify(mark.fields)
      if (!workspace) dispatch(actions.tracked(mark))
    }
  }
}

export const refreshLists = (): Thunk => async (dispatch, _getState, api) => {
  dispatch(actions.recent(await api.recent()))
  dispatch(actions.recovery(await api.recovery()))
  dispatch(actions.mcp(await api.mcpStatus()))
}

/** Reads the user's dashboards (the defaults when none were saved). */
export const loadPrefs = (): Thunk => async (dispatch, _getState, api) => {
  try {
    dispatch(actions.prefs(readPrefs(await api.uiPrefs())))
  } catch {
    dispatch(actions.prefs(defaultPrefs()))
  }
}

/** Changes the user's dashboards and saves them; a failed save keeps the
 * change for this session and says so. */
export const setPrefs =
  (prefs: Prefs): Thunk =>
  async (dispatch, _getState, api) => {
    dispatch(actions.prefs(prefs))
    try {
      await api.setUiPrefs(JSON.stringify(prefs))
    } catch (e) {
      dispatch(actions.error(codeOf(e)))
    }
  }

/** Stores the build's workspace (its own dashboards, its kept
 * snapshots) in the build: one undoable change. */
export const setWorkspace =
  (workspace: Workspace): Thunk<Outcome | null> =>
  (dispatch) =>
    dispatch(run({ 'set-workspace': { document: writeWorkspace(workspace) } }))

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

/** A read that changes nothing: no refresh, no error shown; null on
 * failure (for details a screen can do without). */
export const query =
  <T>(call: (api: LoomApi) => Promise<T>): Thunk<T | null> =>
  async (_dispatch, _getState, api) => {
    try {
      return await call(api)
    } catch {
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

/** Starts a new build (UX brief note 3). */
export const newBuild = (): Thunk => async (dispatch) => {
  const outcome = await dispatch(run('new-build'))
  if (outcome !== null && outcome !== 'nothing') {
    dispatch(actions.go('dashboard'))
  }
}

/** Does `what` now. */
export const perform =
  (what: Replace): Thunk =>
  async (dispatch) => {
    if (what === 'new-build') return dispatch(newBuild())
    if (what === 'open-file')
      return dispatch(openBuild((api) => api.openFile()))
    if (what === 'import-file')
      return dispatch(importBuild((api) => api.importFile()))
    if (what.dropped === 'pob')
      return dispatch(importBuild((api) => api.openDropped()))
    // A build document opens; anything else fails with UI_DROP.
    return dispatch(openBuild((api) => api.openDropped()))
  }

/** Does `what`, first asking when it would replace unsaved changes
 * (UX-01). A dropped file that is not a build never asks. */
export const replaceBuild =
  (what: Replace): Thunk =>
  async (dispatch, getState) => {
    const dirty = getState().session.snapshot?.dirty === true
    const harmless = typeof what === 'object' && what.dropped === 'other'
    if (dirty && !harmless) {
      dispatch(actions.confirm(what))
      return
    }
    await dispatch(perform(what))
  }

/** Saves the build: to its file, or asks where when it has none. */
export const saveBuild = (): Thunk<Outcome | null> => (dispatch, getState) => {
  const path = getState().session.snapshot?.path
  return dispatch(
    attempt((api) => (path == null ? api.saveAs() : api.execute('save'))),
  )
}

/** Closes the build and goes to the start screen. */
export const closeBuild = (): Thunk => async (dispatch) => {
  const outcome = await dispatch(run('close'))
  if (outcome !== null) {
    dispatch(actions.go('start'))
  }
}

export const setPermissions =
  (permissions: McpPermissions): Thunk =>
  async (dispatch, _getState, api) => {
    dispatch(actions.mcp(await api.setPermissions(permissions)))
  }

/** Sets how the application looks for updates. */
export const setUpdateMode =
  (mode: UpdateMode): Thunk =>
  async (dispatch, _getState, api) => {
    try {
      dispatch(actions.updates(await api.setUpdateMode(mode)))
    } catch (e) {
      dispatch(actions.error(codeOf(e)))
    }
  }

/** Checks for an update now. */
export const checkUpdates = (): Thunk => async (dispatch, _getState, api) => {
  try {
    dispatch(actions.updates(await api.checkUpdates()))
    dispatch(actions.updateLater(null))
    dispatch(actions.error(null))
  } catch (e) {
    dispatch(actions.error(codeOf(e)))
  }
}

/** Installs the offered update (the user approved it); the application
 * restarts when it is done. */
export const installUpdate = (): Thunk => async (dispatch, _getState, api) => {
  try {
    await api.installUpdate()
  } catch (e) {
    dispatch(actions.error(codeOf(e)))
    const status = await api.updateStatus().catch(() => null)
    if (status !== null) dispatch(actions.updates(status))
  }
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
    if (typeof kind === 'object' && 'saved' in kind) {
      store.dispatch(actions.flash(t('flash.saved', { path: kind.saved.path })))
    }
    if (
      kind === 'mcp-changed' ||
      (typeof kind === 'object' && 'saved' in kind)
    ) {
      void store.dispatch(refreshLists())
    }
  })
  const stopDrop = await api.onDrop((notice) => {
    store.dispatch(actions.dragging(notice.state === 'over'))
    if (notice.state === 'dropped') {
      // The shell keeps only the latest drop: a question still open for an
      // earlier one is replaced, so the answer opens what was dropped last.
      store.dispatch(actions.confirm(null))
      void store.dispatch(replaceBuild({ dropped: notice.kind }))
    }
  })
  const stopUpdates = await api.onUpdates((status) => {
    store.dispatch(actions.updates(status))
  })
  await store.dispatch(refresh())
  await store.dispatch(refreshLists())
  const updates = await api.updateStatus().catch(() => null)
  if (updates !== null) store.dispatch(actions.updates(updates))
  // The first screen has its data: a packaged smoke run quits here (Phase
  // 5, P1); a failure only means the shell is older or absent.
  setTimeout(() => void api.ready().catch(() => undefined), 0)
  await store.dispatch(loadPrefs())
  return () => {
    stop()
    stopDrop()
    stopUpdates()
  }
}
