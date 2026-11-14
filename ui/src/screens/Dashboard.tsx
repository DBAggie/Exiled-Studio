// Screen 3 (docs/ui, Workspace): offence and defence, then warnings.
// Fields come from the calc result document (never a fixed list), each
// with its state; a field without a value says why, never a blank or a
// zero. A row is one button: label, value, and badge on one line.
import type { FieldResult } from '../api/types'
import { Badge, FieldBadge, ScreenHeading } from '../components'
import { OFFENCE, fieldLabel, t } from '../i18n'
import { actions } from '../store'
import {
  fieldStateText,
  fieldValue,
  reasonText,
  useAppDispatch,
  useAppSelector,
} from '../util'

function why(f: FieldResult): string | null {
  if (f.state === 'unsupported' && f.reason != null) {
    return reasonText(f.reason)
  }
  if (f.state === 'indeterminate' && f.indeterminate !== undefined) {
    return f.indeterminate.map(reasonText).join(' ')
  }
  if (f.state === 'partial' && f.unsupported !== undefined) {
    return f.unsupported.map(reasonText).join(' ')
  }
  return null
}

function FieldPanel({
  id,
  title,
  fields,
  hint,
}: {
  id: string
  title: string
  fields: FieldResult[]
  hint?: string
}) {
  const dispatch = useAppDispatch()
  return (
    <section className="panel" aria-labelledby={id}>
      <div className="panel-head">
        <h2 id={id}>{title}</h2>
        {hint !== undefined && <span className="muted small-text">{hint}</span>}
      </div>
      <ul className="stat-list">
        {fields.map((f) => {
          const reason = why(f)
          const negative =
            f.field.startsWith('resistance.') &&
            typeof f.value === 'number' &&
            f.value < 0
          return (
            <li key={f.field}>
              <button
                type="button"
                className="stat-row"
                aria-label={t('dashboard.row', {
                  field: fieldLabel(f.field),
                  value: fieldValue(f),
                  state: fieldStateText(f),
                  reason: reason === null ? '' : ` ${reason}`,
                })}
                onClick={() => dispatch(actions.breakdown(f.field))}
              >
                <span className="stat-label">
                  <span>{fieldLabel(f.field)}</span>
                  {reason !== null && <span className="reason">{reason}</span>}
                </span>
                <span className="stat-end">
                  <span className={`stat-value${negative ? ' negative' : ''}`}>
                    {fieldValue(f)}
                  </span>
                  <FieldBadge field={f} />
                </span>
              </button>
            </li>
          )
        })}
      </ul>
    </section>
  )
}

export function Dashboard() {
  const dispatch = useAppDispatch()
  const snapshot = useAppSelector((s) => s.session.snapshot)
  const calc = useAppSelector((s) => s.session.calc)
  const build = snapshot?.build
  if (build === null || build === undefined) {
    return null
  }
  const fields = calc?.[1].result.fields ?? []
  const offence = fields.filter((f) => OFFENCE.has(f.field))
  const defence = fields.filter((f) => !OFFENCE.has(f.field))
  const version = build['data-version']
  const character = [
    build.class != null && build.level != null
      ? t('dashboard.character', {
          class: build.ascendancy ?? build.class,
          level: build.level,
        })
      : null,
    build['main-skill'] != null
      ? t('dashboard.mainSkill', { skill: build['main-skill'] })
      : null,
  ].filter((x) => x !== null)
  return (
    <>
      <div className="page-head">
        <ScreenHeading>{t('dashboard.title')}</ScreenHeading>
        <span className="spacer" />
        {version != null && (
          <Badge tone="neutral">
            <span className="mono">
              {version.game} · {version['steam-build'] ?? '?'}
            </span>
          </Badge>
        )}
      </div>
      {character.length > 0 && <p className="lead">{character.join(' · ')}</p>}
      {calc === null ? (
        <p>{t('dashboard.noCalc')}</p>
      ) : (
        <div className="columns">
          <FieldPanel
            id="fields-offence"
            title={t('dashboard.offence')}
            fields={offence}
            hint={t('dashboard.hint')}
          />
          <FieldPanel
            id="fields-defence"
            title={t('dashboard.defence')}
            fields={defence}
          />
        </div>
      )}
      {build.warnings.length > 0 && (
        <section className="panel" aria-labelledby="dw-h">
          <div className="panel-head">
            <h2 id="dw-h">{t('nav.warnings')}</h2>
            <button
              type="button"
              className="link"
              onClick={() => dispatch(actions.go('warnings'))}
            >
              {t('dashboard.viewWarnings')}
            </button>
          </div>
          <p className="muted small-text">
            {build.warnings.length === 1
              ? t('dashboard.warnings1')
              : t('dashboard.warnings', { n: build.warnings.length })}
          </p>
          <ul className="warning-strip">
            {build.warnings.slice(0, 6).map((w) => (
              <li key={`${w.item}-${w.line}`}>
                <span className="mono" title={w.text}>
                  {w.text}
                </span>
                <span className="muted">{w['item-name'] ?? `#${w.item}`}</span>
              </li>
            ))}
          </ul>
        </section>
      )}
    </>
  )
}
