Last Updated: 2026-11-17 (P6-1: slice revision 3, PoB skill table, skill damage over time)

# Phase 6: PoE1 mechanics expansion (tasks)

Order and evidence per PR are in the plan, "Planned PR breakdown". Tick a task in the PR that does it, with the PR number. Every engine PR: unit tests, mechanic fixtures where PoB has the mechanic, before-and-after `calc-run` and `calc-parity` on public and corpus (`sh local/p6/evidence.sh <step>`; baseline `local/p6/base`), register entries, the bench within limits or re-baselined with a reason, handoff and CLAUDE.md updated.

## P6-0: plan
- [x] Plan, context, tasks in `dev/active/phase-6-mechanics/`
- [x] plan-reviewer run; findings applied (context file, "Plan review")
- [x] ACTION-PLAN, handoff, CLAUDE.md updated
- [x] Open PR: #139 (base `dev`)

## P6-0b: evidence bundle with tree packs (local)
- [x] `tools/tree/acquire` 3_28 to 3_25; `tree-pack` chained; `bundle-check` (manifest `4814e3ac…`, as #134 recorded)
- [x] Baseline re-measured on it (`local/p6/base-packs`); plan's numbers updated (#139)

## P6-1: slice revision 3, PoB skill table, skill damage over time
- [x] Slice revision 3: `applies: present`, `TotalDot`, `Field::DotDps` (emitted only when the main skill has a DoT)
- [x] Readers that count fields: parity tests (revision, `present`), UI (`OFFENCE`, `FIRST`, label); golden documents unchanged (no DoT in the synthetic build)
- [x] `pob_data.lua` format 10: `base_flags`, `parts`, `functions`; `loom-parity-cli pob-skills`; generated `poe1/pob_skills.rs` (1,231 skills, regenerates byte-identical). The `data-parity` check of the table itself is left for P6-6, where parts are first read; the base mods read here are `dotIsArea` and the ground flags.
- [x] Skill DoT (`poe1::dot`), DoT multipliers, enemy DoT taken, `TotalDot`; DoT stat rows; Righteous Fire and Vaal Righteous Fire; Righteous Fire as an other-gem buff
- [x] Found on the way: PoB's skills layout before skill sets (`SkillsLayout::Flat`, build v1 `skills.layout` additive); gems the data does not have are skipped as PoB does (a reason), renamed `skillId`s found by name
- [x] Mechanic fixtures (5, all equal to PoB; Essence Drain kept local until P6-10b, D9); code-architecture-reviewer run, findings applied or recorded (D6 to D9); evidence (`docs/phase-6/dot.md`)
- [x] Open PR: #140 (base `plan/phase-6-mechanics`)

## P6-2a: dual wielding
- [x] The off hand's weapon as its own quantities (`OffHandWeapon`, `OffHandWeaponMapping`); which hands attack (`CalcActiveSkill.lua` 318 to 335)
- [x] One pass per hand and PoB's `combineStat` (average, harmonic mean, `DPS` with double hits); inherent dual-wield speed and block; the item card's off-hand weapon
- [x] Fixture `smite-dual-wield-templar.xml` (21 fields equal to PoB; Q11 rate DIV-0072, DIV-0073; breakdown DIV-0074 `harness`); `docs/phase-6/dual-wield.md`
- [x] Open PR: #141 (base `feat/p6-1-dot`)
## P6-2b: ignite and the shared ailment base
- [x] `poe1::ailment`: the shared base (chance, config, rate and duration, stacks and roll, source recomputed in the ailment's config, crit weighting) and ignite; `ignite-dps` (slice `present`)
- [x] Keyword flags `KEYWORD_ATTACK` and `KEYWORD_SPELL` ("with Attack/Spell Skills" rows); Elemental Overload's ailment more and `AilmentsAreNeverFromCrit`; Elemental Weakness's duration on the enemy; the critical ailment DoT multiplier
- [x] Fixtures (3; Q11 DIV-0075 to DIV-0078); community Smite builds' ignite (Q11 DIV-0079 to DIV-0082); `data-parity` export format 10; `docs/phase-6/ailments.md`
- [x] Open PR: #142 (base `feat/p6-2a-dual-wield`)
## P6-2c: bleed and poison
- [x] The ailment base generalized (`ailment::damaging`, one spec per ailment); bleed and poison; `bleed-dps`, `poison-dps` (slice `present`)
- [x] `CannotInflict`, `MELEE_HIT`; bleed and poison rows; Melee Physical Damage's and staff ailment damage leave `NO_EFFECT`
- [x] P6-2b review applied (dual-wield hands, `HitChance` dependency, staff row, register prose, the Elemental Overload fixture); fixtures (3; Q11 DIV-0083 to DIV-0087); `docs/phase-6/ailments.md`
- [x] Open PR: #143 (base `feat/p6-2b-ignite`)
## P6-2d: impale
- [x] `ailment::impale` (chance, default maximum stacks, stored hit, effect, the enemy's armour and damage taken, `(1 + x) - 1`); `poe1.impale-dps` (slice `present`); `hit::armour_reduction` split
- [x] Impale rows; fixture `smite-impale-templar.xml` (Q11 DIV-0088 to DIV-0090); `docs/phase-6/ailments.md`, "P6-2d"
- [x] Open PR: #144 (base `feat/p6-2c-bleed-poison`)
## P6-3: cluster jewels (what remains)
## P6-4a: config inputs (P6-4 split: flasks need a data run, so they are P6-4b; P6-3 waits on the cluster extraction)
- [x] The P6-2c review applied: poison's physical part with chaos modifiers (`chaos_damage_+%` mapped), skill-data ailment keys as reasons, the staff row on attacks only; fixture `smite-poison-chaos-templar.xml` (Q11 DIV-0091 to DIV-0093); `docs/phase-6/ailments.md`
- [x] Tailwind and Adrenaline (`poe1::buffs`); 61 inputs into `config::NAMED_ONLY`, 2 into `ENEMY_DAMAGE`, 4 into the new `config::INERT`
- [x] Fixtures `ice-nova-tailwind-adrenaline-witch.xml`, `smite-tailwind-adrenaline-templar.xml` (Q11 DIV-0094 to DIV-0096); `docs/phase-6/config-inputs.md`
- [x] Open PR: #145 (base `feat/p6-2d-impale`)
## P6-4b: flasks
- [x] The P6-4a review applied: a guard that no `NAMED_ONLY` name is settled; an `input_use` test; `DEFAULT_ON`; the corrected counts and readers
- [x] Data: the `Flasks` extractor root, `poe1/flasks.json` (converter, loader, validator, synthetic row), `data-parity` table `flasks` (export format 11; 192 values, 0 differences); evidence bundle `local/p6/packs/b-flasks`
- [x] `loom_calc::poe1::flasks` (effect scale, merge, Silver's Onslaught), `FlaskMapping` (charges, duration, eight aliases), rows for `flask_effect_+%`, `magic_utility_flask_effect_+%`, Basalt, Stibnite, Silver, Phasing; fixture `smite-flasks-templar.xml` (Q11 DIV-0097 to DIV-0099); `docs/phase-6/flasks.md`
- [x] Open PR: #146 (base `feat/p6-4-config-inputs`)
- [ ] Maintainer: the next data run ships `poe1/flasks.json` (the extraction with the `Flasks` root)
## P6-6a: the skill sweep and skill stats, batch 1 (order change: before P6-5, whose channelled skills need P6-6's parts)
- [x] The P6-4b review applied: negative buffs merge at the smallest scale; data without the flask table keeps `active-flask` a reason; flags unscaled; flask tests; doc and test placement
- [x] `tools/oracle/sweep/make_sweep.py` (376 builds, local) and `skill_stats.py`; `poe1::skill_no_output` (148 stats), 7 + 2 `NoEffect` rows, fire and cold skill conversion, `global_always_hit`; `docs/phase-6/skill-data.md`
- [x] Open PR: #147 (base `feat/p6-4b-flasks`)
## P6-6b: PoB skill data as a rule (flags, parts, the vetting reason), with the sweep as evidence
- [x] The P6-6a review: `global_always_hit` gem-only; the classifier's whole-tree scan; mapping tests; flask merge slot
- [x] `hit::pob_part` / `pob_vetted` replace `SKILL_PARTS` and the vetted lists; spells take the projectile flag; DoT: `pob-skill-data` without PoB data or with unread base mods (four Complete-but-wrong DoTs found by the sweep)
- [x] Open PR: #148 (base `feat/p6-6a-skill-sweep`)
## P6-6c: the P6-6b review (melee and projectile by weapon; tagged stat map entries unvetted)
- [x] PoB's weapon rule for melee and projectile; `PobSkill::own_stats` (generator, regenerated table) in the hit and DoT vetting; explicit DoT key lists; tests; `docs/phase-6/skill-data.md`, "P6-6c"
- [x] Open PR: #149 (base `feat/p6-6b-skill-data`)
## P6-5: channelling and Vaal skills
- [x] Channelled skills computed (no tick cap; stages and release left to the vetting rule); Vaal skills checked by the sweep; the P6-6c review (`weapon_flags`, melee over projectile for spells and DoT); `docs/phase-6/channelling.md`
- [x] Open PR: #150 (base `feat/p6-6c-skill-review`)
## P6-6d: totems (P6-6 split: traps, mines, brands are P6-6e)
- [x] Totems' hit and DoT computed per totem, the player's action speed for self-cast skills only; the P6-5 review (release skills unvetted, Vaal wording, test placement); `docs/phase-6/totems.md`
- [x] Open PR: #151 (base `feat/p6-5-channel-vaal`)
## P6-6e: traps, mines, brands
- [x] Trap and mine spells computed with PoB's throwing and laying time (`trap-mine-data` reason); brands, trap and mine attacks left not computed; the P6-6d review (`totem-data` reason, tests, module docs); `docs/phase-6/totems.md`
- [x] Open PR: #152 (base `feat/p6-6d-totems-traps`)
## P6-7a: minion data
- [x] `poe1/minions.json` (converter, loader, validator, synthetic row); data-parity `minions` (export format 12; 5,959 values, 1 rule); evidence bundle `local/p6/packs/b-minions`; the P6-6e review (docs); `docs/phase-6/minions.md`
- [x] Open PR: #153 (base `feat/p6-6e-traps-mines-brands`)
- [ ] Maintainer: the next data run ships `poe1/minions.json`
## P6-7a2: the P6-7a review
- [x] Converter strict on IDs and types; validator (duplicate types, empty fields, negatives, skill references); PoB's rod key; `MinionType::life_scaling`; `pob_data.lua` stops on duplicates; start cost measured (+40 ms); `docs/phase-6/minions.md`
- [x] Open PR: #154 (base `feat/p6-7a-minion-data`)
## P6-7b: minion actor (plan: `p6-7b-minion-actor-plan.md`, reviewed by plan-reviewer)
- [x] Plan written and reviewed; split into b1 (data, blocked on the maintainer's depot download of the `.ot` bundles), b2 (actor in the core and parity tool, no value change), b3a, b3b, b4
- [x] Plan PR: #155 (base `feat/p6-7a2-minion-review`)
- [x] P6-7b2: `Actor`, `ActorState`, `Calc` per actor; `Field::Minion`; slice revision 4 (`pob_path`, `MINION_NAMES`, `applies: minion`); every document byte-identical
- [x] P6-7b2 PR: #156 (base `feat/p6-7b-plan`)
- [x] P6-7b2b: the P6-7b2 review (`with_actor` refuses a missing minion, skill sources per actor, engine tests)
- [x] P6-7b2b PR: #157 (base `feat/p6-7b2-actor`), auras and buffs on it, minion defences
- [x] P6-7b1 (after the maintainer's depot download, 2026-11-17): the extractor reads every monster `.ot` as PoB does (`monstertemplate.rs`, manifest format 6, 7,176 files); `poe1/minions.json` `object_stats` (3,265 object types, chains resolved as PoB, `values` and `unread` kept, validator); export format 13 (`minion_mods`, `minion_templates`); data-parity table `minion_mods` (332 lists, 17 differences, 2 rules); generated `pob_minions.rs` (331 minions) and the skills' minion lists; evidence bundle `local/p6/packs/b-ot`; the P6-11e review (`MainSkill::can_use`, test fixes); `docs/phase-6/minions.md`, "P6-7b1"
- [x] P6-7b1 PR #163 (base `feat/p6-11e-dot-weapon`)
- [x] P6-7b1b: the P6-7b1 review (monster base excluded, `can_use` test and docs, validator dangling and empty entries, one difference without `object_stats`, generator sort and split, export guard, docs); `docs/phase-6/minions.md`, "The P6-7b1 review"
- [x] P6-7b1b PR #164 (base `feat/p6-7b1-minion-data`)
- [x] P6-7b3a: the minion step (`minion_actor`: choice with `skillMinion`, level from the game's stat, base values, `MinionMapping`, template extras, the life override, reasons `minion-modifier` and `minion-buffs`, `MinionPending`); shared phase helpers; sweep of 52 builds (0 unexpected, 32 `Complete` minion fields equal to PoB); 4 mechanic fixtures; the #164 review's minors; `docs/phase-6/minions.md`, "P6-7b3a"
- [x] P6-7b3a PR #165 (base `feat/p6-7b1b-review`)
- [x] P6-7b3a2: the #165 review (12 item skills, `minion-buffs` in the main group, `base_numbers` and the `Ingest` test, six stat slots, early return, mapping comments); `docs/phase-6/minions.md`, "The P6-7b3a review"
- [x] P6-7b3a2 PR #166 (base `feat/p6-7b3a-minion-base`)
- [x] P6-7b3b: the character's minion modifiers on the minion (31 rows in `poe1::minions::STATS`, `GolemMinion`, `minion_modifiers`, `gem_modifiers`, `ModStore::elsewhere`), `MinionPending` removed, PoB's more rounding in `phases::chain`; fixture `minion-summon-stone-golem-golem-lines-witch.xml`; sweep 40 `Complete` minion fields equal to PoB; `docs/phase-6/minions.md`, "P6-7b3b" (PR #167, branch `feat/p6-remaining`)
- [x] Vaal Summon Skeletons' minion level (the game's stat 68 against PoB's level requirement 70 at gem level 20): registered with P6-9c (DIV-0111 to DIV-0113, `game-truth`; confirmation in game is a maintainer item), fixture `minion-vaal-summon-skeletons-witch.xml`
- [ ] A fixture with `set_maximum_life_is_one`: waits on a data run with gem level requirements (every such minion's level comes from them or the character)
- [x] P6-7b4: the character's auras and buffs on the minion (`gems::minion_buffs`, `Totals::from_effects`; `deferred_reasons` removed); fixture `minion-summon-stone-golem-discipline-witch.xml`; `docs/phase-6/minions.md`, "P6-7b4"
- [ ] Maintainer: the next data run ships `poe1/minions.json` with `object_stats`
## P6-7c: minion hits and DoT, spectres
- [x] P6-7c1: the minion's hit (`poe1::minion_hit`, `Field::for_minion` under the minion actor, the monster weapon with `CalcData::monster_ally_damage` and `WeaponValue::AttackRate`, `damageFixup`, `CannotBeEvaded`, the minion crit multiplier base, the shared enemy's effects and conditions, `on_minion` damage and speeds, the minion's offence monster mods); slice revision 5 (six minion hit fields); sweep: 10 minion hits, 58 equal to PoB, 2 registered (DIV-0123, DIV-0124); fixture `minion-raise-zombie-damage-speed-lines-witch.xml`, nine retargeted; `docs/phase-6/minions.md`, "P6-7c, part 1"
- [x] P6-7c2: the summoning skill's supports on the minion's skills (`support_modifiers`, `SupportOnMinion`, `NO_MINION_SUPPORTS`, `NOT_MINION_STATS`), Brutality's and Minion Damage's own rows, `DealNo` in the gem mapping; fixture `minion-raise-zombie-supports-witch.xml` (31 fields equal to PoB); public 184 builds, 0 unexpected, 3,491 `Complete`; `docs/phase-6/minions.md`, "P6-7c, part 2"
- [x] P6-7c3a: the minion's spells (`skill::instance_for_actor`, PoB's effectiveness at the minion's level; `minion_hit::VETTED_SPELLS`: the Flame Golem's Immolate; damage effectiveness 1; `active_skill_cast_speed_+%_final`); `minion-summon-flame-golem-witch.xml` retargeted, every minion offence value equal to PoB; `docs/phase-6/minions.md`, "P6-7c, part 3a"
- [x] P6-7c3b: a spell's cooldown caps its rate (`hit::cooldown_limit`, the character's spells too); the Lightning Golem's Storm Orb; fixture `minion-summon-lightning-golem-witch.xml`; `docs/phase-6/minions.md`, "P6-7c, part 3b"
- [x] P6-7c3c: a minion attack's own stat map (`minion_hit::OWN_STATS`, `minion_actor::OwnSkillMapping`): the Carrion Golem's combo attack, every value equal to PoB; `docs/phase-6/minions.md`, "P6-7c, part 3c"
- [x] P6-7c3d: the minion's ignite, bleeding, poison, impale, and combined DPS (slice revision 11, `applies: minion-present`; `combined::combine` under the minion actor; the monster template's critical ailment DoT multiplier); DIV-0174; 15 minion fixtures retargeted; `docs/phase-6/minions.md`, "P6-7c, part 3d"
- [ ] P6-7c3 rest: the relic's Nova (P6-8a, data), the minion's damage over time, spectres
## P6-8a: triggers
- [x] P6-8a part 1: the shared handler (`poe1::triggers`: sources computed as their own skills, cooldown cap, trigger chance, PoB's rotation simulation), Cast On Critical Strike, Cast when Damage Taken, Cast when Stunned; `showAverage` a hit-scoped marker; Smite's aura and Arc as other gems; four fixtures; DIV-0175 to DIV-0180; `docs/phase-6/triggers.md`
- [x] P6-8a part 2: Cast while Channelling (`CWCHandler`), Cast on Melee Kill, Cast on Ward Break; triggered damage (`TRIGGERED_DAMAGE`); Cyclone's unscalable movement buff; five fixtures; DIV-0181 to DIV-0184; `docs/phase-6/triggers.md`, "Part 2"
- [ ] P6-8a further configs: Spellslinger, Arcanist Brand, Mark on Hit, Hextouch, Cast on Death, item triggers; the Holy Relic's Nova (data: skill level requirements)
## P6-8b: mirages
- [x] P6-8b: Mirage Archer (`CalcOptions::mirage`, the copy computed by `calculate`, its DPS in `combined-dps`); fixture `mirage-archer-burning-arrow-templar.xml`; DIV-0185 to DIV-0188; `docs/phase-6/mirages.md`
- [ ] P6-8b rest: General's Cry's mirage warriors, Sacred Wisps (each an unmapped-stat reason meanwhile)
## P6-9: the remaining gems (supports, marks, curse priority, exerts, banners, heralds)
- [x] Order change (decision without the maintainer, D10): P6-9 before P6-7c, since every minion build's fields carry the summoning gem's `unmodelled-gem` until P6-9 models it, so P6-7c's minion hit fields could not be shown `Complete`
- [x] P6-9a: 15 inert skills, 43 plain supports, nine auras (Anger, Wrath, Determination, Grace, the four Purities, Vitality); `gem_effects` 894 values, 0 differences; three aura fixtures; `docs/phase-6/gems.md`
- [x] P6-9b: PoB's curse slots and priority, the curse limit stat, a hex's own curse effect, six hexes (Despair, Flammability, Vulnerability, Conductivity, Enfeeble, Temporal Chains); `gem_effects` 954 values, 0 differences; two curse fixtures; `docs/phase-6/gems.md`, "P6-9b"
- [x] P6-9c: summoning skills inert for the character (20, each minion's skills checked in PoB's export: no `GlobalEffect`), six minion supports; `gem_effects` 1,110 values, 0 differences; minion sweep 109 `Complete` minion fields (80 before Vaal Summon Skeletons' level registered); fixtures: four minion builds and Vaal Summon Skeletons with DIV-0111 to DIV-0113; `docs/phase-6/gems.md`, "P6-9c"
- [x] P6-9d: skills with their own data (16 inert), Hatred, Herald of Thunder, Malevolence, Enduring Cry; `gem_effects` 1,237 values, 0 differences; fixture with Enlighten; `docs/phase-6/gems.md`, "P6-9d"
- [x] The review of P6-7b3b to P6-9c applied (`ModStore::more_rounded` in every phase, the minion's unread totals, unsettled minion-only values, conditional gem minion modifiers, the export scripts committed); `docs/phase-6/gems.md`, "The review"
- [x] P6-9e: marks (PoB's mark rule; `EnemyProjectileDamageTaken`, `EnemyPhysicalDamageReduction`), Pride (`BuffStat::ConfigIs`), Punishment (`BuffStat::WhenState`, `conditionEnemyLowLife` settled), guards (`EffectType::Guard`, PoB's guard slots), Tempest Shield (disabled without a shield, still reserves), offerings (`BuffStat::MinionOnly`), Chaos and Carrion golems, General's Cry, 17 plain supports; `gem_effects` 1,476 values, 0 differences; 7 fixtures (Q11 DIV-0114 to DIV-0122); `docs/phase-6/gems.md`, "P6-9e"
- [x] P6-9f: Arrogance (life reservation, `LIFE_RESERVATION_SUPPORTS`), six inert skills; `docs/phase-6/gems.md`, "P6-9f"
- [ ] P6-9 remaining: warcry power (Battlemage's, Ancestral, Infernal, Seismic Cry), Eternal Blessing, Skitterbots (data run), the other skills with their own buffs (`docs/phase-6/gems.md`, "P6-9e", "Not yet")
## P6-10a: timeless jewels
- [x] P6-10a part 1: extraction roots `TIMELESS_TABLES_POE1` (the three tables read; 718 rows as PoB's export); `docs/phase-6/timeless.md`
- [ ] Maintainer: the data run on build 26530219's files (this workstation's install is 26694821) and the decision on the seed algorithm's source (Q-P6-1: GPL reference not allowed; MIT ports exist); then `poe1/timeless.json`, its `data-parity` table, and the port
## P6-10b: recovery and defence outputs
- [x] P6-10b part 1: life and mana regeneration (`poe1::regen`, fields `life-regen` and `mana-regen`, slice revision 6; Arcane Surge, consecrated ground, the stationary and stopped-DoT state rows; `mapping::REGEN_REASONS` and `elsewhere` keep reasons on these fields); fixtures `ice-nova-regeneration-witch.xml`, `ice-nova-arcane-surge-regeneration-witch.xml`; every regeneration value equal to PoB; `docs/phase-6/recovery.md`
- [x] P6-10b part 2: energy shield regeneration, recharge, and delay (slice revision 7), Zealot's Oath, the rare or unique enemy nearby condition (`conditions::ENEMY_RARE_OR_UNIQUE_NEARBY`); fixtures `ice-nova-energy-shield-recovery-witch.xml`, `ice-nova-zealots-oath-witch.xml`; every energy shield value equal to PoB; `docs/phase-6/recovery.md`, "Part 2"
- [x] P6-10b part 3: movement speed (`poe1::movement`, fields `movement-speed` and `effective-movement-speed`, slice revision 8), the armour bases' movement penalty, Onslaught's and Adrenaline's movement speed, `mapping::MOVEMENT_REASONS`; fixture `ice-nova-movement-speed-witch.xml`; every movement value equal to PoB; `docs/phase-6/recovery.md`, "Part 3"
- [x] P6-10b part 4: the main skill's mana cost (`poe1::costs`, field `mana-cost`, slice revision 9; `mapping::COST_REASONS`); fixture `ice-nova-mana-cost-witch.xml`; every mana cost equal to PoB; `docs/phase-6/recovery.md`, "Part 4"
- [x] The review of P6-10b parts 1 to 4 applied (cost efficiency more and clamp order, Zealot's Oath keeping life's reasons, shared marker and action speed helpers, tests); `docs/phase-6/recovery.md`, "The review of parts 1 to 4"
- [x] P6-10b part 5: base damage reduction (`poe1::reduction`, slice revision 12; endurance charges, Adrenaline, the stationary row, `REDUCTION_REASONS`); 850 values equal to PoB; `docs/phase-6/recovery.md`, "Part 5"
- [x] P6-10b part 6: the mana cost per second (`costs::per_second`, slice revision 13); 85 equal to PoB, 44 Q11 (DIV-0189 to DIV-0232); `docs/phase-6/recovery.md`, "Part 6"
- [x] P6-10b part 7: the leech caps (`poe1::leech`, slice revision 15; seven `Increased` rows); each field equal to PoB in 177 builds; `docs/phase-6/recovery.md`, "Part 7"
- [x] P6-10b part 8: the life and energy shield costs (`costs`, slice revision 16; Lifetap, the life and mana mastery and its attack form, Lifetap's buff); fixtures `cost-lifetap-ice-nova-witch.xml`, `cost-life-share-ice-nova-witch.xml`, `cost-attack-life-share-smite-templar.xml`; DIV-0273 to DIV-0278; `docs/phase-6/recovery.md`, "Part 8"
- [x] P6-10b part 9: the leech rates (`leech::rates`, slice revision 17; 32 leech rows from PoB's parse; Blood Rage's buff); fixtures `leech-ice-nova-witch.xml`, `leech-instant-cap-ice-nova-witch.xml`; DIV-0279 to DIV-0281; `docs/phase-6/recovery.md`, "Part 9"
- [x] P6-10b part 10: damage taken over time (`poe1::taken`, slice revision 18; `phases::resistance_in` for the over-time resistance; 15 rows); each field equal to PoB in 175 builds; `docs/phase-6/recovery.md`, "Part 10"
- [x] P6-10b part 11: the chance to evade (`poe1::evade`, slice revision 19; `hit::pob_hit_chance`; Blind's less accuracy; Unwavering Stance); equal to PoB in 182 builds; `docs/phase-6/recovery.md`, "Part 11"
- [x] P6-10b part 12: the enemy's hit (`poe1::enemy_hit`, slice revision 20; `CalcData::monster_damage`; 31 rows; fortification, delirium, Enfeeble; `BuffStat::Defence`); equal to PoB in 169 builds; `docs/phase-6/recovery.md`, "Part 12"
- [x] P6-10b part 13: maximum hit taken (`enemy_hit::maximum_hit`, slice revision 21; the three marker kinds; hit recently and moving settled); equal to PoB in 168 builds; `docs/phase-6/recovery.md`, "Part 13"
- [x] P6-10b part 14: the effective health pool (`poe1::ehp`, slice revision 22; hits to die, block, suppression, not hit; `BlockEffect`, `AvoidProjectiles`, `EhpReason`); equal to PoB in 168 builds; `docs/phase-6/recovery.md`, "Part 14"
- [x] P6-10b part 15: mind over matter in maximum hit and the hits to die (`ManaBeforeLife`; the keystone row; `enemy_hit::mom_hit_pool`); fixtures `mom-ice-nova-witch.xml`, `mom-physical-ice-nova-witch.xml`; `docs/phase-6/recovery.md`, "Part 15"
- [ ] P6-10b further parts: EHP's other pools (guard, ward, life loss prevention, recoup, Eldritch Battery), damage over time EHP, a dual wielder's leech, soul costs (bundle needs `VaalSouls`), degeneration (D7), the reduction against a hit, maximum hit and EHP
## P6-11: stat mop-up (batches)
- [x] P6-11a: shields and dual wielding (`DualWielding` state, six rows; fixture `smite-shield-templar.xml`, Q11 DIV-0100, DIV-0101); `docs/phase-6/stat-mop-up.md`
- [x] P6-11a PR #158 (base `feat/p6-7b2b-actor-review`)
- [x] P6-11b: weapon types (PoB's weapon `ModFlag`s per hand, `KeywordFlag.Bow`, ailments' `WeaponMask`; 43 hit rows, 3 state rows with `UsingWand`, 2 `NoEffect`), the P6-11a review (one `DualWielding` definition, three shield rows), and the dual-wield hit DPS fix (combined speed; DIV-0074 retired); fixtures `smite-axe-lines-templar.xml`, `smite-dual-wield-lines-templar.xml` (Q11 DIV-0102 to DIV-0107); `docs/phase-6/stat-mop-up.md`
- [x] P6-11b PR #159 (base `feat/p6-11a-stat-batch`)
- [x] P6-11c: the P6-11b review (an attack's DoT keeps its weapon flags, per-hand ailment flags, Intimidating Cry's uptime on the combined speed via `hand_rate`/`hand_hit`, `MainHandWeapon` replaced by `Flags::WEAPON`, staff lines on `Flags::STAFF`, `KEYWORD_ARROW`, wand damage); fixture `burning-arrow-bow-lines-templar.xml` (Q11 DIV-0108 to DIV-0110); `docs/phase-6/stat-mop-up.md`
- [x] P6-11c PR #160 (base `feat/p6-11b-stat-batch`)
- [x] P6-11d: the P6-11c review (arrow keyword from the active gem only, an attack DoT's main-hand flags with `Unarmed`, weapon-row and dual-wield Intimidating Cry tests); `docs/phase-6/stat-mop-up.md`
- [x] P6-11d PR #161 (base `feat/p6-11c-weapon-review`)
- [x] P6-11e: the P6-11d review (an attack DoT's main hand as PoB's `getWeaponFlags`, with the skill's weapon check and reasons; tests); `docs/phase-6/stat-mop-up.md`
- [x] P6-11e PR #162 (base `feat/p6-11d-arrow-review`)
- [x] P6-11f: block per 5% (`BlockPer5`, PoB's `PerStat`), evasion per frenzy charge, eight `NoEffect` rows; fixture `ice-nova-block-per-5-witch.xml`; `docs/phase-6/stat-mop-up.md`, "P6-11f" (PR #167)
- [x] P6-11g: `BlockedRecently` from any of its three checks (`conditions::CHECKS` with several inputs per condition), the armour if Blocked Recently row, 26 `NoEffect` rows (regeneration, leech, recovery, recharge, costs, keystones without a field); fixture `smite-blocked-recently-no-effect-templar.xml` (Q11 DIV-0125, DIV-0126); `docs/phase-6/stat-mop-up.md`, "P6-11g"
- [x] P6-11h: Glancing Blows (`BlockChance` MORE 100), the resolver's lone hidden stat assumed 1, `EnemyExplode` out of the gem table, the maintainer-run real-bundle tests passing again (`mapping::no_effect_by_rule`); fixture `ice-nova-glancing-blows-witch.xml`; `docs/phase-6/stat-mop-up.md`, "P6-11h"
- [x] The review of P6-9e to P6-11h applied (total minion field copies with an engine test, enemy states, enemy quantities out of `SupportOnMinion`, `ExtraMinionSkill`, the minion skill's own list, unvalidated minion fields not computed, `settle`'s removal, the weapon sentinel, resolver tests; Spike B's rate and data-parity unchanged); `docs/phase-6/minions.md`, "The review of P6-9e to P6-11h"
- [x] P6-11i: six flask lines that reach no field (`flasks::NO_EFFECT_LOCAL`)
- [x] P6-11j: the chance to ignore the enemy's physical damage reduction with PoB's config mode (`hit::ignore_reduction`); fixtures `smite-ignore-pdr-average-templar.xml`, `smite-ignore-pdr-max-templar.xml` (Q11 DIV-0127 to DIV-0132); `docs/phase-6/stat-mop-up.md`, "P6-11i and P6-11j"
- [x] P6-11k: the equipped shield's energy shield and defences (`Poe1Quantity::SlotIncreased`, a piece's own modifier), the effect of your auras on you (`SkillAuraEffectOnSelf`); fixture `ice-nova-shield-defences-aura-self-witch.xml`; 34 values toward PoB; `docs/phase-6/stat-mop-up.md`, "P6-11k"
- [x] P6-11l: more maximum life with at least 6 Life Masteries (`PassiveKind::life_mastery`, `states::SIX_LIFE_MASTERIES`); three corpus builds' Life equal to PoB; `docs/phase-6/stat-mop-up.md`, "P6-11l"
- [x] P6-11m: 21 `NoEffect` rows (PoB-parsed, no slice field); global unmapped pairs 418 to 407; `docs/phase-6/stat-mop-up.md`, "P6-11m"
- [x] P6-11u: durations, projectile counts, and chains as `NoEffect` with guards; `docs/phase-6/stat-mop-up.md`, "P6-11u"
- [x] P6-11t: regeneration per charge and per stat (spell block, uncapped fire resistance), channelling mana costs; `docs/phase-6/stat-mop-up.md`, "P6-11t"
- [x] P6-11s: damage per aura or herald affecting you (`stacks::AuraOrHerald`, `gems::aura_or_herald_count`); `docs/phase-6/stat-mop-up.md`, "P6-11s"
- [x] P6-11r: supports an item grants (`item_supports::ROWS`, 203 rows from PoB's parse), PoB's `addBestSupport`, Blasphemy modelled; `docs/phase-6/stat-mop-up.md`, "P6-11r"
- [x] P6-11q: socketed gem levels per slot (`gem_levels::SOCKETED_STATS`, `GemSlot`, `InSlot`), levels checked against PoB with a local probe; `docs/phase-6/stat-mop-up.md`, "P6-11q"
- [x] P6-11p: Onslaught on kill, `KilledRecently` settled from the check and the kill count; `docs/phase-6/stat-mop-up.md`, "P6-11p"
- [x] P6-11o: `AffectedBy<Aura>` settled from the gem table's auras (`gems::affected_by`, `conditions::AFFECTED_BY`), 12 state rows; `docs/phase-6/stat-mop-up.md`, "P6-11o"
- [x] P6-11n: the flask mapping honours `NoEffect` rows; 13 hit rows, a damage over time row, 3 `NoEffect` rows, 3 `EhpReason` markers; 52 values toward PoB; `docs/phase-6/stat-mop-up.md`, "P6-11n"
## P6-12: combined and full DPS; report
- [x] P6-12 part 1: `combined-dps` (`poe1::combined`, slice revision 10; Culling Strike's multiplier); a flag stat at 0 sets no flag (`smite-bleed-templar.xml`'s bleeding computed again); Q11 DIV-0133 to DIV-0173; `docs/phase-6/combined-dps.md`
- [x] P6-12: a skill without a hit adds PoB's 0 (`combined::has_hit`); a skill PoB disables for the hands has no mana cost (`MainSkill::disabled_by_hands`), weapon types compared as PoB's (`skill::pob_weapon_type`); `docs/phase-6/recovery.md`, "A disabled skill's cost"
- [x] P6-12 part 2: `full-dps` (`poe1::full_dps`, slice revision 14, `CalcOptions::nested`); 97 equal to PoB, 40 Q11 (DIV-0233 to DIV-0272); `docs/phase-6/combined-dps.md`, "Part 2"
- [x] P6-12: the Phase 6 report `docs/phase-6/phase-6-report.md` (at P6-11n: public 13,547 `Complete`, corpus 0; register 281; the gate not met yet; the bench re-checked, every budget met, `calculate` 6.7 to 64.9 ms from full DPS); updated as later steps land
