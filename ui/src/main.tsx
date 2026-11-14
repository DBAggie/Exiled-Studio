import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import { Provider } from 'react-redux'

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
const store = makeStore(api)
void startSync(store, api)

createRoot(root).render(
  <StrictMode>
    <Provider store={store}>
      <App />
    </Provider>
  </StrictMode>,
)
