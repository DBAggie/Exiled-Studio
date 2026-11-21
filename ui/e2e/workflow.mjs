// The real application under WebDriver (Phase 4 plan, decision 14 d):
// the built app (loom-tauri with the `e2e` feature), in WebView2 on
// Windows and WebKitGTK on Linux, through tauri-driver. No dependencies:
// W3C WebDriver over fetch.
//
//   node e2e/workflow.mjs <app binary> <work dir>
//
// The work dir holds import.xml (the build the "open" dialog returns) and
// bundle.loombundle (the pack the "install game data" dialog returns); the
// app saves saved.json and exports export.xml there (the e2e picker). The
// caller starts tauri-driver on 127.0.0.1:4444 and sets LOOM_E2E_DIR,
// LOOM_BUNDLE_DIR, and LOOM_DATA_DIR for the app.
import { existsSync, readFileSync } from 'node:fs'
import { join } from 'node:path'

const [app, work] = process.argv.slice(2)
if (app === undefined || work === undefined) {
  console.error('usage: node e2e/workflow.mjs <app binary> <work dir>')
  process.exit(2)
}

const DRIVER = 'http://127.0.0.1:4444'
const ELEMENT = 'element-6066-11e4-a52e-4f735466cecf'
const TIMEOUT = 60_000

async function wd(method, path, body) {
  const res = await fetch(`${DRIVER}${path}`, {
    method,
    headers: { 'content-type': 'application/json' },
    body: body === undefined ? undefined : JSON.stringify(body),
  })
  const json = await res.json()
  if (!res.ok) {
    throw new Error(
      `${method} ${path}: ${res.status} ${JSON.stringify(json.value)}`,
    )
  }
  return json.value
}

const sleep = (ms) => new Promise((r) => setTimeout(r, ms))

async function until(what, f) {
  const start = Date.now()
  let last
  while (Date.now() - start < TIMEOUT) {
    try {
      const v = await f()
      if (v) return v
    } catch (e) {
      last = e
    }
    await sleep(200)
  }
  throw new Error(
    `timed out waiting for ${what}${last ? `: ${last.message}` : ''}`,
  )
}

const lit = (s) =>
  s.includes("'") ? `concat('${s.split("'").join(`', "'", '`)}')` : `'${s}'`

let session

async function find(xpath) {
  const v = await wd('POST', `/session/${session}/element`, {
    using: 'xpath',
    value: xpath,
  })
  return v[ELEMENT]
}

const button = (name) =>
  `//button[normalize-space(.)=${lit(name)} or @aria-label=${lit(name)}]`

async function click(xpath, what) {
  const id = await until(what, () => find(xpath))
  await wd('POST', `/session/${session}/element/${id}/click`, {})
}

async function text(what) {
  return until(`text "${what}"`, async () => {
    const source = await wd('GET', `/session/${session}/source`)
    return source.includes(what)
  })
}

function step(name) {
  console.log(`- ${name}`)
}

try {
  session = (
    await wd('POST', '/session', {
      capabilities: {
        alwaysMatch: { 'tauri:options': { application: app } },
      },
    })
  ).sessionId

  step('the start screen')
  await text('What do you want to do?')

  step('import a file the user picks')
  await click(
    button('Import a Path of Building file (.xml)…'),
    'the import button',
  )
  await text('Item lines read')
  await click(button('Open the dashboard'), 'continue')
  await text('Up to date')

  step('edit an item and apply it')
  await click(button('Equipment'), 'the equipment menu')
  await click(button('Edit Loom Test Band'), 'the item')
  // The synthetic build's "+15 to maximum Life" is line 7 of item 1
  // (tools/parity-cli/testdata/build-round-trip/synthetic-poe1.xml). A
  // modifier line shows as text until its Edit button is pressed.
  await click(button('Edit line 7'), 'the edit button of the life line')
  const input = await until('the life line', () =>
    find(`//input[@aria-label='Line 7 text']`),
  )
  await wd('POST', `/session/${session}/element/${input}/clear`, {})
  await wd('POST', `/session/${session}/element/${input}/value`, {
    text: '+25 to maximum Life',
  })
  await click(button('Apply'), 'apply')
  await text('Edit applied. Undo is available.')
  await text('Up to date')

  step('undo')
  await click(button('Undo'), 'undo')
  await text('Up to date')

  step('allocate a node on the passive tree')
  // run.sh and run.ps1 import the build as the synthetic tree bundle's
  // Test Brute with only its start (crates/loom-data/tests/fixtures/
  // bundle-poe1-tree, made by make_bundles.py).
  await click(button('Passive tree'), 'the tree screen')
  await text('Passive points: 0 ·')
  await click(button('List'), 'the list view')
  const search = await until('the node search', () =>
    find(`//input[@type='search']`),
  )
  await wd('POST', `/session/${session}/element/${search}/value`, {
    text: 'Test Notable 0-2',
  })
  await click(button('Allocate Test Notable 0-2'), 'the notable')
  await text('Remove Test Notable 0-2')
  await until('the points spent', async () => {
    const source = await wd('GET', `/session/${session}/source`)
    return !source.includes('Passive points: 0 ·')
  })
  await text('Up to date')

  step('socket a cluster jewel and allocate a node it adds (T5b)')
  // The large socket 5000 takes the build's large cluster jewel (item 5,
  // added by run.sh and run.ps1); its notables then allocate.
  // The list re-renders after an allocation or a socketed jewel, which can
  // drop what was typed, so the box is found again, cleared, and typed
  // into until the wanted button shows (CI saw "element not interactable"
  // on the first clear, and later the added notable never listed).
  const searchFor = (what, wanted) =>
    until(`the node search for ${what}`, async () => {
      const box = await find(`//input[@type='search']`)
      await wd('POST', `/session/${session}/element/${box}/clear`, {})
      await wd('POST', `/session/${session}/element/${box}/value`, {
        text: what,
      })
      for (let i = 0; i < 10; i++) {
        try {
          return await find(button(wanted))
        } catch {
          await sleep(200)
        }
      }
      return false
    })
  await searchFor('Test Large Jewel Socket', 'Allocate Test Large Jewel Socket')
  await click(button('Allocate Test Large Jewel Socket'), 'the large socket')
  await click(
    button('Choose jewel for Test Large Jewel Socket'),
    "the socket's jewel picker",
  )
  await click(
    `//button[contains(normalize-space(.), 'Loom Test Cluster')]`,
    'the cluster jewel',
  )
  await text('Up to date')
  await searchFor('Test Cluster Notable A', 'Allocate Test Cluster Notable A')
  await click(button('Allocate Test Cluster Notable A'), 'the added notable')
  await text('Remove Test Cluster Notable A')
  await text('Up to date')

  step('save (the first save asks where)')
  await click(button('Save'), 'save')
  await until('saved.json', () => existsSync(join(work, 'saved.json')))

  step('export Path of Building XML')
  await click(
    `//button[starts-with(normalize-space(.), 'Export')]`,
    'the export menu',
  )
  await click(
    `//*[@role='menuitem' and normalize-space(.)=${lit('Path of Building file (.xml)…')}]`,
    'the XML export',
  )
  await until('export.xml', () => existsSync(join(work, 'export.xml')))
  const xml = readFileSync(join(work, 'export.xml'), 'utf8')
  if (!xml.includes('<PathOfBuilding')) {
    throw new Error('export.xml is not a PoB build')
  }
  // The allocated notable (node 10023) is in the exported spec.
  if (!/<Spec [^>]*nodes="[^"]*\b10023\b/.test(xml)) {
    throw new Error('export.xml does not have the allocated notable')
  }
  // So are the cluster jewel's socket and a node it adds (65536 and up).
  if (!/<Socket nodeId="5000" itemId="5"/.test(xml)) {
    throw new Error('export.xml does not have the socketed cluster jewel')
  }
  if (!/<Spec [^>]*nodes="[^"]*\b6\d{4}\b/.test(xml)) {
    throw new Error('export.xml does not have a cluster jewel node')
  }

  step('install game data from a signed pack (Phase 5, P4)')
  // run.sh and run.ps1 pack the synthetic tree bundle signed with the
  // public test key, which an e2e build trusts; the e2e picker returns it.
  await click(button('About'), 'About')
  await click(button('Install game data from a file…'), 'the install button')
  await text('Game data installed.')
  await text('Signed by the project')

  console.log('e2e workflow passed')
} catch (e) {
  console.error(`e2e workflow failed: ${e.message}`)
  if (session !== undefined) {
    try {
      // The page's text, not its markup (the markup's icons filled the
      // first 4,000 characters), and the search box's value.
      const shown = await wd('POST', `/session/${session}/execute/sync`, {
        script:
          "const s = document.querySelector('input[type=search]'); return `search: ${s ? JSON.stringify(s.value) : 'none'}\\n` + document.body.innerText",
        args: [],
      })
      console.error(String(shown).slice(0, 8000))
    } catch {
      // The page is gone.
    }
  }
  process.exitCode = 1
} finally {
  if (session !== undefined) {
    await wd('DELETE', `/session/${session}`).catch(() => {})
  }
}
