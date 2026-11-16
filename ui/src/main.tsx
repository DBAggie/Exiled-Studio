import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import { Provider } from 'react-redux'

// The bundled fonts (SIL OFL 1.1, licenses in public/licenses/): weight
// axis only, served from the app, never a CDN (the app is offline).
import '@fontsource-variable/inter/wght.css'
import '@fontsource-variable/space-grotesk/wght.css'
import '@fontsource-variable/jetbrains-mono/wght.css'
import './index.css'
import { App } from './App.tsx'
import { FakeApi } from './api/fake'
import { inTauri, tauriApi } from './api/tauri'
import { makeStore, startSync } from './store'

const root = document.getElementById('root')
if (root === null) {
  throw new Error('index.html is missing the #root element')
}

// Outside the shell (a plain browser during UI work) the in-memory API
// stands in, with sample data.
const api = inTauri() ? tauriApi : new FakeApi()
// In `pnpm dev` the fake is reachable from the console, to stage what an
// AI client would do (`loomFake.addRequest(...)`).
if (import.meta.env.DEV && api instanceof FakeApi) {
  Object.assign(window, { loomFake: api })
}
const store = makeStore(api)
void startSync(store, api)

createRoot(root).render(
  <StrictMode>
    <Provider store={store}>
      <App />
    </Provider>
  </StrictMode>,
)
