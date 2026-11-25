// What a number does not count yet, for people (the dashboard cards
// redesign, maintainer 2026-11-24): a card says it in one line with a
// Details button; Details groups the reasons by kind with counts, names
// each by the game's text (a line, a passive, a gem, an option; never a
// stat ID) with a link to where it is, and keeps the calculation's codes
// behind "Show technical detail". The breakdown page shows the same
// groups.
import { Dialog } from 'radix-ui'
import { useEffect, useMemo, useState } from 'react'

import type { BuildView, FieldResult, PassiveLines } from './api/types'
import { plain } from './colourCodes'
import { optionOf } from './configOptions'
import { fieldLabel, t, tDynamic, type MessageKey } from './i18n'
import {
  byGroup,
  codeCounts,
  entriesOf,
  passiveLine,
  type ReasonEntry,
  type ReasonGroup,
} from './reasons'
import { actions, query } from './store'
import { reasonText, useAppDispatch, useAppSelector } from './util'

const GROUP_LABEL: Record<ReasonGroup, MessageKey> = {
  lines: 'reasons.group.lines',
  stats: 'reasons.group.stats',
  config: 'reasons.group.config',
  content: 'reasons.group.content',
  other: 'reasons.group.other',
}

/** How many things, in words. */
function countText(n: number): string {
  return n === 1 ? t('reasons.one') : t('reasons.count', { n })
}

/** One row of a group: reasons with the same name in the same place. */
interface Row {
  key: string
  name: string
  where: string
  go: (() => void) | null
  n: number
}

type Lines = ReadonlyMap<number, readonly string[]>

/** The lines of the items the entries point into, as the game prints
 * them, by item and line index. */
function useItemLines(entries: ReasonEntry[]): Lines {
  const dispatch = useAppDispatch()
  const version = useAppSelector((s) => s.session.snapshot?.version)
  const [lines, setLines] = useState<Lines>(new Map())
  const ids = [
    ...new Set(
      entries.flatMap((e) =>
        e.target.kind === 'item' && e.target.line !== null
          ? [e.target.item]
          : [],
      ),
    ),
  ]
    .sort((a, b) => a - b)
    .join(',')
  useEffect(() => {
    if (ids === '') return
    let live = true
    for (const id of ids.split(',').map(Number)) {
      void dispatch(query((api) => api.item(id))).then((v) => {
        if (!live || v === null) return
        const shown = v.lines.map((l) => l.shown ?? l.text)
        setLines((m) => new Map(m).set(id, shown))
      })
    }
    return () => {
      live = false
    }
  }, [dispatch, ids, version])
  return lines
}

/** The lines of the passives the entries name a stat of, each with the
 * stats it prints (asked once per set of passives and data). */
function usePassiveLines(entries: ReasonEntry[]): PassiveLines[] {
  const dispatch = useAppDispatch()
  const data = useAppSelector(
    (s) => s.session.snapshot?.build?.['data-version'],
  )
  const [passives, setPassives] = useState<PassiveLines[]>([])
  const nodes = [
    ...new Set(
      entries.flatMap((e) =>
        e.target.kind === 'passive' && e.detail !== null ? [e.target.node] : [],
      ),
    ),
  ]
    .sort((a, b) => a - b)
    .join(',')
  useEffect(() => {
    if (nodes === '') return
    let live = true
    void dispatch(
      query((api) => api.passiveLines(nodes.split(',').map(Number))),
    ).then((v) => {
      if (live && v !== null) setPassives(v)
    })
    return () => {
      live = false
    }
  }, [dispatch, nodes, data])
  return passives
}

/** The rows of a group: each entry named and placed, the same ones
 * together. */
function rowsOf(
  entries: ReasonEntry[],
  build: BuildView | null | undefined,
  lines: Lines,
  passives: PassiveLines[],
  go: (to: () => void) => () => void,
  dispatch: ReturnType<typeof useAppDispatch>,
): Row[] {
  const rows = new Map<string, Row>()
  for (const e of entries) {
    let name = reasonText(e.code)
    let where = ''
    let to: (() => void) | null = null
    const target = e.target
    switch (target.kind) {
      case 'item': {
        const item = build?.items.find((i) => i.id === target.item)
        where = item?.name ?? item?.base ?? t('reasons.item')
        to = () => dispatch(actions.editItem(target.item))
        name =
          target.line !== null
            ? (lines.get(target.item)?.[target.line] ??
              t('reasons.lineN', { n: target.line + 1 }))
            : tDynamic(`reasons.what.${e.detail ?? ''}`, 'reasons.whatItem')
        break
      }
      case 'passive': {
        const passive = build?.['passive-names'].find(
          (p) => p.node === target.node,
        )?.name
        // A modifier of a passive is named by its own line, with the
        // passive as its place (RV-11); until the lines are known, or for
        // a reason about the whole passive, by the passive.
        const line = passiveLine(passives, target.node, e.detail)
        name =
          line?.text ??
          passive ??
          tDynamic(`reasons.what.${e.detail ?? ''}`, 'reasons.passive')
        where = line === null ? t('nav.tree') : line.passive
        to = () => dispatch(actions.go('tree'))
        break
      }
      case 'gem': {
        const group = build?.['skill-groups'][target.group]
        name =
          group?.gems[target.gem]?.name ??
          t('reasons.gemN', { n: target.gem + 1 })
        where = t('nav.skills')
        to = () => dispatch(actions.go('skills'))
        break
      }
      case 'config':
        name = plain(optionOf(target.input)?.label ?? target.input)
        where = t('nav.config')
        to = () => dispatch(actions.go('config'))
        break
      case 'custom':
        name =
          build?.config?.blocks[target.block]?.lines[target.line]?.text ??
          t('reasons.lineN', { n: target.line + 1 })
        where = t('config.blocks')
        to = () => dispatch(actions.go('config'))
        break
      default:
    }
    const key = `${name}\u0000${where}`
    const row = rows.get(key)
    if (row === undefined) {
      rows.set(key, { key, name, where, go: to && go(to), n: 1 })
    } else {
      row.n += 1
    }
  }
  return [...rows.values()]
}

/** The reasons grouped by kind, each group folded with its count. */
export function ReasonGroups({
  entries,
  onLeave,
}: {
  entries: ReasonEntry[]
  /** Called before a row's link leaves the page (a dialog closes). */
  onLeave?: () => void
}) {
  const dispatch = useAppDispatch()
  const build = useAppSelector((s) => s.session.snapshot?.build)
  const lines = useItemLines(entries)
  const passives = usePassiveLines(entries)
  const groups = useMemo(() => byGroup(entries), [entries])
  const go = (to: () => void) => () => {
    onLeave?.()
    to()
  }
  return (
    <div className="reason-groups">
      {groups.map((g) => (
        <details key={g.group} className="reason-group">
          <summary>
            <span>{t(GROUP_LABEL[g.group])}</span>
            <span className="reason-count">{g.entries.length}</span>
          </summary>
          <ul>
            {rowsOf(g.entries, build, lines, passives, go, dispatch).map(
              (row) => (
                <li key={row.key}>
                  <span className="reason-name">
                    {row.name}
                    {row.n > 1 && (
                      <span className="muted">
                        {' '}
                        {t('reasons.times', { n: row.n })}
                      </span>
                    )}
                  </span>
                  {row.go !== null && (
                    <button type="button" className="link" onClick={row.go}>
                      {row.where}
                    </button>
                  )}
                </li>
              ),
            )}
          </ul>
        </details>
      ))}
      <details className="reason-codes">
        <summary>{t('review.showDetail')}</summary>
        <p className="error-code">
          {codeCounts(entries)
            .map(([code, n]) => `${code} ${n}`)
            .join(' · ')}
        </p>
        <ul className="error-code">
          {[
            ...new Set(
              entries
                .filter((e) => e.detail !== null)
                .map((e) => `${e.code}: ${e.detail}`),
            ),
          ].map((text) => (
            <li key={text}>{text}</li>
          ))}
        </ul>
      </details>
    </div>
  )
}

/** A card's one line about what its value leaves out, with Details. */
export function ReasonSummary({ field }: { field: FieldResult }) {
  const [open, setOpen] = useState(false)
  const entries = useMemo(() => entriesOf(field), [field])
  const first = entries[0]
  if (first === undefined) return null
  // One reason about no particular thing says itself.
  if (entries.length === 1 && first.target.kind === 'none') {
    return <p className="reason-summary">{reasonText(first.code)}</p>
  }
  const label = fieldLabel(field.field)
  return (
    <p className="reason-summary">
      {countText(entries.length)}
      {' · '}
      <Dialog.Root open={open} onOpenChange={setOpen}>
        <Dialog.Trigger asChild>
          <button
            type="button"
            className="link"
            aria-label={t('reasons.detailsFor', { field: label })}
          >
            {t('reasons.details')}
          </button>
        </Dialog.Trigger>
        <Dialog.Portal>
          <Dialog.Overlay className="dialog-overlay" />
          <Dialog.Content className="dialog-content dialog-wide">
            <div className="panel-head">
              <Dialog.Title>
                {t('reasons.title', {
                  field: label,
                  count: countText(entries.length),
                })}
              </Dialog.Title>
              <span className="spacer" />
              <Dialog.Close asChild>
                <button type="button">{t('dialog.close')}</button>
              </Dialog.Close>
            </div>
            <Dialog.Description className="muted">
              {t('reasons.help')}
            </Dialog.Description>
            <ReasonGroups entries={entries} onLeave={() => setOpen(false)} />
          </Dialog.Content>
        </Dialog.Portal>
      </Dialog.Root>
    </p>
  )
}
