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
  type ItemEdit,
  type ItemEditPreview,
  type ItemView,
  type LineView,
  type LoomApi,
  type McpPermissions,
  type McpStatus,
  type Outcome,
  type RecentEntry,
  type RecoveryEntry,
  type Snapshot,
} from './types'

export interface FakeOptions {
  /** Whether game data is loaded. */
  data?: boolean
  /** A code every import fails with. */
  importFails?: string
  /** A code every calculation fails with. */
  calcFails?: string
  recent?: RecentEntry[]
  recovery?: RecoveryEntry[]
}

function line(
  index: number,
  text: string,
  extra: Partial<LineView> = {},
): LineView {
  return {
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
  ]
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

interface State {
  items: ItemView[]
}

export class FakeApi implements LoomApi {
  version = 0
  private options: FakeOptions
  private listeners: Listener[] = []
  private current: State | null = null
  private undoStack: State[] = []
  private redoStack: State[] = []
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

  private done(): Outcome {
    return { done: { version: this.version } }
  }

  snapshot(): Promise<Snapshot> {
    const items = this.current?.items ?? []
    const warnings = items.flatMap((item) =>
      item.lines
        .filter((l) => l.status === 'warning')
        .map((l) => ({
          item: item.id,
          'item-name': item.name,
          line: l.index,
          text: l.text,
          code: l.code ?? '',
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
              lines: {
                modifiers: all.length,
                matched: count('matched'),
                adapted: count('adapted'),
                warnings: count('warning'),
                excluded: count('excluded'),
              },
              items: items.map((i) => ({
                id: i.id,
                name: i.name,
                base: i.base,
                rarity: i.rarity,
                slot: i.slot,
                warnings: i.lines.filter((l) => l.status === 'warning').length,
              })),
              warnings,
            },
      calc: this.calcState,
      history: {
        'can-undo': this.undoStack.length > 0,
        'can-redo': this.redoStack.length > 0,
        undo: null,
        redo: null,
      },
      dirty: this.dirty,
      path: this.path,
      origin: null,
      data: {
        bundle: this.options.data === true ? '/data/bundle' : null,
        'data-version':
          this.options.data === true
            ? { game: 'poe1', 'schema-version': 3, 'steam-build': 26530219 }
            : null,
        'tree-version': this.options.data === true ? '3_29' : null,
        error: null,
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
    return { items }
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
    if ('open-recent' in command) {
      if (!this.recentList.some((r) => r.id === command['open-recent'].id)) {
        this.fail('APP_NOT_FOUND')
      }
      this.path = 'recent.json'
      this.dirty = false
      this.changed({ items: sampleItems() })
      return this.done()
    }
    if ('restore' in command) {
      this.recoveryList = this.recoveryList.filter(
        (r) => r.id !== command.restore.id,
      )
      this.dirty = true
      this.changed({ items: sampleItems() })
      return this.done()
    }
    if ('discard' in command) {
      this.recoveryList = this.recoveryList.filter(
        (r) => r.id !== command.discard.id,
      )
      return 'nothing'
    }
    if ('approve' in command) {
      const r = this.request(command.approve.id)
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
    this.changed({ items: sampleItems() })
    return this.done()
  }

  recent() {
    return Promise.resolve(this.recentList)
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
    this.mcp.token = 'fake-token'
    return this.mcpStatus()
  }

  mcpStop() {
    this.mcp.running = false
    this.mcp.url = null
    this.mcp.token = null
    this.mcp.permissions['standing-grant'] = false
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

  importFile() {
    return Promise.resolve(this.importText('<PathOfBuilding/>'))
  }

  openFile() {
    this.path = 'build.json'
    this.dirty = false
    this.undoStack = []
    this.redoStack = []
    this.changed({ items: sampleItems() })
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

  chooseBundle() {
    this.options.data = true
    this.version += 1
    this.emit('build-changed')
    return Promise.resolve(this.done())
  }

  onEvent(listener: Listener) {
    this.listeners.push(listener)
    return Promise.resolve(() => {
      this.listeners = this.listeners.filter((l) => l !== listener)
    })
  }
}
