# Release channels and the download page

The maintainer's request on 2026-11-15:
- **Stable:** `main` always holds the stable application.
- **Experimental:** a protected `experimental` branch receives approved states of `dev` and holds the experimental installers.
- **Download page:** a page on GitHub Pages lets users find and download the installers.

## Branches
| Branch | Holds | Protection |
|---|---|---|
| `dev` | integration: every PR targets it | none (PRs merge into it) |
| `experimental` | approved states of `dev`; experimental releases are tagged here | same as `main` (set 2026-11-15) |
| `main` | the stable application; stable releases are tagged here | required checks `ci-ok` and `PR evidence`, up to date before merging, linear history, conversations resolved, rules for admins too, no force push, no deletion |

- **The flow.** The maintainer merges `dev` into `experimental` by a PR when a state of `dev` is approved for testers. The maintainer merges into `main` by a PR when a state is ready to be stable: usually `experimental`'s, after testing.
- **CI.** It runs on every PR, whatever its base, and on pushes to all three branches.
- **Made 2026-11-15** from `dev` at `7644be8`. Its protection was copied from `main`'s and checked equal.

## Channels
| | Stable | Experimental |
|---|---|---|
| Tag | `v1.2.0` on `main` | `v1.2.0-3` on `experimental` |
| GitHub release | a release | a pre-release |
| Application | Exiled Studio (`io.github.dbaggie.exiledstudio`) | Exiled Studio Experimental (`io.github.dbaggie.exiledstudio.experimental`) |
| Binary | `exiled-studio` | `exiled-studio-experimental` |
| Update feed | `releases/latest/download/latest.json` | `https://dbaggie.github.io/Exiled-Studio/experimental/latest.json` |
| Data feed | `releases/latest/download/data-feed.json` and `.sig` | `https://dbaggie.github.io/Exiled-Studio/experimental/data-feed.json` and `.sig` |
| MCP token in the OS store | `Exiled Studio MCP server` | `Exiled Studio Experimental MCP server` |

- **The experimental application is another application.** It has its own name, install folder, MSI upgrade code, data folder (settings, recovery, recent builds, installed game data, logs), and MCP token. So it installs beside the stable one and never replaces it, and a tester's experimental build files never reach the stable application's folders.
  - One thing is shared: both use the MCP port from their own settings, which start at the same default. Running both with the MCP server on needs another port in one of them.
- **Updates stay within a channel.** Each application checks only its own feed, so a stable installation is never offered an experimental version.
- **GitHub's `latest` never names a pre-release.** So the experimental feeds cannot be `releases/latest/download/...`. The site copies them from the newest published pre-release (below). The packages they point to are that release's assets, and the updater checks each package's signature against the update key (`docs/phase-5/updates.md`). The site is only a pointer.
- **Versions.** An experimental version carries a numeric pre-release: `1.2.0-3`. The MSI needs a number there and takes it as its fourth field (`1.2.0.3`, at most 65535). The updater compares versions by SemVer, so `1.2.0-3` < `1.2.0-4` < `1.2.0`.
  - The version still moves only in a release commit on `dev` (`docs/phase-5/packaging.md`, "Version policy"). That commit sets `1.2.0-3` for an experimental release, or `1.2.0` for a stable one.

### How a build gets its channel
- **`LOOM_CHANNEL`** at build time: unset or `stable`, or `experimental`.
  - `crates/loom-tauri/build.rs` turns `experimental` into `cfg(loom_experimental)` and refuses any other value.
  - `loom_tauri::release::Channel::CURRENT` picks the data feed and the token entry from it.
  - The smoke run's build info reports the channel, and `tools/package/smoke.ps1` and `smoke.sh` check it against `LOOM_CHANNEL`.
- **`crates/loom-tauri/tauri.experimental.conf.json`** carries the name, binary, identifier, window title, and update endpoint.
  - `tauri build` takes it with `--config`.
  - The portable zip's plain `cargo build` takes it from `TAURI_CONFIG`.
  - `crates/loom-tauri/tests/release_config.rs` checks that it differs from the stable configuration only where it must.

### The release workflow
`tools/package/channel.sh` runs first and decides the channel from the ref:

| Ref | Channel | Checks |
|---|---|---|
| tag `v1.2.0` | stable | the tag equals the workspace version; the commit is on `main` |
| tag `v1.2.0-3` | experimental | the tag equals the workspace version; the pre-release is a number up to 65535; the commit is on `experimental` |
| any other tag `v*` | refused | |
| `experimental` by hand | experimental | nothing is released |
| any other branch by hand | stable | nothing is released |

- **The build jobs** build with `LOOM_CHANNEL` and the channel's configuration. The packaging scripts name their files and folders after the channel's product.
- **The draft job** drafts an experimental release as a pre-release titled "Exiled Studio Experimental 1.2.0-3".
- **The `release` environment** must allow `experimental` as well as `main` and `v*` tags (`docs/release/signing.md`, "Release environment").

## The download page
`https://dbaggie.github.io/Exiled-Studio/`

- **What it shows.** The newest published stable release and the newest published experimental one, each with:
  - its version, date, and release notes link;
  - a download button per platform (the NSIS setup, the AppImage);
  - a table of every installer with its size and SHA-256 (from GitHub's asset digest).
- **What it leaves out.** Drafts are never shown. Signatures, feeds, and data packs are not listed. A channel without a release says so and links to all releases.
- **The page itself.** It is plain HTML and CSS, with no script and nothing loaded from elsewhere. It has no counter or analytics (ADR-011). GitHub, which hosts it, sees the visitor's IP address, as for any GitHub page.
- **Sources:** `site/index.html` (the template), `site/style.css`, and `tools/site/build.py`, which fills the template from GitHub's release list. Its tests are `tools/site/test_build.py`, run by CI's `site` job.
- **Workflow:** `.github/workflows/pages.yml`.
  - It runs when a release is published, edited, or deleted, when the site's sources change on `main`, and by hand.
  - It builds the page and copies the newest pre-release's `latest.json`, `data-feed.json`, and `data-feed.json.sig` into `experimental/`.
  - It deploys to the `github-pages` environment.
  - A release event runs the workflow as it is on `main`, so the site starts working once this reaches `main`.
- **Settings, made 2026-11-15:**
  - Pages is on, built by GitHub Actions.
  - The `github-pages` environment deploys from `main` and from tags `v*`. A release event's ref is its tag.
- **Cache.** GitHub's servers cache a page for about ten minutes, so a new release appears on the page, and reaches the experimental application's update check, up to ten minutes after the workflow finishes.

## Releases
The maintainer named the first release 0.9 (2026-11-16). One state of `dev` goes to both channels, through two release commits, since a tag must equal the workspace version:

| Version | Release commit on `dev` | Branch | Tag |
|---|---|---|---|
| `0.9.0-1` | PR #119 (keys, shipped data, version) | `experimental` | `v0.9.0-1`, a pre-release |
| `0.9.0` | branch `chore/release-0.9.0-stable` (version only), merged after `experimental`'s PR #120 | `main` | `v0.9.0`, a release |

- **`dev` must never be deleted.** The repository deletes a PR's head branch when it merges, so merging #120 (`dev` into `experimental`) deleted `dev` (2026-11-16; restored at once at `b89587b`, nothing lost; #121 had been retargeted to `experimental` and was moved back). The ruleset "dev: never deleted" now blocks it.
- **The first `v0.9.0-1` run failed** at its last bundling step: `tauri build` reads the updater public key from the configuration, which is empty in `tauri.conf.json`. `tools/package/updater-key.sh` now writes `release/updater.pub` into the run's copy of `tauri.updater.conf.json`. The fix went to `experimental` by its own PR (a merge of `dev` would have brought version `0.9.0`) and to `dev` by another; `v0.9.0-1`, never published, was moved to the fixed commit.
- **Merging `dev` into a protected branch.** Both allow only squash merges with a linear history. So a squash commit on `experimental` or `main` is not an ancestor of `dev`, and the next merge's base stays where the branch was made. Later merges may then conflict where `dev` changed lines the earlier squash had also changed. If that happens, resolve it in the PR from `dev` by taking `dev`'s side: the target branch only ever holds states of `dev`.
