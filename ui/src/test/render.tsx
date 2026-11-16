// Renders the app over a FakeApi, as main.tsx does over the shell's API.
import { render } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import axe from 'axe-core'
import { Provider } from 'react-redux'
import { expect } from 'vitest'

import { App } from '../App'
import { FakeApi, type FakeOptions } from '../api/fake'
import { makeStore, startSync } from '../store'

export async function renderApp(options: FakeOptions = {}) {
  const api = new FakeApi(options)
  const store = makeStore(api)
  await startSync(store, api)
  const user = userEvent.setup()
  const view = render(
    <Provider store={store}>
      <App />
    </Provider>,
  )
  return { api, store, user, ...view }
}

/** No axe violations (WCAG 2.x A and AA rules axe checks in a DOM).
 * Colour contrast needs layout jsdom lacks; theme.test.ts checks it from
 * the tokens. */
export async function expectAccessible(node: Element = document.body) {
  const results = await axe.run(node, {
    runOnly: {
      type: 'tag',
      values: ['wcag2a', 'wcag2aa', 'wcag21a', 'wcag21aa', 'wcag22aa'],
    },
    rules: { 'color-contrast': { enabled: false } },
  })
  const summary = results.violations.map(
    (v) =>
      `${v.id}: ${v.help} (${v.nodes.map((n) => n.target.join(' ')).join('; ')})`,
  )
  expect(summary).toEqual([])
}
