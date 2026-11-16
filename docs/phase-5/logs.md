# Logs, diagnostics, and crash records (Phase 5, P5; PR #115)

Step P5 of `dev/active/passive-tree-and-phase-5/passive-tree-and-phase-5-plan.md` (design P-f; ADR-014). Everything here lives in the host crates `loom-app` and `loom-tauri`. No core crate logs, so the core crates stay wasm32-safe (ADR-016). Nothing is sent anywhere (ADR-011): the logs, the crash records, and the diagnostic package stay on the user's disk until the user attaches the package to a report themselves.

## Where things are

| What | Where | Module |
|---|---|---|
| Logs, JSON lines | `<data>/logs/loom.log`, then `loom.1.log` to `loom.6.log` | `loom_app::logs` |
| Crash records | `<data>/crashes/<id>.json`, the 20 newest kept | `loom_app::crash` |
| Diagnostic package | a `.zip` the user picks the path for | `loom_app::diagnostics` |

`<data>` is the application's data folder (`LOOM_DATA_DIR` in tests and the e2e).

## Logs

- **Format.** `tracing` 0.1 with `tracing-subscriber`'s JSON layer. Each event is one line with `timestamp`, `level`, `target`, `message`, its fields, and the current span's fields under `span`. Only the application's own events are written: targets under `loom` (a `Targets` filter), at `INFO` and above. A library's events never reach the file.
- **Rotation.** By size. Before a write that would take `loom.log` past 5 MiB, `loom.6.log` is deleted, each `loom.<n>.log` becomes `loom.<n+1>.log`, and `loom.log` becomes `loom.1.log`. The newest 7 files are kept: at most 35 MiB. If a rotation fails (a file held open by another program), the line goes into the current file and the next write tries again.
- **Each event is buffered,** redacted, and written as one line, so a rotation never splits an event.
- **What is logged** (targets):

  | Target | Event | Fields |
  |---|---|---|
  | `loom::app` | `start` | `version`, `os` |
  | `loom::command` | each command the writer runs | `command` (kebab name), `outcome` `ok`, `version`, `ms`; for imports and opens, the file's `ext`, `size`, and `sha256`; for `import-text`, the text's `size` and `sha256` |
  | `loom::command` | a failed command (`WARN`) | `command`, `error` (the stable code only, never the message), `ms` |
  | `loom::calc` | `calculated`, `cancelled`, or `failed` | `state` or `error`, `ms` |
  | `loom::recovery` | `written`, or not written (`WARN`) | `steps` |
  | `loom::diagnostics` | `exported` | `files` |
  | `loom::crash` | `panic` (`ERROR`) | `id`, `thread`, `location` |

  Bundle installs and switches (`use-bundle`, the data commands) are commands, so they are logged the same way. An MCP tool that changes the build runs an app command, so it is logged as that command. The MCP server's own requests and approvals have no events of their own yet. Updates have no code yet; when they do, they log through the same commands.

  An imported file's hash is read when the command is logged, a second read of the file. Build files are small, and the hash describes the file at that moment.
- **Correlation IDs.** `App::execute` already gave each command a correlation number. The writer runs each command inside `info_span!("command", correlation)`. The worker runs the calculation that command started inside `info_span!("calc", correlation, version)`. So a command's line and its calculation's line carry the same `span.correlation`. A calculation that a later command superseded carries the later one.

## Redaction

`logs::Redactor` runs on every log line, every crash record field, and every file in the diagnostic package.

- **The home folder** (`USERPROFILE` or `HOME`) becomes `~`. It is matched in four forms: as written, with `/` separators, JSON-escaped once (`\\`, a path field), and twice (`\\\\`, a path `Debug`-formatted into a field).
- **The user name** (`USERNAME` or `USER`, three characters or longer) becomes `<user>`, only as a whole word: the name `bob` leaves `bobsled` alone.
- Matching ignores ASCII case, and the longest needle wins.
- **Not covered:** 8.3 short names (`JUSTIN~1`), percent-encoded `file:///` paths, and a non-ASCII name in another case. No logged field carries those today.

What is never logged: build text, item text, share codes, file names, and error messages. A build, an imported file, or pasted text appears only as its size and SHA-256. Error messages may quote build text, so only the stable error code is logged.

## Crash records

`crash::install` sets a panic hook in the shell's setup, after the logs and before `App::start`.

- **The record.** Each panic writes one `CrashRecord` JSON file: `id` (`<seconds>-<pid>-<n>`, digits and `-` only, so a path made from it stays in the folder), `at`, `version`, `thread`, and the panic's `message` and `location`, all redacted.
- **The message.** A formatted panic message can quote what the code was reading, such as a build line inside an `unwrap` of a parse error. So a message made at run time keeps its words but replaces each quoted span (`"…"`, `'…'`) with `"…"`, and is cut to 200 characters (`crash::scrub`). A literal message (`panic!("…")` without arguments) is kept as written.
- **Order.** The hook writes the record first, then an `ERROR` log event, then runs the hook that was there before. A log writer that cannot proceed does not cost the record. A panic inside the hook itself is not recorded again (a per-thread guard). Records that tie on time are ordered by the ID's numbers.
- **Caught panics are recorded too.** That includes a calculation's panic, which fails only that calculation. The UI calls it an internal error rather than a crash.
- **The next start.** The start screen lists the records (`crashes`) above the recovery snapshot it already offers. Each record shows its version, time, and message, and can be dismissed (`dismiss_crash`; `APP_NOT_FOUND` when absent).

## The diagnostic package

On the About screen, under "Diagnostics":

1. "Preview the diagnostic package" (`diagnostics`) collects the package and shows each file's name, size, and full text.
2. "Save the diagnostic package…" (`export_diagnostics`) asks where to save and writes the zip there. Cancelling the dialog writes nothing.

The package holds, in order:

1. `about.json`: the application, version, OS, architecture, the data version, the installed bundle's ID (never its folder), and whether a build is open.
2. `logs/<file>` for each log file, newest first, each cut to the whole lines in its last 1 MiB.
3. `crashes/<id>.json` for each record.

**Preview equals zip by construction.** `App::diagnostics` keeps the package it returned, and every save writes exactly that package: a second save, or a retry after a failed write, writes the same files. A fresh package is collected only when nothing was previewed. Events logged after the preview do not reach the zip. The preview renders a file's text only while that file is open.

**The zip.** Stored entries, UTF-8 names, and a fixed date (1980-01-01), so the same package always gives the same bytes. It is written by `diagnostics::zip`, about 60 lines. A package too large for a zip without the 64-bit extension is an error, never a silently shorter zip; `collect` stays far below that. `diagnostics::unzip` reads it back and checks each CRC; the tests use it. Python's `zipfile` read a written package once as an independent check: every CRC matched, and so did the names, sizes, and the 1980-01-01 date.

## Tests (the P5 acceptance)

| Acceptance | Test |
|---|---|
| Redaction | `logs::tests` (each form of the home folder, twice-escaped included; the user name as a word; case); `crash::tests` (a panic message quoting build text keeps none of it); `logs_workflow` (no build name, item name, or XML in the log; no file in the package names the data folder) |
| Retention | `logs::tests` (rotation keeps the limit's number of files, newest first); `crash::tests` (records ordered by time, then the ID's numbers) |
| Correlation IDs | `logs_workflow`: an `import-text` line and its calculation's `loom::calc` line share `span.correlation`; the import is logged by size and SHA-256 |
| Preview equals zip | `diagnostics::tests` (exact round trip, deterministic bytes, refuses non-zip input); `logs_workflow` (after more events, a failed write and then two saves each give the preview's files) |
| Panic record | `crash::tests` (write, list newest first, dismiss, refuse `../`; the installed hook records caught panics with their thread and location, a literal message kept, a formatted one scrubbed) |
| UI | `ui/src/test/diagnostics.test.tsx` (preview then save, axe clean; the crash notice shown and dismissed) |

`crates/loom-app/tests/logs_workflow.rs` is a test binary of its own because it installs the process's global subscriber. All of these run in CI with `cargo test --workspace` and `pnpm test`.

## Choices and deviations

- **Our own stored-zip writer, not a zip crate.** The package is a handful of small text files. A deterministic stored zip is about 60 lines plus `crc32fast`, and its reader serves the preview-equals-zip test. A zip crate would bring compression and many dependencies that nothing else needs.
- **No `tracing-appender`.** Its rolling writer rotates by time, not size. `RotatingFile` (about 80 lines) rotates by size as the plan asks.
- **Logs cut to their last 1 MiB in the package.** This keeps the preview readable and the package small. The full files stay in `<data>/logs/` for the user to attach if asked.
- **The level is fixed at `INFO`.** A setting for `DEBUG` can come when there is a debug event worth having.

## Review and what is left

The code-architecture-reviewer's two important findings are fixed. One was a formatted panic message that could carry build text; it is now scrubbed. The other was a second save, or a retry after a failed write, writing a fresh package instead of the preview; the export now keeps the previewed package. These minor findings are fixed too:
- the hook writes the record before it logs, and guards against a panic inside itself;
- the twice-escaped path form;
- the user name matched as a word only;
- the `loom` target filter;
- a failed rotation keeps the line;
- the tail cut at a line start;
- the zip refuses what it cannot hold;
- records that tie on time ordered by the ID's numbers;
- `ImportText` named without serializing its text;
- the export refused after stop;
- the crash notice keeps a record whose dismissal failed;
- the preview's text rendered only when opened.

Left for later:
- **The MCP server's own events.** Its requests and approvals have no log events yet.
- **Unparseable files in `crashes/`.** They are skipped, never pruned.
- **One data folder, two processes.** Two processes on one data folder would race on rotation. This is the same assumption the recovery copies make.
