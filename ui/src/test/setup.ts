// jsdom lacks a few browser APIs the headless primitives call.
import { afterEach } from 'vitest'
import { cleanup } from '@testing-library/react'

class ResizeObserverStub {
  observe() {}
  unobserve() {}
  disconnect() {}
}

const g = globalThis as unknown as Record<string, unknown>
g.ResizeObserver ??= ResizeObserverStub

const proto = Element.prototype as unknown as Record<string, unknown>
proto.scrollIntoView ??= () => {}
proto.hasPointerCapture ??= () => false
proto.releasePointerCapture ??= () => {}
proto.setPointerCapture ??= () => {}

// jsdom has no 2D canvas: drawing is skipped (the tree's list view is
// what tests use, as screen reader users do).
HTMLCanvasElement.prototype.getContext = () => null

afterEach(() => {
  cleanup()
})
