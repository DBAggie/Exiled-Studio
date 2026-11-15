// The loaded data's passive tree layout (the shell's `tree_layout`),
// fetched once per data load and shared by the screens that name nodes,
// and its sprites (`tree_sprites`), when the bundle has them.
import { useEffect, useState } from 'react'

import type { TreeLayout, TreeSprites } from '../api/types'
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

/** A sprite's place on its loaded sheet, in pixels. */
export interface SpriteRect {
  image: HTMLImageElement
  x: number
  y: number
  w: number
  h: number
}

/** The sprites, ready to draw: by category, then by sprite name. */
export interface LoadedSprites {
  /** Tree units per pixel of a sheet (1 / the export's zoom). */
  unit: number
  sprite(category: string, name: string | null | undefined): SpriteRect | null
  /** A node's art, as the export names it. */
  art: Map<number, TreeSprites['nodes'][number]>
}

function load(s: TreeSprites): Promise<LoadedSprites> {
  return Promise.all(
    s.sheets.map(
      (sheet) =>
        new Promise<[string, HTMLImageElement]>((resolve, reject) => {
          const image = new Image()
          image.onload = () => {
            resolve([sheet.file, image])
          }
          image.onerror = () => {
            reject(new Error(sheet.file))
          }
          image.src = sheet.image
        }),
    ),
  ).then((loaded) => {
    const images = new Map(loaded)
    const categories = new Map(
      s.categories.map((c) => [
        c.name,
        new Map(
          c.sprites.map((r) => [
            r.name,
            { image: images.get(c.file), x: r.x, y: r.y, w: r.w, h: r.h },
          ]),
        ),
      ]),
    )
    return {
      unit: 1 / s.zoom,
      sprite(category, name) {
        if (name == null) return null
        const r = categories.get(category)?.get(name)
        return r?.image === undefined ? null : { ...r, image: r.image }
      },
      art: new Map(s.nodes.map((n) => [n.node, n])),
    }
  })
}

/** The sprites, kept while the same data is loaded. */
let cachedSprites: { key: string; sprites: LoadedSprites | null } | null = null

/** The loaded data's tree sprites: `null` while they load or when the
 * bundle has none (the canvas then draws its own shapes). */
export function useTreeSprites(): LoadedSprites | null {
  const dispatch = useAppDispatch()
  const data = useAppSelector((s) => s.session.snapshot?.data)
  const key =
    data?.bundle == null
      ? null
      : `${data.bundle}:${data['data-version']?.['steam-build'] ?? ''}`
  const [fetched, setFetched] = useState<{
    key: string
    sprites: LoadedSprites | null
  } | null>(null)
  useEffect(() => {
    if (key === null || (cachedSprites !== null && cachedSprites.key === key))
      return
    let live = true
    void dispatch(query((api) => api.treeSprites()))
      .then((s) => (s === null ? null : load(s)))
      .catch(() => null)
      .then((sprites) => {
        cachedSprites = { key, sprites }
        if (live) setFetched({ key, sprites })
      })
    return () => {
      live = false
    }
  }, [dispatch, key])
  if (key === null) return null
  if (cachedSprites !== null && cachedSprites.key === key)
    return cachedSprites.sprites
  return fetched !== null && fetched.key === key ? fetched.sprites : null
}
