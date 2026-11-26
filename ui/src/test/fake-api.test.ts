import { describe, expect, it } from 'vitest'

import { FakeApi } from '../api/fake'
import { ApiFailure, type Command } from '../api/types'

const failure = async (p: Promise<unknown>) => {
  try {
    await p
  } catch (e) {
    return e instanceof ApiFailure ? e.code : String(e)
  }
  return null
}

describe('the fake API', () => {
  it('does nothing for a command it has no handler for', async () => {
    const api = new FakeApi({ recent: [] })
    await api.execute('new-build')
    const version = api.version
    expect(await api.execute({ 'use-config-set': { id: 1 } })).toBe('nothing')
    expect(await api.execute({ constructor: {} } as unknown as Command)).toBe(
      'nothing',
    )
    expect(api.version).toBe(version)
  })

  it('walks the history with undo and redo', async () => {
    const api = new FakeApi()
    expect(await api.execute('undo')).toBe('nothing')
    await api.execute('new-build')
    await api.execute({ 'set-league': { league: 'standard' } })
    expect((await api.snapshot()).build?.league).toBe('standard')
    await api.execute('undo')
    expect((await api.snapshot()).build?.league).toBe('challenge')
    await api.execute('redo')
    const snap = await api.snapshot()
    expect(snap.build?.league).toBe('standard')
    expect(snap.history['can-redo']).toBe(false)
    expect(await api.execute('redo')).toBe('nothing')
  })

  it('needs an open build for an edit, and refuses another spec', async () => {
    const api = new FakeApi()
    expect(
      await failure(api.execute({ 'set-league': { league: 'standard' } })),
    ).toBe('APP_NO_BUILD')
    await api.execute('new-build')
    expect(
      await failure(
        api.execute({ 'allocate-node': { spec: 2, node: 11 } } as Command),
      ),
    ).toBe('APP_TREE')
  })
})
