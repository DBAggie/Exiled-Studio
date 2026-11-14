// Screen 1 (docs/ui, Main): import a PoB build, open a saved one, the game
// data, unsaved work to restore, recent builds.
import { useId, useState } from 'react'

import { ErrorCode, Icon, ScreenHeading } from '../components'
import { errorText, t } from '../i18n'
import {
  actions,
  attempt,
  importBuild,
  openBuild,
  refreshLists,
  run,
} from '../store'
import { useAppDispatch, useAppSelector } from '../util'

function when(seconds: number): string {
  return new Date(seconds * 1000).toLocaleString(undefined, {
    dateStyle: 'medium',
    timeStyle: 'short',
  })
}

const relative = new Intl.RelativeTimeFormat(undefined, { numeric: 'auto' })

/** "2 hours ago", "yesterday" (UX-20). */
function ago(seconds: number): string {
  const diff = seconds - Date.now() / 1000
  const units: [Intl.RelativeTimeFormatUnit, number][] = [
    ['year', 31_536_000],
    ['month', 2_592_000],
    ['week', 604_800],
    ['day', 86_400],
    ['hour', 3_600],
    ['minute', 60],
  ]
  for (const [unit, size] of units) {
    if (Math.abs(diff) >= size)
      return relative.format(Math.round(diff / size), unit)
  }
  return t('start.justNow')
}

/** A path's last part; the full path is shown on hover. */
function fileName(path: string): string {
  return path.split(/[\\/]/).pop() ?? path
}

export function Start() {
  const dispatch = useAppDispatch()
  const snapshot = useAppSelector((s) => s.session.snapshot)
  const flow = useAppSelector((s) => s.importFlow)
  const recent = useAppSelector((s) => s.lists.recent)
  const recovery = useAppSelector((s) => s.lists.recovery)
  const [text, setText] = useState('')
  const textId = useId()
  const data = snapshot?.data
  const version = data?.['data-version']
  const reading = flow.state === 'reading'

  return (
    <>
      <div>
        <ScreenHeading>{t('start.title')}</ScreenHeading>
        <p className="lead">{t('start.lead')}</p>
      </div>

      {/* Unsaved work first: it is what the user most likely came back
          for (UX-19). */}
      {recovery.length > 0 && (
        <section className="panel recovery" aria-labelledby="recovery-h">
          <div className="panel-head">
            <h2 id="recovery-h">{t('start.recovery')}</h2>
          </div>
          <ul className="recovery-list">
            {recovery.map((r) => {
              const name = r.name === '' ? t('app.untitled') : r.name
              const file = r.path ?? r.origin
              return (
                <li key={r.id}>
                  <div>
                    <strong>{name}</strong>
                    <span className="sub">
                      {t('start.recoveryWhen', { when: ago(r['saved-at']) })}
                      {file != null && (
                        <>
                          {' · '}
                          <span className="mono" title={file}>
                            {fileName(file)}
                          </span>
                        </>
                      )}
                    </span>
                  </div>
                  <div className="row end">
                    <button
                      type="button"
                      className="primary"
                      aria-label={t('start.restore', { name })}
                      onClick={() =>
                        void dispatch(
                          openBuild((api) =>
                            api.execute({ restore: { id: r.id } }),
                          ),
                        )
                      }
                    >
                      {t('start.restoreShort')}
                    </button>
                    <button
                      type="button"
                      aria-label={t('start.discard', { name })}
                      onClick={() =>
                        void dispatch(run({ discard: { id: r.id } })).then(() =>
                          dispatch(refreshLists()),
                        )
                      }
                    >
                      {t('start.discardShort')}
                    </button>
                  </div>
                </li>
              )
            })}
          </ul>
        </section>
      )}
      <div className="start-grid">
        <section className="panel import-card" aria-labelledby={`${textId}-h`}>
          <h2 id={`${textId}-h`}>
            <Icon name="download" size={22} />
            {t('start.import')}
          </h2>
          <form
            onSubmit={(e) => {
              e.preventDefault()
              // A pobb.in or pastebin link: say so before trying (UX-18).
              if (/^\s*https?:\/\//i.test(text)) {
                dispatch(actions.importStarted())
                dispatch(actions.importFailed('UI_LINK'))
                return
              }
              void dispatch(
                importBuild((api) => api.execute({ 'import-text': { text } })),
              )
            }}
          >
            <label htmlFor={textId} className="muted">
              {t('start.importLabel')}
            </label>
            <textarea
              id={textId}
              rows={4}
              value={text}
              onChange={(e) => setText(e.target.value)}
            />
            <div className="row">
              <button
                type="submit"
                className="primary"
                disabled={reading || text.trim() === ''}
              >
                {t('start.importButton')}
              </button>
              <button
                type="button"
                disabled={reading}
                onClick={() =>
                  void dispatch(importBuild((api) => api.importFile()))
                }
              >
                {t('start.importFile')}
              </button>
              <span className="muted">{t('start.importSafe')}</span>
            </div>
          </form>
          {reading && <p role="status">{t('import.reading')}</p>}
          {flow.state === 'failed' && (
            <div role="alert">
              <p className="error">{errorText(flow.code)}</p>
              {flow.code !== 'UI_LINK' && <ErrorCode code={flow.code} />}
            </div>
          )}
        </section>

        <div className="start-side">
          <section className="panel side-card" aria-labelledby="open-h">
            <h2 id="open-h">{t('start.openTitle')}</h2>
            <p className="muted">{t('start.openHelp')}</p>
            <button
              type="button"
              onClick={() => void dispatch(openBuild((api) => api.openFile()))}
            >
              {t('start.open')}
            </button>
          </section>

          <section className="panel side-card" aria-labelledby="data-h">
            <h2 id="data-h">{t('start.data')}</h2>
            {version === null || version === undefined ? (
              <p className="muted">{t('start.dataNone')}</p>
            ) : (
              <p>
                {t('start.dataLoaded', {
                  game:
                    version.game === 'poe1'
                      ? 'Path of Exile'
                      : 'Path of Exile 2',
                  build: version['steam-build'] ?? '?',
                })}
              </p>
            )}
            {data?.error !== null && data?.error !== undefined && (
              <>
                <p className="error">{errorText(data.error.code)}</p>
                <ErrorCode code={data.error.code} />
              </>
            )}
            <button
              type="button"
              onClick={() =>
                void dispatch(attempt((api) => api.chooseBundle()))
              }
            >
              <Icon name="folder" />
              {t('start.chooseData')}
            </button>
          </section>
        </div>
      </div>

      <section aria-labelledby="recent-h">
        <div className="recent-head">
          <h2 id="recent-h">{t('start.recent')}</h2>
        </div>
        {recent.length === 0 ? (
          <p className="muted">{t('start.recentNone')}</p>
        ) : (
          <div className="panel table-panel">
            <table>
              <thead>
                <tr>
                  <th scope="col">{t('start.colBuild')}</th>
                  <th scope="col">{t('start.colOpened')}</th>
                  <th scope="col">
                    <span className="visually-hidden">
                      {t('start.colActions')}
                    </span>
                  </th>
                </tr>
              </thead>
              <tbody>
                {recent.map((r) => (
                  <tr key={r.id}>
                    <th scope="row">
                      <button
                        type="button"
                        className="recent-name"
                        onClick={() =>
                          void dispatch(
                            openBuild((api) =>
                              api.execute({ 'open-recent': { id: r.id } }),
                            ),
                          )
                        }
                      >
                        {r.name}
                      </button>
                      <span className="sub mono" title={r.path}>
                        {fileName(r.path)}
                      </span>
                    </th>
                    <td title={when(r.at)}>{ago(r.at)}</td>
                    <td className="right">
                      <button
                        type="button"
                        className="link"
                        aria-label={t('start.forget', { name: r.name })}
                        onClick={() =>
                          void dispatch(
                            run({ 'forget-recent': { id: r.id } }),
                          ).then(() => dispatch(refreshLists()))
                        }
                      >
                        {t('start.forgetShort')}
                      </button>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </section>
    </>
  )
}
