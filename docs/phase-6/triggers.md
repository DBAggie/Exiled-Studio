# P6-8a: triggers

A triggered skill's rate is PoB v2.67.2's (`Modules/CalcTriggers.lua`), in `loom_calc::poe1::triggers`. The spell path takes that rate in place of the cast rate (`CalcOffence.lua` 2223 to 2227).

## Part 1: the shared handler, Cast On Critical Strike, Cast when Damage Taken, Cast when Stunned

**Which trigger** (`calcs.triggers`, 1592 to 1617): PoB looks a config up by the skill's id, its name, then the name of the trigger support that applies to it.
- **Ported** (`triggers::Config`):
  - Cast On Critical Strike and its awakened gem: the source is the attack of the same group with the highest rate; the triggered skills are the group's spells it supports.
  - Cast when Damage Taken (with its threshold) and Cast when Stunned: global triggers, their rate the trigger's cooldown cap.
- **Not ported:** a skill with another trigger support PoB marks `isTrigger`, or with the `Triggered` type, stays not computed, as before. A skill with only `InbuiltTrigger` and no config is self-cast in PoB, as in the engine.

**The handler** (`defaultTriggerHandler`, 390 to 880):
1. **The source:** each candidate is computed as its own skill, as PoB's `calcs.buildActiveSkill` does. The engine runs `calculate` on a copy of the build with the candidate as its main skill (`skill::with_main_skill`; the candidates are `skill::active_skills`).
   - The highest `HitSpeed`, else `Speed`, wins; the first wins a tie.
   - Never a source: a triggered skill, a disabled one, one another thing uses, or the main skill itself.
   - No source: PoB reports the skill as self-cast, and so does the engine.
2. **The cooldowns:** the trigger support's level cooldown and the triggered skill's, each rounded up to a server tick unless the skill stores more than one use. The cap is `1 / max(both)`.
3. **The effective source rate:** the source's rate; a global trigger's is the cap.
4. **The trigger chance:** the source's hit chance when not 100, times its crit chance for Cast On Critical Strike when not 100.
5. **The trigger rate:** `min(cap, rate)` for a skill with more than one stored use that is the only one triggered; the effective rate for a global trigger; else PoB's rotation simulation (`calcMultiSpellRotationImpact`, 87 to 133, `triggers::rotation_rate`).
6. **The source's reasons:** the triggered skill's speed carries the source's speed, hit chance, and crit chance reasons. A source field not computed leaves the speed unsupported (`trigger-source`).

**Rows and tables:**
- `support_cast_on_crit_spell_damage_+%_final`: `Damage` MORE with `ModFlag.Spell` (`sup_dex.lua` 597).
- `cast_on_damage_taken_damage_+%_final`: `Damage` MORE (`sup_str.lua` 1158).
- `NoEffect` (read by the trigger, or no PoB row): `cast_spell_on_linked_attack_crit`, `cast_linked_spells_on_attack_crit_%`, `cast_on_damage_taken_threshold`, `cast_on_damage_taken_%`, `spell_uncastable_if_triggerable`, `local_support_gem_max_skill_level_requirement_to_support`, `triggered_skill_uses_main_hand_or_averaged_attack_time_for_pvp_scaling`.
- **`base_skill_show_average_damage_instead_of_dps` is a field marker** (`mapping::HIT_REASONS`). It was a reason on every field; it is now a reason on the hit's speed, only when the rate is not a trigger's. PoB turns `showAverage` off for a triggered skill (`CalcOffence.lua` 2227).
- **Gem tables (`gems::SKILLS`, `SUPPORTS`):**
  - Smite's aura as another skill (`LightningMin` and `LightningMax` without flags; its shock chance `NoSlice`). As the main skill it acts through the hit, as before.
  - Arc as an inert skill (PoB's export: no global effect).
  - The trigger supports with no global stat.
- **`data-parity`, table `gem_effects`:** 1,519 values compared, 0 differences. The tool now allows:
  - the trigger supports' `Triggered` type and their `SkillData` keys;
  - Smite's negated `AffectedByVaalSmite` condition (Vaal Smite has no entry, so a build with it is partial);
  - the shock chance's name.

**Not modelled:** cooldown recovery, added cooldown, and added cast time (their stats are unmapped, so reasons); the halved rate of dual-wielded item triggers; Unleash; projectiles that hit several times; trigger-related damage (`TriggeredDamage`).

**The Holy Relic's Nova** (1167) waits for data. PoB picks a minion skill's level as the highest whose level requirement the minion's level reaches (`CalcActiveSkill.lua` 895 to 902), and the bundle does not carry skills' level requirements yet. The fixture `local/p6/fx-trig/trigger-holy-relic-smite-templar.xml` and its oracle output are kept locally for that step.

### Evidence

- **Fixtures** (new, all original), PoB's outputs in `local/p6/fx-trig-oracle`:
  - `trigger-coc-ice-nova-templar.xml`: Smite, Cast On Critical Strike, and Ice Nova. Every value but the rate equals PoB's, the 838.877 average damage with Smite's aura included.
    - The rate is 0.0055511 against PoB's 0.0055539. It is PoB's simulation fed Smite's attack rate, 1.2743628 against PoB's 1.275: the registered Q11 difference.
    - With PoB's 1.275 the same simulation (`local` script `coc_sim.py`) gives PoB's value to the bit, for one spell and for two.
    - Registered: DIV-0175 to DIV-0177, `game-truth`, for `Speed`, `TotalDPS`, and `CombinedDPS`.
  - `trigger-coc-two-spells-templar.xml`: Ice Nova and Arc share the triggers. The same, DIV-0178 to DIV-0180.
  - `trigger-coc-no-attack-templar.xml`: no attack, so self-cast. Every value equals PoB's.
  - `trigger-cwdt-ice-nova-templar.xml`: Cast when Damage Taken. The rate (3.788, its 0.25 s rounded to a tick), its less damage, and the DPS equal PoB's.
- **Public `calc-parity` on `b-ot`:** 200 builds, 179 expected, 0 unexpected; `Complete` fields 5,349 (5,225 before).
  - Elemental Hit of the Spectrum's damage moved toward PoB (the Cast when Damage Taken and Cast On Critical Strike gems are now known, so their groups' buffs count).
  - Three other documents changed only in their reasons.
- **Corpus:** 0 unexpected. One build's armour moved toward PoB (21,689 to 23,938 of 30,838).
- **Minion sweep:** unchanged.
- **Skill sweep** (376 builds, not registered): 32 unexpected, 31 before.
  - The new one is Storm Call (alternate Y)'s mana cost. PoB 2.67.2 has no such skill, so it writes 0. It is one of 14 skills the game data has and PoB does not, whose costs the engine reads from the game.
  - Their costs were hidden behind `showAverage`'s reason on every field until now.
  - The others are those same skills' costs, Smite's Q11 attack rate, and Volcanic Fissure (alternate X), as before.
- **Tests:**
  - `a_lone_skill_without_cooldown_should_follow_the_source`;
  - `the_rotation_should_share_and_cap_the_triggers`;
  - the damage-taken guard lists Cast when Damage Taken's rows;
  - the workspace's 1,763 tests and clippy pass.

## Part 2: Cast while Channelling, Cast on Melee Kill, Cast on Ward Break

**Configs:**
- **Cast while Channelling** (`CWCHandler`, `CalcTriggers.lua` 219 to 388):
  - The source is the group's first channelling skill, not triggered, that the support's first part supports.
  - Its interval is that part's `cast_while_channelling_time_ms` (PoB's `triggerTime`).
  - The rate is the rotation of the group's spells it triggers at `1 / interval`, with no trigger cooldown and a chance of 100.
- **Cast on Melee Kill** (1155): as Cast On Critical Strike from the group's melee attacks, without the crit chance.
  - It applies only while the character has killed recently, which is PoB's `Condition:KilledRecently` from the config check or a kill count above 0. Otherwise PoB reports self-cast.
  - A kill count of another type leaves the speed unsupported.
- **Cast on Ward Break** (1195): a global trigger, as Cast when Stunned.
- **Not ported:** Cast on Death. PoB's config returns none, but the support's `triggeredOnDeath` changes the skill's time and damage (`CalcOffence.lua` 2403, 4909), so a skill it supports stays not computed.

**Triggered damage:** PoB turns a skill's `TriggeredDamage` into `Damage` when the trigger's source is the skill itself (`addTriggerIncMoreMods`, its `SkillType.Triggered` tag read in the source's config). That is the case for Cast while Channelling and the global triggers.
- `triggers::Outcome::Rate` carries that flag.
- The spell path settles the state `triggers::TRIGGERED_DAMAGE` before the damage.
- Cast while Channelling's `support_cast_while_channelling_triggered_skill_damage_+%_final` is a `STATE_ROWS` row on it.

**Cyclone:**
- Its movement speed buff (`MovementSpeed` MORE with `Condition:ChannellingCyclone`, `unscalable`) is a gem table entry. The config check `channellingCycloneCheck` is now a settled condition (`conditions::CHANNELLING_CYCLONE`, out of `NAMED_ONLY`).
- An `unscalable` buff takes scale 1 (`gems::UNSCALABLE`).
- The `gem_effects` tool renders its `unscalable` tag.

**Rows:**
- `Damage` MORE with `ModFlag.Spell`: `support_cast_on_melee_kill_spell_damage_+%_final` and `cast_on_ward_break_damage_+%_final`.
- `NoEffect`: `cast_spell_on_linked_melee_kill`, `cast_spell_while_linked_skill_channelling`, `cast_while_channelling_time_ms`, `trigger_on_ward_break_%_chance`, and `cast_linked_spells_on_melee_kill_%` (PoB's `chanceToTriggerOnMeleeKill`, which no module reads).
- `cooldown_recovery_rate_+%_per_100_ward` stays unmapped: the engine does not compute ward.

**`data-parity`, table `gem_effects`:** 1,569 values compared, 0 differences.

### Evidence

- **Fixtures** (new, all original; PoB's outputs in `local/p6/fx-trig2-oracle` and `fx-trig2b-oracle`):
  - `trigger-cwc-ice-nova-templar.xml`: Cyclone, Cast while Channelling, Ice Nova. Every value equals PoB's (the rate 3.128, the 20% less damage).
  - `trigger-cwc-channelling-cyclone-templar.xml`: the same with the check on. Cyclone's movement speed equals PoB's (0.8).
  - `trigger-comk-ice-nova-templar.xml`: Smite, Cast on Melee Kill, Ice Nova, killed recently. The damage equals PoB's.
    - The rate is PoB's simulation fed Smite's Q11 attack rate: with PoB's 1.275 it gives PoB's 0.28056166190371506 to the bit.
    - Registered: DIV-0181 to DIV-0184, `game-truth` (`Speed`, `TotalDPS`, `CombinedDPS`, and the `TotalDPS` breakdown's printed total).
  - `trigger-comk-no-kill-templar.xml`: no recent kill, so self-cast. Every value equals PoB's.
- **Local only** (no `Complete` field to target):
  - Cast on Ward Break: rate, damage, and DPS equal PoB's, `partial` for the ward stat.
  - Cast on Death: not computed.
- **Public `calc-parity` on `b-ot`:** 204 builds, 183 expected, 0 unexpected; `Complete` fields 5,473 (5,349 before).
  - Three test builds moved, all `indeterminate` or `partial`.
  - `Generals Perforate Zerker.xml`: Cyclone's buff now applies, as the config check is on, so movement went 1.87 to 1.496 against PoB's 1.832. PoB's value has other unmodelled content. The fixture above shows the buff equals PoB's.
  - `Dual Wield Cospris CoC.xml`: its Cast On Critical Strike spell is now computed.
- **Corpus and the minion sweep:** unchanged; 0 unexpected.
- **Skill sweep:** unchanged.
- **Tests:** the state row's expected list; the workspace's 1,763 tests and clippy pass.

## Part 3: Spellslinger, and cooldown recovery in every trigger (open items S3)

Built on PR #187 (2026-11-26), the first S3 step toward version 1.0 (`docs/plan/ACTION-PLAN.md`, "Version 1.0").

**Spellslinger** (`CalcTriggers.lua` 1200 to 1212), `triggers::Config::Spellslinger`:
- **The source** is any projectile attack with the wand flag (the main hand holds a wand), in any group, not only the spell's. It is not triggered, not disabled, and not the main skill.
- **On use** (`triggerOnUse`): the source's hit chance is not a factor.
- **The cooldown** is the support's level cooldown (0.6 s).
- **The rotation** is the main spell alone: PoB's config has no `triggeredSkillCond`, so each supported spell is simulated on its own.

**Cooldown recovery in every trigger.** PoB divides both cooldowns by the triggered skill's `calcLib.mod(CooldownRecovery)`, in the cap (575 to 576) and in the rotation (`packageSkillDataForSimulation`, 64 and 69). The engine did not; P6-8a listed it as not modelled.
- `triggers::rate` now reads the factor from the store, `(1 + inc / 100) x more`, over the character's untagged and typed terms and the skill's own. These are the quantities a self-cast spell's cooldown reads (`hit::cooldown_limit`).
- **The main skill's own `base_cooldown_speed_+%`** (its gem's or a support's, Spellslinger's 38%) was unmapped on the main skill. `SkillMapping` now maps it to `TypedCooldownRecovery` without a type: only the skill's cooldown and its trigger read it, and no other skill's totals do.

**The damage:**
- **Wand damage:** `hit::wand_damage_to_spells` adds the share of the wand's base damage (`gain_%_of_base_wand_damage_as_added_spell_damage`, 174% at level 20). It reads each type's weapon data after local modifiers, takes the mean of two wands, floors, and adds the result as added damage BASE with `ModFlag.Spell` (`CalcOffence.lua` 879 to 897).
- `support_spellslinger_damage_+%_final`: `Damage` MORE with `KeywordFlag.Hit` or `Ailment` (`HIT_ROWS`).
- **`showAverage` from base mods:** PoB sets it from a gem's hand-written base mods (Spellslinger's support has one) and clears it when a trigger sets the rate (2227). `hit::spell_hit` settles `hit::SHOW_AVERAGE` for a self-cast spell whose active gem or supports set it, and `combined::combine` then takes the average damage as its base, as it does for traps and mines. A trap's or mine's rule is unchanged.

**Reservation:**
- The supported spell reserves the support's level share, as Blasphemy's does. This needed only the support's table entry (`gems::SUPPORTS`, `SupportSpellslinger`).
- "#% increased Mana Reservation Efficiency of Skills Supported by Spellslinger" (`spellslinger_mana_reservation_efficiency_+%` and its `-2%_per_1` form; `ModParser.lua` 1596) is `Poe1Quantity::SpellslingerManaReservationEfficiency`. It is added for skills the support supports (`Totals::reservation_for_skill`).

**Rows and tables:**
- `NoEffect` (read elsewhere): `gain_%_of_base_wand_damage_as_added_spell_damage`, `spellslinger_trigger_on_wand_attack_%`, `quality_display_wand_damage_as_added_spell_damage_is_gem`.
- Inert skills (no global effect in PoB's data): Fireball, Kinetic Blast, Power Siphon.
- **`data-parity`:** `gem_effects` allows Spellslinger's `SkillData` keys, its `Triggered` and `HasReservation` types, and its level reservation (`RESERVING_SUPPORTS`). The result is 753 differences, `uncategorized` 0, the same as `dev`.

### Evidence

Run on the main workstation, 2026-11-26, with the pinned oracle (PoB v2.67.2, `b32759a`, clean) and the evidence bundle `local/p6/packs/b-ct` (build 26530219).

- **Fixtures** (new, all original; each run three times by PoB, identical):
  - `trigger-spellslinger-ice-nova-witch.xml`: Ice Nova with Spellslinger, Kinetic Blast on a Carved Wand in another group. Every compared field equals PoB's: the rate (the source's 1.84 under the cap of 2.1645), the average damage with the wand's share, the less damage, and the reserved mana (472 unreserved).
  - `trigger-spellslinger-two-spells-witch.xml`: Ice Nova and Fireball, each reserving (354 unreserved), both in full DPS. Every compared field equals PoB's.
  - `trigger-spellslinger-no-attack-witch.xml`: no wand attack, so self-cast. The combined DPS is the average damage (592.93), PoB's `showAverage`. Every compared field equals PoB's.
  - In all three, 231 fields are `Complete`, none partial; hit chance is not computed for a spell, as before.
- **The rotation with cooldown recovery** reproduces PoB's 1.197 for a 2.25 source rate, a 0.6 s cooldown, and 1.38 recovery. That is PoB's `SkillTriggerRate` on a first version of the fixture with Power Siphon as the source; Power Siphon's own stats are unmapped, so its fixture would carry their reasons, and Kinetic Blast replaced it. The test is `the_cooldown_recovery_should_divide_the_cooldowns_in_the_rotation`.
- **Public `calc-parity`** (539 builds, the calc scope): 0 unexpected, as before. `Complete` fields went from 39,309 to 39,540, the new fixtures' 231.
  - 8 fields of `kinetic-blast-of-clustering-necromancer.xml` moved, all now equal to PoB's: block and spell block chance, unreserved mana (2,923 to 267), mana regeneration, and the EHP fields. Spellslinger's gem and its reservation efficiency now count.
  - Nothing moved away from PoB.
- **Corpus** (21 builds, local): 2 builds changed, 3 fields moved, 4 reasons gone (3 `unmodelled-gem`, 1 `unmapped-stat`). There is no oracle run of the corpus here.
- **Tests:** the workspace's 1,859 tests, clippy, fmt, and the wasm32 check pass.

**Left for S3:** Mark on Hit, Hextouch, Arcanist Brand, Cast on Death, Mirage Archer, Manaforged Arrows, the item triggers (`trigger_socketed_spell_on_skill_use_%`, the game-truth list), and `spellslinger_cooldown_duration_+%` (no build has it).

## Part 4: Arcanist Brand and Cast on Death (open items S3)

Built on branch `feat/s3-triggers` (2026-11-26), after PR #187.

**Arcanist Brand** (`CalcTriggers.lua` 1321 to 1341), `triggers::Config::ArcanistBrand`, `triggers::arcanist_rate`:
- **The source** is the first Arcanist Brand among the active skills, in any group (PoB's `activeSkillList` loop). The triggered skills are the group's spells its support part supports.
- **The brand's own context.** PoB sums the brand's modifiers in the brand's skill list. The engine collects the build with the brand as the main skill (`engine::collected`) and the new option `CalcOptions::own_skill_context`, so the brand's gems and supports act on it although the engine computes no hit for it. Its flags are `ModFlag.Cast` and `Spell`, the spell keyword, and `SkillType.Brand`.
- **The activation rate** is `repeatFrequency x (1 + inc / 100) x more x attached`:
  - `repeatFrequency` is the brand's `base_sigil_repeat_frequency_ms` over 1000. It is a period, and PoB multiplies it as it is; for the gem it is 1 s, so the value is the same as a frequency.
  - `inc` is the `Speed` and `BrandActivationFrequency` INC sums: cast speed reaches the brand (`additive_cast_speed_modifiers_apply_to_sigil_repeat_frequency`), and so does its quality, `sigil_repeat_frequency_+%` (the new `Poe1Quantity::BrandActivationFrequency`).
  - `more` is the `BrandActivationFrequency` MORE product.
  - `attached` is PoB's `attachedBrandCount` (`CalcPerform.lua` 1322 to 1326): the brand's attach limit (its gems' `base_number_of_sigils_allowed_per_target` and the character's) at most the active brand limit (3 and the additions), or the config's `BrandsAttachedToEnemy` at most that.
- **The rotation.** The config has `triggeredSkillCond`, so PoB always simulates the rotation, with no trigger cooldown and a chance of 100 (809). Its charge branch (792) is never taken for such a config: it compares two tables by identity. The cap is not read.
- **The spell:**
  - PoB turns its `TriggeredDamage` into `Damage`: `support_brand_damage_+%_final` (`TRIGGERED_DAMAGE`), and `trigger_brand_support_hit_damage_+%_final_vs_branded_enemy` with the config's `targetBrandedEnemy`, on unless set off (the new state `TRIGGERED_DAMAGE_BRANDED`).
  - The support adds the `brand` skill flag, so PoB's `KeywordFlag.Brand` reaches the spell ("Brand Damage"). The engine's `Flags::SKILL_BRAND` stands for both the keyword and the `SkillType.Brand` tag. The one tag row, cast speed with brand skills, reaches only the cast rate, which a triggered spell does not read.
- **Rows:** `NoEffect` for the brand's markers (`triggered_by_brand_support`, `base_sigil_repeat_frequency_ms`, `additive_cast_speed_modifiers_apply_to_sigil_repeat_frequency`, `base_number_of_sigils_allowed_per_target`, `base_deal_no_damage`), the support's less area and projectile range. Arcanist Brand is an inert skill (`BrandSupport`) and its support part a support without a global stat.

**Cast on Death** (`CalcTriggers.lua` 1342, `CalcPerform.lua` 1438 to 1447), `triggers::Config::CastOnDeath`:
- PoB's config returns none. The support's `triggeredOnDeath` sets the skill's `triggerTime` to `60 x 1000` ("technically any large value would do"), which `CalcOffence.lua` 2214 to 2221 reads as seconds over the skill's `CooldownRecovery` INC. The engine ports that stand-in: the rate is `(1 + inc / 100) / 60000`, without action speed or the tick cap.
- `showAverage` stays: the support's `base_skill_show_average_damage_instead_of_dps`, not cleared as a trigger rate's is (2227). The combined DPS is the average damage.
- `cast_on_death_damage_+%_final_while_dead` is `Damage` MORE (PoB applies it unconditionally). The other stats are markers or no slice field (`cast_on_death_%`, `spell_only_castable_on_death`, `no_cost`, the area while dead); the cost is 0 from the level's mana multiplier.
- **Not modelled:** the damaging ailments. PoB counts an ignite's stacks without the rate (4909) and a poison or a bleed by its average (4767, 6032). Each such field of a Cast on Death spell is not computed (`cast-on-death-ailment`).

**Mark On Hit and Hextouch** (1275, 1278) trigger a mark or a hex. Their effects apply as any curse's (P6-9g), and the engine computes no field from a curse's rate, so their configs give nothing to port.

**Mirage Archer** is not a trigger config in PoB (`CalcMirages.lua`). It is in since P6-8b (`docs/phase-6/mirages.md`).

**Full DPS with `showAverage`.** The full DPS inferred a skill's cull multiplier from its combined DPS over its parts. For a skill that shows its average damage, the combined DPS has no rate, so that ratio is not a cull. `full_dps` now skips the inference when the combined DPS is the average damage. PoB's full DPS takes such a skill's `TotalDPS` (`Calcs.lua` 287 to 289).

### Evidence

Run on the main workstation, 2026-11-26, with the pinned oracle (PoB v2.67.2, `b32759a`, clean) and the evidence bundle `local/p6/packs/b-ct` (build 26530219). The baseline is `dev` at `eb2e8c79`, built in a worktree and run the same way.

- **Fixtures** (new, all original; each run three times by PoB, identical). Every compared field equals PoB's:
  - `trigger-arcanist-ice-nova-witch.xml`: Ice Nova with Arcanist Brand. The rate 1, the average damage 429.27 (the support's less damage and its more damage against the branded enemy), the mana cost per second.
  - `trigger-arcanist-two-spells-witch.xml`: Ice Nova and Arc with Arcanist Brand and Faster Casting, the brand at 20% quality. The activations 1.49 per second (the quality's 10% and Faster Casting's cast speed in the brand's context), Ice Nova's share 0.74649 from PoB's rotation. The full DPS is partial for Arc's own reasons.
  - `trigger-cast-on-death-ice-nova-witch.xml`: Ice Nova with Cast on Death. The speed 1/60,000, the average damage 1,833.26 as the combined DPS, the mana cost 0, the full DPS 0.0306 (the hit's DPS).
- **Public `calc-parity`** (539 builds, the calc scope): 0 unexpected, as on the baseline. `Complete` fields went from 39,540 to 39,769, the new fixtures' 229.
  - 5 fields of `kinetic-fusillade-ballista-hierophant.xml` moved toward PoB (average damage, hit, ignite, combined, and full DPS). Its Arcanist Brand is now a known gem, no longer `unmodelled-gem`.
  - Nothing moved away from PoB.
- **A sorting bug caught on the way:** an out-of-order support entry hid Cast On Critical Strike from the binary search and moved the Cast On Critical Strike fixtures away from PoB. The new test `the_gem_tables_should_be_sorted_by_id` guards both gem tables.
- **Corpus** (21 builds, local): 1 build changed, a Detonate Dead build with an Arcanist Brand in another group. Its `unmodelled-gem` reason is gone (76 fields). Five of its damage values moved (hit DPS 27,845 to 46,592) and its own reasons now show, all fields `indeterminate`. There is no oracle run of the corpus here.
- **`data-parity`:** 753 differences, `uncategorized` 0, the same as `dev`. `gem_effects` allows Arcanist Brand's `Brand` type, its support's `Triggered` type and `triggeredByBrand`, and Cast on Death's `Triggered` type and marks.
- **Tests:** the workspace's 1,860 tests, clippy, and fmt pass.

**Left for S3:** Manaforged Arrows (a triggered bow attack: the attack path takes no triggered attack yet, and the rate needs both skills' raw mana costs), the item triggers (`trigger_socketed_spell_on_skill_use_%`, the game-truth list), and `spellslinger_cooldown_duration_+%` (no build has it).
