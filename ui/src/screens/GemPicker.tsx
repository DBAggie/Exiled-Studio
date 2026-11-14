// The gem picker: the game data's gems, searched by name or tag, active
// skills or supports. For supports, the ones that fit the group come
// first (the game data's support rules); the others are listed apart.
// Each pick adds the gem to the group as one undoable step; the dialog
// stays open so a whole link can be filled in one go.
import { Dialog } from 'radix-ui'
import { useEffect, useId, useState } from 'react'

import type { GemCatalogEntry } from '../api/types'
import { t } from '../i18n'
import { actions, query, run } from '../store'
import { useAppDispatch } from '../util'

type Kind = 'active' | 'support'

function matches(e: GemCatalogEntry, text: string): boolean {
  const q = text.trim().toLowerCase()
  if (q === '') return true
  return (
    e.name.toLowerCase().includes(q) ||
    e.tags.some((tag) => tag.toLowerCase() === q)
  )
}

function Entry({
  entry,
  twin,
  onPick,
}: {
  entry: GemCatalogEntry
  // Another gem has the same name (an old and a new version): the data's
  // ID tells them apart.
  twin: boolean
  onPick: (e: GemCatalogEntry) => void
}) {
  const name = twin ? `${entry.name} (${entry.id})` : entry.name
  return (
    <li>
      <button
        type="button"
        className="picker-entry"
        aria-label={t('picker.add', { gem: name })}
        onClick={() => onPick(entry)}
      >
        <span
          className={`gem-mark gem-${entry.colour ?? 'unknown'}${entry.support ? ' square' : ''}`}
          aria-hidden="true"
        />
        <span className="picker-name">
          {entry.name}
          {twin && <span className="muted small-text"> {entry.id}</span>}
        </span>
        <span className="muted small-text">{entry.tags.join(', ')}</span>
      </button>
    </li>
  )
}

export function GemPicker({
  open,
  onOpenChange,
  group,
  title,
}: {
  open: boolean
  onOpenChange: (open: boolean) => void
  group: number
  title: string
}) {
  const dispatch = useAppDispatch()
  const searchId = useId()
  const [catalog, setCatalog] = useState<GemCatalogEntry[] | null>(null)
  const [text, setText] = useState('')
  const [kind, setKind] = useState<Kind>('active')
  const [added, setAdded] = useState<string[]>([])

  useEffect(() => {
    if (!open) return
    let live = true
    void dispatch(query((api) => api.gems(group))).then((c) => {
      if (live) setCatalog(c ?? [])
    })
    return () => {
      live = false
    }
  }, [dispatch, open, group])

  const pick = (e: GemCatalogEntry) => {
    void dispatch(run({ 'add-gem': { group, gem: e.id } })).then((o) => {
      if (o !== null) {
        setAdded((a) => [...a, e.name])
        dispatch(actions.announce(t('picker.added', { gem: e.name })))
      }
    })
  }

  const shown = (catalog ?? []).filter(
    (e) => e.support === (kind === 'support') && matches(e, text),
  )
  const fitting = shown.filter((e) => e.fits !== false)
  const other = shown.filter((e) => e.fits === false)
  const counts = new Map<string, number>()
  for (const e of catalog ?? []) {
    const key = `${e.support}:${e.name}`
    counts.set(key, (counts.get(key) ?? 0) + 1)
  }
  const twin = (e: GemCatalogEntry) =>
    (counts.get(`${e.support}:${e.name}`) ?? 0) > 1

  return (
    <Dialog.Root
      open={open}
      onOpenChange={(o) => {
        if (!o) {
          setText('')
          setAdded([])
        }
        onOpenChange(o)
      }}
    >
      <Dialog.Portal>
        <Dialog.Content className="dialog-content picker">
          <Dialog.Title>{t('picker.title', { group: title })}</Dialog.Title>
          <Dialog.Description className="muted">
            {t('picker.help')}
          </Dialog.Description>
          <div className="row">
            <div
              className="segmented"
              role="group"
              aria-label={t('picker.kind')}
            >
              <button
                type="button"
                aria-pressed={kind === 'active'}
                onClick={() => setKind('active')}
              >
                {t('picker.actives')}
              </button>
              <button
                type="button"
                aria-pressed={kind === 'support'}
                onClick={() => setKind('support')}
              >
                {t('picker.supports')}
              </button>
            </div>
            <label htmlFor={searchId} className="visually-hidden">
              {t('picker.search')}
            </label>
            <input
              id={searchId}
              type="search"
              className="spacer"
              placeholder={t('picker.search')}
              value={text}
              onChange={(e) => setText(e.target.value)}
            />
          </div>
          {catalog === null ? (
            <p className="muted">{t('about.loading')}</p>
          ) : (
            <div className="picker-lists">
              {kind === 'support' && other.length > 0 && (
                <p className="muted small-text">{t('picker.fitting')}</p>
              )}
              <ul className="picker-list" aria-label={t('picker.results')}>
                {fitting.slice(0, 200).map((e) => (
                  <Entry key={e.id} entry={e} twin={twin(e)} onPick={pick} />
                ))}
              </ul>
              {fitting.length === 0 && (
                <p className="muted">{t('picker.none')}</p>
              )}
              {kind === 'support' && other.length > 0 && (
                <details className="picker-other">
                  <summary>
                    {t('picker.notFitting', { n: other.length })}
                  </summary>
                  <ul className="picker-list">
                    {other.slice(0, 200).map((e) => (
                      <Entry
                        key={e.id}
                        entry={e}
                        twin={twin(e)}
                        onPick={pick}
                      />
                    ))}
                  </ul>
                </details>
              )}
            </div>
          )}
          <div className="row">
            {added.length > 0 && (
              <p className="muted small-text spacer" role="status">
                {t('picker.addedList', { gems: added.join(', ') })}
              </p>
            )}
            <Dialog.Close asChild>
              <button type="button" className="primary">
                {t('picker.done')}
              </button>
            </Dialog.Close>
          </div>
        </Dialog.Content>
      </Dialog.Portal>
    </Dialog.Root>
  )
}
