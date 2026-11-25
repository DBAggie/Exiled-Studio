// The workspace (docs/ui, global header and side menu) and the start
// screen. Screens read projections from the store; every change is a
// command to the application.
import { Dialog, DropdownMenu } from 'radix-ui'
import {
  Fragment,
  useEffect,
  useId,
  useRef,
  useState,
  type ReactNode,
} from 'react'

import type { DataVersionView, LoadoutView } from './api/types'
import {
  Badge,
  Brand,
  ErrorAlert,
  Icon,
  type IconName,
  LiveRegion,
} from './components'
import { errorText, t, tDynamic } from './i18n'
import {
  actions,
  attempt,
  fetchData,
  closeBuild,
  perform,
  replaceBuild,
  run,
  saveBuild,
  setPrefs,
  type Screen,
} from './store'
import { Breakdown } from './screens/Breakdown'
import { Dashboard } from './screens/Dashboard'
import { ImportReview } from './screens/ImportReview'
import { ItemEditor, Items } from './screens/Items'
import { About, AiConnection, Warnings } from './screens/Other'
import { UpdatePrompt } from './screens/Updates'
import { Config } from './screens/Config'
import { Skills } from './screens/Skills'
import { Start } from './screens/Start'
import { Tree } from './screens/Tree'
import {
  buildStatus,
  dataName,
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
      // Nothing to act on: said quietly (UX audit A-04).
      return (
        // A dot, named on hover and for a screen reader: the header's
        // room goes to the build's state (the revamp's screens).
        <span className="calc-quiet" title={t('calc.done')}>
          <span className="calc-dot" aria-hidden="true" />
          <span className="visually-hidden">{t('calc.done')}</span>
        </span>
      )
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

/** The most stages the header shows as a stepper. */
const MAX_STEPS = 5

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
  // A few stages are a stepper (the redesign): the build's progression
  // in order, one press each. More than fit, or sets that are mixed, keep
  // the list.
  if (active !== undefined && loadouts.length <= MAX_STEPS) {
    return (
      <span className="stage-steps" role="group" aria-label={t('header.stage')}>
        {loadouts.map((l, i) => (
          <Fragment key={l.spec}>
            {i > 0 && (
              <span className="stage-arrow" aria-hidden="true">
                ›
              </span>
            )}
            <button
              type="button"
              aria-pressed={l.active}
              onClick={() => {
                if (!l.active) {
                  void dispatch(run({ 'use-loadout': { spec: l.spec } }))
                }
              }}
            >
              {l.title}
            </button>
          </Fragment>
        ))}
      </span>
    )
  }
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
      {/* The name gives way in a narrow window: its whole text on hover. */}
      <span className="build-name" title={build?.name || undefined}>
        {build?.name === '' || build?.name === undefined
          ? t('app.untitled')
          : build.name}
      </span>
      {/* The build's state, said quietly: one line of text, not pills (the
          revamp's shell, 2026-11-25). Only a failure keeps its colour. */}
      <span className="header-status">
        {status !== null && (
          <span className={`status-${status.tone}`}>{status.text}</span>
        )}
        {status !== null && snapshot?.dirty === true && (
          <span aria-hidden="true">·</span>
        )}
        {snapshot?.dirty === true && <span>{t('header.unsaved')}</span>}
      </span>
      <span className="spacer" />
      <CalcStatus />
      {build != null && build.loadouts.length > 1 && (
        <StagePicker loadouts={build.loadouts} />
      )}
      <span className="header-history">
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
      </span>
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
  icon,
  label,
  children,
}: {
  screen: Screen
  current: boolean
  icon: IconName
  /** The page's name: the button's text, and its tooltip on the rail. */
  label: string
  /** A count beside the name. */
  children?: ReactNode
}) {
  const dispatch = useAppDispatch()
  return (
    <li>
      <button
        type="button"
        aria-current={current ? 'page' : undefined}
        title={label}
        onClick={() => {
          // The screen already shown does not mount again: move the focus
          // to its heading as a new screen would (keyboard test B1).
          dispatch(actions.go(screen))
          if (current) {
            document.querySelector<HTMLElement>('h1[tabindex]')?.focus()
          }
        }}
      >
        <Icon name={icon} />
        <span className="nav-text">{label}</span>
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
  const prefs = useAppSelector((s) => s.ui.prefs)
  const rail = prefs.rail === true
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
        <NavButton
          screen="dashboard"
          current={current('dashboard')}
          icon="nav-dashboard"
          label={t('nav.dashboard')}
        />
        <NavButton
          screen="items"
          current={current('items')}
          icon="nav-items"
          label={t('nav.items')}
        />
        <NavButton
          screen="skills"
          current={current('skills')}
          icon="nav-skills"
          label={t('nav.skills')}
        />
        <NavButton
          screen="tree"
          current={current('tree')}
          icon="nav-tree"
          label={t('nav.tree')}
        />
        <NavButton
          screen="config"
          current={current('config')}
          icon="nav-config"
          label={t('nav.config')}
        />
        <NavButton
          screen="warnings"
          current={current('warnings')}
          icon="nav-warnings"
          label={t('nav.warnings')}
        >
          {warnings > 0 && ' '}
          {warnings > 0 && (
            <Badge tone="warn" className="count">
              {warnings}
            </Badge>
          )}
        </NavButton>
      </ul>
      <ul className="nav-bottom">
        <NavButton
          screen="ai"
          current={current('ai')}
          icon="nav-ai"
          label={t('nav.ai')}
        >
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
        <NavButton
          screen="about"
          current={current('about')}
          icon="nav-about"
          label={t('nav.about')}
        />
        <li>
          <button
            type="button"
            title={t('header.close')}
            onClick={() =>
              dirty ? setConfirmClose(true) : void dispatch(closeBuild())
            }
          >
            <Icon name="nav-close" />
            <span className="nav-text">{t('header.close')}</span>
          </button>
        </li>
        <li>
          <button
            type="button"
            className="nav-toggle"
            aria-pressed={rail}
            title={rail ? t('nav.expand') : t('nav.collapse')}
            onClick={() => void dispatch(setPrefs({ ...prefs, rail: !rail }))}
          >
            <Icon name="rail" />
            <span className="nav-text">{t('nav.collapse')}</span>
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

/** Asks before an action replaces a build with unsaved changes (UX-01):
 * save first, go on without saving, or cancel. */
function ReplaceDialog() {
  const dispatch = useAppDispatch()
  const what = useAppSelector((s) => s.ui.confirm)
  const close = () => dispatch(actions.confirm(null))
  const go = async (save: boolean) => {
    if (what === null) return
    close()
    if (save) {
      const saved = await dispatch(saveBuild())
      // Cancelled or failed: keep the build (the error shows).
      if (saved === null || saved === 'nothing') return
    }
    await dispatch(perform(what))
  }
  return (
    <Dialog.Root
      open={what !== null}
      onOpenChange={(open) => {
        if (!open) close()
      }}
    >
      <Dialog.Portal>
        <Dialog.Content className="dialog-content">
          <Dialog.Title>{t('replace.title')}</Dialog.Title>
          <Dialog.Description className="muted">
            {t('replace.help')}
          </Dialog.Description>
          <div className="row">
            <button
              type="button"
              className="primary"
              onClick={() => void go(true)}
            >
              {t('replace.save')}
            </button>
            <button type="button" onClick={() => void go(false)}>
              {t('replace.discard')}
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

/** Shown while a file is dragged over the window (start page B). */
function DropHint() {
  const dragging = useAppSelector((s) => s.ui.dragging)
  if (!dragging) return null
  return (
    <div className="drop-hint" role="status">
      <p>{t('drop.hint')}</p>
    </div>
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
    case 'tree':
      return <Tree />
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
        <span className="header-status">{t('start.offline')}</span>
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
        <UpdatePrompt />
        {screen === 'about' ? <About /> : <Start />}
      </main>
      <footer className="app-footer">
        {/* The game data in one quiet line (start page B; UX-17: no error
            up front). */}
        <DataLine version={version} />
        {/* About says it itself: once on the page (UX-22). */}
        {screen !== 'about' && <span>{t('app.notice')}</span>}
        <span>{t('app.licensed')}</span>
      </footer>
    </div>
  )
}

function DataLine({
  version,
}: {
  version: DataVersionView | null | undefined
}) {
  const dispatch = useAppDispatch()
  const data = useAppSelector((s) => s.session.snapshot?.data)
  // Game data is the application's job (maintainer 2026-11-24): the shell
  // gets what is missing at start; this line says where that is, and
  // offers the one action when it could not (asked not to connect at
  // start, or the download failed).
  const auto = useAppSelector((s) => s.lists.dataAuto)
  const offline = useAppSelector(
    (s) => s.lists.updates?.mode === 'strict-offline',
  )
  const error = data?.error
  const installed = data?.installed ?? []
  const getting = auto?.state === 'checking' || auto?.state === 'downloading'
  const none = version == null && data?.loading !== true
  return (
    <span className="data-line">
      {version != null
        ? t('start.dataReady', {
            name: dataName(version, data?.['tree-version']),
          })
        : data?.loading === true
          ? t('start.dataLoading')
          : getting
            ? auto.state === 'downloading'
              ? t('start.dataDownloading')
              : t('start.dataChecking')
            : error != null
              ? t('start.dataFailed')
              : t('start.dataNoneYet')}{' '}
      {data?.notice != null && (
        <>{tDynamic(`notice.${data.notice.code}`, 'error.generic')} </>
      )}
      {none && !getting && auto?.state === 'failed' && (
        <>
          {tDynamic(
            `start.dataWhy.${auto.error?.code ?? ''}`,
            'start.dataWhy.other',
          )}{' '}
        </>
      )}
      {/* With installed data, the choice is About's list (B-1). Without:
          the signed download, or, never connecting, the signed file. The
          development folder is About's only. */}
      {installed.length > 0 || version != null ? (
        <button
          type="button"
          className="link"
          onClick={() => dispatch(actions.go('about'))}
        >
          {version != null ? t('start.dataChange') : t('start.dataChoose')}
        </button>
      ) : getting || data?.loading === true ? null : offline ? (
        <button
          type="button"
          className="link"
          onClick={() => void dispatch(attempt((api) => api.installBundle()))}
        >
          {t('data.install')}
        </button>
      ) : (
        <button
          type="button"
          className="link"
          onClick={() => void dispatch(fetchData())}
        >
          {auto?.state === 'failed'
            ? t('start.dataRetry')
            : t('start.dataDownload')}
        </button>
      )}
    </span>
  )
}

/** The open build was made with another installed bundle than the loaded
 * one (ADR-010): offered, never switched silently. */
function PinnedData() {
  const dispatch = useAppDispatch()
  const pinned = useAppSelector((s) => s.session.snapshot?.data?.pinned)
  if (pinned == null) return null
  return (
    <div className="panel" role="note">
      <p>
        {t('data.pinned', { id: pinned })}{' '}
        <button
          type="button"
          onClick={() => void dispatch(run({ 'use-bundle': { id: pinned } }))}
        >
          {t('data.use')}
        </button>
      </p>
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
 * redo, Ctrl+S save, Ctrl+O open, Ctrl+N new build (asking first when
 * either would replace unsaved changes). In a text field, undo and redo
 * stay the field's own. Ctrl+N and Ctrl+O are the browser's otherwise (a
 * new window, a file), so they are always taken. */
function useShortcuts(hasBuild: boolean) {
  const dispatch = useAppDispatch()
  const hasData = useAppSelector(
    (s) => s.session.snapshot?.data?.['data-version'] != null,
  )
  // A question is open (replace this build?): its own keys only.
  const asking = useAppSelector((s) => s.ui.confirm !== null)
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (!(e.ctrlKey || e.metaKey) || e.altKey || asking) return
      const key = e.key.toLowerCase()
      const text = inTextField(e.target)
      if (key === 's' && hasBuild) {
        e.preventDefault()
        void dispatch(saveBuild())
      } else if (key === 'o') {
        e.preventDefault()
        void dispatch(replaceBuild('open-file'))
      } else if (key === 'n') {
        e.preventDefault()
        if (hasData) void dispatch(replaceBuild('new-build'))
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
  }, [dispatch, hasBuild, hasData, asking])
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
      <ReplaceDialog />
      <DropHint />
    </>
  )
}

function Workspace({ screen }: { screen: Screen }) {
  const rail = useAppSelector((s) => s.ui.prefs.rail === true)
  const content = useRef<HTMLElement>(null)
  // The page scrolls inside the frame (the header and the side menu stay
  // put): a new page starts at its top.
  useEffect(() => {
    content.current?.scrollTo?.(0, 0)
  }, [screen])
  return (
    <div className={rail ? 'app nav-rail' : 'app'}>
      <Header />
      <SideNav />
      <main className="content" ref={content}>
        <ErrorAlert />
        <UpdatePrompt />
        <Flash />
        <Notices />
        <PinnedData />
        <Content key={screen} screen={screen} />
      </main>
      <ShareCodeDialog />
    </div>
  )
}
