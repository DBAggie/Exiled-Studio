Last Updated: 2026-11-10 (PR C0: plan)

# Spike C: context

## Key files
- `docs/plan/ACTION-PLAN.md`, "Spike C: Thin oracle harness": scope and gate (authoritative).
- `docs/adr/006-parity-and-oracle.md`: thin harness, pinned tag, no copying, divergence register fields, layered comparison.
- `docs/adr/001-migration-and-platform.md`: the game is the truth, PoB is the regression oracle.
- `docs/adr/007-corpus-and-coverage.md`: corpus of 21 maintainer builds, scrubbing.
- `docs/adr/011-integrations-and-privacy.md`, `docs/adr/016-wasm-compatible-core.md`: no network outside integrations, core crates stay pure (the oracle code lives in `tools/parity-cli`).
- `docs/spikes/spike-c-legacy-oracle.md`: older detailed specification (failure taxonomy, manifest fields, required test cases, completion report template). Superseded where it conflicts with the ACTION-PLAN.
- `tools/parity-cli/src/main.rs`: hand-rolled subcommand matching; new `oracle-*` subcommands follow the same pattern.
- Legacy (read only, cite as `v2.67.2:<path>`): `src/HeadlessWrapper.lua`, `src/Launch.lua`, `src/Modules/Main.lua`, `src/Modules/Common.lua`, `src/Modules/DataJewelFileLoader.lua`, `spec/GenerateBuilds.lua`, `spec/TestBuilds/3.13/`, `Dockerfile`, `.github/workflows/test.yml`.

## Inputs on the maintainer workstation
- Legacy repo `D:\POE Dev\PathOfBuilding` (read only; working tree at `v2.67.2-122-g3dbca4463`, past the pin; tags available for `git show v2.67.2:<path>`).
- Corpus `D:\Documents\Path of Building\Builds` (21 PoB XML files; never committed).
- `local/pob-v2.67.2/` holds a `src/` copy from Spike B (`render-check`); Spike C uses a real git checkout in `local/oracle/pob/` instead.
- No LuaJIT or Lua installed (checked 2026-11-10: `luajit`, `lua` not on PATH).

## Decisions
- Spike B closed in PR C0, following the B0 precedent (B0 closed Spike A in the same PR as the Spike B plan).
- Plan decisions: plan file, "Design constraints and decisions".

## plan-reviewer (2026-11-10) summary
Verdict: proceed with changes. All findings applied to the plan.
- Blockers: (1) a `NewFileSearch` shim is not needed and would make results depend on `.bin` file times; dropped, the stub stays. (2) The adapter cannot switch off the `.bin` cache write, and PoB's `.gitignore` hides the file from a plain status check; added an `io.open` write-guard shim and an ignored-files tree check.
- Should-fix: jewel loading is lazy (C2 measurement removed); startup-error path (`dofile` returns, `build` nil); zlib pinned and recorded; LuaJIT hash-randomization flags checked in C1, 3 repeats; field-specific tolerances in the register (`comparison`), exact by default; a synthetic timeless-jewel fixture in CI; determinism defined over the whole outcome; C2 split into C2a and C2b; fake adapter scripts instead of test hooks in `oracle.lua`; environment allow-list (`SystemRoot`, minimal `PATH` on Windows); hand-written schema check (no JSON-schema crate).
- Nits: AGENTS.md network rule exemption for acquire scripts (C1); `src/` working directory dependency noted; `GetTime` stub recorded as relied on; PoB's 3.13 test builds marked `old-tree`.

## Open questions for the maintainer
1. **Headless shims.** The tag's wrapper stubs `Inflate`, so timeless jewels (in 9 of the 21 corpus builds) compute without their data. Plan: two named adapter shims, recorded in every output: `Inflate` through LuaJIT FFI to a pinned zlib, and an `io.open` guard that refuses writes into the checkout (PoB writes a `.bin` cache there). The `NewFileSearch` stub is kept on purpose. Alternative: accept those builds as `partial`. Recommendation: shims.
2. **CI fixtures.** Default: CI runs PoB's own 5 test builds (from the pinned checkout, not copied) plus synthetic fixtures; the 21 corpus builds run only on the workstation, with scrubbed summaries committed. Alternative: commit a few scrubbed corpus builds (the repository is public).
3. **Runtime pin.** Plan: LuaJIT at `2460b3ff` and `luautf8 0.1.6-1`, the versions PoB's own test image uses, built from source on both platforms. Alternative on Windows: PoB's shipped `runtime/lua51.dll` (unknown LuaJIT commit).
4. **CI gating.** Should the `oracle` job be required in `ci-ok` from C4, or informational until it has been stable for a while?
5. **Layout.** Plan: `tools/oracle/` for the non-Rust oracle files (adapter, manifest, schema, fixtures, acquire scripts) and `docs/parity/` for the divergence register. The spec suggested a top-level `oracle/`. Either needs the AGENTS.md layout updated in C1.

## Facts found while planning (2026-11-10)
- Tag `v2.67.2` is commit `b32759ab0f31a1c8499a0d420cb0f0633d4fe478` (`git rev-parse v2.67.2^{commit}` in the legacy repo); tagged 2026-08-08. `src/GameVersions.lua` lists tree versions up to `3_29`.
- `v2.67.2:src/HeadlessWrapper.lua` (227 lines) defines the SimpleGraphic stubs inline (no `_SimpleGraphic.def.lua` at the tag), stubs `require("lcurl.safe")`, `dofile("Launch.lua")`, sets `continuousIntegrationMode = os.getenv("CI")`, runs `OnInit` and one `OnFrame`, prints `promptMsg` and calls `io.read` on startup errors, and defines `loadBuildFromXML(xmlText, name)` -> `SetMode("BUILD", ...)` + `OnFrame`. Stubs: `Inflate`/`Deflate` return `""`, `GetTime` returns 0, `GetScriptPath`/`GetRuntimePath`/`GetUserPath`/`GetWorkDir` return `""`.
- `Modules/Main.lua` builds `userPath` from `GetUserPath()` and loads `Settings.xml` from it, so headless looks for `/Path of Building/Settings.xml`.
- `Modules/DataJewelFileLoader.lua` (corrected by the plan review): with `GetScriptPath() == ""` it probes `./Data/...` then `./src`; `NewFileSearch` is used only to compare file times of a `.bin` cache against the `.zip` parts (stub returns nil, so the parts are always read); the parts are read with `io.open`, passed to `Inflate` (zlib header `78 da`), and the result is written to `<script>/Data/TimelessJewelData/<name>.bin` (`"wb+"`) because `DataLegionLookUpTableHelper.lua` calls the loader without `cacheUncompressed` (default true). PoB's `.gitignore` ignores `src/Data/TimelessJewelData/*.bin`. `src/Data/TimelessJewelData/` at the tag holds `.zip.partN` files.
- Jewel data loads when a build loads, not at init (`Data.lua` creates an empty `data.timelessJewelLUTs`; `PassiveSpec.lua` calls `data.readLUT`), so an `Inflate` override after `dofile` of the wrapper takes effect.
- On a startup error the wrapper prints `promptMsg`, calls `io.read`, and `return`s from `dofile`, leaving `build` nil. `Launch.lua` skips the update check because `GetTime` returns 0.
- Native modules needed headless: `lua-utf8` (`Modules/Common.lua`, unconditional `require`; used in number formatting). `xml`, `base64`, `sha1`, `dkjson` are Lua files in `runtime/lua/`. `lcurl.safe` is stubbed by the wrapper.
- PoB CI (`.github/workflows/test.yml`) runs `busted --lua=luajit` in a container built from `Dockerfile`: LuaJIT `git checkout 2460b3ff93a1c955de3d62cfc825de7d68dc272e`, `luarocks install luautf8 0.1.6-1`. Its ModCache check runs `luajit HeadlessWrapper.lua` from `src/` with `LUA_PATH=../runtime/lua/?.lua;../runtime/lua/?/init.lua`.
- `spec/GenerateBuilds.lua` loads each test build with `loadBuildFromXML` and serializes `build.calcsTab.mainOutput`: the same read path the adapter uses.
- `spec/TestBuilds/3.13/` at the tag: 5 XML builds with expected `.lua` outputs (Dual Savior, Dual Wield Cospris CoC, Generals Perforate Zerker, Mirage Archer Toxic Rain, OccVortex).
- Corpus: 9 of the 21 builds contain a timeless jewel (name match on Glorious Vanity, Lethal Pride, Brutal Restraint, Militant Faith, Elegant Hubris, Heroic Tragedy).
- Windows `runtime/` at the tag ships `lua51.dll` (616,448 bytes), `lua-utf8.dll`, `zlib1.dll`, and no `luajit.exe`.
