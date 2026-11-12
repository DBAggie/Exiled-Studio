//! What a calculation reads (ADR-003; Phase 3 plan, decision 7): the
//! resolved build (borrowed, immutable), a view of the game data, and the
//! options.

use std::collections::BTreeMap;

use loom_data::bundle::Bundle;
pub use loom_data::bundle::passives::StatAmount;
pub use loom_data::bundle::skills::{
    ActiveSkill, EffectLevel, GemEffect, GrantedEffect, SkillGem, StatSet, StatSetLevel,
};
use loom_domain::{Build, ConfigSetId, DataVersion, ItemId, ItemSetId, SpecId, TreeVersion};

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

    /// A passive node's place in the tree: whether it is a root (a class
    /// or ascendancy start) and the nodes it is linked to, either way.
    fn passive_links(&self, node: u32) -> Option<PassiveLinks>;

    /// The properties of the item base with this name; `None` when no
    /// base has the name.
    fn item_base(&self, name: &str) -> Option<ItemBase>;

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

    /// A monster type's armour and evasion multipliers, percent.
    fn monster_type(&self, id: &str) -> Option<MonsterDefences>;

    /// A game constant as `(value, divisor)`.
    fn game_constant(&self, id: &str) -> Option<(i64, i64)>;
}

/// The game's monster stats at one level ([`CalcData::monster_level`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MonsterStats {
    /// Evasion rating.
    pub evasion: i64,
    /// Accuracy rating.
    pub accuracy: i64,
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
    /// A shield's chance to block, percent.
    pub block: Option<i64>,
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

/// A passive node's links in the tree ([`CalcData::passive_links`]).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PassiveLinks {
    /// A class or ascendancy start: allocated nodes connect to one.
    pub root: bool,
    /// A cluster jewel socket: one inside a cluster jewel's subgraph
    /// connects through the jewel, not the tree.
    pub expansion: bool,
    /// The linked nodes, sorted and unique.
    pub links: Vec<u32>,
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

    fn passive_links(&self, node: u32) -> Option<PassiveLinks> {
        let tables = self.passives.as_ref()?;
        let me = i64::from(node);
        let passive = tables.passives.iter().find(|p| p.node == me)?;
        let mut links = Vec::new();
        // The tree stores each link once, on one of its two nodes.
        for n in tables.tree.groups.iter().flat_map(|g| &g.nodes) {
            if n.node == me {
                links.extend(
                    n.connections
                        .iter()
                        .filter_map(|c| u32::try_from(c.node).ok()),
                );
            } else if n.connections.iter().any(|c| c.node == me)
                && let Ok(other) = u32::try_from(n.node)
            {
                links.push(other);
            }
        }
        links.sort_unstable();
        links.dedup();
        Some(PassiveLinks {
            root: !passive.characters.is_empty() || passive.ascendancy_start,
            expansion: passive.expansion,
            links,
        })
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
                        block,
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
