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

Built on branch `feat/s3-triggers`, PR #188 (2026-11-26), after PR #187.

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

**Left for S3:** Manaforged Arrows (a triggered bow attack: the attack path takes no triggered attack yet, and the rate needs both skills' raw mana costs), the item triggers (`trigger_socketed_spell_on_skill_use_%`, the game-truth list), and `spellslinger_cooldown_duration_+%` (no build has it). The item lines that trigger a socketed spell are part 5.

## Part 5: the item lines that trigger a socketed spell (open items S3)

Built on branch `feat/s3-triggers`, PR #188 (2026-11-26), after part 4.

**What PoB does.** An item line such as Mjolner's "Trigger a Socketed Lightning Spell on Hit, with a 0.25 second Cooldown" is an `ExtraSupport` of a trigger support at level 1, for the item's slot (`ModParser.lua` 3148 to 3176 and 5276 to 5286). The support joins the socketed spell's group, makes it `Triggered`, and its name is the config (`calcs.triggers`, 1592 to 1617, `triggerNameLower`).

**The lines** (`item_supports::TRIGGER_ROWS`, served by `item_supports::row` at level 1 like the "Supported by" rows):

| Stat | Support | Config (`CalcTriggers.lua`) |
|---|---|---|
| `trigger_socketed_spell_on_attack_%` | `SupportTriggerSpellOnAttack` | Poet's Pen (1009): a wand attack, on use; the spells of the item's slot |
| `chance_to_trigger_socketed_spell_on_bow_attack_%` | `SupportTriggerSpellOnBowAttack` | Asenath's Chant (1031): a bow attack, on use; the spells of the item's slot |
| `local_unique_attacks_cast_socketed_lightning_spells_%` | `SupportUniqueMjolnerLightningSpellsCastOnHit` | Mjolner (1109): an attack with a mace or a one-handed weapon in another group; the group's lightning spells |
| `local_unique_cast_socketed_cold_skills_on_melee_critical_strike` | `SupportUniqueCosprisMaliceColdSpellsCastOnMeleeCriticalStrike` | Cospri's Malice (1125): a melee attack with a sword or a one-handed weapon, on crit; the cold spells of the item's slot |
| `main_hand_trigger_socketed_spell_on_freezing_hit` | `SupportTriggerSpellOnBowAttackFreezeHit` | Wing of the Wyvern (1117): a bow attack in another group; the group's spells |
| `body_armour_trigger_socketed_spell_on_unarmed_melee_critical_hit_cooldown` | `SupportTriggerSpellOnUnarmedMeleeCriticalHit` | Seven Teachings (1131): an unarmed melee attack, on crit; the group's spells |
| `local_shield_trigger_socketed_elemental_spell_on_block_display_cooldown` | `SupportTriggerElementalSpellOnBlock` | Svalinn (1498): a global trigger; the group's spells |
| `local_unique_cast_socketed_spell_on_block` | `SupportUniqueVirulenceSpellsCastOnBlock` | Festering Resentment (1505): the same |
| `local_weapon_trigger_socketed_fire_spell_on_hit_display_cooldown` | `SupportTriggerFireSpellOnHit` | the enchantment (1521): a melee attack; the group's spells |

- **A chance below 100.** The three `%` stats carry the chance to trigger. PoB ignores it (its `ExtraSupport` has no chance, and its patterns for "#% chance to" forms do not all match the game's text), so a line below 100% leaves the rate unsupported (`item-trigger-chance`).
- **The other stats are flags.** Their value is a cooldown shown in seconds, which reads back as a range (`unsettled-value`); PoB does not read it, and neither does the engine (`gem_levels`: the row's `ItemSupport` is a flag).
- **Left out:** "Trigger Socketed Spell on Kill" (Squirming Terror: at 100% the game's text has no "% chance" prefix, which PoB's pattern 3160 needs, so PoB applies nothing); Focus (`helmetFocusHandler`); the lines that trigger a bow skill or a curse; Seven Teachings is ported but finds no source, as the engine computes no unarmed attack.

**The handler**, `triggers::Config` with the eight new configs:
- **The source** is any active attack, not triggered and not the main skill, that meets the config's rule. Its weapon flags are the main hand's (PoB's `skillCfg.flags`, `CalcActiveSkill.lua` 293 to 309 and 455, from `hit::weapon_type_flags`; an empty hand is `Unarmed` without `Weapon1H`). The highest rate wins, as for the other configs.
- **On use** (Poet's Pen, Asenath's Chant): the source's hit chance is not a factor. **On crit** (Cospri's Malice, Seven Teachings): its crit chance is.
- **The trigger cooldown** is the support's level cooldown from the bundle (0.25 s; Asenath's Chant 0.3 s).
- **Dual wielding** (441 to 449): an item's trigger from an attack halves the source rate, unless the attack hits with both weapons (`hit::double_hits`, lifted out of the attack path). A hand that is not known leaves the rate unsupported.
- **The rotation** is the triggered spells PoB's `triggeredSkillCond` picks: of the main skill's group (`slotMatch`), or of its item's slot (Poet's Pen, Asenath's Chant, Cospri's Malice).
- **Cast on block** is a global trigger with `triggeredSkillCond`: the effective rate is the cap, and PoB simulates the rotation at it (809), with PoB's `TriggeredDamage` as `Damage`.

**Rows and tables:**
- `NoEffect`: the supports' markers `skill_has_trigger_from_unique_item`, `socketed_triggered_skills_use_weapon_attack_time_for_pvp_scaling`, `unique_cospris_malice_cold_spells_triggered`, `unique_mjolner_lightning_spells_triggered`, `trigger_effect_group_on_block`, `spell_is_triggered_on_hit`.
- `gems::SUPPORTS`: the nine supports, no global stat. `gems::SKILLS`: Spark as an inert skill (PoB's export: no global effect).
- **`data-parity`, table `gem_effects`:** allows the supports' `Triggered` type and their `SkillData` marks. 753 differences, `uncategorized` 0, the same as `dev`.

### Evidence

Run on the main workstation, 2026-11-26, with the pinned oracle (PoB v2.67.2, `b32759a`, clean) and the evidence bundle `local/p6/packs/b-ct` (build 26530219). The baseline is part 4's run.

- **Fixtures** (new, all original; each run three times by PoB, identical; the bases have whole-millisecond attack times, so Q11 does not enter). Every compared field equals PoB's:
  - `trigger-poets-pen-ice-nova-witch.xml`: the rate 1.84, Kinetic Blast's use rate under the 0.25 s cooldown.
  - `trigger-poets-pen-dual-wield-witch.xml`: a second wand in the off hand, the rate 1.012, half the source rate (Kinetic Blast's 1.84 with dual wielding's 10% more attack speed, 2.024).
  - `trigger-asenaths-chant-ice-nova-ranger.xml`: Ice Nova in the helmet, Burning Arrow with a bow, the rate 0.8759 from 0.875 under the 0.3 s cooldown.
  - `trigger-wing-of-the-wyvern-ice-nova-ranger.xml`: the line PoB joins from two, the rate 0.2188 (the hit chance applies).
  - `trigger-mjolner-spark-templar.xml`: Spark and Smite, the rate 0.23375, Spark's cost 0 from the support's cost multiplier. Spark's own fields stay partial for its PoB data (`pob-skill-data`), so 69 fields are targets; no lightning spell in the gem tables is free of its own reasons.
  - `trigger-cospris-ice-nova-templar.xml`: Smite's hit and crit chance, the rate 0.00362.
  - `trigger-svalinn-ice-nova-templar.xml` and `trigger-festering-resentment-ice-nova-witch.xml`: the rate 3.7803, PoB's rotation at the 3.7879 cap.
  - `trigger-fire-spell-on-hit-fireball-templar.xml`: Fireball and Smite, the rate 0.23375, the ignite too.
  - 686 fields are targets in all.
- **Public `calc-parity`** (551 builds, the calc scope): 0 unexpected. `Complete` fields went from 39,769 to 40,455, the new fixtures' 686. No other value moved. One document changed in its reasons only: `poisonous-concoction-of-bouncing-pathfinder.xml` lost its Svalinn line's `unmapped-stat`.
- **The evidence oracle** now holds the three Spellslinger fixtures' outputs (run again here), which part 3's run had added only on CI, so the local `calc-parity` no longer reports them missing.
- **Corpus** (21 builds, local): no document changed.
- **Tests:** `trigger_rows_should_be_sorted_at_level_one`; the gem tables' sort test covers the new entries; the workspace's 1,861 tests, clippy, fmt, and the wasm32 check pass.

**Left for S3:** Manaforged Arrows and the skills items grant and trigger (Molten Burst, Starfall, Bone Nova, and kin), all attacks: the attack path takes no trigger rate yet. Then Focus, Squirming Terror (the game-truth list, with `trigger_socketed_spell_on_skill_use_%`), and `spellslinger_cooldown_duration_+%` (no build has it). Part 6 gives the attack path a trigger rate and ports Manaforged Arrows.

## Part 6: a trigger rate in the attack path, and Manaforged Arrows (open items S3)

Built on branch `feat/s3-triggers`, PR #188 (2026-11-26), after part 5.

**The attack path** (`hit::attack_skill`, `hit::attack_hit`): a triggered attack is computed when the engine ports its trigger (`triggers::computable`), as a triggered spell is. PoB gives every pass the trigger rate as its speed (`CalcOffence.lua` 2223 to 2227), so the engine replaces each hand's speed with the rate, carrying the trigger's reasons; a dual wielder's combined speed is then the rate too. A rate that cannot be computed leaves the attack's fields not computed. The triggered damage state is settled from the trigger, as in the spell path.

**The costs before the hit.** PoB computes a skill's costs before the passes' damage (1677 to 1800). The engine's cost phase ran after the hit, since the costs per second read its speed. It is now two phases: `costs` before `hit`, and `cost-per-second` after `dot` (`costs::per_second_phase`). Only the graph's node numbers change: on the public builds and the corpus, no document changed in a value, a state, or a reason (`local/p6/doc_cmp.py`, which ignores node numbers).

**Manaforged Arrows** (`CalcTriggers.lua` 1378, the handler's 522 to 556), `triggers::Config::ManaforgedArrows`:
- **The source** is any bow attack (the main hand's weapon flags), not triggered and not the main skill, on use (no hit chance).
- **The rate:** the source's rate over `ceil(threshold x triggered / source)`, the threshold the support's `support_manaforged_arrows_mana_cost_%_threshold` over 100 (3), each cost PoB's `ManaCostRaw`, the cost before its roundings (`costs::mana_cost_raw`, read from the store; the source's in its own collected context, as Arcanist Brand's). A triggered attack without a cost keeps the source's rate; a source without a cost gives a rate of 0, as PoB's.
- **The rotation:** PoB's config has no `triggeredSkillCond`, so the main skill alone, as Spellslinger's, with the support's 0.5 s cooldown.
- **The damage:** `support_manaforged_arrows_damage_+%_final` is `Damage` MORE. `support_manaforged_arrows_damage_+%_final_per_mana_spent` (PoB's `Damage` MORE with `PerStat` `ManaCost`) is `Poe1Quantity::DamageMorePerManaCost`: the attack path adds a `Damage` MORE effect of its value times `floor(cost + 0.0001)`, from the cost field, before the damage (`Calc::add_effect`, PoB's mid-calculation mod).
- **`NoEffect`:** the threshold (read by the trigger) and `triggered_by_manaforged_arrows_support_%_chance` (100, no PoB stat map entry).

### Evidence

Run on the main workstation, 2026-11-26, with the pinned oracle (PoB v2.67.2, `b32759a`, clean) and the evidence bundle `local/p6/packs/b-ct` (build 26530219). The baseline is part 5's run (`local/p6/s3c`).

- **Fixtures** (new, all original; a Recurve Bow, whose attack time is a whole number of milliseconds; each run three times by PoB, identical). Every compared field equals PoB's:
  - `trigger-manaforged-burning-arrow-ranger.xml`: Burning Arrow with Manaforged Arrows, a second Burning Arrow the source. The triggered attack's cost is 28 (the support's multiplier), the source's 14: `ceil(3 x 28 / 14) = 6` uses, the rate 0.14583 from the source's 0.875; the damage with the support's less damage and its 28% more per mana spent.
  - `trigger-manaforged-low-level-source-ranger.xml`: a level 1 source with its lower cost, the rate 0.07955.
  - 158 fields are targets.
- **Public `calc-parity`** (553 builds, the calc scope): 0 unexpected. `Complete` fields went from 40,455 to 40,613, the new fixtures' 158. No other document changed apart from node numbers.
- **Corpus** (21 builds, local): no document changed apart from node numbers.
- **`data-parity`:** 753 differences, `uncategorized` 0.
- **Tests:** the phase order test (`costs` before `hit`, `cost-per-second` after `dot`); the workspace's 1,861 tests, clippy, fmt, and the wasm32 check pass.

**Left for S3:** the skills items grant and trigger (Molten Burst, Starfall, Bone Nova, Icicle Burst, and kin): attacks with their own cooldowns, found by the unique's name in their saved group's source, each with its weapon data from the item; Focus; Squirming Terror and `trigger_socketed_spell_on_skill_use_%` (the game-truth list); `spellslinger_cooldown_duration_+%` (no build has it). The skills items grant and trigger are part 7.

## Part 7: the skills items grant and trigger (open items S3)

Built on branch `feat/s3-triggers`, PR #188 (2026-11-26), after part 6.

**What PoB does.** A line such as Ngamahu's Flame's "20% chance to Trigger Level 16 Molten Burst on Melee Hit" is an `ExtraSkill` with `triggered` and the line's chance (`ModParser.lua` 2054 to 2071, 3278 to 3335). PoB builds a socket group for the skill and saves it in the build with its `source`, `Item:<id>:<Unique>, <base>`; the engine reads such groups since P6-11fe. The config is PoB's by the skill's ID (`TriggeredMoltenStrike`, 1566), else by the unique's name in the source (`getUniqueItemTriggerName`, 1575 to 1590).

**The lines** (`triggers::GRANTED_TRIGGERS`, nine stats): each maps to `GrantsSkill` (the group must be there, as for the other granted skills) and to `Poe1Quantity::GrantedTriggerChance`, `Base` with the chance for the four `%` stats, else a flag (100%). A skill an item grants is triggered only with its item's line; without one it is used by hand, as in PoB.

**The configs** (`triggers::Config`):

| Config | Uniques (`CalcTriggers.lua`) | Source |
|---|---|---|
| `UniqueMeleeOrAttack` | Ngamahu's Flame (973) and Molten Burst by ID (1566), Limbsplit (947), The Cauteriser (950) | a melee or attack skill |
| `Starcaller` | Starcaller (979) | the same, on crit (its line's `onCrit`) |
| `MoonbendersWing` | Moonbender's Wing (968) | the same, PoB's cooldown of 1 s |
| `UniqueDamageOrAttack` | Cameria's Avarice (976), Uul-Netol's Embrace (982), Duskblight (953), Mark of the Elder and of the Shaper (1003, 1006), Sporeguard (1000) | a damaging or attack skill |
| `LioneyesPaws` | Lioneye's Paws (956) | a bow attack, on use, PoB's cooldown of 1 s |

- No trigger support, so no trigger cooldown: the skill's own cooldown caps it, the main skill alone in the rotation, the line's chance with the source's hit chance (an attack's only, 732) and crit chance (Starcaller).
- **The attack path** takes a triggered attack with its own cooldown when its trigger is ported (part 6 took the rate; the cooldown gate now lets it through).
- **No cost:** PoB clears the cost of a skill an item grants and triggers (`SkillsTab.lua` 1161 to 1165); the game gives it none. `triggers::granted_and_triggered` zeroes the costs and the raw cost.
- **Not listed:** the configs whose skill no slice field reads: the minions (Law of the Wilds, Rigwald's Crest, Jorrhast's Blacksteel, Ashcaller, Arakaali's Fang, Mark of the Shaper's anomaly), Shade Form (Replica Eternity Shroud, Shroud of the Lightless), Contaminate. Mark of the Shaper and Sporeguard are in the table, as PoB's config is the same as their kin's.

**Rows and tables:**
- `NoEffect`: the granted skills' trigger markers (`attack_trigger_on_*`, `trigger_on_hit_vs_frozen_enemy_%`, `attack_unusable_if_triggerable`), Icicle Burst's `never_freeze` (`freeze`), Rain of Arrows' `base_number_of_arrows` (`projectile-count`, as `number_of_additional_arrows`), `projectiles_nova` and `skill_art_variation` (no PoB entry).
- `gems::SKILLS`: the eight granted skills as inert skills, so a source's own calculation does not count them as unknown gems.
- **`data-parity`:** `gem_effects` allows their `Triggered` type and Bone Nova's and Icicle Burst's `triggered` mark: 753 differences, `uncategorized` 0.

### Evidence

Run on the main workstation, 2026-11-26, with the pinned oracle (PoB v2.67.2, `b32759a`, clean) and the evidence bundle `local/p6/packs/b-ct` (build 26530219). The baseline is part 6's run (`local/p6/s3d`).

- **Fixtures** (new, all original, each run three times by PoB, identical; made by `local/p6/s3e_fixtures.py`; each item carries only its line). Every average damage equals PoB's, and the trigger rates:
  - equal PoB's: Ngamahu's Flame (0.04675, the 20% chance), Starcaller (0.00257, on crit), Mark of the Elder (0.04298), Lioneye's Paws (0.87588, on use);
  - Cameria's Avarice and Uul-Netol's Embrace (Gavel, Vaal Axe): PoB 0.21505, the engine 0.21499; Limbsplit (Woodsplitter): 0.19926 against 0.19947; Moonbender's Wing (Tomahawk): 0.23008 against 0.22997. Each is Q11: the same builds with Smite as the main skill have the same hit chance (22) on both sides, and Smite's attack rate differs only by Q11's floor (1.105 against 1.10533, 1.275 against 1.27436). PoB's rotation fed each side's rate (`local/p6/rotation_sim.py`, and the test `the_rotation_should_give_pobs_rate_for_limbsplit`) gives each side's value to the bit. Limbsplit's and Moonbender's fields are `Complete`, so their differences are registered: DIV-0700 to DIV-0708, `game-truth`, Q11. Cameria's Avarice's and Uul-Netol's Embrace's speed stays partial for their skills' PoB data (`pob-skill-data`), as does Icicle Burst's, Bone Nova's, Molten Burst's, and Rain of Arrows' damage; their values equal PoB's. (Bone Nova's, Molten Burst's, and Rain of Arrows' are `Complete` since 2026-11-26, Bone Nova's rate registered as DIV-0761 to DIV-0765: `docs/phase-6/skill-data.md`, "The agent list".)
  - Moonbender's Wing's Lightning Warp costs 0, as PoB's.
  - 583 fields are targets.
- **Duskblight** is left out: the game's mod grants Stalking Pustule at level 1, but the bundle's granted effect has only a level 15 row, so the engine reports `no-level`. Which level the game uses is a question for the data (`dev/active/open-items/maintainer-follow-ups.md`).
- **Public `calc-parity`** (561 builds): 0 unexpected. `Complete` fields went from 40,613 to 41,196, the new fixtures' 583. No other public document changed.
- **Corpus** (21 builds, local): one document lost its `base_number_of_arrows` reason; no value or state changed.
- **Tests:** `granted_triggers_should_be_found_by_line_and_unique`, `the_rotation_should_give_pobs_rate_for_limbsplit`; the workspace's tests, clippy, fmt, and the wasm32 check pass.

## Part 8: Focus, Kitava's Thirst, Maloney's Mechanism, The Hidden Blade (open items S3)

Built on branch `feat/s3-triggers`, PR #188 (2026-11-26), after part 7.

**The crafted Focus line** ("Trigger Socketed Spells when you Focus, with a 0.25 second Cooldown", `trigger_socketed_spells_when_you_focus_%`, a chance row of `item_supports::TRIGGER_ROWS`; support `SupportTriggerSpellFromHelmet`). PoB's `helmetFocusHandler` (`CalcTriggers.lua` 135 to 217) assumes Focus is used as soon as its cooldown ends: the rate is `1 / (duration + cooldown / FocusCooldownRecovery)`, Focus's 4 s duration and 8 s cooldown from the bundle (`triggers::focus_period`), so 1/12 per second; the cap is shown, not applied; the triggered damage applies. No stat maps to `FocusCooldownRecovery`: "Focus has #% increased Cooldown Recovery Rate" stays a reason.

**Kitava's Thirst** (1081; `ModParser.lua` 3166). Its two-stat line gives the support `SupportCastOnManaSpent` (`cast_socketed_spells_on_X_mana_spent`, which also carries the mana to spend, `Poe1Quantity::KitavaManaSpent`) and the chance (`cast_socketed_spells_on_mana_spent_%_chance`, `KitavaTriggerChance`), which PoB applies. The source is the fastest spell or attack, not triggered and not the main skill, whose mana cost reaches the amount (PoB's comparer, 1085 to 1089); a repeat spends no mana, so PoB divides the rate by the source's repeats (475 to 481): a source with any repeat modifier is not modelled (`kitava-repeats`). The main skill alone in the rotation, the support's 0.1 s cooldown.
- **"+# to Total Mana Cost of Skills"** (`skill_mana_cost_+`, PoB's `ManaCost` BASE, `ModParser.lua` 190) is now mapped: it was unmapped, and the fixture needs it to reach Kitava's amount.

**Maloney's Mechanism** (1018; `chance_to_trigger_socketed_bow_skill_on_bow_attack_%`, a chance row; support `SupportTriggerBowSkillOnBowAttack`): a bow attack on use; the rotation the ranged attacks of the quiver's slot; a triggered attack, through part 6's attack path. Its replica (spells, `useCastRate`) is not ported.

**The Hidden Blade** (924; `local_display_grants_level_x_hidden_blade` in `triggers::GRANTED_TRIGGERS`): only while phasing (PoB disables the skill otherwise; the engine leaves it unsupported, `hidden-blade-phasing`); a global trigger at PoB's cap of 2 per second, or its cooldown's when it has one. `buffPhasing` is now a config check setting `Condition:Phasing` (`conditions::PHASING`, `ConfigOptions.lua` 1025), so other "while Phasing" lines are settled too.

**A fix on the way:** a trigger on use, or from a spell, read the source's hit chance field before deciding not to use it; a spell has none, so the rate was unsupported. It now reads it only when it counts (part 7's damaging-skill sources and Kitava's spells).

**Rows and tables:** `NoEffect` for `skill_triggered_when_you_focus_chance_%` (PoB's `chanceToTriggerOnFocus`, which no module reads) and `support_cast_on_mana_spent`; the three supports in `gems::SUPPORTS`, Unseen Strike in `gems::SKILLS`; `data-parity` allows their `Triggered` type and marks: 753 differences, `uncategorized` 0.

### Evidence

Run on the main workstation, 2026-11-26, with the pinned oracle (PoB v2.67.2, `b32759a`, clean) and the evidence bundle `local/p6/packs/b-ct`. The baseline is part 7's run (`local/p6/s3e`).

- **Fixtures** (new, all original, each run three times by PoB, identical; made by `local/p6/s3f_fixtures.py`). Every compared field equals PoB's:
  - `trigger-focus-ice-nova-witch.xml`: the rate 1/12 (0.08333).
  - `trigger-kitavas-thirst-ice-nova-witch.xml`: Fireball with +100 total mana cost (120) the source at 1.333 casts per second, the 50% chance: 0.6667; Ice Nova's own cost 123.
  - `trigger-maloneys-mechanism-burning-arrow-ranger.xml`: 0.87588 from the bow attack's 0.875 under the 1 s cooldown.
  - `trigger-hidden-blade-unseen-strike-ranger.xml`: 2 per second, no cost; Unseen Strike's damage stays partial for its PoB skill data (`pob-skill-data`), equal to PoB's (`Complete` since 2026-11-26, `docs/phase-6/skill-data.md`, "The agent list").
  - 302 fields are targets.
- **Public `calc-parity`** (565 builds): 0 unexpected. `Complete` fields went from 41,196 to 41,498, the new fixtures' 302. One other document changed in its reasons only: `holy-flame-totem-hierophant.xml` lost its crafted Focus line's `unmapped-stat`.
- **Corpus** (21 builds, local): no document changed.
- **Tests:** the workspace's 1,863 tests, clippy, fmt, and the wasm32 check pass.

## Where S3 stands

Every trigger config PoB has that changes a slice field is ported, but these:
- **The engine's data or model lacks something:** Seven Teachings (no unarmed attack), Duskblight (Stalking Pustule's level 1 row; maintainer follow-ups), the Holy Relic's Nova (skills' level requirements), Maloney's replica (spells by cast rate).
- **PoB applies nothing, so they wait for an in-game check (the game-truth list):** Squirming Terror at 100% (`local_display_chance_to_trigger_socketed_spell_on_kill_%`), `trigger_socketed_spell_on_skill_use_%` (the crafted "Trigger a Socketed Spell on Using a Skill"), and `spellslinger_cooldown_duration_+%`, which no PoB file names.
- **No slice field reads their rate:** the minions items trigger (Law of the Wilds, Rigwald's Crest, Jorrhast's Blacksteel, Ashcaller, Arakaali's Fang, Mark of the Shaper's anomaly), Shade Form, Contaminate, Mark On Hit, Hextouch, Vixen's Entrapment and the hex on trap (curses).
- **Not item triggers, outside S3's list:** the skills' own triggers (Kinetic Instability, Battlemage's Cry, Autoexertion, General's Cry, Snipe, Shattershard, Doom Blast, and kin) stay `unmodelled` where builds use them.
