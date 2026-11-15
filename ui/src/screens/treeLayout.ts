// The loaded data's passive tree layout (the shell's `tree_layout`),
// fetched once per data load and shared by the screens that name nodes.
import { useEffect, useState } from 'react'

import type { TreeLayout } from '../api/types'
import { query } from '../store'
import { useAppDispatch, useAppSelector } from '../util'

/** The layout, kept while the same data is loaded. */
let cached: { key: string; layout: TreeLayout } | null = null

/** The loaded data's tree: `null` while it loads, `'none'` without one. */
export function useTreeLayout(): TreeLayout | 'none' | null {
  const dispatch = useAppDispatch()
  const data = useAppSelector((s) => s.session.snapshot?.data)
  const key =
    data?.bundle == null
      ? null
      : `${data.bundle}:${data['data-version']?.['steam-build'] ?? ''}`
  const [fetched, setFetched] = useState<{
    key: string
    layout: TreeLayout | 'none'
  } | null>(null)
  useEffect(() => {
    if (key === null || (cached !== null && cached.key === key)) return
    let live = true
    void dispatch(query((api) => api.treeLayout())).then((l) => {
      if (!live) return
      if (l !== null) cached = { key, layout: l }
      setFetched({ key, layout: l ?? 'none' })
    })
    return () => {
      live = false
    }
  }, [dispatch, key])
  if (key === null) return 'none'
  if (cached !== null && cached.key === key) return cached.layout
  return fetched !== null && fetched.key === key ? fetched.layout : null
}
