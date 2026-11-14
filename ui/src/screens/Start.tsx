// Screen 1 (docs/ui): import a PoB build, open a saved one, recent builds,
// unsaved work to restore, the game data.
import { useId, useState } from 'react'

import { ScreenHeading } from '../components'
import { errorText, t } from '../i18n'
import { attempt, importBuild, openBuild, run } from '../store'
import { useAppDispatch, useAppSelector } from '../util'

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
    <div className="start">
      <ScreenHeading>{t('start.title')}</ScreenHeading>
      <p className="muted">{t('start.lead')}</p>

      <section className="panel" aria-labelledby={`${textId}-h`}>
        <h2 id={`${textId}-h`}>{t('start.import')}</h2>
        <form
          onSubmit={(e) => {
            e.preventDefault()
            void dispatch(
              importBuild((api) => api.execute({ 'import-text': { text } })),
            )
          }}
        >
          <label htmlFor={textId}>{t('start.importLabel')}</label>
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
            <button
              type="button"
              onClick={() => void dispatch(openBuild((api) => api.openFile()))}
            >
              {t('start.open')}
            </button>
          </div>
        </form>
        {reading && <p role="status">{t('import.reading')}</p>}
        {flow.state === 'failed' && (
          <p role="alert" className="error">
            {errorText(flow.code)} <code>{flow.code}</code>
          </p>
        )}
      </section>

      {recovery.length > 0 && (
        <section className="panel" aria-labelledby="recovery-h">
          <h2 id="recovery-h">{t('start.recovery')}</h2>
          <ul>
            {recovery.map((r) => (
              <li key={r.id} className="row">
                <span>{r.name === '' ? t('app.untitled') : r.name}</span>
                <span className="muted">
                  {new Date(r['saved-at'] * 1000).toLocaleString()}
                </span>
                <button
                  type="button"
                  onClick={() =>
                    void dispatch(
                      openBuild((api) =>
                        api.execute({ restore: { id: r.id } }),
                      ),
                    )
                  }
                >
                  {t('start.restore', {
                    name: r.name === '' ? t('app.untitled') : r.name,
                  })}
                </button>
                <button
                  type="button"
                  onClick={() => void dispatch(run({ discard: { id: r.id } }))}
                >
                  {t('start.discard', {
                    name: r.name === '' ? t('app.untitled') : r.name,
                  })}
                </button>
              </li>
            ))}
          </ul>
        </section>
      )}

      <section className="panel" aria-labelledby="recent-h">
        <h2 id="recent-h">{t('start.recent')}</h2>
        {recent.length === 0 ? (
          <p className="muted">{t('start.recentNone')}</p>
        ) : (
          <ul>
            {recent.map((r) => (
              <li key={r.id}>
                <button
                  type="button"
                  onClick={() =>
                    void dispatch(
                      openBuild((api) =>
                        api.execute({ 'open-recent': { id: r.id } }),
                      ),
                    )
                  }
                >
                  {r.name}
                </button>{' '}
                <span className="muted mono">{r.path}</span>
              </li>
            ))}
          </ul>
        )}
      </section>

      <section className="panel" aria-labelledby="data-h">
        <h2 id="data-h">{t('start.data')}</h2>
        {version === null || version === undefined ? (
          <p>{t('start.dataNone')}</p>
        ) : (
          <p>
            {t('start.dataLoaded', {
              game:
                version.game === 'poe1' ? 'Path of Exile' : 'Path of Exile 2',
              build: version['steam-build'] ?? '?',
            })}
          </p>
        )}
        {data?.error !== null && data?.error !== undefined && (
          <p className="error">{errorText(data.error.code)}</p>
        )}
        <button
          type="button"
          onClick={() => void dispatch(attempt((api) => api.chooseBundle()))}
        >
          {t('start.chooseData')}
        </button>
      </section>

      <p className="notice">{t('app.notice')}</p>
    </div>
  )
}
