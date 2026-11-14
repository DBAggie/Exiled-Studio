// Screen 3 (docs/ui): offence and defence, then warnings. Fields come from
// the calc result document (never a fixed list), each with its state; a
// field without a value says why, never a blank or a zero.
import type { FieldResult } from '../api/types'
import { StateBadge, ScreenHeading } from '../components'
import { OFFENCE, fieldLabel, t } from '../i18n'
import { actions } from '../store'
import { fieldValue, reasonText, useAppDispatch, useAppSelector } from '../util'

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

function FieldTable({
  title,
  fields,
}: {
  title: string
  fields: FieldResult[]
}) {
  const dispatch = useAppDispatch()
  const id = `fields-${title}`
  return (
    <section className="panel" aria-labelledby={id}>
      <h2 id={id}>{title}</h2>
      <table>
        <thead>
          <tr>
            <th scope="col">{t('dashboard.field')}</th>
            <th scope="col">{t('dashboard.value')}</th>
            <th scope="col">{t('dashboard.state')}</th>
          </tr>
        </thead>
        <tbody>
          {fields.map((f) => {
            const reason = why(f)
            return (
              <tr key={f.field}>
                <th scope="row">
                  <button
                    type="button"
                    aria-label={t('dashboard.explain', {
                      field: fieldLabel(f.field),
                    })}
                    onClick={() => dispatch(actions.breakdown(f.field))}
                  >
                    {fieldLabel(f.field)}
                  </button>
                </th>
                <td className="mono">{fieldValue(f)}</td>
                <td>
                  <StateBadge state={f.state} />
                  {reason !== null && <div className="muted">{reason}</div>}
                </td>
              </tr>
            )
          })}
        </tbody>
      </table>
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
  return (
    <section>
      <ScreenHeading>{t('dashboard.title')}</ScreenHeading>
      <p className="muted">
        {build.class != null &&
          build.level != null &&
          t('dashboard.character', { class: build.class, level: build.level })}
        {build['main-skill'] != null && (
          <> · {t('dashboard.mainSkill', { skill: build['main-skill'] })}</>
        )}
      </p>
      {calc === null ? (
        <p>{t('dashboard.noCalc')}</p>
      ) : (
        <div className="grid-2">
          <FieldTable title={t('dashboard.offence')} fields={offence} />
          <FieldTable title={t('dashboard.defence')} fields={defence} />
        </div>
      )}
      {build.warnings.length > 0 && (
        <section className="panel" aria-labelledby="dw-h">
          <h2 id="dw-h">{t('nav.warnings', { n: build.warnings.length })}</h2>
          <p>{t('dashboard.warnings', { n: build.warnings.length })}</p>
          <button
            type="button"
            onClick={() => dispatch(actions.go('warnings'))}
          >
            {t('dashboard.viewWarnings')}
          </button>
        </section>
      )}
    </section>
  )
}
