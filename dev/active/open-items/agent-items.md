Last Updated: 2026-11-24

# Open items: the agent's list

The maintainer (2026-11-24): "compile all items and complete them". Every open item an agent can finish without the maintainer, from `open-items-inventory.md` (class A), the follow-ups' "Planned for the session, may not be reached", and the items found to need no data run (#2 of the same message). Ordered by fields cleared per unit of effort. Each is done as one commit on PR #168 with its evidence (the per-step routine in `open-items-context.md`), and ticked here; one that cannot finish moves to `maintainer-follow-ups.md` with what was found.

Fixture budget: the oracle job now runs only a pull request's own fixtures (`.github/scripts/oracle-select.py`), so the 75-fixture cap of S0 no longer binds a pull request; a full run (pushes to `dev`) still grows with each fixture (about 14.8 s each on Windows).

## Found to need no data run (2026-11-24)
- [ ] **Skitterbots** (`SummonSkitterbots`, `Skitterbots`; 5 public, 7 corpus builds): the bundle `b-ct` has the monster varieties (`Metadata/Monsters/Skitterbot/SkitterbotCold`, `SkitterbotLightning`, ...) in `poe1/minions.json`; model the skill and its auras (PoB's `Data/Minions.lua` 1038, 1079).
- [ ] **The Holy Relic's Nova** (`docs/phase-6/triggers.md`, "The Holy Relic's Nova"): data schema 5 carries skills' level requirements (`EffectLevel::level_requirement`); port PoB's minion skill level (`CalcActiveSkill.lua` 895 to 902) and the kept fixture.
- [ ] **Vaal souls**: already in `b-ct` (2,268 skill levels carry `vaal_souls`); any reason still waiting on it is agent work.
- [ ] **The 3_29 ascendancy auras** (Ramako, Void Beacon, Augury of Penitence, Unwavering Crusade: no stats in `PassiveSkills`): look for their buffs in the game files on this workstation (the 26530219 depot in `local/b2/steam-26530219`) before calling it a data gap.

## Small, existing fixtures
- [ ] **Static Strike part 1** (3 Smite builds' `FullDPS`): its function and beam damage act only in part 2; its zap stats `NoEffect` outside part 2, the read lists by part.
- [ ] **Vaal Summon Skeletons' full DPS** (417 against 115): trace with an instrumented PoB copy in the scratchpad.

## S9: resolver and item gaps
- [ ] `ambiguous` (public 655 fields in 9 builds; corpus 470 in 6): local against global readings by the item's class (`docs/spikes/spike-b-resolution.md`, the Mods filter).
- [ ] `no-variant-fits` (457 / 159), `only-shadowed` (corpus 67).
- [ ] `scaled-line` catalyst (501) and modifier magnitude (365 / 160); `excluded-line` section (231).
- [ ] `unmodelled-item`: `ambiguous-base` (157 / 224), `flask-base` (227), `slot`, `jewel-limit`, `flask-merge`, `abyssal-socket`, `no-physical-damage`; `unhandled-kind` `item-property` ("Intangibility: #%", a flask's recovery line).
- [ ] `unmodelled-passive`: `unresolved-tree-line` (284 / 600), `unconnected-node` (454 / 304), `cluster-jewel-node` (462), `timeless-jewel` (136 / 151), `unknown-node-kind`; `unread-line`.

## S1: skills
- [ ] The other `pob-skill-data` skills, most builds first: Cyclone, Fire Trap (corpus 3), Thrown Shield, Frenzy, Rain of Spores, Vaal Detonate Dead, Poison Arrow, Freezing Pulse, Boneshatter (Trarthus), Elemental Hit (Spectrum), Kinetic Blast, Kinetic Fusillade, Shock Nova, Blood Spears, Lacerate, Lightning Trap, Frost Bolt Nova, Winter Orb, Ice Spear, Vaal Blight, Tornado, Essence Drain, Scorching Ray, Ground Slam, Arc.
- [ ] `socket-colour-quality` (275 / 77), `sigil-of-power-stages` (78 / 161), `skill-chill-effect` (155), `minion-buffs`, `minion-modifier`, `aura-debuff-effect`, `requires-shield`, `rallying-cry-allies`, `wave-of-conviction-exposure`, `full-dps-skill`.
- [ ] The close-out's `C-unmodelled-skill` stats owned by the skills above (101 at S0).

## S2: gems
- [ ] Buffs and auras with global rows: Arcane Cloak, Energy Blade, Petrified Blood, Mana Infused Staff, Aspect of the Cat, Raise Spiders, Convocation, Vaal Grace, Vaal Smite, Close Combat.
- [ ] Cold Snap and Unbound Ailments (their chill effect); Corrupting Fever; Blink Arrow; Vaal Absolution; Arcanist Brand, Sacred Wisps, Living Lightning, Spell Totem supports; Summon Sentinel of Radiance; Pact of Ghorr; Summon Elemental Relic; Bane.

## S8: the close-out's closed reasons and config
- [x] Item-granted skills (`C-item-skill`, 10 of 13 stats; Herald of the Breach 280 fields): the grant lines as `GrantsSkill`, the saved group checked (`docs/phase-6/stat-mop-up.md`, "Item-granted skills"); `local/p6/gs`
- [ ] `C-engine-system` (69), `C-keystone` (6: Corrupted Defences, Crimson Dance, Precise Technique, Quiet Might, Tinctures' life drain, Wicked Ward), `C-gem-property` (7), `C-no-slice` (2).
- [ ] Config: brittle (an enemy-side crit that reaches minions), the minion inputs (`minionsConditionFullLife`, `minionBuffOnslaught`, the minion charges), Feeding Frenzy, Elemental Equilibrium (`enemyConditionHitByLightningDamage`, `EEIgnoreHitDamage`), `multiplierNearbyAlly`, `presetBossSkills`, `TotalSpectreLife`, `detonateDeadCorpseLife`, `multiplierManaBurnStacks`, `bandit` values, `NoLifeModifiersOnBodyArmour`.

## The larger steps
- [ ] **S5:** mine, trap, and totem costs per second; warcry power and the cries that scale with it; General's Cry's mirage warriors; Sacred Wisps.
- [ ] **S6:** the minion's own damage over time; spectres.
- [ ] **S7:** ward's other lines, EHP's other pools, damage over time EHP, conversion smoothing in maximum hit, degeneration, a dual wielder's ailments, leech, and block.
- [ ] **S3:** triggers and Spellslinger (`CalcTriggers.lua` on the P6-8a model).
- [ ] **S4:** Blood and Sand, and PoB's full DPS pass (an instrumented PoB copy).

## Performance (from the profile, `docs/phase-5/performance.md`)
- [ ] A per-calculation cache of the resolved main skill and active skills.
- [ ] The full DPS pass without recollecting the whole build per included skill.
