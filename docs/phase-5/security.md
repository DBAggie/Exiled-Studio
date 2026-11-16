# Security (Phase 5, P6)

The plan's step P6 and decision P-g (`dev/active/passive-tree-and-phase-5/`). **Blocker:**
- a RustSec advisory not ignored with a reason;
- a gitleaks finding;
- an open fuzz crash.

Gate 8: no blocker, by CI and the fuzz records below.

## What the application trusts, and what it does not
| Input | From | Handled by | Checks |
|---|---|---|---|
| A PoB XML file, a share code | the user opens or pastes it | `loom_serialization::legacy::{pob_xml, share_code}` | a capped zlib inflate (32 MiB), typed errors, no panic (properties, fuzz) |
| A build document | the user opens it | `loom_serialization::dto::v1` | the schema's types, migrations; no panic (properties, fuzz) |
| A pasted item | the clipboard | `legacy::game_copy` | typed reading, unknown lines kept as reasons (ADR-015) |
| A data bundle pack | a file the user picks, the shipped pack, a feed download | `loom_app::install` | signature first, then the listing, then capped extraction with hashes (`docs/phase-5/data-bundles.md`) |
| The data feed | GitHub, over HTTPS | `loom_data::pack::verify_feed` | a trusted signature, format, HTTPS URLs, no lower sequence |
| An application update | GitHub, over HTTPS | `tauri-plugin-updater` | the minisign signature with the compiled-in key; only a greater version (`docs/phase-5/updates.md`) |
| An MCP request | a local client with the token | `loom_integrations::mcp` | loopback only, the bearer token, approval for changes (ADR-017) |
| The webview | the bundled UI | Tauri capabilities | events only; every command is the shell's own (below) |

**Network paths:** the update check, the data feed (both off in strict offline), and the MCP server on loopback. There is no telemetry (ADR-011).

## Advisories
- CI's `advisories` job runs `cargo deny --all-features --locked check advisories` (cargo-deny 0.20.2, `deny.toml`). It is required in `ci-ok`.
  - Vulnerabilities always fail.
  - Unmaintained and unsound crates fail at any depth, and yanked versions fail too.
- The advisory database changes without a code change, so a new advisory can turn any PR red. That is the plan's blocker: it is fixed, or ignored with its reason, before anything merges.

| Ignored | Crate | Why |
|---|---|---|
| RUSTSEC-2024-0370 | proc-macro-error 1.0.4, unmaintained | a build-time proc macro of `glib-macros`, through Tauri's GTK3 bindings (`gtk` 0.18, `muda`), Linux only; no runtime code |
| RUSTSEC-2024-0429 | glib 0.18.5, unsound `VariantStrIter` | the same GTK3 path, Linux only; our code never calls glib; the fix (glib 0.20) waits for Tauri to leave gtk-rs 0.18 |

Run here 2026-11-15: `advisories ok, bans ok, licenses ok, sources ok`. The two ignores were the only findings.

The fuzz workspace (`fuzz/Cargo.lock`) is not checked by this job. Its targets are never shipped, and their dependencies beyond the shipped crates' are `libfuzzer-sys` alone.

## Secrets
- CI's `secrets` job runs gitleaks 8.30.1 over the whole history (`gitleaks git --redact --exit-code 1`), required in `ci-ok`.
  - The binary is pinned by version and by the SHA-256 in the release's checksums file.
  - Default rules, no allow-list.
- The updater test key in `crates/loom-tauri/tests/fixtures/updater/` is password-encrypted, signs nothing real, and is not flagged.
- The data test key is derived from a public phrase (`loom_data::signature::TEST_KEY_PHRASE`) and has no secret file.
- Run here 2026-11-15 with the same version (Windows build, checksum checked): 170 commits, no leaks.

## Fuzzing and properties
**Fuzz targets** in `fuzz/`, its own workspace (cargo-fuzz and libFuzzer):

| Target | Entry point |
|---|---|
| `pob_xml` | `pob_xml::import` |
| `share_code` | `share_code::import` |
| `build_document` | `dto::v1::from_json` |
| `signature` | `pack::trust_of` (parses the signature file, then `signature::verify`) against the test key |
| `install_pack` | `install::install_from` into a fresh folder: whatever installs must verify file by file, and no `.incoming-*` folder may remain |
| `data_feed` | `pack::verify_feed` against the test key |

- **Two-part inputs.** `signature` and `data_feed` read `<first length: u16 LE><first><second>`.
- **Seeds.** `fuzz/seed.sh` seeds `fuzz/corpus/` from the public fixtures, including packs, a signature, and a feed signed with the test key.
- **Install target.** `install::install_from` takes any `Read + Seek`. `install_pack` opens the file and calls it.

**Where they run:**
- **CI's `fuzz-build`** (required) checks on every change that the targets build.
- **`.github/workflows/fuzz.yml`** fuzzes each target for 10 minutes on Linux:
  - weekly, by hand, and on any PR that changes `fuzz/` or the workflow;
  - on pinned nightly `nightly-2026-11-01` with cargo-fuzz 0.13.2;
  - a crash fails the run and uploads its input.
- **Not on this workstation.** libFuzzer does not link on Windows MSVC here (`__start___sancov_cntrs` unresolved), and there is no WSL or Docker.

**Properties** (`proptest`, in CI's `rust` job on every change) drive the same entry points with random and mutated real input:

| File | Properties |
|---|---|
| `crates/loom-data/tests/properties.rs` | any manifest bytes and any listing: no panic; hostile entry names (`..`, absolute, drive, backslash, `.`, empty segment) are `PACK_PATH`; a size other than the manifest's is refused; a manifest changed after signing is never trusted; any signature fails without panic; any feed bytes are refused; a signed feed passes exactly when its sequence is not below the highest seen |
| `crates/loom-serialization/tests/properties.rs` | any text and changed real documents: PoB XML, share codes, build documents read or are refused, no panic; a share code decodes to the text it encodes |
| `crates/loom-app/tests/pack_properties.rs` | any bytes, and a real signed pack with up to four bytes changed: refused or installed whole, what installs verifies file by file, no `.incoming-*` left |

### Records
| Date | Where | What | Result |
|---|---|---|---|
| 2026-11-15 | this workstation | the 15 properties of `loom-data` and `loom-serialization` at 20,000 cases each (`PROPTEST_CASES=20000`, release, 72 s with the build), not coverage-guided; the pack properties at their fixed 128 cases | all passed, no failure |
| 2026-11-15 | CI, PR #118's `fuzz.yml` run 43779992583 at `f7fd002` (ubuntu, nightly-2026-11-01) | 6 targets, 600 s each, coverage-guided from `fuzz/seed.sh`'s seeds | no crash, timeout, or leak in any target |

The run's executions and coverage (libFuzzer's final line):

| Target | Runs | Coverage (`cov`) | Corpus |
|---|---|---|---|
| `pob_xml` | 8,019,681 | 4,010 | 3,322 inputs |
| `share_code` | 10,383,060 | 3,291 | 1,426 inputs |
| `build_document` | 4,894,333 | 6,649 | 1,762 inputs |
| `signature` | 4,298,388 | 2,267 | 1,060 inputs |
| `install_pack` | 1,375,501 | 3,877 | 1,018 inputs |
| `data_feed` | 6,017,752 | 2,383 | 1,243 inputs |

## The other hostile-input tests
| Concern | Tests |
|---|---|
| Signature, revocation, quarantine | `crates/loom-app/tests/install.rs`; `loom_data::signature` and `loom_data::pack` unit tests |
| Zip slip, zip bomb | `install.rs` (zip slip, links, declared sizes, an 8 MiB bomb in a pack under 1 MiB) and the properties |
| Feed rollback | `loom_data::pack` unit tests and the properties |
| Update signature and downgrade | `crates/loom-tauri/tests/updater.rs` (a changed package refused; equal, lower, and pre-release versions not offered) |
| Recovery | `crates/loom-persistence/tests/recovery_recent.rs`; `crates/loom-app/tests/workflow.rs` (unsaved changes recovered after the application ends) |

## Capabilities
- The webview's permissions must equal the committed allow-list, `crates/loom-tauri/capability-allow-list.json`; the test `capabilities_should_equal_the_committed_allow_list` checks it.
- Today: `main` with `core:event:allow-listen` and `core:event:allow-unlisten`.
- A second test refuses any `dialog:*`, `fs:*`, `shell:*`, or `updater:*` permission: the Rust side opens dialogs, reads files, and runs updates.
- A new permission needs a reviewed change to the list.
- `command_list_should_match_the_uis_and_every_command_should_be_registered` checks that the shell registers exactly the UI's commands. It runs them in strict offline, so a test never reaches the network.
