// The dashboards a build keeps for itself (UX audit follow-up; ADR-009
// Amendment 1): which components the build's author chose to show and how
// they are laid out, so the build looks the same to whoever opens the
// file. A typed section of the build document (`BuildView.dashboards`,
// `set-dashboards`): layout only, never a calculated number. The numbers a
// component shows are always calculated from the build as it is.
//
// The application keeps a component's kind and width as text, so a file
// from a newer version still opens; this module reads what this version
// knows and drops the rest (dropped for good on the next change).
import { useMemo } from 'react'

import {
  type DashComponent,
  type DashboardDef,
  MAX_DASHBOARDS,
  readDashboards,
} from './dashboards'
import { useAppSelector } from './util'

export interface BuildDashboards {
  list: DashboardDef[]
  /** The dashboard shown when the build is opened. */
  active: string
}

/** The build's dashboards in `raw` (the build view's section); `undefined`
 * when the build keeps none this version reads. */
export function readBuildDashboards(raw: unknown): BuildDashboards | undefined {
  if (typeof raw !== 'object' || raw === null) return undefined
  const own = raw as Record<string, unknown>
  const list = readDashboards(own.list).slice(0, MAX_DASHBOARDS)
  if (list.length === 0) return undefined
  const active =
    typeof own.active === 'string' && list.some((d) => d.id === own.active)
      ? own.active
      : list[0]!.id
  return { list, active }
}

/** A component as the build document holds it: only what its kind uses,
 * a whole number for a target. */
function written(c: DashComponent) {
  return {
    id: c.id,
    kind: c.kind as string,
    width: c.width as string,
    ...(c.field === undefined ? {} : { field: c.field }),
    ...(c.fields === undefined || c.fields.length === 0
      ? {}
      : { fields: c.fields }),
    ...(c.vary === undefined ? {} : { vary: c.vary as string }),
    ...(c.input === undefined ? {} : { input: c.input }),
    ...(c.target === undefined
      ? {}
      : { target: Math.max(1, Math.round(c.target)) }),
    ...(c.break === true ? { break: true } : {}),
  }
}

/** The section to store for `own`. */
export function writeBuildDashboards(own: BuildDashboards) {
  return {
    active: own.active,
    list: own.list.map((d) => ({
      id: d.id,
      name: d.name,
      components: d.components.map(written),
    })),
  }
}

/** The open build's own dashboards, if it keeps a set. */
export function useBuildDashboards(): BuildDashboards | undefined {
  const raw = useAppSelector((s) => s.session.snapshot?.build?.dashboards)
  return useMemo(() => readBuildDashboards(raw), [raw])
}
