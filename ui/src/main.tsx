import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import { Provider } from 'react-redux'

// The bundled fonts (SIL OFL 1.1, licenses in public/licenses/): weight
// axis only, served from the app, never a CDN (the app is offline).
import '@fontsource-variable/inter/wght.css'
import '@fontsource-variable/space-grotesk/wght.css'
import '@fontsource-variable/jetbrains-mono/wght.css'
import './index.css'
import './dash.css'
import './redesign.css'
import { App } from './App.tsx'
import { FakeApi } from './api/fake'
import { inTauri, tauriApi } from './api/tauri'
import {
  type Screen,
  actions,
  makeStore,
  markOf,
  refresh,
  startSync,
} from './store'

const root = document.getElementById('root')
if (root === null) {
  throw new Error('index.html is missing the #root element')
}

// Outside the shell (a plain browser during UI work) the in-memory API
// stands in, with sample data.
const api = inTauri() ? tauriApi : new FakeApi({ demo: true, ...demoLists() })

/** Sample recent builds and unsaved work, so the start page can be looked
 * at in a browser with its lists filled. */
function demoLists() {
  const now = Math.floor(Date.now() / 1000)
  const recent = (
    [
      ['Smite Inquisitor', 'Templar', 'Inquisitor', 92, 'Smite', 2 * 3600],
      ['Ice Nova league start', 'Witch', 'Occultist', 68, 'Ice Nova', 86400],
      ['Essence Drain', 'Witch', 'Trickster', 94, 'Essence Drain', 3 * 86400],
      ['Bow test', 'Ranger', null, 12, null, 9 * 86400],
      ['Untitled idea', null, null, null, null, 40 * 86400],
    ] as const
  ).map(([name, cls, ascendancy, level, skill, age], i) => ({
    id: `r${i + 1}`,
    name,
    path: `C:\\Builds\\${name.toLowerCase().replaceAll(' ', '-')}.loom.json`,
    at: now - age,
    class: cls,
    ascendancy,
    level,
    'main-skill': skill,
  }))
  return {
    recent,
    recovery: [
      {
        id: 'u1',
        name: 'Smite Inquisitor',
        path: recent[0]!.path,
        'saved-at': now - 1500,
      },
    ],
  }
}
// In `pnpm dev` the fake is reachable from the console, to stage what an
// AI client would do (`loomFake.addRequest(...)`).
if (import.meta.env.DEV && api instanceof FakeApi) {
  Object.assign(window, { loomFake: api })
}
const store = makeStore(api)
void startSync(store, api).then(async () => {
  // In `pnpm dev` over the fake, `?screen=skills` opens the sample build
  // on that screen and `&dash=trends` on that dashboard, so a screen can
  // be looked at (or captured by a headless browser) without clicking
  // through; `&pin` pins the numbers and `&stage=2` then switches stage,
  // to show a comparison.
  if (!import.meta.env.DEV || !(api instanceof FakeApi)) return
  const params = new URLSearchParams(window.location.search)
  const screen = params.get('screen')
  if (screen === null) return
  await api.importFile()
  await store.dispatch(refresh())
  await new Promise((done) => setTimeout(done, 50))
  await store.dispatch(refresh())
  const { ui, session } = store.getState()
  const dash = params.get('dash')
  if (dash !== null)
    store.dispatch(actions.prefs({ ...ui.prefs, active: dash }))
  if (params.has('pin') && session.calc !== null) {
    store.dispatch(actions.baseline(markOf(session.calc, 'pinned numbers')))
  }
  const stage = params.get('stage')
  if (stage !== null) {
    await api.execute({ 'use-loadout': { spec: Number(stage) } })
    await new Promise((done) => setTimeout(done, 50))
    await store.dispatch(refresh())
  }
  // `&vs=2` compares the tree and the equipment with that stage; `&table`
  // shows the Skills page as a table.
  const vs = params.get('vs')
  if (vs !== null) store.dispatch(actions.compareStage(Number(vs)))
  if (params.has('table')) {
    store.dispatch(
      actions.prefs({ ...store.getState().ui.prefs, skillsTable: true }),
    )
  }
  // `screen=item&item=1` opens that item; `screen=breakdown&field=life`
  // that stat's breakdown.
  if (screen === 'item') {
    store.dispatch(actions.editItem(Number(params.get('item') ?? 1)))
  } else if (screen === 'breakdown') {
    store.dispatch(actions.breakdown(params.get('field') ?? 'life'))
  } else {
    store.dispatch(actions.go(screen as Screen))
  }
})

createRoot(root).render(
  <StrictMode>
    <Provider store={store}>
      <App />
    </Provider>
  </StrictMode>,
)
