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
- [x] The cluster tables on build 26530219: extract `26530219-ot2` writes `poe1/cluster_jewels.json`; data-parity `uncategorized` 0 (rule `cluster-notable-order-duplicate-name`); evidence bundle `b-cl`: public 255 toward PoB, 31 away (earlier gaps uncovered), 0 unexpected (`docs/tree/cluster-jewels.md`, "The cluster tables on build 26530219")
- [x] Data schema 5: gem levels carry their level requirement and Vaal soul costs (`EffectLevel`, PoB export format 14, data-parity rule `skills-level-requirement-single-level`); the minion's level follows PoB's rule (enemy, character up to a cap, then the game's stat, then the requirement); evidence bundle `b-gl` (`docs/phase-6/minions.md`, "Data schema 5")
- [ ] A fixture with `set_maximum_life_is_one`: its minions (Summon Elemental Relic, Summon Sentinel of Radiance) are item-granted skills, so a gem fixture cannot reach them; the public builds that use them are computed since data schema 5
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
- [x] P6-9g: Mark On Hit (the mark's curse effect; PoB reads no trigger stat of it), Eternal Blessing (no reservation, aura effect); fixtures `ice-nova-mark-on-hit-witch.xml`, `ice-nova-eternal-blessing-hatred-witch.xml` (`docs/phase-6/gems.md`, "P6-9g")
- [x] P6-11bb: charge stats of several kinds (`charges::MULTI`); fixture `ice-nova-total-charges-lines-witch.xml`; public 80 toward, 41 now equal (`docs/phase-6/stat-mop-up.md`, "P6-11bb")
- [x] P6-11bc: rage on attack hit, minion area, avoid shock for all ailments; public 11 toward (`docs/phase-6/stat-mop-up.md`, "P6-11bc")
- [x] P6-11bd: bleeding's DoT multiplier; stun and minion aggression `NoEffect`; public 4 toward (`docs/phase-6/stat-mop-up.md`, "P6-11bd")
- [x] P6-11be: reflect, minion recovery on death, explode, hexproof `NoEffect` (`docs/phase-6/stat-mop-up.md`, "P6-11be")
- [x] P6-11bf: damage with hits against chilled enemies (`DamageVsChilled`, applied by `damage_taken::enemy_states`); fixture `ice-nova-damage-vs-chilled-witch.xml` (`docs/phase-6/stat-mop-up.md`, "P6-11bf")
- [x] P6-11bg: the frenzy charges' maximum from the power charges' (`ChargeValue::MaximumFromPower`); fixture `ice-nova-frenzy-max-from-power-witch.xml` (`docs/phase-6/stat-mop-up.md`, "P6-11bg")
- [x] P6-11bh: the minion's added damage of every type (`docs/phase-6/stat-mop-up.md`, "P6-11bh")
- [x] P6-11bi: the socketed skills' cost and reservation multiplier (`SocketedCostMultiplier`, `cost_mores`); fixture `ice-nova-socketed-cost-multiplier-witch.xml` (`docs/phase-6/stat-mop-up.md`, "P6-11bi")
- [x] P6-11bj: life and energy shield recovered on block while wielding a staff (`BlockGain::EnergyShieldSpellPercent`); fixture `ehp-staff-recover-on-block-ice-nova-witch.xml` (`docs/phase-6/stat-mop-up.md`, "P6-11bj")
- [x] P6-11bk: totem, trap, and mine keywords from the skill types; totem and mine damage (`Flags::KEYWORD_TOTEM`, `KEYWORD_TRAP`, `KEYWORD_MINE`); fixture `ice-nova-totem-mine-damage-lines-witch.xml` (`docs/phase-6/stat-mop-up.md`, "P6-11bk")
- [x] P6-11bl: energy shield from the equipped helmet; counting as having maximum charges (`ChargeValue::CountAsMaximum`); fixtures `ice-nova-count-as-max-charges-witch.xml`, `ice-nova-helmet-energy-shield-witch.xml` (`docs/phase-6/stat-mop-up.md`, "P6-11bl")
- [x] P6-11bm: seven more rage lines as `CanGain`; fixture `smite-rage-on-hit-with-axes-templar.xml` (Q11 DIV-0302 to DIV-0307) (`docs/phase-6/stat-mop-up.md`, "P6-11bm")
- [x] P6-11bn: nearby enemies' resistances (`EnemyResistance` rows); fixture `ice-nova-nearby-enemy-resistances-witch.xml` (`docs/phase-6/stat-mop-up.md`, "P6-11bn")
- [x] P6-11bo: the ends of the damage range (`BoundDamage`; Volatility, Ryslatha's Clutches); fixture `smite-volatility-min-max-physical-templar.xml` (Q11 DIV-0308 to DIV-0313) (`docs/phase-6/stat-mop-up.md`, "P6-11bo")
- [x] P6-11bp: gain as extra damage (eleven `GainAs` rows; `GainAs` a hit quantity); fixture `smite-gain-as-extra-templar.xml` (Q11 DIV-0314 to DIV-0319) (`docs/phase-6/stat-mop-up.md`, "P6-11bp")
- [x] P6-11bq: life regeneration per endurance charge (`PerCharge::LifeRegenPerMinute`); a test, no fixture (a per-minute line from text is unsettled) (`docs/phase-6/stat-mop-up.md`, "P6-11bq")
- [x] P6-11br: five NoEffect rows (stun, gem experience, projectile directions, two flask lines) (`docs/phase-6/stat-mop-up.md`, "P6-11br")
- [x] P6-11bu: life regeneration per fortification (`RegenPerFortification`, `enemy_hit::fortification_stacks`); a test (`docs/phase-6/stat-mop-up.md`, "P6-11bu")
- [x] P6-11bx: elemental resistances treated as inverted (`InvertElementalResistChance`); fixture `ice-nova-inverted-resistance-witch.xml` (`docs/phase-6/stat-mop-up.md`, "P6-11bx")
- [x] P6-11by: the body armour's armour and energy shield with hybrid helmet, gloves, and boots (`BodyArmourIfHybridPieces`, `phases::hybrid_pieces`) (`docs/phase-6/stat-mop-up.md`, "P6-11by")
- [x] P6-11bz: a flask's enemy damage taken on consecrated ground and while ignited (`flasks::FLASK_ROWS`); fixture `ice-nova-flask-enemy-damage-taken-witch.xml` (`docs/phase-6/stat-mop-up.md`, "P6-11bz")
- [x] P6-11ca: Pulverise, attacks' additional crit chance, a flask's crit on consecrated ground; fixtures `smite-pulverise-attack-crit-templar.xml` (Q11 DIV-0327 to DIV-0332), `ice-nova-flask-consecrated-crit-witch.xml` (`docs/phase-6/stat-mop-up.md`, "P6-11ca")
- [x] P6-11cb: energy shield increased by spell block chance or chaos resistance; fixtures `ice-nova-energy-shield-from-spell-block-witch.xml`, `ice-nova-energy-shield-from-chaos-resistance-witch.xml` (`docs/phase-6/stat-mop-up.md`, "P6-11cb")
- [x] P6-11cc: block if hit recently; a weapon's poison chance; fixtures `ehp-block-if-hit-recently-ice-nova-witch.xml`, `smite-weapon-poison-chance-templar.xml` (Q11 DIV-0333 to DIV-0339) (`docs/phase-6/stat-mop-up.md`, "P6-11cc")
- [x] P6-11cd: the spectre limit, `NoEffect` (`docs/phase-6/stat-mop-up.md`, "P6-11cd")
- [x] P6-11ce: life loss prevented in the hit pools, each hit, and the speed-up (`LifeLossPrevented`); fixture `ehp-life-loss-prevented-flask-ice-nova-witch.xml` (`docs/phase-6/recovery.md`, "P6-11ce")
- [x] P6-11cf: projectile damage against nearby enemies; trap damage; totems' crit multiplier and attack speed; reflected damage `NoEffect`; fixture `burning-arrow-projectile-trap-totem-lines-templar.xml` (Q11 DIV-0340 to DIV-0345) (`docs/phase-6/stat-mop-up.md`, "P6-11cf")
- [x] P6-11cg: physical damage reduction per hit taken recently (`StackKind::HitsTakenRecently`); fixture `ehp-reduction-per-hit-taken-ice-nova-witch.xml` (`docs/phase-6/stat-mop-up.md`, "P6-11cg")
- [x] P6-11ch: Arcane Surge's effect and its more spell damage; fixture `ice-nova-arcane-surge-effect-witch.xml` (`docs/phase-6/stat-mop-up.md`, "P6-11ch")
- [x] P6-11ci: the character's warcry speed in Intimidating Cry's terms (`SkillTiming::WarcrySpeed`); fixture `smite-intimidating-cry-warcry-speed-templar.xml` (Q11 DIV-0346 to DIV-0353) (`docs/phase-6/gems.md`, "P6-11ci")
- [x] P6-11cj: Empower's supported level (`skill::supported_level`); fixture `ice-nova-empower-hatred-witch.xml` (`docs/phase-6/gems.md`, "P6-11cj")
- [x] P6-11ck: Enhance's supported quality; fixture `smite-enhance-quality-templar.xml` (Q11 DIV-0354 to DIV-0359) (`docs/phase-6/gems.md`, "P6-11ck")
- [x] P6-11cl: the typed supported levels (`skill::SUPPORTED_TYPED_LEVEL_STATS`); fixture `ice-nova-awakened-added-cold-witch.xml` (`docs/phase-6/gems.md`, "P6-11cl")
- [x] P6-11cm: minion damage if you've used a minion skill recently (`conditions::used_minion_skill`, minion modifiers with the character's settled conditions); three fixtures (`docs/phase-6/stat-mop-up.md`, "P6-11cm")
- [x] P6-11cn: armour and physical and elemental damage reduction per endurance charge (`charges::PerCharge`); fixture `ice-nova-endurance-reduction-lines-witch.xml` (`docs/phase-6/stat-mop-up.md`, "P6-11cn")
- [x] P6-11co: projectile attack skills' crit chance and multiplier (`Flags::SKILL_RANGED_ATTACK`); fixture `burning-arrow-projectile-attack-crit-templar.xml` (Q11 DIV-0360 to DIV-0365) (`docs/phase-6/stat-mop-up.md`, "P6-11co")
- [x] P6-11cp: ignoring the armour movement penalties (`Poe1Quantity::IgnoreMovementPenalties`, `movement::IGNORE_MOVEMENT_PENALTIES`); two fixtures (`docs/phase-6/stat-mop-up.md`, "P6-11cp")
- [x] P6-11cq: life regeneration while using a life flask (`flasks::using_life_flask`, `conditions::USING_LIFE_FLASK`); two fixtures (`docs/phase-6/stat-mop-up.md`, "P6-11cq")
- [x] P6-11cr: Cyclone's movement speed as the main skill (`SkillMapping`, `mapping::CYCLONE_MOVEMENT_STAT`); PoB checked with the check off (`docs/phase-6/stat-mop-up.md`, "P6-11cr")
- [x] P6-11cs: minion more damage (Necromancer) and minion crit multiplier (`MinionValue::CritMultiplier`); fixture `minion-raise-zombie-more-damage-crit-multiplier-witch.xml` (`docs/phase-6/stat-mop-up.md`, "P6-11cs")
- [x] P6-11ct: minion damage while affected by a herald (`conditions::AFFECTED_BY_HERALD`); fixture `minion-raise-zombie-herald-damage-witch.xml` (`docs/phase-6/stat-mop-up.md`, "P6-11ct")
- [x] P6-11cu: minion penetration against cursed enemies (`gems::curses_enemy`, `conditions::ENEMY_CURSED`, `MinionValue::Penetration`); two fixtures (`docs/phase-6/stat-mop-up.md`, "P6-11cu")
- [x] Temporal Chains' `buff_time_passed_+%_other_than_temporal_chains` was `NoSlice` in `gems::SKILLS`, but PoB lengthens the ailments on the cursed enemy (found in P6-11cu): ported in P6-11cv
- [x] P6-11cv: Temporal Chains lengthens the ailments (`Poe1Quantity::EnemyBuffExpireFaster`, PoB's `debuffDurationMult`); two fixtures (Q11 DIV-0366 to DIV-0371) (`docs/phase-6/stat-mop-up.md`, "P6-11cv")
- [x] P6-11cw: attack damage per 450 armour (`hit::DAMAGE_PER_STAT`, generalizing P6-11bw); fixture `smite-attack-damage-per-armour-templar.xml` (Q11 DIV-0372 to DIV-0377) (`docs/phase-6/stat-mop-up.md`, "P6-11cw")
- [x] P6-11cx: brand and mark skills' cast speed (`skill::SKILL_TYPE_FLAGS`); fixture `ice-nova-brand-mark-cast-speed-witch.xml` (`docs/phase-6/stat-mop-up.md`, "P6-11cx")
- [x] P6-11cy: trap throwing and mine laying speed, Trap and Mine Damage, Trap Support's damage; fixture `ice-nova-trap-throwing-speed-witch.xml` (`docs/phase-6/stat-mop-up.md`, "P6-11cy"). Mines' own support stats (costs as reservation, detonation) stay reasons
- [x] P6-11bv: while Leeching (the four checks in `conditions::CHECKS`, the automatic case's stats in `config::DEPENDS_ON`, seven state rows); fixture `ice-nova-leeching-lines-witch.xml` (`docs/phase-6/stat-mop-up.md`, "P6-11bv")
- [x] P6-11bw: spell damage per 100 maximum mana (`hit::spell_damage_per_mana`, a computed term in the damage increase, as `phases::per_5`); fixtures `ice-nova-spell-damage-per-mana-below-witch.xml`, `ice-nova-spell-damage-per-mana-capped-witch.xml` (`docs/phase-6/stat-mop-up.md`, "P6-11bw")
- [ ] Maintainer (game truth): "#% increased maximum Life per Red Socket on equipped Staff" and the mana per blue socket line. PoB parses them with a `Multiplier:<Colour>SocketInWeapon 1` it never sets (`CalcSetup.lua` counts only empty and socketed-gem sockets), so they give nothing there. Applying them needs an in-game check and divergence entries; reasons meanwhile.
- [ ] "Grants #% increased Attack Speed per 8% Quality" and "Your Offerings have #% increased Effect": PoB leaves text unparsed, so it applies neither; the game does. Reasons until a game check.
- [x] Minion damage "if you've used a Minion Skill Recently": done in P6-11cm.
- [ ] PoB-only skills (`EnemyExplode`, "On Kill Monster Explosion", in 6 public builds): no game granted effect has the ID, so the gem is `unmodelled-gem` before any table is read; it reaches a field only as the main skill or in full DPS, and none of the builds includes it
- [x] P6-11bs: warcry power and uptime (`gems::warcry_terms`, `BuffStat::PerPower`, `skill_modifiers::SkillTiming`); Enduring Cry's regeneration and Intimidating Cry's movement speed; fixtures `ice-nova-warcry-power-witch.xml`, `ice-nova-warcry-power-config-witch.xml` (`docs/phase-6/gems.md`, "P6-11bs")
- [x] P6-11bt: Ruthless (`hit::ruthless_effect`); fixture `smite-ruthless-bleed-templar.xml` (Q11 DIV-0320 to DIV-0326) (`docs/phase-6/gems.md`, "P6-11bt")
- [ ] Warcry rest: Seismic Cry's armour, exertion for Seismic, Battlemage's, Ancestral, Infernal Cry (their gems are not modelled)
- [ ] Spellslinger (the supported spell reserves and triggers from wand attacks), Frost Shield (stages, the enemy in its globe), Blood and Sand, Bear Trap, mines
- [ ] P6-9 remaining: Skitterbots (data run), the other skills with their own buffs (`docs/phase-6/gems.md`, "P6-9e", "Not yet")
## P6-10a: timeless jewels
- [x] P6-10a part 1: extraction roots `TIMELESS_TABLES_POE1` (the three tables read; 718 rows as PoB's export); `docs/phase-6/timeless.md`
- [x] Q-P6-1 decided 2026-11-19: PoB's lookup tables, not a ported generator (the "MIT" ports are GPL-3.0 or unlicensed); plan decision 11 amended; the local consistency check: PoB's tables and Vilsol's agree on all 149,920 sampled lookups (`docs/phase-6/timeless.md`)
- [x] Build 26530219's full depot on this workstation (`local/b2/steam-26530219`, validated 2026-11-19)
- [x] The data run with the timeless roots: extract `26530219-p6` from the full depot (2026-11-19); `poe1/timeless.json` (`bundle`), `poe1/timeless_luts.json` and the 11 results files (`timeless-pack`, all types, plan decisions T1 to T3); the validator's `BUNDLE_TIMELESS`; PoB export format 15; data-parity table `timeless` (7,041 rows, 80 categorized differences, every checksum equal); evidence bundle `b-tl` (`docs/phase-6/timeless.md`, "Part 2")
- [x] The engine for the six types that are not Abyss jewels: `loom_pipeline::timeless` (`socketed`, `conquests`, `needed`), `ResolvedPassives.conquered`, the app reads a type's results on demand (`GameData::timeless_results`); `TimelessKeystone` gone; the parser's selected and unselected runs (`check_links` alike); life recoup, two `NoEffect` rows; four mechanic fixtures, 77 fields each equal to PoB (`docs/phase-6/timeless.md`, "Part 3")
- [x] The Abyss jewels: `loom_data::bundle::timeless::abyss` (PoB's `DataAbyssJewelLookUpTableHelper.lua`), the engine's socket records and Zorath's path and ascendancy picks; five mechanic fixtures equal to PoB but DIV-0294, DIV-0295 (`pob-bug`, PoB's per-second rounding of per-minute rolls); `docs/phase-6/timeless.md`, "Part 4"
- [x] P6-11aw: devotion (`base_devotion` `NoEffect`, its 30 readers as guards) and ward (`EHP_REASONS`); fixtures `timeless-militant-faith-ice-nova-witch.xml` (77 fields), `timeless-heroic-tragedy-ice-nova-witch.xml` (71 fields, EHP partial); `docs/phase-6/stat-mop-up.md`, "P6-11aw"
- [x] P6-11ay: added damage with each weapon type (64 rows), avoid shock and ignite; fixture `burning-arrow-weapon-added-damage-templar.xml`, DIV-0296 to DIV-0301 (Q11) (`docs/phase-6/stat-mop-up.md`, "P6-11ay")
- [x] P6-11az: devotion and ten of its readers (`poe1::devotion`, each the stat it means times the count); three Militant Faith fixtures (`docs/phase-6/stat-mop-up.md`, "P6-11az")
- [x] P6-11ba: the Abyss additions' other stats: blind, taunt, hinder chances `NoEffect`; 15 state rows (dual wielding, shield, been hit recently, moving); fixture `ice-nova-moving-hit-recently-lines-witch.xml` (`docs/phase-6/stat-mop-up.md`, "P6-11ba")
- [ ] The rest: the minion chances to ignite and bleed, the minion's added chaos damage, the two-handed variants (`UsingTwoHandedWeapon` unsettled); devotion's other readers; a ward pool
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
- [ ] P6-10b further parts: EHP's other pools (ward; life loss prevention done in P6-11ce, without Petrified Blood's below-half share; guard, recoup, Eldritch Battery done in P6-11ae, P6-11am, P6-11ap), damage over time EHP, a dual wielder's leech, soul costs (bundle needs `VaalSouls`), degeneration (D7), the reduction against a hit, maximum hit and EHP
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
- [x] P6-11au: mana regeneration if you've inflicted exposure recently (`exposure::STATS`, `Poe1Quantity::RegenIfExposureApplied`), `ElementalHitAltX` and `SupportMoreDuration` inert; `docs/phase-6/stat-mop-up.md`, "P6-11au"
- [x] P6-11at: herald buff effect (`Poe1Quantity::HeraldBuffEffect` in `skill_modifiers`), three `NoEffect` rows; one mechanic fixture; `docs/phase-6/stat-mop-up.md`, "P6-11at"
- [x] P6-11as: minion added cold damage and double damage (`MinionValue::AddedDamage`, `DoubleDamageChance`), mana recoup in `anyRecoup` (`Poe1Quantity::ManaRecoup`), four `NoEffect` rows; two mechanic fixtures; `docs/phase-6/stat-mop-up.md`, "P6-11as"
- [x] P6-11ar: Summon Stone Golem of Safeguarding's life before yours (`Poe1Quantity::TakenFromStoneGolem`, `enemy_hit::stone_golem_life` by a nested calculation, the ally pool in the maximum hit and the effective health pool, `TotalStoneGolemLife` applied); two mechanic fixtures; `docs/phase-6/stat-mop-up.md`, "P6-11ar"
- [x] P6-11aq: Withered on the enemy (`gems::can_wither`, `damage_taken::withered`, `multiplierWitheredStackCount` read; `withered_effect_+%`; spell chaos added damage; Withering Step a buff); two mechanic fixtures; `docs/phase-6/stat-mop-up.md`, "P6-11aq"
- [x] P6-11ap: Eldritch Battery (`Poe1Quantity::EsProtectsMana`: PoB's `calcMoMEBPool`, the shield before mana in the simulated hits, the mana cost per second as `ESPerSecondCost`, 50% less recharge); two mechanic fixtures; `docs/phase-6/stat-mop-up.md`, "P6-11ap"
- [x] P6-11ao: Eternal Youth; keystones in a Timeless jewel's radius not applied (`loom_pipeline::timeless_keystones`, `PassiveState::TimelessKeystone`, both replaced by P6-10a part 3's conquests); one mechanic fixture; `docs/phase-6/stat-mop-up.md`, "P6-11ao"
- [x] P6-11av: tattoos, runegrafts, and keystone tattoos: extraction roots `TATTOO_TABLES_POE1`, `poe1/tattoos.json`, validator, PoB export format 16, data-parity table `tattoos` (rule `tattoo-display-units`, `uncategorized` 0), the spec's `<Overrides>` applied (`ResolvedPassives::changed`), tattooed nodes as PoB's in every public and corpus build; fixture `tattoos-ice-nova-witch.xml`; evidence bundle `b-tt` (`docs/phase-6/tattoos.md`)
- [x] P6-11ax: catalyst quality settled from the stats' `IsScalable` (bundle `Stat::scalable`, `pob_item::line_values_with`, `LineValues::Catalysed`); Lightning Trap's Mageblood Strength; evidence bundle `b-ct` (`docs/phase-6/stat-mop-up.md`, "P6-11ax")
- [x] The attribute gaps tattoos made visible: Lightning Trap's by P6-11ax; Cyclone's is a `{custom}` staff line the importer excludes by Spike B's plan (`docs/phase-6/tattoos.md`, "Left")
- [ ] Maintainer: whether item `{custom}` lines are applied as PoB does (ADR-015 Amendment 2 covers config custom lines only; Cyclone's staff)
- [x] P6-11an: Blood Magic (keystone row, life reservation flag in `skill_modifiers::Totals`), one mechanic fixture; `docs/phase-6/stat-mop-up.md`, "P6-11an"
- [x] P6-11am: Divine Shield (energy shield capped at armour, its pseudo recoup limiting the estimate's speed-up), two mechanic fixtures; `docs/phase-6/stat-mop-up.md`, "P6-11am"
- [x] P6-11al: lucky spell suppression (`enemy_hit::effective_suppression`), two mechanic fixtures; `docs/phase-6/stat-mop-up.md`, "P6-11al"
- [x] P6-11ak: life and energy shield gained on block in the effective health pool (`BlockGain`, `ehp` gains, PoB's limited speed-up and top-level clamp), three mechanic fixtures; `docs/phase-6/stat-mop-up.md`, "P6-11ak"
- [x] P6-11aj: repeats, Multistrike and Spell Echo (`poe1::repeats`), four mechanic fixtures, DIV-0282 to DIV-0293; `docs/phase-6/stat-mop-up.md`, "P6-11aj"
- [x] P6-11ai: on full life (`conditions::FULL_LIFE`); `docs/phase-6/stat-mop-up.md`, "P6-11ai"
- [x] P6-11ah: fire resistance modifiers applying to cold and lightning (`poe1::resist_conversion`); `docs/phase-6/stat-mop-up.md`, "P6-11ah"
- [x] P6-11ag: heralds' mana reservation efficiency (`skill_modifiers::Totals::reservation_for`); `docs/phase-6/stat-mop-up.md`, "P6-11ag"
- [x] P6-11af: Efficacy as hit rows, damage per herald (`stacks::StackKind::Herald`); `docs/phase-6/stat-mop-up.md`, "P6-11af"
- [x] P6-11ae: guard skills in the defence estimate (`gems::BuffStat::Unscalable`, `enemy_hit`, `ehp`); `docs/phase-6/stat-mop-up.md`, "P6-11ae"
- [x] P6-11ad: low life and low energy shield in the pools (`enemy_hit`), the life thresholds, three mechanic fixtures; `docs/phase-6/stat-mop-up.md`, "P6-11ad"
- [x] P6-11ac: the eldritch block implicit (`monster_base_block_%`); `docs/phase-6/stat-mop-up.md`, "P6-11ac"
- [x] P6-11ab: Rapid Decay and Swift Affliction as hit rows; `docs/phase-6/stat-mop-up.md`, "P6-11ab"
- [x] P6-11aa: Infused Channelling (`gems::BuffStat::CheckModCond`, its damage as a hit row); `docs/phase-6/stat-mop-up.md`, "P6-11aa"
- [x] P6-11z: Arctic Armour's damage taken while stationary; `docs/phase-6/stat-mop-up.md`, "P6-11z"
- [x] P6-11y: Momentum's Swiftness (`gems::BuffStat::PerCount`), the main skill's support global stats no effect in `SkillMapping`; `docs/phase-6/stat-mop-up.md`, "P6-11y"
- [x] P6-11x: Flesh and Stone's Sand Stance (`gems::BuffStat::SandStance`), fortification rounded with `DamageTakenWhenHit`; `docs/phase-6/stat-mop-up.md`, "P6-11x"
- [x] P6-11w: movement speed on burning ground (`conditions::ON_BURNING_GROUND`); `docs/phase-6/stat-mop-up.md`, "P6-11w"
- [x] P6-11v: nearby enemies (`stacks::NearbyEnemies`, Lunaris's three lines); `docs/phase-6/stat-mop-up.md`, "P6-11v"
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

## For review (CI on PR #167, 2026-11-20)
- [x] build round-trip: the synthetic build's pinned field counts follow the slice (10 unsupported, 41 indeterminate at revision 22; `.github/workflows/ci.yml`).
- [x] oracle: over 250 fixtures took 44 to 46 minutes, past the job's 45-minute limit since before P6-10a; the limit is 90 now (maintainer to review).
- [x] e2e (Linux): the node search box is found again before it is cleared ("element not interactable" after the list re-rendered, intermittent).
- [ ] `loom-persistence` `tests::an_empty_build_should_round_trip_through_a_file` failed once in a full workspace run on 2026-11-20 (panic at `src/tests.rs:181`) and passed on rerun: a flaky test, likely a shared temporary path; look at it.
- [ ] e2e (Windows): "session not created: DevToolsActivePort file doesn't exist" in every run since the Phase 6 cascade's merge into dev (`1097108`), on the same runner image as the last pass; the e2e app starts and stays up on this workstation (no driver here). Maintainer to look on a machine with tauri-driver and msedgedriver.
- [x] bench: `calculate` +119.6% over the baseline; re-baselined from CI run 44683273308. No single cause: the bench build's calculation grows with each mechanic, 1.4 ms at `dev` to 2.8 ms (`docs/phase-5/performance.md`, "Re-baselined 2026-11-20"). Maintainer: a profile before Phase 7.
- [x] Linux e2e passes since the fix (CI run 44683273308); dev's `1097108` run failed e2e on both platforms too.
