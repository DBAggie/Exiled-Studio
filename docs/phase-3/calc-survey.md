# Calc scope survey (P3-3)

Phase 3 plan, step 3 (`dev/active/phase-3-calc-slice/`). `loom-parity-cli calc-survey` decides from game data and the build, never from the engine, which builds are in the calc slice's scope (plan decision 3), and lists what a calculation of them would need. This page records the rules, the run, the result, and the decisions it leaves for the maintainer.

**Result in one line:** under the plan's rules, 1 of 50 builds is in scope (`winter-orb-elementalist`, a spell); no attack is. Gate 1 needs at least 3 in-scope builds per chosen skill, so the denominator cannot be met from the current fixtures and corpus. The plan's remedy (credited public builds of the chosen skills) needs the maintainer's choice of skills first (Q2, Q7, and new Q8 below).

## Rules (`tools/parity-cli/src/calc_survey/`)
**Main skill, as PoB v2.67.2 finds it** (`main_skill.rs`; `Modules/CalcSetup.lua`, `Classes/SkillsTab.lua`): the active skill set's groups; group `mainSocketGroup`, clamped; its active skills are the non-support granted effects of its enabled gems, in order (the gem effect's own, then its additional ones); `mainActiveSkill`, clamped. A gem is found by `gemId` and `variantId` (or the gem effect whose granted effect is `skillId`); without `gemId` by `skillId`. PoB skips an effect it marks as global (its own stat map, not game data) while `enableGlobal<n>` is off; the survey tries every combination of such effects counted or not and reports `main-skill-uncertain` when they pick different skills. A gem the bundle does not have is `unknown-gem` (PoB v2.67.2 skips it too: the 7 corpus cases are gem variants PoB no longer knows either).

**Supports** apply as `calcLib.canGrantedEffectSupportActiveSkill` decides: not when the skill cannot be supported or a support's excluded types match; when its allowed types match or are empty (postfix `AND`/`OR`/`NOT`). Each applied support adds its types, in group order. Not modelled (named in the module docs): support weapon types, supports items add, item-granted skills taking other groups' supports, cross-linked groups.

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

**Tree and passives:** the active spec's tree version must be the scope file's (`3_29`, evidence in the file: P3-2 compared every passive of this bundle with PoB's `TreeData/3_29/tree.lua`); a node or mastery effect the bundle does not have is `tree:unknown-passive`. Both make a calculation `indeterminate` (plan decision 3, `CALC_TREE_VERSION`).

**Excluded mechanics (Q7, proposed in `docs/phase-3/calc-scope-poe1.json`):** `timeless-jewel` (base `Metadata/Items/Jewels/JewelTimeless` in an allocated socket), `cluster-jewel` (a base tag `expansion_jewel_*` in an allocated socket), `not-carried-passive` (an allocated passive a `not-carried` rule of `docs/phase-3/data-parity-rules-poe1.json` names: 15 buff or skill passives, 147 bloodline passives). Each has a test; a mechanic the scope file does not list does not exclude.

**Inventory** (`inventory.rs`): stats of allocated nodes and mastery effects, keystones; stat IDs, typed kinds, and line classes of the items in use (active weapon set) and of jewels in allocated sockets; the main skill's applied and unapplied supports; other enabled groups' skills by class (aura, herald, curse, warcry, guard, banner, minion, buff, movement, attack, spell); config input names. Game-data IDs, PoB config names, and counts only (ADR-007).

## Run
Bundle `local/p3/p3-1c/evidence2/bundle-poe1-a` (Steam build 26530219, format 2); tool commit `f061f18` (clean); scope file and data-parity rules as committed. Public: the community, synthetic, and discrepancy fixtures and PoB's 3.13 test builds (29); corpus: the maintainer's 21 builds, `--anonymous`. Two runs each, byte-identical (summaries `c8441e7d…` public, `622deb07…` corpus). Outputs: `docs/phase-3/calc-survey/` (`public-summary.json`, `public-report.json`, `corpus-poe1-26530219-summary.json`, `corpus-poe1-26530219-report.json`).

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

## Decisions for the maintainer (before P3-6)
- **Q2 (skills).** By the plan's rule (most in-scope builds, then the smaller inventory) the spell is `WinterOrb` (1 build) and there is no attack. Winter Orb is channelled: its DPS uses a hit rate from stages (oracle: `Speed` 9.8, `HitSpeed` 5.78), heavier than a plain self-cast spell. Choose: Winter Orb, or another self-cast hit spell; and an attack.
- **Q7 (exclusions).** Confirm the three proposed exclusions. Without `cluster-jewel` and `timeless-jewel` the engine would need the cluster and timeless jewel tables (new bundle sections); with them, endgame builds are mostly out.
- **Q8 (new): how to reach 3 in-scope builds per skill.** The fixtures and corpus give 1 and 0. Options:
  - (a) **add credited public builds** (ADR-007 Amendment 1, as the plan says) of the chosen skills that are on the 3_29 tree without cluster or timeless jewels in the active spec: in practice league-start or early-map setups from guides (P3-3b, a follow-up PR once the skills are chosen);
  - (b) **derived fixtures**: copies of public builds with the active spec switched to a guide's earlier 3_29 spec (the builds hold several), recorded as changed in `SOURCES.md`;
  - (c) **lower the minimum** (the plan says at least 3; changing it needs the maintainer).

  The recommendation is (a), with (b) where a guide's build already holds a 3_29 spec without the excluded jewels.

The scope file stays `"status": "proposed"` with no chosen skill until the maintainer answers; `calc-survey` then reports the chosen skills' in-scope builds and exits 1 while either has fewer than 3.

## Slice fields (`docs/phase-3/slice-fields.json`, revision 1, proposed; Q3, Q6)
The plan's default list, with two changes found in the oracle outputs:
- **`AverageDamage` replaces `AverageHit`:** PoB has no top-level `AverageHit` for attacks (it is per hand; 16 of the 29 public outputs lack it), and its hit DPS line prints `output.AverageDamage` for both kinds (`Modules/CalcOffence.lua`).
- **`HitSpeed` added:** `TotalDPS` multiplies by `HitSpeed` when the skill has one (Winter Orb), else `Speed`.

Every field is `exact` for now (Q6: a tolerance only with a written reason once successor outputs exist). The ten oracle breakdowns are written with their line formats and term groupings: `chain` (Life, Mana: `breakdown.simple`; TotalDPS), `slots` (Armour, Evasion, EnergyShield: `breakdown.slot` rows per item slot, `Global`, `Conversion`), and `min-max-total` (resistances). A test checks every oracle breakdown has its field and every rule is well formed.
