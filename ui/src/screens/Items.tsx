// Screens 7 and 5 (docs/ui, Equipment and ItemEditor), in this slice: the
// item list and the item editor (line status, edit, remove, add, the
// effect of the edit before Apply, applied as one undoable step). The
// editor shows an item card (UX-08) with a variant picker (UX-09), and
// each line as the game prints it; PoB's markup shows only while a line
// is edited (UX-07).
import { useEffect, useId, useState } from 'react'

import type {
  League,
  ItemEdit,
  ItemEditPreview,
  SetView,
  ItemSummary,
  ItemView,
  LineOp,
} from '../api/types'
import {
  Badge,
  ErrorCode,
  Hint,
  Icon,
  LineBadge,
  ScreenHeading,
  Sockets,
} from '../components'
import { socketsOverCap, valueLines, writtenProperties } from '../cardValues'
import { fieldLabel, t, tDynamic } from '../i18n'
import { actions, attempt, query, run } from '../store'
import { Inventory } from './Inventory'
import {
  cleanTitle,
  itemName,
  useAppDispatch,
  useAppSelector,
  withUnit,
} from '../util'

/** The equipment slots in the order the game's inventory shows them. */
const SLOT_ORDER = [
  'Weapon 1',
  'Weapon 2',
  'Helmet',
  'Body Armour',
  'Gloves',
  'Boots',
  'Amulet',
  'Ring 1',
  'Ring 2',
  'Ring 3',
  'Belt',
  'Flask 1',
  'Flask 2',
  'Flask 3',
  'Flask 4',
  'Flask 5',
]

/** Equipped first, in inventory order (abyssal and other slots after the
 * slot they belong to); then jewels in the passive tree. */
function slotRank(item: ItemSummary): number {
  if (item.slot == null) return 10_000
  const i = SLOT_ORDER.findIndex(
    (s) => item.slot === s || item.slot?.startsWith(`${s} `),
  )
  return i === -1 ? 1_000 : i
}

function ItemRow({ item, place }: { item: ItemSummary; place: string }) {
  const dispatch = useAppDispatch()
  const groups = useAppSelector(
    (s) => s.session.snapshot?.build?.['skill-groups'],
  )
  const league = useAppSelector((s) => s.session.snapshot?.build?.league)
  const legacy = item.legacy && league === 'challenge'
  // The sockets a gem of the active skill set sits in, for an item in use.
  const filled = new Set(
    item['in-use']
      ? (groups ?? [])
          .filter((g) => g.slot != null && g.slot === item.slot)
          .flatMap((g) => g.gems.map((x) => x.socket))
          .filter((s) => s != null)
      : [],
  )
  return (
    <li>
      <button
        type="button"
        className="item-row"
        aria-label={t('items.edit', { name: itemName(item) })}
        onClick={() => dispatch(actions.editItem(item.id))}
      >
        <span className="slot">{place}</span>
        <span>
          <span className={`name rarity-${item.rarity ?? 'normal'}`}>
            {itemName(item)}
          </span>
          {item.base != null && item.base !== itemName(item) && (
            <span className="sub">{item.base}</span>
          )}
          {item.sockets.length > 0 && (
            <span className="item-sockets">
              <Sockets groups={item.sockets} filled={filled} />
            </span>
          )}
        </span>
        <span className="item-badges">
          {legacy && <Badge tone="warn">{t('items.legacy')}</Badge>}
          {item.warnings > 0 && (
            <Badge tone={item['in-use'] ? 'warn' : 'neutral'}>
              {t('items.warningCount', { n: item.warnings })}
            </Badge>
          )}
        </span>
      </button>
    </li>
  )
}

function ItemSetPicker({ sets }: { sets: SetView[] }) {
  const dispatch = useAppDispatch()
  const id = useId()
  const active = sets.find((s) => s.active)
  return (
    <div className="set-picker">
      <label htmlFor={id}>{t('items.set')}</label>
      <select
        id={id}
        value={active?.id ?? ''}
        onChange={(e) =>
          void dispatch(run({ 'use-item-set': { id: Number(e.target.value) } }))
        }
      >
        {sets.map((s) => (
          <option key={s.id} value={s.id}>
            {setTitle(s)}
          </option>
        ))}
      </select>
      <Hint>{t('items.setHelp')}</Hint>
    </div>
  )
}

/** The league the build is planned for: in a challenge league, legacy
 * versions of unique items cannot be had, and items set to one say so. */
export function LeaguePicker({ league }: { league: League }) {
  const dispatch = useAppDispatch()
  const id = useId()
  return (
    <div className="set-picker">
      <label htmlFor={id}>{t('league.label')}</label>
      <select
        id={id}
        value={league}
        onChange={(e) =>
          void dispatch(
            run({ 'set-league': { league: e.target.value as League } }),
          )
        }
      >
        <option value="challenge">{t('league.challenge')}</option>
        <option value="standard">{t('league.standard')}</option>
      </select>
    </div>
  )
}

/** A set's title, without PoB's loadout markers (`Campaign {1,2}`). */
function setTitle(s: SetView): string {
  const title = cleanTitle(s.title)
  return title !== '' ? title : t('items.setN', { n: s.id })
}

export function Items() {
  const build = useAppSelector((s) => s.session.snapshot?.build)
  if (build == null) return null
  const items = build.items
  const sets = build['item-sets']
  const used = items
    .filter((i) => i['in-use'])
    .sort((a, b) => slotRank(a) - slotRank(b))
  const unused = items.filter((i) => !i['in-use'])
  const inSets = (item: ItemSummary) => {
    const names = sets
      .filter((s) => item.sets.includes(s.id))
      .map((s) => setTitle(s))
    return names.length === 0 ? t('items.inNoSet') : names.join(', ')
  }
  return (
    <>
      <div className="page-head">
        <ScreenHeading>{t('items.title')}</ScreenHeading>
        <span className="spacer" />
        <LeaguePicker league={build.league} />
        {sets.length > 1 && <ItemSetPicker sets={sets} />}
      </div>
      {/* The inventory stays in view beside the list (the brief's screen
          7); one column where the window is narrow. */}
      <div className="equip-layout">
        <Inventory />
        <div className="equip-side">
          <PasteItem />
          {items.length === 0 ? (
            <p>{t('items.none')}</p>
          ) : (
            <>
              <section
                className="panel table-panel"
                aria-labelledby="items-used"
              >
                <div className="panel-head">
                  <h2 id="items-used">{t('items.inUse')}</h2>
                </div>
                {used.length === 0 ? (
                  <p className="panel-foot">{t('items.inUseNone')}</p>
                ) : (
                  <ul className="item-list">
                    {used.map((item) => (
                      <ItemRow
                        key={item.id}
                        item={item}
                        place={
                          item['tree-socket'] != null
                            ? t('items.treeSocket')
                            : (item.slot ?? '—')
                        }
                      />
                    ))}
                  </ul>
                )}
              </section>
              {unused.length > 0 && (
                <details className="panel table-panel unused-items">
                  <summary>
                    {unused.length === 1
                      ? t('items.notInUseCount1')
                      : t('items.notInUseCount', { n: unused.length })}
                    <span className="muted small-text">
                      {' '}
                      {t('items.notInUseHelp')}
                    </span>
                  </summary>
                  <ul className="item-list">
                    {unused.map((item) => (
                      <ItemRow key={item.id} item={item} place={inSets(item)} />
                    ))}
                  </ul>
                </details>
              )}
            </>
          )}
        </div>
      </div>
    </>
  )
}

/** Whether a paste lands in a field of its own (then the page leaves it). */
function inField(target: EventTarget | null): boolean {
  return (
    target instanceof HTMLElement &&
    (target.isContentEditable ||
      ['INPUT', 'TEXTAREA', 'SELECT'].includes(target.tagName))
  )
}

/** "Paste an item" (B-2 item evidence, E3): the game's item copy or PoB's
 * item text becomes a new item, in a slot if one is picked, as one
 * undoable step. Pasting an item anywhere on the page outside a field
 * opens this with the text in it; nothing is added until Add. */
function PasteItem() {
  const dispatch = useAppDispatch()
  const textId = useId()
  const slotId = useId()
  const [open, setOpen] = useState(false)
  const [text, setText] = useState('')
  const [slot, setSlot] = useState('')
  const [added, setAdded] = useState<string | null>(null)
  useEffect(() => {
    const onPaste = (e: ClipboardEvent) => {
      if (inField(e.target)) return
      const pasted = e.clipboardData?.getData('text') ?? ''
      if (!/^\s*(Item Class: .*\n\s*)?Rarity: /.test(pasted)) return
      e.preventDefault()
      setText(pasted)
      setAdded(null)
      setOpen(true)
    }
    document.addEventListener('paste', onPaste)
    return () => document.removeEventListener('paste', onPaste)
  }, [])
  const add = async () => {
    const outcome = await dispatch(
      run({ 'add-item': { text, slot: slot === '' ? null : slot } }),
    )
    if (outcome !== null) {
      const name = text
        .split('\n')
        .map((l) => l.trim())
        .filter((l) => l !== '' && !/^(Item Class|Rarity):/.test(l))[0]
      setAdded(name ?? '')
      setText('')
    }
  }
  return (
    <details
      className="panel paste-item"
      open={open}
      onToggle={(e) => setOpen(e.currentTarget.open)}
    >
      <summary>{t('items.paste.title')}</summary>
      <p className="muted small-text">{t('items.paste.help')}</p>
      <label htmlFor={textId}>{t('items.paste.text')}</label>
      <textarea
        id={textId}
        rows={8}
        value={text}
        spellCheck={false}
        onChange={(e) => {
          setText(e.target.value)
          setAdded(null)
        }}
      />
      <div className="paste-actions">
        <label htmlFor={slotId}>{t('items.paste.slot')}</label>
        <select
          id={slotId}
          value={slot}
          onChange={(e) => setSlot(e.target.value)}
        >
          <option value="">{t('items.paste.noSlot')}</option>
          {SLOT_ORDER.filter((s) => s !== 'Ring 3').map((s) => (
            <option key={s} value={s}>
              {s}
            </option>
          ))}
        </select>
        <button
          type="button"
          className="primary"
          disabled={text.trim() === ''}
          onClick={() => void add()}
        >
          {t('items.paste.add')}
        </button>
      </div>
      {added !== null && (
        <p role="status">{t('items.paste.added', { name: added })}</p>
      )}
    </details>
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
/** Lines the item card shows, or file fields: folded under "File fields". */
const ON_CARD = new Set([
  'spec',
  'item-class',
  'influence',
  'flag',
  'variant-base',
])

/** The item card (UX-08): the base values, requirements, and sockets a
 * player reads, and the variant picker (UX-09). `locked` while a draft
 * is open: a variant change reads the item again. */
function ItemCardView({ item, locked }: { item: ItemView; locked: boolean }) {
  const dispatch = useAppDispatch()
  const id = useId()
  const league = useAppSelector((s) => s.session.snapshot?.build?.league)
  const c = item.card
  const eras = c['variant-eras']
  // Patch versions are an "item version"; a player's choice is a "kind".
  const versions = eras.some((e) => e !== 'choice')
  const selected =
    c['selected-variant'] == null ? null : eras[c['selected-variant'] - 1]
  const current = c['current-variant']
  const select = (variant: number) =>
    void dispatch(
      run({
        'edit-item': {
          item: item.id,
          ops: [{ 'select-variant': { variant } }],
          expect: [],
        },
      }),
    )
  const over = socketsOverCap(c)
  const facts = [
    c['item-class'],
    c.quality == null ? null : t('card.quality', { q: c.quality }),
    c['item-level'] == null
      ? null
      : t('card.itemLevel', { n: c['item-level'] }),
    c['level-req'] == null ? null : t('card.levelReq', { n: c['level-req'] }),
    c['max-sockets'] == null || c['max-sockets'] === 0
      ? null
      : t('card.maxSockets', { n: c['max-sockets'] }),
    over == null ? null : t('card.overCap', { n: over }),
    ...valueLines(c).map((v) => `${v.name}: ${v.value}`),
    c.values?.partial === true ? t('values.partial') : null,
    ...writtenProperties(c).map((p) => `${p.name}: ${p.value}`),
    ...c.influences.map((i) => t('card.influence', { name: i })),
    ...c.flags,
  ].filter((f): f is string => f != null && f !== '')
  if (facts.length === 0 && c.sockets.length === 0 && c.variants.length < 2) {
    return null
  }
  return (
    <div className="item-card">
      {c.sockets.length > 0 && <Sockets groups={c.sockets} />}
      {facts.length > 0 && (
        <ul className="item-facts" aria-label={t('card.label')}>
          {facts.map((f, i) => (
            <li key={i}>{f}</li>
          ))}
        </ul>
      )}
      {c.variants.length > 1 && (
        <div className="set-picker">
          <label htmlFor={id}>
            {t(versions ? 'card.version' : 'card.kind')}
          </label>
          <select
            id={id}
            value={c['selected-variant'] ?? ''}
            disabled={locked}
            onChange={(e) => select(Number(e.target.value))}
          >
            {c['selected-variant'] == null && (
              <option value="">{t('card.noVariant')}</option>
            )}
            {c.variants.map((v, i) => (
              <option key={i} value={i + 1}>
                {eras[i] === 'legacy' ? t('card.legacyOption', { name: v }) : v}
              </option>
            ))}
          </select>
          {locked && (
            <span className="muted small-text">{t('card.variantLocked')}</span>
          )}
        </div>
      )}
      {selected === 'legacy' &&
        (league === 'challenge' ? (
          <p className="legacy-note" role="note">
            <Badge tone="warn">{t('items.legacy')}</Badge>
            <span>{t('card.legacyChallenge')}</span>
            {current != null && (
              <button
                type="button"
                className="btn"
                disabled={locked}
                onClick={() => select(current)}
              >
                {t('card.useCurrent', {
                  name: c.variants[current - 1] ?? current,
                })}
              </button>
            )}
          </p>
        ) : (
          <p className="legacy-note muted" role="note">
            {t('card.legacyStandard')}
          </p>
        ))}
    </div>
  )
}

/** The changed fields of a preview, as "before → after" rows. */
function Impact({
  preview,
  inUse,
}: {
  preview: ItemEditPreview
  inUse: boolean | undefined
}) {
  const changed = preview.fields.filter(
    (f) => f.before !== f.after || f['state-before'] !== f['state-after'],
  )
  if (changed.length === 0) {
    // Say why when the item is not in use (UX-06).
    return (
      <p className="muted">
        {inUse === false ? t('editor.noEffectUnused') : t('editor.noEffect')}
      </p>
    )
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
  const inUse = useAppSelector(
    (s) =>
      s.session.snapshot?.build?.items.find((i) => i.id === id)?.['in-use'],
  )
  const [item, setItem] = useState<ItemView | null>(null)
  const [draft, setDraft] = useState<Draft>(emptyDraft)
  const [newLine, setNewLine] = useState('')
  const [preview, setPreview] = useState<ItemEditPreview | null>(null)
  const [applied, setApplied] = useState(false)
  // The last applied change's effect, kept until the next edit (UX-24).
  const [last, setLast] = useState<ItemEditPreview | null>(null)
  const dirty = useAppSelector((s) => s.session.snapshot?.dirty === true)
  // The game stat IDs each line reads: for checking, off by default (UX-10).
  const [showStats, setShowStats] = useState(false)
  // Lines whose PoB text is open for editing (UX-07).
  const [editing, setEditing] = useState<ReadonlySet<number>>(new Set())
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

  const edit = item === null ? null : editOf(item, draft)
  const changed = edit !== null && edit.ops.length > 0
  // A live preview a moment after the user stops typing (UX-24).
  const editKey = changed ? JSON.stringify(edit) : ''
  useEffect(() => {
    if (editKey === '') return
    let live = true
    const timer = setTimeout(() => {
      void dispatch(
        query((api) => api.preview(JSON.parse(editKey) as ItemEdit)),
      ).then((p) => {
        if (live && p !== null) setPreview(p)
      })
    }, 500)
    return () => {
      live = false
      clearTimeout(timer)
    }
  }, [dispatch, editKey])

  if (item === null || edit === null) return null

  const reset = () => {
    setDraft(emptyDraft())
    setPreview(null)
    setEditing(new Set())
  }
  const fileFields = item.lines.filter(
    (line) => ON_CARD.has(line.kind) && line.status !== 'warning',
  )

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
            <ItemCardView item={item} locked={drafting} />
            {inUse === false && (
              <p className="top-gap">
                <Badge tone="neutral">{t('items.notInUse')}</Badge>{' '}
                <span className="muted small-text">
                  {t('items.notInUseEditor')}
                </span>
              </p>
            )}
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
          <label className="check stats-toggle">
            <input
              type="checkbox"
              checked={showStats}
              onChange={(e) => setShowStats(e.target.checked)}
            />
            {t('editor.showStats')}
          </label>
        </p>
        <ol className="lines">
          {item.lines
            .filter(
              (line) =>
                !(IN_HEADING.has(line.kind) || ON_CARD.has(line.kind)) ||
                line.status === 'warning',
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
                    {editable &&
                    !removed &&
                    (line.kind !== 'modifier' ||
                      editing.has(line.index) ||
                      text !== line.text) ? (
                      <input
                        aria-label={t('editor.lineLabel', { n })}
                        autoFocus={editing.has(line.index)}
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
                        {removed ? (
                          <del>{line.shown ?? line.text}</del>
                        ) : (
                          (line.shown ?? line.text)
                        )}
                        {line.marks.map((m) => (
                          <Badge key={m} tone="info" className="line-mark">
                            {tDynamic(`editor.mark.${m}`, 'editor.mark.other', {
                              mark: m,
                            })}
                          </Badge>
                        ))}
                        {line.variants.length > 0 &&
                          item.card.variants.length > 0 && (
                            <Badge tone="neutral" className="line-mark">
                              {t('editor.variantOf', {
                                names: line.variants
                                  .map((v) => item.card.variants[v - 1] ?? v)
                                  .join(', '),
                              })}
                            </Badge>
                          )}
                        {editable && !removed && (
                          <button
                            type="button"
                            className="link small-text line-edit"
                            aria-label={t('editor.editLine', { n })}
                            onClick={() =>
                              setEditing(new Set([...editing, line.index]))
                            }
                          >
                            {t('editor.edit')}
                          </button>
                        )}
                      </span>
                    )}
                    {showStats && line.stats.length > 0 && !removed && (
                      <div
                        className={`stats${text !== line.text ? ' stale' : ''}`}
                      >
                        {text !== line.text && (
                          <span className="stale-note">
                            {t('editor.statsStale')}{' '}
                          </span>
                        )}
                        {line.stats
                          .map((s) => `${s.id} ${s.value}`)
                          .join(' · ')}
                      </div>
                    )}
                    {line.code != null && !removed && (
                      <div className="stats">
                        {tDynamic(`warning.${line.code}`, 'reason.generic', {
                          kind: line.code,
                        })}
                        <ErrorCode code={line.code} />
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
        {fileFields.length > 0 && (
          <details className="file-fields">
            <summary>
              {t('editor.fileFields', { n: fileFields.length })}
            </summary>
            <p className="muted small-text">{t('editor.fileFieldsHelp')}</p>
            <ul>
              {fileFields.map((line) => (
                <li key={line.index}>
                  <code>{line.text}</code>
                </li>
              ))}
            </ul>
          </details>
        )}
      </section>

      <div className="aside stack">
        <section className="panel" aria-labelledby="ep-h">
          <div className="panel-head">
            <h2 id="ep-h">{t('editor.effect')}</h2>
          </div>
          {preview !== null ? (
            <Impact preview={preview} inUse={inUse} />
          ) : !changed && last !== null ? (
            <>
              <p className="muted small-text">{t('editor.lastApplied')}</p>
              <Impact preview={last} inUse={inUse} />
            </>
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
              onClick={() => {
                const apply = async () => {
                  // The effect to keep: the one shown, else asked for now.
                  const shown =
                    preview ??
                    (await dispatch(query((api) => api.preview(edit))))
                  const o = await dispatch(run({ 'edit-item': edit }))
                  if (o !== null) {
                    reset()
                    setApplied(true)
                    setLast(shown)
                  }
                }
                void apply()
              }}
            >
              {t('editor.apply')}
            </button>
            <button type="button" disabled={!changed} onClick={reset}>
              {t('editor.reset')}
            </button>
          </div>
          {/* Until the build is saved (UX-24). */}
          {applied && dirty && <p role="status">{t('editor.applied')}</p>}
        </section>
      </div>
    </div>
  )
}
