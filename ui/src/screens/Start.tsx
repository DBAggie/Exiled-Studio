// Screen 1 (docs/ui, Main): import a PoB build, open a saved one, the game
// data, unsaved work to restore, recent builds.
import { useId, useState } from 'react'

import { Icon, ScreenHeading } from '../components'
import { errorText, t } from '../i18n'
import { attempt, importBuild, openBuild, run } from '../store'
import { useAppDispatch, useAppSelector } from '../util'

function when(seconds: number): string {
  return new Date(seconds * 1000).toLocaleString()
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

      <div className="start-grid">
        <section className="panel import-card" aria-labelledby={`${textId}-h`}>
          <h2 id={`${textId}-h`}>
            <Icon name="download" size={22} />
            {t('start.import')}
          </h2>
          <form
            onSubmit={(e) => {
              e.preventDefault()
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
            <p role="alert" className="error">
              {errorText(flow.code)} <code>{flow.code}</code>
            </p>
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
              <p className="error">{errorText(data.error.code)}</p>
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

      {recovery.length > 0 && (
        <section className="panel table-panel" aria-labelledby="recovery-h">
          <div className="panel-head">
            <h2 id="recovery-h">{t('start.recovery')}</h2>
          </div>
          <table>
            <thead>
              <tr>
                <th scope="col">{t('start.colBuild')}</th>
                <th scope="col">{t('start.colSaved')}</th>
                <th scope="col">
                  <span className="visually-hidden">
                    {t('start.colActions')}
                  </span>
                </th>
              </tr>
            </thead>
            <tbody>
              {recovery.map((r) => {
                const name = r.name === '' ? t('app.untitled') : r.name
                return (
                  <tr key={r.id}>
                    <th scope="row">{name}</th>
                    <td className="muted">{when(r['saved-at'])}</td>
                    <td className="right">
                      <div className="row end">
                        <button
                          type="button"
                          className="primary"
                          onClick={() =>
                            void dispatch(
                              openBuild((api) =>
                                api.execute({ restore: { id: r.id } }),
                              ),
                            )
                          }
                        >
                          {t('start.restore', { name })}
                        </button>
                        <button
                          type="button"
                          onClick={() =>
                            void dispatch(run({ discard: { id: r.id } }))
                          }
                        >
                          {t('start.discard', { name })}
                        </button>
                      </div>
                    </td>
                  </tr>
                )
              })}
            </tbody>
          </table>
        </section>
      )}

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
                  <th scope="col">{t('start.colFile')}</th>
                  <th scope="col">{t('start.colOpened')}</th>
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
                    </th>
                    <td className="muted mono small-text">{r.path}</td>
                    <td>{when(r.at)}</td>
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
