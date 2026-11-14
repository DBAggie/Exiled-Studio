// The equipment panel (B-2): the active item set laid out as the game's
// inventory, with the weapon swap (I / II), and the game's hover tooltip
// over each item; holding Alt (or the "Details" switch) shows each roll's
// range and the item level, as the game does.
import { useEffect, useLayoutEffect, useRef, useState } from 'react'

import type { ItemSummary, ItemView, LineView } from '../api/types'
import { t } from '../i18n'
import { actions, attempt, run } from '../store'
import { Sprite } from '../sprites'
import { type SpriteKind, spriteFor } from '../spriteKind'
import { itemName, useAppDispatch, useAppSelector } from '../util'

/** Item classes that take both hands: the off hand shows them faded. */
const TWO_HANDED = new Set([
  'Bow',
  'Staff',
  'Warstaff',
  'Two Hand Axe',
  'Two Hand Mace',
  'Two Hand Sword',
  'FishingRod',
])

/** The panel's slots: the slot name in the first weapon set, its grid
 * area, and the sprite of an empty slot. */
const SLOTS: { slot: string; area: string; empty: SpriteKind }[] = [
  { slot: 'Weapon 1', area: 'w1', empty: 'one-hand' },
  { slot: 'Weapon 2', area: 'w2', empty: 'shield' },
  { slot: 'Helmet', area: 'helm', empty: 'helmet' },
  { slot: 'Body Armour', area: 'body', empty: 'body' },
  { slot: 'Gloves', area: 'glove', empty: 'gloves' },
  { slot: 'Boots', area: 'boot', empty: 'boots' },
  { slot: 'Amulet', area: 'amul', empty: 'amulet' },
  { slot: 'Ring 1', area: 'ring1', empty: 'ring' },
  { slot: 'Ring 2', area: 'ring2', empty: 'ring' },
  { slot: 'Belt', area: 'belt', empty: 'belt' },
  { slot: 'Flask 1', area: 'f1', empty: 'flask' },
  { slot: 'Flask 2', area: 'f2', empty: 'flask' },
  { slot: 'Flask 3', area: 'f3', empty: 'flask' },
  { slot: 'Flask 4', area: 'f4', empty: 'flask' },
  { slot: 'Flask 5', area: 'f5', empty: 'flask' },
]

/** The slot an item of the panel sits in, with the weapon swap. */
function slotName(slot: string, second: boolean): string {
  return second && slot.startsWith('Weapon') ? `${slot} Swap` : slot
}

/** Which tooltip is open, and where its item is on screen. */
interface Hover {
  id: number
  rect: DOMRect
}

/** The game's hover tooltip for item `id`, next to `rect`. */
function Tooltip({ hover, details }: { hover: Hover; details: boolean }) {
  const dispatch = useAppDispatch()
  const version = useAppSelector((s) => s.session.snapshot?.version)
  const [item, setItem] = useState<ItemView | null>(null)
  const ref = useRef<HTMLDivElement>(null)
  const [place, setPlace] = useState({ left: 0, top: 0 })
  useEffect(() => {
    let live = true
    void dispatch(attempt((api) => api.item(hover.id))).then((v) => {
      if (live && v != null) setItem(v)
    })
    return () => {
      live = false
    }
  }, [dispatch, hover.id, version])
  // Beside the item, kept on screen (the game's placement).
  useLayoutEffect(() => {
    const box = ref.current?.getBoundingClientRect()
    if (box == null) return
    const gap = 8
    let left = hover.rect.right + gap
    if (left + box.width > window.innerWidth - gap) {
      left = Math.max(gap, hover.rect.left - box.width - gap)
    }
    const top = Math.min(
      Math.max(gap, hover.rect.top),
      Math.max(gap, window.innerHeight - box.height - gap),
    )
    setPlace({ left, top })
  }, [hover.rect, item, details])
  if (item == null || item.id !== hover.id) return null
  return (
    <div
      ref={ref}
      className={`game-tooltip rarity-frame-${item.rarity?.toLowerCase() ?? 'normal'}`}
      role="tooltip"
      id="item-tooltip"
      style={{ left: place.left, top: place.top }}
    >
      <TooltipBody item={item} details={details} />
    </div>
  )
}

/** The requirement line's parts, as the game prints them: the level,
 * then attributes above 0. */
function requirementParts(
  c: ItemView['card'],
): { label: 'level' | 'str' | 'dex' | 'int'; value: string }[] {
  const r = c.requirements
  const level =
    r?.level != null
      ? String(r.level)
      : c['level-req'] != null && c['level-req'] !== '0'
        ? c['level-req']
        : null
  const parts: { label: 'level' | 'str' | 'dex' | 'int'; value: string }[] = []
  if (level != null && level !== '0')
    parts.push({ label: 'level', value: level })
  if (r != null) {
    if (r.strength > 0) parts.push({ label: 'str', value: String(r.strength) })
    if (r.dexterity > 0)
      parts.push({ label: 'dex', value: String(r.dexterity) })
    if (r.intelligence > 0) {
      parts.push({ label: 'int', value: String(r.intelligence) })
    }
  }
  return parts
}

/** A mod's Alt header: `Prefix Modifier "Urchin's" (Tier: 2) — Life,
 * Defences`, the parts the data has. */
function affixHeader(a: ItemView['card']['affixes'][number]): string {
  let out = t(a.kind === 'prefix' ? 'tooltip.prefix' : 'tooltip.suffix')
  if (a.name != null) out += ` “${a.name}”`
  if (a.tier != null) out += ` ${t('tooltip.tier', { n: a.tier })}`
  if (a.tags.length > 0) out += ` — ${a.tags.join(', ')}`
  return out
}

/** A line's colour class as the game prints it. */
function lineTone(line: LineView): string {
  if (line.marks.includes('fractured')) return 'tt-fractured'
  if (line.marks.includes('crafted')) return 'tt-crafted'
  if (line.section === 'enchant') return 'tt-enchant'
  return 'tt-mod'
}

/** The tooltip's text: the game's order and blocks. */
export function TooltipBody({
  item,
  details,
}: {
  item: ItemView
  details: boolean
}) {
  const c = item.card
  const rarity = item.rarity?.toLowerCase() ?? 'normal'
  const twoLines =
    (rarity === 'rare' || rarity === 'unique') &&
    item.base != null &&
    item.name != null &&
    item.name !== item.base
  const mods = (section: string) =>
    item.lines.filter(
      (l) =>
        l.kind === 'modifier' &&
        l.section === section &&
        l.status !== 'excluded',
    )
  const lineText = (l: LineView) =>
    (details ? (l.ranged ?? l.shown) : l.shown) ?? l.text
  const blocks: React.ReactNode[] = []
  const props = [
    c['item-class'] != null && <p key="class">{c['item-class']}</p>,
    c.quality != null && (
      <p key="q">
        {t('tooltip.quality')}{' '}
        <span className="tt-augmented">+{c.quality}%</span>
      </p>
    ),
    ...c.properties
      .filter((p) => p.name !== 'Requires Class')
      .map((p, i) => (
        <p key={`p${i}`}>
          {p.name}: <span className="tt-value">{p.value}</span>
        </p>
      )),
  ].filter(Boolean)
  if (props.length > 0) blocks.push(props)
  if (details && c['item-level'] != null) {
    blocks.push(
      <p key="ilvl">
        {t('tooltip.itemLevel')}{' '}
        <span className="tt-value">{c['item-level']}</span>
      </p>,
    )
  }
  const req = requirementParts(c)
  if (req.length > 0) {
    blocks.push(
      <p key="req">
        {t('tooltip.requires')}{' '}
        {req.map((part, i) => (
          <span key={part.label}>
            {i > 0 && ', '}
            {part.label === 'level' ? (
              <>
                {t('tooltip.level')}{' '}
                <span className="tt-value">{part.value}</span>
              </>
            ) : (
              <>
                <span className="tt-value">{part.value}</span>{' '}
                {t(`tooltip.${part.label}`)}
              </>
            )}
          </span>
        ))}
      </p>,
    )
  }
  const line = (l: LineView, key: string | number = l.index) => (
    <p key={key} className={lineTone(l)}>
      {lineText(l)}
    </p>
  )
  for (const section of ['enchant', 'implicit', 'explicit']) {
    const lines = mods(section)
    if (lines.length === 0) continue
    // Alt: each named mod's lines under its header, the rest after.
    if (details && section === 'explicit' && c.affixes.length > 0) {
      const byIndex = new Map(lines.map((l) => [l.index, l]))
      const headed = new Set(c.affixes.flatMap((a) => a.lines))
      blocks.push([
        ...c.affixes.flatMap((a, ai) => [
          <p key={`h${ai}`} className="tt-affix">
            {affixHeader(a)}
          </p>,
          ...a.lines
            .map((i) => byIndex.get(i))
            .filter((l): l is LineView => l != null)
            .map((l) => line(l, `${ai}-${l.index}`)),
        ]),
        ...lines.filter((l) => !headed.has(l.index)).map((l) => line(l)),
      ])
      continue
    }
    blocks.push(lines.map((l) => line(l)))
  }
  const flags = [
    ...c.influences.map((i) => t('card.influence', { name: i })),
    ...c.flags,
  ]
  if (flags.length > 0) {
    blocks.push(
      flags.map((f) => (
        <p key={f} className={/corrupt/i.test(f) ? 'tt-corrupted' : 'tt-flag'}>
          {f}
        </p>
      )),
    )
  }
  return (
    <>
      <div className={`tt-header rarity-frame-${rarity}`}>
        <p className="tt-name">{itemName(item)}</p>
        {twoLines && <p className="tt-name">{item.base}</p>}
      </div>
      {blocks.map((b, i) => (
        <div className="tt-block" key={i}>
          {b}
        </div>
      ))}
    </>
  )
}

/** Slots two cells wide: their sockets zigzag two per row, as in the
 * game (left to right, then right to left). */
const WIDE = new Set(['w1', 'w2', 'helm', 'body', 'glove', 'boot'])

/** A socket's colour token by its letter. */
const SOCKET_FILL: Record<string, string> = {
  R: 'var(--c-gem-red)',
  G: 'var(--c-gem-green)',
  B: 'var(--c-gem-blue)',
  W: 'var(--c-gem-white)',
  A: 'var(--c-tt-label)',
}

/** An item's sockets drawn over it as the game lays them out: two per row
 * zigzagging on a wide item, one row on a belt, else one column; a bar
 * between linked sockets. Filled sockets (a gem in them) are solid. */
function InvSockets({
  groups,
  filled,
  area,
}: {
  groups: string[]
  filled: ReadonlySet<number>
  area: string
}) {
  const sockets = groups.flatMap((g, gi) =>
    [...g].map((c, ci) => ({ c, linked: ci > 0, gi })),
  )
  const n = sockets.length
  const at = (i: number): [number, number] => {
    if (area === 'belt') return [i, 0]
    if (!WIDE.has(area)) return [0, i]
    const row = Math.floor(i / 2)
    const col = row % 2 === 0 ? i % 2 : 1 - (i % 2)
    return [col, row]
  }
  const cols = area === 'belt' ? n : WIDE.has(area) ? Math.min(2, n) : 1
  const rows = area === 'belt' ? 1 : WIDE.has(area) ? Math.ceil(n / 2) : n
  const step = 26
  const centre = (i: number) => {
    const [x, y] = at(i)
    return [x * step + step / 2, y * step + step / 2] as const
  }
  return (
    <svg
      className="inv-socket-grid"
      viewBox={`0 0 ${cols * step} ${rows * step}`}
      style={{ width: `${cols * 2.3}rem` }}
      aria-hidden="true"
      focusable="false"
    >
      {sockets.map((s, i) => {
        if (!s.linked) return null
        const [x1, y1] = centre(i - 1)
        const [x2, y2] = centre(i)
        return (
          <line
            key={`l${i}`}
            x1={x1}
            y1={y1}
            x2={x2}
            y2={y2}
            className="inv-link"
          />
        )
      })}
      {sockets.map((s, i) => {
        const [cx, cy] = centre(i)
        const fill = SOCKET_FILL[s.c.toUpperCase()] ?? 'var(--c-tt-label)'
        return (
          <circle
            key={i}
            cx={cx}
            cy={cy}
            r={9.5}
            className={filled.has(i) ? 'inv-socket filled' : 'inv-socket'}
            style={{ stroke: fill, fill }}
          />
        )
      })}
    </svg>
  )
}

/** One slot of the panel: its item (sprite, sockets, abyssal jewels), the
 * faded two-handed weapon in the off hand, or empty. */
function Slot({
  area,
  label,
  item,
  shadow,
  empty,
  jewels,
  filled,
  onHover,
}: {
  area: string
  label: string
  item: ItemSummary | undefined
  shadow: ItemSummary | undefined
  empty: SpriteKind
  jewels: ItemSummary[]
  filled: ReadonlySet<number>
  onHover: (h: Hover | null) => void
}) {
  const dispatch = useAppDispatch()
  const hover = (id: number) => (e: { currentTarget: Element }) =>
    onHover({ id, rect: e.currentTarget.getBoundingClientRect() })
  const leave = () => onHover(null)
  if (item == null) {
    return (
      <div className="inv-cell" style={{ gridArea: area }}>
        <div
          className={`inv-slot inv-empty${shadow != null ? ' inv-shadow' : ''}`}
          role="img"
          aria-label={
            shadow != null
              ? t('inventory.twoHanded', {
                  slot: label,
                  name: itemName(shadow),
                })
              : t('inventory.empty', { slot: label })
          }
        >
          <Sprite
            kind={
              shadow != null ? spriteFor(shadow['item-class'], label) : empty
            }
            itemClass={shadow?.['item-class']}
          />
        </div>
      </div>
    )
  }
  return (
    <div className="inv-cell" style={{ gridArea: area }}>
      <button
        type="button"
        className={`inv-slot rarity-frame-${item.rarity?.toLowerCase() ?? 'normal'}`}
        aria-label={t('inventory.slot', { slot: label, name: itemName(item) })}
        aria-describedby="item-tooltip"
        onMouseEnter={hover(item.id)}
        onFocus={hover(item.id)}
        onMouseLeave={leave}
        onBlur={leave}
        onClick={() => dispatch(actions.editItem(item.id))}
      >
        <Sprite
          kind={spriteFor(item['item-class'], label)}
          itemClass={item['item-class']}
        />
        {item.sockets.length > 0 && (
          <span className="inv-sockets" aria-hidden="true">
            <InvSockets groups={item.sockets} filled={filled} area={area} />
          </span>
        )}
      </button>
      {jewels.length > 0 && (
        <span className="inv-jewels">
          {jewels.map((j) => (
            <button
              type="button"
              key={j.id}
              className={`inv-jewel rarity-frame-${j.rarity?.toLowerCase() ?? 'normal'}`}
              aria-label={t('inventory.abyssal', {
                slot: label,
                name: itemName(j),
              })}
              aria-describedby="item-tooltip"
              onMouseEnter={hover(j.id)}
              onFocus={hover(j.id)}
              onMouseLeave={leave}
              onBlur={leave}
              onClick={() => dispatch(actions.editItem(j.id))}
            >
              <Sprite kind="jewel" />
            </button>
          ))}
        </span>
      )}
    </div>
  )
}

/** The weapon swap's I / II switch, above each weapon. */
function SwapSwitch({ second, area }: { second: boolean; area: string }) {
  const dispatch = useAppDispatch()
  const pick = (want: boolean) => () => {
    if (want !== second) {
      void dispatch(run({ 'use-weapon-set': { second: want } }))
    }
  }
  return (
    <div
      className="inv-swap"
      style={{ gridArea: area }}
      role="group"
      aria-label={t('inventory.weaponSet')}
    >
      <button type="button" aria-pressed={!second} onClick={pick(false)}>
        I
      </button>
      <button type="button" aria-pressed={second} onClick={pick(true)}>
        II
      </button>
    </div>
  )
}

export function Inventory() {
  const build = useAppSelector((s) => s.session.snapshot?.build)
  const [hover, setHover] = useState<Hover | null>(null)
  const [altHeld, setAltHeld] = useState(false)
  const [details, setDetails] = useState(false)
  // Alt shows the details while held, as in the game.
  useEffect(() => {
    const down = (e: KeyboardEvent) => {
      if (e.key === 'Alt') {
        e.preventDefault()
        setAltHeld(true)
      }
    }
    const up = (e: KeyboardEvent) => {
      if (e.key === 'Alt') setAltHeld(false)
    }
    const blur = () => setAltHeld(false)
    window.addEventListener('keydown', down)
    window.addEventListener('keyup', up)
    window.addEventListener('blur', blur)
    return () => {
      window.removeEventListener('keydown', down)
      window.removeEventListener('keyup', up)
      window.removeEventListener('blur', blur)
    }
  }, [])
  if (build == null) return null
  const second = build['second-weapon-set']
  const bySlot = new Map(
    build.items
      .filter((i) => i.slot != null)
      .map((i) => [i.slot as string, i] as const),
  )
  const groups = build['skill-groups']
  const filledIn = (slot: string) =>
    new Set(
      groups
        .filter((g) => g.slot === slot)
        .flatMap((g) => g.gems.map((x) => x.socket))
        .filter((s) => s != null),
    )
  const main = bySlot.get(slotName('Weapon 1', second))
  const twoHanded = TWO_HANDED.has(main?.['item-class'] ?? '')
  return (
    <section className="panel inventory-panel" aria-labelledby="inv-title">
      <div className="panel-head">
        <h2 id="inv-title">{t('inventory.title')}</h2>
        <span className="spacer" />
        <label className="check">
          <input
            type="checkbox"
            checked={details}
            onChange={(e) => setDetails(e.target.checked)}
          />
          {t('inventory.details')}
        </label>
      </div>
      <div className="inventory">
        <SwapSwitch second={second} area="s1" />
        <SwapSwitch second={second} area="s2" />
        {SLOTS.map((s) => {
          const slot = slotName(s.slot, second)
          const item = bySlot.get(slot)
          // The game's slot names (`Body Armour`), as PoB writes them.
          const label = s.slot
          return (
            <Slot
              key={s.slot}
              area={s.area}
              label={label}
              item={item}
              shadow={
                s.slot === 'Weapon 2' && item == null && twoHanded
                  ? main
                  : undefined
              }
              empty={s.empty}
              jewels={build.items.filter((i) =>
                i.slot?.startsWith(`${slot} Abyssal Socket`),
              )}
              filled={item == null ? new Set() : filledIn(slot)}
              onHover={setHover}
            />
          )
        })}
        {/* The game's square left of the flasks: shown, not used yet. */}
        <div
          className="inv-slot inv-empty inv-spare"
          style={{ gridArea: 'spare' }}
          aria-hidden="true"
        />
      </div>
      {hover != null && <Tooltip hover={hover} details={details || altHeld} />}
    </section>
  )
}
