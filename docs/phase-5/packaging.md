# Packaging (Phase 5, P1)

The plan's step P1 and decision P-a (`dev/active/passive-tree-and-phase-5/`): installers and portable packages for Windows and Linux, one version policy, a PR-time `package` job, and a `release` workflow that installs, starts, and uninstalls every artifact.

## Artifacts

| Artifact | Platform | Made by | Installs to | Updates (P3) |
|---|---|---|---|---|
| NSIS setup (`*_x64-setup.exe`) | Windows | `tauri build --bundles nsis --config crates/loom-tauri/tauri.nsis.conf.json` | per user, `%LOCALAPPDATA%\Exiled Studio` | installs the new version itself |
| MSI (`*_x64_en-US.msi`) | Windows | `tauri build --bundles msi` | per machine, `%ProgramFiles%\Exiled Studio` | installs the new version itself |
| Portable zip (`*_x64-portable.zip`) | Windows | `tools/package/portable.ps1` | anywhere it is unpacked | notifies only |
| AppImage | Linux | `tauri build --bundles appimage` | runs where it is | installs the new version itself |
| `.deb` | Linux | `tauri build --bundles deb` | `/usr/bin/exiled-studio` | notifies only |

- **Configuration:** `crates/loom-tauri/tauri.conf.json` (`bundle`). The executable is `exiled-studio` (`mainBinaryName`); the installers show the license with GGG's notice (`crates/loom-tauri/installer/LICENSE.txt`).
- **WebView2 (Windows).**
  - The NSIS setup carries Microsoft's offline WebView2 installer (`tauri.nsis.conf.json`), so installing needs no network (ADR-010, "fully useful offline"). It adds about 130 MB to the setup.
  - The MSI and the default config embed the small bootstrapper, which downloads WebView2 if the machine lacks it. Windows 10 and 11 ship WebView2, so this only matters on machines where it was removed.
  - The portable zip expects WebView2 to be present.
- **Portable zip.** It holds `Exiled Studio.exe`, a `portable` marker file, and the license. With the marker beside it, the application keeps its data in `data\` beside the executable, not in the user's application data folder (`loom_tauri::release::portable_data`). `LOOM_DATA_DIR` still wins, for development.
- **One trap:** `tauri build` patches the executable in place with its installer type (Tauri's `__TAURI_BUNDLE_TYPE`). The portable zip is therefore made from the executable `cargo build` writes, before any `tauri build`. Otherwise it would report itself as an installer and try to update itself.
- **Not made:** macOS packages (out of scope), RPM.
- **No OS code signing** (maintainer, MQ4). Windows SmartScreen warns on first run of an unsigned setup ("Windows protected your PC"; More info, Run anyway), and some browsers mark the download. The update and data signatures (P2) are separate from OS code signing; they protect updates and data, not the first download.

## Version policy
- **One version:** the workspace's (`Cargo.toml`, `[workspace.package] version`). `tauri.conf.json` names none, so the bundler takes the crate's. The UI shows `ui/package.json`'s, which must equal it. Test: `crates/loom-tauri/tests/release_config.rs`.
- **SemVer.** Tags are `v<version>`; the first release is `0.1.0`.
- **Two channels** (`docs/release/channels.md`):
  - a stable version (`1.2.0`) is tagged on `main`;
  - an experimental one carries a numeric pre-release (`1.2.0-3`, which the MSI takes as `1.2.0.3`) and is tagged on `experimental`.
  - `tools/package/channel.sh` refuses any other tag, a tag that differs from the workspace version, and a tag on the wrong branch.
- **The version moves only in a release commit:** one commit on `dev` that changes `Cargo.toml`, `Cargo.lock`, and `ui/package.json` together, merged to `experimental` or `main` and tagged there. Until then it stays `0.0.0`, which no release carries.
- **Downgrades:** the updater refuses a version lower than or equal to the running one (P3).

## Smoke run
`--smoke <file>` starts the application as usual (`loom_tauri::release`).
- **When the first screen has rendered**, the UI calls `app_ready`; the application then writes its build info to `<file>` and quits with exit code 0. A GUI executable on Windows has no console to print to, hence the file. If the screen does not come up within 90 seconds, it writes `ready: false` and quits with exit code 2.
- **What it writes:**

  ```json
  { "ready": true, "version": "0.0.0", "channel": "stable", "e2e": false, "debug": false, "artifact": "nsis", "portable": false }
  ```

  `artifact` is `nsis`, `msi`, `app-image`, `deb`, or `plain` (the portable zip, or a development build). `channel` is `stable` or `experimental`.
- **The checks.** `tools/package/smoke.ps1` and `smoke.sh` check the build info. The release workflow passes the expected version.
  - The first screen came up, and the exit code is 0.
  - The `e2e` feature is absent; this is gate 6's check that the end-to-end test picker never ships.
  - It is not a debug build.
  - The artifact is the expected one.
  - The channel is `LOOM_CHANNEL`'s (stable when unset).
  - A zip is portable; anything else is not.

  An installed build runs with a throwaway `LOOM_DATA_DIR`, never the user's application data.

## CI
- **`package` (every PR, gates `ci-ok`).** Windows builds the portable zip; Linux builds the AppImage (`APPIMAGE_EXTRACT_AND_RUN`, since runners have no FUSE). Each is smoke-run, Linux under Xvfb. The packages are uploaded as artifacts.
- **`release` (`.github/workflows/release.yml`; by hand, or on a `v*` tag).**
  - All five artifacts are built.
  - The portable zip and the AppImage are smoke-run.
  - The NSIS setup and the MSI are installed silently, smoke-run from the installed folder, uninstalled silently, and the folder must be gone afterwards (`tools/package/install-windows.ps1`: NSIS `/S`, `msiexec /qn`, `msiexec /x`).
  - The `.deb` is installed with `apt-get`, smoke-run, and removed (`tools/package/install-deb.sh`).
  - On a tag, a draft GitHub release with every artifact; the maintainer writes the notes and publishes it (MQ5).
  - The jobs run in the `release` environment (`docs/release/signing.md`, "Release environment").
- **Gate 6** needs one recorded `release` run (workflow_dispatch). A workflow can be dispatched only once it is on the default branch, so the recorded run comes after this sprint's PR reaches `main`; it is the maintainer's.

## Evidence (2026-11-15, this workstation, Windows 11)
- **The portable zip,** from a release build with `tauri/custom-protocol`, with `tools/package/portable.ps1` and `smoke.ps1`: `ready: true`, artifact `plain`, portable, `e2e: false`, `debug: false`. Its data folder (`logs`, `recovery`) was beside the executable.
- **The NSIS setup** (`tauri build --bundles nsis`, 5.8 MB with the bootstrapper), with `install-windows.ps1 nsis`:
  - installed per user without a prompt;
  - started from `%LOCALAPPDATA%\Exiled Studio`, reporting artifact `nsis`;
  - uninstalled; the folder was gone.
- **Not run here:** the MSI (a per-machine install needs administrator rights), the AppImage, and the `.deb`. These run in CI's `package` job and in the `release` workflow.
