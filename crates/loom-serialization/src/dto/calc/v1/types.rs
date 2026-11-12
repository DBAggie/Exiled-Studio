//! The calc result v1 DTOs. Field and variant names are the document's;
//! the engine's may change without changing them.
//!
//! Conventions (as build document v1):
//! - every list is a JSON array in the engine's order (fields by
//!   [`loom_calc::Field`]'s order, reasons by their sets' order, nodes by
//!   index); no type is a map;
//! - enums are externally tagged with kebab-case names: a unit variant is
//!   a string (`"not-computed"`), any other is a one-key object
//!   (`{"unmapped-stat": {...}}`);
//! - an absent optional value, and an empty reason, `children`, or `term`,
//!   is left out;
//! - unknown fields are refused.
//!
//! Names of fields, quantities, terms, operations, and the engine's
//! labels are strings: `Field` and `Quantity` as their `Display`
//! (`resistance.fire`, `maximum-resistance.fire`), the rest by their
//! stable `name()`.

use serde::{Deserialize, Serialize};

use crate::dto::v1::{Adapted, Game, Number};

/// Declares DTO types with the shared derives and serde settings.
macro_rules! dto {
    ($($item:item)*) => {
        $(
            #[derive(Debug, Clone, Serialize, Deserialize)]
            #[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
            #[serde(deny_unknown_fields)]
            $item
        )*
    };
}

/// Whether a list is empty (for `skip_serializing_if`).
fn is_empty<T>(v: &[T]) -> bool {
    v.is_empty()
}

dto! {
    /// The document: its format, its version, and the result.
    pub struct Document {
        /// Always `loom-calc-result`.
        #[cfg_attr(feature = "schema", schemars(extend("const" = super::FORMAT)))]
        pub format: String,
        /// Always 1 in this version.
        #[cfg_attr(feature = "schema", schemars(extend("const" = super::VERSION)))]
        pub version: u32,
        /// The result.
        pub result: CalcResult,
    }

    /// A calculation's result.
    pub struct CalcResult {
        /// The build's game.
        pub game: Game,
        /// The worst state of the fields; `unsupported` when the game has
        /// no calculation.
        pub state: StateKind,
        /// The reasons that apply to every computed field.
        pub reasons: Reasons,
        /// Every field of the game's slice; empty when the game has no
        /// calculation.
        pub fields: Vec<FieldResult>,
        /// The provenance graph; a node's position is its index.
        pub graph: Vec<Node>,
    }

    /// One field's result.
    pub struct FieldResult {
        /// The field's name (`life`, `resistance.fire`).
        pub field: String,
        /// The value; absent when the field is not computed.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub value: Option<Number>,
        /// The value's node in `graph`; absent when not computed. Its
        /// value is `value`.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub node: Option<u32>,
        /// The state.
        pub state: StateKind,
        /// Why the field is not computed: present exactly when `state` is
        /// `unsupported`.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub reason: Option<UnsupportedReason>,
        /// What was not modelled: not empty when `state` is `partial`,
        /// possibly empty when `indeterminate`, empty otherwise.
        #[serde(default, skip_serializing_if = "is_empty")]
        pub unsupported: Vec<UnsupportedReason>,
        /// Why the value cannot be settled: not empty exactly when `state`
        /// is `indeterminate`.
        #[serde(default, skip_serializing_if = "is_empty")]
        pub indeterminate: Vec<IndeterminateReason>,
    }

    /// Reasons collected for the whole build.
    pub struct Reasons {
        /// Content that was not modelled.
        #[serde(default, skip_serializing_if = "is_empty")]
        pub unsupported: Vec<UnsupportedReason>,
        /// Values that could not be settled.
        #[serde(default, skip_serializing_if = "is_empty")]
        pub indeterminate: Vec<IndeterminateReason>,
    }

    /// A state's kind, from best to worst.
    #[serde(rename_all = "kebab-case")]
    pub enum StateKind {
        /// Every contribution modelled.
        Complete,
        /// Computed; some content not modelled.
        Partial,
        /// Not computed.
        Unsupported,
        /// Not settled.
        Indeterminate,
    }

    /// Why a field is not complete, or not computed.
    #[serde(rename_all = "kebab-case")]
    pub enum UnsupportedReason {
        /// A stat the stat-to-effect mapping does not know.
        UnmappedStat {
            /// The stat ID.
            stat: String,
            /// Where it comes from.
            source: Source,
        },
        /// An item line that did not resolve.
        UnresolvedLine {
            /// The line.
            source: Source,
            /// The resolver's reason.
            reason: String,
        },
        /// An item line excluded by design whose effect the game may
        /// still apply.
        ExcludedLine {
            /// The line.
            source: Source,
            /// The exclusion.
            exclusion: String,
        },
        /// An item line that was never resolved.
        PendingLine {
            /// The line.
            source: Source,
        },
        /// A typed resolution without a handler.
        UnhandledKind {
            /// The typed resolution's `kind`.
            kind: String,
            /// Where it comes from.
            source: Source,
        },
        /// An effect needs a named condition the build and config do not
        /// settle.
        UnmodelledCondition {
            /// The condition's name.
            condition: String,
            /// The quantity of the effect left out.
            quantity: String,
            /// The source of the effect left out.
            source: Source,
        },
        /// A field derived from another field that is not computed.
        DependsOnNotComputed {
            /// The field it needs.
            field: String,
        },
        /// The engine does not compute this field yet.
        NotComputed,
        /// An enabled gem whose effects the engine does not model yet.
        UnmodelledGem {
            /// The gem.
            source: Source,
        },
        /// A config input the engine does not model yet.
        UnmodelledConfig {
            /// The input's name.
            input: String,
        },
        /// A game-specific build choice the engine does not model.
        UnmodelledChoice {
            /// Which choice.
            choice: String,
            /// Its value, as the build writes it.
            value: String,
        },
        /// An item in use whose effect goes beyond its lines, not
        /// modelled yet.
        UnmodelledItem {
            /// The item's ID.
            item: u32,
            /// Its slot, as the build names it.
            slot: String,
            /// What is not modelled.
            what: String,
        },
        /// An allocated passive whose stats the data does not hold, by a
        /// rule of the game (a node generated from a jewel), not modelled
        /// yet.
        UnmodelledPassive {
            /// The passive.
            source: Source,
            /// What is not modelled.
            what: String,
        },
        /// An item line the import did not read.
        UnreadLine {
            /// The line.
            source: Source,
        },
        /// An item line whose values the import format scales in a way the
        /// engine does not model; it gives no effect.
        ScaledLine {
            /// The line.
            source: Source,
            /// The scaling (`catalyst`, `corrupted-range`,
            /// `modifier-magnitude`).
            scaling: String,
        },
        /// Passives are allocated, but the data's tree version was not
        /// given, so the build's was not checked.
        UncheckedTreeVersion,
        /// Part of the main skill the engine does not model yet.
        UnmodelledSkill {
            /// The gem (or the rule) it comes from.
            source: Source,
            /// What is not modelled.
            what: String,
        },
    }

    /// Why a value cannot be settled.
    #[serde(rename_all = "kebab-case")]
    pub enum IndeterminateReason {
        /// Two overrides of one quantity give different values.
        ConflictingOverride {
            /// The quantity.
            quantity: String,
        },
        /// A stat's value read from its line is not one number.
        UnsettledValue {
            /// The stat ID.
            stat: String,
            /// Where it comes from.
            source: Source,
        },
        /// The build names something the game data does not have.
        MissingData {
            /// The table.
            table: String,
            /// The key the build gives.
            key: String,
        },
        /// The build's passive tree is another version than the data's.
        TreeVersion {
            /// The build's.
            build: String,
            /// The data's.
            data: String,
        },
    }

    /// Where a contribution comes from.
    #[serde(rename_all = "kebab-case")]
    pub enum Source {
        /// The character itself.
        Character,
        /// A line of an item.
        Item {
            /// The item's ID.
            item: u32,
            /// The line's index among the item's lines.
            line: u64,
            /// The import tolerance the line needed, if any.
            #[serde(default, skip_serializing_if = "Option::is_none")]
            adapted: Option<Adapted>,
        },
        /// An item's base (the properties the game data gives it).
        ItemBase {
            /// The item's ID.
            item: u32,
        },
        /// An allocated passive node.
        Passive {
            /// The node's ID.
            node: u32,
        },
        /// A gem.
        Gem {
            /// The socket group's index.
            group: u64,
            /// The gem's index in the group.
            gem: u64,
        },
        /// A config input.
        Config {
            /// The input's name.
            input: String,
        },
        /// The enemy.
        Enemy,
        /// A rule of the engine, by name.
        Rule(String),
        /// A line of a custom modifier block of the active config set.
        CustomModifier {
            /// The block's index in the set.
            block: u64,
            /// The line's index among the block's lines.
            line: u64,
        },
        /// A custom modifier line of a config input of the active config
        /// set (PoB's legacy `customMods`).
        ConfigInputLine {
            /// The input's name.
            input: String,
            /// The line's index among the input's lines.
            line: u64,
        },
    }

    /// A provenance graph node. Its children are earlier nodes; its value
    /// is what the engine computed, equal to the graph's evaluation from
    /// the leaves.
    pub struct Node {
        /// What the node is.
        pub kind: NodeKind,
        /// The breakdown term (`base`, `increased`, `more`, `override`,
        /// `result`, or `step.<name>`); absent for leaves and overrides.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub term: Option<String>,
        /// The children's indices, in order: a sum's terms, a product's
        /// factors, an override's `by` then `replaced`, a minimum's or
        /// maximum's two inputs.
        #[serde(default, skip_serializing_if = "is_empty")]
        pub children: Vec<u32>,
        /// The value.
        pub value: Number,
    }

    /// What a node is, and how its children combine.
    #[serde(rename_all = "kebab-case")]
    pub enum NodeKind {
        /// One effect from one source (a leaf; its value is the effect's).
        Contribution {
            /// Where it comes from.
            source: Source,
            /// What it changes.
            quantity: String,
            /// How it combines (`base`, `increased`, `more`, `flag`,
            /// `override`).
            op: String,
        },
        /// A constant of a rule or of the game data (a leaf).
        Constant {
            /// What it is.
            label: String,
        },
        /// The sum of the children, from 0.
        Sum,
        /// The product of the children, from 1.
        Product,
        /// `1 + child / 100`.
        PercentFactor,
        /// 1 if a child is not 0, else 0.
        Any,
        /// The first child's value, replacing the second's.
        Override,
        /// The child rounded down.
        Floor,
        /// The child rounded half up.
        RoundHalfUp,
        /// The child's integer part, towards zero.
        Trunc,
        /// The smaller child.
        Min,
        /// The larger child.
        Max,
        /// The first child less the second.
        Difference,
        /// The first child divided by the second.
        Quotient,
        /// The first child to the power of the second.
        Power,
    }
}
