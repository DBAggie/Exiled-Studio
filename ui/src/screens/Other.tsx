// Warnings, AI connection (screen 6, docs/ui), and About.
import { useState } from 'react'

import { Badge, ScreenHeading } from '../components'
import { t, tDynamic, type MessageKey } from '../i18n'
import { actions, attempt, run, setPermissions } from '../store'
import { useAppDispatch, useAppSelector } from '../util'

export function Warnings() {
  const dispatch = useAppDispatch()
  const warnings = useAppSelector(
    (s) => s.session.snapshot?.build?.warnings ?? [],
  )
  return (
    <section>
      <ScreenHeading>{t('warnings.title')}</ScreenHeading>
      <p className="muted">{t('warnings.help')}</p>
      {warnings.length === 0 ? (
        <p>{t('warnings.none')}</p>
      ) : (
        <table>
          <thead>
            <tr>
              <th scope="col">{t('warnings.item')}</th>
              <th scope="col">{t('warnings.text')}</th>
              <th scope="col">{t('warnings.reason')}</th>
            </tr>
          </thead>
          <tbody>
            {warnings.map((w) => (
              <tr key={`${w.item}-${w.line}`}>
                <td>
                  <button
                    type="button"
                    onClick={() => dispatch(actions.editItem(w.item))}
                  >
                    {w['item-name'] ?? `#${w.item}`}
                  </button>
                </td>
                <td className="mono">{w.text}</td>
                <td>
                  {tDynamic(`warning.${w.code}`, 'reason.generic', {
                    kind: w.code,
                  })}{' '}
                  <span className="muted">({w.code})</span>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </section>
  )
}

export function AiConnection() {
  const dispatch = useAppDispatch()
  const mcp = useAppSelector((s) => s.lists.mcp)
  const [showToken, setShowToken] = useState(false)
  if (mcp === null) return null
  const p = mcp.permissions
  const command =
    mcp.url === null || mcp.url === undefined
      ? ''
      : `claude mcp add --transport http exiled-studio ${mcp.url} --header "Authorization: Bearer ${showToken ? (mcp.token ?? '') : '<token>'}"`
  return (
    <section>
      <ScreenHeading>{t('ai.title')}</ScreenHeading>
      <p className="muted">{t('ai.help')}</p>
      <section className="panel" aria-labelledby="ai-server">
        <h2 id="ai-server">
          {mcp.running
            ? t('ai.running', { url: mcp.url ?? '' })
            : t('ai.stopped')}
        </h2>
        {mcp.running ? (
          <>
            <p>
              {t('ai.token')}: <code>{showToken ? mcp.token : '••••••••'}</code>{' '}
              <button
                type="button"
                aria-pressed={showToken}
                onClick={() => setShowToken(!showToken)}
              >
                {showToken ? t('ai.hideToken') : t('ai.showToken')}
              </button>
            </p>
            <p>
              {t('ai.command')}: <code>{command}</code>
            </p>
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
          </>
        ) : (
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
        )}
      </section>
      <fieldset className="panel">
        <legend>{t('ai.permissions')}</legend>
        <p>
          <input type="checkbox" id="ai-read" checked disabled />{' '}
          <label htmlFor="ai-read">{t('ai.read')}</label>
        </p>
        <p>
          <input
            type="checkbox"
            id="ai-requests"
            checked={p.requests}
            onChange={(e) =>
              void dispatch(
                setPermissions({
                  requests: e.target.checked,
                  'standing-grant': e.target.checked && p['standing-grant'],
                }),
              )
            }
          />{' '}
          <label htmlFor="ai-requests">{t('ai.requests')}</label>
        </p>
        <p>
          <input
            type="checkbox"
            id="ai-grant"
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
          />{' '}
          <label htmlFor="ai-grant">{t('ai.grant')}</label>
        </p>
      </fieldset>
      <section className="panel" aria-labelledby="ai-requests-h">
        <h2 id="ai-requests-h">{t('ai.requestsTitle')}</h2>
        {mcp.requests.length === 0 ? (
          <p className="muted">{t('ai.requestsNone')}</p>
        ) : (
          <ul>
            {[...mcp.requests].reverse().map((r) => (
              <li key={r.id} className="panel">
                <p>
                  <strong>#{r.id}</strong> {t('ai.from', { client: r.client })}{' '}
                  <Badge
                    tone={
                      r.status === 'pending'
                        ? 'info'
                        : r.status === 'applied'
                          ? 'ok'
                          : 'neutral'
                    }
                  >
                    {t(`ai.status.${r.status}` as MessageKey)}
                  </Badge>
                </p>
                {r.note !== null && r.note !== undefined && <p>{r.note}</p>}
                <ul className="mono">
                  {r.edit.ops.map((op, i) => (
                    <li key={i}>{JSON.stringify(op)}</li>
                  ))}
                </ul>
                {r.preview?.fields
                  .filter((f) => f.before !== f.after)
                  .map((f) => (
                    <p key={f.field} className="mono">
                      {f.field}: {f.before ?? '—'} → {f.after ?? '—'}
                    </p>
                  ))}
                {r.error !== null && r.error !== undefined && (
                  <p className="error">{r.error.code}</p>
                )}
                {r.status === 'pending' && (
                  <div className="row">
                    <button
                      type="button"
                      className="primary"
                      onClick={() =>
                        void dispatch(run({ approve: { id: r.id } }))
                      }
                    >
                      {t('ai.approve', { id: r.id })}
                    </button>
                    <button
                      type="button"
                      onClick={() =>
                        void dispatch(run({ decline: { id: r.id } }))
                      }
                    >
                      {t('ai.decline', { id: r.id })}
                    </button>
                  </div>
                )}
              </li>
            ))}
          </ul>
        )}
      </section>
    </section>
  )
}

export function About() {
  return (
    <section>
      <ScreenHeading>{t('about.title')}</ScreenHeading>
      <p>{t('app.notice')}</p>
      <p>{t('about.license')}</p>
      <p>{t('about.privacy')}</p>
    </section>
  )
}
