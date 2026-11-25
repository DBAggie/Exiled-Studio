// Screen 2 (docs/ui, ImportReview): what the import read. What is
// calculated first, then the lines with warnings (kept, not calculated):
// those on items in use, then, folded, those on other items; the same
// line on several copies of an item once (UX-16). The data the build is
// pinned to.
import { ScreenHeading } from '../components'
import { formatNumber, t } from '../i18n'
import { ReviewList } from '../ReviewLines'
import { actions } from '../store'
import {
  splitWarnings,
  useAppDispatch,
  useAppSelector,
  useDataName,
  warningRows,
} from '../util'

export function ImportReview() {
  const dispatch = useAppDispatch()
  const build = useAppSelector((s) => s.session.snapshot?.build)
  const dataLabel = useDataName(build?.['data-version'])
  if (build === null || build === undefined) {
    return null
  }
  const c = build.lines
  const { used, unused } = splitWarnings(build)
  const name = build.name === '' ? t('app.untitled') : build.name
  // Of the lines that apply (not other variants or notes), the share the
  // calculation reads (UX-16).
  const applies = c.modifiers - c.excluded
  const calculated = c.matched + c.adapted
  const share =
    applies > 0
      ? `${formatNumber(Math.floor((calculated / applies) * 1000) / 10)}%`
      : '—'
  const open = () => {
    dispatch(actions.importDismissed())
    dispatch(actions.go('dashboard'))
  }

  return (
    <div className="stack import-page">
      {/* The name, what the import is, and the way on in one band (UX
          audit follow-up): the numbers start right under it. */}
      <div className="import-head">
        <div className="import-title">
          <ScreenHeading>{name}</ScreenHeading>
          <p className="muted">
            {build.warnings.length === 0
              ? t('import.summaryClean')
              : build.warnings.length === 1
                ? t('import.summaryWarnings1')
                : t('import.summaryWarnings', { n: build.warnings.length })}
            {unused.length > 0 &&
              ` ${
                used.length === 0
                  ? t('import.summaryUnusedAll')
                  : t('import.summaryUnused', { n: unused.length })
              }`}
          </p>
        </div>
        <button type="button" className="primary" onClick={open}>
          {t('import.continue')}
        </button>
      </div>

      <dl className="stat-cards">
        <div className="panel stat-card">
          <dt>{t('import.calculated')}</dt>
          <dd>{formatNumber(calculated)}</dd>
          <dd className="sub">
            {t('import.calculatedOf', { share, n: applies })}
          </dd>
        </div>
        <div className="panel stat-card">
          <dt>{t('import.lines')}</dt>
          <dd>{formatNumber(c.modifiers)}</dd>
          <dd className="sub">
            {t('import.acrossItems', { n: build.items.length })}
          </dd>
        </div>
        <div className="panel stat-card">
          <dt>{t('import.adapted')}</dt>
          <dd>{formatNumber(c.adapted)}</dd>
          <dd className="sub">{t('import.adaptedHelp')}</dd>
        </div>
        <div className="panel stat-card">
          <dt>{t('import.warnings')}</dt>
          {/* What touches the numbers: the lines on items in use. Those
              on other items are said beside it. */}
          <dd className={used.length > 0 ? 'tone-warn' : undefined}>
            {formatNumber(warningRows(used).length)}
          </dd>
          <dd className="sub">
            {unused.length > 0
              ? t('import.reviewMore', { n: warningRows(unused).length })
              : t('import.warningsSub')}
          </dd>
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
              {used.length > 0 ? (
                <ReviewList warnings={used} />
              ) : (
                <p className="panel-foot">{t('import.reviewNoneUsed')}</p>
              )}
              {unused.length > 0 && (
                <details className="unused-review">
                  <summary>
                    {t('import.reviewUnused', {
                      n: warningRows(unused).length,
                    })}
                  </summary>
                  <ReviewList warnings={unused} />
                </details>
              )}
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
              <dd>
                {formatNumber(c.excluded)}
                <span className="sub">{t('import.excludedHelp')}</span>
              </dd>
            </div>
            <div>
              <dt className="muted">{t('import.data')}</dt>
              <dd>{dataLabel}</dd>
            </div>
          </dl>
          <p className="muted small-text">{t('import.partialHelp')}</p>
        </section>
      </div>
    </div>
  )
}
