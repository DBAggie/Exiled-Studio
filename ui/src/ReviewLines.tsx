// Lines the calculation does not read, one row each (the "Lines to
// review" redesign, maintainer 2026-11-24): the line as the game prints
// it, why it is not calculated in plain words, its item, and the text as
// written with its code behind "Show technical detail". The import
// review and the Warnings page share it; the same line on several copies
// of an item is one row (UX-16).
import { useId, useState } from 'react'

import type { WarningView } from './api/types'
import { ErrorCode } from './components'
import { t, tDynamic } from './i18n'
import { actions } from './store'
import { useAppDispatch, warningRows, type WarningRow } from './util'

/** The line as the game prints it: its roll applied, its prefixes left
 * out; as written when it is not a modifier line. */
function gameText(w: WarningView): string {
  return w.shown ?? w.text
}

function ReviewRow({ w, items }: WarningRow) {
  const dispatch = useAppDispatch()
  const [open, setOpen] = useState(false)
  const id = useId()
  const item = w['item-name'] ?? `#${w.item}`
  const variants = w.variants ?? []
  return (
    <li className={open ? 'open' : undefined}>
      <div className="review-line">
        <span id={`${id}-line`} className="line-text">
          {gameText(w)}
        </span>
        <span className="review-reason">
          {tDynamic(`warning.${w.code}`, 'reason.generic', { kind: w.code })}
        </span>
        {open && (
          <div id={`${id}-detail`} className="review-detail">
            <span className="review-detail-head">
              {variants.length === 0
                ? t('review.detail')
                : variants.length === 1
                  ? t('review.detailVariant', { n: variants[0] ?? 0 })
                  : t('review.detailVariants', { list: variants.join(', ') })}
            </span>
            <code>{w.text}</code>
            <ErrorCode code={w.code} />
          </div>
        )}
      </div>
      <button
        type="button"
        className="link plain review-item"
        aria-label={t('import.openItemName', { item, line: w.line + 1 })}
        onClick={() => dispatch(actions.editItem(w.item))}
      >
        {items.length > 1
          ? t('import.onCopies', { item, n: items.length })
          : item}
      </button>
      {/* A quiet link, not a button a row: its full name is read aloud
          and shown on hover. */}
      <button
        type="button"
        className="link quiet review-toggle"
        aria-expanded={open}
        aria-controls={open ? `${id}-detail` : undefined}
        aria-describedby={`${id}-line`}
        aria-label={open ? t('review.hideDetail') : t('review.showDetail')}
        title={open ? t('review.hideDetail') : t('review.showDetail')}
        onClick={() => setOpen(!open)}
      >
        {open ? t('review.hideShort') : t('review.detailShort')}
      </button>
    </li>
  )
}

/** The lines, one row each. */
export function ReviewList({ warnings }: { warnings: WarningView[] }) {
  return (
    <>
      <div className="review-head" aria-hidden="true">
        <span>{t('review.colLine')}</span>
        <span>{t('review.colItem')}</span>
        <span />
      </div>
      <ul className="review-list">
        {warningRows(warnings).map((row) => (
          <ReviewRow key={`${row.w.item}-${row.w.line}`} {...row} />
        ))}
      </ul>
    </>
  )
}
