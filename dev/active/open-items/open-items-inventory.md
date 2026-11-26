Last Updated: 2026-11-24

# Open items inventory (S0)

Every distinct reason in the public and corpus runs at `dev` `6deeb5da` (`local/p6/n/`, bundle `b-ct`), one class each: **A** agent, **B** data run, **C** maintainer. Made by `local/p6/tools/s0_inventory.py`; checked by `local/p6/tools/inventory_check.py`. Fields are fields blocked (a field counts once per reason); `sole` counts fields where it is the only reason.

## Fixture budget

Measured on PR #167 CI run 45112521852 at 4dd449a4 (461 fixtures): oracle windows 113.5 min, ubuntu 81.3 min: about 14.8 s per fixture on Windows. At 531 fixtures the Windows job takes about 131 min of its 150, so this session adds at most **75 fixtures** in all, several lines per fixture. S1 and S9 work on existing fixtures first.

## Totals by class

- **A:** 275 reasons, 28379 field-reason pairs (public and corpus)
- **B:** 0 reasons, 0 field-reason pairs (public and corpus)
- **C:** 69 reasons, 7048 field-reason pairs (public and corpus)
- Mixed keys: `unmodelled-gem` holds class B parts (the Skitterbots gems, a minion data run); `unresolved-line` `no-template` holds an A share for S9. See the gem table and S9.

## Reasons other than stats

| Reason | Class | Step | Effort | Public fields/builds | Corpus fields/builds | Note |
|---|---|---|---|---|---|---|
| `unmodelled-gem \| {}` | A | S2 | L | 1419/19 | 1114/15 | per gem, see parts |
| `unresolved-line \| {"reason": "no-template"}` | C | S9 | M | 1259/17 | 757/10 | mostly older wordings (needs a new Adapted kind: ADR-015, build and calc result v1) and note text; the A share in S9 |
| `unresolved-line \| {"reason": "ambiguous"}` | A | S9 | M | 655/9 | 470/6 | templates found; several readings |
| `unmodelled-passive \| unresolved-tree-line` | A | S9 | M | 284/4 | 600/8 | checked in S9: resolver gap (A) or older tree wording (C) |
| `unmodelled-passive \| unconnected-node` | A | S9 | M | 454/6 | 304/4 | PoB's handling of allocated nodes off the tree |
| `unresolved-line \| {"reason": "no-variant-fits"}` | A | S9 | M | 457/6 | 159/2 | templates found; no variant fits the value |
| `scaled-line \| {"scaling": "modifier-magnitude"}` | A | S9 | M | 365/5 | 160/2 |  |
| `scaled-line \| {"scaling": "catalyst"}` | A | S9 | M | 501/7 |  |  |
| `unmodelled-passive \| cluster-jewel-node` | A | S9 | L | 462/6 |  | b-ct carries the cluster tables; calc side |
| `unmodelled-item \| ambiguous-base` | A | S9 | M | 157/2 | 224/3 |  |
| `unmodelled-skill \| socket-colour-quality` | A | S1 | M | 275/4 | 77/1 |  |
| `unmodelled-skill \| pob-skill-data` | A | S1 | L | 219/31 | 108/13 |  |
| `unmodelled-passive \| timeless-jewel` | A | S9 | L | 136/2 | 151/2 | b-ct carries the timeless tables; calc side |
| `unmodelled-config \| {"input": "stormRainBeamOverlap"}` | A | S8 | S | 81/1 | 163/2 |  |
| `unmodelled-skill \| sigil-of-power-stages` | A | S1 | M | 78/1 | 161/2 |  |
| `excluded-line \| {"exclusion": "section"}` | A | S9 | M | 231/3 |  |  |
| `unmodelled-item \| flask-base` | A | S9 | M | 227/3 |  |  |
| `unmodelled-config \| {"input": "minionsConditionFullLife"}` | A | S6 | S | 147/2 | 79/1 |  |
| `unmodelled-config \| {"input": "feedingFrenzyFeedingFrenzyActive"}` | A | S8 | S | 138/2 | 79/1 |  |
| `unmodelled-config \| {"input": "ActiveBrands"}` | A | S8 | S | 70/1 | 140/2 |  |
| `unmodelled-config \| {"input": "toxicRainPodOverlap"}` | A | S8 | S | 67/1 | 130/2 |  |
| `unmodelled-config \| {"input": "conditionEnemyOnBrittleGround"}` | A | S8 | S |  | 159/2 |  |
| `unmodelled-skill \| skill-chill-effect` | A | S1 | M | 155/2 |  |  |
| `unhandled-kind \| {"kind": "item-property"}` | A | S9 | M | 70/1 | 80/1 |  |
| `unmodelled-config \| {"input": "buffAlchemistsGenius"}` | A | S8 | S |  | 130/2 |  |
| `unmodelled-condition \| {"condition": "NoLifeModifiersOnBodyArmour", "quantity": "life"}` | A | S8 | S | 37/2 | 52/3 |  |
| `unmodelled-config \| {"input": "conditionBrittleEffect"}` | A | S8 | S | 81/1 |  |  |
| `unmodelled-config \| {"input": "conditionEnemySapped"}` | A | S8 | S | 81/1 |  |  |
| `unmodelled-config \| {"input": "conditionEnemyBrittle"}` | A | S8 | S | 81/1 |  |  |
| `unmodelled-config \| {"input": "conditionEnemyScorched"}` | A | S8 | S | 81/1 |  |  |
| `unmodelled-item \| slot` | A | S9 | M |  | 81/1 |  |
| `unmodelled-config \| {"input": "conditionEnemyCrushed"}` | A | S8 | S |  | 81/1 |  |
| `unmodelled-config \| {"input": "conditionEnemyMoving"}` | A | S8 | S | 80/1 |  |  |
| `unmodelled-config \| {"input": "enemyConditionHitByLightningDamage"}` | A | S8 | S | 79/1 |  |  |
| `unmodelled-config \| {"input": "EEIgnoreHitDamage"}` | A | S8 | S | 79/1 |  |  |
| `unmodelled-config \| {"input": "buffUnholyMight"}` | A | S8 | S | 78/1 |  |  |
| `unmodelled-item \| jewel-limit` | A | S9 | M | 78/1 |  |  |
| `unmodelled-item \| flask-merge` | A | S9 | M | 77/1 |  |  |
| `unmodelled-config \| {"input": "bandit"}` | A | S8 | S | 77/1 |  |  |
| `unmodelled-passive \| unknown-node-kind` | A | S9 | S | 77/1 |  |  |
| `unmodelled-skill \| requires-shield` | A | S1 | M |  | 77/1 |  |
| `unmodelled-skill \| wave-of-conviction-exposure` | A | S1 | M |  | 76/1 |  |
| `unmodelled-config \| {"input": "conditionEnemyOnScorchedGround"}` | A | S8 | S |  | 76/1 |  |
| `unmodelled-config \| {"input": "detonateDeadCorpseLife"}` | A | S8 | S |  | 76/1 |  |
| `unmodelled-skill \| totem-data` | A | S1 | L | 71/9 | 1/1 |  |
| `unmodelled-config \| {"input": "minionsUseEnduranceCharges"}` | A | S6 | S | 69/1 |  |  |
| `unmodelled-config \| {"input": "minionBuffOnslaught"}` | A | S6 | S | 69/1 |  |  |
| `unmodelled-config \| {"input": "minionsOverrideFrenzyCharges"}` | A | S6 | S | 69/1 |  |  |
| `unmodelled-config \| {"input": "minionsUseFrenzyCharges"}` | A | S6 | S | 69/1 |  |  |
| `unmodelled-config \| {"input": "TotalSpectreLife"}` | A | S6 | S | 69/1 |  |  |
| `unmodelled-item \| abyssal-socket` | A | S9 | M | 69/1 |  |  |
| `unmodelled-config \| {"input": "multiplierNearbyAlly"}` | A | S8 | S | 68/1 |  |  |
| `unmodelled-skill \| rallying-cry-allies` | A | S1 | M | 68/1 |  |  |
| `unresolved-line \| {"reason": "only-shadowed"}` | A | S9 | S |  | 67/1 |  |
| `unread-line \| {}` | A | S9 | S | 66/1 |  | checked in S9 |
| `unmodelled-config \| {"input": "multiplierManaBurnStacks"}` | A | S8 | S |  | 65/1 |  |
| `unmodelled-skill \| aura-debuff-effect` | A | S1 | M | 54/1 |  |  |
| `unmodelled-config \| {"input": "presetBossSkills"}` | A | S7 | S | 54/1 |  |  |
| `unmodelled-config \| {"input": "conditionEnemyOnSappedGround"}` | A | S8 | S | 54/1 |  |  |
| `unmodelled-skill \| minion-buffs` | A | S1 | M | 23/4 | 22/2 |  |
| `unmodelled-skill \| reservation-becomes-cost` | A | S1 | M | 27/9 |  |  |
| `unmodelled-skill \| full-dps-skill` | A | S1 | M | 9/9 | 3/3 |  |
| `unmodelled-skill \| minion-modifier` | A | S1 | M |  | 9/1 |  |
| `unmodelled-skill \| full-dps-vaal` | A | S1 | M | 4/4 | 1/1 |  |
| `unmodelled-item \| no-physical-damage` | A | S9 | M | 4/1 |  |  |
| `unmodelled-condition \| {"condition": "DualWielding", "quantity": "block-chance"}` | A | S9 | M |  | 1/1 | an ambiguous base in a hand leaves DualWielding unsettled (S7, 2026-11-26) |
| `unmodelled-skill \| skitterbots-supports` | A | S2 | S |  |  | Summon Skitterbots with a support whose shock, chill, or non-damaging ailment effect, or Bonechill's flag, PoB's override reads (added after S0) |

### `unmodelled-gem` by gem

| Gem | Class | Step | In | Note |
|---|---|---|---|---|
| `ArcaneCloak` | A | S2 | public |  |
| `AspectOfTheCat` | A | S2 | corpus |  |
| `Bane` | A | S2 | public |  |
| `BlinkArrow` | A | S2 | public, corpus |  |
| `BrandSupport` | A | S2 | public, corpus |  |
| `ColdSnap` | A | S2 | public, corpus |  |
| `ConduitSigil` | A | S2 | public |  |
| `Convocation` | A | S2 | corpus |  |
| `CorruptingFever` | A | S2 | corpus |  |
| `EnergyBlade` | A | S2 | public |  |
| `GeneralsCry` | A | S5 | public |  |
| `ManaInfusedStaff` | A | S2 | public |  |
| `PactOfGhorr` | A | S2 | public |  |
| `PetrifiedBlood` | A | S2 | public |  |
| `PrimalAegis` | A | S2 | public, corpus |  |
| `PuresteelBanner` | A | S2 | public |  |
| `RainOfSpores` | A | S2 | public |  |
| `RaiseSpectre` | A | S6 | public, corpus | S6 checks the monster data in b-ct |
| `RaiseSpiders` | A | S2 | public |  |
| `Skitterbots` | B | S2 | corpus | Skitterbots: minion data run |
| `Spellslinger` | A | S3 | public, corpus |  |
| `SummonElementalRelic` | A | S2 | public |  |
| `SummonSentinelOfRadiance` | A | S2 | public |  |
| `SummonSkitterbots` | B | S2 | public, corpus | Skitterbots: minion data run (report, deferred) |
| `SupportAddedChaosDamage` | A | S2 | corpus |  |
| `SupportAuraDuration` | A | S2 | corpus |  |
| `SupportBarrage` | A | S2 | corpus |  |
| `SupportBloodMagic` | A | S2 | public |  |
| `SupportCloseCombat` | A | S2 | public |  |
| `SupportEclipse` | A | S2 | public |  |
| `SupportIncreasedDuration` | A | S2 | public, corpus |  |
| `SupportLivingLightning` | A | S2 | public |  |
| `SupportOnslaught` | A | S2 | corpus |  |
| `SupportRangedAttackTotem` | A | S2 | corpus |  |
| `SupportSacredWisps` | A | S5 | public |  |
| `SupportSpellTotem` | A | S2 | corpus |  |
| `SupportUnboundAilments` | A | S2 | public |  |
| `VaalAbsolution` | A | S2 | corpus |  |
| `VaalAncestralWarchief` | A | S2 | public |  |
| `VaalGrace` | A | S2 | public |  |
| `VaalSmite` | A | S2 | public |  |

## Unmapped stats

| Stat | Class | Step | Public fields/builds | Corpus fields/builds | Note |
|---|---|---|---|---|---|
| `local_display_grants_skill_herald_of_the_breach_level` | A | S8 | 70/1 | 210/3 | close-out C-item-skill: item-granted skills (PoB's ExtraSkill) |
| `magic_utility_flasks_cannot_be_used` | C |  | 218/3 |  | game-truth list: PoB applies nothing; needs the maintainer's check in game |
| `trigger_socketed_spell_on_skill_use_%` | C |  | 138/2 | 79/1 | game-truth list: PoB applies nothing; needs the maintainer's check in game |
| `active_skill_shock_as_though_damage_+%_final` | A | S1 |  | 163/2 | close-out C-unmodelled-skill: owned by a skill S1 or S2 models |
| `spellslinger_mana_reservation_efficiency_+%` | A | S3 | 78/1 | 80/1 | deferred out of Phase 6; stretch step |
| `base_aggravate_bleeding_on_attack_hit_chance_%` | C |  | 157/2 |  | game-truth list: PoB applies nothing; needs the maintainer's check in game |
| `empowered_attack_damage_+%` | A | S8 | 148/2 |  | close-out C-engine-system: a system the engine lacks; revisited in S8 |
| `local_unique_jewel_spectres_gain_soul_eater_on_kill_%_chance_with_50_int_in_radius` | C |  | 69/1 | 79/1 | game-truth list: PoB applies nothing; needs the maintainer's check in game |
| `alternate_ascendancy_herald_buff_effect_+%_final_per_1%_mana_reserved` | A | S8 | 70/1 | 70/1 | close-out C-engine-system: a system the engine lacks; revisited in S8 |
| `alternate_ascendancy_herald_minion_and_herald_damage_+%_final_per_1%_life_reserved` | A | S8 | 70/1 | 70/1 | close-out C-engine-system: a system the engine lacks; revisited in S8 |
| `support_blunt_chance_to_trigger_shockwave_on_hit_%` | A | S1 | 83/4 |  | close-out C-unmodelled-skill: owned by a skill S1 or S2 models |
| `intimidating_cry_cooldown_speed_+%` | C |  |  | 83/1 | game-truth list: PoB applies nothing; needs the maintainer's check in game |
| `quiver_mod_effect_+%` | A | S8 |  | 83/1 | close-out C-engine-system: a system the engine lacks; revisited in S8 |
| `keystone_precise_technique` | A | S8 |  | 82/1 | close-out C-keystone: keystone rule not ported |
| `armour_from_helmet_and_gloves_+%` | C |  |  | 82/1 | game-truth list: PoB applies nothing; needs the maintainer's check in game |
| `body_armour_implicit_damage_taken_-1%_final_per_X_strength` | A | S8 |  | 82/1 |  |
| `support_spiritual_cry_damage_+%_final` | A | S1 | 81/2 |  | close-out C-unmodelled-skill: owned by a skill S1 or S2 models |
| `number_of_warcries_exerting_this_action` | A | S1 | 81/2 |  | close-out C-unmodelled-skill: owned by a skill S1 or S2 models |
| `triggered_by_spiritual_cry` | A | S1 | 81/2 |  | close-out C-unmodelled-skill: owned by a skill S1 or S2 models |
| `attack_minimum_added_cold_damage_for_elemental_hit` | A | S1 | 81/1 |  | close-out C-unmodelled-skill: owned by a skill S1 or S2 models |
| `spell_suppression_chance_%_if_all_equipment_grants_evasion` | A | S8 | 81/1 |  | close-out C-engine-system: a system the engine lacks; revisited in S8 |
| `elemental_hit_no_physical_chaos_damage` | A | S1 | 81/1 |  | close-out C-unmodelled-skill: owned by a skill S1 or S2 models |
| `attack_maximum_added_fire_damage_for_elemental_hit` | A | S1 | 81/1 |  | close-out C-unmodelled-skill: owned by a skill S1 or S2 models |
| `attack_maximum_added_cold_damage_for_elemental_hit` | A | S1 | 81/1 |  | close-out C-unmodelled-skill: owned by a skill S1 or S2 models |
| `elemental_hit_damage_+%_final_per_enemy_elemental_ailment` | A | S1 | 81/1 |  | close-out C-unmodelled-skill: owned by a skill S1 or S2 models |
| `attack_maximum_added_lightning_damage_for_elemental_hit` | A | S1 | 81/1 |  | close-out C-unmodelled-skill: owned by a skill S1 or S2 models |
| `local_display_grants_skill_accuracy_crits_aura_level` | A | S8 | 81/1 |  | close-out C-item-skill: item-granted skills (PoB's ExtraSkill) |
| `player_far_shot` | A | S8 | 81/1 |  | close-out C-engine-system: a system the engine lacks; revisited in S8 |
| `attack_minimum_added_lightning_damage_for_elemental_hit` | A | S1 | 81/1 |  | close-out C-unmodelled-skill: owned by a skill S1 or S2 models |
| `attack_minimum_added_fire_damage_for_elemental_hit` | A | S1 | 81/1 |  | close-out C-unmodelled-skill: owned by a skill S1 or S2 models |
| `local_accuracy_rating_+%_per_2%_quality` | C |  |  | 81/1 | game-truth list: PoB applies nothing; needs the maintainer's check in game |
| `attack_and_cast_speed_+%_with_physical_skills` | C |  |  | 81/1 | game-truth list: PoB applies nothing; needs the maintainer's check in game |
| `keystone_natures_presence` | C |  |  | 81/1 | game-truth list: PoB applies nothing; needs the maintainer's check in game |
| `local_item_quality_+` | A | S8 | 80/1 |  | close-out C-engine-system: a system the engine lacks; revisited in S8 |
| `warcries_have_infinite_power` | A | S8 | 80/1 |  | close-out C-engine-system: a system the engine lacks; revisited in S8 |
| `skill_area_of_effect_+%_final_in_sand_stance` | A | S1 | 80/1 |  | close-out C-unmodelled-skill: owned by a skill S1 or S2 models |
| `enemy_phys_reduction_%_penalty_vs_hit` | C |  | 80/1 |  | game-truth list: PoB applies nothing; needs the maintainer's check in game |
| `blood_spears_damage_+%_final_in_blood_stance` | A | S1 | 80/1 |  | close-out C-unmodelled-skill: owned by a skill S1 or S2 models |
| `local_maximum_life_per_2%_quality` | C |  | 80/1 |  | game-truth list: PoB applies nothing; needs the maintainer's check in game |
| `enemies_taunted_by_your_warcies_are_intimidated` | A | S8 | 80/1 |  | close-out C-engine-system: a system the engine lacks; revisited in S8 |
| `generals_cry_maximum_warriors_+` | C |  | 80/1 |  | game-truth list: PoB applies nothing; needs the maintainer's check in game |
| `local_quality_does_not_increase_defences` | C |  | 80/1 |  | game-truth list: PoB applies nothing; needs the maintainer's check in game |
| `warcries_exert_twice_as_many_attacks` | A | S8 | 80/1 |  | close-out C-engine-system: a system the engine lacks; revisited in S8 |
| `local_double_damage_with_attacks` | A | S8 | 80/1 |  | close-out C-engine-system: a system the engine lacks; revisited in S8 |
| `missing_unreserved_life_%_gained_as_life_before_hit_per_defiance` | A | S7 | 80/1 |  | deferred out of Phase 6; stretch step |
| `blood_spears_base_number_of_spears` | C |  | 80/1 |  | game-truth list: PoB applies nothing; needs the maintainer's check in game |
| `local_display_grants_skill_enduring_cry_level` | A | S8 | 80/1 |  | close-out C-item-skill: item-granted skills (PoB's ExtraSkill) |
| `boneshatter_trauma_base_duration_ms` | A | S1 | 80/1 |  | close-out C-unmodelled-skill: owned by a skill S1 or S2 models |
| `trauma_strike_damage_+%_final_per_trauma_capped` | A | S1 | 80/1 |  | close-out C-unmodelled-skill: owned by a skill S1 or S2 models |
| `additional_maximum_all_elemental_resistances_%_if_all_equipment_grants_armour` | A | S8 | 80/1 |  | close-out C-engine-system: a system the engine lacks; revisited in S8 |
| `aggravate_bleeding_on_exerted_attack_hit_chance_%` | C |  | 80/1 |  | game-truth list: PoB applies nothing; needs the maintainer's check in game |
| `trauma_strike_self_damage_per_trauma` | A | S1 | 80/1 |  | close-out C-unmodelled-skill: owned by a skill S1 or S2 models |
| `self_critical_strike_multiplier_-%_per_endurance_charge` | C |  | 80/1 |  | game-truth list: PoB applies nothing; needs the maintainer's check in game |
| `lose_all_trauma_at_X_trauma` | A | S1 | 80/1 |  | close-out C-unmodelled-skill: owned by a skill S1 or S2 models |
| `curse_on_hit_level_vulnerability` | A | S8 | 80/1 |  | close-out C-item-skill: item-granted skills (PoB's ExtraSkill) |
| `impale_effect_+%_for_impales_inflicted_by_two_handed_weapons_on_non_impaled_enemies` | C |  | 80/1 |  | game-truth list: PoB applies nothing; needs the maintainer's check in game |
| `life_leech_does_not_stop_at_full_life` | A | S8 | 80/1 |  | close-out C-engine-system: a system the engine lacks; revisited in S8 |
| `kinetic_fusillade_maximum_floating_projectiles` | A | S1 | 80/1 |  | close-out C-unmodelled-skill: owned by a skill S1 or S2 models |
| `unique_helmet_cast_speed_+%_applies_to_attack_speed_at_%_of_original_value` | A | S8 | 80/1 |  | close-out C-engine-system: a system the engine lacks; revisited in S8 |
| `kinetic_fusillade_base_delay_between_projectiles_ms` | A | S1 | 80/1 |  | close-out C-unmodelled-skill: owned by a skill S1 or S2 models |
| `support_greater_volley_projectile_damage_+%_final` | A | S1 | 80/1 |  | close-out C-unmodelled-skill: owned by a skill S1 or S2 models |
| `determination_aura_effect_+%_while_at_minimum_endurance_charges` | A | S8 | 80/1 |  | close-out C-engine-system: a system the engine lacks; revisited in S8 |
| `totems_nearby_enemies_damage_taken_+%` | C |  | 80/1 |  | game-truth list: PoB applies nothing; needs the maintainer's check in game |
| `local_critical_strike_chance_+%_per_4%_quality` | C |  | 80/1 |  | game-truth list: PoB applies nothing; needs the maintainer's check in game |
| `support_reduced_duration_damage_+%_final` | A | S1 | 80/1 |  | close-out C-unmodelled-skill: owned by a skill S1 or S2 models |
| `ailment_types_apply_damage_taken_+%` | A | S8 | 80/1 |  | close-out C-engine-system: a system the engine lacks; revisited in S8 |
| `kinetic_fusillade_damage_+%_final_per_projectile_fired` | A | S1 | 80/1 |  | close-out C-unmodelled-skill: owned by a skill S1 or S2 models |
| `gain_wand_accuracy_rating_equal_to_intelligence` | A | S8 | 80/1 |  | close-out C-engine-system: a system the engine lacks; revisited in S8 |
| `support_parallel_projectile_number_of_points_per_side` | A | S1 | 80/1 |  | close-out C-unmodelled-skill: owned by a skill S1 or S2 models |
| `anger_aura_effect_+%_while_at_maximum_endurance_charges` | A | S8 | 80/1 |  | close-out C-engine-system: a system the engine lacks; revisited in S8 |
| `skill_can_only_use_non_melee_weapons` | A | S1 |  | 80/1 | close-out C-unmodelled-skill: owned by a skill S1 or S2 models |
| `frenzy_skill_attack_damage_+%_final_per_frenzy_charge` | A | S1 |  | 80/1 | close-out C-unmodelled-skill: owned by a skill S1 or S2 models |
| `frenzy_skill_attack_speed_+%_final_per_frenzy_charge` | A | S1 |  | 80/1 | close-out C-unmodelled-skill: owned by a skill S1 or S2 models |
| `support_barrage_damage_+%_final` | A | S1 |  | 80/1 | close-out C-unmodelled-skill: owned by a skill S1 or S2 models |
| `critical_strikes_do_not_always_ignite` | A | S8 |  | 80/1 | close-out C-engine-system: a system the engine lacks; revisited in S8 |
| `support_barrage_trap_and_mine_throwing_time_+%_final_per_projectile_fired` | A | S1 |  | 80/1 | close-out C-unmodelled-skill: owned by a skill S1 or S2 models |
| `projectiles_barrage` | A | S1 |  | 80/1 | close-out C-unmodelled-skill: owned by a skill S1 or S2 models |
| `support_barrage_attack_time_+%_per_projectile_fired` | A | S1 |  | 80/1 | close-out C-unmodelled-skill: owned by a skill S1 or S2 models |
| `enemies_shocked_by_you_have_physical_damage_%_converted_to_lightning` | A | S8 |  | 80/1 | close-out C-engine-system: a system the engine lacks; revisited in S8 |
| `thrown_shield_secondary_projectile_damage_+%_final` | C |  |  | 80/1 | game-truth list: PoB applies nothing; needs the maintainer's check in game |
| `support_maim_chance_physical_damage_+%_final` | A | S1 |  | 80/1 | close-out C-unmodelled-skill: owned by a skill S1 or S2 models |
| `keystone_crimson_dance` | A | S8 |  | 80/1 | close-out C-keystone: keystone rule not ported |
| `primary_projectile_chains_+` | A | S1 |  | 80/1 | close-out C-unmodelled-skill: owned by a skill S1 or S2 models |
| `support_unbound_ailments_ailment_damage_+%_final` | A | S1 |  | 80/1 | close-out C-unmodelled-skill: owned by a skill S1 or S2 models |
| `non_damaging_ailment_effect_+%` | A | S8 |  | 80/1 | close-out C-engine-system: a system the engine lacks; revisited in S8 |
| `body_armour_defences_doubled_while_no_gems_in_body_armour` | C |  |  | 80/1 | game-truth list: PoB applies nothing; needs the maintainer's check in game |
| `maximum_life_+%_while_no_gems_in_gloves` | C |  |  | 80/1 | game-truth list: PoB applies nothing; needs the maintainer's check in game |
| `elemental_resistance_%_while_no_gems_in_helmet` | C |  |  | 80/1 | game-truth list: PoB applies nothing; needs the maintainer's check in game |
| `local_flask_recovery_amount_%_to_recover_instantly` | A | S8 | 79/1 |  | close-out C-engine-system: a system the engine lacks; revisited in S8 |
| `enemies_explode_on_death_by_attack_for_10%_life_as_physical_damage` | A | S8 |  | 79/1 | close-out C-no-slice: PoB reads it into no slice field; NoEffect candidate |
| `base_bone_golem_granted_buff_effect_+%` | C |  |  | 79/1 | game-truth list: PoB applies nothing; needs the maintainer's check in game |
| `minion_base_physical_damage_%_to_convert_to_fire_per_red_socket_on_item` | A | S8 |  | 79/1 |  |
| `minion_base_physical_damage_%_to_convert_to_chaos_per_white_socket_on_item` | A | S8 |  | 79/1 |  |
| `minion_base_physical_damage_%_to_convert_to_cold_per_green_socket_on_item` | A | S8 |  | 79/1 |  |
| `minion_base_physical_damage_%_to_convert_to_lightning_per_blue_socket_on_item` | A | S8 |  | 79/1 |  |
| `minion_damage_over_time_multiplier_+_per_minion_abyss_jewel_up_to_+30` | C |  | 78/1 |  | game-truth list: PoB applies nothing; needs the maintainer's check in game |
| `local_socketed_support_gem_quality_+` | A | S8 | 78/1 |  | close-out C-gem-property: PoB's GemProperty, ExtraSupport, LinkedSupport |
| `keystone_corrupted_defences` | A | S8 | 78/1 |  | close-out C-keystone: keystone rule not ported |
| `support_gems_socketed_in_off_hand_also_support_main_hand_skills` | A | S8 | 78/1 |  | close-out C-gem-property: PoB's GemProperty, ExtraSupport, LinkedSupport |
| `gain_debilitating_presence_ms_on_kill_vs_rare_or_unique_enemy` | A | S8 | 78/1 |  | close-out C-engine-system: a system the engine lacks; revisited in S8 |
| `local_display_raise_spider_on_kill_%_chance` | A | S8 | 78/1 |  | close-out C-item-skill: item-granted skills (PoB's ExtraSkill) |
| `apply_maximum_wither_stacks_%_chance_when_you_apply_withered` | C |  | 78/1 |  | game-truth list: PoB applies nothing; needs the maintainer's check in game |
| `projectiles_cannot_split` | A | S1 | 78/1 |  | close-out C-unmodelled-skill: owned by a skill S1 or S2 models |
| `support_fork_projectile_damage_+%_final` | A | S1 | 78/1 |  | close-out C-unmodelled-skill: owned by a skill S1 or S2 models |
| `skill_used_by_sacred_wisp_damage_+%_final` | A | S1 | 78/1 |  | close-out C-unmodelled-skill: owned by a skill S1 or S2 models |
| `summon_sacred_wisps_on_hit` | A | S1 | 78/1 |  | close-out C-unmodelled-skill: owned by a skill S1 or S2 models |
| `enemies_near_corpses_created_recently_are_shocked_and_chilled` | A | S8 | 78/1 |  | close-out C-engine-system: a system the engine lacks; revisited in S8 |
| `added_physical_damage_to_attacks_equal_to_%_maximum_mana` | A | S1 | 78/1 |  | close-out C-unmodelled-skill: owned by a skill S1 or S2 models |
| `reduce_enemy_chaos_resistance_%` | A | S1 | 78/1 |  | close-out C-unmodelled-skill: owned by a skill S1 or S2 models |
| `regenerate_%_maximum_mana_over_2_seconds_on_consuming_corpse` | A | S10 | 78/1 |  |  |
| `necromancer_mana_regeneration_rate_per_minute_for_you_and_allies_per_nearby_corpse` | C |  | 78/1 |  | game-truth list: PoB applies nothing; needs the maintainer's check in game |
| `kinetic_blast_modifiers_to_number_of_projectiles_instead_apply_to_number_of_clusters` | A | S1 | 78/1 |  | close-out C-unmodelled-skill: owned by a skill S1 or S2 models |
| `shock_maximum_magnitude_+` | A | S8 | 78/1 |  | close-out C-engine-system: a system the engine lacks; revisited in S8 |
| `archmage_gain_lightning_damage_%_of_max_unreserved_mana` | A | S1 | 78/1 |  | close-out C-unmodelled-skill: owned by a skill S1 or S2 models |
| `mana_%_to_add_as_energy_shield` | A | S8 | 78/1 |  | close-out C-engine-system: a system the engine lacks; revisited in S8 |
| `additive_mana_modifiers_apply_to_shock_effect_at_30%_value` | A | S8 | 78/1 |  | close-out C-engine-system: a system the engine lacks; revisited in S8 |
| `exposure_you_inflict_has_minimum_resistance_%` | A | S8 | 78/1 |  | close-out C-engine-system: a system the engine lacks; revisited in S8 |
| `raider_shock_maximum_effect_override_%` | A | S8 | 77/1 |  | close-out C-engine-system: a system the engine lacks; revisited in S8 |
| `spell_damage_taken_+_per_barkskin_stack` | A | S8 | 77/1 |  | close-out C-item-skill: item-granted skills (PoB's ExtraSkill) |
| `elusive_effect_+%` | A | S8 | 77/1 |  | close-out C-engine-system: a system the engine lacks; revisited in S8 |
| `spell_damage_%_suppressed_per_bark_below_max` | A | S8 | 77/1 |  | close-out C-item-skill: item-granted skills (PoB's ExtraSkill) |
| `tailwind_if_have_crit_recently` | C |  | 77/1 |  | game-truth list: PoB applies nothing; needs the maintainer's check in game |
| `local_unique_jewel_life_increases_applies_to_energy_shield` | C |  | 77/1 |  | game-truth list: PoB applies nothing; needs the maintainer's check in game |
| `support_multiple_projectile_damage_+%_final` | A | S1 | 77/1 |  | close-out C-unmodelled-skill: owned by a skill S1 or S2 models |
| `raider_shocks_you_apply_can_stack_up_to_50_times` | A | S8 | 77/1 |  | close-out C-engine-system: a system the engine lacks; revisited in S8 |
| `elusive_effect_on_self_+%_per_power_charge` | A | S8 | 77/1 |  | close-out C-engine-system: a system the engine lacks; revisited in S8 |
| `local_energy_shield` | A | S8 | 77/1 |  | close-out C-engine-system: a system the engine lacks; revisited in S8 |
| `base_spell_cooldown_speed_+%` | A | S1 | 77/1 |  | close-out C-unmodelled-skill: owned by a skill S1 or S2 models |
| `ice_spear_second_form_critical_strike_multiplier_+` | C |  | 77/1 |  | game-truth list: PoB applies nothing; needs the maintainer's check in game |
| `ice_spear_second_form_critical_strike_chance_+%` | C |  | 77/1 |  | game-truth list: PoB applies nothing; needs the maintainer's check in game |
| `ice_spear_second_form_projectile_speed_+%_final` | C |  | 77/1 |  | game-truth list: PoB applies nothing; needs the maintainer's check in game |
| `reduce_enemy_elemental_resistance_%` | A | S1 | 77/1 |  | close-out C-unmodelled-skill: owned by a skill S1 or S2 models |
| `local_display_grants_skill_barkskin` | A | S8 | 77/1 |  | close-out C-item-skill: item-granted skills (PoB's ExtraSkill) |
| `critical_strike_chance_+%_per_mine_detonated_recently_up_to_100%` | C |  | 77/1 |  | game-truth list: PoB applies nothing; needs the maintainer's check in game |
| `critical_strike_multiplier_+_per_mine_detonated_recently_up_to_40` | C |  | 77/1 |  | game-truth list: PoB applies nothing; needs the maintainer's check in game |
| `curse_on_hit_level_frostbite` | A | S8 | 77/1 |  | close-out C-item-skill: item-granted skills (PoB's ExtraSkill) |
| `local_flask_recover_instantly_when_on_low_life` | C |  | 77/1 |  | game-truth list: PoB applies nothing; needs the maintainer's check in game |
| `summoned_skitterbots_non_damaging_ailment_effect_+%` | A | S1 | 77/1 |  | close-out C-unmodelled-skill: owned by a skill S1 or S2 models |
| `sabotuer_mines_apply_damage_taken_+%_to_nearby_enemies_up_to_10%` | A | S8 | 77/1 |  | close-out C-engine-system: a system the engine lacks; revisited in S8 |
| `summon_fire_skitterbot` | A | S1 | 77/1 |  | close-out C-unmodelled-skill: owned by a skill S1 or S2 models |
| `summoned_skitterbots_auras_also_affect_you` | A | S1 | 77/1 |  | close-out C-unmodelled-skill: owned by a skill S1 or S2 models |
| `mine_detonation_speed_+%` | C |  | 77/1 |  | game-truth list: PoB applies nothing; needs the maintainer's check in game |
| `mine_%_chance_to_detonate_twice` | C |  | 77/1 |  | game-truth list: PoB applies nothing; needs the maintainer's check in game |
| `spell_critical_strikes_bifurcate` | C |  | 77/1 |  | game-truth list: PoB applies nothing; needs the maintainer's check in game |
| `mine_aura_effect_+%` | A | S8 | 77/1 |  | close-out C-engine-system: a system the engine lacks; revisited in S8 |
| `elemental_damage_+%_per_different_enemy_elemental_ailment` | C |  | 77/1 |  | game-truth list: PoB applies nothing; needs the maintainer's check in game |
| `crits_have_culling_strike` | A | S8 | 77/1 |  | close-out C-engine-system: a system the engine lacks; revisited in S8 |
| `active_skill_bleeding_damage_+%_final` | A | S1 | 77/1 |  | close-out C-unmodelled-skill: owned by a skill S1 or S2 models |
| `keystone_tinctures_drain_life` | A | S8 | 77/1 |  | close-out C-keystone: keystone rule not ported |
| `support_better_ailments_ailment_damage_+%_final` | A | S1 | 77/1 |  | close-out C-unmodelled-skill: owned by a skill S1 or S2 models |
| `all_skill_gem_quality_+` | A | S8 | 77/1 |  | close-out C-gem-property: PoB's GemProperty, ExtraSupport, LinkedSupport |
| `support_better_ailments_hit_damage_+%_final` | A | S1 | 77/1 |  | close-out C-unmodelled-skill: owned by a skill S1 or S2 models |
| `enemies_in_presence_and_you_count_as_moving_when_affected_by_elemental_ailments` | A | S8 | 77/1 |  | close-out C-engine-system: a system the engine lacks; revisited in S8 |
| `local_flask_chance_to_freeze_shock_ignite_%_while_healing` | C |  | 77/1 |  | game-truth list: PoB applies nothing; needs the maintainer's check in game |
| `local_unique_flask_nearby_enemies_cursed_with_level_x_vulnerability_during_flask_effect` | A | S8 | 77/1 |  | close-out C-item-skill: item-granted skills (PoB's ExtraSkill) |
| `non_chilled_enemies_you_bleed_are_chilled` | C |  | 77/1 |  | game-truth list: PoB applies nothing; needs the maintainer's check in game |
| `dual_wield_inherent_bonuses_are_doubled` | A | S8 | 77/1 |  | close-out C-engine-system: a system the engine lacks; revisited in S8 |
| `random_skill_gem_level_+_index` | C |  | 77/1 |  | game-truth list: PoB applies nothing; needs the maintainer's check in game |
| `throw_traps_in_circle_radius` | A | S1 | 77/1 |  | close-out C-unmodelled-skill: owned by a skill S1 or S2 models |
| `critical_strike_chance_+%_vs_enemies_with_lightning_exposure` | C |  | 77/1 |  | game-truth list: PoB applies nothing; needs the maintainer's check in game |
| `random_skill_gem_level_+_level` | C |  | 77/1 |  | game-truth list: PoB applies nothing; needs the maintainer's check in game |
| `trap_%_chance_to_trigger_twice` | C |  | 77/1 |  | game-truth list: PoB applies nothing; needs the maintainer's check in game |
| `curse_on_hit_level_conductivity` | A | S8 | 77/1 |  | close-out C-item-skill: item-granted skills (PoB's ExtraSkill) |
| `shock_effect_+%` | A | S8 | 77/1 |  | close-out C-engine-system: a system the engine lacks; revisited in S8 |
| `support_additional_trap_mine_%_chance_for_3_additional_trap_mine` | A | S1 | 77/1 |  | close-out C-unmodelled-skill: owned by a skill S1 or S2 models |
| `support_additional_trap_mine_%_chance_for_1_additional_trap_mine` | A | S8 | 77/1 |  | close-out C-engine-system: a system the engine lacks; revisited in S8 |
| `support_additional_trap_mine_%_chance_for_2_additional_trap_mine` | A | S1 | 77/1 |  | close-out C-unmodelled-skill: owned by a skill S1 or S2 models |
| `frost_fury_duration_+%_per_stage` | A | S1 | 77/1 |  | close-out C-unmodelled-skill: owned by a skill S1 or S2 models |
| `frost_fury_fire_speed_+%_per_stage` | A | S1 | 77/1 |  | close-out C-unmodelled-skill: owned by a skill S1 or S2 models |
| `support_lesser_multiple_projectile_damage_+%_final` | A | S1 | 77/1 |  | close-out C-unmodelled-skill: owned by a skill S1 or S2 models |
| `frost_fury_max_number_of_stages` | A | S1 | 77/1 |  | close-out C-unmodelled-skill: owned by a skill S1 or S2 models |
| `frost_fury_fire_speed_+%_final_while_channelling` | A | S1 | 77/1 |  | close-out C-unmodelled-skill: owned by a skill S1 or S2 models |
| `support_focus_channel_damage_+%_final_per_second_channelling_up_to_60%` | A | S1 | 77/1 |  | close-out C-unmodelled-skill: owned by a skill S1 or S2 models |
| `display_frost_fury_additive_cast_speed_modifiers_apply_to_fire_speed` | A | S1 | 77/1 |  | close-out C-unmodelled-skill: owned by a skill S1 or S2 models |
| `frost_fury_base_fire_interval_ms` | A | S1 | 77/1 |  | close-out C-unmodelled-skill: owned by a skill S1 or S2 models |
| `local_unique_jewel_freezing_pulse_damage_+%_if_enemy_shattered_recently_with_50_int_in_radius` | C |  |  | 77/1 | game-truth list: PoB applies nothing; needs the maintainer's check in game |
| `local_unique_jewel_freezing_pulse_number_of_additional_projectiles_with_50_int_in_radius` | C |  |  | 77/1 | game-truth list: PoB applies nothing; needs the maintainer's check in game |
| `display_what_freezing_pulse_does` | A | S1 |  | 77/1 | close-out C-unmodelled-skill: owned by a skill S1 or S2 models |
| `support_cruelty_hit_damage_+%_final` | A | S1 |  | 76/1 | close-out C-unmodelled-skill: owned by a skill S1 or S2 models |
| `detonate_dead_%_chance_to_detonate_additional_corpse` | C |  |  | 76/1 | game-truth list: PoB applies nothing; needs the maintainer's check in game |
| `corpse_explosion_monster_life_permillage_fire` | A | S1 |  | 76/1 | close-out C-unmodelled-skill: owned by a skill S1 or S2 models |
| `all_damage_can_ignite` | A | S8 |  | 76/1 | close-out C-engine-system: a system the engine lacks; revisited in S8 |
| `support_ignite_prolif_ignite_damage_+%_final` | A | S1 |  | 76/1 | close-out C-unmodelled-skill: owned by a skill S1 or S2 models |
| `hits_grant_cruelty` | A | S1 |  | 76/1 | close-out C-unmodelled-skill: owned by a skill S1 or S2 models |
| `gain_alchemists_genius_on_flask_use_%` | A | S8 |  | 75/1 | close-out C-engine-system: a system the engine lacks; revisited in S8 |
| `base_number_of_arrows` | A | S1 |  | 75/1 | close-out C-unmodelled-skill: owned by a skill S1 or S2 models |
| `support_damage_while_on_full_life_+%_final` | A | S1 |  | 75/1 | close-out C-unmodelled-skill: owned by a skill S1 or S2 models |
| `local_display_socketed_gems_have_mana_reservation_+%` | A | S8 |  | 74/1 | close-out C-engine-system: a system the engine lacks; revisited in S8 |
| `local_display_trigger_level_x_reflection_skill_on_equip` | A | S8 | 71/1 |  | close-out C-item-skill: item-granted skills (PoB's ExtraSkill) |
| `self_bleed_duration_+%` | A | S1 | 71/1 |  | close-out C-unmodelled-skill: owned by a skill S1 or S2 models |
| `damage_over_time_+%_per_100_max_life` | C |  | 71/1 |  | game-truth list: PoB applies nothing; needs the maintainer's check in game |
| `num_magic_utility_flasks_always_apply` | A | S8 | 71/1 |  | close-out C-engine-system: a system the engine lacks; revisited in S8 |
| `consecrated_ground_while_stationary_radius_if_highest_attribute_is_strength` | C |  | 71/1 |  | game-truth list: PoB applies nothing; needs the maintainer's check in game |
| `your_auras_except_malevolence_are_disabled` | A | S8 | 71/1 |  | close-out C-engine-system: a system the engine lacks; revisited in S8 |
| `local_unique_jewel_passive_jewel_socket_mod_effect_+%_with_corrupted_magic_jewel_socketed` | A | S8 | 71/1 |  | close-out C-engine-system: a system the engine lacks; revisited in S8 |
| `spell_damage_+%_per_100_max_life` | A | S8 | 71/1 |  | close-out C-engine-system: a system the engine lacks; revisited in S8 |
| `hinder_enemy_chaos_damage_taken_+%` | A | S8 | 70/1 |  | close-out C-engine-system: a system the engine lacks; revisited in S8 |
| `base_movement_velocity_+%` | A | S1 | 70/1 |  | close-out C-unmodelled-skill: owned by a skill S1 or S2 models |
| `non_curse_aura_effect_+%_per_10_devotion` | A | S8 | 70/1 |  | close-out C-engine-system: a system the engine lacks; revisited in S8 |
| `keystone_quiet_might` | A | S8 | 70/1 |  | close-out C-keystone: keystone rule not ported |
| `keystone_soul_tether` | C |  | 70/1 |  | game-truth list: PoB applies nothing; needs the maintainer's check in game |
| `extra_damage_rolls_with_lightning_damage_on_non_critical_hits` | A | S8 | 70/1 |  | close-out C-engine-system: a system the engine lacks; revisited in S8 |
| `curse_on_hit_level_despair` | C |  | 70/1 |  | game-truth list: PoB applies nothing; needs the maintainer's check in game |
| `modifiers_to_minimum_frenzy_charges_instead_apply_to_affliction_charges` | A | S8 | 70/1 |  | close-out C-engine-system: a system the engine lacks; revisited in S8 |
| `projectile_damage_+%_per_chain` | A | S8 | 70/1 |  | close-out C-engine-system: a system the engine lacks; revisited in S8 |
| `local_shield_trigger_socketed_elemental_spell_on_block_display_cooldown` | A | S8 | 70/1 |  | close-out C-gem-property: PoB's GemProperty, ExtraSupport, LinkedSupport |
| `attacks_chance_to_poison_%_on_max_frenzy_charges` | A | S8 | 70/1 |  | close-out C-engine-system: a system the engine lacks; revisited in S8 |
| `additional_poison_chance_%_when_inflicting_poison` | A | S8 | 70/1 |  | close-out C-engine-system: a system the engine lacks; revisited in S8 |
| `maximum_affliction_charges_is_equal_to_maximum_frenzy_charges` | A | S8 | 70/1 |  | close-out C-engine-system: a system the engine lacks; revisited in S8 |
| `projectile_damage_+%_per_remaining_chain` | C |  | 70/1 |  | game-truth list: PoB applies nothing; needs the maintainer's check in game |
| `gain_affliction_charges_instead_of_frenzy_charges` | A | S8 | 70/1 |  | close-out C-engine-system: a system the engine lacks; revisited in S8 |
| `local_unique_flask_explode_enemies_for_10%_life_as_random_element_on_kill_chance_%_during_flask_effect` | A | S8 |  | 70/1 | close-out C-no-slice: PoB reads it into no slice field; NoEffect candidate |
| `local_nearby_enemies_base_physical_damage_%_to_convert_to_fire` | A | S8 |  | 70/1 | close-out C-engine-system: a system the engine lacks; revisited in S8 |
| `local_left_ring_socketed_curse_replaces_skitterbots_chilling_aura` | A | S1 | 69/1 |  | close-out C-unmodelled-skill: owned by a skill S1 or S2 models |
| `keystone_wicked_ward` | A | S8 | 69/1 |  | close-out C-keystone: keystone rule not ported |
| `local_socketed_curse_gem_level_+` | A | S8 | 69/1 |  | close-out C-gem-property: PoB's GemProperty, ExtraSupport, LinkedSupport |
| `local_right_ring_socketed_curse_replaces_skitterbots_shocking_aura` | A | S1 | 69/1 |  | close-out C-unmodelled-skill: owned by a skill S1 or S2 models |
| `global_defences_+%_per_raised_spectre` | A | S6 | 69/1 |  | deferred out of Phase 6; stretch step |
| `damage_removed_from_radiant_sentinel_before_life_or_es_%` | A | S1 | 69/1 |  | close-out C-unmodelled-skill: owned by a skill S1 or S2 models |
| `local_display_grants_skill_summon_radiant_sentinel` | A | S8 | 69/1 |  | close-out C-item-skill: item-granted skills (PoB's ExtraSkill) |
| `minion_chance_to_gain_onslaught_on_kill_for_4_seconds_%` | A | S8 | 69/1 |  | close-out C-engine-system: a system the engine lacks; revisited in S8 |
| `minion_maximum_life_%_to_add_as_maximum_energy_shield` | A | S8 | 69/1 |  | close-out C-engine-system: a system the engine lacks; revisited in S8 |
| `raise_spectre_gem_level_+` | A | S8 | 69/1 |  | close-out C-gem-property: PoB's GemProperty, ExtraSupport, LinkedSupport |
| `life_leech_from_minion_damage_%` | C |  | 69/1 |  | game-truth list: PoB applies nothing; needs the maintainer's check in game |
| `additional_physical_damage_reduction_%_per_minion_up_to_10%` | A | S8 | 69/1 |  | close-out C-engine-system: a system the engine lacks; revisited in S8 |
| `gloves_mod_effect_+%` | A | S8 | 68/1 |  | close-out C-engine-system: a system the engine lacks; revisited in S8 |
| `travel_skills_cannot_be_exerted` | A | S8 | 68/1 |  |  |
| `warcry_empowers_next_x_melee_attacks` | A | S8 | 68/1 |  | close-out C-engine-system: a system the engine lacks; revisited in S8 |
| `damage_removed_from_your_nearest_permanent_mercenary_before_life_or_es_%_if_their_current_life_is_higher_than_yours` | C |  | 68/1 |  | game-truth list: PoB applies nothing; needs the maintainer's check in game |
| `warcries_have_minimum_10_power` | A | S8 | 68/1 |  | close-out C-engine-system: a system the engine lacks; revisited in S8 |
| `base_mana_cost_efficiency_+%_of_link_skills` | A | S8 | 68/1 |  | close-out C-engine-system: a system the engine lacks; revisited in S8 |
| `chance_to_gain_adrenaline_for_2_seconds_on_leech_removed_by_filling_unreserved_life_%` | A | S8 | 68/1 |  |  |
| `boots_mod_effect_+%` | A | S8 | 68/1 |  | close-out C-engine-system: a system the engine lacks; revisited in S8 |
| `warcry_grant_X_rage_per_5_power` | A | S5 | 68/1 |  |  |
| `chaos_weakness_ignores_hexproof` | A | S8 | 67/1 |  | close-out C-engine-system: a system the engine lacks; revisited in S8 |
| `enchantment_boots_attack_and_cast_speed_+%_for_4_seconds_on_kill` | C |  | 67/1 |  | game-truth list: PoB applies nothing; needs the maintainer's check in game |
| `additional_max_mirage_archers` | A | S8 | 67/1 |  | close-out C-engine-system: a system the engine lacks; revisited in S8 |
| `cannot_use_flask_in_fifth_slot` | A | S8 | 67/1 |  | close-out C-engine-system: a system the engine lacks; revisited in S8 |
| `toxic_rain_damage_+%` | C |  | 67/1 |  | game-truth list: PoB applies nothing; needs the maintainer's check in game |
| `spellslinger_mana_reservation_+%` | C |  |  | 67/1 | game-truth list: PoB applies nothing; needs the maintainer's check in game |
| `self_take_no_extra_damage_from_critical_strikes_if_have_been_crit_recently` | C |  | 44/2 | 22/1 | game-truth list: PoB applies nothing; needs the maintainer's check in game |
| `support_pierce_projectile_damage_+%_final` | A | S1 |  | 55/1 | close-out C-unmodelled-skill: owned by a skill S1 or S2 models |
| `trigger_socketed_spells_when_you_focus_%` | A | S8 | 54/1 |  | close-out C-gem-property: PoB's GemProperty, ExtraSupport, LinkedSupport |
| `take_half_area_damage_from_hit_%_chance` | C |  | 24/2 | 12/1 | game-truth list: PoB applies nothing; needs the maintainer's check in game |
| `active_skill_minion_physical_damage_+%_final` | C |  | 20/1 |  | game-truth list: PoB applies nothing; needs the maintainer's check in game |
| `support_minion_defensive_stance_minion_damage_taken_+%_final` | A | S1 | 10/1 |  | close-out C-unmodelled-skill: owned by a skill S1 or S2 models |
| `support_minion_defensive_stance_minion_damage_+%_final_against_enemies_near_you` | A | S1 | 10/1 |  | close-out C-unmodelled-skill: owned by a skill S1 or S2 models |
| `support_guardians_blessing_minion_physical_damage_%_of_maximum_life_and_ES_taken_per_minute` | A | S1 | 10/1 |  | close-out C-unmodelled-skill: owned by a skill S1 or S2 models |
| `restore_ward_on_block_%` | C |  | 6/1 |  | game-truth list: PoB applies nothing; needs the maintainer's check in game |
| `local_display_socketed_attacks_mana_cost_+` | C |  | 5/1 |  | game-truth list: PoB applies nothing; needs the maintainer's check in game |
| `base_skill_show_average_damage_instead_of_dps` | A | S8 | 5/1 |  | close-out C-engine-system: a system the engine lacks; revisited in S8 |
| `mana_cost_+%_while_on_full_energy_shield` | C |  | 4/1 |  | game-truth list: PoB applies nothing; needs the maintainer's check in game |
| `manaweave_cost_equals_%_unreserved_mana` | A | S1 | 4/1 |  | close-out C-unmodelled-skill: owned by a skill S1 or S2 models |
| `support_focus_channel_cost_+%_final_per_second_channelling_up_to_100%` | A | S1 | 4/1 |  | close-out C-unmodelled-skill: owned by a skill S1 or S2 models |
| `local_display_spend_energy_shield_for_costs_before_mana_for_socketed_skills` | C |  | 3/1 |  | game-truth list: PoB applies nothing; needs the maintainer's check in game |
| `static_strike_beam_damage_+%_final` | A | S1 | 3/3 |  | close-out C-unmodelled-skill: owned by a skill S1 or S2 models |
| `static_strike_base_zap_frequency_ms` | A | S1 | 3/3 |  | close-out C-unmodelled-skill: owned by a skill S1 or S2 models |
| `minion_grant_puppet_master_buff_to_parent_on_hit_%` | A | S1 | 2/2 |  | close-out C-unmodelled-skill: owned by a skill S1 or S2 models |
| `support_minion_totem_resistance_elemental_damage_+%_final` | C |  | 2/2 |  | game-truth list: PoB applies nothing; needs the maintainer's check in game |
| `minions_inflict_exposure_on_hit_%_chance` | A | S1 | 2/2 |  | close-out C-unmodelled-skill: owned by a skill S1 or S2 models |
| `movement_velocity_+%_while_no_gems_in_boots` | C |  |  | 2/1 | game-truth list: PoB applies nothing; needs the maintainer's check in game |
| `bone_golem_grants_minion_maximum_added_physical_damage` | A | S1 | 1/1 |  | close-out C-unmodelled-skill: owned by a skill S1 or S2 models |
| `bone_golem_damage_per_non_golem_minion_nearby_maximum_%` | A | S1 | 1/1 |  | close-out C-unmodelled-skill: owned by a skill S1 or S2 models |
| `bone_golem_damage_+%_final_per_non_golem_minion_nearby` | A | S1 | 1/1 |  | close-out C-unmodelled-skill: owned by a skill S1 or S2 models |
| `bone_golem_grants_minion_minimum_added_physical_damage` | A | S1 | 1/1 |  | close-out C-unmodelled-skill: owned by a skill S1 or S2 models |
| `support_spell_rapid_fire_repeat_use_damage_+%_final` | A | S1 | 1/1 |  | close-out C-unmodelled-skill: owned by a skill S1 or S2 models |
| `support_anticipation_rapid_fire_count` | A | S1 | 1/1 |  | close-out C-unmodelled-skill: owned by a skill S1 or S2 models |
| `number_of_tornados_allowed` | A | S1 | 1/1 |  | close-out C-unmodelled-skill: owned by a skill S1 or S2 models |
| `tornado_base_damage_interval_ms` | A | S1 | 1/1 |  | close-out C-unmodelled-skill: owned by a skill S1 or S2 models |
| `chance_to_summon_support_ghost_on_killing_blow_%` | A | S1 | 1/1 |  | close-out C-unmodelled-skill: owned by a skill S1 or S2 models |
| `chance_to_summon_support_ghost_on_hitting_rare_or_unique_%` | A | S1 | 1/1 |  | close-out C-unmodelled-skill: owned by a skill S1 or S2 models |
| `support_ghost_base_duration` | A | S1 | 1/1 |  | close-out C-unmodelled-skill: owned by a skill S1 or S2 models |
| `display_max_fire_beam_stacks` | A | S1 | 1/1 |  | close-out C-unmodelled-skill: owned by a skill S1 or S2 models |
| `fire_beam_additional_stack_damage_+%_final` | A | S1 | 1/1 |  | close-out C-unmodelled-skill: owned by a skill S1 or S2 models |
| `base_fire_damage_resistance_%` | A | S1 | 1/1 |  | close-out C-unmodelled-skill: owned by a skill S1 or S2 models |
| `groundslam_damage_to_close_targets_+%_final` | A | S1 | 1/1 |  | close-out C-unmodelled-skill: owned by a skill S1 or S2 models |
| `support_chance_to_bleed_bleeding_damage_+%_final` | A | S1 | 1/1 |  | close-out C-unmodelled-skill: owned by a skill S1 or S2 models |
| `arc_damage_+%_final_for_each_remaining_chain` | C |  | 1/1 |  | game-truth list: PoB applies nothing; needs the maintainer's check in game |
