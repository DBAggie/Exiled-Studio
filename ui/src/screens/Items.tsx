// Screens 7 and 5 (docs/ui), in this slice: the item list and the item
// editor (line status, edit, remove, add, preview of the effect, apply as
// one undoable step).
import { useEffect, useId, useState } from 'react'

import type { ItemEdit, ItemEditPreview, ItemView, LineOp } from '../api/types'
import { Badge, LineBadge, ScreenHeading } from '../components'
import { fieldLabel, formatNumber, t, tDynamic } from '../i18n'
import { actions, attempt, run } from '../store'
import { useAppDispatch, useAppSelector } from '../util'

function name(item: {
  name?: string | null
  base?: string | null
  id: number
}) {
  return item.name ?? item.base ?? `#${item.id}`
}

export function Items() {
  const dispatch = useAppDispatch()
  const items = useAppSelector((s) => s.session.snapshot?.build?.items ?? [])
  return (
    <section>
      <ScreenHeading>{t('items.title')}</ScreenHeading>
      {items.length === 0 ? (
        <p>{t('items.none')}</p>
      ) : (
        <table>
          <thead>
            <tr>
              <th scope="col">{t('items.name')}</th>
              <th scope="col">{t('items.base')}</th>
              <th scope="col">{t('items.slot')}</th>
              <th scope="col">{t('items.warnings')}</th>
            </tr>
          </thead>
          <tbody>
            {items.map((item) => (
              <tr key={item.id}>
                <th scope="row">
                  <button
                    type="button"
                    className={`rarity-${item.rarity ?? 'normal'}`}
                    aria-label={t('items.edit', { name: name(item) })}
                    onClick={() => dispatch(actions.editItem(item.id))}
                  >
                    {name(item)}
                  </button>
                </th>
                <td>{item.base ?? '—'}</td>
                <td>{item.slot ?? '—'}</td>
                <td>
                  {item.warnings > 0 ? (
                    <Badge tone="warn">{item.warnings}</Badge>
                  ) : (
                    '0'
                  )}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </section>
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
    <section>
      <ScreenHeading>{t('editor.title', { name: name(item) })}</ScreenHeading>
      <table>
        <thead>
          <tr>
            <th scope="col">{t('editor.line')}</th>
            <th scope="col">{t('editor.text')}</th>
            <th scope="col">{t('editor.status')}</th>
            <th scope="col">{t('editor.stats')}</th>
            <th scope="col">
              <span className="visually-hidden">
                {t('editor.remove', { n: '' })}
              </span>
            </th>
          </tr>
        </thead>
        <tbody>
          {item.lines.map((line) => {
            const n = line.index + 1
            const editable = EDITABLE.has(line.kind)
            const removed = draft.removed.has(line.index)
            const text = draft.texts[line.index] ?? line.text
            return (
              <tr key={line.index}>
                <td>{n}</td>
                <td>
                  {editable && !removed ? (
                    <input
                      className="mono"
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
                      size={Math.max(20, text.length)}
                    />
                  ) : (
                    <span className={removed ? 'muted mono' : 'mono'}>
                      {removed ? <del>{line.text}</del> : line.text}
                    </span>
                  )}
                </td>
                <td>
                  {removed ? (
                    <Badge tone="neutral">{t('editor.removed')}</Badge>
                  ) : text !== line.text ? (
                    <Badge tone="neutral">{t('status.edited')}</Badge>
                  ) : (
                    <LineBadge status={line.status} />
                  )}
                  {line.code !== null &&
                    line.code !== undefined &&
                    !removed && (
                      <div className="muted">
                        {tDynamic(`warning.${line.code}`, 'reason.generic', {
                          kind: line.code,
                        })}
                      </div>
                    )}
                </td>
                <td className="mono">
                  {line.stats.map((s) => (
                    <div key={s.id}>
                      {s.id} {s.value}
                    </div>
                  ))}
                </td>
                <td>
                  {editable && (
                    <button
                      type="button"
                      aria-pressed={removed}
                      onClick={() => {
                        const next = new Set(draft.removed)
                        if (removed) next.delete(line.index)
                        else next.add(line.index)
                        setPreview(null)
                        setDraft({ ...draft, removed: next })
                      }}
                    >
                      {removed
                        ? t('editor.restore', { n })
                        : t('editor.remove', { n })}
                    </button>
                  )}
                </td>
              </tr>
            )
          })}
        </tbody>
      </table>

      <form
        className="row panel"
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
          className="mono"
          value={newLine}
          onChange={(e) => setNewLine(e.target.value)}
        />
        <button type="submit">{t('editor.add')}</button>
        {draft.added.map((a, i) => (
          <Badge key={i} tone="neutral">
            + {a}
          </Badge>
        ))}
      </form>

      <div className="row">
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
        <button type="button" onClick={() => dispatch(actions.go('items'))}>
          {t('editor.back')}
        </button>
      </div>
      {!changed && <p className="muted">{t('editor.noChanges')}</p>}
      {applied && <p role="status">{t('editor.applied')}</p>}

      {preview !== null && (
        <section className="panel" aria-labelledby="ep-h">
          <h2 id="ep-h">{t('editor.effect')}</h2>
          <table>
            <thead>
              <tr>
                <th scope="col">{t('dashboard.field')}</th>
                <th scope="col">{t('editor.before')}</th>
                <th scope="col">{t('editor.after')}</th>
              </tr>
            </thead>
            <tbody>
              {preview.fields
                .filter(
                  (f) =>
                    f.before !== f.after ||
                    f['state-before'] !== f['state-after'],
                )
                .map((f) => (
                  <tr key={f.field}>
                    <th scope="row">{fieldLabel(f.field)}</th>
                    <td className="mono">
                      {f.before == null ? '—' : formatNumber(f.before)}
                    </td>
                    <td className="mono">
                      {f.after == null ? '—' : formatNumber(f.after)}
                    </td>
                  </tr>
                ))}
            </tbody>
          </table>
        </section>
      )}
    </section>
  )
}
