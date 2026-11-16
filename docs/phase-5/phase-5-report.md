# Phase 5 report: packaging, signing, updates

The Phase 5 steps of the passive tree and Phase 5 plan (`dev/active/passive-tree-and-phase-5/`), done in one sprint at the maintainer's request (2026-11-15: "perform all open p5 steps in this next sprint"). P5 (logs, crash records, the diagnostic package) was its own PR, #115. P1 to P4 and P6 to P8 are one PR, #118, from `feat/p5-release`.

**Gate status: not met yet.** Every CI item passes on this workstation, and CI runs them on the PR. Two maintainer items remain:
- the `release` workflow's recorded run (gate 6);
- the maintainer's decision on the remaining performance exceptions (gate 9). Startup, the main one, is met since the startup fix.

The real keys and the shipped bundle are the maintainer's too. The fallback lets gates 6 and 7 pass on throwaway keys.

## What was built
| Step | What | Document |
|---|---|---|
| P1 packaging | NSIS setup (with the offline WebView2 installer), MSI, portable zip, AppImage, `.deb`; one version policy; `--smoke`; the `package` job on every PR; the `release` workflow that installs, starts, and uninstalls each artifact and drafts a release on a tag | `docs/phase-5/packaging.md` |
| P2 signing | Ed25519 data signatures (`loom_data::signature`), trust lists with revocations compiled in (`release/trust.json`, `trust-test.json`), the release tool `loom-release`, throwaway keys when the secrets are absent, the runbook | `docs/release/signing.md` |
| P3 app updates | `tauri-plugin-updater` with `native-tls`; modes automatic check, manual, strict offline; no downgrade; the setup, the MSI, and the AppImage install on approval; the `.deb` and the portable zip notify only | `docs/phase-5/updates.md` |
| P4 data bundles | `.loombundle` packs installed as hostile input (signature first, listing checks, capped extraction with hashes, one rename); quarantine and acceptance; Verify; versions side by side and rollback by choice; the signed data feed with no rollback; the shipped bundle from `release/bundle.json` | `docs/phase-5/data-bundles.md` |
| P5 logs | local logs with redaction, crash records, the diagnostic package previewed before saving (PR #115) | `docs/phase-5/logs.md` |
| P6 security | cargo-deny advisories; gitleaks over the history; six fuzz targets with a weekly workflow; proptest properties; the capability allow-list | `docs/phase-5/security.md` |
| P7 performance | `loom-bench`; CI instruction counts against a baseline; the reference tier measured here; the startup fix | `docs/phase-5/performance.md` |
| Channels and the download page | stable from `main`, experimental from the protected `experimental` branch as a separate application; the download page on GitHub Pages (maintainer's request, 2026-11-15) | `docs/release/channels.md` |

## Gate
| # | Item | Status |
|---|---|---|
| 6 | The `package` job's smoke runs (CI); one recorded `release` run installing, starting, and uninstalling the five artifacts, `e2e` absent | `package`: in CI on every PR, first run on this PR. The portable zip's smoke run passed here (P1). **The recorded `release` run is the maintainer's.** `release.yml` reaches `workflow_dispatch` once it is on the default branch. Until the `release` environment has its secrets, it runs on throwaway keys (fallback). |
| 7 | Update, signature, rollback, quarantine, revocation, downgrade, feed-rollback, zip-slip, zip-bomb tests (CI) | pass here: `crates/loom-tauri/tests/updater.rs`, `crates/loom-app/tests/install.rs`, `loom_data::pack` and `signature` units, the properties; in CI's `rust` job |
| 8 | Security: no unignored RustSec advisory, no gitleaks finding, no open fuzz crash | advisories: two ignores with reasons, both on Tauri's Linux GTK3 path, otherwise clean; gitleaks: no leaks in 170 commits; properties: 20,000 cases each, no failure; fuzzing: this PR's `fuzz.yml` run (43779992583), six targets at 10 minutes each, 1.4 to 10.4 million runs, no crash (`security.md`, "Records"). libFuzzer does not link on this Windows workstation, so fuzzing runs on Linux CI. **Met.** |
| 9 | Budgets met on the reference tier or exceptions listed (MR); CI instruction-count check passes | Startup with the real bundle (1.77 s median against 2.0 s, after the startup fix), import and first calculation, item-edit preview, allocation preview, and the application's memory are inside budget. **Exceptions for the maintainer:** the NSIS setup's size, tree paint and fps (to measure), version conversion (no feature yet), and memory counted without WebView2. The instruction-count baseline is committed from this PR's first `bench` run (43779626395), and the `bench` job passes against it. |
| 10 | Logs, redaction, diagnostic preview, crash record tests (CI) | pass (PR #115, in CI's `rust` job) |

## Found on the way
- **The data feed commands ran reqwest's blocking client on Tauri's async runtime, which panics.** The command-list test caught it. Both commands now do their work on a blocking thread, and the test runs every command in strict offline, so it never reaches the network.
- **The start hashed the loaded bundle in full, and loaded its tables twice.** `GameData::load` ran the bundle validator, which hashes every file, then loaded the tables again. Plan decision P-e meant the start to check sizes only. Fixed at the maintainer's request (below).
- **libFuzzer does not link on Windows MSVC here** (`__start___sancov_cntrs` unresolved). Fuzzing runs on Linux in `fuzz.yml`, and CI checks that the targets build.

## Review
`code-architecture-reviewer` reviewed the branch on 2026-11-15. It found no security hole in the hostile-input path, and no breach of the crate boundaries or ADRs.

**Fixed in this PR:**
- **Same-ID packs.** A pack with an installed ID and another manifest is now replaced when it is signed by a trusted key, extracted first and then swapped. Re-installing repairs damaged files. A weaker signature never replaces a trusted one.
- **Download timeouts.** Downloads have a time limit that grows with the size. An update's download, which the plugin also times, gets an hour.
- **Secrets.** They no longer enter the release jobs' environment. The update key pair is checked against `release/updater.pub`.
- **Portable names.** Manifest paths are checked for names that alias on Windows: case, trailing dots, device names.
- **Leftovers.** Temporary folders are swept.
- **Overflow.** Size arithmetic saturates, and a feed pack's size is capped.
- **HTTPS.** The feed client is HTTPS only, redirects included.
- **Busy guards.** A download or an install cannot run twice at once, and a dropped check clears its busy flag.
- **Baseline.** A missing `bench` baseline fails in CI.
- **Docs.** The documents now match the code.

**Left, recorded:**
- feed expiry and signature domain separation, for the next feed format (`data-bundles.md`, "Not done here");
- the update key readable by build steps through its file during a release job (`signing.md`);
- the fuzz workspace outside the advisories job (`security.md`).

## After the review: the maintainer's requests (2026-11-15)
"Make performance improvements you would recommend", and a download page with a stable and an experimental channel.

- **The startup fix** (`performance.md`, "Startup").
  - An installed bundle is checked by sizes at start, not hashes; its hashes were checked at install and are checked on Verify.
  - The validator hands over the tables it loaded.
  - The window comes up before the data, which loads on the writer thread; the start screen says "Loading game data…".
  - Startup with the real bundle went from a median of 3.11 s to 1.77 s, under the 2.0 s budget.
  - A new `bench` scenario, `start`, with its baseline from CI run 43787174465 (5,679,942 instructions).
- **Channels** (`docs/release/channels.md`).
  - `main` holds the stable application. The new `experimental` branch, made from `dev` and protected like `main`, holds the experimental one.
  - An experimental release is a pre-release tagged `v1.2.0-3` on `experimental`. It builds Exiled Studio Experimental: another application, with its own identifier, data folder, MCP token, and update and data feeds.
  - `tools/package/channel.sh` refuses a tag on the wrong branch.
- **The download page:** `https://dbaggie.github.io/Exiled-Studio/`.
  - It is built from GitHub's releases by `tools/site/build.py`, and published by `pages.yml` when a release changes. It also serves the experimental feeds.
  - Pages is on, and its environment deploys from `main` and `v*` tags. The page goes live once `pages.yml` reaches `main`.

## For the maintainer
1. **Keys and secrets:** done 2026-11-16 for 0.9 (`docs/release/signing.md`, "First keys"). Left: copy the private keys offline twice, then delete the workstation folder if you like.
2. **The shipped bundle:** done 2026-11-16 (`docs/phase-5/data-bundles.md`, "The shipped bundle": a draft release's asset).
3. **Gate 6:** run `release.yml` by hand once it reaches `main`, and record the run in `packaging.md`.
4. **Gate 9:** accept or refuse the NSIS size exception; measure tree paint and pan and zoom fps in the release app; say whether peak memory includes WebView2. (Startup is met.)
5. **After two releases:** a real update from one to the next (`updates.md`, "Not tested here").
6. **SmartScreen:** the installers are not code signed (MQ4); the README says how to run them.
7. **The release environment** allows `experimental` (made 2026-11-16). The first releases are 0.9: `v0.9.0-1` on `experimental`, `v0.9.0` on `main` (`docs/release/channels.md`, "Releases").

## Not in Phase 5
The passive tree steps below the cut line (T3 cluster jewels, T4 older versions, T5b, T8's corpus run) are their own PRs.
