// Screens 7 and 5 (docs/ui, Equipment and ItemEditor), in this slice: the
// item list and the item editor (line status, edit, remove, add, the
// effect of the edit before Apply, applied as one undoable step).
import { useEffect, useId, useState } from 'react'

import type { ItemEdit, ItemEditPreview, ItemView, LineOp } from '../api/types'
import { Badge, Icon, LineBadge, ScreenHeading } from '../components'
import { fieldLabel, t, tDynamic } from '../i18n'
import { actions, attempt, run } from '../store'
import { itemName, useAppDispatch, useAppSelector, withUnit } from '../util'

export function Items() {
  const dispatch = useAppDispatch()
  const items = useAppSelector((s) => s.session.snapshot?.build?.items ?? [])
  return (
    <>
      <ScreenHeading>{t('items.title')}</ScreenHeading>
      {items.length === 0 ? (
        <p>{t('items.none')}</p>
      ) : (
        <section className="panel table-panel" aria-label={t('items.title')}>
          <ul className="item-list">
            {items.map((item) => (
              <li key={item.id}>
                <button
                  type="button"
                  className="item-row"
                  aria-label={t('items.edit', { name: itemName(item) })}
                  onClick={() => dispatch(actions.editItem(item.id))}
                >
                  <span className="slot">{item.slot ?? '—'}</span>
                  <span>
                    <span className={`name rarity-${item.rarity ?? 'normal'}`}>
                      {itemName(item)}
                    </span>
                    {item.base != null && item.base !== itemName(item) && (
                      <span className="sub">{item.base}</span>
                    )}
                  </span>
                  {item.warnings > 0 ? (
                    <Badge tone="warn">
                      {t('items.warningCount', { n: item.warnings })}
                    </Badge>
                  ) : (
                    <span />
                  )}
                </button>
              </li>
            ))}
          </ul>
        </section>
      )}
    </>
  )
}

interface Draft {
  texts: Record<number, string>
  removed: Set<number>
  added: string[]
}

const emptyDraft = (): Draft => ({ texts: {}, removed: new Set(), added: [] })

function editOf(item: ItemView, draft: Draft): ItemEdit {
  const ops: LineOp[] = []
  const expect: ItemEdit['expect'] = []
  for (const line of item.lines) {
    if (draft.removed.has(line.index)) {
      ops.push({ remove: { line: line.index } })
      expect.push({ line: line.index, text: line.text })
    } else {
      const text = draft.texts[line.index]
      if (text !== undefined && text !== line.text) {
        ops.push({ replace: { line: line.index, text } })
        expect.push({ line: line.index, text: line.text })
      }
    }
  }
  for (const text of draft.added) {
    if (text.trim() !== '') ops.push({ add: { text } })
  }
  return { item: item.id, ops, expect }
}

const EDITABLE = new Set(['modifier', 'unrecognized', 'spec-unknown'])
/** Lines the editor's heading shows (rarity, name, base) or that only lay
 * the text out (separators): kept in the item and its export, not listed
 * again unless they carry a warning (ADR-015: never hidden). */
const IN_HEADING = new Set(['rarity', 'name', 'base', 'separator'])

/** The changed fields of a preview, as "before → after" rows. */
function Impact({ preview }: { preview: ItemEditPreview }) {
  const changed = preview.fields.filter(
    (f) => f.before !== f.after || f['state-before'] !== f['state-after'],
  )
  if (changed.length === 0) {
    return <p className="muted">{t('editor.noEffect')}</p>
  }
  return (
    <ul className="impact-list">
      {changed.map((f) => {
        const delta =
          f.before != null && f.after != null ? f.after - f.before : null
        return (
          <li key={f.field}>
            <span>{fieldLabel(f.field)}</span>
            <span className="row">
              <span className="before">
                {f.before == null ? '—' : withUnit(f.field, f.before)} →
              </span>
              <span className="after">
                {f.after == null ? '—' : withUnit(f.field, f.after)}
              </span>
              {delta !== null && delta !== 0 && (
                <span className={delta > 0 ? 'delta-up' : 'delta-down'}>
                  {delta > 0 ? '+' : '−'}
                  {withUnit(f.field, Math.abs(delta))}
                </span>
              )}
            </span>
          </li>
        )
      })}
    </ul>
  )
}

export function ItemEditor() {
  const dispatch = useAppDispatch()
  const id = useAppSelector((s) => s.ui.item)
  const version = useAppSelector((s) => s.session.snapshot?.version)
  const [item, setItem] = useState<ItemView | null>(null)
  const [draft, setDraft] = useState<Draft>(emptyDraft)
  const [newLine, setNewLine] = useState('')
  const [preview, setPreview] = useState<ItemEditPreview | null>(null)
  const [applied, setApplied] = useState(false)
  const addId = useId()

  // While a draft is open the editor keeps the item it was made against,
  // so the edit names the lines the user saw; if they changed meanwhile,
  // the application refuses it as stale (APP_STALE_EDIT).
  const drafting =
    Object.keys(draft.texts).length > 0 ||
    draft.removed.size > 0 ||
    draft.added.length > 0
  useEffect(() => {
    if (id === null || drafting) return
    let live = true
    void dispatch(attempt((api) => api.item(id))).then((v) => {
      if (live && v !== null) setItem(v)
    })
    return () => {
      live = false
    }
  }, [dispatch, id, version, drafting])

  if (item === null) return null
  const edit = editOf(item, draft)
  const changed = edit.ops.length > 0

  const reset = () => {
    setDraft(emptyDraft())
    setPreview(null)
  }

  return (
    <div className="columns">
      <section className="panel table-panel" aria-labelledby="item-h">
        <div className="item-head">
          <div className="spacer">
            <p className="eyebrow">
              {[item.slot, item.rarity].filter((x) => x != null).join(' · ')}
            </p>
            <ScreenHeading
              id="item-h"
              className={`rarity-${item.rarity ?? 'normal'}`}
            >
              {itemName(item)}
            </ScreenHeading>
            {item.base != null && <p className="muted">{item.base}</p>}
          </div>
          <button type="button" onClick={() => dispatch(actions.go('items'))}>
            {t('editor.back')}
          </button>
        </div>
        <p className="legend">
          <span>
            <LineBadge status="matched" />
            {t('editor.legendMatched')}
          </span>
          <span>
            <LineBadge status="adapted" />
            {t('editor.legendAdapted')}
          </span>
          <span>
            <LineBadge status="warning" />
            {t('editor.legendWarning')}
          </span>
        </p>
        <ol className="lines">
          {item.lines
            .filter(
              (line) => !IN_HEADING.has(line.kind) || line.status === 'warning',
            )
            .map((line) => {
              const n = line.index + 1
              const editable = EDITABLE.has(line.kind)
              const removed = draft.removed.has(line.index)
              const text = draft.texts[line.index] ?? line.text
              const kind = line.section ?? line.kind
              return (
                <li
                  key={line.index}
                  className={`line-row${line.status === 'warning' && !removed ? ' warning' : ''}`}
                >
                  <span className="kind">
                    {tDynamic(`editor.kind.${kind}`, 'editor.kind.other', {
                      kind,
                    })}
                  </span>
                  <div>
                    {editable && !removed ? (
                      <input
                        aria-label={t('editor.lineLabel', { n })}
                        value={text}
                        onChange={(e) => {
                          setPreview(null)
                          setDraft({
                            ...draft,
                            texts: {
                              ...draft.texts,
                              [line.index]: e.target.value,
                            },
                          })
                        }}
                      />
                    ) : (
                      <span className={removed ? 'text muted' : 'text'}>
                        {removed ? <del>{line.text}</del> : line.text}
                      </span>
                    )}
                    {line.stats.length > 0 && !removed && (
                      <div className="stats">
                        {line.stats
                          .map((s) => `${s.id} ${s.value}`)
                          .join(' · ')}
                      </div>
                    )}
                    {line.code != null && !removed && (
                      <div className="stats">
                        {line.code} ·{' '}
                        {tDynamic(`warning.${line.code}`, 'reason.generic', {
                          kind: line.code,
                        })}
                      </div>
                    )}
                  </div>
                  <span className="status">
                    {removed ? (
                      <Badge tone="neutral">{t('editor.removed')}</Badge>
                    ) : text !== line.text ? (
                      <Badge tone="neutral">{t('status.edited')}</Badge>
                    ) : (
                      <LineBadge status={line.status} />
                    )}
                  </span>
                  {editable ? (
                    <button
                      type="button"
                      className="icon"
                      aria-pressed={removed}
                      aria-label={
                        removed
                          ? t('editor.restore', { n })
                          : t('editor.remove', { n })
                      }
                      title={
                        removed
                          ? t('editor.restore', { n })
                          : t('editor.remove', { n })
                      }
                      onClick={() => {
                        const next = new Set(draft.removed)
                        if (removed) next.delete(line.index)
                        else next.add(line.index)
                        setPreview(null)
                        setDraft({ ...draft, removed: next })
                      }}
                    >
                      {removed ? <Icon name="undo" /> : <Icon name="close" />}
                    </button>
                  ) : (
                    <span />
                  )}
                </li>
              )
            })}
          {draft.added.map((a, i) => (
            <li key={`added-${i}`} className="line-row">
              <span className="kind">{t('editor.kind.added')}</span>
              <span className="text">{a}</span>
              <span className="status">
                <Badge tone="neutral">{t('status.edited')}</Badge>
              </span>
              <span />
            </li>
          ))}
        </ol>
        <form
          className="add-line"
          onSubmit={(e) => {
            e.preventDefault()
            if (newLine.trim() === '') return
            setPreview(null)
            setDraft({ ...draft, added: [...draft.added, newLine] })
            setNewLine('')
          }}
        >
          <label htmlFor={addId}>{t('editor.addLabel')}</label>
          <input
            id={addId}
            value={newLine}
            placeholder={t('editor.addPlaceholder')}
            onChange={(e) => setNewLine(e.target.value)}
          />
          <button type="submit">{t('editor.add')}</button>
        </form>
      </section>

      <div className="aside stack">
        <section className="panel" aria-labelledby="ep-h">
          <div className="panel-head">
            <h2 id="ep-h">{t('editor.effect')}</h2>
          </div>
          {preview !== null ? (
            <Impact preview={preview} />
          ) : (
            <p className="muted">
              {changed ? t('editor.previewHint') : t('editor.noChanges')}
            </p>
          )}
          <div className="row top-gap">
            <button
              type="button"
              disabled={!changed}
              onClick={() =>
                void dispatch(attempt((api) => api.preview(edit))).then((p) => {
                  if (p !== null) setPreview(p)
                })
              }
            >
              {t('editor.preview')}
            </button>
          </div>
        </section>
        <section className="panel stack" aria-label={t('editor.applyTitle')}>
          <p className="muted">{t('editor.applyHelp')}</p>
          <div className="row">
            <button
              type="button"
              className="primary"
              disabled={!changed}
              onClick={() =>
                void dispatch(run({ 'edit-item': edit })).then((o) => {
                  if (o !== null) {
                    reset()
                    setApplied(true)
                  }
                })
              }
            >
              {t('editor.apply')}
            </button>
            <button type="button" disabled={!changed} onClick={reset}>
              {t('editor.reset')}
            </button>
          </div>
          {applied && <p role="status">{t('editor.applied')}</p>}
        </section>
      </div>
    </div>
  )
}
