# P6-7a: minion data

Plan decision 7 (minions are a second actor) and decision 8 (data first): this step adds the monster data a minion or a spectre is computed from, with its `data-parity` table. No calculation reads it yet; P6-7b and P6-7c do.

## The data

- **Bundle:** `poe1/minions.json` (`loom_data::bundle::poe1::Minions`; converter `tools/parity-cli/src/minions.rs`). It holds every monster variety, since a spectre can be any monster: 10,691 rows from build 26530219. It also holds the monster types they use.
- **Extraction:** the tables are already extracted. `MonsterVarieties` and `MonsterTypes` come with existing roots, and `MonsterResistances` is referenced by `MonsterTypes`.
- **What is kept**, as PoB's minion exporter reads it (`src/Export/Scripts/minions.lua`, `#emit`):
  - **A variety's:**
    - name and tags;
    - life and damage multipliers;
    - attack duration in milliseconds (the schema's `AttackSpeed`);
    - maximum attack distance;
    - main- and off-hand item classes;
    - granted effects (its skills), mods, and special mods;
    - object type (`BaseMonsterTypeIndex`).
  - **Its type's:**
    - accuracy, armour, evasion, and energy shield;
    - damage spread;
    - the merciless resistances (`MonsterResistances`);
    - the alternate life scalings;
    - whether its base damage ignores attack speed;
    - the game's player-minion mark.
- **Not carried:** the object type's `.ot` stats, which PoB adds to a minion's mod list.
- **Validator:** unique variety IDs, and each variety's type in the file. A synthetic row and a mutation test cover these checks.

## `data-parity`, table `minions` (export format 12)

The export adds `data.minions`, PoB's minions and spectres, keyed by the monster variety's ID. Spectres are keyed by it. A minion's PoB key is a short name, which PoB's own template (`Export/Minions/Minions.txt`, `#monster <id> <name>`) maps to its variety.

Each is compared in PoB's form:
- the name;
- life, damage, and accuracy over 100;
- energy shield `0.4 x value / 100`, and armour and evasion over 100, each absent at 0;
- the resistances;
- damage spread over 100;
- attack time over 1000;
- attack range;
- the life scaling;
- base damage ignoring attack speed;
- the weapon types (PoB's `itemClassMap`);
- the damage fixup from the mods;
- the tags;
- the skill list, as a prefix: PoB adds skills its template names.

PoB's mod list is not compared, because it includes the `.ot` stats.

**Result:** 332 minions and spectres, 5,959 values. The only difference is `GuardianRelicAll`, a minion PoB writes by hand in its template (no game variety), which is a `pob-derivation` rule. The PoB skill table, regenerated from format 12, is unchanged. Of the 968 differences, 353 are uncategorized, all `cluster_jewels`, as before (P6-3).

**Evidence bundle:** `local/p6/packs/b-minions` is the flask evidence bundle with only `poe1/minions.json` added, and `bundle-check` reports no findings. The bundle is local and unsigned. **Maintainer:** the next data run ships the file.

**Size:** the file is 11 MB as JSON. The bundle is zipped when shipped.

## The P6-6e review, applied here

`docs/phase-6/totems.md`, "P6-6e":
- the trap and mine rate ignores throwing speed, whose stats are unmapped;
- the module docs are corrected.

The review's other suggestion, integration tests for the trap and mine rate and reasons, is left to P6-7b's test pass. The sweep (33 builds against the oracle) is the evidence meanwhile.
