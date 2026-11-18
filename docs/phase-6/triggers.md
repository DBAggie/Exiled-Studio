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
