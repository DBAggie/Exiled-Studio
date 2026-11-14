// Screen 2 (docs/ui, ImportReview): what the import read. What is
// calculated first, then the lines with warnings (kept, not calculated):
// those on items in use, then, folded, those on other items; the same
// line on several copies of an item once (UX-16). The data the build is
// pinned to.
import type { WarningView } from '../api/types'
import { Badge, ErrorCode, ScreenHeading } from '../components'
import { formatNumber, t, tDynamic } from '../i18n'
import { actions } from '../store'
import {
  buildStatus,
  splitWarnings,
  useAppDispatch,
  useAppSelector,
} from '../util'

/** Warnings with the same item name, text, and reason as one row, with
 * the items it is on. */
function grouped(warnings: WarningView[]) {
  const rows = new Map<string, { w: WarningView; items: number[] }>()
  for (const w of warnings) {
    const key = `${w['item-name'] ?? w.item}\u0000${w.text}\u0000${w.code}`
    const row = rows.get(key)
    if (row === undefined) rows.set(key, { w, items: [w.item] })
    else if (!row.items.includes(w.item)) row.items.push(w.item)
  }
  return [...rows.values()]
}

function ReviewList({ warnings }: { warnings: WarningView[] }) {
  const dispatch = useAppDispatch()
  return (
    <ul className="review-list">
      {grouped(warnings).map(({ w, items }) => {
        const item = w['item-name'] ?? `#${w.item}`
        return (
          <li key={`${w.item}-${w.line}`}>
            <div>
              <div className="line-text">{w.text}</div>
              <span className="sub">
                {items.length > 1
                  ? t('import.onCopies', { item, n: items.length })
                  : item}
              </span>
            </div>
            <div>
              <Badge tone="warn">{t('warning.badge')}</Badge>
              <p className="muted small-text">
                {tDynamic(`warning.${w.code}`, 'reason.generic', {
                  kind: w.code,
                })}
              </p>
              <ErrorCode code={w.code} />
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
  )
}

export function ImportReview() {
  const dispatch = useAppDispatch()
  const build = useAppSelector((s) => s.session.snapshot?.build)
  if (build === null || build === undefined) {
    return null
  }
  const c = build.lines
  const version = build['data-version']
  const status = buildStatus(build)
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
      <div className="pills">
        <Badge tone={status.tone}>
          {t('import.imported', {
            state: tDynamic(
              `state.${build['in-use-completeness']}`,
              'state.unsupported',
            ),
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
        {unused.length > 0 &&
          ` ${
            used.length === 0
              ? t('import.summaryUnusedAll')
              : t('import.summaryUnused', { n: unused.length })
          }`}
      </p>
      <div className="row">
        <button type="button" className="primary" onClick={open}>
          {t('import.continue')}
        </button>
      </div>

      <dl className="stat-cards">
        <div className="panel stat-card">
          <dt>{t('import.calculated')}</dt>
          <dd className="tone-ok">{formatNumber(calculated)}</dd>
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
              {used.length > 0 ? (
                <ReviewList warnings={used} />
              ) : (
                <p className="panel-foot">{t('import.reviewNoneUsed')}</p>
              )}
              {unused.length > 0 && (
                <details className="unused-review">
                  <summary>
                    {t('import.reviewUnused', { n: grouped(unused).length })}
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
