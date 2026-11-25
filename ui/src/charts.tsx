// The charts the dashboards draw (UX audit 2026-11-15, section D): our own
// small SVG and CSS marks, no chart library (the app is offline and its
// dependencies are few). One measure per chart, one axis, a hover and a
// keyboard readout on every plot, values in text beside every mark, and a
// table of the same numbers one click away (the panel's detail view), so
// colour and shape are never the only way to read one.
import {
  type KeyboardEvent,
  type ReactNode,
  useEffect,
  useId,
  useRef,
  useState,
} from 'react'

import { compactNumber, niceTicks } from './chartMath'
import { formatNumber, t } from './i18n'

/** An element's width, measured now and when it changes; `fallback` where
 * layout is not available. */
function useWidth(fallback: number) {
  const ref = useRef<HTMLDivElement>(null)
  const [width, setWidth] = useState(fallback)
  useEffect(() => {
    const el = ref.current
    if (el === null) return
    const measure = () => {
      const w = el.clientWidth
      if (w > 0) setWidth(w)
    }
    measure()
    if (typeof ResizeObserver === 'undefined') return
    const observer = new ResizeObserver(measure)
    observer.observe(el)
    return () => observer.disconnect()
  }, [])
  return [ref, width] as const
}

/** A trend in a tile's corner: the values in order, the last one marked.
 * Decorative: the tile says the value and the change in text. */
export function Sparkline({
  values,
  width = 96,
  height = 28,
}: {
  values: number[]
  width?: number
  height?: number
}) {
  if (values.length < 2) return null
  const min = Math.min(...values)
  const max = Math.max(...values)
  const span = max - min || 1
  const pad = 3
  const x = (i: number) =>
    pad + (i * (width - 2 * pad)) / Math.max(1, values.length - 1)
  const y = (v: number) =>
    height - pad - ((v - min) / span) * (height - 2 * pad)
  const path = values
    .map(
      (v, i) => `${i === 0 ? 'M' : 'L'}${x(i).toFixed(1)},${y(v).toFixed(1)}`,
    )
    .join(' ')
  const last = values.length - 1
  return (
    <svg
      className="sparkline"
      width={width}
      height={height}
      viewBox={`0 0 ${width} ${height}`}
      aria-hidden="true"
      focusable="false"
    >
      <path d={path} className="chart-line" />
      <circle
        cx={x(last)}
        cy={y(values[last]!)}
        r={2.5}
        className="chart-dot"
      />
    </svg>
  )
}

export interface ChartPoint {
  /** The point's place on the horizontal axis, as text ("Endgame", "20"). */
  label: string
  /** Its value; `null` when there is none (a gap in the line). */
  y: number | null
  /** More to say in the readout ("after edit of Shiny Wand"). */
  note?: string
}

/** One measure along an ordered axis: a line with a dot per point, a few
 * round gridlines, the build's current point marked, an optional target
 * line, and a readout that follows the pointer or the arrow keys. */
export function LineChart({
  points,
  label,
  format = formatNumber,
  current,
  target,
  height = 180,
  step = false,
  zero = false,
}: {
  points: ChartPoint[]
  /** What the chart shows, for assistive technology. */
  label: string
  format?: (value: number) => string
  /** The index of the point that is the build as it is now. */
  current?: number
  /** A level to reach, drawn across the plot. */
  target?: { y: number; label: string }
  height?: number
  /** Hold each value until the next point (values that change at once). */
  step?: boolean
  /** Start the axis at zero (amounts that grow from nothing). */
  zero?: boolean
}) {
  const [ref, width] = useWidth(480)
  const [active, setActive] = useState<number | null>(null)
  const id = useId()
  const known = points
    .map((p, i) => ({ i, y: p.y }))
    .filter((p): p is { i: number; y: number } => p.y !== null)
  if (known.length === 0) {
    return <p className="muted small-text">{t('chart.noData')}</p>
  }
  const values = known.map((p) => p.y)
  if (target !== undefined) values.push(target.y)
  if (zero) values.push(0)
  const ticks = niceTicks(Math.min(...values), Math.max(...values), 3)
  const low = ticks[0]!
  const high = ticks[ticks.length - 1]!
  // A gutter on the left holds the axis values, clear of the plot.
  const left = 44
  const right = 10
  const top = 10
  const bottom = 22
  const plotW = Math.max(40, width - left - right)
  const plotH = height - top - bottom
  const x = (i: number) =>
    left + (points.length === 1 ? plotW / 2 : (i * plotW) / (points.length - 1))
  const y = (v: number) => top + plotH - ((v - low) / (high - low || 1)) * plotH
  let path = ''
  let pen = false
  let before: { i: number; y: number } | null = null
  for (const p of points.map((q, i) => ({ i, y: q.y }))) {
    if (p.y === null) {
      pen = false
      continue
    }
    if (!pen) {
      path += `M${x(p.i).toFixed(1)},${y(p.y).toFixed(1)} `
    } else if (step && before !== null) {
      path += `H${x(p.i).toFixed(1)} V${y(p.y).toFixed(1)} `
    } else {
      path += `L${x(p.i).toFixed(1)},${y(p.y).toFixed(1)} `
    }
    pen = true
    before = { i: p.i, y: p.y }
  }
  // Labels on the horizontal axis: the ends, the current point, and as
  // many between as fit without touching.
  // Each label's room is measured from its own text (the figure face is
  // fixed-width: 0.6 em of the 12 px tick), so long names such as a
  // stage's do not run into each other (RV-08). The 7.2 follows
  // `.chart-tick` (`--fs-meta` in `--font-figure`): change them together.
  const last = points.length - 1
  const room = (i: number): [number, number] => {
    const w = points[i]!.label.length * 7.2
    const at = x(i)
    return i === 0
      ? [at, at + w]
      : i === last
        ? [at - w, at]
        : [at - w / 2, at + w / 2]
  }
  const kept: number[] = []
  const order = [0, last, ...(current === undefined ? [] : [current])]
  for (let i = 1; i < last; i++) order.push(i)
  for (const i of order) {
    if (i < 0 || i > last || kept.includes(i)) continue
    const [from, to] = room(i)
    const clear = kept.every((k) => {
      const [f, t2] = room(k)
      return to + 12 <= f || t2 + 12 <= from
    })
    if (clear) kept.push(i)
  }
  const labelled = (i: number) => kept.includes(i)
  const nearest = (clientX: number, box: DOMRect) => {
    const at = clientX - box.left
    let best = known[0]!.i
    for (const p of known) {
      if (Math.abs(x(p.i) - at) < Math.abs(x(best) - at)) best = p.i
    }
    return best
  }
  const onKey = (e: KeyboardEvent<HTMLDivElement>) => {
    const order = known.map((p) => p.i)
    const at = active === null ? -1 : order.indexOf(active)
    let next: number | undefined
    if (e.key === 'ArrowRight') next = order[Math.min(order.length - 1, at + 1)]
    else if (e.key === 'ArrowLeft') next = order[Math.max(0, at - 1)]
    else if (e.key === 'Home') next = order[0]
    else if (e.key === 'End') next = order[order.length - 1]
    else if (e.key === 'Escape') {
      setActive(null)
      return
    } else return
    e.preventDefault()
    if (next !== undefined) setActive(next)
  }
  const shown = active !== null ? points[active] : undefined
  // The readout stays inside the plot: left of the point past the middle.
  const flip = active !== null && x(active) > left + plotW / 2
  return (
    <div
      ref={ref}
      className="chart"
      role="group"
      aria-label={label}
      aria-describedby={`${id}-help`}
      tabIndex={0}
      onKeyDown={onKey}
      onBlur={() => setActive(null)}
      onPointerMove={(e) =>
        setActive(nearest(e.clientX, e.currentTarget.getBoundingClientRect()))
      }
      onPointerLeave={() => setActive(null)}
    >
      <span id={`${id}-help`} className="visually-hidden">
        {t('chart.keys')}
      </span>
      <svg width={width} height={height} aria-hidden="true" focusable="false">
        {ticks.map((v) => (
          <g key={v}>
            <line
              x1={left}
              x2={left + plotW}
              y1={y(v)}
              y2={y(v)}
              className="chart-grid"
            />
            <text
              x={left - 8}
              y={y(v) + 4}
              textAnchor="end"
              className="chart-tick"
            >
              {compactNumber(v)}
            </text>
          </g>
        ))}
        {target !== undefined && (
          <g>
            <line
              x1={left}
              x2={left + plotW}
              y1={y(target.y)}
              y2={y(target.y)}
              className="chart-target"
            />
            <text
              x={left + 4}
              y={y(target.y) - 5}
              className="chart-tick chart-target-label"
            >
              {target.label}
            </text>
          </g>
        )}
        <path d={path} className="chart-line" />
        {points.length <= 24 &&
          known.map((p) => (
            <circle
              key={p.i}
              cx={x(p.i)}
              cy={y(p.y)}
              r={p.i === current ? 5 : 3}
              className={p.i === current ? 'chart-dot current' : 'chart-dot'}
            />
          ))}
        {points.length > 24 &&
          current !== undefined &&
          points[current]?.y != null && (
            <circle
              cx={x(current)}
              cy={y(points[current].y)}
              r={5}
              className="chart-dot current"
            />
          )}
        {points.map(
          (p, i) =>
            labelled(i) && (
              <text
                key={i}
                x={x(i)}
                y={height - 6}
                textAnchor={
                  i === 0 ? 'start' : i === points.length - 1 ? 'end' : 'middle'
                }
                className={i === current ? 'chart-tick current' : 'chart-tick'}
              >
                {p.label}
              </text>
            ),
        )}
        {active !== null && shown?.y != null && (
          <g>
            <line
              x1={x(active)}
              x2={x(active)}
              y1={top}
              y2={top + plotH}
              className="chart-cross"
            />
            <circle
              cx={x(active)}
              cy={y(shown.y)}
              r={5}
              className="chart-dot active"
            />
          </g>
        )}
      </svg>
      {/* The readout, said aloud as it moves. */}
      <div
        className="chart-readout"
        role="status"
        style={
          active === null
            ? { visibility: 'hidden' }
            : flip
              ? { right: width - x(active) + 10, top: 4 }
              : { left: x(active) + 10, top: 4 }
        }
      >
        {shown !== undefined && (
          <>
            <strong>{shown.y === null ? '—' : format(shown.y)}</strong>
            <span>
              {shown.label}
              {active === current && ` · ${t('chart.now')}`}
            </span>
            {shown.note !== undefined && <span>{shown.note}</span>}
          </>
        )}
      </div>
    </div>
  )
}

/** A value as a bar on a fixed scale, with its number beside it: `low` to
 * `high` (zero inside when values can be negative), an optional cap mark,
 * and an optional lighter part behind (a total the value is part of). */
export function Meter({
  value,
  low = 0,
  high,
  cap,
  behind,
  tone = 'series',
}: {
  value: number
  low?: number
  high: number
  /** A level marked across the bar (a resistance cap). */
  cap?: number
  /** A larger amount drawn behind the value (the pool before reservation). */
  behind?: number
  tone?: 'series' | 'life' | 'mana' | 'shield' | 'bad' | 'good'
}) {
  const span = high - low || 1
  const at = (v: number) =>
    `${Math.min(100, Math.max(0, ((v - low) / span) * 100))}%`
  const zero = Math.min(100, Math.max(0, ((0 - low) / span) * 100))
  const end = Math.min(100, Math.max(0, ((value - low) / span) * 100))
  return (
    <span className="meter" aria-hidden="true">
      {behind !== undefined && (
        <span
          className={`meter-fill meter-${tone} behind`}
          style={{ width: at(behind) }}
        />
      )}
      <span
        className={`meter-fill meter-${value < 0 ? 'bad' : tone}`}
        style={{
          left: `${Math.min(zero, end)}%`,
          width: `${Math.abs(end - zero)}%`,
        }}
      />
      {low < 0 && <span className="meter-zero" style={{ left: `${zero}%` }} />}
      {cap !== undefined && (
        <span className="meter-cap" style={{ left: at(cap) }} />
      )}
    </span>
  )
}

/** A change as a bar from the middle: right and marked ▲ for more, left
 * and ▼ for less, sized by `share` (-1 to 1). */
export function DeltaBar({ share }: { share: number }) {
  const size = `${Math.min(50, Math.abs(share) * 50)}%`
  return (
    <span className="delta-bar" aria-hidden="true">
      <span className="delta-mid" />
      {share !== 0 && (
        <span
          className={share > 0 ? 'delta-fill up' : 'delta-fill down'}
          style={
            share > 0
              ? { left: '50%', width: size }
              : { right: '50%', width: size }
          }
        />
      )}
    </span>
  )
}

/** A change in words and a glyph, never colour alone: ▲ 1,234 (+3.2%). */
export function Delta({
  now,
  before,
  format = formatNumber,
}: {
  now: number | null | undefined
  before: number | null | undefined
  format?: (value: number) => string
}): ReactNode {
  if (now == null || before == null) return null
  const change = now - before
  if (Math.abs(change) < 1e-9) {
    return <span className="delta same">{t('delta.same')}</span>
  }
  const percent =
    before === 0 ? null : Math.round((change / Math.abs(before)) * 1000) / 10
  return (
    <span className={change > 0 ? 'delta up' : 'delta down'}>
      <span aria-hidden="true">{change > 0 ? '▲' : '▼'}</span>{' '}
      <span className="visually-hidden">
        {change > 0 ? t('delta.up') : t('delta.down')}
      </span>
      {format(Math.abs(change))}
      {percent !== null &&
        ` (${change > 0 ? '+' : '−'}${formatNumber(Math.abs(percent))}%)`}
    </span>
  )
}
