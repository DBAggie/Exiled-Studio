// Warnings, AI connection (screen 6, docs/ui AiApproval), and About.
import { useState } from 'react'

import type {
  ChangeRequest,
  FieldDelta,
  ItemSummary,
  LineOp,
} from '../api/types'
import { Badge, Icon, ScreenHeading, type Tone } from '../components'
import { fieldLabel, formatNumber, t, tDynamic, type MessageKey } from '../i18n'
import { actions, attempt, run, setPermissions } from '../store'
import { itemName, useAppDispatch, useAppSelector, withUnit } from '../util'

export function Warnings() {
  const dispatch = useAppDispatch()
  const warnings = useAppSelector(
    (s) => s.session.snapshot?.build?.warnings ?? [],
  )
  return (
    <>
      <ScreenHeading>{t('warnings.title')}</ScreenHeading>
      <p className="lead">{t('warnings.help')}</p>
      {warnings.length === 0 ? (
        <p>{t('warnings.none')}</p>
      ) : (
        <section className="panel table-panel" aria-label={t('warnings.title')}>
          <table>
            <thead>
              <tr>
                <th scope="col">{t('warnings.text')}</th>
                <th scope="col">{t('warnings.item')}</th>
                <th scope="col">{t('warnings.reason')}</th>
              </tr>
            </thead>
            <tbody>
              {warnings.map((w) => (
                <tr key={`${w.item}-${w.line}`}>
                  <td className="mono">{w.text}</td>
                  <td>
                    <button
                      type="button"
                      className="link"
                      onClick={() => dispatch(actions.editItem(w.item))}
                    >
                      {w['item-name'] ?? `#${w.item}`}
                    </button>
                  </td>
                  <td>
                    <Badge tone="warn">
                      <span className="mono">{w.code}</span>
                    </Badge>
                    <p className="muted small-text">
                      {tDynamic(`warning.${w.code}`, 'reason.generic', {
                        kind: w.code,
                      })}
                    </p>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </section>
      )}
    </>
  )
}

/** A request's edit as what it removes and what it adds, in words: the
 * lines the AI saw (`expect`) and the text it asks for. */
function changeBlocks(r: ChangeRequest): { remove: string[]; add: string[] } {
  const seen = (line: number) =>
    r.edit.expect.find((e) => e.line === line)?.text ??
    t('ai.lineN', { n: line + 1 })
  const remove: string[] = []
  const add: string[] = []
  for (const op of r.edit.ops as LineOp[]) {
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
  const item = items.find((i) => i.id === r.edit.item)
  const name = item === undefined ? `#${r.edit.item}` : itemName(item)
  const { remove, add } = changeBlocks(r)
  const effects = (r.preview?.fields ?? []).filter((f) => f.before !== f.after)
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
        {r.preview != null && (
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
        {r.error != null && <p className="error">{r.error.code}</p>}
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
      <ScreenHeading>{t('ai.title')}</ScreenHeading>
      <p className="lead">{t('ai.help')}</p>
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
              <p className="muted small-text">{t('ai.connectHelp')}</p>
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
              </div>
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

export function About() {
  return (
    <section className="stack about">
      <ScreenHeading>{t('about.title')}</ScreenHeading>
      <p>{t('app.notice')}</p>
      <p>{t('about.license')}</p>
      <p>{t('about.privacy')}</p>
      <p className="muted">{t('about.fonts')}</p>
    </section>
  )
}
