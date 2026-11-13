# Other gems (Phase 3, step 12)

Every enabled gem but the main skill's was a reason on every field until P3-12h (`unmodelled-gem`, the largest reason left in all 7 in-scope builds). This page records how the engine models them, PR by PR, and the evidence.

## P3-12h: inert skills, reservation, auras and buffs

### What PoB v2.67.2 does

Every loop over `env.player.activeSkillList` in `Modules/Calc*.lua` was read. Another skill reaches the character only through:

| Path | PoB | In this PR |
|---|---|---|
| Which skills exist | `CalcSetup.lua` 1504 to 1740: each group of the active set that is the main group, or is enabled with its slot in the weapon set in use (`slotEnabled`); each enabled gem's effects in order; an effect with a `GlobalEffect` mod only when the gem's `enableGlobal<n>` is on (1652; `enableGlobal1` defaults on, `enableGlobal2` off); the group's supports applied as for the main skill | Modelled (`skill::group_skills`, `poe1::gems`) |
| Groups with a `source` (an item, the tree) | Rebuilt from the source (1340 to 1402) | A reason |
| Reservation | 1928 to 2040; `doActorLifeManaReservation` 526 to 560 | Modelled (below) |
| Buffs, auras | The skill's and its supports' mods with a `GlobalEffect` tag (`CalcActiveSkill.lua` 794 to 859), merged across skills per buff name keeping each mod's largest value (`mergeBuff`, `CalcPerform.lua` 44 to 66), scaled by buff and aura effect, applied unless `effectCond` fails (2256 to 2480) | Modelled for the skills in `SKILLS` (below) |
| By type | Minion counts, totems, `HaveDamageableMinion`, chilling areas, herald and aura counts, `AffectedBy<Buff>` (1320 to 1430, 2112 to 2126, 2272, 2415): named multipliers and conditions only modifiers read | No effect: no mapping row reads a named condition (the row types hold flags only), so a stat that reads one is a reason |
| By name | Warcries' exerted attacks, pacts, Blight and others' stages (2226 to 2252; `CalcOffence.lua` 2543, 2806) | None of the modelled skills is named in any calc module (checked) |
| Curses, debuffs, minions' skills, triggers, brands, mirages | Their own code | A reason (not in the tables) |
| Ailment maxima | `<Ailment>Max` of every skill (3534) | The `gem_effects` check refuses a modelled skill with one |

A gem's quality gains 10% in a socket of its colour (`Classes/SkillsTab.lua` 1210 to 1245). The colour is the granted effect's `Attribute`, which the bundle does not carry, so a modelled gem (the main skill included) in a group whose slot holds an item with a `Sockets` line is `unmodelled-skill` `socket-colour-quality` (Follow-ups).

### The tables

`loom_calc::poe1::gems::SKILLS`, by granted effect ID:
- **Inert** (nothing reaches the character): Freezing Pulse, Frostblink, Frostblink of Wintry Blast, Ground Slam, Holy Flame Totem, Ice Nova, Leap Slam, Shield Charge, Static Strike, Vaal Ice Nova.
- **Buffs**, each with its global stats as PoB maps them:

| Skill | Rows that reach the slice | Rows that reach no slice field |
|---|---|---|
| Precision (aura) | `accuracy_rating` Accuracy BASE; `skill_buff_grants_critical_strike_chance_+%` CritChance INC | |
| Clarity (aura) | | mana regeneration |
| Haste (aura) | attack speed INC (attacks), cast speed INC (casts) | movement speed |
| Discipline (aura) | `base_maximum_energy_shield` EnergyShield BASE | recharge rate |
| Zealotry (aura) | spell crit chance INC, spell damage MORE | |
| Herald of Ice (buff) | added cold damage to attacks and to spells | |
| Blood Rage (buff) | attack speed INC (attacks) | degeneration, leech |
| Arctic Armour (buff) | | freeze immunity, damage taken when hit |

`SUPPORTS`: supports whose stats act on their skill alone (Added Cold Damage, Chance to Bleed, Elemental Army, Elemental Proliferation, Faster Attacks, Fortify, Increased Critical Damage, Increased Critical Strikes, Inspiration, Intensify, Melee Physical Damage, Power Charge On Critical, Ruthless, Shockwave, Trinity, Urgent Orders), and three whose global stats reach no slice field: Infused Channelling (damage taken while channelling), Momentum (movement speed), Summon Phantasm (Phantasmal Might, which applies only with `Condition:PhantasmalMight`, set only by the modifier "Each Summoned Phantasm grants you Phantasmal Might", `ModParser.lua` 4473: its stat `summoned_phantasms_grant_buff` must stay a reason).

A skill not in `SKILLS` is `unmodelled-gem`; so is a support not in `SUPPORTS` applied to a modelled skill. A support that applies to no active skill of its group does nothing, as in PoB.

### The check against PoB

PoB's data export goes to format 4 (`tools/oracle/data/pob_data.lua`): each skill's stat-map mods (its `statMap`, else PoB's `SkillStatMap.lua`) with their tags, its base mods, minions, and a support's added types and minions. The `data-parity` table `gem_effects` compares every table entry the bundle holds with it (`tools/parity-cli/src/data_parity/gem_effects.rs`):
- the stats with a `GlobalEffect` mod equal the table's, and so do the buff's `effectType` and name;
- each slice row equals PoB's mod (name, type, flags as attack, spell, cast, hit, projectile, no `div` or `mult`, no other tag);
- no other mod scales buffs or reservation (`BuffEffect`, `AuraEffect`, `Reserved`, `ReservationEfficiency`, `SupportManaMultiplier`, `ExtraSkillMod`, ...), sets an ailment's maximum, or sets a `SkillData` key other than the skill's own data (base damage, duration, radius, stun, repeat rate, labels);
- no forbidden type (an inert skill: no reservation, buff, aura, herald, hex, brand, warcry, trigger; a buff skill: no minion, totem, warcry, trigger; a support adds none of reservation, buff, aura, herald, totem, trigger but Momentum's `Buff`), no trigger, no minion but Summon Phantasm's phantasm, no support level that sets a percent reservation.

Result: 242 values compared, 0 differences. The first run found 20, each a `SkillData` key of the skill's own data, Holy Flame Totem's `SummonsTotem` type, or Momentum's added `Buff` type; each was read in PoB and allowed with its reason in the check's lists. A maintainer-run test (`the_gem_tables_should_name_real_granted_effects`) checks that every table ID is a granted effect of the real bundle, so none escapes the check.

### Reservation

Per skill with `HasReservation` and not `ReservationBecomesCost` (`CalcPerform.lua` 1940), for each of its level's reservations:
- `mult` is the applied supports' cost multipliers, each `1 + (cost_multiplier - 100) / 100` (PoB's `manaMultiplier` is the game's value less 100) as MORE, floored to 4 decimals twice (`SupportManaMultiplier` is a high-precision mod: `ModList.lua` 139 to 141, then `CalcPerform`'s `floor(.., 4)`);
- flat: `floor(flat x mult)`, then `round(.. x (100 + inc) / 100 x more / (1 + eff / 100) / effMore, 0)`, at least 0;
- percent: `percent x mult`, then the same, rounded to 2 decimals.

`inc`, `more`, and efficiency are 1 and 0: every stat that changes them is a reason (`DEPENDS_ON`, and by name every stat ID with `reservation` or `reserves_no`: `GUARDED_WORDS`). Each skill's amount is a `Base` effect on `Poe1Quantity::Reservation(pool, flat|percent)` with the gem as its source. The unreserved phase computes `max - (flat + ceil(max x percent / 100))` (one `ceil` on the summed percent, as PoB), with a new graph node `Ceil` (calc result v1, additive as P3-12b's rule allows; schema and TypeScript regenerated). A pool no skill reserves keeps its one-term graph.

This follows PoB's model; the game's own rounding of reservation was not tested in game (it would be a register entry if it differs).

### Buffs

A modelled buff skill's instance stats (and its supports'), at the gem's level (raised by the global gem level modifiers since P3-8i) and quality, give the table's rows; across skills each stat keeps its largest value per buff name (two Herald of Ice gems give one buff), and the rows act on the character with the gem as source. PoB applies buffs after attributes, life, and mana (`CalcPerform.lua` 1905 against 3284): a test checks no buff row targets them. Buff and aura effect stay reasons (`DEPENDS_ON`, `GUARDED_WORDS`: `aura_effect`, `buff_effect`, gem levels and quality), so every buff acts at a scale of exactly 1, where PoB's `ScaleAddList` adds the mods unchanged.

The main skill keeps its own path (its stats through `SkillMapping`); a main skill that reserves is a reason.

### Mechanic fixtures

| Fixture | What it isolates |
|---|---|
| `smite-other-gems-templar.xml` | Smite on a Karui Maul with an act 1 setup's other gems: inert skills and supports, Precision and Clarity at level 2, a level 1 Precision the level 2 one outvalues |
| `smite-blood-rage-haste-templar.xml` | Blood Rage, Haste (50% reserved), Arctic Armour; a Haste with `enableGlobal1` off, which PoB does not create |
| `ice-nova-zealotry-discipline-witch.xml` | Zealotry and Discipline (percent reservations, energy shield, more spell damage, spell crit) |
| `ice-nova-herald-of-ice-witch.xml` | Herald of Ice twice: level 20 with Increased Critical Strikes (25% x 1.2 = 30%), level 5 alone (25%); one buff, two reservations |

### Evidence

Maintainer-run (`local/p3/p3-12h/evidence.sh`, P3-12g's bundle `local/p3/p3-12g/bundle-poe1-a`, unchanged: this PR adds no game data), calc runs from clean `0d04119`; `data-parity` from clean `f336006` (the review tightened the check after the calc runs; the engine is the same). Summaries in `docs/phase-3/other-gems/`.

- **PoB data export** (format 4): two runs byte-identical.
- **`data-parity`**: 534 differences, `uncategorized` 0 (unchanged from P3-12g); the new `gem_effects` table compares 255 values with 0 differences; two runs identical; the record `docs/phase-3/data-parity-poe1-26530219.json` replaced. The review's stricter check found one more case: Summon Phantasm adds `CreatesMinion` to its skill (its phantasm, already allowed), allowed by the same named-condition argument.
- **Oracle**: 104 fixtures, every expectation met, 3 runs each.
- **`calc-parity`** (`p3-12h-public-calc-parity-summary.json`): 33 registered differences (DIV-0001 to DIV-0033, unchanged), 0 unexpected, no unused entry; 1380 fields match; every mechanic target met, 1370 of 1370 over 67 fixtures, the four new ones on all their targets (the first quick run matched them too: Herald of Ice needed no `game-truth` entry).
- **Calc runs**: public and corpus documents byte-identical over two runs; round trips pass (public and corpus).
- **Against P3-12g** (`p3-12h-public-vs-p3-12g.json`): no field state changed; 42 values in 16 public builds changed, every one toward PoB's (6 now equal it, 36 closer, none farther); 354 reasons gone and 26 added in 28 builds (the added ones: 23 `socket-colour-quality`, and 3 `unmodelled-gem` for Arcane Surge in the Ice Nova builds' main group, which also supports the Vaal gem's other effect, Vaal Ice Nova; PoB gives its buff through that skill too, and the buff is not modelled yet). Corpus: 5 values in 5 builds (unreserved mana), 143 reasons gone, 8 added.
- **In-scope builds**: `unmodelled-gem` reasons fell from 65 to 20 (both campaign Ice Nova builds), 17 to 10 (Ice Nova mapping), and for Smite act1, act2, act5, act10 from 10, 11, 16, 17 to 0, 4, 8, 8. **Smite act1 has no gem reason left**: its average damage, crit chance, hit chance, and unreserved mana now equal PoB's; its only reasons are the two custom lines ("Perfect Forest Warrior", Q13; "Onslaught", ambiguous), and its speed differs by the Onslaught line and Q11.
- 1195 workspace tests, clippy, fmt, wasm32; the maintainer-run real-bundle tests (every `DEPENDS_ON` stat and every table ID in the real bundle).

### Follow-ups

- **Socket colour quality** (done in P3-12i, below): needs the granted effect's `Attribute` in the bundle (extractor and bundle), then a gem's quality +10 in a socket of its colour, for the main skill too. The maintainer confirmed the mechanic (2026-11-13): since 3.29 a gem of any colour fits any socket (before, a gem needed a socket of its colour or a white one), and a matching colour adds quality to the gem. A white socket gives no bonus (maintainer, 2026-11-13), as in PoB, where only an item whose sockets always match (Dialla's Malefaction) gives it whatever the colour.
- **Other gems** of the in-scope builds: golems (the minion's buff, the Lightning Golem's Wrath), curses (Frostbite, Elemental Weakness), Frost Bomb's exposure, warcries (Intimidating Cry, Rallying Cry) and their exerted attacks, Herald of Ash (gain as extra), Herald of Purity (a minion), Flesh and Stone (stances), Righteous Fire, Automation, Convocation, Guardian's Blessing, Arcane Surge and Elemental Army's buffs.
- **Buff and aura effect, reservation modifiers**: map them with the scale PoB applies (`ScaleAddMod`'s rounding) and the reservation terms. Partly done in P3-8l (`docs/phase-3/stat-mapping.md`, "P3-8l"): reservation efficiency, "Reservation of Skills", and the effect of non-curse auras, with `ScaleAddMod`'s rounding, the main skill's own aura (Smite's) included. Left: "Mana (Life) Reservation of Skills" (PoB tags them `Aura`), buff effect on you, a skill's own effect and reservation stats, the MORE ones.
- **"Perfect Forest Warrior"** in the four Smite builds' active custom block (Q13, context file).

## P3-12i: socket colour quality

**The game** (maintainer, in game, 2026-11-13): since 3.29 a gem of any colour fits any socket; a gem in a socket of its own colour gains quality; a white socket gives none.

### What PoB v2.67.2 does

- `Classes/SkillsTab.lua` 1210 to 1245 (`UpdateSocketGroups`): for each socket group of the active skill set, in list order, enabled or not, that has a slot, gem `i` sits in socket `offset + i`. `offset` counts every gem of the earlier groups in the same slot, disabled gems and gems PoB does not know included. A gem PoB knows matches when the slot's item has `colourAlwaysMatches`, or when the socket at its index has the colour of its granted effect's `color` (1 `R`, 2 `G`, 3 `B`; `W`, `A`, and any other `color` never match).
- `Classes/Item.lua` 658 to 666: the `Sockets:` line gives one socket per letter of `RGBWA`, in order (a space starts a new link group; the index runs on). Lines 1482 to 1490: an item without the line gets its base's `socketLimit` sockets of a default colour. `socketLimit` is PoB's own `#socketLimit` directive, not game data. Lines 2588 to 2635: "Has # Sockets", "Has # Abyssal Sockets", "Has no Sockets", and "Cannot have non-Abyssal sockets" rebuild the list. Line 2636: "Gems Socketed always have the Quality bonus from Socket Colour" sets `colourAlwaysMatches`.
- `Modules/CalcSetup.lua` 1578 (supports) and 1654 (active skills): the instance's quality is the gem's plus `data.misc.MatchingSocketQualityBonus` (10, `Modules/Data.lua` 297) when it matches. The bonus has no cap. `CalcActiveSkill.lua` 545 to 547 also adds a `GemSocketQuality` mod, which only the displayed gem quality reads (not a slice field).

### Game data

| Change | Where |
|---|---|
| The granted effect's colour: `GrantedEffect.attribute`, the game's `GrantedEffects.Attribute` as stored (1 to 4), optional (PoE2 has no column; an older bundle has no field) | `loom_data::bundle::skills`, converter `tools/parity-cli/src/bundle/skills.rs` (required in the PoE1 layout, so a renamed column fails the run) |
| The validator: an attribute other than 1 to 4 is `BUNDLE_VALUE_RANGE` | `crates/loom-data/src/bundle/validate.rs` |
| PoB's data export, format 5: each skill's `color` | `tools/oracle/data/pob_data.lua` |
| `data-parity`, table `skills`, field `color`: the bundle's attribute against PoB's `color`, which PoB's exporter copies from the same column (`src/Export/Scripts/skills.lua` 157) | `tools/parity-cli/src/data_parity/compare.rs` |

The extract already held the column (the extractor writes every column), so P3-12g's extract was converted again; no new extraction was needed.

### The engine

`loom_calc::poe1::sockets::socket_matches` follows PoB's rule for each group of the active set and each of its gems. The gem's colour is that of its first granted effect, which is PoB's `grantedEffect`, else `gemData.grantedEffect`. The item is the last entry of the slot's name in the active item set; an empty slot or `itemId="0"` gives no match. `skill::main_skill` and `skill::group_skills` add 10 to a matching gem's instance quality, so the main skill's hit, its supports, and every other skill's buff stats read it.

What the engine cannot know is `SocketMatch::Unknown`. The gem keeps its own quality, and a gem of the main skill or of a buff (its active gem or an applied support) is `unmodelled-skill` `socket-colour-quality`; an inert skill's quality reaches nothing, nor does a reservation (it reads the level's row only). The unknown cases:

- `no-sockets-line`: PoB would add `socketLimit` sockets of a default colour. It can over-report: a base without a limit (a ring, an amulet, a belt) has no sockets in PoB, so no match;
- `several-sockets-lines`;
- `socket-count-line`: a line that rebuilds the sockets (`REBUILDS`: `local_has_X_sockets`, `local_has_X_abyss_sockets`, `local_has_no_sockets`, `local_cannot_have_non_abyssal_sockets`);
- `untyped-gem` (untyped gem content in a group of the slot: the index is not certain), `untyped-item`;
- the item set's untyped content is not a case: it is also PoB's jewel-socket entries (`SocketIdURL`), and `collect` reads the typed slots only too. The first evidence run treated it as unknown, which left every gem of 14 public builds a reason; the fix (`76ec05e`) came before the calc runs below.
- `no-colour`: the bundle has no colour for the gem (an older bundle), and its socket is red, green, or blue (a white, abyssal, or missing socket gives no match whatever the colour).

"Gems Socketed always have the Quality bonus from Socket Colour" (`local_can_socket_gems_ignoring_colour`) makes every gem of the item match and joins `NO_EFFECT` (category `sockets`): its only effect is the gems' quality. A test checks that it and `local_has_X_abyss_sockets` are `sockets` rows and that the other `REBUILDS` stats stay unmapped.

A gem's quality now reaches stats it did not before. Melee Physical Damage's quality stat `melee_physical_damage_+%` became a hit row (PoB `SkillStatMap.lua` 776: `PhysicalDamage` INC with `ModFlag.Melee`).

### Mechanic fixtures

| Fixture | What it isolates |
|---|---|
| `ice-nova-socket-colours-witch.xml` | A Plate Vest socketed `B-B-R-W G-B`: Ice Nova and Controlled Destruction in blue sockets (+10 each: Controlled Destruction's 5% spell damage), Increased Critical Strikes (blue) in a red socket and Faster Casting in a white one (none); a second group in the same slot from socket 5: Precision in green (+10), Haste (green) in blue (none) |
| `smite-socket-colours-templar.xml` | A Karui Maul (1000 ms: no Q11 difference) socketed `R-R-G`: Smite, Melee Physical Damage (5% increased melee physical damage), Faster Attacks (5% increased attack speed), each in a socket of its colour |

The first run of both matched PoB on every target once `melee_physical_damage_+%` was mapped.

Unit tests (`crates/loom-calc/tests/hit.rs`) cover matching, other colours, white, missing, and abyssal sockets; the offset across groups of one slot, with a disabled gem, a disabled group, a gem the data does not have, and other slots; the always-matching line, also beside "Has no Sockets" and on an unselected variant line (not part of the item); each unknown case (`no-sockets-line`, `several-sockets-lines`, each `REBUILDS` stat, `untyped-gem` for its group and the later ones of the slot, `untyped-item`, `no-colour`) and its reason; the last entry of a slot name; and an empty slot. A maintainer-run test (`real_gems_should_have_pobs_colours`) checks that every granted effect of the real bundle has a colour from 1 to 4 and that the fixtures' gems have PoB's.

### Review

`code-architecture-reviewer`: no blockers; every PoB claim confirmed (the offset rule, which gems count and which effect gives the colour, the last slot entry, the always-matching flag after the rebuilds, abyssal sockets, the quality in both setup paths). Applied: `no-colour` only where the socket's colour would decide it (`69b3cb1`); the tests above for the cases it listed; the notes on `no-sockets-line` over-reporting and on reservation. Not changed: `socket_matches` runs per `main_skill` call and once in `gems` (a few allocations); a set entry whose slot name PoB does not have (PoB ignores it; a malformed build).

### Evidence

Maintainer-run, 2026-11-13, Windows. `local/p3/p3-12i/evidence.sh` (tool commit `a80df66`, clean): the bundle twice, the PoB export twice, `data-parity` twice, `bundle-check`, the oracle run, the round trips. `local/p3/p3-12i/evidence-calc.sh` (tool commit `69b3cb1`, clean): the calc runs and `calc-parity` again after the fixes above (the `untyped-slot` case and the review). Summaries in `docs/phase-3/other-gems/p3-12i-*`.

| Run | Result |
|---|---|
| `bundle` twice from P3-12g's extract (`local/p3/p3-12g/extract-poe1`) | Byte-identical, records too. Against P3-12g's bundle only `bundle.json` and `data/granted_effects.json` differ. The new local bundle is `local/p3/p3-12i/bundle-poe1-a` (manifest SHA-256 `ab807d33…`); the record `docs/phase-3/bundle-poe1-26530219.json` is replaced. Colours: 784 strength, 1,073 dexterity, 1,679 intelligence, 9,293 none |
| `bundle-check` | Exit 0, no findings |
| `pob_data.lua` twice (format 5) | Byte-identical; the oracle checkout stayed clean |
| `data-parity` twice | Byte-identical. Table `skills` compares 1,508 more values (`color`), 0 differences; in total 534 differences, `uncategorized` 0, 314 overlay rows, as before. The record `docs/phase-3/data-parity-poe1-26530219.json` is replaced (`docs/phase-3/data-parity.md`, "P3-12i") |
| `oracle-run`, the whole manifest | 106 fixtures, every expectation met, 3/3 identical runs each |
| `calc-run`, public, twice | 106 documents (105 builds and `malformed.xml`, refused as expected); 1449 complete, 334 partial, 425 unsupported, 312 indeterminate. Byte-identical twice |
| `calc-parity`, public, with the overlay run, the register, and the manifest | 33 differences, all registered (DIV-0001 to DIV-0033, unchanged), 0 unexpected, no unused entry; 1425 fields match. **Every mechanic target met: 1415 of 1415 over 69 fixtures**, the two new ones on all their targets (22 and 23). Exit 5 (`incomplete`), as before |
| `calc-run` and `calc-parity`, corpus (21, `--anonymous`), twice | Byte-identical; no difference (exit 5) |
| `build-round-trip`, public and corpus | 106 and 21 builds pass |

**Against P3-12h** (`p3-12i-public-vs-p3-12h.json`, `p3-12i-corpus-poe1-26530219-vs-p3-12h.json`). No field state changes. 13 values in 5 public builds change, every one closer to PoB's: Boneshatter, Elemental Hit, Kinetic Blast, Lacerate, and Shock Nova (average damage, hit DPS; speed for Elemental Hit and Shock Nova; crit multiplier for Kinetic Blast), through their gems' socket quality and the melee physical damage row. 47 reasons gone in 18 public builds, none added: every `socket-colour-quality` (23, in 14 builds; the in-scope Ice Nova mapping build and Smite act5 and act10 among them) and 24 `melee_physical_damage_+%`. Corpus: no value change; 15 reasons gone (8 `socket-colour-quality`, 7 `melee_physical_damage_+%`), 1 added: `projectile_damage_+%`, a support's quality stat that a matching socket now gives a value.

Tests: 1202 workspace tests, clippy, fmt, wasm32; the maintainer-run real-bundle tests with `LOOM_BUNDLE_DIR=local/p3/p3-12i/bundle-poe1-a` (`real_gems_should_have_pobs_colours` new; the enemy, skill, and parser ones pass).

### Follow-ups

- An item without a `Sockets` line would need PoB's `socketLimit` per base, which is PoB's directive, not game data; it stays a reason.
- The custom "Onslaught" line (next step), the other gems left (above), and the in-scope builds' unmapped stats.
