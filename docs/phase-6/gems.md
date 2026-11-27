# P6-9: the remaining gems

Plan step P6-9: the gems the engine leaves `unmodelled-gem`, a reason on every field of a build that has one. They are other groups' skills and the supports applied to them (`loom_calc::poe1::gems`, P3-12h), plus marks, curse priority, exerts, banners, and heralds.

## How a gem is added

The method is P3-12h's:
- A skill or support goes into `gems::SKILLS` or `gems::SUPPORTS` with its kind and its buff rows.
- Every entry is checked against PoB v2.67.2's skill data by the data-parity table `gem_effects`:
  - its `GlobalEffect` stats are exactly the table's;
  - each row has PoB's mod name, type, and flags;
  - no other mod scales buffs or reservation;
  - its types fit its kind.
- Any difference counts as `uncategorized`.

**Finding candidates:** `local/p6/classify_gems.py` reads PoB's export (format 13) for each gem that is `unmodelled-gem` in the public builds or the corpus, ranked by count (`local/p6/gem_names.py`). A gem is an inert candidate when it has no `GlobalEffect` mod, no mod the table blocks, no `SkillData` key outside its display keys, and no type its kind forbids.

## P6-9a: inert skills, plain supports, and nine auras

**Inert active skills (15).** None has a global effect, so as another group's skill it gives the character nothing. Its reservation, if any, is still computed:
- Autoexertion, Blast Rain, Burning Arrow, Divine Ire, Earthshatter, Frenzy, Detonate Mines, Ice Spear, Poisonous Concoction of Bouncing, Portal, Searing Bond, Spectral Shield Throw, Spellslinger, Sunder, Vaal Flicker Strike.

**Plain supports (43).** No row. A support applied to another group's skill acts only on that skill:
- Brutality, Controlled Destruction, Greater Multiple Projectiles, Melee Physical Damage, Vicious Projectiles, Void Manipulation, and their Awakened versions;
- Barrage, Burning Damage, Combustion, Concentrated Effect, Cooldown Recovery, Culling Strike, Damage on Full Life, Efficacy, Elemental Damage with Attacks, Elemental Focus, Empower, Enhance, Enlighten, Faster Casting, Feeding Frenzy, Focused Ballista, Hextouch, Hypothermia, Ice Bite, Immolate, Increased Area of Effect, Lifetap, Multiple Totems, Pierce, Pulverise, Second Wind, Spell Cascade, Spell Echo, Swift Affliction, Swift Assembly, Unleash, Volatility, Withering Touch.
- A support applied to the main skill acts through its stats on the skill (`SkillMapping`), as before.

**Auras (9), as PoB's `GlobalEffect` mods:**

| Aura | Rows |
|---|---|
| Anger | added fire damage to attacks (`ModFlag.Attack` here, as the mapping's attack rows) and to spells |
| Wrath | added lightning damage to attacks; `LightningDamage` MORE for spells |
| Determination | `Armour` BASE and MORE |
| Grace | `Evasion` BASE and MORE |
| Purity of Elements | `ElementalResist` BASE (all three); its ailment immunity reaches no field |
| Purity of Fire, Ice, Lightning | the type's resistance and its maximum, BASE |
| Vitality | its life regeneration reaches no field |

They are scaled, merged, and reserved as the other auras are, and they reach the minion too (P6-7b4).

**The parity tool:** `gem_effects` now:
- names the character's resistances and maximums in PoB's form;
- writes a typed more with other flags (`LightningDamage` MORE `[spell]`) without the type's flag;
- allows `ElementalAilmentImmune` as a mod no field reads.

**Evidence:**
- **`data-parity`, table `gem_effects`:** 894 values compared, 0 differences.
- **Fixtures:** three Ice Nova Witch builds, each with two auras in their own groups. All 67 computed fields are `Complete` and equal to PoB, reservation included:
  - `ice-nova-anger-purity-of-elements-witch.xml`;
  - `ice-nova-determination-purity-of-fire-witch.xml`;
  - `ice-nova-grace-purity-of-ice-witch.xml`.
- **`unmodelled-gem` reasons:** in the public builds, 262 to 248; in the corpus, 306 to 264.
- **Corpus:** 39 values moved toward PoB, none away, and 11 are now equal to PoB. Those include fire and cold resistances, unreserved mana, and a minion's armour from Determination.
- **Public `calc-parity`:** 166 builds, 109 expected differences, 0 unexpected.
- Both runs are byte-identical.

Note for fixtures: PoB writes the game's gem item as `gemId` (Purity of Elements is `SkillGemPurity`, Purity of Fire `SkillGemFireResistAura`). PoB's export key (`SkillGemPurityOfElements`) does not resolve in the bundle.

## P6-9b: hexes and PoB's curse slots

**Curse slots** (`gems::curse_slots`, `curse_priority`; `CalcPerform.lua` 3155 to 3212, `determineCursePriority` 563 to 593):
- **The limit:** 1, plus `number_of_additional_curses_allowed` (PoB's `EnemyCurseLimit` BASE, `SkillStatMap.lua` 1317). That stat leaves `gems::DEPENDS_ON` and maps to `Poe1Quantity::CurseLimit`; a conditional one is a `curse-limit` reason.
- **The walk:** the curses, in the skill list's order, take the slots as PoB fills them. Each takes the first free slot, or the slot of a curse of its name when its priority is higher. Otherwise it takes the last slot holding a lower priority.
- **The priority:** PoB's base table (`Modules/Data.lua` 322 to 335), plus 100 per socket position (at most 8), plus the group's item slot, plus 20,000 for an aura.
- **Not modelled:** a group granted by an item (PoB's 11,000) is skipped by `gems`. Curses that ignore the limit, and marks, come from stats the engine does not map, so they stay reasons.
- Before this step, a second hex of another name was a `curse-limit` reason on every field.

**A curse's own effect:** a hex gem's own `curse_effect_+%` (Despair's, a hex's quality) is PoB's `CurseEffect` INC on the skill's own list. The curse scale now adds it to the character's: `Totals::curse_scale_with`.

**Hexes (6):**

| Hex | Rows on the enemy |
|---|---|
| Despair | chaos resistance; `SelfChaosMin` and `SelfChaosMax`, added damage of every hit (`CalcOffence.lua` 3263) |
| Flammability | fire resistance; `SelfIgniteDuration` INC (`mult = -1`); `SelfIgniteChance`, added to every hit's chance to ignite (4108) |
| Vulnerability | `PhysicalDamageTaken` INC; `SelfBleedChance` (4097); `SelfBleedDuration` INC |
| Conductivity | lightning resistance; its shock chance and duration reach no field |
| Enfeeble | none: the enemy's accuracy and damage reach no field |
| Temporal Chains | none: the enemy's action speed and buff duration reach no field |

Punishment is left `unmodelled-gem`: its damage taken applies only while the enemy is on low life (PoB's enemy `Condition:LowLife`).

**The parity tool:**
- `gem_effects` names a curse's or debuff's added damage, ailment chances, and ailment durations PoB's way (`Self...`).
- It allows the enemy-only names in `ENEMY_NO_SLICE_NAMES` for a curse's `NoSlice` stats.

**Evidence:**
- **`gem_effects`:** 954 values compared, 0 differences.
- **Fixtures:** all 44 computed fields are `Complete` and equal to PoB:
  - `ice-nova-curse-priority-witch.xml`: Elemental Weakness and Despair with one slot, so Despair is kept;
  - `ice-nova-curse-slots-witch.xml`: Elemental Weakness, Frostbite, and Despair with two slots, so Despair replaces Frostbite.
- **`unmodelled-gem` reasons:** in the public builds, 231 to 218; in the corpus, 246 to 234.
- **Corpus:** 3 values moved toward PoB, none away.
- **Public `calc-parity`:** 168 builds, 109 expected differences, 0 unexpected.

## P6-9c: the summoning skills

A summoning skill's minions act through the minion step. For the character, such a skill is inert when neither it nor any of its minions' skills has a `GlobalEffect` mod. A minion's skills come from PoB's minion data and templates, and `local/p6/minion_skill_globals.py` checks them in PoB's export. Of the 87 summoning skills in PoB, only Summon Elemental Relic's minion has auras (Anger, Hatred).

**Inert (20):**
- Absolution, Animate Guardian, Animate Weapon, Dominating Blow, Raise Zombie, Summon Raging Spirit, and Summon Skeletons, with their PoB variants;
- Vaal Summon Skeletons, Holy Strike, Summon Holy Relic of Conviction, and Summon Void Spawn.

Each of these is in `gem_effects`' `ALLOWED_MINIONS`, the list of skills whose minions are known. Vaal Summon Skeletons' variants of Mages and of Archers are not in PoB v2.67.2's data, so they cannot be checked and stay `unmodelled-gem`.

**Minion supports with no row (6):**
- Minion Damage and its Awakened version, Minion Speed, Infernal Legion, Predator, and Minion Pact.
- Minion Life and Meat Shield were in already.
- A support's minion stats reach the minion through the minion step (P6-7b3b, `gem_modifiers`) when its skill is the main skill.

**Vaal Summon Skeletons' level:**
- The game's `base_display_minion_actor_level` at gem level 20 is 68. PoB takes the gem level's requirement, 70.
- With the gem modelled, the skeleton's fields are `Complete`. Its life, armour, and evasion are registered `game-truth` divergences (DIV-0111 to DIV-0113), with fixture `minion-vaal-summon-skeletons-witch.xml`.
- Confirmation in game (the skeletons' level) is a maintainer item.

**Evidence:**
- **`gem_effects`:** 1,110 values compared, 0 differences.
- **The minion sweep (52 builds):** 109 minion fields `Complete` and equal to PoB, or registered. That is 40 before P6-9c. 13 builds have every minion field `Complete`.
- **Fixtures (`Complete` and equal to PoB):** the Raise Zombie build with minion lines (P6-7b3b, local until now), and the sweep's Raise Zombie, Summon Skeletons, and Summon Raging Spirit.
- **Public `calc-parity`:** 173 builds, 112 expected differences (the three new entries), 0 unexpected. `Complete` fields: 3,124.

## P6-9d: skills with their own data, and four buffs

**A skill's own data:** PoB reads these `SkillData` keys only from the skill's own `skillData` (checked in `CalcOffence.lua`), never another skill's. `gem_effects` now counts them as the skill's own keys:
- base damage and damage over time: `FireMin`, `FireMax`, `LightningMin`, `LightningMax`, `ChaosMin`, `ChaosMax`, `ChaosDot`, `ColdDot`, `PhysicalDot`;
- timing: `damageInterval`, `hitFrequency`, `fixedCastTime`;
- corpse explosion data: `explodeCorpse`, `corpseExplosionLifeMultiplier`, `hitChanceIsExplodeChance`;
- `dpsMultiplier`, `dotIsProjectile`.

16 skills are then inert as another group's skill: Flame Dash, Detonate Dead's corpse explosion (`EnemyExplode`), Desecrate, Dash, Toxic Rain, Shield Crush, Tornado, Volatile Dead, Vaal Detonate Dead, Orb of Storms, Fire Trap, Storm Rain, Double Strike, Essence Drain, Contagion, and Rolling Magma.

**Buffs:**

| Skill | Rows |
|---|---|
| Hatred (aura) | `PhysicalDamageGainAsCold` BASE, `Poe1Quantity::GainAs` |
| Herald of Thunder (buff) | added lightning damage to attacks and spells |
| Malevolence (aura) | `Damage` MORE with `ModFlag.Dot` (`Flags::DOT`); its skill effect duration reaches no field |
| Enduring Cry (warcry) | none reaches a slice field (regeneration, warcry power) |

`gem_effects` reads `ModFlag.Dot` as `dot`, and allows `Duration` and `LifeRegenPercent` as names no field reads.

**Not yet, with why:**
- Tempest Shield: PoB disables it without a shield, which the other-gems table does not check yet.
- Pride: its effect is PoB's `modCond` from the config.
- The marks: they need the mark slot rule.
- General's and Battlemage's Cry: mirage warriors, warcry power.
- The offerings and Bone and Carrion golems: minion buffs with their own effect stats.

**Evidence:**
- **`gem_effects`:** 1,237 values compared, 0 differences.
- **Fixture:** `ice-nova-herald-of-thunder-enlighten-witch.xml` (Herald of Thunder and Purity of Ice with Enlighten 3, and Hatred). All 22 computed fields are `Complete` and equal to PoB, reservation included.
- **Corpus:** 8 values moved toward PoB and 5 away. Three builds' unreserved mana went further below PoB's: the newly modelled reserving skills add reservation that PoB lowers with modifiers the engine does not read yet (those fields stay `Partial`). The fixture shows the skills' own reservation equal to PoB.

## P6-9e: marks, Pride, Punishment, guards, offerings, two golems, and supports

**Marks** (Assassin's, Sniper's, and Poacher's Mark) are curses typed `Mark`. PoB puts them in the same curse slots as hexes, with one extra rule (`CalcPerform.lua` 3162 to 3212, "Currently assume only 1 mark is possible"):
- while a mark holds a slot, no other mark takes one, even a free one;
- a mark replaced by a curse of higher priority frees the rule.

`gems::curse_slots` ports this. Tests: `curse_slots_should_hold_one_mark`, `curse_slots_should_free_the_mark_rule_when_a_mark_is_replaced`.

| Skill | Rows |
|---|---|
| Assassin's Mark | the enemy's `SelfCritChance` (stat / 100) and `SelfCritMultiplier` BASE: a hit adds them to its crit chance's base and its extra damage (`CalcOffence.lua` 3015, 3125), so they are the character's `CritChance` and `CritMultiplier` sums, as Despair's added damage is |
| Sniper's Mark | the enemy's `ProjectileDamageTaken` INC: new `Poe1Quantity::EnemyProjectileDamageTaken`, which only a hit with the projectile flag adds to `takenInc` (3482); damage over time never reads it (a damage-over-time context can carry the projectile flag, so a conditioned `EnemyDamageTaken` row would be wrong) |
| Poacher's Mark | the enemy's `PhysicalDamageReduction` BASE: new `Poe1Quantity::EnemyPhysicalDamageReduction`, added to the armour reduction of a physical hit before its floor at 0 (3434) and alone for physical damage over time (4457); and added physical damage taken with `ModFlag.Hit` |

**Pride** is an `AuraDebuff` on the enemy. Its two stats are chosen by PoB's `modCond`, which only the config sets: `PrideMaxEffect` from `prideEffect` = `MAX` (`ConfigOptions.lua` 566), else `PrideMinEffect` (`CalcSetup.lua` 539). `CalcActiveSkill.lua` 806 drops a mod whose `modCond` fails. The new `BuffStat::ConfigIs` reads the list input; `prideEffect` is now one of `gems::is_input`'s inputs. Pride is scaled as an aura, as Flesh and Stone is. Its `DebuffEffect` term is a reason (`DEPENDS_ON`).

**Punishment**'s damage taken applies while the enemy is on low life (a `Condition` tag on the mod). The new `BuffStat::WhenState` carries the state into the curse's rows. `conditionEnemyLowLife` moved from `config::NAMED_ONLY` to `conditions::CHECKS` (`Enemy:LowLife`).

**Guards** (`EffectType::Guard`): Molten Shell, Vaal Molten Shell, Steelskin. PoB's guard slots (`CalcPerform.lua` 3256 to 3275):
- Vaal Molten Shell alone;
- else every Vaal guard and one other.

With two or more non-Vaal guards, PoB keeps the first in its table's iteration order, which is not defined. So none of them applies and each is an `unmodelled-skill` `guard-slot` reason (`gems::guard_pick`, test `guard_pick_should_follow_pobs_slots`). PoB scales a guard by the character's buff effect alone, with no gem stat.

**Tempest Shield:** spell block and shock immunity. A skill typed `RequiresShield` is disabled without a shield in the off hand (`CalcActiveSkill.lua` 269 to 273): no buff, and its supports' buffs neither. It still reserves, because PoB's reservation loop (1939) does not check `disable`. The fixture without a shield shows the reservation equal to PoB. An unknown off hand is a `requires-shield` reason.

**Offerings** (Bone and Flesh Offering): `buffNotPlayer` with `buffMinions`. The new `BuffStat::MinionOnly` adds rows in `minion_buffs` only. `offering_spells_effect_+%` joins `OWN_EFFECT_STATS` (PoB's `BuffEffect` INC on the skill).

**Golems:** Summon Chaos Golem (chaos resistance, damage over time multiplier) and Summon Carrion Golem. The Carrion Golem's added damage goes to your other minions, which compute no hit yet (P6-7c); its stats are `NoSlice("minion-buff")` until then. Neither golem's minion skills have a `GlobalEffect` mod.

**General's Cry:** a warcry whose only global mod is its warcry power flag. Its mirage warriors act only for a skill it triggers (P6-8b).

**Supports:** 17 plain supports, classified by `classify_gems.py` (no global or blocked mod):
- Awakened Empower, Cruelty, Deadly Ailments, Endurance Charge on Melee Stun, Fire Penetration, Fist of War;
- Greater Chain, Greater Multiple Projectiles, Greater Multistrike, Greater Volley, Ignite Proliferation, Impale;
- Invert the Rules, Melee Splash, Multistrike, Overexertion, Trap and Mine Damage.

**Not yet, with why:**
- Battlemage's, Ancestral, Infernal, and Seismic Cry: their rows (crit chance, resistances, conversion, armour) scale with warcry power, which is not modelled.
- Arrogance and Eternal Blessing: reservation as life, and aura effect.
- Cast when Damage Taken, Mirage Archer, Manaforged Arrows, Mark on Hit: triggers (P6-8a, P6-8b).
- Summon Skitterbots: its aura chills, and its minions' level is the character's (a data run).
- Herald of the Hive, Blood and Sand, banners, brands, Phase Run, Berserk, Frost Shield, Vaal auras, Wave of Conviction, Flame Surge, Cyclone, and others: each has its own buff or data.

**Evidence:**
- **`gem_effects`:** 1,476 values compared, 0 differences.
- **Fixtures (7), all equal to PoB or registered (Q11, DIV-0114 to DIV-0122):**
  - `ice-nova-marks-witch.xml`;
  - `burning-arrow-snipers-mark-templar.xml`;
  - `smite-poachers-mark-pride-punishment-templar.xml`;
  - `smite-pride-initial-templar.xml`;
  - `ice-nova-guard-shield-golem-witch.xml`;
  - `ice-nova-tempest-shield-no-shield-witch.xml`;
  - `minion-raise-zombie-offerings-witch.xml`: the minion's block and speed are not slice fields, so this one only shows that the character gets none of the offerings' buffs.
- **Public:** 182 builds, 121 expected differences, 0 unexpected; `Complete` fields 3,330 (3,168 before, plus the 7 fixtures). 34 values of the other builds moved: 33 toward PoB, 1 away (unreserved mana, below).
- **Corpus:** 0 unexpected. 32 values moved: 29 toward PoB (7 now equal), 3 away. The three away are unreserved mana, as in P6-9d: the newly modelled reserving skills add reservation that PoB lowers with modifiers the engine does not read yet (those fields stay `Partial`).

## The review of P6-7b3b to P6-9c (code-architecture-reviewer, no blockers)

Applied:
- **M1, M2: PoB's more rounding everywhere the phases read a more.**
  - `ModStore::more_rounded` rounds the product to two decimals whenever an effect applies. The nodes are always added, so the graph no longer depends on float noise.
  - `phases` uses it for every chained quantity, the resistances, block, and the primary defences, the minion's included. Test: `more_rounded_should_round_to_two_decimals`.
  - PoB rounds per mod name. The store rounds per quantity, which differs only where several names feed one quantity (energy shield's `EnergyShield` and `Defences`); this is noted in the doc.
- **M3:** the minion step adds the `unread-skill-modifier` reason when the buff totals did not read a modifier.
- **M4:** an unsettled minion-only value reaches the minion's reasons for every kind (regeneration, movement, cooldown too), except a golem one for a skill that is not a golem. Test: `applies_to_should_keep_every_kind_but_golem_for_others`.
- **m1:** a conditional minion modifier from the main skill's gems is a `minion-modifier` reason, as from the character.
- **m8:** the export scripts behind the inert classification are committed: `tools/oracle/data/classify_gems.py`, `minion_skill_globals.py`. `ENEMY_NO_SLICE_NAMES`' `Damage` is noted.
- **m9:** line ranges made consistent.

Left, with why:
- **m3:** curses that ignore the limit, and hexproof enemies. Their sources are unmapped stats or PoB config options the engine classifies, so they stay reasons.
- **m4:** the socket index uses the gem's position among typed gems. It differs from PoB's only with untyped gems or a repeated curse gem in one group.
- **m7:** the register has only `accepted` and `retired`. DIV-0111 to DIV-0113 note that confirmation in game is a maintainer item.

## P6-9f: Arrogance and five inert skills

**Arrogance** (`SupportArrogance`; PoB's `CalcPerform.lua` 1955 to 1964):
- **Its reservation:** the supported skill reserves life instead of mana. Life's base reservation gains mana's, and each pool takes its own terms (`gems::LIFE_RESERVATION_SUPPORTS`, read in `reservation`).
- **Its aura effect** is the supported aura's own scale term (`OWN_EFFECT_STATS`, as before).
- **Data parity:** `gem_effects` allows its `BloodMagicReserved` mod. Allowances now also match a mod name, not only a skill-data key.

**Five inert skills.** PoB's skill data gives each no global effect and no reservation, so as a skill that is not the main one each changes nothing:
- **The skills:** Forbidden Rite and its AltX, Summon Sentinel of Radiance, the Mirage Warriors, Whirling Blades.
- **Data parity:** `gem_effects` allows their own skill data (Forbidden Rite's damage and cost from life and energy shield, the Sentinel's minion level, the Mirage Warriors' trigger, Whirling Blades' double hit), read only when each is the main skill. The Sentinel's minion template has no skill.

**Not yet:** Eternal Blessing, Skitterbots, Raise Spectre, the relics, the warcries' power, and the other skills with their own buffs.
- Some of those reasons come from gems without a `gemId` (skills an item grants), which the tables cannot reach. Enemy Explode is one: PoB's id is not a granted effect of the game's data (a maintainer-run test checks each table entry against the bundle).

### Evidence

- **Public `calc-parity` on `b-ot`:** 210 builds, 280 expected, 0 unexpected.
- **Arrogance's builds** (all partial):
  - `ethereal-knives-mines-assassin.xml`'s unreserved life and mana regeneration now equal PoB's (2,471 and 118.8).
  - `kinetic-fusillade-ballista-hierophant.xml`'s unreserved life is closer: 3,448 against PoB's 3,548, from 3,821.
  - Their maximum hit and effective health pool follow that life. Where PoB's pools hold more than the engine models (mind over matter, guard), those values moved away from PoB's.
- **Values:** 29 moved toward PoB, 18 away (those dependents).
- **Data parity:** `uncategorized` 353.
- **Corpus:** 0 unexpected.
- **Tests:** the workspace's 1,766 tests and clippy pass.

## P6-9g: Mark On Hit

Mark On Hit (`SupportMarkOnHit`) supports a mark. PoB v2.67.2 reads two stats of it (`Data/SkillStatMap.lua`):
- `mark_skills_curse_effect_+%` is `CurseEffect` INC on a `SkillType.Mark` skill (2107). The engine adds it to the mark's own curse effect (`gems::MARK_CURSE_EFFECT_STAT`), as the gem's own `curse_effect_+%`.
- `trigger_on_attack_hit_against_rare_or_unique` sets `triggerMarkOnRareOrUnique` (273), which no calc module reads, so the mark applies as if cast. The engine does the same.

The support is in `gems::SUPPORTS` with no global stat.

**Fixture** `ice-nova-mark-on-hit-witch.xml`: Assassin's Mark with Mark On Hit (level 20, quality 20). Every computed field equals PoB's.

**Evidence** (`b-ct`, against P6-11ba's run):
- **Public:** 0 unexpected; 15 values toward PoB, none away, 2 now equal (Kinetic Blast's crit chance).
- **Corpus:** 13 toward PoB, none away.

**Eternal Blessing** (P6-9g, `SupportEternalBlessing`, `Data/Skills/sup_str.lua` 2062):
- **What PoB applies:** its base mods set the supported aura's flat and percent mana and life reservations to 0, and its `aura_effect_+%` (+19 at level 20) scales the aura. The engine lists it in `NO_RESERVATION_SUPPORTS`, and its aura effect is the aura's own scale term (`OWN_EFFECT_STATS`, read from supports too).
- **What PoB doesn't apply:** "... also disables non-blessing mana reservation skills" is in no calc module, so the other auras still apply there; the engine does the same.
- **Fixture** `ice-nova-eternal-blessing-hatred-witch.xml`: Hatred blessed, Purity of Ice reserving. Every computed field equals PoB's.
- **Evidence:**
  - **Public:** 16 values toward PoB, none away (Lacerate's damage, RF Chieftain's combined DPS).
  - **Corpus:** unchanged.

**data-parity** (`gem_effects`, run late, 2026-11-20): the two supports first showed 4 differences, each a path the engine reads on purpose. Each is now an allowance in `tools/parity-cli/src/data_parity/gem_effects.rs` with its reason:
- Eternal Blessing's four reservation keys, as Guardian's Blessing's;
- Mark On Hit's `triggerMarkOnRareOrUnique` and its `Triggered` type, as Automation's.

After that: 1,645 values compared, 0 differences, uncategorized 0.

**Not modelled: PoB-only skills.** `EnemyExplode` ("On Kill Monster Explosion", in 6 public builds) has no game granted effect of that ID, so its gem is `unmodelled-gem` before any table is read (tasks file).

## P6-11bs: warcry power and uptime

PoB v2.67.2 scales a warcry's buff (`CalcPerform.lua` 2311 to 2350) by `(1 + inc / 100) x more x bonus x uptime`:
- `inc` is the warcry's `BuffEffect`, `BuffEffectOnSelf`, and `BuffEffectOnPlayer` INC.
- `bonus` is `floor(min(power, limit) / div)` for a mod whose `GlobalEffect` tag has a `div`, else 1.
- `uptime` is `min(duration / cooldown, 1)`.

The terms:
- **Power:** the config's `multiplierWarcryPower` override, applied when not 0, else its placeholder when not 0 (`ConfigTab.lua` 1088 to 1093). Otherwise `max(base x (1 + inc / 100), minimum)`: `base` is 20 against the Boss, Pinnacle, and Uber kinds (`ConfigOptions.lua` 2097, 2132, 2166), `inc` is "increased total Power counted by Warcries".
- **Duration:** PoB's `calcSkillDuration` (`CalcOffence.lua` 300 to 313): the gem's `base_skill_effect_duration` times `max(calcLib.mod(Duration), 0)`. This version has no server tick rounding.
- **Cooldown:** the level's over `calcLib.mod(CooldownRecovery)`.

What the engine does:
- `gems::warcry_terms` computes the three terms from the warcry's gems' stats and the character's.
- The character's are the new `skill_modifiers::SkillTiming` sums in `Totals`, from six stats PoB reads as INC: "Skill Effect Duration", "Cooldown Recovery Rate", warcry duration, warcry cooldown recovery, warcry buff effect, and total power. The warcry ones carry `KeywordFlag.Warcry`.
- `skill_effect_duration_+%` leaves `NO_EFFECT`. Through a skill's own gems it stays no effect (the skill mapping's rule); the other five stay reasons there.
- A new `BuffStat::PerPower { div, limit, rows }` holds a row per power. `add_rows` scales it by the bonus and the uptime, and a warcry's other rows by the uptime.
- Enduring Cry's life regeneration (`RegenPercent` BASE, the stat over 60, step 5 up to 25) and Intimidating Cry's movement speed (`MovementSpeed` INC, step 5 up to 30) are such rows. Each was a field's reason marker before.
- A flat cooldown change (`base_cooldown_modifier_ms`), a missing level, or a power that is not a number is an `unmodelled-skill` reason (`warcry-uptime` or `warcry-power`).
- **Scaling:** PoB scales `<Pool>RegenPercent` BASE at precision 2 and `<Pool>Regen` BASE at precision 1 (`Modules/Data.lua` 437 to 454). `skill_modifiers::scale` now does too. Clarity's, Vitality's, and the golems' regeneration were scaled with the default rule before.

**Fixtures:** every computed field of both equals PoB's.
- `ice-nova-warcry-power-witch.xml`: both cries at the boss's power 20.
- `ice-nova-warcry-power-config-witch.xml`: the config's power 12, with lines of warcry duration, skill effect duration, warcry cooldown recovery, and warcry buff effect.

**data-parity** (`gem_effects`): 1,647 values compared, the two new rows with their `div` and `limit` among them, 0 differences.

**Evidence** (`b-ct`, against P6-11br's run):
- **Public:** 0 unexpected. 11 values moved toward PoB, 6 now equal, none away. 6 fields went from partial to complete: the Inquisitor builds' movement speed and a Chieftain's, plus three builds' life regeneration.
- **Corpus:** 0 unexpected; 5 values toward PoB, 1 now equal, none away.

## P6-11bt: Ruthless

Ruthless (`SupportRuthless`, `Data/Skills/sup_str.lua` 4685 to 4720) makes every third use of a melee strike or slam a Ruthless blow.

What PoB does (`CalcOffence.lua` 2889 to 2909):
- It reads `RuthlessBlowMaxCount` BASE (the constant 3), and `RuthlessBlowHitMultiplier` and `RuthlessBlowAilmentMultiplier` BASE with `ModFlag.Melee`.
- In the average mode (`ruthlessSupportMode`, the default), the chance is `round(100 / count)`. Each effect is `1 - chance / 100 + chance / 100 x (1 + multiplier / 100)`.
- The hit's effect joins `allMult` after the scaled damage effect (3352). The ailment effect multiplies bleed's and poison's base (4451, 4721), not ignite's.

What the engine does:
- `Poe1Quantity::Ruthless` carries the three terms as `HIT_ROWS` rows, read by the skill mapping as hit quantities.
- `hit::ruthless_effect` computes an effect only when a count is present, so a hit without Ruthless keeps its graph.
- The hit multiplies its scaled damage effect by the hit effect. `ailment` multiplies bleed's and poison's base by the ailment effect, summed with the hit's context as PoB's `cfg`.
- The `MAX` mode (chance 100, the DPS divided by the count) is an `unmodelled-config` reason.
- The stun duration override is `NoEffect` (`stun`).

**Fixture** `smite-ruthless-bleed-templar.xml`: Smite with Ruthless, a Siege Axe, and a 25% chance to bleed. The hit and the bleed equal PoB's. The differences are Q11's floored attack rate (`DIV-0320` to `DIV-0326`); the bleed's DPS follows the rate, as `DIV-0137` already registers.

**Register:** the `added` date of the Q11 entries `DIV-0302` to `DIV-0319` is corrected to 2026-11-20. The helper had a fixed date.

**Evidence** (`b-ct`, against P6-11bs's run):
- **Public:** 0 unexpected; 7 values toward PoB, none away (a Juggernaut's Boneshatter, among others).
- **Corpus:** 0 unexpected; 1 value toward PoB, none away.

## P6-11ci: the character's warcry speed

PoB's warcry cast time (`calcWarcryCastTime`, `CalcOffence.lua` 289 to 298) reads `calcLib.mod(skillModList, "WarcrySpeed")`, the warcry's gems' and the character's "#% increased Warcry Speed" (`WarcrySpeed` INC, `KeywordFlag.Warcry`). Intimidating Cry's exerted attacks read it through their uptime (P6-9e).

What the engine does:
- `skill_modifiers::SkillTiming::WarcrySpeed` (a `STATS` row) sums the character's warcry speed into `Totals`.
- `gems::intimidating_terms` adds it to the gems' warcry speed.

**Fixture** `smite-intimidating-cry-warcry-speed-templar.xml`: Smite with Intimidating Cry in another group and 30% increased warcry speed. The differences are Q11's floored attack rate (`DIV-0346` to `DIV-0353`). They include the average damage, through the exerted attacks' uptime, as `DIV-0056` registers.

**Evidence** (`b-ct`, against P6-11ch's run): public and corpus 0 unexpected; no value moved (no build pairs the line with Intimidating Cry).

## P6-11cj: Empower's supported level

Empower's "Supported Skills have +# to Level" (`supported_active_skill_gem_level_+`) is PoB's `SupportedGemProperty` `level` (`SkillStatMap.lua` 2304).
- **PoB:** for a skill from a gem that is not a support, PoB adds it to the active effect's level before reading the level's data (`CalcActiveSkill.lua` 549 to 557).
- **The engine:** `skill::supported_level` adds the applied supports' value to the active gem's level, for the main skill and for each group's skills (`GroupSkill`: auras and buffs), when the skill is from a gem.
- **Mapping:** the stat is a `NoEffect` row (`gem-level`), so the support's own mapping adds no reason. The gems guard names it.

**Fixture** `ice-nova-empower-hatred-witch.xml`: Ice Nova and Hatred, each with Empower at level 3 (+2). Every computed field equals PoB's; the average damage is 907.8 against 727.5 without Empower.

**data-parity:** `uncategorized` 0.

**Evidence** (`b-ct`, against P6-11ci's run):
- **Public:** 0 unexpected; 12 values toward PoB, 3 now equal, none away.
- **Corpus:** 0 unexpected; 7 toward PoB, none away.

## P6-11ck: Enhance's supported quality

Enhance's "Supported Skills have +#% to Quality" (`supported_active_skill_gem_quality_%`) is PoB's `SupportedGemProperty` `quality`, added to the active effect's quality the same way as P6-11cj's level. `skill::supported_level` adds both, and the quality stat is a `NoEffect` row too.

**Fixture** `smite-enhance-quality-templar.xml`: Smite's area part with Enhance at level 4 (+24% quality). Smite's quality stat is more area damage, and the area hit is 1.15 times the same build without Enhance, as in PoB. The differences are Q11's floored attack rate (`DIV-0354` to `DIV-0359`).

**Evidence** (`b-ct`, against P6-11cj's run):
- **Public:** 0 unexpected; no value moved (the build with Enhance has other reasons).
- **Corpus:** 0 unexpected; 7 values toward PoB, none away.

## P6-11cl: the typed supported levels

The awakened supports' "+# to Level of Supported <Type> Skill Gems" are PoB's `SupportedGemProperty` `level`, each with a `SkillType` tag (`SkillStatMap.lua` 2284 to 2330).
- **The types:** aura, chaos, cold, fire, lightning, minion, physical, strike (`MeleeSingleTarget`).
- **Two keyword variants:** curse is `KeywordFlag.Curse`, the `AppliesCurse` type. Elemental is the fire, cold, or lightning keyword.
- **The engine:** `skill::SUPPORTED_TYPED_LEVEL_STATS` names each stat and the types of which the skill needs one. `supported_level` reads them against the skill's types after its supports added theirs, as PoB's `activeSkill.skillTypes`. Each is a `NoEffect` row (`gem-level`).
- **Tests:** a real-bundle test checks the type names, and the gems guard accepts the table.

**Fixture** `ice-nova-awakened-added-cold-witch.xml`: Ice Nova with Awakened Added Cold Damage at level 5 (+1 cold skill level). Every computed field equals PoB's.

**data-parity:** `uncategorized` 0.

**Evidence** (`b-ct`, against P6-11ck's run): public and corpus 0 unexpected; no value moved (no other build has a typed level).

## Open items S2 (2026-11-24): gems checked against PoB's data

The survey (S0) listed the unmodelled gems by skill ID (`local/p6/tools/gems_unmodelled.py`, public and corpus by content hash). Each candidate was checked against PoB v2.67.2's export (`local/p6/tools/gem_globals.py`: `GlobalEffect` mods, types, functions) and its calc modules (references by name), then entered and checked by `data-parity` (`gem_effects`, which refuses an entry whose PoB data names a minion, a trigger, a totem, a buff type, or what scales buffs, reservation, or an ailment).

**Added** (`data-parity` `uncategorized` 0): `SupportAddedChaosDamage` (its added chaos damage is the supported skill's own) and `SupportEclipse`, each with no global stat.

**Refused by `data-parity`, so still `unmodelled-gem`** (each needs its own model):

- Blink Arrow: its minion (`ArrowClone`, `minionUseBowAndQuiver`);
- Cold Snap: `active_skill_chill_effect_+%_final` (`EnemyChillEffect` MORE), which a chilling area's guaranteed chill reads (`poe1::damage_taken`);
- Corrupting Fever: a `Buff` type;
- Vaal Absolution: its soul gain prevention data;
- Arcanist Brand (`SupportBrandSupport`) and Sacred Wisps: triggers;
- Living Lightning: a `CreatesMinion` type;
- Spell Totem: `SummonsTotem`, `ReservationBecomesCost`, and the totem's level;
- Unbound Ailments: `non_damaging_ailment_effect_+%` (`EnemyChillEffect` INC).

**Not inert by PoB's calc modules:** Summon Sentinel of Radiance (its life takes damage before yours, `CalcDefence.lua` 34, `CalcPerform.lua` 1223), Pact of Ghorr (the pacts' uptime, `CalcOffence.lua` 2804), Summon Elemental Relic (its minion's auras), and the buffs and auras with global rows (Arcane Cloak, Energy Blade, Petrified Blood, Mana Infused Staff, Aspect of the Cat, Raise Spiders, Convocation, Vaal Grace, Vaal Smite, Close Combat): each needs its rows as a `Kind::Buff` with fixtures, left for a later step. Skitterbots need a data run (follow-ups).

## Summon Skitterbots (open items, 2026-11-24)

S0 listed Skitterbots as a data run; the bundle `b-ct` has their monster varieties, and PoB's model of the skill beside the main one needs none of them. PoB v2.67.2:

- **Its global effect:** `skitterbots_trap_mine_damage_+%_final`, `Damage` MORE with `KeywordFlag.Mine` or `Trap`, a `Buff` (`act_int.lua` 20085); its reservation, 35% of mana.
- **Its auras** (`CalcPerform.lua` 1360 to 1394, instead of the chilling-area branch): `ShockOverride` BASE `15 x (1 + (inc + SkitterbotAilmentEffect) / 100)` and `ChillOverride` BASE `10 x (...)`, `inc` the skill's own `EnemyShockEffect` and `EnemyChillEffect` INC of source `Skill` (the gem's `shock_effect_+%`, `chill_effect_+%` at its level); each marks the enemy shocked or chilled. The ailment table takes an override unscaled (3509 to 3519).

**The engine:** `gems::SKILLS` has `SummonSkitterbots` (`Kind::Buff`, the damage row, its reservation); `gems::skitterbots` adds `Poe1Quantity::SkillShockOverride` and `SkillChillOverride`, which `damage_taken::shock` and `chill` take unscaled and which shock or chill the enemy without the config checks; `conditions::settle` sets the enemy's `Shocked` while one is active (`gems::skitterbots_shock`). Reasons: a support with a shock, chill, or non-damaging ailment effect, or Bonechill (`skitterbots-supports`), and the unmapped stats that stop or change the auras. `data-parity`: the gem's minions, types, and chill effect allowed with the reasons above; trap and mine keyword flags now written as PoB writes them (`other-flags 0/12288`); `uncategorized` 0.

**Fixture:** `ice-nova-skitterbots-witch.xml`: Ice Nova with Skitterbots in another group and crit chance against shocked enemies; PoB's `CurrentShock` 17, `CurrentChill` 11; every computed field equals PoB.

**Evidence** (`local/p6/sk` against `local/p6/gs`): public `Complete` 39,309 (+77, the fixture), 0 unexpected, 18 values toward PoB, none away; corpus 0 unexpected, 17 toward, 3 away: one build (by hash `b6098b68`) whose average damage, ignite, and hit DPS now overshoot PoB. PoB's own `CurrentShock` there is 18, the engine's too (a level 21 gem: 15 x 1.20); the build was near PoB before only through other missing terms (its fields keep their other reasons), as the close-out's batches recorded for such builds. Distinct reasons: 318 to 319 (`skitterbots-supports` new); field-reason pairs: corpus 9,434 to 9,272.


## Vaal Grace and the chance to evade (open items S2, 2026-11-26)

**PoB v2.67.2:** Vaal Grace's aura (`act_dex.lua` 8998 to 9040) has one global stat, `chance_to_evade_attacks_%` 15, `EvadeChance` BASE (an `Aura`). "+#% chance to Evade Attack Hits" on an item or a custom line is the same mod (`ModParser.lua` 254 to 258). `CalcDefence.lua` 1075 to 1096 subtracts `EvadeChance` BASE from the enemy's chance to hit: `<Kind>EvadeChance = max(0, min(95, (100 - (hitChance(evasion, accuracy) - EvadeChance BASE) x enemy hit chance mod) x calcLib.mod(EvadeChance, <Kind>EvadeChance)))`.

**The engine:**
- `Poe1Quantity::EvadeChance`: `Base`, `Increased`, and `More`. `evade` subtracts the base from the enemy's chance to hit and scales by the modifiers, as PoB does; no stat maps to `Increased` or `More` yet.
- The mapping row `chance_to_evade_attacks_%` gives it `Base`.
- `gems::SKILLS` has `VaalGrace` (`Kind::Buff`, an aura, that row), so its aura effect scales it as any aura's.
- Its "Damage taken from Suppressed Hits is Unlucky" (`damage_taken_from_suppressed_hits_is_unlucky`) has no row in PoB's stat map, so PoB applies nothing, nor the engine. It changes the enemy's hit in the game: a game-truth item, not registered.
- `data-parity`'s `gem_effects`: the row's name `EvadeChance`; Vaal Grace's `skillEffectAppliesToSoulGainPrevention` (read only for its own soul gain prevention, `CalcOffence.lua` 1577) is a skill-data key that does not block.

**Fixtures** (original, made by `local/p6/s2-fx1.py`; each run three times by PoB, identical), a level 90 Ranger with 3,000 evasion:
- `vaal-grace-evade-chance-ranger.xml`: Vaal Grace (level 20) in its own group; the evade chances 38 (23 without it);
- `chance-to-evade-attack-hits-ranger.xml`: "+10% chance to Evade Attack Hits" as a custom line; 33.
- Every compared field equals PoB's; 174 targets.

**Evidence** (`local/p6/s2-e1` on `b-pb`, against `local/p6/s9a`):
- **Public:** 0 unexpected; `Complete` 49,371 to 49,545 (the fixtures).
  - `Mirage Archer Toxic Rain.xml` (Vaal Grace): its evade chances 67 to 82 (PoB 87) and its configured not-hit chance 37 to 44.5 (PoB 50.45), toward PoB.
  - Four of its effective HP values moved away: `TotalEHP` 27,752 to 31,512 (PoB 29,271), and its hit counts. Its life (4,158, PoB 3,974) and maximum hits (physical 10,076, PoB 8,188) were already above PoB's, from its other reasons; the larger evade chance multiplies them. The build stays indeterminate.
- **Corpus:** 0 unexpected; documents equal but for node numbers.
- **Tests:** the mapping test of the row, the workspace tests, the real-bundle tests, clippy, rustfmt, and wasm32.


## Summon Skitterbots' supports, and Unbound Ailments (open items S2, 2026-11-26)

**PoB v2.67.2** (`CalcPerform.lua` 1357 to 1383):
- **The `inc` of the Skitterbots' shock and chill** is `skillModList:Sum("INC", { source = "Skill" }, "EnemyShockEffect")` (and `EnemyChillEffect`): every mod of source `Skill`, the gem's and its supports' stats.
  - Shock: `shock_effect_+%`, `lightning_ailment_effect_+%`, `non_damaging_ailment_effect_+%`.
  - Chill: `chill_effect_+%`, `cold_ailment_effect_+%`, `non_damaging_ailment_effect_+%` (`SkillStatMap.lua` 1085 to 1119).
- **Bonechill on the Skitterbots** sets `HasBonechill` (they can chill) and the guaranteed Bonechill (1379 to 1380). The chilled enemy then takes increased cold damage by the chill's effect (3444).
  - Its `SkitterbotBonechill` flag is read only for the chill on you (844), which only "the auras affect you" starts.

**The engine** (`gems::skitterbots`):
- The two overrides sum those stats over the gem and its supports (`SHOCK_EFFECT_STATS`, `CHILL_EFFECT_STATS`).
- Bonechill gives `Poe1Quantity::Bonechill(true)`, the chilling area's guarantee, which `damage_taken` reads.
- The `skitterbots-supports` reason (Summon Skitterbots, above) is no longer given; the unmapped stats that change the auras stay reasons.

**Unbound Ailments** joins `gems::SUPPORTS`: no global stat (PoB's data has none).
- Its `non_damaging_ailment_effect_+%` is the supported skill's own: read for the Skitterbots.
- On a chilling area it is a `skill-chill-effect` reason (`gems::guaranteed_chill`, now also for a support's chill effect stat), as Bonechill's chill effect there.
- `data-parity`'s `gem_effects` allows its `EnemyChillEffect` with those readings.

**Fixtures** (original, made by `local/p6/s2-fx2.py`; each run three times by PoB, identical): Ice Nova with Summon Skitterbots in another group and 40% increased crit chance against shocked enemies, as `ice-nova-skitterbots-witch.xml`:
- `ice-nova-skitterbots-unbound-ailments-witch.xml`: Unbound Ailments (level 20) on the Skitterbots; PoB's `CurrentShock` 25 and `CurrentChill` 17 (17 and 11 without it).
- `ice-nova-skitterbots-bonechill-witch.xml`: Bonechill (level 20); `CurrentChill` 16, the average damage 864 to 982 from the cold damage taken.
- Every compared field equals PoB's; 174 targets.

**Evidence** (`local/p6/s2-e2` on `b-pb`, against `local/p6/s2-e1`):
- **Public:** 0 unexpected; `Complete` 49,545 to 49,719 (the fixtures). Seven values moved, all toward PoB:
  - `ethereal-knives-mines-assassin.xml` (Skitterbots with Bonechill and Unbound Ailments): its average damage, hit, ignite, full, and combined DPS, and its unreserved mana 560 to 182 (PoB 174; Unbound Ailments' reservation multiplier now counts);
  - `forged-frostbearer-spectre-necromancer.xml`'s full DPS, 7.79 to 8.84 million (PoB 9.13 million).
- **Corpus:** 0 unexpected; one value moved, now equal to PoB's (a build's unreserved mana, 221 to 5, PoB 5); one other document lost the reason in its reasons only.
- **Tests:** `gems`' tests (the tables sorted), the workspace tests, the real-bundle tests, clippy, rustfmt, and wasm32.


## Vaal Smite's aura (open items S2, 2026-11-26)

**PoB v2.67.2:**
- **Vaal Smite's aura** (`act_str.lua` 10517 to 10575) has Smite's aura's three stats without their condition: `LightningMin` and `LightningMax` BASE, and `EnemyShockChance`.
- **Smite's aura rows** each have `Condition` `AffectedByVaalSmite` negated (10325 to 10334). PoB sets that condition while Vaal Smite's aura applies (`CalcPerform.lua` 2417 to 2419), so Vaal Smite's aura replaces Smite's.
- **The gem has two effects,** Vaal Smite and Smite, each applied by its `enableGlobal<n>`.
- Its `global_always_hit` (`cannotBeEvaded`) is the skill's own (`CalcOffence.lua` 2162).

**The engine:**
- `gems::SKILLS` has `VaalSmite` (`Kind::Buff`, an aura, Smite's rows; the shock chance no slice field reads, as Smite's).
- `gems::apply` drops Smite's aura when Vaal Smite's is collected.
- Smite as the main skill gives its aura through the hit's mapping (P3-8l), which the condition would turn off: with Vaal Smite's aura up, that is an `unmodelled-skill` `vaal-smite-over-smite` reason.
- `data-parity`'s `gem_effects`: `cannotBeEvaded` is a skill-data key that does not block.

**Fixtures** (original, made by `local/p6/s2-fx3.py`; each run three times by PoB, identical), Ice Nova (level 20) with Vaal Smite (level 20) in another group:
- `ice-nova-vaal-smite-aura-templar.xml`: only the Vaal effect's global on; average damage 952.68;
- `ice-nova-vaal-smite-over-smite-templar.xml`: both on, the same output as the first (PoB's hash too);
- `ice-nova-vaal-smite-smite-only-templar.xml`: only Smite's; 877.98.
- Every compared field equals PoB's; 261 targets.

**Evidence** (`local/p6/s2-e3` on `b-pb`, against `local/p6/s2-e2`):
- **Public:** 0 unexpected; `Complete` 49,719 to 49,980 (the fixtures). No other value moved: `flicker-strike-berserker.xml`'s Vaal Smite has its Vaal effect's global off (it loses the `unmodelled-gem` reason; its other reasons stay).
- **Corpus:** 0 unexpected; no document changed.
- **Tests:** `gems`' tests, the workspace tests, the real-bundle tests, clippy, rustfmt, and wasm32.
