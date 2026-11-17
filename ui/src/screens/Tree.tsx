// The passive tree (passive tree plan, T6; ADR-008 Amendment 1): the
// loaded data's tree drawn on a canvas, with GGG's published sprites when
// the bundle carries them (T-a, ADR-005 Amendment 2) and in our own shapes
// otherwise, with a DOM card over it, and a list view that does everything
// the canvas does, for keyboard and screen reader users.
//
// Clicking a node allocates it and the path to it, or deallocates it and
// what it leaves unconnected; hovering shows what that would do and its
// effect on the calculated fields first (the application's preview,
// never committed). A mastery opens its effect picker, and an allocated
// jewel socket its jewel picker (cluster jewels aside). The class,
// ascendancy, and a reset are one undoable change each. Nodes the editor
// keeps but does not edit (bloodline passives, cluster jewels' nodes) are
// drawn faded and listed as kept.
import { Dialog } from 'radix-ui'
import {
  useCallback,
  useEffect,
  useId,
  useMemo,
  useRef,
  useState,
  type PointerEvent as ReactPointerEvent,
  type ReactNode,
} from 'react'

import type {
  TreeLayout,
  TreeNodeView,
  TreePreview,
  TreeView,
} from '../api/types'
import { Hint, ScreenHeading } from '../components'
import { floatingStyle, placeByPoint, useFloating } from '../floating'
import { fieldLabel, formatNumber, t, tDynamic } from '../i18n'
import { query, run } from '../store'
import {
  useAppDispatch,
  useAppSelector,
  useAttributes,
  withUnit,
} from '../util'
import { nodeDiff, useCompareStage, useStageView } from '../stageDiff'
import { StageCompareSelect } from './StageDiff'
import {
  type LoadedSprites,
  type SpriteRect,
  clusterSize,
  fits,
  useTreeLayout,
  useTreeSprites,
  withClusterNodes,
} from './treeLayout'

/** The hovered node's card, by the pointer and always whole in the
 * window. `sized` changes when its content does. */
function HoverCard({
  x,
  y,
  sized,
  children,
}: {
  x: number
  y: number
  sized: unknown
  children: ReactNode
}) {
  const ref = useRef<HTMLDivElement>(null)
  const placement = useFloating(
    ref,
    (box, view) => placeByPoint({ x, y }, box, view),
    `${x}:${y}:${sized === null ? 0 : 1}`,
  )
  return (
    <div
      ref={ref}
      className="tree-card"
      role="tooltip"
      style={floatingStyle(placement)}
    >
      {children}
    </div>
  )
}

/** A node's drawn radius by kind, in tree units. */
const RADIUS: Record<TreeNodeView['kind'], number> = {
  normal: 26,
  notable: 40,
  keystone: 56,
  mastery: 36,
  'jewel-socket': 34,
  'cluster-socket': 34,
  'class-start': 70,
  'ascendancy-start': 30,
}

const kindText = (n: TreeNodeView) =>
  tDynamic(`tree.kind.${n.kind}`, 'tree.kind.normal')

function matches(n: TreeNodeView, text: string): boolean {
  const q = text.trim().toLowerCase()
  if (q === '') return false
  return (
    n.name.toLowerCase().includes(q) ||
    n.lines.some((l) => l.toLowerCase().includes(q))
  )
}

/** The token colours the canvas draws with (the CSS has them). */
function colours(el: Element) {
  const s = getComputedStyle(el)
  const c = (name: string, fallback: string) =>
    s.getPropertyValue(name).trim() || fallback
  return {
    bg: c('--c-tree-bg', 'black'),
    link: c('--c-tree-link', 'gray'),
    linkOn: c('--c-tree-link-on', 'orange'),
    node: c('--c-tree-node', 'gray'),
    nodeOn: c('--c-tree-node-on', 'orange'),
    path: c('--c-tree-path', 'skyblue'),
    removed: c('--c-tree-removed', 'red'),
    match: c('--c-tree-match', 'yellow'),
    kept: c('--c-tree-kept', 'dimgray'),
    ring: c('--c-tree-ring', 'white'),
  }
}

/** The nodes only the other stage has (`added`) and only this one has. */
interface StageNodes {
  added: Set<number>
  removed: Set<number>
}

interface Camera {
  x: number
  y: number
  scale: number
}

/**
 * Draws a node's sprites centred on `p` (`scale`: screen pixels per tree
 * unit): its icon, then its frame, allocated or not, as the export names
 * them; a mastery's icon unconnected, connected (`groupOn`), or chosen.
 * False, drawing nothing, when the sprites lack the node's art (class and
 * ascendancy starts are always our shapes).
 */
function drawArt(
  ctx: CanvasRenderingContext2D,
  s: LoadedSprites,
  n: TreeNodeView,
  p: { x: number; y: number },
  scale: number,
  on: boolean,
  groupOn: boolean,
): boolean {
  const art = s.art.get(n.id)
  const put = (r: SpriteRect | null) => {
    if (r === null) return false
    const w = r.w * s.unit * scale
    const h = r.h * s.unit * scale
    ctx.drawImage(r.image, r.x, r.y, r.w, r.h, p.x - w / 2, p.y - h / 2, w, h)
    return true
  }
  const framed = (icons: string, frame: string) => {
    const icon = s.sprite(`${icons}${on ? 'Active' : 'Inactive'}`, art?.icon)
    const border = s.sprite('frame', frame)
    if (icon === null || border === null) return false
    put(icon)
    put(border)
    return true
  }
  switch (n.kind) {
    case 'normal':
      return framed('normal', on ? 'PSSkillFrameActive' : 'PSSkillFrame')
    case 'notable':
      return framed(
        'notable',
        on ? 'NotableFrameAllocated' : 'NotableFrameUnallocated',
      )
    case 'keystone':
      return framed(
        'keystone',
        on ? 'KeystoneFrameAllocated' : 'KeystoneFrameUnallocated',
      )
    case 'jewel-socket':
    case 'cluster-socket':
      return put(
        s.sprite('frame', on ? 'JewelFrameAllocated' : 'JewelFrameUnallocated'),
      )
    case 'mastery':
      return on
        ? put(s.sprite('masteryActiveSelected', art?.['active-icon']))
        : put(
            s.sprite(
              groupOn ? 'masteryConnected' : 'masteryInactive',
              art?.['inactive-icon'],
            ),
          )
    case 'class-start':
    case 'ascendancy-start':
      return false
  }
}

function TreeCanvas({
  layout,
  sprites,
  tree,
  found,
  preview,
  diff,
  onHover,
  onActivate,
}: {
  layout: TreeLayout
  sprites: LoadedSprites | null
  tree: TreeView
  found: Set<number>
  preview: TreePreview | null
  /** The nodes that differ from the stage compared with. */
  diff: StageNodes | null
  onHover: (node: TreeNodeView | null, at: { x: number; y: number }) => void
  onActivate: (node: TreeNodeView) => void
}) {
  const canvas = useRef<HTMLCanvasElement>(null)
  const [camera, setCamera] = useState<Camera | null>(null)
  const drag = useRef<{ x: number; y: number; moved: boolean } | null>(null)
  const placed = useMemo(
    () =>
      layout.nodes.filter(
        (n) => n.position != null && n['cluster-area'] !== true,
      ),
    [layout],
  )
  const byId = useMemo(
    () => new Map(layout.nodes.map((n) => [n.id, n])),
    [layout],
  )
  const allocated = useMemo(() => new Set(tree.nodes), [tree])
  // The groups with an allocated node: their masteries show as connected.
  const groupsOn = useMemo(
    () =>
      new Set(
        layout.nodes
          .filter((n) => n.kind !== 'mastery' && allocated.has(n.id))
          .map((n) => n.group),
      ),
    [allocated, layout],
  )
  const path = useMemo(() => new Set(preview?.path ?? []), [preview])
  const removed = useMemo(() => new Set(preview?.removed ?? []), [preview])

  // Fit the tree on first draw.
  const fit = useCallback(() => {
    const el = canvas.current
    if (el === null || placed.length === 0) return
    const xs = placed.map((n) => n.position?.x ?? 0)
    const ys = placed.map((n) => n.position?.y ?? 0)
    const [x0, x1, y0, y1] = [
      Math.min(...xs),
      Math.max(...xs),
      Math.min(...ys),
      Math.max(...ys),
    ]
    const w = el.clientWidth || 800
    const h = el.clientHeight || 600
    const scale = Math.min(w / (x1 - x0 + 400), h / (y1 - y0 + 400))
    setCamera({ x: (x0 + x1) / 2, y: (y0 + y1) / 2, scale })
  }, [placed])
  useEffect(() => {
    if (camera === null) fit()
  }, [camera, fit])

  const toScreen = useCallback(
    (x: number, y: number, el: HTMLCanvasElement, cam: Camera) => ({
      x: (x - cam.x) * cam.scale + el.clientWidth / 2,
      y: (y - cam.y) * cam.scale + el.clientHeight / 2,
    }),
    [],
  )

  // Draw.
  useEffect(() => {
    const el = canvas.current
    if (el === null || camera === null) return
    const ctx = el.getContext('2d')
    if (ctx === null) return
    const dpr = window.devicePixelRatio || 1
    el.width = Math.round(el.clientWidth * dpr)
    el.height = Math.round(el.clientHeight * dpr)
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0)
    const c = colours(el)
    ctx.fillStyle = c.bg
    ctx.fillRect(0, 0, el.clientWidth, el.clientHeight)
    const at = (n: TreeNodeView) =>
      toScreen(n.position?.x ?? 0, n.position?.y ?? 0, el, camera)
    // Links: an arc when both ends share a group and an orbit, else a line.
    for (const a of placed) {
      if (a.kind === 'mastery') continue
      for (const id of a.links) {
        const b = byId.get(id)
        if (
          b?.position == null ||
          b['cluster-area'] === true ||
          b.kind === 'mastery' ||
          a.ascendancy !== b.ascendancy ||
          (a.kind === 'class-start' && b.kind === 'ascendancy-start') ||
          (b.kind === 'class-start' && a.kind === 'ascendancy-start')
        )
          continue
        const on = allocated.has(a.id) && allocated.has(b.id)
        const onPath =
          (path.has(a.id) || allocated.has(a.id)) &&
          (path.has(b.id) || allocated.has(b.id)) &&
          (path.has(a.id) || path.has(b.id))
        const gone = removed.has(a.id) || removed.has(b.id)
        ctx.strokeStyle = gone
          ? c.removed
          : onPath
            ? c.path
            : on
              ? c.linkOn
              : c.link
        ctx.lineWidth = Math.max(1, (on || onPath ? 10 : 5) * camera.scale)
        ctx.globalAlpha = a.modelled && b.modelled ? 1 : 0.35
        ctx.beginPath()
        const pa = at(a)
        const pb = at(b)
        const g = layout.groups[a.group]
        if (
          a.group === b.group &&
          a.orbit === b.orbit &&
          a.orbit > 0 &&
          g !== undefined
        ) {
          const centre = toScreen(g.x, g.y, el, camera)
          const r = (layout['orbit-radii'][a.orbit] ?? 0) * camera.scale
          let s = Math.atan2(pa.y - centre.y, pa.x - centre.x)
          let e = Math.atan2(pb.y - centre.y, pb.x - centre.x)
          // The shorter way round.
          let d = e - s
          while (d > Math.PI) d -= 2 * Math.PI
          while (d < -Math.PI) d += 2 * Math.PI
          if (d < 0) [s, e] = [e, s]
          ctx.arc(centre.x, centre.y, r, s, e)
        } else {
          ctx.moveTo(pa.x, pa.y)
          ctx.lineTo(pb.x, pb.y)
        }
        ctx.stroke()
      }
    }
    ctx.globalAlpha = 1
    for (const n of placed) {
      const p = at(n)
      // A class start keeps a readable size when the whole tree shows.
      const r = Math.max(
        n.kind === 'class-start' ? 8 : 2,
        RADIUS[n.kind] * camera.scale,
      )
      const on = allocated.has(n.id)
      ctx.globalAlpha = n.modelled ? 1 : 0.4
      if (
        sprites !== null &&
        drawArt(ctx, sprites, n, p, camera.scale, on, groupsOn.has(n.group))
      ) {
        // The preview's path and removals as rings over the art.
        if (path.has(n.id) || removed.has(n.id)) {
          ctx.lineWidth = Math.max(2, 8 * camera.scale)
          ctx.strokeStyle = removed.has(n.id) ? c.removed : c.path
          ctx.beginPath()
          ctx.arc(p.x, p.y, r + 2, 0, 2 * Math.PI)
          ctx.stroke()
        }
        if (found.has(n.id)) {
          ctx.lineWidth = 2
          ctx.strokeStyle = c.match
          ctx.beginPath()
          ctx.arc(p.x, p.y, r + 5, 0, 2 * Math.PI)
          ctx.stroke()
        }
        continue
      }
      ctx.fillStyle = removed.has(n.id)
        ? c.removed
        : path.has(n.id)
          ? c.path
          : on
            ? c.nodeOn
            : n.modelled
              ? c.node
              : c.kept
      ctx.beginPath()
      if (n.kind === 'keystone' || n.kind === 'class-start') {
        // An octagon.
        for (let i = 0; i < 8; i += 1) {
          const a = (Math.PI / 4) * i + Math.PI / 8
          const x = p.x + r * Math.cos(a)
          const y = p.y + r * Math.sin(a)
          if (i === 0) ctx.moveTo(x, y)
          else ctx.lineTo(x, y)
        }
        ctx.closePath()
      } else if (n.kind === 'jewel-socket' || n.kind === 'cluster-socket') {
        ctx.moveTo(p.x, p.y - r)
        ctx.lineTo(p.x + r, p.y)
        ctx.lineTo(p.x, p.y + r)
        ctx.lineTo(p.x - r, p.y)
        ctx.closePath()
      } else {
        ctx.arc(p.x, p.y, r, 0, 2 * Math.PI)
      }
      if (n.kind === 'mastery') {
        ctx.lineWidth = Math.max(1, 6 * camera.scale)
        ctx.strokeStyle = ctx.fillStyle
        ctx.stroke()
      } else {
        ctx.fill()
      }
      if (
        n.kind === 'notable' ||
        n.kind === 'keystone' ||
        n.kind === 'class-start'
      ) {
        // The class start's outline keeps its octagon readable when small.
        ctx.lineWidth = Math.max(
          n.kind === 'class-start' ? 2 : 1,
          4 * camera.scale,
        )
        ctx.strokeStyle = c.ring
        ctx.stroke()
      }
      if (found.has(n.id)) {
        ctx.lineWidth = 2
        ctx.strokeStyle = c.match
        ctx.beginPath()
        ctx.arc(p.x, p.y, r + 5, 0, 2 * Math.PI)
        ctx.stroke()
      }
    }
    ctx.globalAlpha = 1
    // The stage compared with: a dashed ring on a node only it has, a
    // solid ring on a node only this stage has (the line, not the colour
    // alone, tells them apart).
    if (diff !== null) {
      for (const n of placed) {
        const other = diff.added.has(n.id)
        if (!other && !diff.removed.has(n.id)) continue
        const p = at(n)
        const r = Math.max(4, RADIUS[n.kind] * camera.scale) + 4
        ctx.lineWidth = 2.5
        ctx.strokeStyle = other ? c.path : c.removed
        ctx.setLineDash(other ? [5, 4] : [])
        ctx.beginPath()
        ctx.arc(p.x, p.y, r, 0, 2 * Math.PI)
        ctx.stroke()
      }
      ctx.setLineDash([])
    }
  }, [
    allocated,
    byId,
    camera,
    diff,
    found,
    groupsOn,
    layout,
    path,
    placed,
    removed,
    sprites,
    toScreen,
  ])

  const hit = (e: ReactPointerEvent<HTMLCanvasElement>) => {
    const el = canvas.current
    if (el === null || camera === null) return null
    const rect = el.getBoundingClientRect()
    const x =
      (e.clientX - rect.left - el.clientWidth / 2) / camera.scale + camera.x
    const y =
      (e.clientY - rect.top - el.clientHeight / 2) / camera.scale + camera.y
    let best: TreeNodeView | null = null
    let bestD = Infinity
    for (const n of placed) {
      const dx = (n.position?.x ?? 0) - x
      const dy = (n.position?.y ?? 0) - y
      const d = Math.hypot(dx, dy)
      const reach = RADIUS[n.kind] + 6 / camera.scale
      if (d <= reach && d < bestD) {
        best = n
        bestD = d
      }
    }
    return best
  }

  const zoom = (factor: number, cx?: number, cy?: number) => {
    const el = canvas.current
    if (el === null || camera === null) return
    const scale = Math.min(2, Math.max(0.01, camera.scale * factor))
    // Keep the point under the pointer still.
    const px = (cx ?? el.clientWidth / 2) - el.clientWidth / 2
    const py = (cy ?? el.clientHeight / 2) - el.clientHeight / 2
    setCamera({
      scale,
      x: camera.x + px / camera.scale - px / scale,
      y: camera.y + py / camera.scale - py / scale,
    })
  }

  return (
    <div className="tree-canvas-wrap">
      <canvas
        ref={canvas}
        className="tree-canvas"
        // The list view is the accessible equivalent (ADR-008).
        aria-hidden="true"
        onWheel={(e) => {
          const rect = e.currentTarget.getBoundingClientRect()
          zoom(
            e.deltaY < 0 ? 1.2 : 1 / 1.2,
            e.clientX - rect.left,
            e.clientY - rect.top,
          )
        }}
        onPointerDown={(e) => {
          drag.current = { x: e.clientX, y: e.clientY, moved: false }
          e.currentTarget.setPointerCapture?.(e.pointerId)
        }}
        onPointerMove={(e) => {
          const d = drag.current
          if (d !== null && camera !== null) {
            const dx = e.clientX - d.x
            const dy = e.clientY - d.y
            if (d.moved || Math.hypot(dx, dy) > 4) {
              d.moved = true
              d.x = e.clientX
              d.y = e.clientY
              setCamera({
                ...camera,
                x: camera.x - dx / camera.scale,
                y: camera.y - dy / camera.scale,
              })
              return
            }
          }
          // Window coordinates: the card is placed in the window, so it is
          // never cut off by the canvas or the page (UX audit A-02).
          onHover(hit(e), { x: e.clientX, y: e.clientY })
        }}
        onPointerUp={(e) => {
          const d = drag.current
          drag.current = null
          if (d !== null && !d.moved) {
            const n = hit(e)
            if (n !== null) onActivate(n)
          }
        }}
        onPointerLeave={() => onHover(null, { x: 0, y: 0 })}
      />
      <div className="tree-zoom" role="group" aria-label={t('tree.zoom')}>
        <button
          type="button"
          onClick={() => zoom(1.25)}
          aria-label={t('tree.zoomIn')}
        >
          +
        </button>
        <button
          type="button"
          onClick={() => zoom(0.8)}
          aria-label={t('tree.zoomOut')}
        >
          −
        </button>
        <button type="button" onClick={fit}>
          {t('tree.fit')}
        </button>
      </div>
    </div>
  )
}

/** A preview's changed fields. */
function Impact({ preview }: { preview: TreePreview }) {
  const changed = preview.fields.filter(
    (f) => f.before !== f.after || f['state-before'] !== f['state-after'],
  )
  if (changed.length === 0) return null
  return (
    <ul className="impact-list">
      {changed.map((f) => {
        const delta =
          f.before != null && f.after != null ? f.after - f.before : null
        return (
          <li key={f.field}>
            <span>{fieldLabel(f.field)}</span>
            <span className="row">
              <span className="before">
                {f.before == null ? '—' : withUnit(f.field, f.before)} →
              </span>
              <span className="after">
                {f.after == null ? '—' : withUnit(f.field, f.after)}
              </span>
              {delta !== null && delta !== 0 && (
                <span className={delta > 0 ? 'delta-up' : 'delta-down'}>
                  {delta > 0 ? '+' : '−'}
                  {withUnit(f.field, Math.abs(delta))}
                </span>
              )}
            </span>
          </li>
        )
      })}
    </ul>
  )
}

/** A flavour text or quote, line by line, as the game prints it. */
function Flavour({ text }: { text: string }) {
  return (
    <p className="tree-flavour">
      {text.split('\n').map((line, i) => (
        <span key={i}>
          {i > 0 && <br />}
          {line}
        </span>
      ))}
    </p>
  )
}

/** What a node is and what clicking it would do. */
function NodeCard({
  node,
  tree,
  layout,
  preview,
}: {
  node: TreeNodeView
  tree: TreeView
  layout: TreeLayout
  preview: TreePreview | null | 'refused'
}) {
  const chosen = tree.masteries.find((m) => m.node === node.id)
  const effect = layout.masteries
    .find((g) => g.id === node['mastery-group'])
    ?.effects.find((e) => e.effect === chosen?.effect)
  const allocated = tree.nodes.includes(node.id)
  // A keystone's flavour text; an ascendancy start shows its quote (B-2
  // item evidence, E10: the game data's).
  const flavour =
    node.flavour ??
    (node.kind === 'ascendancy-start'
      ? layout.classes
          .flatMap((c) => c.ascendancies)
          .find((a) => a.id === node.ascendancy)?.flavour
      : null) ??
    null
  const jewel = tree.jewels?.find((j) => j.node === node.id)?.item
  const jewelName = useAppSelector(
    (s) =>
      s.session.snapshot?.build?.items.find((i) => i.id === jewel)?.name ??
      null,
  )
  return (
    <>
      <p className="tree-card-name">{node.name}</p>
      <p className="muted small-text">
        {kindText(node)}
        {node.ascendancy != null && ` · ${node.ascendancy}`}
      </p>
      {node.lines.length > 0 && (
        <ul className="tree-lines">
          {node.lines.map((l, i) => (
            <li key={i}>{l}</li>
          ))}
        </ul>
      )}
      {flavour != null && <Flavour text={flavour} />}
      {(node.kind === 'jewel-socket' || node.kind === 'cluster-socket') &&
        allocated && (
          <p className="small-text">{jewelName ?? t('tree.emptySocket')}</p>
        )}
      {node.kind === 'mastery' && (
        <ul className="tree-lines">
          {(effect?.lines ?? [t('tree.chooseEffect')]).map((l, i) => (
            <li key={i}>{l}</li>
          ))}
        </ul>
      )}
      {!node.modelled ? (
        <p className="muted small-text">{t('tree.keptNote')}</p>
      ) : tree.locked != null ? null : preview === 'refused' ? (
        node.kind !== 'class-start' &&
        node.kind !== 'ascendancy-start' &&
        node.kind !== 'mastery' && (
          <p className="muted small-text">{t('tree.unreachable')}</p>
        )
      ) : preview !== null ? (
        <>
          <p className="small-text">
            {allocated
              ? t('tree.removes', { n: preview.removed.length })
              : t('tree.allocates', { n: preview.path.length })}
          </p>
          <Impact preview={preview} />
        </>
      ) : null}
    </>
  )
}

function MasteryPicker({
  node,
  tree,
  layout,
  onClose,
}: {
  node: TreeNodeView | null
  tree: TreeView
  layout: TreeLayout
  onClose: () => void
}) {
  const dispatch = useAppDispatch()
  const group = layout.masteries.find((g) => g.id === node?.['mastery-group'])
  const chosen = tree.masteries.find((m) => m.node === node?.id)?.effect
  const takenBy = (effect: number) =>
    tree.masteries.find(
      (m) =>
        m.effect === effect &&
        m.node !== node?.id &&
        layout.nodes.find((n) => n.id === m.node)?.['mastery-group'] ===
          node?.['mastery-group'],
    )
  return (
    <Dialog.Root open={node !== null} onOpenChange={(o) => !o && onClose()}>
      <Dialog.Portal>
        <Dialog.Content className="dialog-content picker">
          <Dialog.Title>{node?.name}</Dialog.Title>
          <Dialog.Description className="muted">
            {t('tree.masteryHelp')}
          </Dialog.Description>
          <ul className="picker-list" aria-label={t('tree.effects')}>
            {(group?.effects ?? []).map((e) => {
              const taken = takenBy(e.effect) !== undefined
              return (
                <li key={e.effect}>
                  <button
                    type="button"
                    className="picker-entry"
                    aria-pressed={chosen === e.effect}
                    disabled={taken}
                    onClick={() => {
                      if (node === null) return
                      void dispatch(
                        run({
                          'set-mastery-effect': {
                            spec: tree.spec,
                            node: node.id,
                            effect: e.effect,
                          },
                        }),
                      ).then(onClose)
                    }}
                  >
                    <span className="picker-name">{e.lines.join(' · ')}</span>
                    {taken && (
                      <span className="muted small-text">
                        {t('tree.taken')}
                      </span>
                    )}
                  </button>
                </li>
              )
            })}
          </ul>
          <div className="row">
            {chosen !== undefined && node !== null && (
              <button
                type="button"
                onClick={() =>
                  void dispatch(
                    run({
                      'deallocate-node': { spec: tree.spec, node: node.id },
                    }),
                  ).then(onClose)
                }
              >
                {t('tree.removeMastery')}
              </button>
            )}
            <span className="spacer" />
            <Dialog.Close asChild>
              <button type="button" className="primary">
                {t('picker.done')}
              </button>
            </Dialog.Close>
          </div>
        </Dialog.Content>
      </Dialog.Portal>
    </Dialog.Root>
  )
}

/** The item classes a tree socket takes (the application's
 * `JEWEL_CLASSES`). */
const JEWEL_CLASSES = ['Jewel', 'AbyssJewel']

function JewelPicker({
  node,
  tree,
  onClose,
}: {
  node: TreeNodeView | null
  tree: TreeView
  onClose: () => void
}) {
  const dispatch = useAppDispatch()
  const items = useAppSelector((s) => s.session.snapshot?.build?.items ?? [])
  // A cluster jewel goes only in a cluster socket that takes its size;
  // any other jewel in any socket (T5b).
  const takes =
    tree['cluster-sockets']?.find((c) => c.node === node?.id)?.size ?? null
  const jewels = items.filter((i) => {
    const size = clusterSize(i.base ?? '')
    if (size !== null) return takes !== null && fits(size, takes)
    return JEWEL_CLASSES.includes(i['item-class'] ?? '')
  })
  const socketed = tree.jewels?.find((j) => j.node === node?.id)?.item ?? null
  const socket = (item: number | null) => {
    if (node === null) return
    void dispatch(
      run({
        'socket-jewel': {
          spec: tree.spec,
          node: node.id,
          ...(item === null ? {} : { item }),
        },
      }),
    ).then(onClose)
  }
  const elsewhere = (item: number) =>
    tree.jewels?.find((j) => j.item === item && j.node !== node?.id)
  return (
    <Dialog.Root open={node !== null} onOpenChange={(o) => !o && onClose()}>
      <Dialog.Portal>
        <Dialog.Content className="dialog-content picker">
          <Dialog.Title>{node?.name}</Dialog.Title>
          <Dialog.Description className="muted">
            {t('tree.jewelHelp')}
          </Dialog.Description>
          {jewels.length === 0 ? (
            <p className="muted">{t('tree.noJewels')}</p>
          ) : (
            <ul className="picker-list" aria-label={t('tree.jewels')}>
              {jewels.map((j) => (
                <li key={j.id}>
                  <button
                    type="button"
                    className="picker-entry"
                    aria-pressed={socketed === j.id}
                    onClick={() => socket(j.id)}
                  >
                    <span className="picker-name">{j.name}</span>
                    <span className="muted small-text">
                      {j.base}
                      {elsewhere(j.id) !== undefined &&
                        ` · ${t('tree.inAnotherSocket')}`}
                    </span>
                  </button>
                </li>
              ))}
            </ul>
          )}
          <div className="row">
            {socketed !== null && (
              <button type="button" onClick={() => socket(null)}>
                {t('tree.emptyTheSocket')}
              </button>
            )}
            {node !== null && (
              <button
                type="button"
                onClick={() =>
                  void dispatch(
                    run({
                      'deallocate-node': { spec: tree.spec, node: node.id },
                    }),
                  ).then(onClose)
                }
              >
                {t('tree.removeSocket')}
              </button>
            )}
            <span className="spacer" />
            <Dialog.Close asChild>
              <button type="button" className="primary">
                {t('picker.done')}
              </button>
            </Dialog.Close>
          </div>
        </Dialog.Content>
      </Dialog.Portal>
    </Dialog.Root>
  )
}

type Section =
  'allocated' | 'found' | 'keystone' | 'notable' | 'mastery' | 'ascendancy'

/** The non-visual tree: every node the canvas offers, as lists with the
 * same actions (ADR-008). */
function TreeList({
  layout,
  tree,
  search,
  onActivate,
}: {
  layout: TreeLayout
  tree: TreeView
  search: string
  onActivate: (node: TreeNodeView) => void
}) {
  const [section, setSection] = useState<Section>(
    search.trim() === '' ? 'allocated' : 'found',
  )
  const [seen, setSeen] = useState(search)
  if (seen !== search) {
    setSeen(search)
    if (search.trim() !== '') setSection('found')
  }
  const allocated = new Set(tree.nodes)
  const asc = layout.classes
    .find((c) => c.number === tree.class)
    ?.ascendancies.find((a) => a.number === tree.ascendancy)?.id
  const pick = (n: TreeNodeView): boolean => {
    switch (section) {
      case 'allocated':
        return allocated.has(n.id) || tree.kept.includes(n.id)
      case 'found':
        return matches(n, search)
      case 'keystone':
        return n.kind === 'keystone'
      case 'notable':
        return n.kind === 'notable' && n.ascendancy == null
      case 'mastery':
        return n.kind === 'mastery'
      case 'ascendancy':
        return n.ascendancy != null && n.ascendancy === asc
    }
  }
  const shown = layout.nodes.filter(
    (n) => n['cluster-area'] !== true && pick(n),
  )
  const sections: Section[] = [
    'allocated',
    'found',
    'keystone',
    'notable',
    'mastery',
    'ascendancy',
  ]
  return (
    <div className="tree-list">
      <div className="segmented" role="group" aria-label={t('tree.listLabel')}>
        {sections.map((s) => (
          <button
            key={s}
            type="button"
            aria-pressed={section === s}
            onClick={() => setSection(s)}
          >
            {tDynamic(`tree.section.${s}`, 'tree.section.allocated')}
          </button>
        ))}
      </div>
      {shown.length === 0 ? (
        <p className="muted">{t('tree.listEmpty')}</p>
      ) : (
        <ul className="tree-node-list" aria-label={t('tree.listLabel')}>
          {shown.slice(0, 300).map((n) => {
            const on = allocated.has(n.id)
            const start =
              n.kind === 'class-start' || n.kind === 'ascendancy-start'
            return (
              <li key={n.id} className={on ? 'on' : undefined}>
                <div className="spacer">
                  <span className="tree-card-name">{n.name}</span>{' '}
                  <span className="muted small-text">
                    {kindText(n)}
                    {on && ` · ${t('tree.allocated')}`}
                    {!n.modelled && ` · ${t('tree.kept')}`}
                  </span>
                  {n.lines.length > 0 && (
                    <p className="small-text">{n.lines.join(' · ')}</p>
                  )}
                  {n.flavour != null && <Flavour text={n.flavour} />}
                </div>
                {n.modelled && !start && tree.locked == null && (
                  <button type="button" onClick={() => onActivate(n)}>
                    {n.kind === 'mastery'
                      ? t('tree.chooseEffectFor', { node: n.name })
                      : on &&
                          (n.kind === 'jewel-socket' ||
                            n.kind === 'cluster-socket')
                        ? t('tree.chooseJewelFor', { node: n.name })
                        : on
                          ? t('tree.deallocate', { node: n.name })
                          : t('tree.allocate', { node: n.name })}
                  </button>
                )}
              </li>
            )
          })}
        </ul>
      )}
    </div>
  )
}

/** What differs from the stage compared with, in words: the counts,
 * what the rings on the tree mean, and the nodes by name. */
function StageNodeDiff({
  title,
  state,
  diff,
  layout,
}: {
  title: string
  state: 'none' | 'loading' | 'failed' | 'done'
  diff: StageNodes | null
  layout: TreeLayout
}) {
  if (state === 'loading') {
    return (
      <p className="muted small-text" role="status">
        {t('stageDiff.loading')}
      </p>
    )
  }
  if (diff === null) {
    return (
      <p className="small-text" role="status">
        {t('stageDiff.failed')}
      </p>
    )
  }
  if (diff.added.size === 0 && diff.removed.size === 0) {
    return (
      <p className="muted small-text" role="status">
        {t('stageDiff.same')}
      </p>
    )
  }
  const names = (ids: Set<number>) => {
    const byId = new Map(layout.nodes.map((n) => [n.id, n]))
    const rank = { keystone: 0, notable: 1 } as Record<string, number>
    return [...ids]
      .map((id) => byId.get(id))
      .filter((n) => n !== undefined)
      .sort((a, b) => (rank[a.kind] ?? 2) - (rank[b.kind] ?? 2))
      .map((n) => n.name)
  }
  const lists = [
    ['other', t('stageDiff.onlyOther', { other: title }), names(diff.added)],
    ['here', t('stageDiff.onlyHere'), names(diff.removed)],
  ] as const
  return (
    <details className="stage-diff">
      <summary>
        {t('stageDiff.treeSummary', {
          other: title,
          added: diff.added.size,
          removed: diff.removed.size,
        })}
      </summary>
      <div className="stage-diff-lists">
        {lists.map(([key, heading, list]) => (
          <div key={key}>
            <h3 className="eyebrow">
              <span
                className={`diff-ring diff-ring-${key}`}
                aria-hidden="true"
              />
              {heading}
            </h3>
            {list.length === 0 ? (
              <p className="muted small-text">—</p>
            ) : (
              <ul className="small-text">
                {list.map((name, i) => (
                  <li key={i}>{name}</li>
                ))}
              </ul>
            )}
          </div>
        ))}
      </div>
    </details>
  )
}

export function Tree() {
  const dispatch = useAppDispatch()
  const tree = useAppSelector((s) => s.session.snapshot?.build?.tree ?? null)
  // The data's layout with the spec's cluster jewel nodes (T5b).
  const loaded = useTreeLayout()
  const layout = useMemo(() => withClusterNodes(loaded, tree), [loaded, tree])
  const sprites = useTreeSprites()
  // The attribute totals the tree's allocations give (B-2 item evidence,
  // E11: the tree screenshots show them by the points).
  const attributes = useAttributes()
  const [search, setSearch] = useState('')
  const [mode, setMode] = useState<'canvas' | 'list' | null>(null)
  const [hover, setHover] = useState<{
    node: TreeNodeView
    x: number
    y: number
  } | null>(null)
  const [previewed, setPreviewed] = useState<{
    node: number
    version: number
    value: TreePreview | 'refused'
  } | null>(null)
  const [mastery, setMastery] = useState<TreeNodeView | null>(null)
  const [socket, setSocket] = useState<TreeNodeView | null>(null)
  const [confirm, setConfirm] = useState<
    | { kind: 'class'; class: number }
    | { kind: 'reset' }
    | { kind: 'delete' }
    | { kind: 'convert'; version: string; dropped: number }
    | null
  >(null)
  const [specForm, setSpecForm] = useState<{
    kind: 'new' | 'copy' | 'rename'
    title: string
  } | null>(null)
  const specs = useAppSelector((s) => s.session.snapshot?.build?.specs ?? [])
  // The stage this one is compared with (UX audit follow-up).
  const build = useAppSelector((s) => s.session.snapshot?.build)
  const stage = useCompareStage(build)
  const other = useStageView(stage?.spec ?? null)
  const diff = useMemo(() => {
    if (other.state !== 'done' || tree === null) return null
    const d = nodeDiff(tree.nodes, other.view.nodes)
    return { added: new Set(d.added), removed: new Set(d.removed) }
  }, [other, tree])
  const specNameId = useId()
  const searchId = useId()
  const classId = useId()
  const ascId = useId()

  const ready = layout !== null && layout !== 'none'
  const found = useMemo(
    () =>
      ready
        ? new Set(
            layout.nodes.filter((n) => matches(n, search)).map((n) => n.id),
          )
        : new Set<number>(),
    [layout, ready, search],
  )

  // The hovered node's preview, asked once per node and build version.
  const version = useAppSelector((s) => s.session.snapshot?.version ?? 0)
  const hovered = hover?.node.id ?? null
  // A locked spec is shown, not edited: nothing to preview.
  const spec = tree == null || tree.locked != null ? null : tree.spec
  useEffect(() => {
    if (hovered === null || spec === null) return
    let live = true
    const timer = setTimeout(() => {
      void dispatch(query((api) => api.previewTree(spec, hovered))).then(
        (p) => {
          if (live)
            setPreviewed({ node: hovered, version, value: p ?? 'refused' })
        },
      )
    }, 120)
    return () => {
      live = false
      clearTimeout(timer)
    }
  }, [dispatch, hovered, spec, version])
  const preview =
    previewed !== null &&
    previewed.node === hovered &&
    previewed.version === version
      ? previewed.value
      : null

  if (layout === null) {
    return (
      <section className="screen">
        <ScreenHeading>{t('tree.title')}</ScreenHeading>
        <p className="muted">{t('about.loading')}</p>
      </section>
    )
  }
  if (layout === 'none' || tree === null) {
    return (
      <section className="screen">
        <ScreenHeading>{t('tree.title')}</ScreenHeading>
        <p className="muted">{t('tree.noData')}</p>
      </section>
    )
  }

  const view = mode ?? (layout.placed ? 'canvas' : 'list')
  const cls = layout.classes.find((c) => c.number === tree.class)
  const quote =
    cls?.ascendancies.find((a) => a.number === tree.ascendancy)?.flavour ?? null
  const locked = tree.locked ?? null
  const activate = (n: TreeNodeView) => {
    if (
      locked !== null ||
      !n.modelled ||
      n.kind === 'class-start' ||
      n.kind === 'ascendancy-start'
    )
      return
    if (n.kind === 'mastery') {
      setMastery(n)
      return
    }
    if (
      (n.kind === 'jewel-socket' || n.kind === 'cluster-socket') &&
      tree.nodes.includes(n.id)
    ) {
      setSocket(n)
      return
    }
    const command = tree.nodes.includes(n.id)
      ? { 'deallocate-node': { spec: tree.spec, node: n.id } }
      : { 'allocate-node': { spec: tree.spec, node: n.id } }
    void dispatch(run(command))
  }
  const starts = new Set(
    layout.nodes
      .filter((n) => n.kind === 'class-start' || n.kind === 'ascendancy-start')
      .map((n) => n.id),
  )
  const lossy = tree.nodes.some((n) => !starts.has(n))
  const specTitle =
    specs.find((s) => s.id === tree.spec)?.title ?? t('tree.specUntitled')
  const saveSpec = () => {
    if (specForm === null || specForm.title.trim() === '') return
    const command =
      specForm.kind === 'rename'
        ? { 'rename-spec': { spec: tree.spec, title: specForm.title } }
        : {
            'new-spec': {
              title: specForm.title,
              copy: specForm.kind === 'copy' ? tree.spec : null,
            },
          }
    setSpecForm(null)
    void dispatch(run(command))
  }

  return (
    <section className="screen tree-screen">
      {/* The heading row: the page, how to use it (one press away), and
          the tree's own actions; then one row of controls. */}
      <div className="page-head">
        <ScreenHeading>{t('tree.title')}</ScreenHeading>
        <Hint>{t('tree.canvasHelp')}</Hint>
        <span className="spacer" />
        <div
          className="row tree-specs"
          role="group"
          aria-label={t('tree.specs')}
        >
          <span className="muted small-text">{specTitle}</span>
          <button
            type="button"
            className="small"
            onClick={() => setSpecForm({ kind: 'new', title: '' })}
          >
            {t('tree.specNew')}
          </button>
          <button
            type="button"
            className="small"
            onClick={() =>
              setSpecForm({ kind: 'copy', title: `${specTitle} (copy)` })
            }
          >
            {t('tree.specCopy')}
          </button>
          <button
            type="button"
            className="small"
            onClick={() => setSpecForm({ kind: 'rename', title: specTitle })}
          >
            {t('tree.specRename')}
          </button>
          <button
            type="button"
            className="small"
            disabled={specs.length <= 1}
            onClick={() => setConfirm({ kind: 'delete' })}
          >
            {t('tree.specDelete')}
          </button>
        </div>
      </div>
      <div className="row tree-bar">
        <label htmlFor={classId}>{t('tree.class')}</label>
        <select
          id={classId}
          value={tree.class}
          disabled={locked !== null}
          onChange={(e) => {
            const number = Number(e.target.value)
            if (lossy) setConfirm({ kind: 'class', class: number })
            else
              void dispatch(
                run({ 'select-class': { spec: tree.spec, class: number } }),
              )
          }}
        >
          {cls === undefined && <option value={tree.class}>—</option>}
          {layout.classes.map((c) => (
            <option key={c.number} value={c.number}>
              {c.name}
            </option>
          ))}
        </select>
        <label htmlFor={ascId}>{t('tree.ascendancy')}</label>
        <select
          id={ascId}
          value={tree.ascendancy}
          disabled={locked !== null}
          onChange={(e) =>
            void dispatch(
              run({
                'select-ascendancy': {
                  spec: tree.spec,
                  ascendancy: Number(e.target.value),
                },
              }),
            )
          }
        >
          <option value={0}>{t('tree.noAscendancy')}</option>
          {(cls?.ascendancies ?? []).map((a) => (
            <option key={a.number} value={a.number}>
              {a.name}
            </option>
          ))}
        </select>
        {quote != null && <Flavour text={quote} />}
        <p className="tree-points" aria-live="polite">
          {t('tree.points', {
            n: tree.points,
            a: tree['ascendancy-points'],
          })}
        </p>
        {attributes.str != null &&
          attributes.dex != null &&
          attributes.int != null && (
            <p className="tree-points" aria-live="polite">
              {t('tree.attributes', {
                str: formatNumber(attributes.str),
                dex: formatNumber(attributes.dex),
                int: formatNumber(attributes.int),
              })}
            </p>
          )}
        <span className="spacer" />
        {build != null && <StageCompareSelect build={build} />}
        <label htmlFor={searchId} className="visually-hidden">
          {t('tree.search')}
        </label>
        <input
          id={searchId}
          type="search"
          placeholder={t('tree.search')}
          value={search}
          onChange={(e) => setSearch(e.target.value)}
        />
        {search.trim() !== '' && (
          <span className="muted small-text" role="status">
            {t('tree.found', { n: found.size })}
          </span>
        )}
        <div className="segmented" role="group" aria-label={t('tree.view')}>
          <button
            type="button"
            aria-pressed={view === 'canvas'}
            disabled={!layout.placed}
            onClick={() => setMode('canvas')}
          >
            {t('tree.viewCanvas')}
          </button>
          <button
            type="button"
            aria-pressed={view === 'list'}
            onClick={() => setMode('list')}
          >
            {t('tree.viewList')}
          </button>
        </div>
        <button
          type="button"
          disabled={locked !== null}
          onClick={() => setConfirm({ kind: 'reset' })}
        >
          {t('tree.reset')}
        </button>
      </div>
      {specForm !== null && (
        <form
          className="panel row"
          aria-label={
            specForm.kind === 'rename'
              ? t('tree.specRename')
              : specForm.kind === 'copy'
                ? t('tree.specCopy')
                : t('tree.specNew')
          }
          onSubmit={(e) => {
            e.preventDefault()
            saveSpec()
          }}
        >
          <label htmlFor={specNameId}>{t('tree.specName')}</label>
          <input
            id={specNameId}
            value={specForm.title}
            onChange={(e) =>
              setSpecForm({ ...specForm, title: e.target.value })
            }
          />
          <button
            type="submit"
            className="primary"
            disabled={specForm.title.trim() === ''}
          >
            {t('tree.specSave')}
          </button>
          <button type="button" onClick={() => setSpecForm(null)}>
            {t('tree.cancel')}
          </button>
          {specForm.kind === 'new' && (
            <p className="muted small-text">{t('tree.specNewHelp')}</p>
          )}
        </form>
      )}
      {confirm !== null && (
        <div
          className="panel"
          role="alertdialog"
          aria-labelledby={`${classId}-c`}
        >
          <p id={`${classId}-c`}>
            {confirm.kind === 'reset'
              ? t('tree.resetAsk')
              : confirm.kind === 'delete'
                ? t('tree.specDeleteAsk', { name: specTitle })
                : confirm.kind === 'convert'
                  ? t('tree.convertAsk', {
                      version: confirm.version,
                      n: confirm.dropped,
                    })
                  : t('tree.classAsk')}
          </p>
          <div className="row">
            <button
              type="button"
              className="primary"
              onClick={() => {
                const command =
                  confirm.kind === 'reset'
                    ? { 'reset-tree': { spec: tree.spec } }
                    : confirm.kind === 'delete'
                      ? { 'delete-spec': { spec: tree.spec } }
                      : confirm.kind === 'convert'
                        ? { 'convert-tree-version': { spec: tree.spec } }
                        : {
                            'select-class': {
                              spec: tree.spec,
                              class: confirm.class,
                            },
                          }
                setConfirm(null)
                void dispatch(run(command))
              }}
            >
              {confirm.kind === 'reset'
                ? t('tree.reset')
                : confirm.kind === 'delete'
                  ? t('tree.specDelete')
                  : confirm.kind === 'convert'
                    ? t('tree.convert', { version: confirm.version })
                    : t('tree.changeClass')}
            </button>
            <button type="button" onClick={() => setConfirm(null)}>
              {t('tree.cancel')}
            </button>
          </div>
        </div>
      )}
      {locked !== null && (
        <div className="panel row" role="status">
          <p>{t('tree.locked', { why: locked })}</p>
          {tree.convert != null && (
            <button
              type="button"
              onClick={() =>
                tree.convert != null &&
                setConfirm({
                  kind: 'convert',
                  version: tree.convert.version,
                  dropped: tree.convert.dropped.length,
                })
              }
            >
              {t('tree.convert', { version: tree.convert.version })}
            </button>
          )}
        </div>
      )}
      {tree.kept.length > 0 && (
        <p className="muted small-text">
          {t('tree.keptCount', { n: tree.kept.length })}
        </p>
      )}
      {stage !== null && (
        <StageNodeDiff
          title={stage.title}
          state={other.state}
          diff={diff}
          layout={layout}
        />
      )}
      {!layout.placed && <p className="muted">{t('tree.notPlaced')}</p>}
      {view === 'canvas' && layout.placed ? (
        <div className="tree-stage">
          <TreeCanvas
            layout={layout}
            sprites={sprites}
            tree={tree}
            found={found}
            preview={preview === 'refused' ? null : preview}
            diff={diff}
            onHover={(node, at) =>
              setHover(node === null ? null : { node, ...at })
            }
            onActivate={activate}
          />
          {hover !== null && (
            <HoverCard x={hover.x} y={hover.y} sized={preview}>
              <NodeCard
                node={hover.node}
                tree={tree}
                layout={layout}
                preview={preview}
              />
            </HoverCard>
          )}
        </div>
      ) : (
        <TreeList
          layout={layout}
          tree={tree}
          search={search}
          onActivate={activate}
        />
      )}
      <MasteryPicker
        node={mastery}
        tree={tree}
        layout={layout}
        onClose={() => setMastery(null)}
      />
      <JewelPicker node={socket} tree={tree} onClose={() => setSocket(null)} />
    </section>
  )
}
