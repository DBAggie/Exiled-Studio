//! The Path of Exile only section of a bundle (`poe1/`; ADR-001, Phase 2
//! plan decision 5): data the PoE1 item pipeline reads that PoE2 does not
//! have. Never required by the loader; a PoE2 bundle has none (the
//! validator refuses it there, plan step 4).
//!
//! - `poe1/quality_types.json`: the alternate quality types
//!   (`AlternateQualityTypes`), each with its catalyst mod's fixed stats.
//! - `poe1/grafts.json`: the graft bases (`BrequelGrafts`) with the skill
//!   each grants, and the allowed skill levels
//!   (`BrequelGraftGrantedSkillLevels`). Essences need no file: their item
//!   classes are precomputed into the mod rows (`essence_classes`).
//! - `poe1/pantheons.json`: the pantheon's gods (`PantheonPanelLayout`,
//!   rows not disabled), each with its souls and their stats (P3-12g; the
//!   calc applies a chosen god's souls).
//! - `poe1/jewel_radii.json`: the jewel radii (`PassiveJewelRadii`), each
//!   with its name, radius, and ring (passive tree plan, T-f: which nodes
//!   a radius jewel reaches).
//! - `poe1/unique_flavour.json`: unique items' flavour text (`FlavourText`,
//!   found through `UniqueStashLayout`; B-2's item card, Q-B2-1), each with
//!   the unique's name and the text's ID, as the game writes it.
//! - `poe1/tree_flavour.json`: the passive tree's texts the game shows: a
//!   keystone's flavour text (`PassiveSkills.FlavourText`) and an
//!   ascendancy's quote (`Ascendancy.FlavourText`, with its colour; B-2
//!   item evidence, E10).
//! - `poe1/socket_caps.json`: how many sockets each base can roll by item
//!   level (the item type files' `Sockets` component, `socket_info`, the
//!   nearest file of the base's chain that sets it; B-2 item evidence).
//! - `poe1/cluster_jewels.json`: what a socketed cluster jewel adds to the
//!   tree (passive tree plan, T-i): the jewel sizes, each cluster jewel
//!   base's node layout, the small passives a jewel can grant, the notables
//!   and keystones it can add, and the tree's cluster sockets
//!   ([`ClusterJewels`]).
//! - `poe1/flasks.json`: the flask bases (`Flasks`, Phase 6 P6-4b), each
//!   with its recovery, duration, and buff's stats ([`Flask`]; the calc
//!   applies an active flask's buff).

use serde::{Deserialize, Serialize};

use super::{BundleError, BundleSource, Manifest, json};

/// The section's folder.
pub const DIR: &str = "poe1/";
/// Quality types.
pub const QUALITY_TYPES: &str = "poe1/quality_types.json";
/// Grafts.
pub const GRAFTS: &str = "poe1/grafts.json";
/// Pantheons.
pub const PANTHEONS: &str = "poe1/pantheons.json";
/// Jewel radii.
pub const JEWEL_RADII: &str = "poe1/jewel_radii.json";
/// Unique flavour text.
pub const UNIQUE_FLAVOUR: &str = "poe1/unique_flavour.json";
/// Socket caps by base.
pub const SOCKET_CAPS: &str = "poe1/socket_caps.json";
/// The passive tree's flavour texts.
pub const TREE_FLAVOUR: &str = "poe1/tree_flavour.json";
/// Cluster jewels.
pub const CLUSTER_JEWELS: &str = "poe1/cluster_jewels.json";
/// Flask bases.
pub const FLASKS: &str = "poe1/flasks.json";

/// A flask base (`Flasks`, P6-4b), in the table's order. PoB v2.67.2
/// exports its `flask` base data from the same row
/// (`src/Export/Scripts/bases.lua` 247 to 270).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Flask {
    /// The base item's ID (`BaseItemTypes.Id`).
    pub base: String,
    /// The base item's name, as builds write it.
    pub name: String,
    /// Life recovered per use (`LifePerUse`; 0 for none).
    pub life: i64,
    /// Mana recovered per use (`ManaPerUse`; 0 for none).
    pub mana: i64,
    /// The duration in tenths of a second (`RecoveryTime`).
    pub duration: i64,
    /// The buff's ID (`BuffDefinitions.Id`), when the flask has one.
    #[serde(default)]
    pub buff: Option<String>,
    /// The buff's stats (`(stat, value)`): its `StatsKeys` with
    /// `BuffStatValues` in order, then its `GrantedFlags` at 1, as PoB's
    /// exporter reads them.
    #[serde(default)]
    pub stats: Vec<(String, i64)>,
}

/// What a socketed cluster jewel adds to the tree (module docs), from the
/// game's `PassiveTreeExpansionJewelSizes`, `PassiveTreeExpansionJewels`,
/// `PassiveTreeExpansionSkills`, `PassiveTreeExpansionSpecialSkills`, and
/// `PassiveJewelSlots`, each in its table's order. Node numbers are
/// `PassiveSkillGraphId`s, as the passives table's.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClusterJewels {
    /// The sizes' names (`PassiveTreeExpansionJewelSizes.Name`): a size is
    /// its row's index (0 small, 1 medium, 2 large in the game).
    pub sizes: Vec<String>,
    /// Each cluster jewel base's node layout.
    pub jewels: Vec<ClusterJewelBase>,
    /// The small passives a jewel can grant: a jewel names one by its row's
    /// index (`local_jewel_expansion_passive_node_index`).
    pub skills: Vec<ClusterSkill>,
    /// The notables and keystones a jewel can add, each with the stat that
    /// adds it.
    pub specials: Vec<ClusterSpecial>,
    /// The tree's jewel slots that take a cluster jewel.
    pub slots: Vec<ClusterSlot>,
    /// What each "Added Small Passive Skills also grant: ..." stat grants
    /// each small passive (`local_affliction_jewel_small_nodes_grant_*`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub grants: Vec<ClusterGrant>,
}

/// An "Added Small Passive Skills also grant: ..." stat and the stat it
/// grants each small passive, at the same value: paired by the bundle
/// step from the two stats' descriptions (the granted line's text after
/// "also grant: "), or from its short list of pairs the texts word apart.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClusterGrant {
    /// The jewel's stat.
    pub stat: String,
    /// The stat each small passive gains.
    pub granted: String,
}

/// A cluster jewel base's node layout (`PassiveTreeExpansionJewels`): the
/// orbit indices its sockets, notables, and small passives take, in the
/// order they are filled.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClusterJewelBase {
    /// The base item's ID.
    pub base: String,
    /// Its size ([`ClusterJewels::sizes`]).
    pub size: u32,
    /// The fewest passives it adds.
    pub min_nodes: u32,
    /// The most passives it adds.
    pub max_nodes: u32,
    /// The indices small passives take.
    pub small_indices: Vec<u32>,
    /// The indices notables take.
    pub notable_indices: Vec<u32>,
    /// The indices jewel sockets take.
    pub socket_indices: Vec<u32>,
    /// The indices around its ring.
    pub total_indices: u32,
}

/// A small passive a cluster jewel can grant
/// (`PassiveTreeExpansionSkills`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClusterSkill {
    /// The passive whose name and stats each small passive takes.
    pub node: i64,
    /// The mastery passive drawn in the jewel's middle, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mastery: Option<i64>,
    /// Its tag's ID (`Tags.Id`).
    pub tag: String,
    /// The jewel size it is for ([`ClusterJewels::sizes`]).
    pub size: u32,
}

/// A notable or keystone a cluster jewel can add
/// (`PassiveTreeExpansionSpecialSkills`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClusterSpecial {
    /// The passive it adds.
    pub node: i64,
    /// The jewel stat that adds it (`local_affliction_notable_...`,
    /// `local_jewel_expansion_keystone_...`).
    pub stat: String,
    /// The stat's row in `Stats`: a jewel's notables take their places in
    /// this order.
    pub order: i64,
    /// Whether the passive is a keystone (a jewel adding it adds only it);
    /// else a notable.
    #[serde(default)]
    pub keystone: bool,
}

/// A jewel slot that takes a cluster jewel (`PassiveJewelSlots` rows with
/// a size).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClusterSlot {
    /// The socket passive.
    pub socket: i64,
    /// The largest jewel it takes ([`ClusterJewels::sizes`]).
    pub size: u32,
    /// Its index among the sockets of a cluster (`ClusterIndex`).
    pub index: i64,
    /// The socket whose jewel's cluster holds it, for a nested socket
    /// (`ReplacesSlot`'s socket).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<i64>,
    /// The position proxy: the passive whose group places the jewel's
    /// cluster.
    pub proxy: i64,
    /// The orbit index each jewel size's cluster starts at, by size.
    pub start_indices: Vec<u32>,
}

/// A row of [`CLUSTER_JEWELS`] out of range: where, and why.
pub(super) type Finding = (String, &'static str);

impl ClusterJewels {
    /// The base `base`'s layout, if it is a cluster jewel's.
    #[must_use]
    pub fn jewel(&self, base: &str) -> Option<&ClusterJewelBase> {
        self.jewels.iter().find(|j| j.base == base)
    }

    /// The slot of socket `socket`, if it takes a cluster jewel.
    #[must_use]
    pub fn slot(&self, socket: i64) -> Option<&ClusterSlot> {
        self.slots.iter().find(|s| s.socket == socket)
    }

    /// The stat a small passive gains from the jewel stat `stat`, if any.
    #[must_use]
    pub fn grant(&self, stat: &str) -> Option<&str> {
        self.grants
            .iter()
            .find(|g| g.stat == stat)
            .map(|g| g.granted.as_str())
    }

    /// The special that stat `stat` adds, if any.
    #[must_use]
    pub fn special(&self, stat: &str) -> Option<&ClusterSpecial> {
        self.specials.iter().find(|s| s.stat == stat)
    }

    /// The validator's checks: the passives the file names (each with
    /// where), and the rows whose values are out of range.
    pub(super) fn domain(&self) -> (Vec<(String, i64)>, Vec<Finding>) {
        let sizes = u32::try_from(self.sizes.len()).unwrap_or(u32::MAX);
        let mut nodes = Vec::new();
        let mut bad = Vec::new();
        for (i, j) in self.jewels.iter().enumerate() {
            let at = format!("{CLUSTER_JEWELS}:jewels:{i}");
            let fits = |v: &[u32]| v.iter().all(|x| *x < j.total_indices);
            if j.size >= sizes
                || j.min_nodes > j.max_nodes
                || j.total_indices == 0
                || !fits(&j.small_indices)
                || !fits(&j.notable_indices)
                || !fits(&j.socket_indices)
            {
                bad.push((
                    at,
                    "a size the file lacks, more fewest nodes than most, or an index off the ring",
                ));
            }
        }
        for (i, s) in self.skills.iter().enumerate() {
            let at = format!("{CLUSTER_JEWELS}:skills:{i}");
            nodes.push((at.clone(), s.node));
            nodes.extend(s.mastery.map(|m| (at.clone(), m)));
            if s.size >= sizes {
                bad.push((at, "a size the file lacks"));
            }
        }
        for (i, s) in self.specials.iter().enumerate() {
            nodes.push((format!("{CLUSTER_JEWELS}:specials:{i}"), s.node));
        }
        for s in &self.slots {
            let at = format!("{CLUSTER_JEWELS}:slots:{}", s.socket);
            nodes.push((at.clone(), s.socket));
            nodes.push((at.clone(), s.proxy));
            nodes.extend(s.parent.map(|p| (at.clone(), p)));
            if s.size >= sizes || s.start_indices.is_empty() {
                bad.push((at, "a size the file lacks, or no start index"));
            }
        }
        (nodes, bad)
    }
}

/// The passive tree's flavour texts (module docs), each as the game stores
/// it with its line breaks as `\n`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TreeFlavour {
    /// The passives with a flavour text, by node number.
    pub passives: Vec<PassiveFlavour>,
    /// The ascendancies with a quote, by ID.
    pub ascendancies: Vec<AscendancyFlavour>,
}

/// A passive's flavour text ([`TreeFlavour`]).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PassiveFlavour {
    /// The node number (`PassiveSkillGraphId`).
    pub node: i64,
    /// The text.
    pub text: String,
}

/// An ascendancy's quote ([`TreeFlavour`]).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AscendancyFlavour {
    /// The ascendancy's ID (`Ascendancy.Id`, what nodes name).
    pub id: String,
    /// The quote.
    pub text: String,
    /// The colour the game prints it in (`RGBFlavourTextColour`, `r,g,b`),
    /// when the table gives one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub colour: Option<String>,
}

/// The sockets a base can roll (`docs/ui/b2-data/card-data.md`, "Maximum
/// sockets"): the item type's `socket_info` (`count:item level:weight`),
/// the counts that can roll (weight above 0, item level below the game's
/// 9999 "never") with the item level each needs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SocketCaps {
    /// The base item's ID.
    pub base: String,
    /// `(sockets, item level)`, by socket count.
    pub levels: Vec<(u32, u32)>,
}

impl SocketCaps {
    /// The most sockets the base rolls at `item_level`; 0 when none rolls.
    #[must_use]
    pub fn at(&self, item_level: u32) -> u32 {
        self.levels
            .iter()
            .filter(|(_, level)| *level <= item_level)
            .map(|(count, _)| *count)
            .max()
            .unwrap_or(0)
    }
}

/// A unique item's flavour text. The game links them by a name convention,
/// not a key (`docs/ui/b2-data/card-data.md`): the unique's art
/// (`ItemVisualIdentity.Id`) names the `FlavourText` row, and a per-game
/// list names the rest (upgraded versions that share their base's art).
/// A name may have several rows: a unique whose variants have their own
/// text (Grand Spectrum).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UniqueFlavour {
    /// The unique's name (`Words.Text2`).
    pub name: String,
    /// The `FlavourText` row's ID.
    pub id: String,
    /// The text as the game stores it, with line breaks as `\n` (the
    /// table's `\r\n` made `\n`) and the game's markup (`<default>{...}`,
    /// `<<...>>` glyphs) kept.
    pub text: String,
}

/// A jewel radius (`PassiveJewelRadii`), in the game table's units: the
/// tree before 3.16's, which `loom_tree::poe1::radius::scaled` turns into
/// the layout's tree units (times 1.2). A jewel with this radius reaches
/// the nodes whose distance from its socket is at most `radius`; a ring
/// jewel (Thread of Hope) the nodes from `ring_inner` to `ring_outer`,
/// both included.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JewelRadius {
    /// ID.
    pub id: String,
    /// The name an item's `Radius:` line shows (`ClientStrings`); empty
    /// when the extract has none.
    pub name: String,
    /// The radius.
    pub radius: i64,
    /// The ring's inner radius.
    pub ring_inner: i64,
    /// The ring's outer radius.
    pub ring_outer: i64,
}

/// A god of the pantheon (`PantheonPanelLayout`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Pantheon {
    /// ID (`TheBrineKing`, ...): the name PoB's config stores.
    pub id: String,
    /// A major god (else a minor one).
    pub major: bool,
    /// The god's souls with stats, in the table's order (the god's own
    /// soul first, then its upgrades).
    pub souls: Vec<PantheonSoul>,
}

/// One soul of a god: its name and stats (`Effect<n>_StatsKeys` with
/// `Effect<n>_Values`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PantheonSoul {
    /// The soul's name (`GodName<n>`).
    pub name: String,
    /// The soul's stats (`(stat, value)`), in the table's order.
    pub stats: Vec<(String, i64)>,
}

/// An alternate quality type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QualityType {
    /// ID.
    pub id: String,
    /// The description an item line shows.
    pub description: String,
    /// The catalyst mod's fixed stats (`(stat, value)`).
    #[serde(default)]
    pub stats: Vec<(String, i64)>,
}

/// A graft base and the skill it grants.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GraftBase {
    /// The base item's name.
    pub base: String,
    /// The graft's ID.
    pub graft: String,
    /// The granted skill gem's ID.
    pub gem: String,
    /// The granted skill's name (the gem's base item name).
    pub skill: String,
}

/// `poe1/grafts.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Grafts {
    /// Graft bases.
    pub bases: Vec<GraftBase>,
    /// Allowed skill levels.
    pub levels: Vec<u32>,
}

/// The section.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Poe1Section {
    /// Quality types (`None` when the file is absent).
    pub quality_types: Option<Vec<QualityType>>,
    /// Grafts (`None` when the file is absent).
    pub grafts: Option<Grafts>,
    /// Pantheons (`None` when the file is absent).
    pub pantheons: Option<Vec<Pantheon>>,
    /// Jewel radii (`None` when the file is absent).
    pub jewel_radii: Option<Vec<JewelRadius>>,
    /// Unique flavour text (`None` when the file is absent).
    pub unique_flavour: Option<Vec<UniqueFlavour>>,
    /// Socket caps (`None` when the file is absent).
    pub socket_caps: Option<Vec<SocketCaps>>,
    /// The passive tree's flavour texts (`None` when the file is absent).
    pub tree_flavour: Option<TreeFlavour>,
    /// Cluster jewels (`None` when the file is absent).
    pub cluster_jewels: Option<ClusterJewels>,
    /// Flask bases (`None` when the file is absent).
    pub flasks: Option<Vec<Flask>>,
}

impl Poe1Section {
    /// The stats the section's build choices (the pantheon's souls) and
    /// flask buffs name, each with the row it is on: the validator checks
    /// each is a stat.
    pub(super) fn choice_stats(&self) -> Vec<(String, &str)> {
        let mut out = Vec::new();
        for g in self.pantheons.iter().flatten() {
            for (s, _) in g.souls.iter().flat_map(|soul| &soul.stats) {
                out.push((format!("{PANTHEONS}:{}", g.id), s.as_str()));
            }
        }
        for f in self.flasks.iter().flatten() {
            for (s, _) in &f.stats {
                out.push((format!("{FLASKS}:{}", f.base), s.as_str()));
            }
        }
        out
    }

    /// The flasks' domain checks for the validator: each row's base, and
    /// the rows whose values are out of range (a negative recovery or
    /// duration, a name missing, or stats without a buff).
    pub(super) fn flask_domain(&self) -> (Vec<&str>, Vec<(String, &'static str)>) {
        let rows = self.flasks.as_deref().unwrap_or_default();
        let bases = rows.iter().map(|r| r.base.as_str()).collect();
        let bad = rows
            .iter()
            .filter(|r| {
                r.life < 0
                    || r.mana < 0
                    || r.duration < 0
                    || r.name.is_empty()
                    || (r.buff.is_none() && !r.stats.is_empty())
            })
            .map(|r| {
                (
                    format!("{FLASKS}:{}", r.base),
                    "a negative recovery or duration, no name, or stats without a buff",
                )
            })
            .collect();
        (bases, bad)
    }

    /// The first flask base named `name` (builds name a base, and the
    /// game's legacy rows repeat a name), if the file has one.
    #[must_use]
    pub fn flask_named(&self, name: &str) -> Option<&Flask> {
        self.flasks.iter().flatten().find(|f| f.name == name)
    }

    /// The pantheon's domain checks for the validator: the file, its IDs
    /// (each must be unique and not empty), and the rows whose shape is
    /// wrong (a soul without a name or a stat: the converter keeps only
    /// souls with stats, as PoB's export does).
    pub(super) fn choice_domain(&self) -> (&'static str, Vec<&str>, Vec<(String, &'static str)>) {
        let gods = self.pantheons.as_deref().unwrap_or_default();
        let ids = gods.iter().map(|g| g.id.as_str()).collect();
        let bad = gods
            .iter()
            .filter(|g| {
                g.souls
                    .iter()
                    .any(|s| s.name.is_empty() || s.stats.is_empty())
            })
            .map(|g| {
                (
                    format!("{PANTHEONS}:{}", g.id),
                    "a soul without a name or a stat",
                )
            })
            .collect();
        (PANTHEONS, ids, bad)
    }

    /// The jewel radii's domain checks for the validator: unique IDs, and
    /// the rows whose values are out of range (a negative radius, or a
    /// ring whose inner radius is above its outer one) or out of order. A
    /// ring jewel's stat names its ring by row number
    /// (`loom_tree::Tree::ring`), so the rows must stay in the game's
    /// table order, smallest first: each row's radius and ring above the
    /// row before.
    pub(super) fn radius_domain(&self) -> (Vec<&str>, Vec<(String, &'static str)>) {
        let radii = self.jewel_radii.as_deref().unwrap_or_default();
        let ids = radii.iter().map(|r| r.id.as_str()).collect();
        let mut bad: Vec<(String, &'static str)> = radii
            .iter()
            .filter(|r| r.radius < 0 || r.ring_inner < 0 || r.ring_inner > r.ring_outer)
            .map(|r| {
                (
                    format!("{JEWEL_RADII}:{}", r.id),
                    "a negative radius, or a ring whose inner radius is above its outer one",
                )
            })
            .collect();
        bad.extend(
            radii
                .iter()
                .zip(radii.iter().skip(1))
                .filter(|(before, r)| {
                    r.radius <= before.radius
                        || r.ring_inner < before.ring_inner
                        || r.ring_outer <= before.ring_outer
                })
                .map(|(_, r)| {
                    (
                        format!("{JEWEL_RADII}:{}", r.id),
                        "not after the row before: the rows go from the smallest radius and ring up, in the game's order",
                    )
                }),
        );
        (ids, bad)
    }

    /// Unique flavour text's domain checks for the validator: each row's
    /// key (`name:id`, each must be unique), the number of rows without a
    /// name or an ID, and the rows with no text.
    pub(super) fn flavour_domain(&self) -> (Vec<String>, usize, Vec<(String, &'static str)>) {
        let rows = self.unique_flavour.as_deref().unwrap_or_default();
        let keys = rows
            .iter()
            .map(|r| format!("{}:{}", r.name, r.id))
            .collect();
        let empty = rows
            .iter()
            .filter(|r| r.name.is_empty() || r.id.is_empty())
            .count();
        let bad = rows
            .iter()
            .filter(|r| r.text.trim().is_empty())
            .map(|r| (format!("{UNIQUE_FLAVOUR}:{}:{}", r.name, r.id), "no text"))
            .collect();
        (keys, empty, bad)
    }

    /// Socket caps' domain checks for the validator: each row's base, and
    /// the rows with no level or counts out of order.
    pub(super) fn socket_domain(&self) -> (Vec<&str>, Vec<(String, &'static str)>) {
        let rows = self.socket_caps.as_deref().unwrap_or_default();
        let bases = rows.iter().map(|r| r.base.as_str()).collect();
        let bad = rows
            .iter()
            .filter(|r| {
                r.levels.is_empty()
                    || r.levels
                        .iter()
                        .zip(r.levels.iter().skip(1))
                        .any(|(a, b)| a.0 >= b.0)
            })
            .map(|r| (format!("{SOCKET_CAPS}:{}", r.base), "no counts in order"))
            .collect();
        (bases, bad)
    }

    /// The tree flavour's domain checks for the validator: the passives'
    /// node keys, the ascendancies' IDs, and the rows with no text.
    pub(super) fn tree_flavour_domain(
        &self,
    ) -> (Vec<String>, Vec<&str>, Vec<(String, &'static str)>) {
        let Some(f) = &self.tree_flavour else {
            return (Vec::new(), Vec::new(), Vec::new());
        };
        let nodes = f.passives.iter().map(|p| p.node.to_string()).collect();
        let ids = f.ascendancies.iter().map(|a| a.id.as_str()).collect();
        let bad = f
            .passives
            .iter()
            .filter(|p| p.text.trim().is_empty())
            .map(|p| format!("{TREE_FLAVOUR}:passive:{}", p.node))
            .chain(
                f.ascendancies
                    .iter()
                    .filter(|a| a.text.trim().is_empty())
                    .map(|a| format!("{TREE_FLAVOUR}:ascendancy:{}", a.id)),
            )
            .map(|at| (at, "no text"))
            .collect();
        (nodes, ids, bad)
    }

    /// The flavour text of passive `node`, if the file has one.
    #[must_use]
    pub fn passive_flavour(&self, node: i64) -> Option<&str> {
        self.tree_flavour
            .as_ref()?
            .passives
            .iter()
            .find(|p| p.node == node)
            .map(|p| p.text.as_str())
    }

    /// The quote of ascendancy `id`, if the file has one.
    #[must_use]
    pub fn ascendancy_flavour(&self, id: &str) -> Option<&AscendancyFlavour> {
        self.tree_flavour
            .as_ref()?
            .ascendancies
            .iter()
            .find(|a| a.id == id)
    }

    /// The socket caps of base `id`, if the file has them.
    #[must_use]
    pub fn socket_caps_of(&self, id: &str) -> Option<&SocketCaps> {
        self.socket_caps.iter().flatten().find(|r| r.base == id)
    }

    /// The flavour text rows of the unique named `name`, in the file's
    /// order (none without the file).
    pub fn flavour_of<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a UniqueFlavour> {
        self.unique_flavour
            .iter()
            .flatten()
            .filter(move |r| r.name == name)
    }

    /// Reads the section's files the manifest lists.
    ///
    /// # Errors
    /// [`BundleError`] when a listed file does not read or parse.
    pub(super) fn load(
        source: &dyn BundleSource,
        manifest: &Manifest,
    ) -> Result<Self, BundleError> {
        let listed = |path: &str| manifest.files.iter().any(|f| f.path == path);
        Ok(Self {
            quality_types: if listed(QUALITY_TYPES) {
                Some(json(source, QUALITY_TYPES)?)
            } else {
                None
            },
            grafts: if listed(GRAFTS) {
                Some(json(source, GRAFTS)?)
            } else {
                None
            },
            pantheons: if listed(PANTHEONS) {
                Some(json(source, PANTHEONS)?)
            } else {
                None
            },
            jewel_radii: if listed(JEWEL_RADII) {
                Some(json(source, JEWEL_RADII)?)
            } else {
                None
            },
            unique_flavour: if listed(UNIQUE_FLAVOUR) {
                Some(json(source, UNIQUE_FLAVOUR)?)
            } else {
                None
            },
            socket_caps: if listed(SOCKET_CAPS) {
                Some(json(source, SOCKET_CAPS)?)
            } else {
                None
            },
            tree_flavour: if listed(TREE_FLAVOUR) {
                Some(json(source, TREE_FLAVOUR)?)
            } else {
                None
            },
            cluster_jewels: if listed(CLUSTER_JEWELS) {
                Some(json(source, CLUSTER_JEWELS)?)
            } else {
                None
            },
            flasks: if listed(FLASKS) {
                Some(json(source, FLASKS)?)
            } else {
                None
            },
        })
    }
}
