// The whole-app audit pass (RV-08, docs/ui/ux-audit-pass-2.md): a line
// chart's labels on the horizontal axis never run into each other, however
// long the names (a build's stages).
import { render } from '@testing-library/react'
import { describe, expect, it } from 'vitest'

import { LineChart } from '../charts'

/** The room each drawn label takes, from its place, anchor, and text (the
 * figure face is fixed-width: 7.2 px a character at the tick's size). */
function rooms(container: HTMLElement): [number, number][] {
  return [...container.querySelectorAll('text.chart-tick')]
    .filter((e) => e.getAttribute('y') !== null && /[a-z]/i.test(e.textContent))
    .map((e): [number, number] => {
      const at = Number(e.getAttribute('x'))
      const w = e.textContent.length * 7.2
      const anchor = e.getAttribute('text-anchor')
      return anchor === 'start'
        ? [at, at + w]
        : anchor === 'end'
          ? [at - w, at]
          : [at - w / 2, at + w / 2]
    })
    .sort((a, b) => a[0] - b[0])
}

describe('a line chart’s axis labels', () => {
  it('should not overlap when the names are long', () => {
    const points = Array.from({ length: 18 }, (_, i) => ({
      label: `Min-Max +2 Spectre stage ${i + 1}`,
      y: 1000 + i * 100,
    }))
    const { container } = render(
      <LineChart points={points} label="Life by stage" current={9} />,
    )
    const drawn = rooms(container)
    expect(drawn.length).toBeGreaterThan(0)
    for (let i = 1; i < drawn.length; i++) {
      expect(drawn[i]![0]).toBeGreaterThanOrEqual(drawn[i - 1]![1])
    }
    // The first point is always named.
    expect(container.textContent).toContain('Min-Max +2 Spectre stage 1')
  })

  it('should still name every point when the names are short', () => {
    const points = ['1', '2', '3', '4', '5'].map((label, i) => ({
      label: `L${label}`,
      y: i,
    }))
    const { container } = render(<LineChart points={points} label="By level" />)
    expect(rooms(container)).toHaveLength(5)
  })
})
