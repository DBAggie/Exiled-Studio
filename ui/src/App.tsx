// The workspace (docs/ui, global header and side menu) and the start
// screen. Screens read projections from the store; every change is a
// command to the application.
import { Dialog, DropdownMenu } from 'radix-ui'
import { useState, type ReactNode } from 'react'

import { Badge, Brand, ErrorAlert, Icon, LiveRegion } from './components'
import { errorText, t } from './i18n'
import { actions, attempt, run, type Screen } from './store'
import { Breakdown } from './screens/Breakdown'
import { Dashboard } from './screens/Dashboard'
import { ImportReview } from './screens/ImportReview'
import { ItemEditor, Items } from './screens/Items'
import { About, AiConnection, Warnings } from './screens/Other'
import { Start } from './screens/Start'
import { buildStatus, useAppDispatch, useAppSelector } from './util'

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
          <Dialog.Description className="muted">
            {t('export.codeHelp')}
          </Dialog.Description>
          <textarea
            readOnly
            rows={4}
            value={code ?? ''}
            aria-label={t('export.codeTitle')}
          />
          <div className="row">
            <button
              type="button"
              className="primary"
              onClick={() => {
                void navigator.clipboard
                  ?.writeText(code ?? '')
                  .then(() => setCopied(true))
              }}
            >
              <Icon name="copy" />
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
        <button type="button">
          {t('header.export')}
          <Icon name="chevron" size={16} />
        </button>
      </DropdownMenu.Trigger>
      <DropdownMenu.Portal>
        <DropdownMenu.Content
          className="menu-content"
          align="end"
          sideOffset={6}
        >
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
  const status = build == null ? null : buildStatus(build)
  return (
    <header className="app-header">
      <Brand />
      <span aria-hidden="true" className="muted">
        /
      </span>
      <span className="build-name">
        {build?.name === '' || build?.name === undefined
          ? t('app.untitled')
          : build.name}
      </span>
      {status !== null && <Badge tone={status.tone}>{status.text}</Badge>}
      {snapshot?.dirty === true && (
        <Badge tone="neutral">{t('header.unsaved')}</Badge>
      )}
      <span className="spacer" />
      <CalcStatus />
      <button
        type="button"
        className="icon"
        aria-label={t('header.undo')}
        title={t('header.undo')}
        disabled={history?.['can-undo'] !== true}
        onClick={() => void dispatch(run('undo'))}
      >
        <Icon name="undo" />
      </button>
      <button
        type="button"
        className="icon"
        aria-label={t('header.redo')}
        title={t('header.redo')}
        disabled={history?.['can-redo'] !== true}
        onClick={() => void dispatch(run('redo'))}
      >
        <Icon name="redo" />
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

function NavButton({
  screen,
  current,
  children,
}: {
  screen: Screen
  current: boolean
  children: ReactNode
}) {
  const dispatch = useAppDispatch()
  return (
    <li>
      <button
        type="button"
        aria-current={current ? 'page' : undefined}
        onClick={() => dispatch(actions.go(screen))}
      >
        {children}
      </button>
    </li>
  )
}

function SideNav() {
  const dispatch = useAppDispatch()
  const screen = useAppSelector((s) => s.ui.screen)
  const warnings = useAppSelector(
    (s) => s.session.snapshot?.build?.warnings.length ?? 0,
  )
  const pending = useAppSelector(
    (s) =>
      s.lists.mcp?.requests.filter((r) => r.status === 'pending').length ?? 0,
  )
  const current = (s: Screen) =>
    s === screen ||
    (s === 'dashboard' &&
      (screen === 'breakdown' || screen === 'import-review')) ||
    (s === 'items' && screen === 'item')
  return (
    <nav className="side-nav" aria-label={t('nav.label')}>
      <ul>
        <NavButton screen="dashboard" current={current('dashboard')}>
          {t('nav.dashboard')}
        </NavButton>
        <NavButton screen="items" current={current('items')}>
          {t('nav.items')}
        </NavButton>
        <NavButton screen="warnings" current={current('warnings')}>
          {t('nav.warnings')}
          {warnings > 0 && ' '}
          {warnings > 0 && (
            <Badge tone="warn" className="count">
              {warnings}
            </Badge>
          )}
        </NavButton>
      </ul>
      <ul className="nav-bottom">
        <NavButton screen="ai" current={current('ai')}>
          {t('nav.ai')}
          {pending > 0 && ' '}
          {pending > 0 && (
            <Badge tone="info" className="count">
              <span className="visually-hidden">
                {t('nav.pending', { n: pending })}
              </span>
              <span aria-hidden="true">{pending}</span>
            </Badge>
          )}
        </NavButton>
        <NavButton screen="about" current={current('about')}>
          {t('nav.about')}
        </NavButton>
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

/** The start screen's frame: brand, offline and data status, About; the
 * non-affiliation notice in the footer. */
function StartFrame({ screen }: { screen: Screen }) {
  const dispatch = useAppDispatch()
  const version = useAppSelector(
    (s) => s.session.snapshot?.data?.['data-version'],
  )
  return (
    <div className="start-page">
      <header className="start-header">
        <Brand />
        <span className="spacer" />
        <Badge tone="neutral">{t('start.offline')}</Badge>
        {version != null ? (
          <Badge tone="ok">
            {t('start.dataBadge', {
              game: version.game === 'poe1' ? 'PoE1' : 'PoE2',
              build: version['steam-build'] ?? '?',
            })}
          </Badge>
        ) : (
          <Badge tone="warn">{t('start.noDataBadge')}</Badge>
        )}
        {screen === 'about' ? (
          <button type="button" onClick={() => dispatch(actions.go('start'))}>
            {t('nav.start')}
          </button>
        ) : (
          <button type="button" onClick={() => dispatch(actions.go('about'))}>
            {t('nav.about')}
          </button>
        )}
      </header>
      <main className="start">
        <ErrorAlert />
        {screen === 'about' ? <About /> : <Start />}
      </main>
      <footer className="app-footer">
        <span>{t('app.notice')}</span>
        <span>{t('app.licensed')}</span>
      </footer>
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
        <StartFrame screen={screen} />
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
