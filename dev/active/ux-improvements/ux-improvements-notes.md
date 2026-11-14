Last Updated: 2026-11-14

# UX improvements (side project before #107 continues): the maintainer's brief

Branch `feat/ux-improvements` from `dev` (`218cdc0`). Maintainer 2026-11-14: a UI session to improve the user experience, completed before the passive tree and Phase 5 PR (#107, draft) continues. Scope chosen: a whole-app walkthrough of the real app, friction points listed with screenshots, then the maintainer picks the fixes.

## Maintainer notes before the walkthrough (2026-11-14, verbatim in substance)
1. **Game bundle selection is not user friendly.** The user has to pick a folder holding `bundle.json`. Find a way for the bundle to be selected for the user. The maintainer also wants to know what is in the game bundle (answered in the session; see "What a bundle holds").
2. **Equipment page**: bring it closer to the equipment page in the mockup (`docs/ui/exiled-studio-ui-draft-v1.zip`, live canvas https://claude.ai/artifact/ECz5DeeNukjcxPEiPiCh9B).
3. **No option to start a new build.**
4. **Start page** (where a user loads their build): create some minimalist interface mockups for it.
5. **Dashboards**: the dashboard is meant to be a place where the user has multiple unique dashboards with a variety of components showing stat information; the user can add, remove, and resize components.

## Start page mockups (note 4)
Four minimalist directions on the canvas https://claude.ai/artifact/5993AyXC3qZCKXcreUNasb (2026-11-14): A one field, B command bar, C three actions, D recent first. **Maintainer chose B, the command bar** (2026-11-14): one input that takes a pasted share code or XML, or filters the user's builds; below it the actions New build (Ctrl N), Import from clipboard (Ctrl V), Open a file (Ctrl O), then recent builds; a file can be dropped anywhere in the window; the footer shows the game data status as one quiet line ("Game data ready · PoE1, game build ...") and the About link with GGG's notice. No error is shown up front when data is missing: data is chosen for the user (note 1).

## What a bundle holds (local/p3/p3-8n/bundle-poe1-a, data schema 3, 125 MB unpacked)
- `bundle.json`: the manifest: game, data schema, the game build it came from (Steam build), every file with its size and SHA-256.
- `data/`: game tables in our own shapes: items (`base_items`, `item_classes`, `armour_types`, `weapon_types`, `shield_types`, `mods`), skills and gems (`active_skills`, `gems`, `gem_effects`, `granted_effects`, `skill_types`, `cost_types`), stats (`stats`, `stat_sets`), the passive tree (`passives`, `passive_tree`, `mastery_groups`, `mastery_effects`), characters (`characters`, `ascendancies`, `object_templates`), monsters (`monster_types`, `monster_levels`), constants and lookups.
- `poe1/`: PoE1-only tables (pantheons, grafts, quality types).
- `stat_descriptions/`: the game's text templates that turn stat IDs into the lines players read and back (import resolution, ADR-015).
- No art, audio, or raw game files (ADR-000, ADR-005).

## Bearing on later plans
- Note 1 overlaps Phase 5 P-d in #107's plan (a signed bundle shipped with the installer, installed into the app data folder, chosen automatically). This side project can do the development-side part now (find and remember installed bundles without a folder picker) without the signing.
- Note 5 is the "dashboard tabs and Customize" item the UI fidelity PR left for later (`docs/ui/fidelity/README.md`).

PR: #108 (open, branch `feat/ux-improvements`, base `dev`).
