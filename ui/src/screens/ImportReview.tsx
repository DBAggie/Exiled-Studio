// Screen 2 (docs/ui): what the import read.
import { ScreenHeading } from '../components'
import { t, tDynamic } from '../i18n'
import { actions } from '../store'
import { useAppDispatch, useAppSelector } from '../util'

export function ImportReview() {
  const dispatch = useAppDispatch()
  const build = useAppSelector((s) => s.session.snapshot?.build)
  if (build === null || build === undefined) {
    return null
  }
  const c = build.lines
  const version = build['data-version']
  return (
    <section>
      <ScreenHeading>{t('import.title')}</ScreenHeading>
      <dl className="grid-2">
        <div className="panel">
          <dt className="muted">{t('import.lines')}</dt>
          <dd>{c.modifiers}</dd>
        </div>
        <div className="panel">
          <dt className="muted">{t('import.matched')}</dt>
          <dd>{c.matched}</dd>
        </div>
        <div className="panel">
          <dt className="muted">{t('import.adapted')}</dt>
          <dd>{c.adapted}</dd>
        </div>
        <div className="panel">
          <dt className="muted">{t('import.warnings')}</dt>
          <dd>{c.warnings}</dd>
        </div>
        <div className="panel">
          <dt className="muted">{t('import.excluded')}</dt>
          <dd>{c.excluded}</dd>
        </div>
        <div className="panel">
          <dt className="muted">{t('import.data')}</dt>
          <dd>
            {version === null || version === undefined
              ? '—'
              : `${version.game} · ${version['steam-build'] ?? '?'}`}
          </dd>
        </div>
      </dl>
      {build.warnings.length > 0 && (
        <section className="panel" aria-labelledby="iw-h">
          <h2 id="iw-h">{t('import.warnings')}</h2>
          <p className="muted">{t('import.warningsHelp')}</p>
          <ul>
            {build.warnings.map((w) => (
              <li key={`${w.item}-${w.line}`}>
                <code>{w.text}</code> —{' '}
                {tDynamic(`warning.${w.code}`, 'reason.generic', {
                  kind: w.code,
                })}{' '}
                <span className="muted">({w.code})</span>
              </li>
            ))}
          </ul>
        </section>
      )}
      <button
        type="button"
        className="primary"
        onClick={() => {
          dispatch(actions.importDismissed())
          dispatch(actions.go('dashboard'))
        }}
      >
        {t('import.continue')}
      </button>
    </section>
  )
}
