# Phase 6 report: PoE1 mechanics expansion

Phase 6 is the PoE1 mechanics expansion with the stat mop-up (`dev/active/phase-6-mechanics/`). The maintainer asked for it on 2026-11-17: "do phase 6 in full without stopping. We can do gate testing after."

This report is P6-12's: the counts, the register, the gate checklist, and the performance budgets re-checked. It was first written at PR #167's commit `f366c65` (P6-11n). The counts table and the register are updated to P6-11az (`fd5f3ff`); the reasons table is P6-11ac's (`a1676a8`); the Phase 3 and Phase 6 set figures and the performance record are still P6-11n's.

**Verdict: the gate is not met yet** (the close-out's verdict is in "The close-out (2026-11-24)").
- Gate item 1 holds: every `Complete` field equals PoB's or is registered.
- Item 3 holds for the public builds but not for the corpus, where no field is `Complete` yet.
- Item 2's closed list is below. The steps still open are listed under "Still open".

## The counts

The P6-11ac counts are from `local/p6/p6-11ac/`, made by `sh local/p6/evidence.sh p6-11ac` on `local/p6/packs/b-ot` (the evidence bundle with the tree packs 3_25 to 3_28). The P6-11az counts are from `local/p6/dv/` on `local/p6/packs/b-ct`, which adds data schema 5, the cluster, Timeless, and tattoo tables, and the stats' scalability (`docs/phase-6/timeless.md`, `docs/phase-6/tattoos.md`, `docs/phase-6/stat-mop-up.md`, "P6-11ax").

| Set | Builds | Fields `Complete` | Partial | Unsupported | Indeterminate | calc-parity unexpected |
|---|---|---|---|---|---|---|
| Public, start (P6-0b, 2026-11-17) | 136 | 2,270 | 386 | 482 | 102 | 0 |
| Public, at P6-11n | 210 (209 calculate; one is the expected failure) | 13,547 | 1,925 | 812 | 443 | 0 |
| Public, at P6-11ac | 212 (211 calculate; one is the expected failure) | 13,953 | 1,669 | 816 | 447 | 0 |
| Public, at P6-11az | 253 (252 calculate; one is the expected failure) | 17,146 | 1,658 | 888 | 460 | 0 |
| Public, at P6-11cz | 308 (307 calculate; one is the expected failure) | 21,531 | 1,486 | 1,027 | 604 | 0 |
| Public, at P6-11dj | 315 (314 calculate; one is the expected failure) | 22,078 | 1,486 | 1,037 | 604 | 0 |
| Public, at P6-11do | 320 (319 calculate; one is the expected failure) | 22,470 | 1,486 | 1,043 | 603 | 0 |
| Public, now (P6-11ee, 2026-11-20) | 334 (333 calculate; one is the expected failure) | 23,565 | 1,486 | 1,061 | 603 | 0 |
| Corpus, start | 21 | 0 | 193 | 141 | 170 | 0 |
| Corpus, at P6-11ac | 21 | 0 | 791 | 124 | 749 | 0 |
| Corpus, at P6-11az | 21 | 0 | 804 | 118 | 749 | 0 |
| Corpus, now (P6-11ee) | 21 | 0 | 804 | 118 | 750 | 0 |

- **The Phase 3 set** (the 24 fields every document carries; at P6-11n):
  - Public: 3,090 `Complete` of 4,180, against 2,270 of 3,240 at the start. The new fixtures are mostly complete builds.
  - Corpus: 0 of 420.
- **The Phase 6 set** (slice revisions 3 to 22, each field present when it applies; at P6-11n):
  - Public: 10,457 `Complete`.
  - Every computed field that is complete equals PoB's or is registered.
- **Public calc-parity:** 300 expected differences, 0 unexpected (P6-11az); 376 expected, 0 unexpected (P6-11cz); 400 expected, 0 unexpected (P6-11dj); 423 expected, 0 unexpected (P6-11do); 482 expected, 0 unexpected (P6-11ee). Fields matching PoB: 13,535 at P6-11w, 13,651 at P6-11ac, 16,822 at P6-11az, 21,109 at P6-11cz, 21,633 at P6-11dj, 22,003 at P6-11do, 23,045 at P6-11ee (`local/p6/ee/`, same bundle).

## The register

`docs/parity/divergence-register.json` holds 483 entries at P6-11ee (480 `game-truth`; DIV-0302 to DIV-0483 are Q11 entries of the later attack fixtures). At P6-11az it held 301 entries: 298 `game-truth`, 2 `pob-bug` (DIV-0294 and DIV-0295, PoB's per-second rounding of an Abyss jewel's per-minute rolls, `docs/phase-6/timeless.md`, "Part 4"), and 1 retired `pob-bug` (DIV-0282 to DIV-0293, P6-11aj's Multistrike fixtures, and DIV-0296 to DIV-0301, P6-11ay's bow fixture, are Q11 entries too).
- **Q11 entries (Phase 6 adds DIV-0102 to DIV-0293, except DIV-0111 to DIV-0113 and DIV-0174):** the game floors a weapon's attack time to whole milliseconds (Q11, answered in game 2026-11-12). Each attack fixture's `Speed` difference is carried into the fields that read it:
  - the hit's DPS;
  - combined and full DPS;
  - triggers;
  - the mirage;
  - the costs per second;
  - leech.

  A field is registered only where the build's `Speed` entry is.
- **DIV-0111 to DIV-0113 and DIV-0174:** Vaal Summon Skeletons' minion level. The game's `base_display_minion_actor_level` is 68 at gem level 20; PoB uses the level requirement, 70. Confirmation in game is a maintainer item.

## The gate checklist

1. **Every `Complete` field matches PoB or has a registered, evidenced divergence.** Met: 0 unexpected differences, public and corpus.
2. **No build leaves a field `Unsupported` for a mechanic marked done; the remaining reasons form a closed list.** The list is below. Every `Unsupported` field is either:
   - a mechanic still open (a dual wielder's ailments and leech, traps', mines', and totems' own rates, conversion smoothing in maximum hit, the config inputs below); or
   - not computed for the build's main skill (no hit, no skill).
3. **The `Complete` counts rise.**
   - Public: met, 2,270 to 17,146 (P6-11az).
   - Corpus: not met, still 0. Every corpus build carries unmapped stats and unmodelled gems: 21 of 21 builds for each.

### The remaining reasons (the closed list)

Counted once per build, on the `life` field's reasons in all three of its lists (unsupported, indeterminate, partial); a build-wide reason reaches every field. Public 211 builds, corpus 21 (P6-11ac):

| Reason | Public builds | Corpus builds | What it is |
|---|---|---|---|
| `unmapped-stat` | 26 | 21 | 929 distinct stats across both sets, on any field. These include the field markers (`EhpReason`, `PoolReason`, ...), which are reasons by design. The rest is the P6-11 tail: heralds, marks, brands, sigils, resistance conversions, Eldritch Battery, Eternal Youth, granted notables, guards and Molten Shell, and many uniques' own lines. |
| `unmodelled-gem` | 26 | 21 | A gem of the active set the engine does not model: Frostblink, Second Wind, Toxic Rain, Reflection, Sniper's Mark, Immortal Call, Flame Wall, Spellslinger, Eternal Blessing, Raise Spectre, and others (the rest of P6-9), and the supports linked to them. |
| `unmodelled-config` | 14 (28 inputs) | 12 (16 inputs) | Config inputs no step models yet: the minion's own charges and conditions, brands attached, Feeding Frenzy, Alchemist's Genius, ground effects on the enemy, Toxic Rain and Storm Rain overlaps. |
| `unresolved-line` | 22 | 12 | Item or tree lines the resolver cannot settle: no template, ambiguous, or no variant fits (ADR-015). |
| `unmodelled-passive` | 21 | 12 | Cluster jewel nodes: the evidence bundle has no cluster tables, which wait for the maintainer's extraction. Also unconnected nodes and unresolved tree lines. |
| `tree-version` | 5 | 10 | A build saved on a tree the bundle does not carry (3_13, 3_15, 3_22, 3_23, 3_26 alternate). |
| `scaled-line`, `excluded-line`, `unread-line`, `unsettled-value` | 12, 10, 1, 1 | 4, 5, 0, 0 | Item lines PoB itself scales or skips (P3-8b). |
| `unmodelled-item` | 9 | 6 | Abyssal sockets, flask bases, ambiguous bases, flask merges, a jewel limit, a slot. |
| `unmodelled-skill` | 5 | 2 | Socket colour quality, a reservation, Rallying Cry's allies, a shield requirement. |
| `unmodelled-condition` | 4 | 3 | A passive's condition not settled. |
| `unhandled-kind` | 1 | 1 | An item property. |
| `missing-data` | 0 | 1 | A passive the bundle lacks. |

The field-specific reasons (each on its own fields only) are listed in each step's document:
- `docs/phase-6/recovery.md`: costs, regeneration, leech, the defence estimate's markers `EnemyHitReason`, `PoolReason`, `DotTakenReason`, `EhpReason`;
- `docs/phase-6/triggers.md`, `mirages.md`, `minions.md`, `gems.md`.

## The close-out (2026-11-24)

The maintainer asked (2026-11-24) for the rest of Phase 6 in one session: the stat mop-up only, the engineering items deferred (`dev/active/phase-6-mechanics/p6-close-out-plan.md`). The 450 stats still unmapped after P6-11ik were each put in one batch (`p6-close-out-list.md`) and worked one commit per batch on PR #167: M1 to M7, H, and N (`docs/phase-6/stat-mop-up.md`, "Close-out M1" to "Close-out N").

**Final counts** (`local/p6/n/`, bundle `local/p6/packs/b-ct`, 2026-11-24):

| Set | Builds | Fields `Complete` | Partial | Unsupported | Indeterminate | calc-parity unexpected |
|---|---|---|---|---|---|---|
| Public, at P6-11ik | 509 | 37,038 | | | | 0 |
| Public, after the close-out | 531 (530 calculate; one is the expected failure) | 38,757 | 1,535 | 1,420 | 680 | 0 |
| Corpus, after the close-out | 21 | 0 | 804 | 111 | 762 | 0 |

- **Public calc-parity:** 682 expected differences, 0 unexpected; 38,043 fields match PoB.
- **Distinct `unmapped-stat` reasons** in the public builds and the corpus: 450 at P6-11ik, 278 after the close-out. The 278 are the closed reasons and the game-truth list below, and the field markers' stats.
- **Register:** 683 entries (678 `game-truth`, 5 `pob-bug`, 1 retired); the close-out added the Q11 entries DIV-0653 to DIV-0683 for its attack fixtures.
- **Fixtures:** the close-out added 22 mechanic fixtures, each matching PoB on every computed field (the Smite ones but for the weapon's rate).
- Every batch's public and corpus runs: 0 unexpected. Each value that moved away from PoB is explained in its batch's section: builds already off PoB for other reasons, where PoB applies the new line too.

### The closed reasons, each stat

Each stays a reason (`unmapped-stat`, or the gem's or skill's reason) until a later phase models its system.

- **`C-engine-system`** (70; a system the engine lacks, recorded in the batch that moved it): `additional_max_mirage_archers`, `additional_maximum_all_elemental_resistances_%_if_all_equipment_grants_armour`, `additional_physical_damage_reduction_%_per_minion_up_to_10%`, `additional_poison_chance_%_when_inflicting_poison`, `additive_mana_modifiers_apply_to_shock_effect_at_30%_value`, `ailment_types_apply_damage_taken_+%`, `all_damage_can_ignite`, `alternate_ascendancy_herald_buff_effect_+%_final_per_1%_mana_reserved`, `alternate_ascendancy_herald_minion_and_herald_damage_+%_final_per_1%_life_reserved`, `anger_aura_effect_+%_while_at_maximum_endurance_charges`, `attacks_chance_to_poison_%_on_max_frenzy_charges`, `base_mana_cost_efficiency_+%_of_link_skills`, `base_skill_show_average_damage_instead_of_dps`, `boots_mod_effect_+%`, `cannot_use_flask_in_fifth_slot`, `chaos_weakness_ignores_hexproof`, `critical_strikes_do_not_always_ignite`, `crits_have_culling_strike`, `determination_aura_effect_+%_while_at_minimum_endurance_charges`, `dual_wield_inherent_bonuses_are_doubled`, `elusive_effect_+%`, `elusive_effect_on_self_+%_per_power_charge`, `empowered_attack_damage_+%`, `enemies_in_presence_and_you_count_as_moving_when_affected_by_elemental_ailments`, `enemies_near_corpses_created_recently_are_shocked_and_chilled`, `enemies_shocked_by_you_have_physical_damage_%_converted_to_lightning`, `enemies_taunted_by_your_warcies_are_intimidated`, `exposure_you_inflict_has_minimum_resistance_%`, `extra_damage_rolls_with_lightning_damage_on_non_critical_hits`, `gain_affliction_charges_instead_of_frenzy_charges`, `gain_alchemists_genius_on_flask_use_%`, `gain_debilitating_presence_ms_on_kill_vs_rare_or_unique_enemy`, `gain_wand_accuracy_rating_equal_to_intelligence`, `gloves_mod_effect_+%`, `hinder_enemy_chaos_damage_taken_+%`, `life_leech_does_not_stop_at_full_life`, `local_display_socketed_gems_have_mana_reservation_+%`, `local_double_damage_with_attacks`, `local_energy_shield`, `local_flask_recovery_amount_%_to_recover_instantly`, `local_item_quality_+`, `local_nearby_enemies_base_physical_damage_%_to_convert_to_fire`, `local_unique_jewel_passive_jewel_socket_mod_effect_+%_with_corrupted_magic_jewel_socketed`, `mana_%_to_add_as_energy_shield`, `maximum_affliction_charges_is_equal_to_maximum_frenzy_charges`, `mine_aura_effect_+%`, `minion_chance_to_gain_onslaught_on_kill_for_4_seconds_%`, `minion_maximum_life_%_to_add_as_maximum_energy_shield`, `modifiers_to_minimum_frenzy_charges_instead_apply_to_affliction_charges`, `non_curse_aura_effect_+%_per_10_devotion`, `non_damaging_ailment_effect_+%`, `num_magic_utility_flasks_always_apply`, `number_of_chains`, `player_far_shot`, `projectile_damage_+%_per_chain`, `quiver_mod_effect_+%`, `raider_shock_maximum_effect_override_%`, `raider_shocks_you_apply_can_stack_up_to_50_times`, `sabotuer_mines_apply_damage_taken_+%_to_nearby_enemies_up_to_10%`, `shock_effect_+%`, `shock_maximum_magnitude_+`, `spell_damage_+%_per_100_max_life`, `spell_suppression_chance_%_if_all_equipment_grants_evasion`, `support_additional_trap_mine_%_chance_for_1_additional_trap_mine`, `unique_helmet_cast_speed_+%_applies_to_attack_speed_at_%_of_original_value`, `warcries_exert_twice_as_many_attacks`, `warcries_have_infinite_power`, `warcries_have_minimum_10_power`, `warcry_empowers_next_x_melee_attacks`, `your_auras_except_malevolence_are_disabled`.
- **`C-item-skill`** (13; skills granted by items, PoB's `ExtraSkill` and kin): `curse_on_hit_level_conductivity`, `curse_on_hit_level_frostbite`, `curse_on_hit_level_vulnerability`, `local_display_grants_skill_accuracy_crits_aura_level`, `local_display_grants_skill_barkskin`, `local_display_grants_skill_enduring_cry_level`, `local_display_grants_skill_herald_of_the_breach_level`, `local_display_grants_skill_summon_radiant_sentinel`, `local_display_raise_spider_on_kill_%_chance`, `local_display_trigger_level_x_reflection_skill_on_equip`, `local_unique_flask_nearby_enemies_cursed_with_level_x_vulnerability_during_flask_effect`, `spell_damage_%_suppressed_per_bark_below_max`, `spell_damage_taken_+_per_barkskin_stack`.
- **`C-unmodelled-skill`** (101; text only in a skill's or gem's description, or lines owned by skills the engine does not model): `active_skill_bleeding_damage_+%_final`, `active_skill_shock_as_though_damage_+%_final`, `added_physical_damage_to_attacks_equal_to_%_maximum_mana`, `archmage_gain_lightning_damage_%_of_max_unreserved_mana`, `attack_maximum_added_cold_damage_for_elemental_hit`, `attack_maximum_added_fire_damage_for_elemental_hit`, `attack_maximum_added_lightning_damage_for_elemental_hit`, `attack_minimum_added_cold_damage_for_elemental_hit`, `attack_minimum_added_fire_damage_for_elemental_hit`, `attack_minimum_added_lightning_damage_for_elemental_hit`, `base_fire_damage_resistance_%`, `base_movement_velocity_+%`, `base_number_of_arrows`, `base_spell_cooldown_speed_+%`, `blood_spears_damage_+%_final_in_blood_stance`, `bone_golem_damage_+%_final_per_non_golem_minion_nearby`, `bone_golem_damage_per_non_golem_minion_nearby_maximum_%`, `bone_golem_grants_minion_maximum_added_physical_damage`, `bone_golem_grants_minion_minimum_added_physical_damage`, `boneshatter_trauma_base_duration_ms`, `chance_to_summon_support_ghost_on_hitting_rare_or_unique_%`, `chance_to_summon_support_ghost_on_killing_blow_%`, `corpse_explosion_monster_life_permillage_fire`, `damage_removed_from_radiant_sentinel_before_life_or_es_%`, `display_frost_fury_additive_cast_speed_modifiers_apply_to_fire_speed`, `display_max_fire_beam_stacks`, `display_what_freezing_pulse_does`, `elemental_hit_damage_+%_final_per_enemy_elemental_ailment`, `elemental_hit_no_physical_chaos_damage`, `fire_beam_additional_stack_damage_+%_final`, `frenzy_skill_attack_damage_+%_final_per_frenzy_charge`, `frenzy_skill_attack_speed_+%_final_per_frenzy_charge`, `frost_fury_base_fire_interval_ms`, `frost_fury_duration_+%_per_stage`, `frost_fury_fire_speed_+%_final_while_channelling`, `frost_fury_fire_speed_+%_per_stage`, `frost_fury_max_number_of_stages`, `groundslam_damage_to_close_targets_+%_final`, `hits_grant_cruelty`, `kinetic_blast_modifiers_to_number_of_projectiles_instead_apply_to_number_of_clusters`, `kinetic_fusillade_base_delay_between_projectiles_ms`, `kinetic_fusillade_damage_+%_final_per_projectile_fired`, `kinetic_fusillade_maximum_floating_projectiles`, `local_left_ring_socketed_curse_replaces_skitterbots_chilling_aura`, `local_right_ring_socketed_curse_replaces_skitterbots_shocking_aura`, `lose_all_trauma_at_X_trauma`, `manaweave_cost_equals_%_unreserved_mana`, `minion_grant_puppet_master_buff_to_parent_on_hit_%`, `minions_inflict_exposure_on_hit_%_chance`, `number_of_tornados_allowed`, `number_of_warcries_exerting_this_action`, `primary_projectile_chains_+`, `projectiles_barrage`, `projectiles_cannot_split`, `reduce_enemy_chaos_resistance_%`, `reduce_enemy_elemental_resistance_%`, `self_bleed_duration_+%`, `skill_area_of_effect_+%_final_in_sand_stance`, `skill_can_only_use_non_melee_weapons`, `skill_used_by_sacred_wisp_damage_+%_final`, `static_strike_base_zap_frequency_ms`, `static_strike_beam_damage_+%_final`, `summon_fire_skitterbot`, `summon_sacred_wisps_on_hit`, `summoned_skitterbots_auras_also_affect_you`, `summoned_skitterbots_non_damaging_ailment_effect_+%`, `support_additional_trap_mine_%_chance_for_2_additional_trap_mine`, `support_additional_trap_mine_%_chance_for_3_additional_trap_mine`, `support_anticipation_rapid_fire_count`, `support_barrage_attack_time_+%_per_projectile_fired`, `support_barrage_damage_+%_final`, `support_barrage_trap_and_mine_throwing_time_+%_final_per_projectile_fired`, `support_better_ailments_ailment_damage_+%_final`, `support_better_ailments_hit_damage_+%_final`, `support_blunt_chance_to_trigger_shockwave_on_hit_%`, `support_chance_to_bleed_bleeding_damage_+%_final`, `support_cruelty_hit_damage_+%_final`, `support_damage_while_on_full_life_+%_final`, `support_focus_channel_cost_+%_final_per_second_channelling_up_to_100%`, `support_focus_channel_damage_+%_final_per_second_channelling_up_to_60%`, `support_fork_projectile_damage_+%_final`, `support_ghost_base_duration`, `support_greater_volley_projectile_damage_+%_final`, `support_guardians_blessing_minion_physical_damage_%_of_maximum_life_and_ES_taken_per_minute`, `support_ignite_prolif_ignite_damage_+%_final`, `support_lesser_multiple_projectile_damage_+%_final`, `support_maim_chance_physical_damage_+%_final`, `support_minion_defensive_stance_minion_damage_+%_final_against_enemies_near_you`, `support_minion_defensive_stance_minion_damage_taken_+%_final`, `support_multiple_projectile_damage_+%_final`, `support_parallel_projectile_number_of_points_per_side`, `support_pierce_projectile_damage_+%_final`, `support_reduced_duration_damage_+%_final`, `support_spell_rapid_fire_repeat_use_damage_+%_final`, `support_spiritual_cry_damage_+%_final`, `support_unbound_ailments_ailment_damage_+%_final`, `throw_traps_in_circle_radius`, `tornado_base_damage_interval_ms`, `trauma_strike_damage_+%_final_per_trauma_capped`, `trauma_strike_self_damage_per_trauma`, `triggered_by_spiritual_cry`.
- **`C-keystone`** (6; keystones whose rule is not ported): `keystone_corrupted_defences`, `keystone_crimson_dance`, `keystone_precise_technique`, `keystone_quiet_might`, `keystone_tinctures_drain_life`, `keystone_wicked_ward`.
- **`C-gem-property`** (7; PoB's `GemProperty`, `ExtraSupport`, `LinkedSupport`): `all_skill_gem_quality_+`, `local_shield_trigger_socketed_elemental_spell_on_block_display_cooldown`, `local_socketed_curse_gem_level_+`, `local_socketed_support_gem_quality_+`, `raise_spectre_gem_level_+`, `support_gems_socketed_in_off_hand_also_support_main_hand_skills`, `trigger_socketed_spells_when_you_focus_%`.
- **`C-no-slice`** (2; explosions on kill, which PoB reads into no slice field): `enemies_explode_on_death_by_attack_for_10%_life_as_physical_damage`, `local_unique_flask_explode_enemies_for_10%_life_as_random_element_on_kill_chance_%_during_flask_effect`.

### Deferred out of Phase 6 (maintainer 2026-11-24)

Each stays a named reason:
- Spellslinger; Blood and Sand, with PoB's full DPS pass (`spellslinger_mana_reservation_efficiency_+%`, `spellslinger_mana_reservation_+%`);
- mine, trap, and totem costs per second;
- General's Cry's mirage warriors (`generals_cry_maximum_warriors_+`), Sacred Wisps;
- the other trigger configs: Arcanist Brand, Mark on Hit, Hextouch, Cast on Death, item triggers (`trigger_socketed_spell_on_skill_use_%`);
- spectres (`global_defences_+%_per_raised_spectre`);
- ward's other lines (`restore_ward_on_block_%`);
- EHP's other pools, P6-10b's further parts (`missing_unreserved_life_%_gained_as_life_before_hit_per_defiance`, `take_half_area_damage_from_hit_%_chance`);
- Skitterbots (a data run);
- the minion's own damage over time.

### The maintainer's game-truth list

Lines PoB applies nothing for (it leaves them unparsed, never sets their condition, or reads their multiplier on the wrong actor) that change a slice field in the game. Each stays a reason; a check in the game would let a later step map it with a `game-truth` register entry (68 at the close-out; 67 since the maintainer's in-game check of 2026-11-26 found that PoB applies Energy From Within's line, now mapped, `docs/phase-6/stat-mop-up.md`, "The maintainer's in-game checks"): `active_skill_minion_physical_damage_+%_final`, `aggravate_bleeding_on_exerted_attack_hit_chance_%`, `apply_maximum_wither_stacks_%_chance_when_you_apply_withered`, `arc_damage_+%_final_for_each_remaining_chain`, `armour_from_helmet_and_gloves_+%`, `attack_and_cast_speed_+%_with_physical_skills`, `base_aggravate_bleeding_on_attack_hit_chance_%`, `base_bone_golem_granted_buff_effect_+%`, `blood_spears_base_number_of_spears`, `body_armour_defences_doubled_while_no_gems_in_body_armour`, `consecrated_ground_while_stationary_radius_if_highest_attribute_is_strength`, `critical_strike_chance_+%_per_mine_detonated_recently_up_to_100%`, `critical_strike_chance_+%_vs_enemies_with_lightning_exposure`, `critical_strike_multiplier_+_per_mine_detonated_recently_up_to_40`, `curse_on_hit_level_despair`, `damage_over_time_+%_per_100_max_life`, `damage_removed_from_your_nearest_permanent_mercenary_before_life_or_es_%_if_their_current_life_is_higher_than_yours`, `detonate_dead_%_chance_to_detonate_additional_corpse`, `elemental_damage_+%_per_different_enemy_elemental_ailment`, `elemental_resistance_%_while_no_gems_in_helmet`, `enchantment_boots_attack_and_cast_speed_+%_for_4_seconds_on_kill`, `enemy_phys_reduction_%_penalty_vs_hit`, `generals_cry_maximum_warriors_+`, `ice_spear_second_form_critical_strike_chance_+%`, `ice_spear_second_form_critical_strike_multiplier_+`, `ice_spear_second_form_projectile_speed_+%_final`, `impale_effect_+%_for_impales_inflicted_by_two_handed_weapons_on_non_impaled_enemies`, `intimidating_cry_cooldown_speed_+%`, `keystone_natures_presence`, `keystone_soul_tether`, `life_leech_from_minion_damage_%`, `local_accuracy_rating_+%_per_2%_quality`, `local_critical_strike_chance_+%_per_4%_quality`, `local_display_socketed_attacks_mana_cost_+`, `local_display_spend_energy_shield_for_costs_before_mana_for_socketed_skills`, `local_flask_chance_to_freeze_shock_ignite_%_while_healing`, `local_flask_recover_instantly_when_on_low_life`, `local_maximum_life_per_2%_quality`, `local_quality_does_not_increase_defences`, `local_unique_jewel_freezing_pulse_damage_+%_if_enemy_shattered_recently_with_50_int_in_radius`, `local_unique_jewel_freezing_pulse_number_of_additional_projectiles_with_50_int_in_radius`, `local_unique_jewel_spectres_gain_soul_eater_on_kill_%_chance_with_50_int_in_radius`, `magic_utility_flasks_cannot_be_used`, `mana_cost_+%_while_on_full_energy_shield`, `maximum_life_+%_while_no_gems_in_gloves`, `mine_%_chance_to_detonate_twice`, `mine_detonation_speed_+%`, `minion_damage_over_time_multiplier_+_per_minion_abyss_jewel_up_to_+30`, `movement_velocity_+%_while_no_gems_in_boots`, `necromancer_mana_regeneration_rate_per_minute_for_you_and_allies_per_nearby_corpse`, `non_chilled_enemies_you_bleed_are_chilled`, `projectile_damage_+%_per_remaining_chain`, `random_skill_gem_level_+_index`, `random_skill_gem_level_+_level`, `restore_ward_on_block_%`, `self_critical_strike_multiplier_-%_per_endurance_charge`, `self_take_no_extra_damage_from_critical_strikes_if_have_been_crit_recently`, `spell_critical_strikes_bifurcate`, `spellslinger_mana_reservation_+%`, `support_minion_totem_resistance_elemental_damage_+%_final`, `tailwind_if_have_crit_recently`, `take_half_area_damage_from_hit_%_chance`, `thrown_shield_secondary_projectile_damage_+%_final`, `totems_nearby_enemies_damage_taken_+%`, `toxic_rain_damage_+%`, `trap_%_chance_to_trigger_twice`, `trigger_socketed_spell_on_skill_use_%`.

**Open item (not game truth):** `regenerate_%_maximum_mana_over_2_seconds_on_consuming_corpse`. PoB's `PercentStat` of `Mana` reads a value above the shown maximum (5% of "Regenerate 10% of Mana over 2 seconds" adds 30.0 at 590 mana, 35.0 at 690, about 3.0 at 56; "Regenerate 2% of Mana per second" adds exactly 2% of 590). **Resolved in open items S10 (2026-11-24):** `PercentStat` ceils each line (`ModStore.lua` 532), and the stat is mapped as PoB does (`docs/phase-6/stat-mop-up.md`, "Open items S10").

**Verdict after the close-out:** the gate is the maintainer's. Item 1 holds (0 unexpected, public and corpus). Item 2's closed list is above, with the deferred items. Item 3 holds for the public builds (38,757 `Complete`); the corpus still has no `Complete` field, every build carrying a closed reason.

## Performance (P6-12: the budgets re-checked)

`loom-bench wall --runs 20` on `local/p6/packs/b-ot` with `forged-frostbearer-spectre-necromancer.xml` (release build, 2026-11-18):

| Scenario | Median | Slowest | Phase 5 record |
|---|---|---|---|
| `validate` | 607.1 ms | 625.2 ms | 585.0 ms |
| `load` | 651.0 ms | 677.2 ms | 726.2 ms |
| `start` | 780.7 ms | 800.3 ms | 994.4 ms |
| `import` | 18.0 ms | 20.7 ms | 28.8 ms |
| `calculate` | 64.9 ms | 69.3 ms | 6.7 ms |
| `allocate` | 0.6 ms | 0.9 ms | 1.0 ms |

**Every budget is still met** (ADR-013, reference tier):
- import and first calculation, 83 ms of 1.5 s;
- an item-edit preview, 65 ms of 300 ms;
- an allocation preview, 66 ms of 150 ms.

**`calculate` is ten times slower.** Full DPS (P6-12 part 2) computes each skill included in it as the main skill: this build has several. The defence estimate's hits-to-die adds a little.
- **The CI instruction-count check** (`tools/bench/baseline.json`, measured on CI's runner) will report the rise. The baseline needs a new record from that job, as PR #135 did for the tree's bench bundle: a maintainer item.
- **Caching** a skill's sub-calculation across previews is the obvious later step, if the preview budgets tighten.

## Done in Phase 6

Each step's record is in the tasks file (`dev/active/phase-6-mechanics/phase-6-mechanics-tasks.md`) and its document in `docs/phase-6/`. In brief:
- **P6-1 to P6-6:** damage over time, ailments, dual wielding, flasks, channelling, Vaal skills, totems, traps, mines, brands.
- **P6-7:** minions: data, the actor, hits, ailments, combined DPS.
- **P6-8:** triggers and the mirage.
- **P6-9:** gems: auras, curses, marks, guards, warcries, heralds, summons.
- **P6-10a:** the timeless extraction roots.
- **P6-10b:**
  - regeneration, recharge, movement, costs (mana, life, energy shield, per second);
  - base damage reduction;
  - the leech caps and rates;
  - damage taken over time;
  - the evade chances;
  - the enemy's hit;
  - maximum hit;
  - the effective health pool.
- **P6-11:** the stat mop-up batches a to ac (`docs/phase-6/stat-mop-up.md`). Since P6-11n:
  - P6-11o to P6-11s: lines while an aura affects you, Onslaught on kill, socketed gem levels, supports an item grants, and damage per aura or herald;
  - P6-11t: regeneration per charge and per stat, and channelling mana costs;
  - P6-11u: durations, projectile counts, and chains as `NoEffect` with guards;
  - P6-11v: nearby enemies;
  - P6-11w: burning ground;
  - P6-11x: Flesh and Stone's Sand Stance, and fortification rounded with the other hit MOREs;
  - P6-11y: Momentum's Swiftness, and the main skill's support buffs left to `poe1::gems`;
  - P6-11z: Arctic Armour;
  - P6-11aa: Infused Channelling;
  - P6-11ab: Rapid Decay and Swift Affliction;
  - P6-11ac: the eldritch block implicit.
- **P6-12:** combined and full DPS, and this report.

## Still open

**On this branch** (no maintainer needed):
- **The defence estimate:**
  - ward, aegis, a type's own guard, allies' pools, life loss prevention, and recoup in the pools (the shared guard since P6-11ae, the low life and low energy shield checks since P6-11ad, gains on block since P6-11ak);
  - damage over time EHP;
  - conversion smoothing in maximum hit.
- **A dual wielder's** ailments and leech.
- **The remaining gems** (P6-9) and config inputs (P6-8a's Spellslinger, Arcanist Brand, Mark on Hit, Hextouch, Cast on Death, item triggers; General's Cry's mirages; Sacred Wisps).
- **The minion's** damage over time and spectres (P6-7c3).
- **The P6-11 tail:** 929 distinct stats, including the intended field markers.
- **Repeats' other modes:** `FINAL` and `FINAL_DPS` (P6-11aj modelled the default `AVERAGE` and `NONE`).

**Maintainer items:**
- **Data runs:**
  - the timeless jewel data run (Q-P6-1 decided 2026-11-19: PoB's lookup tables, checked consistent with Vilsol's implementation; build 26530219's files are now on this workstation; `docs/phase-6/timeless.md`);
  - shipping the cluster jewel tables (made from build 26530219's extract on 2026-11-19, evidence bundle `b-cl`, data-parity `uncategorized` 0; `docs/tree/cluster-jewels.md`);
  - `VaalSouls` for soul costs;
  - the relic's Nova skill level requirements.
- **Confirmation in game:** Vaal Summon Skeletons' minion level.
- **The bench baseline** after this PR's `calculate` rise.
- **Shipping** `poe1/minions.json` and `poe1/flasks.json` in the signed bundle.
- **The gate run itself**, as the maintainer asked: "We can do gate testing after."

## Open items (2026-11-24)

The maintainer asked (2026-11-24) for every open item an agent can finish, with whatever needs the maintainer stored as a follow-up. Plan, inventory, and tasks: `dev/active/open-items/`; the maintainer's list: `dev/active/open-items/maintainer-follow-ups.md`. One PR, #168, a commit per step.

**Counts** (bundle `local/p6/packs/b-ct`; baseline `dev` `6deeb5da`, `local/p6/n`; final run `local/p6/s2`):

| | Baseline | After the open items |
|---|---|---|
| Public builds | 531 | 535 (4 new mechanic fixtures) |
| Public fields `Complete` | 38,757 | 39,232 (+475: 171 on existing builds, 304 the new fixtures) |
| Public calc-parity | 682 expected, 0 unexpected | 698 expected, 0 unexpected |
| Corpus fields `Complete` | 0 | 0 (every corpus build keeps a class C or data reason) |
| Corpus field-reason pairs | 10,537 | 9,644 (-893) |
| Public field-reason pairs | 24,890 | 23,874 |
| Distinct reasons (public and corpus) | 344 | 328 (16 cleared) |
| Distinct `unmapped-stat` | 278 | 275 |

Every step's public and corpus runs: 0 unexpected, both runs byte-identical; every value that moved, moved toward PoB.

**The steps:**

- **S0:** every distinct reason classed once (`open-items-inventory.json`, `.md`; `local/p6/tools/inventory_check.py` fails on a reason missing from it): 275 agent (A), 69 maintainer (C: the 68 game-truth stats and the older item wordings under `no-template`), data run (B) only as gem parts (Skitterbots). Fixture budget: 75 for the session (about 14.8 s each on the Windows oracle job); 4 used.
- **S1** (`docs/phase-6/skill-data.md`, `totems.md`, "Open items S1"): the vetting rule's read lists (Shield Crush, Vortex, the DoT's flags); `reservation-becomes-cost` only when a reservation reaches the cost; a Vaal gem's `enableGlobal` flags in the full DPS; `totem-data` narrowed to Siege Ballista of Trarthus, a totem's bleed, and its DoT; no leech field for a placed skill's hit.
- **S8** (`docs/phase-6/config-inputs.md`, "Open items S8"): Storm Rain's and Toxic Rain's overlap inputs as skill data only; `ActiveBrands`, Alchemist's Genius, and the three grounds in `NAMED_ONLY`; Unholy Might, a moving enemy's bleed, scorch, sap, and crush modelled; two stats `NoEffect` (their lines PoB applies no mod of).
- **S10** (`docs/phase-6/stat-mop-up.md`, "Open items S10"): the close-out's open item explained: PoB's `PercentStat` ceils each line (`ModStore.lua` 532), and the stat is mapped as PoB does.
- **S2** (`docs/phase-6/gems.md`, "Open items S2"): every candidate gem checked against PoB's export, calc modules, and `data-parity`; two supports added, nine refused with their blockers recorded.

**Register:** DIV-0684 to DIV-0699, all Q11 under the confirmed rule (the Ballista fixtures' Short Bow and the S8 Smite fixture's Rusted Hatchet, 667 ms); no `game-truth` or `pob-bug` entry; `divergence-check` passes (698 accepted).

**New fixtures:** `ice-nova-s8-unholy-might-withered-witch.xml`, `smite-s8-bleed-moving-enemy-templar.xml`, `ice-nova-s8-scorch-sap-crushed-witch.xml`, `ice-nova-s10-mana-regen-on-corpse-witch.xml`, each matching PoB on every computed field but for the Smite fixture's Q11 rate.

**Every reason left, by class** (the final run's 328, each in the inventory; `inventory_check.py` passes):

| Class | Step | Reasons | What |
|---|---|---|---|
| A | S1 | 114 | `pob-skill-data` skills and the skill kinds, and the close-out's `C-unmodelled-skill` stats owned by them |
| A | S8 | 112 | the close-out's closed reasons (`C-engine-system`, `C-item-skill`, `C-keystone`, `C-gem-property`, `C-no-slice`), the remaining config inputs (brittle, the minion inputs, Feeding Frenzy, Elemental Equilibrium, ...), `NoLifeModifiersOnBodyArmour` |
| A | S9 | 20 | resolver and item gaps: `ambiguous`, `no-variant-fits`, `only-shadowed`, scaled and excluded lines, item kinds, tree lines, cluster and timeless nodes |
| A | S6 | 7 | spectres, the minion inputs |
| A | S7 | 3 | ward and EHP lines, a dual wielder's block |
| A | S2, S3, S5 | 1 each | `unmodelled-gem` (its parts in the gem table), Spellslinger and item triggers, warcry power |
| C | none | 68 | the game-truth list (each needs a check in the game) |
| C | S9 | 1 | `unresolved-line` `no-template`: older item wordings (an ADR-015 amendment and an additive `Adapted` kind) and note text |

**Not reached** (planned as stretch, or the rest of a step; `maintainer-follow-ups.md`, "Planned for the session, may not be reached"): S3 (triggers and Spellslinger), S4 (Blood and Sand, PoB's full DPS pass), S5 (costs per second, warcry power, mirages, wisps), S6 (minion damage over time, spectres), S7 (ward, EHP's other pools, maximum hit, degeneration, a dual wielder's leech), S9 (the resolver and item gaps), and the rest of S1, S2, and S8 as their task lines say.
