// Warnings, AI connection (screen 6, docs/ui AiApproval), and About.
import { useState } from 'react'

import type {
  ChangeRequest,
  DataFeedStatus,
  DiagnosticFile,
  FieldDelta,
  ItemSummary,
  LineOp,
  TreeLayout,
} from '../api/types'
import {
  Badge,
  ErrorCode,
  Hint,
  Icon,
  ScreenHeading,
  type Tone,
} from '../components'
import {
  errorText,
  fieldLabel,
  formatNumber,
  t,
  tDynamic,
  type MessageKey,
} from '../i18n'
import { ReviewList } from '../ReviewLines'
import { actions, attempt, query, run, setPermissions } from '../store'
import {
  dataName,
  itemName,
  splitWarnings,
  useAppDispatch,
  useAppSelector,
  withUnit,
} from '../util'
import { useTreeLayout } from './treeLayout'
import { UpdateSettings } from './Updates'

export function Warnings() {
  const build = useAppSelector((s) => s.session.snapshot?.build)
  if (build == null) return null
  const { used, unused } = splitWarnings(build)
  return (
    <>
      <div className="page-head">
        <ScreenHeading>{t('warnings.title')}</ScreenHeading>
        <Hint>{t('warnings.help')}</Hint>
      </div>
      {build.warnings.length === 0 ? (
        <p>{t('warnings.none')}</p>
      ) : (
        <>
          <h2>{t('warnings.used')}</h2>
          {used.length === 0 ? (
            <p>{t('warnings.usedNone')}</p>
          ) : (
            <section className="panel" aria-label={t('warnings.used')}>
              <ReviewList warnings={used} />
            </section>
          )}
          {unused.length > 0 && (
            <>
              <h2>{t('warnings.unused')}</h2>
              <p className="muted">{t('warnings.unusedHelp')}</p>
              <section className="panel" aria-label={t('warnings.unused')}>
                <ReviewList warnings={unused} />
              </section>
            </>
          )}
        </>
      )}
    </>
  )
}

/** A request's edit as what it removes and what it adds, in words: the
 * lines the AI saw (`expect`) and the text it asks for. */
function changeBlocks(r: ChangeRequest): { remove: string[]; add: string[] } {
  const edit = r.edit
  const remove: string[] = []
  const add: string[] = []
  if (edit == null) return { remove, add }
  const seen = (line: number) =>
    edit.expect.find((e) => e.line === line)?.text ??
    t('ai.lineN', { n: line + 1 })
  for (const op of edit.ops as LineOp[]) {
    if ('replace' in op) {
      remove.push(seen(op.replace.line))
      add.push(op.replace.text)
    } else if ('remove' in op) {
      remove.push(seen(op.remove.line))
    } else if ('add' in op) {
      add.push(op.add.text)
    } else if ('set-range' in op) {
      remove.push(seen(op['set-range'].line))
      add.push(
        t('ai.setRange', {
          line: seen(op['set-range'].line),
          roll: formatNumber(op['set-range'].value * 100),
        }),
      )
    }
  }
  return { remove, add }
}

/** A tree request's change as the nodes it removes and adds, by name:
 * the path an allocation takes, the nodes a removal drops, a mastery's
 * effect. */
function treeBlocks(
  r: ChangeRequest,
  layout: TreeLayout | null,
): { remove: string[]; add: string[] } {
  const name = (id: number) =>
    layout?.nodes.find((n) => n.id === id)?.name ?? t('ai.nodeN', { n: id })
  const preview = r['tree-preview']
  const change = r.tree
  if (change == null) return { remove: [], add: [] }
  if ('allocate-node' in change) {
    const path = preview?.path ?? [change['allocate-node'].node]
    return { remove: [], add: path.map(name) }
  }
  if ('deallocate-node' in change) {
    const node = change['deallocate-node'].node
    const removed = preview?.removed ?? []
    return {
      remove: [node, ...removed.filter((n) => n !== node)].map(name),
      add: [],
    }
  }
  const { node, effect } = change['set-mastery-effect']
  const group = layout?.nodes.find((n) => n.id === node)?.['mastery-group']
  const lines =
    layout?.masteries
      .find((g) => g.id === group)
      ?.effects.find((e) => e.effect === effect)?.lines ?? []
  return {
    remove: [],
    add: [
      t('ai.masteryEffect', {
        node: name(node),
        effect: lines.length > 0 ? lines.join(' · ') : `#${effect}`,
      }),
    ],
  }
}

function effectChip(f: FieldDelta): { tone: Tone; text: string } {
  const show = (v: number | null | undefined) =>
    v == null ? '—' : withUnit(f.field, v)
  const tone: Tone =
    f.before != null && f.after != null
      ? f.after > f.before
        ? 'ok'
        : 'warn'
      : 'neutral'
  return {
    tone,
    text: `${fieldLabel(f.field)} ${show(f.before)} → ${show(f.after)}`,
  }
}

const STATUS_TONE: Record<ChangeRequest['status'], Tone> = {
  pending: 'info',
  applied: 'ok',
  declined: 'neutral',
  failed: 'danger',
}

function Request({ r, items }: { r: ChangeRequest; items: ItemSummary[] }) {
  const dispatch = useAppDispatch()
  const layout = useTreeLayout()
  const item = items.find((i) => i.id === r.edit?.item)
  const name =
    r.tree != null
      ? t('ai.passiveTree')
      : item === undefined
        ? `#${r.edit?.item ?? 0}`
        : itemName(item)
  const { remove, add } =
    r.tree != null
      ? treeBlocks(r, typeof layout === 'object' ? layout : null)
      : changeBlocks(r)
  const effects = (r.preview?.fields ?? r['tree-preview']?.fields ?? []).filter(
    (f) => f.before !== f.after,
  )
  const headingId = `request-${r.id}`
  return (
    <article
      className={`panel request${r.status === 'pending' ? ' pending' : ''}`}
      aria-labelledby={headingId}
    >
      <section>
        <div className="row">
          <Badge tone="info">{t('ai.request')}</Badge>
          <Badge tone={STATUS_TONE[r.status]}>
            {t(`ai.status.${r.status}` as MessageKey)}
          </Badge>
        </div>
        <h3 id={headingId}>
          {t('ai.wants', { client: r.client, item: name })}
        </h3>
        {r.note != null && r.note !== '' && <p className="note">“{r.note}”</p>}
      </section>
      <section className="stack">
        <div className="blocks">
          {remove.length > 0 && (
            <div className="block remove">
              <p className="title">{t('ai.removeFrom', { item: name })}</p>
              <ul>
                {remove.map((l, i) => (
                  <li key={i}>{l}</li>
                ))}
              </ul>
            </div>
          )}
          {add.length > 0 && (
            <div className="block add">
              <p className="title">{t('ai.addTo', { item: name })}</p>
              <ul>
                {add.map((l, i) => (
                  <li key={i}>{l}</li>
                ))}
              </ul>
            </div>
          )}
        </div>
        {(r.preview != null || r['tree-preview'] != null) && (
          <div className="stack tight">
            <p className="muted small-text">{t('ai.calculatedBy')}</p>
            <ul className="chips" aria-label={t('ai.effect')}>
              {effects.length === 0 ? (
                <li>
                  <Badge tone="neutral">{t('ai.noEffect')}</Badge>
                </li>
              ) : (
                effects.map((f) => {
                  const chip = effectChip(f)
                  return (
                    <li key={f.field}>
                      <Badge tone={chip.tone}>{chip.text}</Badge>
                    </li>
                  )
                })
              )}
            </ul>
          </div>
        )}
        {r.error != null && (
          <>
            <p className="error">{errorText(r.error.code)}</p>
            <ErrorCode code={r.error.code} />
          </>
        )}
      </section>
      {r.status === 'pending' && (
        <section className="stack">
          <div className="row">
            <button
              type="button"
              className="primary"
              aria-label={t('ai.approveRequest', { id: r.id })}
              onClick={() => void dispatch(run({ approve: { id: r.id } }))}
            >
              {t('ai.approve')}
            </button>
            <button
              type="button"
              aria-label={t('ai.declineRequest', { id: r.id })}
              onClick={() => void dispatch(run({ decline: { id: r.id } }))}
            >
              {t('ai.decline')}
            </button>
          </div>
          <p className="muted small-text">{t('ai.undoable')}</p>
        </section>
      )}
    </article>
  )
}

export function AiConnection() {
  const dispatch = useAppDispatch()
  const mcp = useAppSelector((s) => s.lists.mcp)
  const items = useAppSelector((s) => s.session.snapshot?.build?.items ?? [])
  const [showToken, setShowToken] = useState(false)
  const [copied, setCopied] = useState(false)
  const [rotated, setRotated] = useState(false)
  if (mcp === null) return null
  const p = mcp.permissions
  // `--scope user`: Claude Code keys a project's servers by its folder's
  // spelling (D:/ and d:/ differ), so a server added for one may not show
  // for the other; a user server shows everywhere.
  const command = (token: string) =>
    mcp.url == null
      ? ''
      : `claude mcp add --scope user --transport http exiled-studio ${mcp.url} --header "Authorization: Bearer ${token}"`
  return (
    <>
      <div className="page-head">
        <ScreenHeading>{t('ai.title')}</ScreenHeading>
        <Hint>{t('ai.help')}</Hint>
      </div>
      <div className="ai-grid">
        <section className="panel client" aria-labelledby="ai-server">
          <div className="panel-head">
            <h2 id="ai-server">{t('ai.server')}</h2>
            <Badge tone={mcp.running ? 'ok' : 'neutral'}>
              {mcp.running ? t('ai.on') : t('ai.off')}
            </Badge>
          </div>
          <p className="muted small-text">
            {mcp.running
              ? t('ai.running', { url: mcp.url ?? '' })
              : t('ai.stopped')}
          </p>

          <fieldset className="perms">
            <legend className="visually-hidden">{t('ai.permissions')}</legend>
            <div className="perm">
              <span className="text">
                <span>{t('ai.read')}</span>
                <span className="sub">{t('ai.readHelp')}</span>
              </span>
              <Badge tone="ok">{t('ai.allowed')}</Badge>
            </div>
            <div className="perm">
              <span className="check">
                <input
                  type="checkbox"
                  id="ai-requests"
                  aria-describedby="ai-requests-help"
                  checked={p.requests}
                  onChange={(e) =>
                    void dispatch(
                      setPermissions({
                        requests: e.target.checked,
                        'standing-grant':
                          e.target.checked && p['standing-grant'],
                      }),
                    )
                  }
                />
                <span className="text">
                  <label htmlFor="ai-requests">{t('ai.requests')}</label>
                  <span className="sub" id="ai-requests-help">
                    {t('ai.requestsHelp')}
                  </span>
                </span>
              </span>
              <Badge tone={p.requests ? 'ok' : 'neutral'}>
                {p.requests ? t('ai.allowed') : t('ai.notAllowed')}
              </Badge>
            </div>
            <div className="perm">
              <span className="check">
                <input
                  type="checkbox"
                  id="ai-grant"
                  aria-describedby="ai-grant-help"
                  checked={p['standing-grant']}
                  disabled={!p.requests}
                  onChange={(e) =>
                    void dispatch(
                      setPermissions({
                        requests: p.requests,
                        'standing-grant': e.target.checked,
                      }),
                    )
                  }
                />
                <span className="text">
                  <label htmlFor="ai-grant">{t('ai.grant')}</label>
                  <span className="sub" id="ai-grant-help">
                    {t('ai.grantHelp')}
                  </span>
                </span>
              </span>
              <Badge tone={p['standing-grant'] ? 'warn' : 'neutral'}>
                {p['standing-grant'] ? t('ai.withoutAsking') : t('ai.askEach')}
              </Badge>
            </div>
          </fieldset>

          {mcp.running ? (
            <div className="stack tight top-gap">
              <h3>{t('ai.connect')}</h3>
              <p className="muted small-text">
                {t('ai.connectHelp')}{' '}
                {mcp['token-kept'] ? t('ai.tokenKept') : t('ai.tokenSession')}
              </p>
              <code className="command">
                {command(showToken ? (mcp.token ?? '') : '<token>')}
              </code>
              <div className="row">
                <button
                  type="button"
                  className="primary"
                  onClick={() => {
                    void navigator.clipboard
                      ?.writeText(command(mcp.token ?? ''))
                      .then(() => setCopied(true))
                  }}
                >
                  <Icon name="copy" />
                  {copied ? t('ai.copied') : t('ai.copyCommand')}
                </button>
                <button
                  type="button"
                  aria-pressed={showToken}
                  onClick={() => setShowToken(!showToken)}
                >
                  {showToken ? t('ai.hideToken') : t('ai.showToken')}
                </button>
                <button
                  type="button"
                  onClick={() =>
                    void dispatch(attempt((api) => api.mcpRotate())).then(
                      (s) => {
                        if (!s) return
                        dispatch(actions.mcp(s))
                        setCopied(false)
                        setRotated(true)
                      },
                    )
                  }
                >
                  {t('ai.rotate')}
                </button>
              </div>
              {rotated && (
                <p className="small-text" role="status">
                  {t('ai.rotated')}
                </p>
              )}
              <div>
                <button
                  type="button"
                  onClick={() =>
                    void dispatch(attempt((api) => api.mcpStop())).then(
                      (s) => s && dispatch(actions.mcp(s)),
                    )
                  }
                >
                  {t('ai.stop')}
                </button>
              </div>
            </div>
          ) : (
            <div className="top-gap">
              <button
                type="button"
                className="primary"
                onClick={() =>
                  void dispatch(attempt((api) => api.mcpStart())).then(
                    (s) => s && dispatch(actions.mcp(s)),
                  )
                }
              >
                {t('ai.start')}
              </button>
            </div>
          )}
        </section>

        <section className="requests" aria-labelledby="ai-requests-h">
          <h2 id="ai-requests-h" className="visually-hidden">
            {t('ai.requestsTitle')}
          </h2>
          {mcp.requests.length === 0 ? (
            <p className="panel muted">{t('ai.requestsNone')}</p>
          ) : (
            [...mcp.requests]
              .reverse()
              .map((r) => <Request key={r.id} r={r} items={items} />)
          )}
        </section>
      </div>
    </>
  )
}

/** The fonts' license texts, bundled with the UI (public/licenses). */
const LICENSES = [
  { name: 'Inter', file: 'inter-OFL-1.1.txt' },
  { name: 'Space Grotesk', file: 'space-grotesk-OFL-1.1.txt' },
  { name: 'JetBrains Mono', file: 'jetbrains-mono-OFL-1.1.txt' },
]

/** A license text, read from the app's own files when opened. */
function LicenseText({ name, file }: { name: string; file: string }) {
  const [text, setText] = useState<string | null>(null)
  return (
    <details
      className="license"
      onToggle={(e) => {
        if (!e.currentTarget.open || text !== null) return
        fetch(`licenses/${file}`)
          .then((r) => (r.ok ? r.text() : Promise.reject(new Error(file))))
          .then(setText, () => setText(t('about.licenseMissing', { file })))
      }}
    >
      <summary>{t('about.licenseOf', { name })}</summary>
      <pre>{text ?? t('about.loading')}</pre>
    </details>
  )
}

/** The installed game data, with the one in use marked and the others
 * offered (B-1); a folder for development, labelled unsigned. */
/** The diagnostic package (Phase 5, P-f): previewed file by file, then
 * saved where the user picks; nothing is sent anywhere. */
function Diagnostics() {
  const dispatch = useAppDispatch()
  const [files, setFiles] = useState<DiagnosticFile[] | null>(null)
  const [saved, setSaved] = useState(false)
  const preview = () => {
    setSaved(false)
    void dispatch(query((api) => api.diagnostics())).then((f) => {
      setFiles(f ?? [])
    })
  }
  const save = () => {
    void dispatch(attempt((api) => api.exportDiagnostics())).then((o) => {
      if (o !== null && o !== 'nothing') setSaved(true)
    })
  }
  return (
    <section className="panel stack" aria-labelledby="diag-h">
      <h2 id="diag-h">{t('diag.title')}</h2>
      <p>{t('diag.intro')}</p>
      <div className="row">
        <button type="button" onClick={preview}>
          {t('diag.preview')}
        </button>
        {files !== null && (
          <button type="button" className="primary" onClick={save}>
            {t('diag.save')}
          </button>
        )}
      </div>
      {saved && (
        <p role="status" className="muted">
          {t('diag.saved')}
        </p>
      )}
      {files !== null && (
        <ul className="diag-files" aria-label={t('diag.files')}>
          {files.map((f) => (
            <li key={f.name}>
              <DiagnosticFileView file={f} />
            </li>
          ))}
        </ul>
      )}
    </section>
  )
}

/** One file of the package; its text is rendered only while open, so a
 * package of several megabytes stays light. */
function DiagnosticFileView({ file }: { file: DiagnosticFile }) {
  const [open, setOpen] = useState(false)
  return (
    <details
      onToggle={(e) => {
        setOpen(e.currentTarget.open)
      }}
    >
      <summary>
        <span className="mono">{file.name}</span>{' '}
        <span className="muted small-text">
          {t('diag.size', { n: file.bytes })}
        </span>
      </summary>
      {open && <pre className="diag-text">{file.text}</pre>}
    </details>
  )
}

function GameData() {
  const dispatch = useAppDispatch()
  const data = useAppSelector((s) => s.session.snapshot?.data)
  const [status, setStatus] = useState<string | null>(null)
  const installed = data?.installed ?? []
  const quarantined = data?.quarantined ?? []
  const offline = useAppSelector(
    (s) => s.lists.updates?.mode === 'strict-offline',
  )
  const [feed, setFeed] = useState<DataFeedStatus | null>(null)
  const checkFeed = () => {
    setStatus(null)
    void dispatch(attempt((api) => api.checkDataFeed())).then((f) => {
      if (f !== null) setFeed(f)
    })
  }
  const download = (id: string) => {
    setStatus(null)
    void dispatch(attempt((api) => api.downloadData(id))).then((o) => {
      if (o === null) return
      setStatus(t('data.installed'))
      setFeed((f) =>
        f === null
          ? f
          : {
              ...f,
              offers: f.offers.map((x) =>
                x.id === id ? { ...x, installed: true } : x,
              ),
            },
      )
    })
  }
  const install = () => {
    setStatus(null)
    void dispatch(attempt((api) => api.installBundle())).then((o) => {
      if (o !== null && o !== 'nothing') setStatus(t('data.installed'))
    })
  }
  const verify = (id: string) => {
    setStatus(null)
    void dispatch(run({ 'verify-bundle': { id } })).then((o) => {
      if (o !== null) setStatus(t('data.verified', { id }))
    })
  }
  return (
    <section className="stack" aria-labelledby="about-data">
      <h2 id="about-data">{t('data.heading')}</h2>
      {installed.length === 0 ? (
        <p className="muted">{t('data.noneInstalled')}</p>
      ) : (
        <ul className="data-list">
          {installed.map((b) => (
            <li key={b.id}>
              <span>
                {t('data.entry', {
                  name: dataName(b['data-version'], b['tree-version']),
                  build: b['data-version']['steam-build'] ?? '?',
                  schema: b['data-version']['schema-version'],
                })}
              </span>
              {b['signed-by'] != null ? (
                <Badge tone="ok">{t('data.signed')}</Badge>
              ) : b.accepted === true ? (
                <Badge tone="warn">{t('data.accepted')}</Badge>
              ) : null}
              {b.id === data?.active ? (
                <Badge tone="ok">{t('data.inUse')}</Badge>
              ) : (
                <button
                  type="button"
                  onClick={() =>
                    void dispatch(run({ 'use-bundle': { id: b.id } }))
                  }
                >
                  {t('data.use')}
                </button>
              )}
              <button
                type="button"
                aria-label={t('data.verifyLabel', { id: b.id })}
                onClick={() => {
                  verify(b.id)
                }}
              >
                {t('data.verify')}
              </button>
            </li>
          ))}
        </ul>
      )}
      {quarantined.length > 0 && (
        <div
          className="panel stack"
          role="region"
          aria-labelledby="quarantine-h"
        >
          <h3 id="quarantine-h">{t('data.quarantine.heading')}</h3>
          <p className="muted">{t('data.quarantine.help')}</p>
          <ul className="data-list">
            {quarantined.map((q) => (
              <li key={q.id} className="stack">
                <span>
                  {t('data.entry', {
                    name: dataName(q['data-version'], null),
                    build: q['data-version']['steam-build'] ?? '?',
                    schema: q['data-version']['schema-version'],
                  })}
                </span>
                <span>
                  {tDynamic(`data.quarantine.${q.reason}`, 'error.generic', {
                    key: q['key-id'] ?? '?',
                  })}
                </span>
                <span className="muted small-text">
                  {t('data.quarantine.size', {
                    files: q.files,
                    bytes: q.bytes,
                  })}
                </span>
                {q.acceptable && (
                  <>
                    <span className="mono selectable">
                      {t('data.quarantine.hash', {
                        hash: q['manifest-sha256'],
                      })}
                    </span>
                    <button
                      type="button"
                      aria-label={t('data.acceptLabel', { id: q.id })}
                      onClick={() =>
                        void dispatch(
                          run({
                            'accept-bundle': {
                              id: q.id,
                              'manifest-sha256': q['manifest-sha256'],
                            },
                          }),
                        )
                      }
                    >
                      {t('data.accept')}
                    </button>
                  </>
                )}
              </li>
            ))}
          </ul>
        </div>
      )}
      {data?.['data-version'] != null && data.active == null && (
        <p className="muted">{t('data.unsignedInUse')}</p>
      )}
      {status !== null && (
        <p role="status" className="muted">
          {status}
        </p>
      )}
      {feed !== null &&
        (feed.offers.length === 0 ? (
          <p className="muted">{t('data.feed.none')}</p>
        ) : (
          <ul className="data-list" aria-label={t('data.feed.list')}>
            {feed.offers.map((o) => (
              <li key={o.id}>
                <span>
                  {t('data.feed.offer', { id: o.id, bytes: o.bytes })}
                </span>
                {o.installed ? (
                  <Badge tone="ok">{t('data.feed.installed')}</Badge>
                ) : (
                  <button
                    type="button"
                    aria-label={t('data.feed.downloadLabel', { id: o.id })}
                    onClick={() => {
                      download(o.id)
                    }}
                  >
                    {t('data.feed.download')}
                  </button>
                )}
              </li>
            ))}
          </ul>
        ))}
      {offline && <p className="muted">{t('data.feed.offline')}</p>}
      <div className="row">
        {!offline && (
          <button type="button" onClick={checkFeed}>
            {t('data.feed.check')}
          </button>
        )}
        <button type="button" onClick={install}>
          {t('data.install')}
        </button>
        <button
          type="button"
          className="link"
          onClick={() => void dispatch(attempt((api) => api.chooseBundle()))}
        >
          {t('data.chooseFolder')}
        </button>
      </div>
    </section>
  )
}

export function About() {
  const version = useAppSelector(
    (s) => s.session.snapshot?.data['data-version'],
  )
  const tree = useAppSelector((s) => s.session.snapshot?.data['tree-version'])
  return (
    <section className="stack about">
      <ScreenHeading>{t('about.title')}</ScreenHeading>
      <p>{t('app.notice')}</p>
      <dl className="about-versions">
        <dt>{t('about.version')}</dt>
        <dd className="mono">{__APP_VERSION__}</dd>
        <dt>{t('about.data')}</dt>
        <dd className="mono">
          {version == null
            ? t('about.dataNone')
            : `${dataName(version, tree)} · ${t('data.steamBuild', { build: version['steam-build'] ?? '?' })}`}
        </dd>
      </dl>
      <GameData />
      <UpdateSettings />
      <Diagnostics />
      <p>{t('about.license')}</p>
      <p>{t('about.privacy')}</p>
      <p className="muted">{t('about.fonts')}</p>
      {LICENSES.map((l) => (
        <LicenseText key={l.file} name={l.name} file={l.file} />
      ))}
    </section>
  )
}
