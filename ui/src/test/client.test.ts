// The Tauri client calls exactly the committed command names (Phase 4
// plan, decision 9; the Rust side checks the same list against its
// handlers), and the fake implements the same interface.
import { readFileSync } from 'node:fs'
import { join } from 'node:path'
import { describe, expect, it } from 'vitest'

import commands from '../api/commands.json'
import { FakeApi } from '../api/fake'
import { tauriApi } from '../api/tauri'

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
})
