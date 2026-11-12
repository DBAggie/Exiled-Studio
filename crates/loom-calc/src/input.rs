//! What a calculation reads (ADR-003; Phase 3 plan, decision 7): the
//! resolved build (borrowed, immutable), a view of the game data, and the
//! options.

use loom_data::bundle::Bundle;
pub use loom_data::bundle::passives::StatAmount;
use loom_domain::{Build, ConfigSetId, DataVersion, ItemSetId, SpecId, TreeVersion};

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

    /// The stats of the passive node with this number.
    fn passive_stats(&self, node: u32) -> Option<&[StatAmount]>;

    /// The stats of the mastery effect with this hash.
    fn mastery_effect_stats(&self, effect: u32) -> Option<&[StatAmount]>;

    /// A passive node's place in the tree: whether it is a root (a class
    /// or ascendancy start) and the nodes it is linked to, either way.
    fn passive_links(&self, node: u32) -> Option<PassiveLinks>;
}

/// A passive node's links in the tree ([`CalcData::passive_links`]).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PassiveLinks {
    /// A class or ascendancy start: allocated nodes connect to one.
    pub root: bool,
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
        self.passives
            .as_ref()?
            .object_templates
            .iter()
            .find(|t| t.id == CHARACTER_TEMPLATE)?
            .stats
            .iter()
            .find(|s| s.stat == stat)
            .map(|s| s.value)
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
            links,
        })
    }
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
