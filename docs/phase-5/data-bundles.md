# Data bundles (Phase 5, P4)

The plan's step P4 and decision P-e (`dev/active/passive-tree-and-phase-5/`): game data as signed packs, installed side by side, checked at every start, with a signed feed of new data. Code: `loom_data::pack` (the checks, wasm-safe), `loom_app::install` (the installer), `loom_app::bundles` (the library), `loom_integrations::data_feed` (the downloads), `loom_tauri::data_updates` (the feed commands); the release tool `tools/release` (`loom-release pack`, `feed`).

## The pack
- A `.loombundle` is a zip (deflate only; the `zip` crate without its other codecs or encryption) of a format 2 bundle. It holds:
  - `bundle.json`, the manifest;
  - `signature.json`, the detached Ed25519 signature over the manifest's exact bytes (`docs/release/signing.md`); optional, an unsigned pack is quarantined;
  - every file the manifest lists.

  Nothing else is allowed. The manifest's reserved `signatures` field stays empty: the signature is beside it, since it cannot sign itself.
- **Made by** `loom-release pack <bundle dir> (--secret <file> | --secret-env <NAME> | --test-key | --unsigned) --out <x.loombundle>`.

## Installing: hostile input
`loom_app::install::install_pack`, in this order:
1. **The signature first.** `bundle.json` and `signature.json` are read from the archive, each capped at 16 MiB, and the pack's trust is decided (`loom_data::pack::trust_of`).
   - A signature that does not verify (the manifest changed after signing), a revoked key, or a malformed signature file: **refused**, nothing written (`PACK_SIGNATURE`).
   - A trusted key: installed and loaded.
   - No signature, or a key this build does not know: installed and **quarantined**.
2. **The manifest.** `loom_data::pack::manifest` requires:
   - a known game;
   - plain, unique paths, none named `bundle.json` or `signature.json`;
   - paths portable to every file system the application runs on: unique ignoring case, no side file's name in any case, no segment ending in `.` or a space, no Windows device name (`con`, `nul`, `com1`, ...);
   - at most 10,000 files and 1 GiB in all.
3. **The listing, before anything is extracted** (`check_listing`). Every entry must be:
   - a regular file: not a folder, not a link (`PACK_PATH`);
   - at a plain relative path: no `..`, absolute path, drive, backslash, or empty segment, so no zip slip (`PACK_PATH`);
   - listed once (`PACK_DUPLICATE`);
   - either the manifest, the signature, or a manifest file (`PACK_UNLISTED`), every manifest file present (`PACK_MISSING`);
   - of the manifest's size by its declared size (`PACK_SIZE`), so a zip bomb's huge declared size is refused here.
4. **Extract and check.**
   - Each file goes into a temporary folder `bundles/<game>/.incoming-*` beside the target. Each read is capped at the manifest size plus one byte, whatever the archive claims.
   - Size and SHA-256 are checked as the file is written (`PACK_CONTENT`).
   - Any failure removes the temporary folder.
5. **One rename** to `bundles/<game>/<steam build>-s<schema>/`.
   - **The same manifest, its files intact** (full hashes): nothing is extracted. The signature is rewritten only when it is not weaker: a trusted pack's always, and an untrusted pack's only over an untrusted one.
   - **The same manifest, its files damaged:** extracted again, a repair. A trusted signature already there stays unless the pack's is trusted too.
   - **Another manifest:** replaced only by a pack signed by a trusted key. That covers a corrected rebuild of the same game build and schema, and an official pack over a user's unsigned one.
     - The new folder is extracted first; then the old one is moved aside (`.outgoing-*`), the new one moved in, and the old one removed.
     - If the new one cannot move in, the old one goes back.
     - An untrusted pack is refused (`APP_PACK_EXISTS`).
   - **Leftovers.** Temporary folders another process left (`.incoming-*`, `.outgoing-*`, after a crash or kill) are removed at the next install and at the start's scan.

## At every start, and on request
- **At start** (`loom_app::install::scan`), every installed folder's signature is checked against the trust list compiled into the build, and its files' sizes against the manifest.
- **The bundle that loads** then goes through the bundle validator with sizes only (`validate::Integrity::Sizes`), as plan decision P-e intends: references, domain rules, and the rest, but no hashes.
  - Its tables are loaded once: the validator hands over what it loaded.
  - A folder chosen by hand for development, or `LOOM_BUNDLE_DIR`, is not installed, so it is hashed in full.
  - Measured in `docs/phase-5/performance.md`, "Startup".
- **Verify** (About, Game data, **Verify**; command `verify-bundle`) checks every file's SHA-256 (`APP_BUNDLE_DAMAGED`).
- The installer checks hashes too, so a bundle is fully checked when it arrives and whenever the user asks.

| Status | Loads | Can the user accept it |
|---|---|---|
| signed by a trusted key | yes | not needed |
| `unsigned`, `unknown-key` | no, until accepted | yes, with the manifest's SHA-256 shown; `accepted.json` records the accepted hash, so a later change quarantines it again |
| `bad-signature`, `revoked`, `damaged` (a file missing or another size) | no | no |

- The validator does not count the side files, `signature.json` and `accepted.json`, as unlisted (`loom_data::pack::SIDE_FILES`).
- **Several versions side by side.** The choice is remembered in `settings.json`: that is the pin. **Rollback** is choosing an older installed one (About, Game data, **Use**). A build made with another installed bundle is offered it, never switched silently (B-1, ADR-010).
- `LOOM_BUNDLE_DIR` and **Choose a game data folder (unsigned, for development)** load a folder where it is, without a signature check. About says "a folder chosen for development (unsigned)".

## The test key
Debug and `e2e` builds trust the public test key (`loom_tauri::TEST_KEYS`, `loom_app::Config::with_test_keys`); a released build does not, and its smoke run checks it is neither debug nor `e2e`. Test packs: `loom-release pack <dir> --test-key`.

## The data feed
- **What it is.** `data-feed.json` on the GitHub release, with `data-feed.json.sig` beside it, signed with the data key.
  - A stable build reads the latest release's.
  - An experimental build reads the project site's copy of the newest pre-release's (`docs/release/channels.md`), since GitHub's `latest` never names a pre-release.
  - Each build's URLs are `loom_tauri::release::Channel::data_feed`. The feed:

  ```json
  { "format": 1, "sequence": 42, "bundles": [ { "id": "poe1/26530219-s4", "url": "https://github.com/.../game-data.loombundle", "sha256": "...", "size": 123 } ] }
  ```

- **Checking** (About, Game data, **Check for new game data**; command `check_data_feed`):
  - the feed and its signature are fetched, capped (`loom_integrations::data_feed`, HTTPS with `native-tls`);
  - `loom_data::pack::verify_feed` requires a trusted key: a feed is never quarantined (`FEED_SIGNATURE`);
  - it also requires format 1, HTTPS URLs, hex hashes, and sizes from 1 byte to 1 GiB (`FEED_MALFORMED`);
  - **a sequence not lower than the highest seen** (`FEED_ROLLBACK`); the highest is kept in `settings.json` (`data-feed-sequence`).
- **Downloading** (**Download and install**; `download_data`):
  - only a pack the last verified feed lists;
  - capped at its size, its size and SHA-256 checked against the feed, a failed download removed;
  - HTTPS only, redirects included;
  - a time limit that grows with the size: two minutes plus a second per 64 KiB. A whole-request limit would fail a large pack on a slow link, and the blocking client has no per-read timeout;
  - one download of a pack at a time (`FEED_BUSY`);
  - then installed like a pack from a file, with its own signature and every check above.
- **Strict offline** refuses both before any connection (`UPDATE_OFFLINE`); the update mode governs both kinds of network use.
- **Automatic** (maintainer decision 2026-11-24: game data is handled by the application, not by the user; `loom_tauri::data_updates`, 2026-11-25). One run reads the feed, then downloads and installs each pack this computer lacks:
  - **which packs** (`wanted`): per game the feed's newest (the feed lists newest first), unless a bundle of that game with the same or a newer Steam build and schema is here, loaded or quarantined. An older pack of the feed is never fetched;
  - **when**: at start in every update mode but strict offline (`at_start`, `starts`, after the installed data has loaded; the `manual` mode too since the maintainer's decision of 2026-11-25: its "Only when I ask" is about new versions of the application), and when the user asks (the start page's **Download game data**; command `fetch_data`). Strict offline never does: the start page then offers **Install game data from a file…**, the signed install;
  - **not** in a smoke run, an `e2e` build, a test (no updater plugin), or with `LOOM_DATA_DIR` set (development): those never connect on their own;
  - **retries**: a failed connection (`FEED_HTTP`) is tried again after 2 s and after 8 s; a refused feed (signature, format, rollback) is not;
  - **what the user sees**: the state (`idle`, `checking`, `downloading`, `done`, `failed`; app API v1 `DataAutoStatus`, additive; event `loom://data-auto`; query `data_auto_status`) on the start page's data line: "Looking for game data…", "Downloading game data…", then the data's name. A failure is a sentence on that line with **Try again**, only when no data is loaded; with data in use nothing is shown. Every failure is logged with its code and reason (`data feed check failed`, `automatic game data failed`);
  - **what is used**: a pack installed while no data is loaded is taken at once (`HostCommand::InstallBundle`); one installed beside loaded data is in use from the next start, as the start picks the newest;
  - the start page no longer leads to the unsigned development folder picker, which is on the About page only; `APP_NO_DATA` says to download the data.
- **Published by** the `release` workflow's draft job (`tools/package/data-feed.sh`), listing the shipped pack. The sequence is the workflow's run number, which only grows.

## The shipped bundle
CI cannot extract game data (ADR-005). The maintainer:
1. builds the bundle locally (`docs/handoff/SUCCESSOR-HANDOFF.md`, the data run);
2. zips the bundle folder and uploads the zip as an asset of a **draft** release, which stays a draft: the zip holds GGG's tree sprites unsigned, and ADR-005 Amendment 2 lets them ship only inside signed bundles. A draft's assets are visible only to the repository's members;
3. commits `release/bundle.json` with that asset's API URL:

   ```json
   { "url": "https://api.github.com/repos/DBAggie/Exiled-Studio/releases/assets/<asset id>", "sha256": "<the zip's SHA-256>" }
   ```

The release jobs fetch it with their own token (`GH_TOKEN`; the two build jobs have `contents: write` to see the draft). A plain `https://` URL still works, without a token.

**The first one (2026-11-16, for 0.9):** `bundle-poe1-26530219-schema4-evidence.zip` (steam build 26530219, schema 4, SHA-256 `ffa2dd01…dc973`), the asset of the draft release "Game data source: poe1 26530219 schema 4 (not for users)", tag name `data-poe1-26530219-s4` (never created: the release stays a draft). An anonymous request for the asset gets 404.

The `release` workflow's `tools/package/shipped-bundle.sh` then:
- downloads the zip and checks its hash;
- runs `bundle-check` on it;
- packs it signed with `LOOM_DATA_KEY` into `crates/loom-tauri/shipped/game-data.loombundle` (ignored by git);
- puts it in the installers' resources (`tauri.shipped.conf.json`, `bundles/`).

At start the application installs each shipped pack not installed yet (`bundles::Library::install_shipped`, `install::install_shipped`), then chooses the data to load. Both happen on the writer thread after the window is up, while the start screen says "Loading game data…" (`docs/phase-5/performance.md`, "Startup").
- **A pack already installed** is left as it is when the installed copy has the same manifest, a trusted signature, and every file at its listed size. Its files are not hashed, the same check as the start's scan; **Verify** hashes them.
- **Anything else** goes through the full install: a first start, an update that ships newer data, damage the sizes show, or an installed copy that is only accepted, not trusted.
- **Until `release/bundle.json` exists** nothing is shipped, and a tag's run fails: a release ships its data.

## Tests
| Where | What |
|---|---|
| `loom_data::pack` (unit) | listing checks: zip slip paths, links and folders, extra, missing, duplicate, and resized entries, the caps; trust (trusted, unknown, unsigned, changed, revoked); feed signature, unknown key, rollback |
| `crates/loom-app/tests/install.rs` | a signed pack installs, loads, and verifies, and installs again as a no-op; unsigned and unknown-key packs are quarantined and accepted only with the shown hash; a changed manifest or a revoked key is refused with nothing left; revoking an installed bundle's key quarantines it for good; zip slip entries are refused with nothing written; an extra entry and a bomb-sized file (8 MiB of zeros in a pack under 1 MiB) are refused by declared size; same-size other bytes are refused while extracting; another manifest with the same ID is refused; damage is caught by sizes at start or by hashes on verify; the start loads an installed bundle checking sizes, and any other folder hashing every file; a shipped pack installs once, is then checked by sizes, is repaired when the sizes show damage, and gives an accepted unsigned copy its trusted signature |
| `crates/loom-app/tests/workflow.rs` | install, quarantine, accept, use, and verify through the application; a test-signed pack loads at once; a damaged installed bundle is listed as quarantined |
| `loom_app::bundles` (unit) | only signed bundles are listed; a quarantined one cannot be used |
| `loom_integrations::data_feed` (unit, loopback) | fetch caps; a download's hash and size checked, nothing left on failure |
| `ui/src/test/bundles-install.test.tsx` | install signed and unsigned packs, the fingerprint, accept, a bad signature offers no accept, verify, the feed's offers and download, strict offline, axe |
| e2e (`ui/e2e/workflow.mjs`) | the real application installs a test-signed pack of the synthetic tree bundle from the picker and shows it signed. Run on this workstation 2026-11-15 (`run.ps1`): passed |

## Not done here
- **No feed expiry.** A frozen feed carrying the highest sequence seen is accepted for as long as it is served. A signed `expires` field, and a context prefix in the signed bytes so manifest and feed signatures cannot be confused, wait for the next feed format.
- **The real data feed and shipped bundle** wait for the maintainer's `release/bundle.json`, data key, and a tagged release.
- **Removing an installed bundle from the UI** is not offered. Folders can be deleted by hand while the application is closed.
