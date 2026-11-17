// Hover displays that are always in view (UX audit A-02): an item's
// tooltip beside its slot, a passive node's card by the pointer. Each is
// placed from its measured size, flipped to the other side when it does
// not fit, clamped to the window, and scaled down when it is taller or
// wider than the window (the game shrinks a long tooltip the same way).
// It is placed again whenever its own size or the window's changes.
import { type RefObject, useLayoutEffect, useState } from 'react'

/** Space kept between a hover display and the window's edge or anchor. */
export const GAP = 8
/** The smallest scale a display shrinks to; below it text stops reading. */
export const MIN_SCALE = 0.6

export interface Box {
  width: number
  height: number
}

export interface Edges {
  left: number
  top: number
  right: number
  bottom: number
}

export interface Placement {
  left: number
  top: number
  scale: number
}

/** The scale at which `box` fits a `view` window, with the gap around. */
export function fitScale(box: Box, view: Box): number {
  const fit = Math.min(
    1,
    (view.width - 2 * GAP) / Math.max(1, box.width),
    (view.height - 2 * GAP) / Math.max(1, box.height),
  )
  return Math.max(MIN_SCALE, fit)
}

const clamp = (value: number, low: number, high: number) =>
  Math.min(Math.max(value, low), Math.max(low, high))

/** Where a display of size `box` goes beside `anchor`: to its right, else
 * to its left, else over whichever side has more room; its top level with
 * the anchor's, moved up as far as it must to stay in the window. */
export function placeBeside(anchor: Edges, box: Box, view: Box): Placement {
  const scale = fitScale(box, view)
  const width = box.width * scale
  const height = box.height * scale
  let left = anchor.right + GAP
  if (left + width > view.width - GAP) {
    const other = anchor.left - GAP - width
    left = other >= GAP ? other : clamp(left, GAP, view.width - GAP - width)
  }
  const top = clamp(anchor.top, GAP, view.height - GAP - height)
  return { left, top, scale }
}

/** Where a display goes by a point (the pointer): below and right of it,
 * flipped to the other side of the point on an axis it does not fit. */
export function placeByPoint(
  point: { x: number; y: number },
  box: Box,
  view: Box,
  offset = 16,
): Placement {
  const scale = fitScale(box, view)
  const width = box.width * scale
  const height = box.height * scale
  let left = point.x + offset
  if (left + width > view.width - GAP) left = point.x - offset - width
  let top = point.y + offset
  if (top + height > view.height - GAP) top = point.y - offset - height
  return {
    left: clamp(left, GAP, view.width - GAP - width),
    top: clamp(top, GAP, view.height - GAP - height),
    scale,
  }
}

/** The window's size. */
function viewport(): Box {
  return { width: window.innerWidth, height: window.innerHeight }
}

/** Places the fixed element `ref` with `place`, from its own unscaled
 * size, now and whenever that size or the window's changes. `null` until
 * it is measured: the element stays hidden for that first frame, so it
 * never flashes in a wrong place. */
export function useFloating(
  ref: RefObject<HTMLElement | null>,
  place: (box: Box, view: Box) => Placement,
  key: unknown,
): Placement | null {
  const [placement, setPlacement] = useState<Placement | null>(null)
  useLayoutEffect(() => {
    const el = ref.current
    if (el === null) {
      setPlacement(null)
      return
    }
    const measure = () => {
      // offsetWidth and offsetHeight ignore the scale transform.
      const box = { width: el.offsetWidth, height: el.offsetHeight }
      const next = place(box, viewport())
      setPlacement((old) =>
        old !== null &&
        old.left === next.left &&
        old.top === next.top &&
        old.scale === next.scale
          ? old
          : next,
      )
    }
    measure()
    const observer =
      typeof ResizeObserver === 'undefined' ? null : new ResizeObserver(measure)
    observer?.observe(el)
    window.addEventListener('resize', measure)
    return () => {
      observer?.disconnect()
      window.removeEventListener('resize', measure)
    }
    // `place` is rebuilt each render; `key` says when its inputs change.
    // oxlint-disable-next-line react-hooks/exhaustive-deps
  }, [ref, key])
  return placement
}

/** The style for a placed display (hidden until measured). */
export function floatingStyle(p: Placement | null): React.CSSProperties {
  return p === null
    ? { left: 0, top: 0, visibility: 'hidden' }
    : {
        left: p.left,
        top: p.top,
        transform: p.scale === 1 ? undefined : `scale(${p.scale})`,
        transformOrigin: 'top left',
      }
}
