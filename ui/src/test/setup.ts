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

afterEach(() => {
  cleanup()
})
