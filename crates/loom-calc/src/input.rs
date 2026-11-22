//! What a calculation reads (ADR-003; Phase 3 plan, decision 7): the
//! resolved build (borrowed, immutable), a view of the game data, and the
//! options.

use std::collections::BTreeMap;

use loom_data::bundle::Bundle;
pub use loom_data::bundle::passives::StatAmount;
pub use loom_data::bundle::skills::{
    ActiveSkill, EffectLevel, FloatStat, GemEffect, GrantedEffect, Reservation, Resource, SkillGem,
    StatSet, StatSetLevel,
};
use loom_domain::{
    Build, ConfigSetId, DataVersion, ItemId, ItemSetId, NodeId, SpecId, TreeVersion,
};

use crate::error::CalcError;

/// A class's base values (the characters table).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CharacterBase {
    /// Base life before per-level growth.
    pub base_life: i64,
    /// Base mana before per-level growth.
    pub base_mana: i64,
    /// Base strength.
    pub strength: i64,
    /// Base dexterity.
    pub dexterity: i64,
    /// Base intelligence.
    pub intelligence: i64,
}

/// The object template every player character extends; its `Stats` are
/// the character constants (per-level growth, caps).
pub const CHARACTER_TEMPLATE: &str = "metadata/characters/character";

/// The object template every monster extends; its `Stats` are the monster
/// constants (the enemy's base maximum resistance, P3-9b).
pub const MONSTER_TEMPLATE: &str = "metadata/monsters/monster";

/// The game data a calculation reads, as `ItemData` is for import. The
/// bundle implements it; the parity tool's data overlay (P3-5) is another
/// implementation, so the core keeps the game's data. Every table read
/// goes through this trait, so the overlay can substitute PoB's values.
pub trait CalcData {
    /// The data's version: the build's must equal it.
    fn data_version(&self) -> DataVersion;

    /// A class's base values, by the class's name as builds write it.
    fn character(&self, class: &str) -> Option<CharacterBase>;

    /// A character constant: a stat of [`CHARACTER_TEMPLATE`].
    fn character_constant(&self, stat: &str) -> Option<i64>;

    /// A monster constant: a stat of [`MONSTER_TEMPLATE`] (PoB's
    /// `data.monsterConstants`).
    fn monster_constant(&self, stat: &str) -> Option<i64>;

    /// The stats of the passive node with this number.
    fn passive_stats(&self, node: u32) -> Option<&[StatAmount]>;

    /// The stats of the mastery effect with this hash.
    fn mastery_effect_stats(&self, effect: u32) -> Option<&[StatAmount]>;

    /// The properties of the item base with this name; `None` when no
    /// base has the name.
    fn item_base(&self, name: &str) -> Option<ItemBase>;

    /// The name of the item base with this ID (P6-11dm: a gem's base item
    /// names a support, "<name> Support", where its gem effect has no name
    /// of its own); `None` (the default) when the data has no such base.
    fn item_base_name(&self, id: &str) -> Option<&str> {
        let _ = id;
        None
    }

    /// A skill gem by its base item ID. Data without skill tables gives
    /// `None` for every skill read: a calculation then reports the gem as
    /// unknown, never a value.
    fn skill_gem(&self, base_item: &str) -> Option<&SkillGem>;

    /// A gem effect by ID.
    fn gem_effect(&self, id: &str) -> Option<&GemEffect>;

    /// A granted effect by ID.
    fn granted_effect(&self, id: &str) -> Option<&GrantedEffect>;

    /// An active skill by ID.
    fn active_skill(&self, id: &str) -> Option<&ActiveSkill>;

    /// A stat set by ID.
    fn stat_set(&self, id: &str) -> Option<&StatSet>;

    /// The game's monster stats at a level (`DefaultMonsterStats`).
    fn monster_level(&self, level: i64) -> Option<MonsterStats>;

    /// A mod's first six stats in row order (PoB's exporter's), each with
    /// its lowest value (P6-7b3a, a minion's mod list); `None` (the
    /// default) when the data has no such mod.
    fn mod_stats(&self, id: &str) -> Option<Vec<(String, i64)>> {
        let _ = id;
        None
    }

    /// The game's monster life columns at a level (`DefaultMonsterStats`;
    /// P6-7b3a, a minion's base life); `None` (the default) when the data
    /// has no such row.
    fn monster_life(&self, level: i64) -> Option<MonsterLife> {
        let _ = level;
        None
    }

    /// The game's minion damage at a level (`DefaultMonsterStats`'s
    /// `MinionDamage`, PoB's `monsterAllyDamageTable`; P6-7c), the stored
    /// single-precision number widened as PoB's table holds it; `None` (the
    /// default) when the data has no such row.
    fn monster_ally_damage(&self, level: i64) -> Option<f64> {
        let _ = level;
        None
    }

    /// A monster type's armour and evasion multipliers, percent.
    fn monster_type(&self, id: &str) -> Option<MonsterDefences>;

    /// A game constant as `(value, divisor)`.
    fn game_constant(&self, id: &str) -> Option<(i64, i64)>;

    /// The Path of Exile only tables ([`crate::poe1::Poe1Data`]; P3-12g).
    /// `None` (the default) when the data has none: a build that needs
    /// them is `Indeterminate` (`missing-data`), never a silent zero.
    fn poe1(&self) -> Option<&dyn crate::poe1::Poe1Data> {
        None
    }

    /// The game's monster damage at a level (`DefaultMonsterStats`, PoB's
    /// `monsterDamageTable`; P6-10b part 12): `None` (the default) when the
    /// data has none.
    fn monster_damage(&self, level: i64) -> Option<f64> {
        let _ = level;
        None
    }

    /// The mods by an implicit tag (P3-8n): the stat sets of the mods that
    /// carry `tag`, and every stat of a mod that does not. `None` (the
    /// default) when the data carries no implicit tags (a bundle before
    /// data schema 3): a condition that needs them stays unsettled.
    fn mods_by_tag(&self, _tag: &str) -> Option<ModsByTag> {
        None
    }

    /// Whether a passive node is a notable, and whether it is an
    /// ascendancy's (P3-8n: which node an item's "Allocates <name>"
    /// grants). `None` (the default) when the data does not say.
    fn passive_kind(&self, _node: u32) -> Option<PassiveKind> {
        None
    }
}

/// The name PoB counts Life Masteries by (`CalcSetup.lua` 689; P6-11l).
pub const LIFE_MASTERY: &str = "Life Mastery";

/// A passive node's kind ([`CalcData::passive_kind`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PassiveKind {
    /// A notable.
    pub notable: bool,
    /// An ascendancy's node.
    pub ascendancy: bool,
    /// A Life Mastery (P6-11l: PoB counts allocated nodes named "Life
    /// Mastery", `CalcSetup.lua` 689).
    pub life_mastery: bool,
}

/// The mods by an implicit tag ([`CalcData::mods_by_tag`]).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ModsByTag {
    /// Each mod that carries the tag (mods without stats left out).
    pub tagged: Vec<TaggedMod>,
    /// Every stat ID of a mod that does not carry the tag.
    pub untagged: std::collections::BTreeSet<String>,
}

/// A mod that carries a tag ([`ModsByTag::tagged`]).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TaggedMod {
    /// The mod's ID.
    pub id: String,
    /// Its stat IDs.
    pub stats: Vec<String>,
}

/// The first data schema whose mods carry their implicit tags.
pub const IMPLICIT_TAGS_SCHEMA: u32 = 3;

/// The game's monster stats at one level ([`CalcData::monster_level`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MonsterStats {
    /// Evasion rating.
    pub evasion: i64,
    /// Accuracy rating.
    pub accuracy: i64,
}

/// The monster life columns of a level ([`CalcData::monster_life`]), as
/// PoB's tables name them (`src/Data/Misc.lua`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MonsterLife {
    /// `monsterLifeTable` (`Life`).
    pub life: i64,
    /// `monsterAllyLifeTable` (`MinionLife`).
    pub ally: i64,
    /// `monsterLifeTable2` (`AltLife1`), when the data has the column.
    pub alt1: Option<i64>,
    /// `monsterLifeTable3` (`AltLife2`).
    pub alt2: i64,
}

/// A monster type's defence multipliers, percent
/// ([`CalcData::monster_type`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MonsterDefences {
    /// Armour multiplier.
    pub armour: i64,
    /// Evasion multiplier.
    pub evasion: i64,
}

/// What an item base is, for its properties ([`CalcData::item_base`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ItemBase {
    /// An armour piece (or shield) with its defences.
    Armour(ArmourBase),
    /// A weapon with its properties.
    Weapon(WeaponBase),
    /// Another base (jewellery, quivers, jewels, flasks).
    Other,
    /// Bases of this name have different properties (PoE1's Two-Toned
    /// Boots: three bases, one name).
    Ambiguous,
}

/// A range of base values a property rolls in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Range {
    /// The lowest roll.
    pub min: i64,
    /// The highest roll.
    pub max: i64,
}

/// An armour base's defences (the armour types and shield types tables).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ArmourBase {
    /// Armour.
    pub armour: Range,
    /// Evasion rating.
    pub evasion: Range,
    /// Energy shield.
    pub energy_shield: Range,
    /// Ward (P6-11hn).
    pub ward: Range,
    /// A shield's chance to block, percent.
    pub block: Option<i64>,
    /// The base's movement speed change, percent (the armour types table;
    /// P6-10b: PoB's `MovementPenalty`, `Item.lua` 2361).
    pub movement_speed: i64,
}

/// A weapon base's properties (the weapon types table) and its item class.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct WeaponBase {
    /// The item class, by ID (`One Hand Mace`, ...): which skills can use
    /// the weapon (an active skill's weapon classes).
    pub class: String,
    /// Physical damage.
    pub physical: Range,
    /// Attack time, milliseconds.
    pub attack_time: i64,
    /// Critical strike chance, hundredths of a percent.
    pub crit_chance: i64,
}

/// An item's base name as the import adapter found it (the adapter's
/// format rules, as `resolve_build` takes them): the name, and a name to
/// try when it is not a base.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BaseName {
    /// The name to look up.
    pub name: String,
    /// A name to look up when `name` is not a base.
    pub fallback: Option<String>,
}

impl BaseName {
    /// The base of this name (else the fallback's) in `data`.
    #[must_use]
    pub fn find(&self, data: &dyn CalcData) -> Option<ItemBase> {
        data.item_base(&self.name)
            .or_else(|| self.fallback.as_deref().and_then(|f| data.item_base(f)))
    }
}

impl CalcData for Bundle {
    fn data_version(&self) -> DataVersion {
        DataVersion {
            game: self.game,
            schema_version: self.manifest.data_schema,
            steam_build: self.manifest.source.steam_build,
        }
    }

    fn character(&self, class: &str) -> Option<CharacterBase> {
        let c = self
            .passives
            .as_ref()?
            .characters
            .iter()
            .find(|c| c.name == class)?;
        Some(CharacterBase {
            base_life: c.base_life,
            base_mana: c.base_mana,
            strength: c.strength,
            dexterity: c.dexterity,
            intelligence: c.intelligence,
        })
    }

    fn character_constant(&self, stat: &str) -> Option<i64> {
        template_stat(self, CHARACTER_TEMPLATE, stat)
    }

    fn monster_constant(&self, stat: &str) -> Option<i64> {
        template_stat(self, MONSTER_TEMPLATE, stat)
    }

    fn passive_stats(&self, node: u32) -> Option<&[StatAmount]> {
        self.passives
            .as_ref()?
            .passives
            .iter()
            .find(|p| p.node == i64::from(node))
            .map(|p| p.stats.as_slice())
    }

    fn mastery_effect_stats(&self, effect: u32) -> Option<&[StatAmount]> {
        self.passives
            .as_ref()?
            .mastery_effects
            .iter()
            .find(|e| e.hash == i64::from(effect))
            .map(|e| e.stats.as_slice())
    }

    fn item_base(&self, name: &str) -> Option<ItemBase> {
        let rows: Vec<_> = self.base_items.iter().filter(|b| b.name == name).collect();
        // Without the property tables (format 1) no base's properties are
        // known: `None`, so the calculation reports missing data rather
        // than calling an armour piece something else.
        let tables = self.properties.as_ref()?;
        if rows.is_empty() {
            return None;
        }
        let tables = Some(tables);
        let range = |r: &loom_data::bundle::properties::ValueRange| Range {
            min: r.min,
            max: r.max,
        };
        let found: Vec<ItemBase> = rows
            .iter()
            .map(|row| {
                let id = &row.id;
                let armour =
                    tables.and_then(|t| t.armour_types.iter().find(|a| a.base_item == *id));
                let weapon =
                    tables.and_then(|t| t.weapon_types.iter().find(|w| w.base_item == *id));
                if let Some(a) = armour {
                    let block = tables
                        .and_then(|t| t.shield_types.iter().find(|s| s.base_item == *id))
                        .map(|s| s.block);
                    ItemBase::Armour(ArmourBase {
                        armour: range(&a.armour),
                        evasion: range(&a.evasion),
                        energy_shield: range(&a.energy_shield),
                        ward: range(&a.ward),
                        block,
                        movement_speed: a.movement_speed,
                    })
                } else if let Some(w) = weapon {
                    ItemBase::Weapon(WeaponBase {
                        class: row.item_class.clone().unwrap_or_default(),
                        physical: Range {
                            min: w.damage_min,
                            max: w.damage_max,
                        },
                        attack_time: w.attack_time,
                        crit_chance: w.crit_chance,
                    })
                } else {
                    ItemBase::Other
                }
            })
            .collect();
        let first = found[0].clone();
        Some(if found.iter().all(|f| *f == first) {
            first
        } else {
            ItemBase::Ambiguous
        })
    }

    fn item_base_name(&self, id: &str) -> Option<&str> {
        self.base_items
            .iter()
            .find(|b| b.id == id)
            .map(|b| b.name.as_str())
    }

    fn skill_gem(&self, base_item: &str) -> Option<&SkillGem> {
        self.skills
            .as_ref()?
            .gems
            .iter()
            .find(|g| g.base_item == base_item)
    }

    fn gem_effect(&self, id: &str) -> Option<&GemEffect> {
        self.skills
            .as_ref()?
            .gem_effects
            .iter()
            .find(|e| e.id == id)
    }

    fn granted_effect(&self, id: &str) -> Option<&GrantedEffect> {
        self.skills
            .as_ref()?
            .granted_effects
            .iter()
            .find(|e| e.id == id)
    }

    fn active_skill(&self, id: &str) -> Option<&ActiveSkill> {
        self.skills
            .as_ref()?
            .active_skills
            .iter()
            .find(|a| a.id == id)
    }

    fn stat_set(&self, id: &str) -> Option<&StatSet> {
        self.skills.as_ref()?.stat_sets.iter().find(|s| s.id == id)
    }

    fn monster_level(&self, level: i64) -> Option<MonsterStats> {
        // Levels run 1, 2, 3, ... without gaps (validated): level n is row
        // n - 1.
        let row = self
            .properties
            .as_ref()?
            .monster_levels
            .get(usize::try_from(level.checked_sub(1)?).ok()?)?;
        (row.level == level).then_some(MonsterStats {
            evasion: row.evasion,
            accuracy: row.accuracy,
        })
    }

    fn monster_damage(&self, level: i64) -> Option<f64> {
        let row = self
            .properties
            .as_ref()?
            .monster_levels
            .get(usize::try_from(level.checked_sub(1)?).ok()?)?;
        (row.level == level).then_some(f64::from(row.damage))
    }

    fn mod_stats(&self, id: &str) -> Option<Vec<(String, i64)>> {
        self.mods
            .iter()
            .find(|m| m.id == id)
            // PoB's exporter reads the first six (`minions.lua` 228; the
            // data-parity rule `minion-mods-six-stat-slots`).
            .map(|m| {
                m.stats
                    .iter()
                    .take(6)
                    .map(|s| (s.stat.clone(), s.min))
                    .collect()
            })
    }

    fn monster_life(&self, level: i64) -> Option<MonsterLife> {
        let row = self
            .properties
            .as_ref()?
            .monster_levels
            .get(usize::try_from(level.checked_sub(1)?).ok()?)?;
        (row.level == level).then_some(MonsterLife {
            life: row.life,
            ally: row.ally_life,
            alt1: row.alt_life_1,
            alt2: row.alt_life_2,
        })
    }

    fn monster_ally_damage(&self, level: i64) -> Option<f64> {
        let row = self
            .properties
            .as_ref()?
            .monster_levels
            .get(usize::try_from(level.checked_sub(1)?).ok()?)?;
        (row.level == level).then_some(f64::from(row.ally_damage))
    }

    fn monster_type(&self, id: &str) -> Option<MonsterDefences> {
        self.properties
            .as_ref()?
            .monster_types
            .iter()
            .find(|m| m.id == id)
            .map(|m| MonsterDefences {
                armour: m.armour,
                evasion: m.evasion,
            })
    }

    fn game_constant(&self, id: &str) -> Option<(i64, i64)> {
        self.properties
            .as_ref()?
            .game_constants
            .iter()
            .find(|c| c.id == id)
            .map(|c| (c.value, c.divisor))
    }

    fn poe1(&self) -> Option<&dyn crate::poe1::Poe1Data> {
        Some(self)
    }

    fn passive_kind(&self, node: u32) -> Option<PassiveKind> {
        let p = self
            .passives
            .as_ref()?
            .passives
            .iter()
            .find(|p| p.node == i64::from(node))?;
        Some(PassiveKind {
            notable: p.notable,
            ascendancy: p.ascendancy.is_some(),
            life_mastery: p.name == LIFE_MASTERY,
        })
    }

    fn mods_by_tag(&self, tag: &str) -> Option<ModsByTag> {
        if self.manifest.data_schema < IMPLICIT_TAGS_SCHEMA {
            return None;
        }
        let mut out = ModsByTag::default();
        for m in &self.mods {
            if m.stats.is_empty() {
                continue;
            }
            if m.implicit_tags.iter().any(|t| t == tag) {
                out.tagged.push(TaggedMod {
                    id: m.id.clone(),
                    stats: m.stats.iter().map(|s| s.stat.clone()).collect(),
                });
            } else {
                out.untagged.extend(m.stats.iter().map(|s| s.stat.clone()));
            }
        }
        Some(out)
    }
}

/// A stat of the object template `template` in `bundle`.
fn template_stat(bundle: &Bundle, template: &str, stat: &str) -> Option<i64> {
    bundle
        .passives
        .as_ref()?
        .object_templates
        .iter()
        .find(|t| t.id == template)?
        .stats
        .iter()
        .find(|s| s.stat == stat)
        .map(|s| s.value)
}

/// A skill picked by position.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SkillChoice {
    /// The socket group's index.
    pub group: usize,
    /// The active skill's index within the group.
    pub active: usize,
}

/// What to calculate. `None` takes the build's own choice.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CalcOptions {
    /// The skill (else the build's main skill).
    pub skill: Option<SkillChoice>,
    /// The passive spec (else the active one).
    pub spec: Option<SpecId>,
    /// The item set (else the active one).
    pub item_set: Option<ItemSetId>,
    /// The config set (else the active one).
    pub config_set: Option<ConfigSetId>,
    /// The tree version of the data's passive tree, as builds name it
    /// (`3_29`). The bundle does not carry the label; the host gives it
    /// (the parity tool reads it from the scope file, which ties it to the
    /// Steam build with evidence). `None`: a build that allocates passives
    /// is `Partial` with `UncheckedTreeVersion`.
    pub tree_version: Option<TreeVersion>,
    /// Each item's base name, as the import adapter names it (its
    /// format's rules, which the core does not know: a legacy name line
    /// holds affixes around the base). The host gives them, as it gives
    /// `resolve_build` its base names. An armour piece without one is
    /// `Partial` (`unmodelled-item`, `no-base-name`).
    pub item_bases: BTreeMap<ItemId, BaseName>,
    /// What the import adapter settled about item lines' values, by item
    /// and line index ([`LineValues`]): its format's rules (PoB's roll
    /// `{range:r}`, catalyst quality), which the core does not know. A
    /// line not listed takes its resolution's values; a ranged one stays
    /// unsettled (`unsettled-value`).
    pub item_lines: BTreeMap<ItemId, BTreeMap<usize, LineValues>>,
    /// The active spec's nodes as the host resolved them against the tree
    /// (passive tree plan, T-e): which count. The host builds it from the
    /// tree (`loom-pipeline` with `loom-tree`); the engine does not walk
    /// the tree's links.
    pub passives: ResolvedPassives,
    /// The main skill as a mirage uses it (Phase 6 P6-8b): the engine sets
    /// it when it computes a mirage's copy of the skill (PoB's
    /// `calcs.copyActiveSkill`, `CalcMirages.lua` 37 to 52). A host leaves
    /// it `None`.
    pub mirage: Option<MirageUse>,
    /// A sub-calculation of the engine's own (P6-12: a trigger's source, a
    /// mirage's copy, a skill of the full damage per second): it computes
    /// no full damage per second itself. A host leaves it `false`.
    pub nested: bool,
}

/// What a mirage changes in its copy of the main skill (P6-8b): Mirage
/// Archer's `Damage` and `Speed` MORE, percent (`CalcMirages.lua` 69 to
/// 84).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MirageUse {
    /// `Damage` MORE, percent.
    pub damage_more: i64,
    /// `Speed` MORE, percent.
    pub speed_more: i64,
}

/// The active spec's allocated nodes, each with what the tree makes of it
/// ([`CalcOptions::passives`]). A node it does not list counts as not
/// connected (a node PoB numbers as a cluster jewel's, as one).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ResolvedPassives {
    /// Each allocated node's state.
    pub nodes: BTreeMap<NodeId, PassiveState>,
    /// The nodes the socketed cluster jewels add (passive tree plan, T-i:
    /// `loom_tree::poe1::cluster`), with their stats: a node listed here
    /// takes these, not the passives table's.
    pub cluster: BTreeMap<NodeId, ClusterPassive>,
    /// The tree version's own stats, when the host resolved the spec
    /// against an older version than the data's (a tree pack; passive tree
    /// plan, T4): the passives table and mastery effects are then not
    /// read.
    pub pack: Option<PackStats>,
    /// The nodes a tattoo, a runegraft, or another override replaces
    /// (P6-11av), or a socketed Timeless jewel conquers (P6-10a,
    /// `loom_pipeline::timeless`), with what they become: a node listed
    /// here takes these stats (instead of its own when replaced), and a
    /// mastery listed here takes no chosen effect.
    pub changed: BTreeMap<NodeId, Conquest>,
}

/// What a tattoo or a Timeless jewel makes of a node
/// ([`ResolvedPassives::changed`]; PoB's `PassiveSpec.lua` 1185 to 1376).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Conquest {
    /// The node's own stats are replaced (else the stats are added).
    pub replaced: bool,
    /// The stats it gains, raw game values (a Glorious Vanity roll, else
    /// the row's fixed value).
    pub stats: Vec<StatAmount>,
    /// Why the change is not known (`timeless-results` without the jewel
    /// type's results, `timeless-jewel` for lines that do not read,
    /// `tattoo-data` without the bundle's overrides, `tattoo` for a name
    /// they do not hold, `tattoo-skill` for an override that grants a
    /// skill): the node then counts nothing and is this reason.
    pub reason: Option<&'static str>,
}

/// The stats of a tree version the host resolved the spec against
/// ([`ResolvedPassives::pack`]).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PackStats {
    /// Every node's stats and kind in that version.
    pub passives: BTreeMap<NodeId, PackNode>,
    /// Every mastery effect's stats in that version, by hash.
    pub mastery_effects: BTreeMap<u32, PackNode>,
}

/// A node's or mastery effect's stats in a tree version ([`PackStats`]).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PackNode {
    /// Its stats.
    pub stats: Vec<StatAmount>,
    /// A node's kind in that version (a mastery effect's is the default).
    pub kind: PassiveKind,
    /// Its lines the pack could not read as stats: each one a reason.
    pub unresolved: Vec<String>,
}

/// A node a cluster jewel adds ([`ResolvedPassives::cluster`]).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ClusterPassive {
    /// Its stats.
    pub stats: Vec<StatAmount>,
    /// Their increased effect, percent (a small passive's: the jewel's
    /// "Added Small Passive Skills have #% increased Effect").
    pub effect: i64,
}

impl ResolvedPassives {
    /// The state of `node`: listed, else as the module says.
    #[must_use]
    pub fn state(&self, node: NodeId) -> PassiveState {
        self.nodes
            .get(&node)
            .copied()
            .unwrap_or(if node.0 & PassiveState::CLUSTER_NODE_BIT == 0 {
                PassiveState::Unconnected
            } else {
                PassiveState::ClusterNode
            })
    }

    /// Whether `node` is allocated and connected.
    #[must_use]
    pub fn connected(&self, node: NodeId) -> bool {
        self.nodes.get(&node) == Some(&PassiveState::Connected)
    }
}

/// What an allocated node is in the calculation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PassiveState {
    /// Connected to an allocated class or ascendancy start through
    /// allocated nodes (PoB's `connectedToStart`): its stats count.
    Connected,
    /// Not connected: PoB drops it when it loads a build; its stats do not
    /// count (`unconnected-node`).
    Unconnected,
    /// A cluster jewel socket inside a cluster jewel's subgraph, not
    /// connected: it connects through the jewel (`cluster-jewel-node`).
    ClusterSocket,
    /// A node PoB generates from a socketed cluster jewel
    /// (`cluster-jewel-node`).
    ClusterNode,
}

impl PassiveState {
    /// The bit PoB sets on the nodes of a cluster jewel's subgraph
    /// (`Classes/PassiveSpec.lua` BuildSubgraph: "16: 1 (signal bit, to
    /// prevent conflict with node hashes)").
    pub const CLUSTER_NODE_BIT: u32 = 0x1_0000;
}

/// What an import adapter settled about one item line's values.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LineValues {
    /// The rolled raw value of each stat of the line's resolution, in
    /// order (`None`: the resolution's own value). The engine takes them
    /// only when the line resolved to that many stats.
    Rolled(Vec<Option<i64>>),
    /// The line's values after a scaling the adapter settled (a catalyst's
    /// quality, P6-11ax): each stat's raw value, in order (`None`: the
    /// resolution's own value). Unlike [`Self::Rolled`], a value may lie
    /// outside the resolution's range, and an exact stat's value is
    /// replaced too. The engine takes them only when the line resolved to
    /// that many stats.
    Catalysed(Vec<Option<i64>>),
    /// The format scales the line's values in a way the engine does not
    /// model: the line gives no effect and the fields are `Partial`
    /// (`scaled-line` with the scaling's name).
    Scaled(Scaling),
}

/// A scaling of an item line's values the engine does not model.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Scaling {
    /// A catalyst's quality.
    Catalyst,
    /// A corrupted implicit's range.
    CorruptedRange,
    /// An item's modifier magnitudes.
    ModifierMagnitude,
}

impl Scaling {
    /// Stable name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Catalyst => "catalyst",
            Self::CorruptedRange => "corrupted-range",
            Self::ModifierMagnitude => "modifier-magnitude",
        }
    }
}

/// A calculation's input: an immutable snapshot.
#[derive(Clone, Copy)]
pub struct CalcInput<'a> {
    /// The resolved build.
    pub build: &'a Build,
    /// The game data.
    pub data: &'a dyn CalcData,
    /// The options.
    pub options: &'a CalcOptions,
}

impl core::fmt::Debug for CalcInput<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("CalcInput")
            .field("game", &self.build.game())
            .field("data", &self.data.data_version())
            .field("options", self.options)
            .finish_non_exhaustive()
    }
}

impl CalcInput<'_> {
    /// Checks that the build was made with this data (ADR-010: a data
    /// update never reinterprets a build silently) and is of its game.
    ///
    /// # Errors
    /// [`CalcError::NoDataVersion`] when the build records none,
    /// [`CalcError::DataVersionMismatch`] when it differs from the data's
    /// or the build's game is not the data's.
    pub fn check_data_version(&self) -> Result<(), CalcError> {
        let data = self.data.data_version();
        let build = self.build.data.as_ref().ok_or(CalcError::NoDataVersion)?;
        if *build != data || self.build.game() != data.game {
            return Err(CalcError::DataVersionMismatch {
                build: build.clone(),
                data,
            });
        }
        Ok(())
    }
}
