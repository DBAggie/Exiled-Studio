# P6-8b: mirages

PoB v2.67.2 computes a mirage as a copy of the main skill (`Modules/CalcMirages.lua`, `calcs.copyActiveSkill`), and adds the copy's output to the main skill's `CombinedDPS` (`CalcOffence.lua` 6088 to 6123).

## Mirage Archer

**What PoB does** (`CalcMirages.lua` 64 to 113):
- **Which skills:** a skill Mirage Archer supports, PoB's `triggeredByMirageArcher` from `skill_can_own_mirage_archers` on a `MirageArcherCanUse` skill.
- **The copy:** the skill with the support's less damage and less attack speed as `Damage` and `Speed` MORE, and no cost.
- **The count:** 1 per `summon_mirage_archer_on_hit` (`MirageArcherMaxCount` BASE 1).
- **Into `CombinedDPS`:**
  - the copy's `TotalDPS` and `ImpaleDPS`, each times the count;
  - its single poison's `PoisonDPS` times the count;
  - its damage over time when `DotCanStack` or the skill has none;
  - a copy ignite or bleeding larger than the skill's own sets the skill's to 0, and is added to PoB's `MirageDPS`, not to `CombinedDPS`;
  - the cull multiplier is the larger of the two.

**The engine:**
- **`CalcOptions::mirage`** (`MirageUse`, internal: a host leaves it `None`): set when the engine computes a mirage's copy. `collect` then adds the less damage and speed as `Damage` and `Speed` MORE (rule source `mirage-archer`).
- **`combined::phase`:** for a skill Mirage Archer supports, runs `calculate` with that option and reads the copy's hit DPS, impale, ignite, and bleeding into the character's combined DPS, as above.
  - The copy's reasons carry over. A copy hit that is not computed leaves the combined DPS unsupported.
  - A copy poison (PoB adds the single poison's `PoisonDPS`, which the engine does not compute) or damage over time (`DotCanStack`, not modelled) is a reason.
  - A larger copy ignite or bleeding sets the skill's own field to 0, as PoB's output does.
- **`NoEffect`** (read by `poe1::combined`, or no PoB module reads them): `skill_can_own_mirage_archers`, `summon_mirage_archer_on_hit`, `support_mirage_archer_damage_+%_final`, `support_mirage_archer_attack_speed_+%_final`, `support_mirage_archer_base_duration`.

**Not ported, each a reason:**
- General's Cry's mirage warriors (`triggered_by_spiritual_cry`, unmapped): PoB replaces the skill's own DPS with its cooldown, mirage count, and exert increases (344 to 423).
- Sacred Wisps (`triggeredBySacredWisps`, its support unmapped).
- Reflection and Tawhoa's Chosen: main skills the engine does not compute.

### Evidence

- **Fixture** `mirage-archer-burning-arrow-templar.xml` (new, original): Burning Arrow with Mirage Archer and the bow lines of `burning-arrow-bow-lines-templar.xml`. Every field is `Complete`.
  - `CombinedDPS` is the skill's own plus the copy's hit DPS: PoB's `MirageDPS` is 2.6153142, the engine's copy 2.6140072.
  - The skill's `TotalDPS` and `IgniteDPS` and the copy's DPS each differ from PoB's by the same ratio, 0.99950025: the bow's Q11 attack rate (1.1544228 against PoB's 1.155).
  - Registered: DIV-0185 to DIV-0188, `game-truth`.
- **Public `calc-parity` on `b-ot`:** 205 builds, 187 expected, 0 unexpected; `Complete` fields 5,506 (5,473 before).
  - Elemental Hit of the Spectrum's combined DPS moved toward PoB (its Mirage Archer). It stays `partial`.
- **Corpus, minion sweep, skill sweep:** unchanged; 0 unexpected in the registered sets.
- **Tests:** the workspace's 1,763 tests and clippy pass.
