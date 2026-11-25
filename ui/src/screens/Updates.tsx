// Application updates (Phase 5, P-d; docs/phase-5/updates.md): the
// setting with its privacy note (About), and the prompt that offers a newer
// version. Every install is the user's click; a package that does not
// update itself only shows where to download.
import type { UpdateMode, UpdateStatus } from '../api/types'
import { t } from '../i18n'
import { actions, checkUpdates, installUpdate, setUpdateMode } from '../store'
import { useAppDispatch, useAppSelector } from '../util'

const MODES = {
  'automatic-check': 'updates.mode.automatic-check',
  manual: 'updates.mode.manual',
  'strict-offline': 'updates.mode.strict-offline',
} as const satisfies Record<UpdateMode, string>

/** The update section of About. */
export function UpdateSettings() {
  const dispatch = useAppDispatch()
  const status = useAppSelector((s) => s.lists.updates)
  if (status === null) return null
  const offline = status.mode === 'strict-offline'
  return (
    <section className="panel stack" aria-labelledby="updates-h">
      <h2 id="updates-h">{t('updates.title')}</h2>
      {/* Each choice is a row that says what it does; the one chosen is
          marked. */}
      <fieldset className="mode-list">
        <legend>{t('updates.mode')}</legend>
        {(Object.keys(MODES) as UpdateMode[]).map((mode) => (
          <div
            key={mode}
            className={`mode-row${status.mode === mode ? ' chosen' : ''}`}
          >
            <input
              type="radio"
              id={`update-mode-${mode}`}
              name="update-mode"
              value={mode}
              checked={status.mode === mode}
              aria-describedby={`update-mode-${mode}-help`}
              onChange={() => void dispatch(setUpdateMode(mode))}
            />
            <div>
              <label htmlFor={`update-mode-${mode}`}>{t(MODES[mode])}</label>
              <span id={`update-mode-${mode}-help`} className="sub">
                {t(`${MODES[mode]}.help`)}
              </span>
            </div>
          </div>
        ))}
      </fieldset>
      <p className="muted small-text">{t('updates.privacy')}</p>
      {!status.available ? (
        <p className="muted">{t('updates.unavailable')}</p>
      ) : offline ? (
        <p className="muted">{t('updates.offline')}</p>
      ) : (
        <div className="row">
          <button
            type="button"
            disabled={status.checking}
            onClick={() => void dispatch(checkUpdates())}
          >
            {status.checking ? t('updates.checking') : t('updates.check')}
          </button>
          {status.checked && status.offer == null && status.error == null && (
            <span role="status">
              {t('updates.current', { version: status.current })}
            </span>
          )}
        </div>
      )}
      {status.offer != null && <Offer status={status} />}
    </section>
  )
}

/** The prompt above the page when a check found a newer version, until the
 * user installs it or puts it off. */
export function UpdatePrompt() {
  const status = useAppSelector((s) => s.lists.updates)
  const later = useAppSelector((s) => s.lists.updateLater)
  if (status?.offer == null || later === status.offer.version) return null
  return (
    <div className="panel" role="region" aria-label={t('updates.available')}>
      <Offer status={status} prompt />
    </div>
  )
}

function Offer({ status, prompt }: { status: UpdateStatus; prompt?: boolean }) {
  const dispatch = useAppDispatch()
  const offer = status.offer
  if (offer == null) return null
  return (
    <div className="stack">
      <p>
        <strong>
          {t('updates.offer', {
            version: offer.version,
            current: status.current,
          })}
        </strong>
      </p>
      {offer.notes != null && offer.notes !== '' && (
        <details>
          <summary>{t('updates.notes')}</summary>
          <pre className="update-notes">{offer.notes}</pre>
        </details>
      )}
      {status.installs ? (
        <>
          <p className="muted">{t('updates.installHelp')}</p>
          <div className="row">
            <button
              type="button"
              className="primary"
              onClick={() => void dispatch(installUpdate())}
            >
              {t('updates.install')}
            </button>
            {prompt === true && (
              <button
                type="button"
                onClick={() => dispatch(actions.updateLater(offer.version))}
              >
                {t('updates.later')}
              </button>
            )}
          </div>
        </>
      ) : (
        <div className="row">
          <p className="selectable">
            {t('updates.notifyOnly', {
              version: offer.version,
              url: offer.url,
            })}
          </p>
          {prompt === true && (
            <button
              type="button"
              onClick={() => dispatch(actions.updateLater(offer.version))}
            >
              {t('updates.later')}
            </button>
          )}
        </div>
      )}
    </div>
  )
}
