// Skills: the active skill set by equipment slot, as build sites show it.
// Each slot card names its item and draws its sockets and links; under it
// the slot's socket groups, each active skill with the supports that
// support it indented beneath (the game data's support rules, PoB's
// `supports`). Level, quality, on or off, and the main skill are edited
// here; every change is one undoable command. Gems are marked by their
// colour, never with game art (ADR-000).
import { ImportedText } from '../ImportedText'
import { plain } from '../colourCodes'
import { Fragment, useId, useState, type ReactNode } from 'react'

import type {
  GemView,
  ItemSummary,
  SetView,
  SkillGroupView,
} from '../api/types'
import { Badge, Hint, Icon, ScreenHeading, Sockets } from '../components'
import { t } from '../i18n'
import { actions, run, setPrefs } from '../store'
import { cleanTitle, itemName, useAppDispatch, useAppSelector } from '../util'

import { GemPicker } from './GemPicker'

/** A number field that sends its value when the user leaves it or
 * presses Enter, and only when it changed and is valid. */
function NumberField({
  value,
  min,
  max,
  label,
  onCommit,
}: {
  value: number
  min: number
  max: number
  label: string
  onCommit: (n: number) => void
}) {
  const [text, setText] = useState(String(value))
  const [seen, setSeen] = useState(value)
  // A new value from the application (undo, another stage) replaces the
  // text.
  if (seen !== value) {
    setSeen(value)
    setText(String(value))
  }
  const commit = () => {
    const n = Number(text)
    if (Number.isInteger(n) && n >= min && n <= max && n !== value) {
      onCommit(n)
    } else {
      setText(String(value))
    }
  }
  return (
    <input
      type="number"
      className="gem-number"
      inputMode="numeric"
      min={min}
      max={max}
      aria-label={label}
      value={text}
      onChange={(e) => setText(e.target.value)}
      onBlur={commit}
      onKeyDown={(e) => {
        if (e.key === 'Enter') commit()
      }}
    />
  )
}

const gemName = (gem: GemView, index: number) =>
  gem.name ?? t('skills.gemN', { n: index + 1 })

/** The active a support is shown under: the main skill when it supports
 * it, else the first active it supports; undefined when none. */
function home(gem: GemView, gems: GemView[]): number | undefined {
  return gem.supports.find((a) => gems[a]?.main === true) ?? gem.supports[0]
}

function GemRow({
  gems,
  group,
  index,
  nested = false,
  children,
}: {
  gems: GemView[]
  group: number
  index: number
  nested?: boolean
  children?: ReactNode
}) {
  const dispatch = useAppDispatch()
  const gem = gems[index]
  if (gem == null) return null
  const name = gemName(gem, index)
  const edit = (change: {
    level?: number
    quality?: number
    enabled?: boolean
  }) => void dispatch(run({ 'edit-gem': { group, gem: index, ...change } }))
  // A support shown under one active: name the others it supports too.
  const shown = home(gem, gems)
  const others = gem.supports.flatMap((a) => {
    if (a === shown) return []
    const other = gems[a]
    return other == null ? [] : [gemName(other, a)]
  })
  return (
    <li>
      <div
        className={`gem-row${gem.enabled ? '' : ' disabled'}${gem.support === true ? ' support' : ''}${nested ? ' nested' : ''}`}
      >
        <span className="gem-name">
          {nested && <span className="gem-branch" aria-hidden="true" />}
          <span
            className={`gem-mark gem-${gem.colour ?? 'unknown'}`}
            aria-hidden="true"
          />
          <span>
            <span className="gem-title">{name}</span>
            <span className="sub">
              {gem.support === true
                ? t('skills.support')
                : gem.support === false
                  ? t('skills.active')
                  : t('skills.unknown')}
              {others.length > 0 && (
                <>
                  {' · '}
                  {t('skills.alsoSupports', { gems: others.join(', ') })}
                </>
              )}
              {gem.main ? (
                <>
                  {' · '}
                  <span className="main-gem">{t('skills.main')}</span>
                </>
              ) : (
                gem.support !== true && (
                  <>
                    {' · '}
                    <button
                      type="button"
                      className="link"
                      aria-label={t('skills.makeMainOf', { gem: name })}
                      onClick={() =>
                        void dispatch(
                          run({
                            'edit-socket-group': {
                              group,
                              main: true,
                              'main-gem': index,
                            },
                          }),
                        )
                      }
                    >
                      {t('skills.makeMain')}
                    </button>
                  </>
                )
              )}
            </span>
          </span>
        </span>
        <NumberField
          value={gem.level}
          min={1}
          max={40}
          label={t('skills.levelOf', { gem: name })}
          onCommit={(level) => edit({ level })}
        />
        <NumberField
          value={gem.quality}
          min={0}
          max={23}
          label={t('skills.qualityOf', { gem: name })}
          onCommit={(quality) => edit({ quality })}
        />
        <input
          type="checkbox"
          aria-label={t('skills.gemEnabled', { gem: name })}
          checked={gem.enabled}
          onChange={(e) => edit({ enabled: e.target.checked })}
        />
        <button
          type="button"
          className="icon small"
          aria-label={t('skills.removeGem', { gem: name })}
          title={t('skills.removeGem', { gem: name })}
          onClick={() =>
            void dispatch(run({ 'remove-gem': { group, gem: index } }))
          }
        >
          <Icon name="close" size={14} />
        </button>
      </div>
      {children}
    </li>
  )
}

/** A group's gems as a tree: each active skill (or a gem the data does
 * not know) at the top, the supports shown under it ([home]) beneath;
 * supports that support nothing in the group last, apart. */
function GemTree({ gems, group }: { gems: GemView[]; group: number }) {
  const actives = gems
    .map((g, i) => ({ g, i }))
    .filter(({ g }) => g.support !== true)
  const under = (a: number) =>
    gems
      .map((g, i) => ({ g, i }))
      .filter(({ g }) => g.support === true && home(g, gems) === a)
  const idle = gems
    .map((g, i) => ({ g, i }))
    .filter(({ g }) => g.support === true && g.supports.length === 0)
  return (
    <>
      <ul className="gem-list">
        {actives.map(({ g, i }) => {
          const supports = under(i)
          return (
            <GemRow key={i} gems={gems} group={group} index={i}>
              {supports.length > 0 && (
                <ul
                  className="gem-list"
                  aria-label={t('skills.supportsOf', { gem: gemName(g, i) })}
                >
                  {supports.map((s) => (
                    <GemRow
                      key={s.i}
                      gems={gems}
                      group={group}
                      index={s.i}
                      nested
                    />
                  ))}
                </ul>
              )}
            </GemRow>
          )
        })}
      </ul>
      {idle.length > 0 && (
        <>
          <p className="muted small-text top-gap">
            {t('skills.supportsNothing')}
          </p>
          <ul className="gem-list idle">
            {idle.map((s) => (
              <GemRow key={s.i} gems={gems} group={group} index={s.i} />
            ))}
          </ul>
        </>
      )}
    </>
  )
}

/** The link group (index in the item's `sockets`) of each socket. */
function linkOf(sockets: string[]): number[] {
  return sockets.flatMap((g, i) => [...g].map(() => i))
}

function GroupBlock({
  group,
  index,
  item,
}: {
  group: SkillGroupView
  index: number
  item: ItemSummary | undefined
}) {
  const dispatch = useAppDispatch()
  const id = useId()
  const [picking, setPicking] = useState(false)
  // The author's label, its colour codes drawn, never printed; names read
  // aloud use it plain.
  const label = group.label != null && group.label !== '' ? group.label : null
  const title =
    (label === null ? '' : plain(label)) || t('skills.groupN', { n: index + 1 })
  // The heading the application writes: the group's skill and its links
  // (maintainer 2026-11-24); the label is the line under it.
  const lead =
    group.gems.find((g) => g.main) ??
    group.gems.find((g) => g.support !== true) ??
    group.gems[0]
  const heading =
    lead?.name == null
      ? title
      : group.gems.length > 1
        ? t('skills.heading', { gem: lead.name, n: group.gems.length })
        : lead.name
  const place = (change: { slot?: string; label?: string }) =>
    void dispatch(run({ 'edit-socket-group': { group: index, ...change } }))
  const links = linkOf(item?.sockets ?? [])
  const used = new Set(
    group.gems
      .map((g) => (g.socket == null ? undefined : links[g.socket]))
      .filter((l) => l != null),
  )
  const unsocketed =
    item != null && item.sockets.length > 0
      ? group.gems.filter((g) => g.socket == null).length
      : 0
  return (
    <section
      className={`skill-group${group.enabled ? '' : ' disabled'}${group.main ? ' main' : ''}`}
      aria-labelledby={id}
    >
      <div className="skill-group-head">
        <div className="skill-group-title">
          <h3 id={id}>{heading}</h3>
          {label !== null && heading !== title && (
            <p className="muted small-text">
              <ImportedText text={label} />
            </p>
          )}
        </div>
        <span className="spacer" />
        {used.size > 1 && <Badge tone="warn">{t('skills.unlinked')}</Badge>}
        {unsocketed > 0 && (
          <Badge tone="warn">
            {unsocketed === 1
              ? t('skills.unsocketed1')
              : t('skills.unsocketed', { n: unsocketed })}
          </Badge>
        )}
        {group.main && <Badge tone="info">{t('skills.mainGroup')}</Badge>}
        <label className="check">
          <input
            type="checkbox"
            aria-label={t('skills.groupEnabledOf', { group: title })}
            checked={group.enabled}
            onChange={(e) =>
              void dispatch(
                run({
                  'edit-socket-group': {
                    group: index,
                    enabled: e.target.checked,
                  },
                }),
              )
            }
          />
          {t('skills.on')}
        </label>
      </div>
      <div className="gem-head muted small-text" aria-hidden="true">
        <span>{t('skills.colGem')}</span>
        <span>{t('skills.colLevel')}</span>
        <span>{t('skills.colQuality')}</span>
        <span>{t('skills.colOn')}</span>
        <span />
      </div>
      {group.gems.length === 0 ? (
        <p className="muted small-text">{t('skills.empty')}</p>
      ) : (
        <GemTree gems={group.gems} group={index} />
      )}
      <div className="row top-gap group-actions">
        <button type="button" onClick={() => setPicking(true)}>
          <Icon name="plus" size={16} />
          {t('skills.addGem')}
        </button>
        <details className="group-settings">
          <summary>{t('skills.groupSettings')}</summary>
          <div className="row">
            <label className="field">
              <span>{t('skills.label')}</span>
              <TextField
                value={group.label ?? ''}
                label={t('skills.labelOf', { group: title })}
                onCommit={(label) => place({ label })}
              />
            </label>
            <label className="field">
              <span>{t('skills.slot')}</span>
              <select
                value={group.slot ?? ''}
                onChange={(e) => place({ slot: e.target.value })}
              >
                <option value="">{t('skills.noSlot')}</option>
                {[
                  ...new Set([
                    ...SLOTS,
                    ...(group.slot == null ? [] : [group.slot]),
                  ]),
                ].map((s) => (
                  <option key={s} value={s}>
                    {s}
                  </option>
                ))}
              </select>
            </label>
            <button
              type="button"
              className="danger"
              onClick={() =>
                void dispatch(
                  run({ 'remove-socket-group': { group: index } }),
                ).then((o) => {
                  if (o !== null) {
                    dispatch(
                      actions.announce(
                        t('skills.groupRemoved', { group: title }),
                      ),
                    )
                  }
                })
              }
            >
              {t('skills.removeGroup')}
            </button>
          </div>
        </details>
      </div>
      <GemPicker
        open={picking}
        onOpenChange={setPicking}
        group={index}
        title={title}
      />
    </section>
  )
}

interface Placed {
  group: SkillGroupView
  index: number
}

function SlotCard({
  slot,
  item,
  groups,
}: {
  slot: string | null
  item: ItemSummary | undefined
  groups: Placed[]
}) {
  const dispatch = useAppDispatch()
  const id = useId()
  const filled = new Set(
    groups.flatMap(({ group }) =>
      group.gems.map((g) => g.socket).filter((s) => s != null),
    ),
  )
  return (
    <section className="panel slot-card" aria-labelledby={id}>
      <div className="panel-head">
        <div>
          <h2 id={id}>{slot ?? t('skills.other')}</h2>
          <span className="sub">
            {slot == null
              ? t('skills.otherSub')
              : item == null
                ? t('skills.noItem')
                : itemName(item)}
          </span>
        </div>
        <span className="spacer" />
        {item != null && <Sockets groups={item.sockets} filled={filled} />}
      </div>
      {groups.length === 0 && (
        <p className="muted small-text">{t('skills.slotEmpty')}</p>
      )}
      {groups.map(({ group, index }) => (
        <GroupBlock key={index} group={group} index={index} item={item} />
      ))}
      <button
        type="button"
        className="link top-gap"
        onClick={() =>
          void dispatch(
            run({ 'add-socket-group': { slot: slot ?? null, label: null } }),
          )
        }
      >
        {slot == null ? t('skills.addGroup') : t('skills.addGroupIn', { slot })}
      </button>
    </section>
  )
}

function SkillSetPicker({ sets }: { sets: SetView[] }) {
  const dispatch = useAppDispatch()
  const id = useId()
  const active = sets.find((s) => s.active)
  return (
    <div className="set-picker">
      <label htmlFor={id}>{t('skills.set')}</label>
      <select
        id={id}
        value={active?.id ?? ''}
        onChange={(e) =>
          void dispatch(
            run({ 'use-skill-set': { id: Number(e.target.value) } }),
          )
        }
      >
        {sets.map((s) => (
          <option key={s.id} value={s.id}>
            {cleanTitle(s.title) || t('skills.setN', { n: s.id })}
          </option>
        ))}
      </select>
    </div>
  )
}

/** The equipment slots a socket group can be in. */
const SLOTS = [
  'Weapon 1',
  'Weapon 2',
  'Helmet',
  'Body Armour',
  'Gloves',
  'Boots',
  'Amulet',
  'Ring 1',
  'Ring 2',
  'Belt',
  'Weapon 1 Swap',
  'Weapon 2 Swap',
]

/** The groups by slot, slots in the order their first group comes, then
 * the slots whose item in use has sockets and no group yet (to fill them);
 * groups without a slot last, always shown (to add one). */
function bySlot(
  groups: SkillGroupView[],
  socketed: string[],
): [string | null, Placed[]][] {
  const slots = new Map<string | null, Placed[]>()
  groups.forEach((group, index) => {
    const key = group.slot ?? null
    slots.set(key, [...(slots.get(key) ?? []), { group, index }])
  })
  for (const s of socketed) {
    if (!slots.has(s)) slots.set(s, [])
  }
  if (!slots.has(null)) slots.set(null, [])
  const entries = [...slots.entries()]
  return [
    ...entries.filter(([s]) => s != null),
    ...entries.filter(([s]) => s == null),
  ]
}

/** A text field that sends its value when the user leaves it or presses
 * Enter, and only when it changed. */
function TextField({
  value,
  label,
  onCommit,
}: {
  value: string
  label: string
  onCommit: (s: string) => void
}) {
  const [text, setText] = useState(value)
  const [seen, setSeen] = useState(value)
  if (seen !== value) {
    setSeen(value)
    setText(value)
  }
  const commit = () => {
    if (text.trim() !== value) onCommit(text.trim())
  }
  return (
    <input
      aria-label={label}
      value={text}
      onChange={(e) => setText(e.target.value)}
      onBlur={commit}
      onKeyDown={(e) => {
        if (e.key === 'Enter') commit()
      }}
    />
  )
}

/** The socket groups as a table, a row each (the brief's screen 10, D;
 * UX audit follow-up): what a build with many groups needs to be read at
 * a glance. A row opens to the group's full editor under it. */
function SkillTable({
  groups,
  inSlot,
}: {
  groups: SkillGroupView[]
  inSlot: (slot: string | null) => ItemSummary | undefined
}) {
  const dispatch = useAppDispatch()
  const [open, setOpen] = useState<number | null>(null)
  const names = (gems: GemView[]) =>
    gems.map((g, i) => (
      <span key={i} className={`gem-chip${g.enabled ? '' : ' muted'}`}>
        <span
          className={`gem-mark gem-${g.colour ?? 'unknown'}${g.support === true ? ' support' : ''}`}
          aria-hidden="true"
        />
        {g.main ? <strong>{g.name ?? '?'}</strong> : (g.name ?? '?')}{' '}
        <span className="muted">{g.level}</span>
      </span>
    ))
  return (
    <div className="panel table-scroll">
      <table className="data-table skill-table">
        <caption className="visually-hidden">
          {t('skills.table.caption')}
        </caption>
        <thead>
          <tr>
            <th scope="col">{t('skills.table.group')}</th>
            <th scope="col">{t('skills.table.skills')}</th>
            <th scope="col">{t('skills.table.supports')}</th>
            <th scope="col">{t('skills.colOn')}</th>
            <th scope="col">
              <span className="visually-hidden">{t('skills.view')}</span>
            </th>
          </tr>
        </thead>
        <tbody>
          {groups.map((group, index) => {
            const title =
              group.label != null && group.label !== ''
                ? group.label
                : t('skills.groupN', { n: index + 1 })
            const actives = group.gems.filter((g) => g.support !== true)
            const supports = group.gems.filter((g) => g.support === true)
            const shown = open === index
            return (
              <Fragment key={index}>
                <tr
                  className={`${group.enabled ? '' : 'disabled'}${group.main ? ' main' : ''}`}
                >
                  <th scope="row">
                    <span className="skill-table-title">{title}</span>
                    <span className="muted small-text">
                      {group.slot ?? t('skills.other')}
                    </span>
                    {group.main && (
                      <Badge tone="info">{t('skills.table.mainMark')}</Badge>
                    )}
                  </th>
                  <td>
                    <span className="gem-chips">
                      {actives.length === 0 ? (
                        <span className="muted">{t('skills.empty')}</span>
                      ) : (
                        names(actives)
                      )}
                    </span>
                  </td>
                  <td>
                    <span className="gem-chips">
                      {supports.length === 0 ? (
                        <span className="muted">
                          {t('skills.table.noSupports')}
                        </span>
                      ) : (
                        names(supports)
                      )}
                    </span>
                  </td>
                  <td>
                    <input
                      type="checkbox"
                      aria-label={t('skills.groupEnabledOf', { group: title })}
                      checked={group.enabled}
                      onChange={(e) =>
                        void dispatch(
                          run({
                            'edit-socket-group': {
                              group: index,
                              enabled: e.target.checked,
                            },
                          }),
                        )
                      }
                    />
                  </td>
                  <td>
                    <button
                      type="button"
                      className="icon quiet"
                      aria-expanded={shown}
                      aria-label={t(
                        shown ? 'skills.table.close' : 'skills.table.open',
                        { name: title },
                      )}
                      title={t(
                        shown ? 'skills.table.close' : 'skills.table.open',
                        { name: title },
                      )}
                      onClick={() => setOpen(shown ? null : index)}
                    >
                      <Icon name={shown ? 'close' : 'sliders'} size={16} />
                    </button>
                  </td>
                </tr>
                {shown && (
                  <tr className="skill-table-editor">
                    <td colSpan={5}>
                      <GroupBlock
                        group={group}
                        index={index}
                        item={inSlot(group.slot ?? null)}
                      />
                    </td>
                  </tr>
                )}
              </Fragment>
            )
          })}
        </tbody>
      </table>
    </div>
  )
}

export function Skills() {
  const dispatch = useAppDispatch()
  const build = useAppSelector((s) => s.session.snapshot?.build)
  const prefs = useAppSelector((s) => s.ui.prefs)
  if (build == null) return null
  const table = prefs.skillsTable === true
  const groups = build['skill-groups']
  const sets = build['skill-sets']
  const inSlot = (slot: string | null) =>
    slot == null
      ? undefined
      : build.items.find((i) => i['in-use'] && i.slot === slot)
  return (
    <>
      <div className="page-head">
        <ScreenHeading>{t('skills.title')}</ScreenHeading>
        <Hint>{t('skills.help')}</Hint>
        <span className="spacer" />
        <div className="segmented" role="group" aria-label={t('skills.view')}>
          {([false, true] as const).map((on) => (
            <button
              type="button"
              key={String(on)}
              aria-pressed={table === on}
              onClick={() => {
                const { skillsTable: _, ...rest } = prefs
                void dispatch(
                  setPrefs(on ? { ...rest, skillsTable: true } : rest),
                )
              }}
            >
              {t(on ? 'skills.view.table' : 'skills.view.cards')}
            </button>
          ))}
        </div>
        {sets.length > 1 && <SkillSetPicker sets={sets} />}
      </div>
      {groups.length === 0 && <p>{t('skills.none')}</p>}
      {table && groups.length > 0 ? (
        <SkillTable groups={groups} inSlot={inSlot} />
      ) : (
        <div className="skill-grid">
          {bySlot(
            groups,
            build.items
              .filter((i) => i['in-use'] && i.sockets.length > 0)
              .flatMap((i) => (i.slot == null ? [] : [i.slot])),
          ).map(([slot, placed]) => (
            <SlotCard
              key={slot ?? ''}
              slot={slot}
              item={inSlot(slot)}
              groups={placed}
            />
          ))}
        </div>
      )}
    </>
  )
}
