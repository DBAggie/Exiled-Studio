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

