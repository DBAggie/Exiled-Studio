// The real application under WebDriver (Phase 4 plan, decision 14 d):
// the built app (loom-tauri with the `e2e` feature), in WebView2 on
// Windows and WebKitGTK on Linux, through tauri-driver. No dependencies:
// W3C WebDriver over fetch.
//
//   node e2e/workflow.mjs <app binary> <work dir>
//
// The work dir holds import.xml (the build the "open" dialog returns); the
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
  await text('Import a Path of Building build')

  step('import a file the user picks')
  await click(button('Open a .xml file…'), 'the import button')
  await text('Import result')
  await click(button('Open the dashboard'), 'continue')
  await text('Calculation up to date')

  step('edit an item and apply it')
  await click(button('Equipment'), 'the equipment menu')
  await click(button('Edit Loom Test Band'), 'the item')
  // The synthetic build's "+15 to maximum Life" is line 7 of item 1
  // (tools/parity-cli/testdata/build-round-trip/synthetic-poe1.xml).
  const input = await until('the life line', () =>
    find(`//input[@aria-label='Line 7 text']`),
  )
  await wd('POST', `/session/${session}/element/${input}/clear`, {})
  await wd('POST', `/session/${session}/element/${input}/value`, {
    text: '+25 to maximum Life',
  })
  await click(button('Apply'), 'apply')
  await text('Edit applied. Undo is available.')
  await text('Calculation up to date')

  step('undo')
  await click(button('Undo'), 'undo')
  await text('Calculation up to date')

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

  console.log('e2e workflow passed')
} catch (e) {
  console.error(`e2e workflow failed: ${e.message}`)
  if (session !== undefined) {
    try {
      const source = await wd('GET', `/session/${session}/source`)
      console.error(source.slice(0, 4000))
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
