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
   - An ID installed with the same manifest: nothing is extracted, only the signature is rewritten (a re-signed pack).
   - An ID installed with another manifest: refused (`APP_PACK_EXISTS`).

## At every start, and on request
- **At start** (`loom_app::install::scan`), every installed folder's signature is checked against the trust list compiled into the build, and its files' sizes against the manifest.
- **The bundle that loads** then goes through the full bundle validator (`GameData::load`), which checks every file's SHA-256 too. Plan decision P-e meant the start to check sizes only. The validator's cost is measured in `docs/phase-5/performance.md` (exception 1), and changing it is the maintainer's choice.
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
- **What it is.** `data-feed.json` on the GitHub release, with `data-feed.json.sig` beside it, signed with the data key:

  ```json
  { "format": 1, "sequence": 42, "bundles": [ { "id": "poe1/26530219-s4", "url": "https://github.com/.../game-data.loombundle", "sha256": "...", "size": 123 } ] }
  ```

- **Checking** (About, Game data, **Check for new game data**; command `check_data_feed`):
  - the feed and its signature are fetched, capped (`loom_integrations::data_feed`, HTTPS with `native-tls`);
  - `loom_data::pack::verify_feed` requires a trusted key: a feed is never quarantined (`FEED_SIGNATURE`);
  - it also requires format 1, HTTPS URLs, and hex hashes (`FEED_MALFORMED`);
  - **a sequence not lower than the highest seen** (`FEED_ROLLBACK`); the highest is kept in `settings.json` (`data-feed-sequence`).
- **Downloading** (**Download and install**; `download_data`):
  - only a pack the last verified feed lists;
  - capped at its size, its size and SHA-256 checked against the feed, a failed download removed;
  - then installed like a pack from a file, with its own signature and every check above.
- **Strict offline** refuses both before any connection (`UPDATE_OFFLINE`); the update mode governs both kinds of network use.
- **Published by** the `release` workflow's draft job (`tools/package/data-feed.sh`), listing the shipped pack. The sequence is the workflow's run number, which only grows.

## The shipped bundle
CI cannot extract game data (ADR-005). The maintainer:
1. builds the bundle locally (`docs/handoff/SUCCESSOR-HANDOFF.md`, the data run);
2. zips the bundle folder and uploads the zip as an asset of a release (a draft is fine);
3. commits `release/bundle.json`:

   ```json
   { "url": "https://github.com/DBAggie/Exiled-Studio/releases/download/<tag>/bundle-poe1-<build>-schema<n>.zip", "sha256": "<the zip's SHA-256>" }
   ```

The `release` workflow's `tools/package/shipped-bundle.sh` then:
- downloads the zip and checks its hash;
- runs `bundle-check` on it;
- packs it signed with `LOOM_DATA_KEY` into `crates/loom-tauri/shipped/game-data.loombundle` (ignored by git);
- puts it in the installers' resources (`tauri.shipped.conf.json`, `bundles/`) and in the portable zip's `bundles\`.

At start the application installs each shipped pack not installed yet (`loom_tauri::install_shipped`). The first one installed is loaded at once when no data is.
- **Until `release/bundle.json` exists** nothing is shipped, and a tag's run fails: a release ships its data.

## Tests
| Where | What |
|---|---|
| `loom_data::pack` (unit) | listing checks: zip slip paths, links and folders, extra, missing, duplicate, and resized entries, the caps; trust (trusted, unknown, unsigned, changed, revoked); feed signature, unknown key, rollback |
| `crates/loom-app/tests/install.rs` | a signed pack installs, loads, and verifies, and installs again as a no-op; unsigned and unknown-key packs are quarantined and accepted only with the shown hash; a changed manifest or a revoked key is refused with nothing left; revoking an installed bundle's key quarantines it for good; zip slip entries are refused with nothing written; an extra entry and a bomb-sized file (8 MiB of zeros in a pack under 1 MiB) are refused by declared size; same-size other bytes are refused while extracting; another manifest with the same ID is refused; damage is caught by sizes at start or by hashes on verify |
| `crates/loom-app/tests/workflow.rs` | install, quarantine, accept, use, and verify through the application; a test-signed pack loads at once; a damaged installed bundle is listed as quarantined |
| `loom_app::bundles` (unit) | only signed bundles are listed; a quarantined one cannot be used |
| `loom_integrations::data_feed` (unit, loopback) | fetch caps; a download's hash and size checked, nothing left on failure |
| `ui/src/test/bundles-install.test.tsx` | install signed and unsigned packs, the fingerprint, accept, a bad signature offers no accept, verify, the feed's offers and download, strict offline, axe |
| e2e (`ui/e2e/workflow.mjs`) | the real application installs a test-signed pack of the synthetic tree bundle from the picker and shows it signed. Run on this workstation 2026-11-15 (`run.ps1`): passed |

## Not done here
- **The real data feed and shipped bundle** wait for the maintainer's `release/bundle.json`, data key, and a tagged release.
- **Removing an installed bundle from the UI** is not offered. Folders can be deleted by hand while the application is closed.
