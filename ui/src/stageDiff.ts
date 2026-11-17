// Comparing the active stage with another (UX audit follow-up, maintainer
// 2026-11-16): which passive nodes and which equipped items differ. The
// other stage is read from the application (`stage_view`), never made
// active, so comparing changes nothing in the build. The choice of stage
// is shared by the tree and the equipment pages for the session.
import { useEffect, useMemo, useState } from 'react'

import type { BuildView, StageView } from './api/types'
import { query } from './store'
import { useAppDispatch, useAppSelector } from './util'

/** The stage compared with, when the build still has it and it is not
 * the active one. */
export function useCompareStage(build: BuildView | null | undefined) {
  const spec = useAppSelector((s) => s.ui.compareStage)
  const stage = build?.loadouts.find((l) => l.spec === spec && !l.active)
  return stage ?? null
}

type Loaded =
  | { state: 'none' }
  | { state: 'loading' }
  | { state: 'failed' }
  | { state: 'done'; view: StageView }

/** The other stage's tree and equipment, read again when the build
 * changes. */
export function useStageView(spec: number | null): Loaded {
  const dispatch = useAppDispatch()
  const version = useAppSelector((s) => s.session.snapshot?.version)
  const [held, setHeld] = useState<{
    key: string
    view: StageView | null
  } | null>(null)
  const key = spec === null ? null : `${spec}:${version}`
  useEffect(() => {
    if (spec === null || key === null) return
    let stale = false
    void dispatch(query((api) => api.stageView(spec))).then((view) => {
      if (!stale) setHeld({ key, view })
    })
    return () => {
      stale = true
    }
  }, [dispatch, spec, key])
  return useMemo((): Loaded => {
    if (key === null) return { state: 'none' }
    if (held === null || held.key !== key) return { state: 'loading' }
    return held.view === null
      ? { state: 'failed' }
      : { state: 'done', view: held.view }
  }, [key, held])
}

/** The allocated nodes that differ between the active spec and another
 * stage's. */
export function nodeDiff(
  here: number[],
  other: number[],
): { added: number[]; removed: number[] } {
  const mine = new Set(here)
  const theirs = new Set(other)
  return {
    added: other.filter((n) => !mine.has(n)),
    removed: here.filter((n) => !theirs.has(n)),
  }
}

/** The slots whose item differs between the active stage and another,
 * in the other stage's then this stage's slot order, and how many slots
 * hold the same item. */
export function slotDiff(
  here: { slot: string; item: number }[],
  other: { slot: string; item: number }[],
): {
  rows: { slot: string; here: number | null; other: number | null }[]
  same: number
} {
  const mine = new Map(here.map((s) => [s.slot, s.item]))
  const theirs = new Map(other.map((s) => [s.slot, s.item]))
  const slots = [...new Set([...mine.keys(), ...theirs.keys()])]
  const rows = slots
    .map((slot) => ({
      slot,
      here: mine.get(slot) ?? null,
      other: theirs.get(slot) ?? null,
    }))
    .filter((r) => r.here !== r.other)
  return { rows, same: slots.length - rows.length }
}
