// Numbers for the charts' axes (kept apart from the components).
import { formatNumber } from './i18n'

const compact = new Intl.NumberFormat(undefined, {
  notation: 'compact',
  maximumFractionDigits: 1,
})

/** A number short enough for an axis (12.3K, 1.2M). */
export function compactNumber(value: number): string {
  return Math.abs(value) >= 10000 ? compact.format(value) : formatNumber(value)
}

/** About `count` round values spanning `min` to `max` (1, 2, or 5 times a
 * power of ten apart), for an axis. */
export function niceTicks(min: number, max: number, count = 4): number[] {
  if (!Number.isFinite(min) || !Number.isFinite(max)) return []
  if (min === max) {
    const pad = Math.abs(min) > 0 ? Math.abs(min) * 0.1 : 1
    min -= pad
    max += pad
  }
  const raw = (max - min) / Math.max(1, count)
  const power = 10 ** Math.floor(Math.log10(raw))
  const unit = raw / power
  const step = (unit >= 5 ? 10 : unit >= 2 ? 5 : unit >= 1 ? 2 : 1) * power
  const first = Math.floor(min / step) * step
  const ticks: number[] = []
  for (let v = first; v <= max + step * 0.999; v += step) {
    ticks.push(Math.round(v / step) * step)
    if (ticks.length > 12) break
  }
  return ticks
}
