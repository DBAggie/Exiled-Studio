# Signing runbook (Phase 5, P2)

The plan's step P2 and decision P-c (`dev/active/passive-tree-and-phase-5/`). Two signatures protect what the application downloads; neither is OS code signing (none, maintainer MQ4; `docs/phase-5/packaging.md`).

| Key | Signs | Verified by | Secret in the `release` environment |
|---|---|---|---|
| **Data key** (Ed25519) | data bundle manifests (`bundle.json`) and the data feed (P4) | `loom_data::signature`, against the trust list compiled into the app | `LOOM_DATA_KEY` |
| **Updater key** (Tauri's minisign key) | application update packages (P3, `docs/phase-5/updates.md`) | `tauri-plugin-updater`, against `release/updater.pub`, compiled into the app | `LOOM_UPDATER_KEY`, `LOOM_UPDATER_KEY_PASSWORD` |

## Data signatures
- **Format.** A detached `signature.json` beside the signed file:

  ```json
  { "format": 1, "algorithm": "ed25519", "key_id": "ed25519:6e846054cb5c7102", "signature": "<128 hex digits>" }
  ```

  It signs the file's exact bytes; nothing is canonicalised. The key ID is `ed25519:` and the first 16 hex digits of the SHA-256 of the 32-byte public key.
- **Verification** (`loom_data::signature::verify`, wasm32-safe): the format and algorithm must be known, the key must be in the trust list and not revoked, and the signature must verify under `verify_strict` (no malleable or small-order signatures).
- **Trust list** (`release/trust.json`, compiled into `loom-app` by `loom_app::trust`):

  ```json
  { "format": 1, "keys": [ { "id": "...", "public_key": "<64 hex>", "purpose": "data", "note": "..." } ], "revoked": [ { "key_id": "...", "reason": "..." } ] }
  ```

  A key whose ID does not match its public key, or that is listed twice, makes the list fail to parse (a test guards the committed list). Until the maintainer generates the first key, `keys` is empty and a release build trusts nothing.

## The test key
- Its seed is the SHA-256 of `loom-test-data-key` (`KeyPair::test_key`), so anyone can sign with it. Fixtures, the e2e test bundle, and local work use it.
- `release/trust-test.json` lists it (`loom-release test-trust` writes the file; a test checks they agree).
- **Only debug and `e2e` builds trust it.** `loom-tauri` asks for it with `cfg!(debug_assertions) || cfg!(feature = "e2e")`; the smoke run checks that a released build is neither (`docs/phase-5/packaging.md`, "Smoke run").

## The release tool
`tools/release` (`loom-release`, maintainer and workflow use; the application never holds a secret key). A secret is read from a file (`--secret <file>`) or an environment variable (`--secret-env <NAME>`), never from an argument.

```sh
cargo run -p loom-release -- keygen --out <offline dir> --note "data key 2026-1"
cargo run -p loom-release -- trust --secret <file>                 # the trust entry of a key
cargo run -p loom-release -- sign bundle.json --secret-env LOOM_DATA_KEY --out signature.json
cargo run -p loom-release -- verify bundle.json signature.json --trust release/trust.json
cargo run -p loom-release -- test-sign bundle.json --out signature.json   # the test key
cargo run -p loom-release -- trust-add release/trust.json --secret <file>   # a key into a trust list
cargo run -p loom-release -- pack <bundle dir> --secret <file> --out game-data.loombundle
cargo run -p loom-release -- feed --sequence <n> --base-url https://.../download/vX --out data-feed.json <pack>...
```

## First keys (the maintainer, once)
1. **Data key.** On an offline machine (or one you trust, with no sync folder): `loom-release keygen --out <dir> --note "data key <year>-1"`. It writes `<dir>/ed25519-<id>.secret` and prints the trust entry.
   - Keep two copies of the secret offline (for example, two USB drives in different places). Losing it means rotating; leaking it means revoking.
   - Add the printed entry to `release/trust.json` in a PR to `dev`.
   - Put the secret's 64 hex digits into the `release` environment's secret `LOOM_DATA_KEY`.
2. **Updater key.** `ui/node_modules/.bin/tauri signer generate -w <dir>/updater.key` (with a password).
   - The public key (the one line of `updater.key.pub`) goes into `release/updater.pub`, in a PR to `dev`. Until then a build cannot check for updates.
   - The private key file's content goes into `LOOM_UPDATER_KEY`, its password into `LOOM_UPDATER_KEY_PASSWORD`.
   - Losing it strands installed copies: they accept only updates signed by the key they shipped with. Keep two offline copies.

## Secrets in the release workflow
- **Never in the job's environment.** Each key step gets its secret alone and writes it to a file in `RUNNER_TEMP/keys` (`tools/package/data-key.sh`, `updater-key.sh`). So `cargo build`, `pnpm`, and the other build steps do not carry it in their environment.
- **The data key** is used once per build job, by `shipped-bundle.sh` to sign the shipped pack, and deleted right after.
- **The update key** reaches only the `tauri build` commands, through `tools/package/with-updater-key.sh`.
- **The draft job's feed step** gets the data key alone.
- **What this does not stop.** A build step can still read the update key's file while the job runs. Full isolation would need signing in a job of its own; that is a later change if the threat warrants it.
- **The update key pair is checked.** In a real run, `updater-key.sh` signs a probe file with the secret and compares the signature's key ID with `release/updater.pub`'s. A mismatch fails the run: installed copies would refuse every update signed with it.

## Release environment (GitHub)
Settings → Environments → New environment `release`:
- **Required reviewers:** the maintainer. Every `release` workflow job waits for approval.
- **Deployment branches and tags:** selected; `main`, `experimental`, and the tag pattern `v*` (`docs/release/channels.md`). A pull request's workflow can never reach the secrets.
- **Secrets:** `LOOM_DATA_KEY`, `LOOM_UPDATER_KEY`, `LOOM_UPDATER_KEY_PASSWORD`.
- **Tag protection** (Settings → Rules → Rulesets): only the maintainer creates `v*` tags.

## Throwaway mode
The update key works the same way (`tools/package/updater-key.sh`): without `LOOM_UPDATER_KEY`, a throwaway key from `tauri signer generate` signs the run's update packages and its public half goes into the run's `release/updater.pub`; a tag's run refuses it; with the secret, `release/updater.pub` must not be empty.

When `LOOM_DATA_KEY` is absent (before the first key exists, or in a fork), `tools/package/data-key.sh` makes a throwaway key and adds it to the run's copy of `release/trust.json` (never committed). The artifacts then verify their own data, so the workflow is fully exercised, but no later build trusts that key. **A tag's run refuses throwaway mode.** With the secret set, its key must already be in `release/trust.json` and not revoked, or the run fails.

## Rotation
1. Generate the new key (as above) and add its entry to `release/trust.json`, keeping the old one.
2. Release a version carrying both; switch `LOOM_DATA_KEY` to the new secret.
3. After users have had time to update (at least one release cycle), remove the old key in a later release. Bundles signed by it stop verifying only in versions that no longer list it.

**The update key** cannot be rotated by a list: an installed copy accepts only packages signed with the key it was built with. Rotate it with a release signed with the old key whose `release/updater.pub` is the new key; every later release is signed with the new key. Copies that skip that release must update by hand.

## Revocation (a leaked key)
1. Add `{ "key_id": "<id>", "reason": "<why>" }` to `revoked` in `release/trust.json`, and remove the key from `keys`. Revocation is checked before anything else, so a revoked key fails even if a later list lists it again.
2. Generate a new key and switch the secret (Rotation, steps 1 and 2) in the same release.
3. Re-sign the shipped bundles and the feed with the new key (P4); the feed's monotonic sequence keeps a captured old feed from being replayed.
4. An installed version that predates the revocation still trusts the key until it updates; say so in the release notes.

## SmartScreen
The installers are not code signed (MQ4). Windows shows "Windows protected your PC" on the first run of a downloaded setup; the user clicks **More info**, then **Run anyway**. The README's install section says this. Data and update signatures do not change it.
