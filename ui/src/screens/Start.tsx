// Screen 1 (docs/ui, Main), start page B (the maintainer's choice, UX
// brief note 4): one command bar that imports a pasted share code or PoB
// XML or finds a recent build; the actions New build, Import from
// clipboard, Open a file; unsaved work to restore; recent builds. The game
// data's status is the frame's footer line (App.tsx), and a file can be
// dropped anywhere in the window.
import { useEffect, useId, useRef, useState } from 'react'

import { ErrorCode, Icon, ScreenHeading } from '../components'
import type { CrashRecord, RecentEntry } from '../api/types'
import { errorText, t } from '../i18n'
import {
  actions,
  importBuild,
  openBuild,
  query,
  refreshLists,
  replaceBuild,
  run,
} from '../store'
import { useAppDispatch, useAppSelector } from '../util'

function when(seconds: number): string {
  return new Date(seconds * 1000).toLocaleString(undefined, {
    dateStyle: 'medium',
    timeStyle: 'short',
  })
}

const relative = new Intl.RelativeTimeFormat(undefined, { numeric: 'auto' })

/** "2 hours ago", "yesterday" (UX-20). */
function ago(seconds: number): string {
  const diff = seconds - Date.now() / 1000
  const units: [Intl.RelativeTimeFormatUnit, number][] = [
    ['year', 31_536_000],
    ['month', 2_592_000],
    ['week', 604_800],
    ['day', 86_400],
    ['hour', 3_600],
    ['minute', 60],
  ]
  for (const [unit, size] of units) {
    if (Math.abs(diff) >= size)
      return relative.format(Math.round(diff / size), unit)
  }
  return t('start.justNow')
}

/** A path's last part; the full path is shown on hover. */
function fileName(path: string): string {
  return path.split(/[\\/]/).pop() ?? path
}

/** Whether text is a build to import rather than words to find a recent
 * build by: PoB XML, a share code (one long run of base64url), or a link
 * (which the import then explains, UX-18). */
function looksLikeBuild(text: string): boolean {
  const s = text.trim()
  return (
    s.startsWith('<') ||
    /^https?:\/\//i.test(s) ||
    (s.length >= 40 && /^[A-Za-z0-9_\-+/=]+$/.test(s))
  )
}

/** "Inquisitor 92 · Smite" (UX-20); empty when the list has nothing. */
function summary(r: RecentEntry): string {
  const who = [r.ascendancy ?? r.class, r.level].filter((x) => x != null)
  return [who.join(' '), r['main-skill']].filter((x) => x).join(' · ')
}

/** Whether a key or paste event belongs to a text field. */
function inField(target: EventTarget | null): boolean {
  return (
    target instanceof HTMLElement &&
    (target.isContentEditable ||
      target instanceof HTMLTextAreaElement ||
      target instanceof HTMLInputElement)
  )
}

/** Crash records from earlier runs (Phase 5, P-f): each shown until the
 * user dismisses it; the recovery offer below is the way back to unsaved
 * work. */
function CrashNotice() {
  const dispatch = useAppDispatch()
  const [crashes, setCrashes] = useState<CrashRecord[]>([])
  useEffect(() => {
    let live = true
    void dispatch(query((api) => api.crashes())).then((c) => {
      if (live) setCrashes(c ?? [])
    })
    return () => {
      live = false
    }
  }, [dispatch])
  if (crashes.length === 0) return null
  const dismiss = (id: string) => {
    void dispatch(query((api) => api.dismissCrash(id).then(() => true))).then(
      (ok) => {
        if (ok) setCrashes((all) => all.filter((c) => c.id !== id))
      },
    )
  }
  return (
    <section className="panel crash-notice" aria-labelledby="crash-h">
      <h2 id="crash-h">{t('crash.title')}</h2>
      <ul>
        {crashes.map((c) => (
          <li key={c.id} className="row">
            <span className="spacer">
              {t('crash.body', {
                version: c.version,
                when: ago(c.at),
                message: c.message,
              })}
            </span>
            <button type="button" onClick={() => dismiss(c.id)}>
              {t('crash.dismiss')}
            </button>
          </li>
        ))}
      </ul>
    </section>
  )
}

export function Start() {
  const dispatch = useAppDispatch()
  const snapshot = useAppSelector((s) => s.session.snapshot)
  const flow = useAppSelector((s) => s.importFlow)
  const recent = useAppSelector((s) => s.lists.recent)
  const recovery = useAppSelector((s) => s.lists.recovery)
  const [text, setText] = useState('')
  const [hint, setHint] = useState<string | null>(null)
  const barRef = useRef<HTMLTextAreaElement>(null)
  const barId = useId()
  const hasData = snapshot?.data?.['data-version'] != null
  const reading = flow.state === 'reading'

  const importText = (value: string) => {
    // A pobb.in or pastebin link: say so before trying (UX-18).
    if (/^\s*https?:\/\//i.test(value)) {
      dispatch(actions.importStarted())
      dispatch(actions.importFailed('UI_LINK'))
      return
    }
    void dispatch(
      importBuild((api) => api.execute({ 'import-text': { text: value } })),
    )
  }

  // Ctrl+V on the page, outside a field, imports the clipboard's text
  // (the paste event needs no clipboard permission). Registered once; it
  // reads the latest state through the ref.
  const onPage = useRef<(e: ClipboardEvent) => void>(() => {})
  useEffect(() => {
    onPage.current = (e) => {
      if (inField(e.target) || !hasData) return
      const pasted = e.clipboardData?.getData('text') ?? ''
      if (pasted.trim() === '') return
      e.preventDefault()
      setText(pasted)
      importText(pasted)
    }
  })
  useEffect(() => {
    const onPaste = (e: ClipboardEvent) => onPage.current(e)
    window.addEventListener('paste', onPaste)
    return () => window.removeEventListener('paste', onPaste)
  }, [])

  const fromClipboard = async () => {
    setHint(null)
    try {
      const pasted = (await navigator.clipboard?.readText()) ?? ''
      if (pasted.trim() === '') {
        setHint(t('start.clipboardEmpty'))
        return
      }
      setText(pasted)
      importText(pasted)
    } catch {
      // Refused: the paste shortcut works without permission.
      barRef.current?.focus()
      setHint(t('start.clipboardRefused'))
    }
  }

  const query = looksLikeBuild(text) ? '' : text.trim().toLowerCase()
  const shown =
    query === ''
      ? recent
      : recent.filter((r) =>
          [r.name, fileName(r.path), summary(r)].some((s) =>
            s.toLowerCase().includes(query),
          ),
        )

  return (
    <>
      <div>
        <ScreenHeading>{t('start.title')}</ScreenHeading>
        <p className="lead">{t('start.lead')}</p>
      </div>

      <CrashNotice />

      {/* Unsaved work first: it is what the user most likely came back
          for (UX-19). */}
      {recovery.length > 0 && (
        <section className="panel recovery" aria-labelledby="recovery-h">
          <div className="panel-head">
            <h2 id="recovery-h">{t('start.recovery')}</h2>
          </div>
          <ul className="recovery-list">
            {recovery.map((r) => {
              const name = r.name === '' ? t('app.untitled') : r.name
              const file = r.path ?? r.origin
              return (
                <li key={r.id}>
                  <div>
                    <strong>{name}</strong>
                    <span className="sub">
                      {t('start.recoveryWhen', { when: ago(r['saved-at']) })}
                      {file != null && (
                        <>
                          {' · '}
                          <span className="mono" title={file}>
                            {fileName(file)}
                          </span>
                        </>
                      )}
                    </span>
                  </div>
                  <div className="row end">
                    <button
                      type="button"
                      className="primary"
                      aria-label={t('start.restore', { name })}
                      onClick={() =>
                        void dispatch(
                          openBuild((api) =>
                            api.execute({ restore: { id: r.id } }),
                          ),
                        )
                      }
                    >
                      {t('start.restoreShort')}
                    </button>
                    <button
                      type="button"
                      aria-label={t('start.discard', { name })}
                      onClick={() =>
                        void dispatch(run({ discard: { id: r.id } })).then(() =>
                          dispatch(refreshLists()),
                        )
                      }
                    >
                      {t('start.discardShort')}
                    </button>
                  </div>
                </li>
              )
            })}
          </ul>
        </section>
      )}

      <section className="panel command-panel" aria-label={t('start.import')}>
        <form
          className="command-bar"
          onSubmit={(e) => {
            e.preventDefault()
            importText(text)
          }}
        >
          <label htmlFor={barId} className="visually-hidden">
            {t('start.importLabel')}
          </label>
          <Icon name="download" size={20} />
          <textarea
            id={barId}
            ref={barRef}
            rows={1}
            value={text}
            placeholder={t('start.barPlaceholder')}
            aria-describedby={`${barId}-help`}
            onChange={(e) => setText(e.target.value)}
            onPaste={(e) => {
              // A build pasted into the empty bar imports at once.
              const pasted = e.clipboardData.getData('text')
              if (text === '' && hasData && looksLikeBuild(pasted)) {
                e.preventDefault()
                setText(pasted)
                importText(pasted)
              }
            }}
            onKeyDown={(e) => {
              if (e.key === 'Enter' && !e.shiftKey) {
                e.preventDefault()
                // Words find a recent build; only a build imports.
                if (hasData && looksLikeBuild(text)) importText(text)
              }
            }}
          />
          <button
            type="submit"
            className="primary"
            disabled={!hasData || reading || text.trim() === ''}
          >
            {t('start.importButton')}
          </button>
        </form>
        <p id={`${barId}-help`} className="muted small">
          {hasData ? t('start.importSafe') : t('start.needData')}
        </p>
        {reading && <p role="status">{t('import.reading')}</p>}
        {hint !== null && <p role="status">{hint}</p>}
        {flow.state === 'failed' && (
          <div role="alert">
            <p className="error">{errorText(flow.code)}</p>
            {flow.code !== 'UI_LINK' && <ErrorCode code={flow.code} />}
          </div>
        )}
        <div className="start-actions">
          <button
            type="button"
            disabled={!hasData}
            onClick={() => void dispatch(replaceBuild('new-build'))}
          >
            <Icon name="plus" />
            {t('start.newBuild')}
            <kbd aria-hidden="true">Ctrl N</kbd>
          </button>
          <button
            type="button"
            disabled={!hasData || reading}
            onClick={() => void fromClipboard()}
          >
            <Icon name="copy" />
            {t('start.fromClipboard')}
            <kbd aria-hidden="true">Ctrl V</kbd>
          </button>
          <button
            type="button"
            onClick={() => void dispatch(replaceBuild('open-file'))}
          >
            <Icon name="folder" />
            {t('start.open')}
            <kbd aria-hidden="true">Ctrl O</kbd>
          </button>
          <button
            type="button"
            className="link"
            disabled={!hasData || reading}
            onClick={() => void dispatch(replaceBuild('import-file'))}
          >
            {t('start.importFile')}
          </button>
        </div>
      </section>

      <section aria-labelledby="recent-h">
        <div className="recent-head">
          <h2 id="recent-h">{t('start.recent')}</h2>
          {query !== '' && (
            <span className="muted" role="status">
              {t('start.recentFiltered', {
                n: shown.length,
                all: recent.length,
              })}
            </span>
          )}
        </div>
        {recent.length === 0 ? (
          <p className="muted">{t('start.recentNone')}</p>
        ) : shown.length === 0 ? (
          <p className="muted">{t('start.recentNoMatch')}</p>
        ) : (
          <div className="panel table-panel">
            <table>
              <thead>
                <tr>
                  <th scope="col">{t('start.colBuild')}</th>
                  <th scope="col">{t('start.colOpened')}</th>
                  <th scope="col">
                    <span className="visually-hidden">
                      {t('start.colActions')}
                    </span>
                  </th>
                </tr>
              </thead>
              <tbody>
                {shown.map((r) => {
                  const what = summary(r)
                  return (
                    <tr key={r.id}>
                      <th scope="row">
                        <button
                          type="button"
                          className="recent-name"
                          onClick={() =>
                            void dispatch(
                              openBuild((api) =>
                                api.execute({ 'open-recent': { id: r.id } }),
                              ),
                            )
                          }
                        >
                          {r.name}
                        </button>
                        {what !== '' && <span className="sub">{what}</span>}
                        <span className="sub mono" title={r.path}>
                          {fileName(r.path)}
                        </span>
                      </th>
                      <td title={when(r.at)}>{ago(r.at)}</td>
                      <td className="right">
                        <button
                          type="button"
                          className="link"
                          aria-label={t('start.forget', { name: r.name })}
                          onClick={() =>
                            void dispatch(
                              run({ 'forget-recent': { id: r.id } }),
                            ).then(() => dispatch(refreshLists()))
                          }
                        >
                          {t('start.forgetShort')}
                        </button>
                      </td>
                    </tr>
                  )
                })}
              </tbody>
            </table>
          </div>
        )}
      </section>
    </>
  )
}
