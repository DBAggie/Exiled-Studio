// The Tauri client calls exactly the committed command names (Phase 4
// plan, decision 9; the Rust side checks the same list against its
// handlers), and the fake implements the same interface.
import { readFileSync } from 'node:fs'
import { join } from 'node:path'
import { describe, expect, it, vi } from 'vitest'

import commands from '../api/commands.json'
import { FakeApi } from '../api/fake'
import { tauriApi } from '../api/tauri'
import { makeStore, startSync } from '../store'

describe('client', () => {
  it('calls every committed command and no other', () => {
    const source = readFileSync(
      join(__dirname, '..', 'api', 'tauri.ts'),
      'utf8',
    )
    const called = [...source.matchAll(/call(?:<[^>]*>)?\('(\w+)'/g)].map(
      (m) => m[1],
    )
    expect([...new Set(called)].sort()).toEqual([...commands].sort())
  })

  it('has the same methods in the shell client and the fake', () => {
    const fake = new FakeApi()
    for (const name of Object.keys(tauriApi)) {
      expect(typeof (fake as unknown as Record<string, unknown>)[name]).toBe(
        'function',
      )
    }
  })

  it('tells the shell once when the first screen has its data', async () => {
    // A packaged smoke run quits on it (Phase 5, P1).
    const api = new FakeApi()
    const ready = vi.spyOn(api, 'ready')
    await startSync(makeStore(api), api)
    await vi.waitFor(() => expect(ready).toHaveBeenCalledOnce())
  })
})
