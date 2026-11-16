// The build's own workspace document (UX audit follow-up, maintainer
// 2026-11-16): the dashboards a build keeps for itself and the snapshots
// of its numbers the user kept by name. The application stores it in the
// build file as text it does not read (`BuildView.workspace`,
// `set-workspace`), as it stores the UI's preferences; this module is the
// only reader and writer. A document this version does not read is the
// empty one (a preference, never an error); what it does not know in a
// document it reads is dropped on the next write.
import { useMemo } from 'react'

import { type DashboardDef, MAX_DASHBOARDS, readDashboards } from './dashboards'
import type { Mark } from './store'
import { useAppSelector } from './util'

/** The numbers of the build at one moment, kept by name. */
export interface Kept {
  id: string
  name: string
  /** Each field's value; `null` when it was not computed. */
  fields: Record<string, number | null>
}

export interface Workspace {
  version: 1
  /** The build's own dashboards; absent while it uses the user's. */
  dashboards?: { list: DashboardDef[]; active: string }
  /** The kept snapshots, oldest first. */
  snapshots: Kept[]
}

/** Enough to compare a build's steps; keeps the build file small. */
export const MAX_SNAPSHOTS = 12
/** A snapshot's name. */
export const MAX_NAME = 40
/** The fields one snapshot keeps (every field the calculation has, with
 * room; a document from elsewhere cannot grow the file without bound). */
const MAX_KEPT_FIELDS = 400

export const emptyWorkspace = (): Workspace => ({ version: 1, snapshots: [] })

function isKept(x: unknown): x is Kept {
  if (typeof x !== 'object' || x === null) return false
  const k = x as Record<string, unknown>
  if (typeof k.id !== 'string' || typeof k.name !== 'string') return false
  if (typeof k.fields !== 'object' || k.fields === null) return false
  const values = Object.values(k.fields as Record<string, unknown>)
  return (
    values.length <= MAX_KEPT_FIELDS &&
    values.every(
      (v) => v === null || (typeof v === 'number' && Number.isFinite(v)),
    )
  )
}

/** The workspace in `text`; the empty one when there is none or it is not
 * a document this version reads. */
export function readWorkspace(text: string | null | undefined): Workspace {
  if (text == null) return emptyWorkspace()
  try {
    const raw = JSON.parse(text) as Record<string, unknown>
    if (raw.version !== 1) return emptyWorkspace()
    const snapshots = (Array.isArray(raw.snapshots) ? raw.snapshots : [])
      .filter(isKept)
      .slice(-MAX_SNAPSHOTS)
      .map((k) => ({
        id: k.id,
        name: k.name.slice(0, MAX_NAME),
        fields: { ...k.fields },
      }))
    const own = raw.dashboards as Record<string, unknown> | undefined
    const list =
      typeof own === 'object' && own !== null
        ? readDashboards(own.list).slice(0, MAX_DASHBOARDS)
        : []
    if (list.length === 0) return { version: 1, snapshots }
    const active =
      typeof own?.active === 'string' && list.some((d) => d.id === own.active)
        ? own.active
        : list[0]!.id
    return { version: 1, dashboards: { list, active }, snapshots }
  } catch {
    return emptyWorkspace()
  }
}

/** The document to store; `null` for the empty workspace (the build file
 * then has none). */
export function writeWorkspace(w: Workspace): string | null {
  if (w.dashboards === undefined && w.snapshots.length === 0) return null
  return JSON.stringify(w)
}

/** A kept snapshot as the numbers to compare against. */
export function markOfKept(k: Kept): Mark {
  return { version: -1, label: k.name, fields: k.fields }
}

/** The open build's workspace (the empty one with no build). */
export function useWorkspace(): Workspace {
  const text = useAppSelector((s) => s.session.snapshot?.build?.workspace)
  return useMemo(() => readWorkspace(text), [text])
}
