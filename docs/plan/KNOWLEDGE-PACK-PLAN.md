# Exiled Studio Knowledge Pack: Plan and Handoff

- Status: **Agreed** (maintainer and Claude, planning session 2026-11-15)
- Owner: DBAggie (maintainer)
- Target repo: `DBAggie/exiled-studio-knowledge` (new). This file becomes its first commit.
- Related repo: `DBAggie/exiled-studio` (codename `loom`). Its `AGENTS.md`, `CLAUDE.md` and ADRs govern any work done there.
- This copy (added 2026-11-16 at the maintainer's request) matches the knowledge repo's first commit `847a091`. Steps 1, 6 and 7 of D9 are done in this repo: ADR-018 and `tools/knowledge-mcp` (PR #123), the dev skill `.claude/skills/exiled-studio-mcp` (PR #124). The other steps are the knowledge repo's.

## Purpose
Give Claude sessions the knowledge to be a Path of Exile expert. The focus is game mechanics, community knowledge and build expertise; story and lore are out of scope. It serves two kinds of session:
- **Dev sessions** working on Exiled Studio.
- **Build-advice sessions** helping a player plan, fix or theorycraft a character.

## Locked decisions

### D1. Audience: shared core with two layers
One shared mechanics core, with a thin **dev layer** (Exiled Studio internals) and a thin **advice layer** (judgment, archetypes, economy) on top.

### D2. Game scope: PoE1 first
- v1 targets **PoE1 3.30 (Covens of Umbra, launched 2026-11-13)**.
- PoE2 gets its own plugin, built only after PoE2 1.0 launches (2026-12-11). Until then a PoE2 stub tells Claude it has no PoE2 knowledge and to check sources.
- **The two games are kept strictly apart.** Every file and record is tagged with `game` and `patch`. A session must establish which game it's working on before answering mechanics questions. Names shared between the games (skills, keywords, currency) are the main risk of confidently wrong answers.
- This matches the main repo's ACTION-PLAN: PoE1 first, PoE2 data already in the pipeline, PoE2 calcs after the Phase 4 gate.

### D3. Knowledge layers (all in v1)
| # | Layer | Form | Volatility |
|---|---|---|---|
| 1 | Game data facts (gems, mods, uniques, bases, passives) | **Tool lookups only**, never recalled from memory | Every patch |
| 2 | Mechanics rules (increased vs more, conversion, hit and ailment pipelines, defence layering and order, leech, recoup, ES recharge, charges, flasks) | Curated reference docs, each note tagged with the patch where it changed | Low |
| 3 | Community: jargon and abbreviations, archetypes, league starters, budget tiers, crafting methods, atlas and farming strategy | Curated docs | Medium |
| 4 | Build judgment: diagnosis checklists, defensive thresholds, gearing progression, reading breakdowns, trade-offs | Playbooks | Low to medium |
| 5 | Exiled Studio dev layer: stat IDs (ADR-015), divergences from PoB, using the MCP | Skill in the main repo | Moves with the code |
| S | **Snapshot layer**: economy and prices, craft costs, league-specific strategy, meta and popularity | Dated snapshots | High, but only refreshed on a schedule (D7), never live |

Rule: whenever Claude relies on snapshot content, it states the **as-of date and the league**.

### D4. Sources and licensing
| Layer | Primary source | Cross-check |
|---|---|---|
| Game data | Exiled Studio extractor bundles | PoB pinned data (MIT) |
| Mechanics | Exiled Studio calc engine + in-game tests | poewiki, poedb, patch notes |
| Jargon, archetypes | Curated by us | Wiki, guides, Reddit |
| Build judgment | Curated playbooks | Oracle corpus builds |
| Economy & meta | poe.ninja | Official trade site (manual) |

Rules:
1. **All prose is written by us.** It is verified against the wiki, the game and the engine, with sources cited by link. **Never copy text from poewiki or poedb.** Their content is CC BY-NC-SA 3.0, which would conflict with MIT.
2. **Economy data comes only from poe.ninja's supported economy-overview endpoints.** They are fetched by the refresh job, cached, sent with a proper User-Agent, and kept low-volume. Claude sessions never call poe.ninja directly. poe.ninja makes no stability guarantee and blocks misbehaving clients.
3. **Meta and build popularity are summarized by hand** from the poe.ninja builds page during refreshes. No unsupported endpoints.
4. Guides, Reddit and forum posts: we take ideas from them and credit the source, but never copy text.
5. GGG's third-party notice applies: nothing may suggest the pack is made or authorised by GGG.
6. License for the knowledge repo: **MIT**.

### D5. Packaging
1. **New repo `exiled-studio-knowledge`**, a Claude Code plugin marketplace, containing plugin **`poe1-expert`** with these skills:
   - `poe1-mechanics`: layer 2
   - `poe1-build-advisor`: layer 4, plus the tool rules from D6
   - `poe1-community`: layer 3
   - `poe1-economy`: snapshot layer
   Each `SKILL.md` is short and works as an index; detail lives in `references/` files loaded only when needed.
2. **The dev layer stays in the main repo** as `.claude/skills/exiled-studio-mcp` (and similar), so it versions with the code and ADRs. It sits beside the existing `rust-best-practices` skill, reviewer agents and dev-docs commands.
3. The plugin's `.mcp.json` launches the knowledge MCP (D6).
4. A future `poe2-expert` plugin sits next to `poe1-expert` in the same marketplace.

Proposed layout:
```
exiled-studio-knowledge/
  .claude-plugin/marketplace.json
  plugins/poe1-expert/
    .claude-plugin/plugin.json
    .mcp.json
    skills/
      poe1-mechanics/{SKILL.md, references/*.md}
      poe1-build-advisor/{SKILL.md, references/*.md}
      poe1-community/{SKILL.md, references/*.md}
      poe1-economy/{SKILL.md, snapshots/<league>/<date>.json + .md}
  plugins/poe2-expert/   (stub until after 2026-12-11)
  schema/knowledge-header.schema.json
  evals/
  tools/refresh/          (economy + patch-note triage scripts)
  .github/workflows/      (validator + eval subset)
```

### D6. Live tools
**Existing app MCP** (main repo, `crates/loom-integrations/src/mcp`, on `dev`): Streamable HTTP on 127.0.0.1 only, bearer token, off until the user starts it in the app, read-only by default. Tools: `get_build`, `list_fields`, `get_breakdown`, `list_items`, `get_item`, `list_warnings`, `preview_item_edit`, `request_item_edit`, `get_request`. It runs **inside the desktop app**.

**6a. A new knowledge MCP** in the main repo at `tools/knowledge-mcp`:
- Rust, built on `loom-data`, **stdio**, read-only, no network.
- Serves the signed bundles and the economy snapshots. Works whether or not the app is running.
- Draft tools: `search_mods`, `get_gem`, `get_unique`, `get_base`, `get_passive`, `resolve_stat` (text to stat ID, reusing Spike B), `get_price`, `get_snapshot_info`.
- Every response carries `game`, `patch`, bundle hash and, for prices, the snapshot's `as_of`.
- Refuses to answer when the bundle's patch and the snapshot's league don't match.
- **Needs ADR-018 before any code** (main repo rule: don't contradict an ADR; propose and wait for the maintainer).

**6b. Rule for checking numbers**, hard-coded in `poe1-build-advisor`:
- When the app MCP is reachable, every numeric claim (DPS, EHP, the effect of a change) must be checked with `get_breakdown` or `preview_item_edit` before Claude states it.
- When it isn't reachable, numbers are labelled **estimates**.
- A field with state `unsupported` or `indeterminate` is never read as zero.

**6c. Additions to the app MCP** (on the main roadmap, not blocking the pack): read-only previews for gem, passive and config changes; `compare_builds`; `list_supported_mechanics`.

### D7. Keeping it current
| Layer | Trigger | Cadence |
|---|---|---|
| Game data | Game patch | Rerun the extractor, rebuild bundles |
| Mechanics | Patch notes | Triage job opens review issues |
| Economy & meta | League clock | League-start week: daily or every 2 days. Then weekly to biweekly. Stop at league end. |
| Jargon, archetypes, playbooks | New league | One review at launch |

1. Every knowledge file has a **front-matter header**:
   ```yaml
   game: poe1
   patch: 3.30.0
   league: Covens of Umbra
   as_of: 2026-11-15
   sources: [<urls>]
   verification: unverified | engine-checked | in-game-checked | maintainer-approved
   ```
   Skills tell Claude to state `as_of` when relying on a snapshot, and to warn when it is older than the cadence above.
2. Refresh jobs run **on the maintainer's workstation**, where the game install is, as scheduled tasks that require that computer. Each run opens a **PR** to `exiled-studio-knowledge` and never commits directly.
3. **Every refresh, economy included, goes through maintainer PR review** (default; can be relaxed to auto-merging validated economy PRs later).
4. Patch-note triage matches changes to mechanics docs by keyword and stat ID and opens issues. It never rewrites mechanics docs on its own.
5. Versioning: plugin version is `<patch>-kb.<n>`, e.g. `3.30.0-kb.4`.
6. At league end, snapshots are archived and marked historical, never deleted.

### D8. Testing
Eval suites in `evals/`, run with and without the plugin to measure the gain:
| Suite | Example | Graded by |
|---|---|---|
| Mechanics Q&A | Increased vs more, conversion order, armour vs big hits, ailment threshold | Answer key + sources |
| Trap questions | PoE2-only or removed mechanics asked in a 3.30 context | Must catch the mismatch |
| Build diagnosis | Oracle corpus builds with planted flaws | Expected findings list |
| Tool discipline | "DPS if I swap this ring?" | Must call `preview_item_edit` |
| Snapshot honesty | Price or meta questions | Must cite `as_of` and league |

v1 gates:
- 0 cross-game contamination
- 100% date disclosure on snapshot answers
- ≥95% tool discipline when the app is reachable
- ≥90% mechanics accuracy
- Clear gain over plain Claude on build diagnosis

CI: on every PR, header schema validation, link check, version consistency and a fast eval subset. The full suite runs on each release and each league launch.

**Answer keys are written or approved by the maintainer**, never by the sessions that write the knowledge. Judgment answers are graded against a rubric by a separate agent.

### D9. Build order (one PR per session)
1. **ADR-018: knowledge MCP.** PR to `exiled-studio`, following its PR, stacking and handoff rules. It blocks step 6.
2. **Knowledge repo skeleton:** marketplace, plugin manifest, skill stubs, header schema, CI validator, PoE2 stub.
3. **Eval answer keys** (maintainer), running alongside every later step so tests come before content.
4. **Mechanics references**, one PR per topic: damage, conversion, crit, ailments, defences, recovery, charges, flasks, minions, auras and curses, totems/traps/mines, triggers.
5. **Community and advisor playbooks:** jargon, archetypes, crafting, atlas strategy, diagnosis checklists.
6. **`tools/knowledge-mcp`** in `exiled-studio`, after ADR-018 is accepted.
7. **Dev skill `exiled-studio-mcp`** in `exiled-studio`.
8. **Economy refresh job and scheduled tasks** on the workstation. 3.30 is live, so take a first snapshot right away.
9. **Full eval run, then a v1 gate review** with the maintainer.

## Install (workstation)
```
/plugin marketplace add DBAggie/exiled-studio-knowledge
/plugin install poe1-expert@exiled-studio-knowledge
```
- The knowledge MCP starts with the plugin over stdio.
- The app MCP: start it in Exiled Studio, then add its URL and bearer token to the client config once. The token persists between starts per ADR-017 Amendment 1.

## Constraints carried over from `exiled-studio`
- Never modify the legacy PoB repo; read only.
- Never copy GGG art, audio or assets (ADR-000). Derived data only.
- Stat-ID first (ADR-015). No telemetry, and network access only in integration adapters (ADR-011). The knowledge MCP has no network.
- Core crates stay wasm32-compatible (ADR-016). `tools/knowledge-mcp` is a host tool, not core.
- `main` is protected. Work on branches, open PRs, and commit or push only when the maintainer asks.

## Next action
Start a new session in `exiled-studio` and draft **ADR-018 (knowledge MCP)** from D6, following that repo's `AGENTS.md` and `CLAUDE.md`. In parallel, create `DBAggie/exiled-studio-knowledge` with this file as the first commit, then do step 2.

## References
- poe.ninja API docs: https://poe.ninja/docs/api
- GGG developer docs and policies: https://www.pathofexile.com/developer/docs
- poewiki license notice: https://www.poewiki.net/wiki/Path_of_Exile_Wiki:General_disclaimer
- poedb license notice: https://poedb.tw/General_disclaimer
- Exiled Studio ADR-017: `docs/adr/017-mcp-reference-addon.md`
