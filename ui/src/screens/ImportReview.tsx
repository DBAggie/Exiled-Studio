// Screen 2 (docs/ui, ImportReview): what the import read. Counts, the
// lines with warnings (kept, not calculated), and the data the build is
// pinned to.
import { Badge, ScreenHeading } from '../components'
import { formatNumber, t, tDynamic } from '../i18n'
import { actions } from '../store'
import { buildStatus, useAppDispatch, useAppSelector } from '../util'

export function ImportReview() {
  const dispatch = useAppDispatch()
  const build = useAppSelector((s) => s.session.snapshot?.build)
  if (build === null || build === undefined) {
    return null
  }
  const c = build.lines
  const version = build['data-version']
  const status = buildStatus(build)
  const name = build.name === '' ? t('app.untitled') : build.name
  const share =
    c.modifiers > 0
      ? `${formatNumber(Math.floor((c.matched / c.modifiers) * 1000) / 10)}%`
      : '—'
  const open = () => {
    dispatch(actions.importDismissed())
    dispatch(actions.go('dashboard'))
  }

  return (
    <div className="stack import-page">
      <div className="pills">
        <Badge tone={status.tone}>
          {t('import.imported', {
            state: tDynamic(`state.${build.completeness}`, 'state.unsupported'),
          })}
        </Badge>
        <Badge tone="neutral">{build.game === 'poe2' ? 'PoE2' : 'PoE1'}</Badge>
      </div>
      <ScreenHeading>{name}</ScreenHeading>
      <p className="muted">
        {build.warnings.length === 0
          ? t('import.summaryClean')
          : build.warnings.length === 1
            ? t('import.summaryWarnings1')
            : t('import.summaryWarnings', { n: build.warnings.length })}
      </p>
      <div className="row">
        <button type="button" className="primary" onClick={open}>
          {t('import.continue')}
        </button>
      </div>

      <dl className="stat-cards">
        <div className="panel stat-card">
          <dt>{t('import.lines')}</dt>
          <dd>{formatNumber(c.modifiers)}</dd>
          <dd className="sub">
            {t('import.acrossItems', { n: build.items.length })}
          </dd>
        </div>
        <div className="panel stat-card">
          <dt>{t('import.matched')}</dt>
          <dd className="tone-ok">{formatNumber(c.matched)}</dd>
          <dd className="sub">{share}</dd>
        </div>
        <div className="panel stat-card">
          <dt>{t('import.adapted')}</dt>
          <dd className="tone-info">{formatNumber(c.adapted)}</dd>
          <dd className="sub">{t('import.adaptedHelp')}</dd>
        </div>
        <div className="panel stat-card">
          <dt>{t('import.warnings')}</dt>
          <dd className={c.warnings > 0 ? 'tone-warn' : undefined}>
            {formatNumber(c.warnings)}
          </dd>
          <dd className="sub">{t('import.warningsSub')}</dd>
        </div>
      </dl>

      <div className="columns">
        <section className="panel table-panel" aria-labelledby="iw-h">
          <div className="panel-head">
            <h2 id="iw-h">{t('import.review')}</h2>
          </div>
          {build.warnings.length === 0 ? (
            <p className="panel-foot">{t('import.reviewNone')}</p>
          ) : (
            <>
              <ul className="review-list">
                {build.warnings.map((w) => {
                  const item = w['item-name'] ?? `#${w.item}`
                  return (
                    <li key={`${w.item}-${w.line}`}>
                      <div>
                        <div className="line-text">{w.text}</div>
                        <span className="sub">{item}</span>
                      </div>
                      <div>
                        <Badge tone="warn">
                          <span className="mono">{w.code}</span>
                        </Badge>
                        <p className="muted small-text">
                          {tDynamic(`warning.${w.code}`, 'reason.generic', {
                            kind: w.code,
                          })}
                        </p>
                      </div>
                      <div className="actions">
                        <button
                          type="button"
                          aria-label={t('import.openItemName', {
                            item,
                            line: w.line + 1,
                          })}
                          onClick={() => dispatch(actions.editItem(w.item))}
                        >
                          {t('import.openItem')}
                        </button>
                      </div>
                    </li>
                  )
                })}
              </ul>
              <p className="panel-foot small-text">
                {t('import.warningsHelp')}
              </p>
            </>
          )}
        </section>

        <section className="panel aside" aria-labelledby="ib-h">
          <div className="panel-head">
            <h2 id="ib-h">{t('import.buildTitle')}</h2>
          </div>
          <dl className="facts">
            {build.class != null && (
              <div>
                <dt className="muted">{t('import.character')}</dt>
                <dd>
                  {build.level != null
                    ? t('dashboard.character', {
                        class: build.ascendancy ?? build.class,
                        level: build.level,
                      })
                    : (build.ascendancy ?? build.class)}
                </dd>
              </div>
            )}
            {build['main-skill'] != null && (
              <div>
                <dt className="muted">{t('import.mainSkill')}</dt>
                <dd>{build['main-skill']}</dd>
              </div>
            )}
            <div>
              <dt className="muted">{t('import.excluded')}</dt>
              <dd>{formatNumber(c.excluded)}</dd>
            </div>
            <div>
              <dt className="muted">{t('import.data')}</dt>
              <dd className="mono">
                {version === null || version === undefined
                  ? '—'
                  : `${version.game} · ${version['steam-build'] ?? '?'}`}
              </dd>
            </div>
          </dl>
          <p className="muted small-text">{t('import.partialHelp')}</p>
        </section>
      </div>
    </div>
  )
}
