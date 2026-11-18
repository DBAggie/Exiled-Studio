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
