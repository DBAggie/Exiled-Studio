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
