// The workspace (docs/ui, global header and side menu) and the start
// screen. Screens read projections from the store; every change is a
// command to the application.
import { Dialog, DropdownMenu } from 'radix-ui'
import { useState } from 'react'

import { Badge, ErrorAlert, LiveRegion } from './components'
import { errorText, t } from './i18n'
import { actions, attempt, run, type Screen } from './store'
import { Breakdown } from './screens/Breakdown'
import { Dashboard } from './screens/Dashboard'
import { ImportReview } from './screens/ImportReview'
import { ItemEditor, Items } from './screens/Items'
import { About, AiConnection, Warnings } from './screens/Other'
import { Start } from './screens/Start'
import { useAppDispatch, useAppSelector } from './util'

function CalcStatus() {
  const calc = useAppSelector((s) => s.session.snapshot?.calc)
  if (calc === undefined) return null
  switch (calc.state) {
    case 'running':
      return <Badge tone="info">{t('calc.running')}</Badge>
    case 'done':
      return <Badge tone="ok">{t('calc.done')}</Badge>
    case 'failed':
      return (
        <Badge tone="danger">
          {t('calc.failed', { reason: errorText(calc.error?.code ?? '') })}
        </Badge>
      )
    default:
      return <Badge tone="neutral">{t('calc.idle')}</Badge>
  }
}

function ShareCodeDialog() {
  const dispatch = useAppDispatch()
  const code = useAppSelector((s) => s.ui.shareCode)
  const [copied, setCopied] = useState(false)
  return (
    <Dialog.Root
      open={code !== null}
      onOpenChange={(open) => {
        if (!open) {
          dispatch(actions.shareCode(null))
          setCopied(false)
        }
      }}
    >
      <Dialog.Portal>
        <Dialog.Content className="dialog-content">
          <Dialog.Title>{t('export.codeTitle')}</Dialog.Title>
          <Dialog.Description>{t('export.codeHelp')}</Dialog.Description>
          <textarea
            readOnly
            rows={4}
            value={code ?? ''}
            aria-label={t('export.codeTitle')}
          />
          <div className="row">
            <button
              type="button"
              onClick={() => {
                void navigator.clipboard
                  ?.writeText(code ?? '')
                  .then(() => setCopied(true))
              }}
            >
              {copied ? t('export.copied') : t('export.copy')}
            </button>
            <Dialog.Close asChild>
              <button type="button">{t('dialog.close')}</button>
            </Dialog.Close>
          </div>
        </Dialog.Content>
      </Dialog.Portal>
    </Dialog.Root>
  )
}

function ExportMenu() {
  const dispatch = useAppDispatch()
  const game = useAppSelector((s) => s.session.snapshot?.build?.game)
  return (
    <DropdownMenu.Root>
      <DropdownMenu.Trigger asChild>
        <button type="button">{t('header.export')} ▾</button>
      </DropdownMenu.Trigger>
      <DropdownMenu.Portal>
        <DropdownMenu.Content className="menu-content" align="end">
          <DropdownMenu.Item
            className="menu-item"
            onSelect={() => void dispatch(attempt((api) => api.saveAs()))}
          >
            {t('export.json')}
          </DropdownMenu.Item>
          <DropdownMenu.Item
            className="menu-item"
            onSelect={() => void dispatch(attempt((api) => api.exportXml()))}
          >
            {t('export.pobXml')}
          </DropdownMenu.Item>
          <DropdownMenu.Item
            className="menu-item"
            onSelect={() =>
              void dispatch(run('export-share-code')).then((o) => {
                if (o !== null && typeof o === 'object' && 'text' in o) {
                  dispatch(actions.shareCode(o.text.text))
                }
              })
            }
          >
            {t('export.shareCode')}
          </DropdownMenu.Item>
          <DropdownMenu.Item className="menu-item" disabled={game !== 'poe2'}>
            {t('export.poe2')}
          </DropdownMenu.Item>
        </DropdownMenu.Content>
      </DropdownMenu.Portal>
    </DropdownMenu.Root>
  )
}

function Header() {
  const dispatch = useAppDispatch()
  const snapshot = useAppSelector((s) => s.session.snapshot)
  const build = snapshot?.build
  const history = snapshot?.history
  return (
    <header className="app-header">
      <span className="brand">{t('app.name')}</span>
      <span aria-hidden="true">›</span>
      <span>
        {build?.name === '' || build?.name === undefined
          ? t('app.untitled')
          : build.name}
      </span>
      {snapshot?.dirty === true && (
        <Badge tone="neutral">{t('header.unsaved')}</Badge>
      )}
      <CalcStatus />
      <span className="spacer" />
      <button
        type="button"
        disabled={history?.['can-undo'] !== true}
        onClick={() => void dispatch(run('undo'))}
      >
        {t('header.undo')}
      </button>
      <button
        type="button"
        disabled={history?.['can-redo'] !== true}
        onClick={() => void dispatch(run('redo'))}
      >
        {t('header.redo')}
      </button>
      <ExportMenu />
      <button
        type="button"
        className="primary"
        onClick={() =>
          void dispatch(
            attempt((api) =>
              snapshot?.path === null || snapshot?.path === undefined
                ? api.saveAs()
                : api.execute('save'),
            ),
          )
        }
      >
        {t('header.save')}
      </button>
    </header>
  )
}

const NAV: { screen: Screen; label: () => string }[] = [
  { screen: 'dashboard', label: () => t('nav.dashboard') },
  { screen: 'items', label: () => t('nav.items') },
  { screen: 'ai', label: () => t('nav.ai') },
  { screen: 'about', label: () => t('nav.about') },
]

function SideNav() {
  const dispatch = useAppDispatch()
  const screen = useAppSelector((s) => s.ui.screen)
  const warnings = useAppSelector(
    (s) => s.session.snapshot?.build?.warnings.length ?? 0,
  )
  const entries = [
    ...NAV.slice(0, 2),
    {
      screen: 'warnings' as Screen,
      label: () => t('nav.warnings', { n: warnings }),
    },
    ...NAV.slice(2),
  ]
  const current = (s: Screen) =>
    s === screen ||
    (s === 'dashboard' &&
      (screen === 'breakdown' || screen === 'import-review')) ||
    (s === 'items' && screen === 'item')
  return (
    <nav className="side-nav" aria-label={t('nav.label')}>
      <ul>
        {entries.map((e) => (
          <li key={e.screen}>
            <button
              type="button"
              aria-current={current(e.screen) ? 'page' : undefined}
              onClick={() => dispatch(actions.go(e.screen))}
            >
              {e.label()}
            </button>
          </li>
        ))}
        <li>
          <button
            type="button"
            onClick={() => {
              void dispatch(run('close'))
              dispatch(actions.go('start'))
            }}
          >
            {t('header.close')}
          </button>
        </li>
      </ul>
    </nav>
  )
}

function Content({ screen }: { screen: Screen }) {
  switch (screen) {
    case 'import-review':
      return <ImportReview />
    case 'dashboard':
      return <Dashboard />
    case 'breakdown':
      return <Breakdown />
    case 'items':
      return <Items />
    case 'item':
      return <ItemEditor />
    case 'warnings':
      return <Warnings />
    case 'ai':
      return <AiConnection />
    case 'about':
      return <About />
    default:
      return null
  }
}

function Notices() {
  const notices = useAppSelector((s) => s.session.snapshot?.notices ?? [])
  if (notices.length === 0) return null
  return (
    <div className="panel" role="note">
      {notices.map((n) => (
        <p key={n.code}>
          {n.code === 'APP_RESOLUTION_CHANGED'
            ? t('notice.APP_RESOLUTION_CHANGED')
            : n.code}
        </p>
      ))}
    </div>
  )
}

export function App() {
  const screen = useAppSelector((s) => s.ui.screen)
  const hasBuild = useAppSelector((s) => s.session.snapshot?.build != null)
  const workspace = hasBuild && screen !== 'start'
  // One live region for the whole session, so announcements survive the
  // change from the start screen to the workspace.
  return (
    <>
      <LiveRegion />
      {workspace ? (
        <Workspace screen={screen} />
      ) : (
        <main>
          <ErrorAlert />
          {screen === 'about' ? <About /> : <Start />}
        </main>
      )}
    </>
  )
}

function Workspace({ screen }: { screen: Screen }) {
  return (
    <div className="app">
      <Header />
      <SideNav />
      <main className="content">
        <ErrorAlert />
        <Notices />
        <Content key={screen} screen={screen} />
      </main>
      <ShareCodeDialog />
    </div>
  )
}
