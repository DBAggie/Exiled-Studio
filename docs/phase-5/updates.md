# Application updates (Phase 5, P3)

The plan's step P3 and decision P-d (`dev/active/passive-tree-and-phase-5/`). The application checks GitHub for a newer version and, with the user's approval, installs it. Code: `crates/loom-tauri/src/updates.rs`; UI: `ui/src/screens/Updates.tsx`.

## How it works
- **Plugin.** `tauri-plugin-updater` 2.14 with `native-tls` and `system-proxy` and no default features. The default `rustls-tls` would bring `webpki-roots`, whose CDLA-Permissive-2.0 license is not on the allow-list (review M12). `native-tls` uses SChannel on Windows and OpenSSL on Linux.
- **Feed.** `latest.json`, an asset of the GitHub release, at `https://github.com/DBAggie/Exiled-Studio/releases/latest/download/latest.json` (`tauri.conf.json`, `plugins.updater.endpoints`). `tools/package/latest-json.sh` writes it in the `release` workflow's draft job.

  | Entry | Package | Found by |
  |---|---|---|
  | `windows-x86_64-nsis` | the NSIS setup | the NSIS install |
  | `windows-x86_64-msi` | the MSI | the MSI install |
  | `linux-x86_64-appimage` | the AppImage | the AppImage |
  | `windows-x86_64` | the NSIS setup | the portable zip (notifies only) |
  | `linux-x86_64` | the AppImage | the `.deb` (notifies only) |

- **Signatures.** Each update package is signed with the update key (minisign) by `tauri build` with `crates/loom-tauri/tauri.updater.conf.json` (`createUpdaterArtifacts`). The plugin checks the signature after the download and before anything is installed. The public key is `release/updater.pub`, compiled in. It is empty until the maintainer makes the key (`docs/release/signing.md`); a build without it cannot check, and About says so.
- **No downgrade.** Only a version strictly greater than the running one is offered (`updates::newer`, the plugin's version comparator). An equal version, a lower one, or a pre-release of the running version is no update. This also makes a replayed old feed harmless.
- **TLS** carries the feed and the download. The signature, not TLS, is what makes a package trusted.

## Modes
Kept in `settings.json` (`updates`), set under About, Updates.

| Mode | Check at start | Check when asked | Install |
|---|---|---|---|
| `automatic-check` (default) | yes, in the background | yes | on the user's click |
| `manual` | no | yes | on the user's click |
| `strict-offline` | no | refused, `UPDATE_OFFLINE` | refused, `UPDATE_OFFLINE` |

- A smoke run and an `e2e` build never check at start.
- Switching to strict offline forgets an offer.

## Per package

| Package | On an offer |
|---|---|
| NSIS setup, MSI, AppImage | the prompt offers **Install and restart** and **Later** |
| `.deb`, portable zip | the prompt shows the release page; there is no install button |

- `updates::installable` refuses a notify-only package (`UPDATE_NOTIFY_ONLY`) before anything is downloaded.
- The `.deb` is updated by the system's package tools and the portable zip by unpacking a new one, so neither replaces itself.
- **Install.** The application downloads the package, the plugin verifies its signature, then installs: on Windows the NSIS or MSI installer (passive mode, a progress window), on Linux by replacing the AppImage.
  - Before the process exits, the application stops as at any exit (`loom_tauri::shutdown`), so a change made since the last autosave is kept for recovery and the MCP server stops.
  - On Linux the application restarts itself.
  - A failed download or signature leaves the installed version unchanged (`UPDATE_INSTALL`).

## Privacy (ADR-011)
- A check is an HTTPS request to GitHub. GitHub sees the IP address, and the URL and the user agent reveal the running version. Nothing else is sent: no identifier, no build data.
- About shows this next to the setting. The README's privacy notes and the installer's license page say it too.
- This is the one network path outside `loom-integrations`: a Tauri adapter, allowed by plan decision P-d and listed in AGENTS.md.
- The webview has no `updater:*` permission. Only the shell's own commands (`update_status`, `set_update_mode`, `check_updates`, `install_update`) reach the plugin.

## Error codes
| Code | Meaning |
|---|---|
| `UPDATE_OFFLINE` | strict offline mode |
| `UPDATE_UNAVAILABLE` | no update key in this build, or no plugin (tests) |
| `UPDATE_BUSY` | a check is already running |
| `UPDATE_CHECK` | the feed could not be fetched or read; kept in the status, not raised |
| `UPDATE_NONE` | an install without an offer |
| `UPDATE_NOTIFY_ONLY` | an install from a `.deb` or the portable zip |
| `UPDATE_INSTALL` | the download, the signature, or the installer failed |

## Tests
- **`crates/loom-tauri/src/updates.rs`, unit tests:**
  - only a greater version is an update, so equal, lower, and pre-release versions are not;
  - the `.deb` and the portable zip never install in any mode;
  - strict offline refuses every package.
- **`crates/loom-tauri/tests/updater.rs`** runs a mock application with the plugin against a loopback HTTP feed.
  - Version 0.2.0 is offered to 0.1.0, and its offer carries the notes, date, and release page.
  - The signed package downloads and verifies.
  - A package changed by one byte is refused.
  - 0.1.0, 0.0.9, and 0.1.0-rc.1 are not offered.
  - An unreachable feed is an error, not an offer.
  - The test key in `tests/fixtures/updater/` was made by `tauri signer generate` and signs nothing real.
- **`ui/src/test/updates.test.tsx`:**
  - the mode and its privacy note; a change is remembered;
  - check on request, and install only on the click;
  - "newest version";
  - **Later** hides the prompt;
  - a notify-only package shows the release page without an install button;
  - a build without a key offers no check;
  - axe checks.

## Not tested here
- **A real install, from one released version to the next.** It needs two releases signed with the real key. It is the maintainer's check after the first two releases: install `0.1.0`, publish `0.1.1`, check, install, and see that About shows `0.1.1`.
- **The `release` workflow's signed packages and `latest.json`.** These are made only in that workflow, so they are checked by its recorded run (gate 6, after this sprint's PR reaches `main`).
