// The workspace (docs/ui, global header and side menu) and the start
// screen. Screens read projections from the store; every change is a
// command to the application.
import { Dialog, DropdownMenu } from 'radix-ui'
import { useEffect, useId, useState, type ReactNode } from 'react'

import type { LoadoutView } from './api/types'
import { Badge, Brand, ErrorAlert, Icon, LiveRegion } from './components'
import { errorText, t, tDynamic } from './i18n'
import {
  actions,
  attempt,
  closeBuild,
  openBuild,
  run,
  saveBuild,
  type Screen,
} from './store'
import { Breakdown } from './screens/Breakdown'
import { Dashboard } from './screens/Dashboard'
import { ImportReview } from './screens/ImportReview'
import { ItemEditor, Items } from './screens/Items'
import { About, AiConnection, Warnings } from './screens/Other'
import { Config } from './screens/Config'
import { Skills } from './screens/Skills'
import { Start } from './screens/Start'
import {
  buildStatus,
  historyText,
  useAppDispatch,
  useAppSelector,
} from './util'

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
            onSelect={() =>
              void dispatch(attempt((api) => api.exportXml())).then((o) => {
                if (o !== null && o !== 'nothing') {
                  dispatch(actions.flash(t('flash.exportedXml')))
                }
              })
            }
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
          {/* Only what applies to this build (UX-25). */}
          {game === 'poe2' && (
            <DropdownMenu.Item className="menu-item" disabled>
              {t('export.poe2')}
            </DropdownMenu.Item>
          )}
          <DropdownMenu.Separator className="menu-separator" />
          <DropdownMenu.Item
            className="menu-item"
            onSelect={() => void dispatch(attempt((api) => api.saveAs()))}
          >
            {t('export.json')}
          </DropdownMenu.Item>
        </DropdownMenu.Content>
      </DropdownMenu.Portal>
    </DropdownMenu.Root>
  )
}

/** The build's stage (PoB's loadout): one choice sets the passive tree,
 * the item set, the skill set, and the configuration together. */
function StagePicker({ loadouts }: { loadouts: LoadoutView[] }) {
  const dispatch = useAppDispatch()
  const id = useId()
  const active = loadouts.find((l) => l.active)
  const spec = useAppSelector(
    (s) => s.session.snapshot?.build?.specs.find((x) => x.active)?.id,
  )
  const near = loadouts.find((l) => l.spec === spec)
  return (
    <span className="stage-picker">
      <label htmlFor={id}>{t('header.stage')}</label>
      <select
        id={id}
        value={active?.spec ?? ''}
        onChange={(e) =>
          void dispatch(
            run({ 'use-loadout': { spec: Number(e.target.value) } }),
          )
        }
      >
        {active === undefined && (
          <option value="" disabled>
            {near === undefined
              ? t('header.stageMixed')
              : t('header.stageMixedOf', { stage: near.title })}
          </option>
        )}
        {loadouts.map((l) => (
          <option key={l.spec} value={l.spec}>
            {l.title}
          </option>
        ))}
      </select>
    </span>
  )
}

function Header() {
  const dispatch = useAppDispatch()
  const snapshot = useAppSelector((s) => s.session.snapshot)
  const build = snapshot?.build
  const history = snapshot?.history
  const status = build == null ? null : buildStatus(build)
  // What undo and redo would change, in words (UX-23).
  const undoLabel =
    history?.undo != null && build != null
      ? t('header.undoWhat', { what: historyText(history.undo, build) })
      : t('header.undo')
  const redoLabel =
    history?.redo != null && build != null
      ? t('header.redoWhat', { what: historyText(history.redo, build) })
      : t('header.redo')
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
      {build != null && build.loadouts.length > 1 && (
        <StagePicker loadouts={build.loadouts} />
      )}
      <span className="spacer" />
      <CalcStatus />
      <button
        type="button"
        className="icon"
        aria-label={t('header.undo')}
        title={`${undoLabel} (Ctrl+Z)`}
        disabled={history?.['can-undo'] !== true}
        onClick={() => void dispatch(run('undo'))}
      >
        <Icon name="undo" />
      </button>
      <button
        type="button"
        className="icon"
        aria-label={t('header.redo')}
        title={`${redoLabel} (Ctrl+Y)`}
        disabled={history?.['can-redo'] !== true}
        onClick={() => void dispatch(run('redo'))}
      >
        <Icon name="redo" />
      </button>
      <ExportMenu />
      <button
        type="button"
        className="primary"
        title={`${t('header.save')} (Ctrl+S)`}
        onClick={() => void dispatch(saveBuild())}
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
  // Only warnings the calculation reads (UX-03).
  const warnings = useAppSelector(
    (s) =>
      s.session.snapshot?.build?.warnings.filter((w) => w['in-use']).length ??
      0,
  )
  const dirty = useAppSelector((s) => s.session.snapshot?.dirty === true)
  const [confirmClose, setConfirmClose] = useState(false)
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
        <NavButton screen="skills" current={current('skills')}>
          {t('nav.skills')}
        </NavButton>
        <NavButton screen="config" current={current('config')}>
          {t('nav.config')}
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
            onClick={() =>
              dirty ? setConfirmClose(true) : void dispatch(closeBuild())
            }
          >
            {t('header.close')}
          </button>
        </li>
      </ul>
      <CloseDialog open={confirmClose} onOpenChange={setConfirmClose} />
    </nav>
  )
}

/** Asks before closing a build with unsaved changes (UX-01): closing
 * drops them and their recovery copy. */
function CloseDialog({
  open,
  onOpenChange,
}: {
  open: boolean
  onOpenChange: (open: boolean) => void
}) {
  const dispatch = useAppDispatch()
  const saveAndClose = async () => {
    const saved = await dispatch(saveBuild())
    // Cancelled or failed: keep the build open (the error shows).
    if (saved === null || saved === 'nothing') {
      onOpenChange(false)
      return
    }
    onOpenChange(false)
    await dispatch(closeBuild())
  }
  return (
    <Dialog.Root open={open} onOpenChange={onOpenChange}>
      <Dialog.Portal>
        <Dialog.Content className="dialog-content">
          <Dialog.Title>{t('close.title')}</Dialog.Title>
          <Dialog.Description className="muted">
            {t('close.help')}
          </Dialog.Description>
          <div className="row">
            <button
              type="button"
              className="primary"
              onClick={() => void saveAndClose()}
            >
              {t('close.save')}
            </button>
            <button
              type="button"
              onClick={() => {
                onOpenChange(false)
                void dispatch(closeBuild())
              }}
            >
              {t('close.discard')}
            </button>
            <Dialog.Close asChild>
              <button type="button">{t('close.cancel')}</button>
            </Dialog.Close>
          </div>
        </Dialog.Content>
      </Dialog.Portal>
    </Dialog.Root>
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
    case 'skills':
      return <Skills />
    case 'config':
      return <Config />
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

/** A finished action confirmed in words (UX-25): saved, exported. */
function Flash() {
  const dispatch = useAppDispatch()
  const flash = useAppSelector((s) => s.ui.flash)
  if (flash === null) return null
  return (
    <div role="status" className="panel flash">
      <p>{flash}</p>
      <button type="button" onClick={() => dispatch(actions.flash(null))}>
        {t('error.dismiss')}
      </button>
    </div>
  )
}

function Notices() {
  const notices = useAppSelector((s) => s.session.snapshot?.notices ?? [])
  if (notices.length === 0) return null
  return (
    <div className="panel" role="note">
      {notices.map((n) => (
        <p key={n.code}>{tDynamic(`notice.${n.code}`, 'error.generic')}</p>
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
        {/* About says it itself: once on the page (UX-22). */}
        {screen !== 'about' && <span>{t('app.notice')}</span>}
        <span>{t('app.licensed')}</span>
      </footer>
    </div>
  )
}

/** Whether a key press belongs to a text field (its own undo). */
function inTextField(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false
  return (
    target.isContentEditable ||
    target instanceof HTMLTextAreaElement ||
    (target instanceof HTMLInputElement &&
      !['checkbox', 'radio', 'button', 'submit'].includes(target.type))
  )
}

/** The keyboard shortcuts (UX-23): Ctrl+Z undo, Ctrl+Y or Ctrl+Shift+Z
 * redo, Ctrl+S save, Ctrl+O open. In a text field, undo and redo stay the
 * field's own. */
function useShortcuts(hasBuild: boolean) {
  const dispatch = useAppDispatch()
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (!(e.ctrlKey || e.metaKey) || e.altKey) return
      const key = e.key.toLowerCase()
      const text = inTextField(e.target)
      if (key === 's' && hasBuild) {
        e.preventDefault()
        void dispatch(saveBuild())
      } else if (key === 'o') {
        e.preventDefault()
        void dispatch(openBuild((api) => api.openFile()))
      } else if (key === 'z' && !e.shiftKey && hasBuild && !text) {
        e.preventDefault()
        void dispatch(run('undo'))
      } else if (
        (key === 'y' || (key === 'z' && e.shiftKey)) &&
        hasBuild &&
        !text
      ) {
        e.preventDefault()
        void dispatch(run('redo'))
      }
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  }, [dispatch, hasBuild])
}

export function App() {
  const screen = useAppSelector((s) => s.ui.screen)
  const hasBuild = useAppSelector((s) => s.session.snapshot?.build != null)
  const workspace = hasBuild && screen !== 'start'
  useShortcuts(hasBuild)
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
        <Flash />
        <Notices />
        <Content key={screen} screen={screen} />
      </main>
      <ShareCodeDialog />
    </div>
  )
}
