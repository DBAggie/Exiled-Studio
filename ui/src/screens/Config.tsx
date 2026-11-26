// Configuration: the active config set's inputs (PoB's configuration
// options: the enemy, charges, conditions) and its custom modifier
// blocks, all editable. Each input says what the calculation does with
// it: applied (no mark), no effect on any number, or not calculated yet
// (the numbers are partial). Every change is one undoable command.
import { useId, useLayoutEffect, useRef, useState } from 'react'

import type {
  ConfigInputView,
  ConfigValueView,
  CustomBlockView,
  SetView,
} from '../api/types'
import { Hint, Icon, ScreenHeading, StateMark } from '../components'
import {
  initialValue,
  matches,
  OPTIONS,
  optionOf,
  relevant,
  SOURCE_TAG,
  type ConfigOption,
} from '../configOptions'
import { LeaguePicker } from './Items'
import { t, tDynamic } from '../i18n'
import { run } from '../store'
import { cleanTitle, inputLabel, useAppDispatch, useAppSelector } from '../util'

type Group = 'enemy' | 'charges' | 'conditions' | 'other'

const GROUPS: Group[] = ['enemy', 'charges', 'conditions', 'other']

function groupOf(name: string): Group {
  if (/^enemy|Enemy/.test(name)) return 'enemy'
  if (/Charge/.test(name)) return 'charges'
  if (name.startsWith('condition')) return 'conditions'
  return 'other'
}

/** PoB's legacy custom modifier input: its text is lines, shown here and
 * not edited as an input. */
const LEGACY = 'customMods'

function NumberInput({
  value,
  label,
  onCommit,
}: {
  value: number
  label: string
  onCommit: (n: number) => void
}) {
  const [text, setText] = useState(String(value))
  const [seen, setSeen] = useState(value)
  if (seen !== value) {
    setSeen(value)
    setText(String(value))
  }
  const commit = () => {
    const n = Number(text)
    if (text.trim() !== '' && Number.isFinite(n) && n !== value) onCommit(n)
    else setText(String(value))
  }
  return (
    <input
      type="number"
      className="config-number"
      aria-label={label}
      value={text}
      step="any"
      onChange={(e) => setText(e.target.value)}
      onBlur={commit}
      onKeyDown={(e) => {
        if (e.key === 'Enter') commit()
      }}
    />
  )
}

function TextInput({
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
    if (text !== value) onCommit(text)
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

/** A PoB list option's choices; the current value stays a choice even
 * when PoB's list does not have it. */
function ChoiceEditor({
  option,
  value,
  label,
  onSet,
}: {
  option: ConfigOption
  value: ConfigValueView
  label: string
  onSet: (v: ConfigValueView) => void
}) {
  const choices = option.list ?? []
  const current =
    'number' in value
      ? String(value.number)
      : 'text' in value
        ? value.text
        : String(value.boolean)
  const known = choices.some((c) => String(c.val) === current)
  return (
    <select
      aria-label={label}
      value={current}
      onChange={(e) => {
        const c = choices.find((x) => String(x.val) === e.target.value)
        if (c == null) return
        onSet(typeof c.val === 'number' ? { number: c.val } : { text: c.val })
      }}
    >
      {!known && <option value={current}>{current}</option>}
      {choices.map((c) => (
        <option key={String(c.val)} value={String(c.val)}>
          {c.label}
        </option>
      ))}
    </select>
  )
}

function ValueEditor({
  input,
  label,
  onSet,
}: {
  input: ConfigInputView
  label: string
  onSet: (v: ConfigValueView) => void
}) {
  const v = input.value
  const option = optionOf(input.name)
  if (option?.type === 'list') {
    return (
      <ChoiceEditor option={option} value={v} label={label} onSet={onSet} />
    )
  }
  if ('boolean' in v) {
    return (
      <input
        type="checkbox"
        aria-label={label}
        checked={v.boolean}
        onChange={(e) => onSet({ boolean: e.target.checked })}
      />
    )
  }
  if ('number' in v) {
    return (
      <NumberInput
        value={v.number}
        label={label}
        onCommit={(n) => onSet({ number: n })}
      />
    )
  }
  if (input.name === LEGACY) {
    return <span className="muted small-text">{t('config.legacy')}</span>
  }
  return (
    <TextInput
      value={v.text}
      label={label}
      onCommit={(s) => onSet({ text: s })}
    />
  )
}

/** What the calculation does with an input, when that is not "counted":
 * the mark for one not calculated yet, a quiet note for one that changes
 * no number. */
function EffectNote({ input }: { input: ConfigInputView }) {
  if (input.effect === 'unmodelled') {
    return <StateMark state="unsupported" name={t('config.unmodelled')} text />
  }
  if (input.effect === 'no-effect') {
    return <span className="muted small-text">{t('config.noEffect')}</span>
  }
  return null
}

/** An input's name: PoB's label for it, else the key in words. */
function optionLabel(name: string): string {
  return optionOf(name)?.label ?? inputLabel(name)
}

/** The first line of PoB's tooltip, as a short help line. */
function helpLine(option: ConfigOption | undefined): string | null {
  const first = option?.tooltip?.split('\n')[0]?.trim()
  return first != null && first !== '' ? first : null
}

/** An input's description: two lines, and when it is longer a "More"
 * that shows the rest (RV-38: it was cut at two lines with the rest only
 * in a hover title). */
function ConfigHelp({ text, title }: { text: string; title?: string }) {
  const el = useRef<HTMLSpanElement>(null)
  const [open, setOpen] = useState(false)
  const [cut, setCut] = useState(false)
  useLayoutEffect(() => {
    const span = el.current
    if (span === null || open) return
    const measure = () => setCut(span.scrollHeight > span.clientHeight + 1)
    measure()
    if (typeof ResizeObserver === 'undefined') return
    const observer = new ResizeObserver(measure)
    observer.observe(span)
    return () => observer.disconnect()
  }, [open, text])
  return (
    <span className="muted small-text">
      <span ref={el} className={open ? undefined : 'config-help'} title={title}>
        {text}
      </span>
      {(cut || open) && (
        <>
          {' '}
          <button
            type="button"
            className="link plain"
            aria-expanded={open}
            onClick={() => setOpen((o) => !o)}
          >
            {open ? t('config.helpLess') : t('config.helpMore')}
          </button>
        </>
      )}
    </span>
  )
}

function InputRow({
  input,
  detail,
}: {
  input: ConfigInputView
  /** Whether the input's technical name is shown. */
  detail: boolean
}) {
  const dispatch = useAppDispatch()
  const option = optionOf(input.name)
  const label = optionLabel(input.name)
  const help = helpLine(option)
  return (
    <li className="config-row">
      <div className="config-name">
        <span>{label}</span>
        {help !== null && <ConfigHelp text={help} title={option?.tooltip} />}
        {(detail || input.placeholder) && (
          <span className="muted small-text">
            {detail && <span className="mono">{input.name}</span>}
            {detail && input.placeholder && ' · '}
            {input.placeholder && t('config.placeholder')}
          </span>
        )}
      </div>
      <div className="config-effect">
        <EffectNote input={input} />
      </div>
      <div className="config-value">
        <ValueEditor
          input={input}
          label={label}
          onSet={(value) =>
            void dispatch(
              run({ 'set-config-input': { name: input.name, value } }),
            )
          }
        />
      </div>
      <button
        type="button"
        className="icon small quiet"
        aria-label={t('config.remove', { input: label })}
        title={t('config.remove', { input: label })}
        onClick={() =>
          void dispatch(run({ 'remove-config-input': { name: input.name } }))
        }
      >
        <Icon name="close" size={14} />
      </button>
    </li>
  )
}

/** How many matches the option picker lists at once. */
const SHOWN = 40

/** Adds any of PoB's options (the catalogue): searched by label or key;
 * by default the options PoB shows for every build and those for this
 * build's skills, all of them on request. */
function AddOption({ have }: { have: ReadonlySet<string> }) {
  const dispatch = useAppDispatch()
  const id = useId()
  const groups = useAppSelector(
    (s) => s.session.snapshot?.build?.['skill-groups'] ?? [],
  )
  const [query, setQuery] = useState('')
  const [all, setAll] = useState(false)
  const skills = new Set(
    groups.flatMap((g) => g.gems.map((x) => x.name ?? '')).filter(Boolean),
  )
  const found = OPTIONS.filter(
    (o) =>
      !have.has(o.var) && (all || relevant(o, skills)) && matches(o, query),
  )
  return (
    <details className="config-add">
      <summary>{t('config.addOption')}</summary>
      <p className="muted small-text">
        {t('config.addOptionHelp', { version: SOURCE_TAG })}
      </p>
      <div className="row">
        <label className="field">
          <span>{t('config.search')}</span>
          <input
            id={id}
            type="search"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
          />
        </label>
        <label className="check">
          <input
            type="checkbox"
            checked={all}
            onChange={(e) => setAll(e.target.checked)}
          />
          {t('config.allOptions')}
        </label>
      </div>
      <p className="muted small-text" role="status">
        {t('config.optionsFound', { n: found.length })}
      </p>
      <ul className="config-options">
        {found.slice(0, SHOWN).map((o) => (
          <li key={o.var}>
            <span className="config-name">
              <span>{o.label}</span>
              <span className="muted small-text">
                {[o.section, o.heading].filter(Boolean).join(' · ')}
              </span>
            </span>
            <button
              type="button"
              className="small"
              aria-label={t('config.addNamed', { option: o.label })}
              onClick={() =>
                void dispatch(
                  run({
                    'set-config-input': {
                      name: o.var,
                      value: initialValue(o),
                    },
                  }),
                )
              }
            >
              <Icon name="plus" size={14} />
              {t('config.add')}
            </button>
          </li>
        ))}
      </ul>
      {found.length > SHOWN && (
        <p className="muted small-text">
          {t('config.optionsMore', { n: found.length - SHOWN })}
        </p>
      )}
    </details>
  )
}

type Kind = 'boolean' | 'number' | 'text'

function AddInput() {
  const dispatch = useAppDispatch()
  const id = useId()
  const [name, setName] = useState('')
  const [kind, setKind] = useState<Kind>('boolean')
  const [text, setText] = useState('')
  const value = (): ConfigValueView | null => {
    if (kind === 'boolean') return { boolean: true }
    if (kind === 'text') return { text }
    const n = Number(text)
    return text.trim() !== '' && Number.isFinite(n) ? { number: n } : null
  }
  const v = value()
  const ok = name.trim() !== '' && name.trim() !== LEGACY && v !== null
  return (
    <details className="config-add">
      <summary>{t('config.addInput')}</summary>
      <p className="muted small-text">{t('config.addHelp')}</p>
      <form
        className="row"
        onSubmit={(e) => {
          e.preventDefault()
          if (!ok) return
          void dispatch(
            run({ 'set-config-input': { name: name.trim(), value: v } }),
          ).then((o) => {
            if (o !== null) {
              setName('')
              setText('')
            }
          })
        }}
      >
        <label className="field">
          <span>{t('config.inputName')}</span>
          <input
            id={id}
            value={name}
            placeholder="conditionOnConsecratedGround"
            onChange={(e) => setName(e.target.value)}
          />
        </label>
        <label className="field">
          <span>{t('config.inputKind')}</span>
          <select
            value={kind}
            onChange={(e) => setKind(e.target.value as Kind)}
          >
            <option value="boolean">{t('config.kindBoolean')}</option>
            <option value="number">{t('config.kindNumber')}</option>
            <option value="text">{t('config.kindText')}</option>
          </select>
        </label>
        {kind !== 'boolean' && (
          <label className="field">
            <span>{t('config.inputValue')}</span>
            <input
              value={text}
              inputMode={kind === 'number' ? 'decimal' : undefined}
              onChange={(e) => setText(e.target.value)}
            />
          </label>
        )}
        <button type="submit" disabled={!ok}>
          <Icon name="plus" size={16} />
          {t('config.add')}
        </button>
      </form>
    </details>
  )
}

const lineNote = (line: CustomBlockView['lines'][number]): string | null => {
  if (line.status === 'warning') return t('config.lineWarning')
  if (line.typed === 'custom-note') return t('config.lineNote')
  if (line.typed === 'custom-pattern') return t('config.linePattern')
  return null
}

function Block({ block, index }: { block: CustomBlockView; index: number }) {
  const dispatch = useAppDispatch()
  const id = useId()
  const [text, setText] = useState(block.text)
  const [seen, setSeen] = useState(block.text)
  if (seen !== block.text) {
    setSeen(block.text)
    setText(block.text)
  }
  const title =
    block.title != null && block.title !== ''
      ? block.title
      : t('config.blockN', { n: index + 1 })
  const edit = (change: { enabled?: boolean; text?: string }) =>
    dispatch(run({ 'edit-custom-block': { block: index, ...change } }))
  const warnings = block.lines.filter((l) => l.status === 'warning').length
  // The lines as they were read are the block; its text is edited on
  // request (an empty block starts with its text open).
  const [asText, setAsText] = useState(block.lines.length === 0)
  const editing = asText || text !== block.text
  return (
    <section
      className={`config-block${block.enabled ? '' : ' disabled'}`}
      aria-labelledby={id}
    >
      <div className="skill-group-head">
        <h3 id={id}>{title}</h3>
        <span className="spacer" />
        {warnings > 0 && (
          <span className="muted small-text">
            {t('config.blockWarnings', { n: warnings })}
          </span>
        )}
        <button
          type="button"
          className="link"
          aria-expanded={editing}
          onClick={() => setAsText(!editing)}
        >
          {editing ? t('config.hideText') : t('config.editText')}
        </button>
        <label className="check">
          <input
            type="checkbox"
            aria-label={t('config.blockOn', { block: title })}
            checked={block.enabled}
            onChange={(e) => void edit({ enabled: e.target.checked })}
          />
          {t('skills.on')}
        </label>
      </div>
      {block.lines.length > 0 && (
        <ul className="config-lines" aria-label={t('config.lines')}>
          {block.lines.map((line, i) => {
            const note = lineNote(line)
            return (
              // A note is a heading of the user's own; a line carries
              // its mark: counted, or not.
              line.typed === 'custom-note' && line.status !== 'warning' ? (
                <li key={i} className="config-line-note">
                  <span>{line.text}</span>
                  <span className="visually-hidden">{note}</span>
                </li>
              ) : (
                <li key={i}>
                  {line.status === 'warning' ? (
                    <StateMark state="unsupported" name={note ?? undefined} />
                  ) : (
                    <StateMark
                      state="complete"
                      name={t('config.lineCounted')}
                    />
                  )}
                  <span>{line.text}</span>
                  {note !== null && line.status !== 'warning' && (
                    <span className="muted small-text">{note}</span>
                  )}
                </li>
              )
            )
          })}
        </ul>
      )}
      {editing && (
        <>
          <label className="visually-hidden" htmlFor={`${id}-text`}>
            {t('config.blockText', { block: title })}
          </label>
          <textarea
            id={`${id}-text`}
            className="config-text"
            rows={Math.min(12, Math.max(3, text.split('\n').length + 1))}
            value={text}
            spellCheck={false}
            onChange={(e) => setText(e.target.value)}
          />
          <div className="row">
            <button
              type="button"
              className="primary"
              disabled={text === block.text}
              onClick={() => void edit({ text })}
            >
              {t('config.applyText')}
            </button>
            {text !== block.text && (
              <button type="button" onClick={() => setText(block.text)}>
                {t('config.revertText')}
              </button>
            )}
            <span className="spacer" />
            <button
              type="button"
              className="danger"
              onClick={() =>
                void dispatch(run({ 'remove-custom-block': { block: index } }))
              }
            >
              {t('config.removeBlock')}
            </button>
          </div>
        </>
      )}
    </section>
  )
}

function ConfigSetPicker({ sets }: { sets: SetView[] }) {
  const dispatch = useAppDispatch()
  const id = useId()
  const active = sets.find((s) => s.active)
  return (
    <div className="set-picker">
      <label htmlFor={id}>{t('config.set')}</label>
      <select
        id={id}
        value={active?.id ?? ''}
        onChange={(e) =>
          void dispatch(
            run({ 'use-config-set': { id: Number(e.target.value) } }),
          )
        }
      >
        {sets.map((s) => (
          <option key={s.id} value={s.id}>
            {cleanTitle(s.title) || t('config.setN', { n: s.id })}
          </option>
        ))}
      </select>
    </div>
  )
}

export function Config() {
  const dispatch = useAppDispatch()
  const build = useAppSelector((s) => s.session.snapshot?.build)
  const [find, setFind] = useState('')
  const [detail, setDetail] = useState(false)
  if (build == null) return null
  const config = build.config
  const sets = build['config-sets']
  const inputs = config?.inputs ?? []
  const unmodelled = inputs.filter((i) => i.effect === 'unmodelled').length
  const words = find.trim().toLowerCase()
  const found = (i: ConfigInputView) =>
    words === '' ||
    optionLabel(i.name).toLowerCase().includes(words) ||
    i.name.toLowerCase().includes(words)
  return (
    <>
      <div className="page-head">
        <ScreenHeading>{t('config.title')}</ScreenHeading>
        <Hint>{t('config.help')}</Hint>
        <span className="spacer" />
        <input
          type="search"
          className="config-find"
          aria-label={t('config.find')}
          placeholder={t('config.find')}
          value={find}
          onChange={(e) => setFind(e.target.value)}
        />
        <LeaguePicker league={build.league} />
        {sets.length > 1 && <ConfigSetPicker sets={sets} />}
      </div>
      {unmodelled > 0 && (
        <p className="muted">{t('config.unmodelledHelp', { n: unmodelled })}</p>
      )}
      <div className="config-grid">
        <section className="panel" aria-labelledby="config-inputs">
          <h2 id="config-inputs">{t('config.inputs')}</h2>
          {inputs.length === 0 && (
            <p className="muted">{t('config.noInputs')}</p>
          )}
          {GROUPS.map((g) => {
            const shown = inputs.filter(
              (i) => groupOf(i.name) === g && found(i),
            )
            if (shown.length === 0) return null
            // Default values that change no number are folded away, so
            // what the build sets stands out.
            const quiet = (i: ConfigInputView) =>
              i.placeholder && i.effect === 'no-effect'
            const set = shown.filter((i) => !quiet(i))
            const defaults = shown.filter(quiet)
            return (
              <div key={g} className="config-group">
                <h3>{tDynamic(`config.group.${g}`, 'config.group.other')}</h3>
                {set.length > 0 && (
                  <ul className="config-list">
                    {set.map((i) => (
                      <InputRow key={i.name} input={i} detail={detail} />
                    ))}
                  </ul>
                )}
                {defaults.length > 0 && (
                  <details className="config-defaults">
                    <summary>
                      {t('config.defaults', { n: defaults.length })}
                    </summary>
                    <ul className="config-list">
                      {defaults.map((i) => (
                        <InputRow key={i.name} input={i} detail={detail} />
                      ))}
                    </ul>
                  </details>
                )}
              </div>
            )
          })}
          {words !== '' && !inputs.some(found) && (
            <p className="muted" role="status">
              {t('config.findNone')}
            </p>
          )}
          <AddOption have={new Set(inputs.map((i) => i.name))} />
          <AddInput />
          <button
            type="button"
            className="link quiet config-detail"
            aria-pressed={detail}
            onClick={() => setDetail(!detail)}
          >
            {detail ? t('review.hideDetail') : t('review.showDetail')}
          </button>
        </section>
        <section className="panel" aria-labelledby="config-blocks">
          <h2 id="config-blocks">{t('config.blocks')}</h2>
          <p className="muted small-text">{t('config.blocksHelp')}</p>
          {(config?.blocks ?? []).map((b, i) => (
            <Block key={i} block={b} index={i} />
          ))}
          <button
            type="button"
            onClick={() =>
              void dispatch(run({ 'add-custom-block': { text: '' } }))
            }
          >
            <Icon name="plus" size={16} />
            {t('config.addBlock')}
          </button>
        </section>
      </div>
    </>
  )
}
