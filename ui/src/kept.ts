// Snapshots of a build's numbers, kept by name to compare with later (UX
// audit follow-up). They are the user's own notes on this computer, never
// part of the build file (maintainer, 2026-11-16: a build file holds the
// build, not a record of what it was): kept in the WebView's local
// storage under the build's file, for the builds most recently worked on.
// Without storage, or for a build with no file yet, they last for the
// session.
import type { Mark } from './store'

/** The numbers of the build at one moment, kept by name. */
export interface Kept {
  id: string
  name: string
  /** Each field's value; `null` when it was not computed. */
  fields: Record<string, number | null>
}

/** Enough to compare a build's steps. */
export const MAX_SNAPSHOTS = 12
/** A snapshot's name. */
export const MAX_NAME = 40
/** The fields one snapshot keeps. */
const MAX_KEPT_FIELDS = 400
/** The builds whose snapshots are kept; the least recently changed go. */
const MAX_BUILDS = 20
const STORAGE_KEY = 'loom.kept.v1'

interface Stored {
  version: 1
  /** Most recently changed first. */
  builds: { key: string; list: Kept[] }[]
}

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

/** The snapshots in `list` this version reads. */
export function readKeptList(list: unknown): Kept[] {
  return (Array.isArray(list) ? (list as unknown[]) : [])
    .filter(isKept)
    .slice(-MAX_SNAPSHOTS)
    .map((k) => ({
      id: k.id,
      name: k.name.slice(0, MAX_NAME),
      fields: { ...k.fields },
    }))
}

function load(): Stored {
  try {
    const raw = JSON.parse(
      window.localStorage.getItem(STORAGE_KEY) ?? 'null',
    ) as Record<string, unknown> | null
    if (raw === null || raw.version !== 1 || !Array.isArray(raw.builds)) {
      return { version: 1, builds: [] }
    }
    return {
      version: 1,
      builds: (raw.builds as unknown[])
        .filter(
          (b): b is { key: string; list: unknown } =>
            typeof b === 'object' &&
            b !== null &&
            typeof (b as Record<string, unknown>).key === 'string',
        )
        .map((b) => ({ key: b.key, list: readKeptList(b.list) })),
    }
  } catch {
    // No storage, or not a document this version reads: nothing kept.
    return { version: 1, builds: [] }
  }
}

/** The snapshots kept for the build at `key` (its file). */
export function readKept(key: string): Kept[] {
  return load().builds.find((b) => b.key === key)?.list ?? []
}

/** Keeps `list` for the build at `key`; an empty list forgets the build. */
export function writeKept(key: string, list: Kept[]): void {
  const others = load().builds.filter((b) => b.key !== key)
  const builds = (
    list.length === 0 ? others : [{ key, list }, ...others]
  ).slice(0, MAX_BUILDS)
  try {
    window.localStorage.setItem(
      STORAGE_KEY,
      JSON.stringify({ version: 1, builds } satisfies Stored),
    )
  } catch {
    // No storage, or it is full: the snapshots last for the session.
  }
}

/** What a build's snapshots are kept under: where it is saved, else the
 * file it was imported from; `null` for a build with neither. */
export function keptKey(
  snapshot: { path?: string | null; origin?: string | null } | null | undefined,
): string | null {
  return snapshot?.path ?? snapshot?.origin ?? null
}

/** A kept snapshot as the numbers to compare against. */
export function markOfKept(k: Kept): Mark {
  return { version: -1, label: k.name, fields: k.fields }
}
