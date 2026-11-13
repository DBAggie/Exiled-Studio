# Calc scope survey (P3-3)

Phase 3 plan, step 3 (`dev/completed/phase-3-calc-slice/`). `loom-parity-cli calc-survey` decides from game data and the build, never from the engine, which builds are in the calc slice's scope (plan decision 3), and lists what a calculation of them would need. This page records the rules, the run, the result, and the decisions it leaves for the maintainer.

**Result in one line:** P3-3 found 1 of 50 builds in scope (`winter-orb-elementalist`) and no attack. The maintainer then chose the skills and how to reach the minimum (2026-11-11, "P3-3b" below): **spell Ice Nova (`IceNova`, 3 in-scope builds), attack Smite (`Smite`, 4)**, all derived setups of two credited Maxroll league-start guides. The scope file is `confirmed`.

## Rules (`tools/parity-cli/src/calc_survey/`)
**Main skill, as PoB v2.67.2 finds it** (`main_skill.rs`; `Modules/CalcSetup.lua`, `Classes/SkillsTab.lua`): the active skill set's groups; group `mainSocketGroup`, clamped; its active skills are the non-support granted effects of its enabled gems, in order (the gem effect's own, then its additional ones); `mainActiveSkill`, clamped. A gem is found by `gemId` and `variantId` (or the gem effect whose granted effect is `skillId`); without `gemId` by `skillId`. PoB skips an effect it marks as global (its own stat map, not game data) while `enableGlobal<n>` is off; the survey tries every combination of such effects counted or not and reports `main-skill-uncertain` when they pick different skills. A gem the bundle does not have is `unknown-gem` (PoB v2.67.2 skips it too: the 7 corpus cases are gem variants PoB no longer knows either).

**Supports** apply as `calcLib.canGrantedEffectSupportActiveSkill` decides: not when the skill cannot be supported, a gems-only support meets a skill an item grants, or a support's excluded types match; when its allowed types match or are empty (postfix `AND`/`OR`/`NOT`). Each applied support adds its types, in group order; PoB decides each support once (its second loop reconsiders only non-support entries). Not modelled (named in the module docs): support weapon types, supports items add, item-granted skills taking other groups' supports, cross-linked groups.

**Archetype rules** (on the skill's types after its supports; `rules.rs`):

| Rule | Game data |
|---|---|
| `not-hit` | not `Attack`, and not `Spell` with `Damage` or `Projectile` (PoB's `skillFlags.hit`): auras, buffs, minion summons |
| `not-manually-cast` | `ActiveSkills.IsManuallyCasted` false |
| `triggered` | `Triggered` or `InbuiltTrigger` |
| `totem`, `trap`, `mine`, `brand` | `SummonsTotem`, `Trapped`, `RemoteMined`, `Brand` |
| `minion` | `Minion`, `CreatesMinion`, or minion types |
| `mirage` | an applied support whose allowed types are exactly `MirageArcherCanUse` (Mirage Archer) |

Channelling is not an archetype rule (the plan names none); Winter Orb is channelled (see Q2).

**Tree and passives:** the active spec's tree version must be the scope file's (`3_29`, evidence in the file: P3-2 compared every passive of this bundle with PoB's `TreeData/3_29/tree.lua`); a node or mastery effect the bundle does not have is `tree:unknown-passive`. Both make a calculation `indeterminate` (plan decision 3, `CALC_TREE_VERSION`). An allocated socket whose jewel the survey cannot read (no item, no base row, or an untyped socket entry) is `tree:unread-jewel` (`indeterminate`), never taken as neither jewel (ADR-015); none occurs in this run.

**Excluded mechanics (Q7, proposed in `docs/phase-3/calc-scope-poe1.json`):** `timeless-jewel` (base `Metadata/Items/Jewels/JewelTimeless` in an allocated socket), `cluster-jewel` (a base tag `expansion_jewel_*` in an allocated socket), `not-carried-passive` (an allocated passive a `not-carried` rule of `docs/phase-3/data-parity-rules-poe1.json` names: 15 buff or skill passives, 147 bloodline passives). Each has a test; a mechanic the scope file does not list does not exclude.

**Inventory** (`inventory.rs`): stats of allocated nodes and mastery effects, keystones; stat IDs, typed kinds, and line classes of the items in use (active weapon set) and of jewels in allocated sockets; the main skill's applied and unapplied supports; other enabled groups' skills by class (aura, herald, curse, warcry, guard, banner, minion, buff, movement, attack, spell); config input names. Game-data IDs, PoB config names, and counts only (ADR-007).

## Run
Bundle `local/p3/p3-1c/evidence2/bundle-poe1-a` (Steam build 26530219, format 2); tool commit `c6ca400` (clean); scope file and data-parity rules as committed. Public: the community, synthetic, and discrepancy fixtures and PoB's 3.13 test builds (29); corpus: the maintainer's 21 builds, `--anonymous`. Two runs each, byte-identical (summaries `79b545fb…` public, `bff2387b…` corpus). Outputs: `docs/phase-3/calc-survey/` (`public-summary.json`, `public-report.json`, `corpus-poe1-26530219-summary.json`, `corpus-poe1-26530219-report.json`). With `--anonymous` the report keeps only the aggregate reach counts, not each build's inventory (ADR-007: a full inventory works as a fingerprint of a real build).

```
calc-survey <public fixtures> --bundle <bundle> --scope docs/phase-3/calc-scope-poe1.json --data-rules docs/phase-3/data-parity-rules-poe1.json --out <dir>
calc-survey "<corpus>" ... --anonymous --out <dir>
```

## Result
| | Public (29) | Corpus (21) |
|---|---|---|
| In scope | 1 (`winter-orb-elementalist`, `WinterOrb`) | 0 |
| Main skill passes the archetype rules | 9 | 5 |
| `tree:version` | 16 | 21 |
| `excluded:cluster-jewel` | 19 | 7 |
| `tree:unknown-passive` (mostly cluster nodes) | 19 | 8 |
| `excluded:timeless-jewel` | 11 | 3 |
| `excluded:not-carried-passive` | 9 | 6 |
| no main skill (no skill set, no active skill, unknown gem) | 9 | 11 |

A build can have several reasons. Candidates (main skill passes the archetype rules), with in-scope builds: spells `WinterOrb` 1 of 1, `ShockNova` 0 of 1 (tree 3_27); attacks `BoneshatterAltX`, `FlickerStrike`, `KineticBlastAltX`, `Earthshatter`, `PoisonousConcoctionAltX`, `Cyclone`, `LacerateAltY` 0 of 1 each (public), `ShieldCrush` 0 of 4, `Frenzy` 0 of 1 (corpus).

**Why the tree rule stays strict.** PoB computes a 3_28 build with its 3_28 tree file. Comparing PoB's own `TreeData/3_28` and `3_29` files: 171 nodes changed (52 new), and 17 of the 18 builds on 3_28 allocate 2 to 10 changed nodes (for example "Attack Mana Cost", "Life and Reduced Cost", masteries). A looser rule "in scope when every allocated node is equal in both trees" would keep 1 of the 18, and that one is out for other reasons.

## Decisions for the maintainer (P3-3; answered in P3-3b)
- **Q2 (skills).** By the plan's rule (most in-scope builds, then the smaller inventory) the spell is `WinterOrb` (1 build) and there is no attack. Winter Orb is channelled: its DPS uses a hit rate from stages (oracle: `Speed` 9.8, `HitSpeed` 5.78), heavier than a plain self-cast spell. Choose: Winter Orb, or another self-cast hit spell; and an attack.
- **Q7 (exclusions).** Confirm the three proposed exclusions. Without `cluster-jewel` and `timeless-jewel` the engine would need the cluster and timeless jewel tables (new bundle sections); with them, endgame builds are mostly out.
- **Q8 (new): how to reach 3 in-scope builds per skill.** The fixtures and corpus give 1 and 0. Options:
  - (a) **add credited public builds** (ADR-007 Amendment 1, as the plan says) of the chosen skills that are on the 3_29 tree without cluster or timeless jewels in the active spec: in practice league-start or early-map setups from guides (P3-3b, a follow-up PR once the skills are chosen);
  - (b) **derived fixtures**: copies of public builds with the active spec switched to a guide's earlier 3_29 spec (the builds hold several), recorded as changed in `SOURCES.md`;
  - (c) **lower the minimum** (the plan says at least 3; changing it needs the maintainer).

  The recommendation is (a), with (b) where a guide's build already holds a 3_29 spec without the excluded jewels.

The scope file stays `"status": "proposed"` with no chosen skill until the maintainer answers; `calc-survey` then reports the chosen skills' in-scope builds and exits 1 while either has fewer than 3.

## P3-3b: chosen skills and their builds (2026-11-11)
**Maintainer decisions (2026-11-11):** Q7, the three exclusions are confirmed; Q8, public builds, and where those are too few, derived setups of a guide (option (b)), with the minimum kept at 3; Q2, the skills follow from which candidates reach the minimum, preferring skills without PoB skill parts.

**Search** (local, nothing committed but the chosen files): candidates without skill parts in PoB's data (Arc, Ice Nova, Frostbolt, Freezing Pulse, Kinetic Bolt; Lightning Arrow, Splitting Steel, Cleave, Double Strike, Tectonic Slam), plus Smite and Static Strike. Sources: poebuilds.cc (3.29 video guides with pobb.in codes), Maxroll's 44 league-start guides (4 export a PoB code), pobarchives.com (refuses automated requests). Each code was surveyed with every passive spec, then with each setup a guide ships (spec, skill set, item set, and main group of the same name made active together).
- Arc: 1 in-scope build of 10 video-guide codes (the others: cluster or timeless jewels, another main skill, older trees); single-spec files, so no setups to derive.
- Ice Nova (Maxroll, aer0): the file opens on its Aspirational setup (cluster jewels; main group Frostblink); its Campaign (lab 1), Campaign, and Mapping setups are in scope.
- Smite of Divine Judgement Inquisitor (Maxroll, Shayd): plain `Smite` is the main skill of the Act 1, 2, 5, and 10 setups (in Act 5 and 10 selected in its group with Leap Slam), all in scope; the transfigured `SmiteAltX` is in scope in one setup (Red Maps).
- Frostbolt, Splitting Steel, Tectonic Slam: no 3.29 guide with a PoB code found; Lightning Arrow's 3.29 guide links a character, not a code.

**Chosen:** spell `IceNova` (no skill parts), attack `Smite` (two PoB skill parts, Melee Hit and Area Hit: P3-5 and P3-10 follow PoB's part selection, `skillPart`, default 1). Seven derived fixtures in `tools/oracle/fixtures/community/` (`ice-nova-elementalist-*`, `smite-inquisitor-act*`), tag `derived`, credited with the exact change in `community/SOURCES.md`. Each changes only the active-setup attributes of the decoded export (and for Smite Act 5 and 10 the group's `mainActiveSkill` and `mainActiveSkillCalcs`, so PoB's CALCS view, which the oracle's breakdowns come from, selects Smite too).

**Oracle:** `oracle-run` on the seven, 3 runs each on the pinned oracle (PoB 2.67.2, Windows): all `ok`, runs identical. The Smite files make one inflate call (the guide file holds a timeless jewel outside the active setup's allocated sockets; the `inflate` shim, as for other community fixtures).

**Survey** from clean `f77cb80`, two runs each byte-identical (summaries `098143f6…` public, `ad813a2d…` corpus): public 36 builds, 8 in scope, `IceNova` 3 and `Smite` 4 in scope, `pass` true, exit 0; corpus 21 builds, 0 in scope (exit 1 on its own: the tool checks the minimum per run, and the corpus adds none; gate 1 counts corpus and public builds together). Outputs in `docs/phase-3/calc-survey/`.

## Slice fields (`docs/phase-3/slice-fields.json`, revision 1, proposed; Q3, Q6)
The plan's default list, with two changes found in the oracle outputs:
- **`AverageDamage` replaces `AverageHit`:** PoB has no top-level `AverageHit` for attacks (it is per hand; 16 of the 29 public outputs lack it), and its hit DPS line prints `output.AverageDamage` for both kinds (`Modules/CalcOffence.lua`).
- **`HitSpeed` added:** `TotalDPS` multiplies by `HitSpeed` when the skill has one (Winter Orb), else `Speed`.

Every field is `exact` for now (Q6: a tolerance only with a written reason once successor outputs exist). The ten oracle breakdowns are written with their line formats and term groupings: `chain` (Life, Mana: `breakdown.simple`; TotalDPS), `slots` (Armour, Evasion, EnergyShield: `breakdown.slot` rows per item slot, `Global`, `Conversion`), and `min-max-total` (resistances). A test checks every oracle breakdown has its field and every rule is well formed.
