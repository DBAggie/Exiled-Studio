// Comparing the active stage with another (UX audit follow-up, maintainer
// 2026-11-16): the picker and the equipment table. The reading and the
// differences are in `../stageDiff`.
import { useId } from 'react'

import type { BuildView } from '../api/types'
import { t } from '../i18n'
import { slotDiff, useCompareStage, useStageView } from '../stageDiff'
import { actions } from '../store'
import { itemName, useAppDispatch } from '../util'

/** The picker: no comparison, or one of the build's other stages. Shown
 * only for a build with more than one stage. */
export function StageCompareSelect({ build }: { build: BuildView }) {
  const dispatch = useAppDispatch()
  const id = useId()
  const chosen = useCompareStage(build)
  const others = build.loadouts.filter((l) => !l.active)
  if (others.length === 0) return null
  return (
    <div className="set-picker">
      <label htmlFor={id}>{t('stageDiff.label')}</label>
      <select
        id={id}
        value={chosen?.spec ?? ''}
        onChange={(e) =>
          dispatch(
            actions.compareStage(
              e.target.value === '' ? null : Number(e.target.value),
            ),
          )
        }
      >
        <option value="">{t('stageDiff.none')}</option>
        {others.map((l) => (
          <option key={l.spec} value={l.spec}>
            {l.title}
          </option>
        ))}
      </select>
    </div>
  )
}

/** The equipment that differs from the stage compared with: a row a
 * slot, this stage's item and the other's. Nothing when no stage is
 * chosen. */
export function EquipmentDiff({ build }: { build: BuildView }) {
  const dispatch = useAppDispatch()
  const stage = useCompareStage(build)
  const loaded = useStageView(stage?.spec ?? null)
  if (stage === null) return null
  const name = (id: number | null) => {
    if (id === null)
      return <span className="muted">{t('stageDiff.empty')}</span>
    const item = build.items.find((i) => i.id === id)
    return (
      <button
        type="button"
        className={`link rarity-${(item?.rarity ?? 'normal').toLowerCase()}`}
        onClick={() => dispatch(actions.editItem(id))}
      >
        {item === undefined ? `#${id}` : itemName(item)}
      </button>
    )
  }
  const here = build.items.flatMap((i) =>
    i['in-use'] && i.slot != null ? [{ slot: i.slot, item: i.id }] : [],
  )
  return (
    <section className="panel table-panel" aria-labelledby="equip-diff">
      <div className="panel-head">
        <h2 id="equip-diff">
          {t('stageDiff.equipment', { other: stage.title })}
        </h2>
      </div>
      {loaded.state === 'loading' && (
        <p className="panel-foot muted" role="status">
          {t('stageDiff.loading')}
        </p>
      )}
      {loaded.state === 'failed' && (
        <p className="panel-foot" role="status">
          {t('stageDiff.failed')}
        </p>
      )}
      {loaded.state === 'done' &&
        (() => {
          const diff = slotDiff(here, loaded.view.slots)
          if (diff.rows.length === 0) {
            return <p className="panel-foot">{t('stageDiff.allSame')}</p>
          }
          return (
            <>
              <div className="table-scroll">
                <table className="data-table stage-diff-table">
                  <thead>
                    <tr>
                      <th scope="col">{t('stageDiff.slot')}</th>
                      <th scope="col">{t('stageDiff.here')}</th>
                      <th scope="col">{stage.title}</th>
                    </tr>
                  </thead>
                  <tbody>
                    {diff.rows.map((r) => (
                      <tr key={r.slot}>
                        <th scope="row">{r.slot}</th>
                        <td>{name(r.here)}</td>
                        <td>{name(r.other)}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
              {diff.same > 0 && (
                <p className="panel-foot muted small-text">
                  {diff.same === 1
                    ? t('stageDiff.sameSlots1')
                    : t('stageDiff.sameSlots', { n: diff.same })}
                </p>
              )}
            </>
          )
        })()}
    </section>
  )
}
