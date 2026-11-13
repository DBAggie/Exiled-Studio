//! The v1 DTOs. Field and variant names are the document's; the domain's
//! may change without changing them.
//!
//! Conventions:
//! - every list is a JSON array in model order; no type is a map, so no
//!   iteration order can change the output;
//! - enums are externally tagged with kebab-case names: a unit variant is
//!   a string (`"flag"`), any other is a one-key object
//!   (`{"modifier": {...}}`);
//! - an absent optional value, and an empty `attributes`, `children`,
//!   `prefixes`, `text_spacing`, or `alternates` list, is left out;
//! - unknown fields are refused, so content from a newer document is
//!   never dropped silently (ADR-015).

use serde::{Deserialize, Serialize};

use super::poe1::Poe1Choices;

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
    /// The document: its format, its version, and the build.
    pub struct Document {
        /// Always `loom-build`.
        #[cfg_attr(feature = "schema", schemars(extend("const" = super::FORMAT)))]
        pub format: String,
        /// Always 1 in this version.
        #[cfg_attr(feature = "schema", schemars(extend("const" = super::VERSION)))]
        pub version: u32,
        /// The build.
        pub build: Build,
    }

    /// A game.
    #[serde(rename_all = "kebab-case")]
    pub enum Game {
        /// Path of Exile.
        Poe1,
        /// Path of Exile 2.
        Poe2,
    }

    /// Choices only one game has; the variant is the build's game.
    #[serde(rename_all = "kebab-case")]
    pub enum GameSpecific {
        /// Path of Exile choices.
        Poe1(Poe1Choices),
        /// Path of Exile 2 choices (none typed yet).
        Poe2(Poe2Choices),
    }

    /// Path of Exile 2 choices (none typed yet).
    pub struct Poe2Choices {}

    /// The data bundle a build was made with.
    pub struct DataVersion {
        /// The bundle's game.
        pub game: Game,
        /// The bundle's data schema version.
        pub schema_version: u32,
        /// The Steam build, if known.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub steam_build: Option<u64>,
    }

    /// A build.
    pub struct Build {
        /// The game.
        pub game: Game,
        /// The game-specific choices; of the build's game.
        pub game_specific: GameSpecific,
        /// The data bundle the build was made with, if recorded.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub data: Option<DataVersion>,
        /// The character.
        pub character: Character,
        /// Passive specs.
        pub passives: Passives,
        /// Skills.
        pub skills: Skills,
        /// Items and item sets.
        pub items: Items,
        /// Config.
        pub config: Config,
        /// The sections in order, typed ones as markers.
        pub layout: Vec<Entry<BuildPart>>,
        /// Untyped attributes of the build's container.
        #[serde(default, skip_serializing_if = "is_empty")]
        pub attributes: Vec<Attribute>,
        /// Untyped content before the build's container.
        #[serde(default, skip_serializing_if = "is_empty")]
        pub prolog: Vec<OpaqueNode>,
        /// Untyped content after the build's container.
        #[serde(default, skip_serializing_if = "is_empty")]
        pub epilog: Vec<OpaqueNode>,
    }

    /// Where a typed section sits among untyped ones.
    #[serde(rename_all = "kebab-case")]
    pub enum BuildPart {
        /// The character.
        Character,
        /// Passive specs.
        Passives,
        /// Skills.
        Skills,
        /// Items.
        Items,
        /// Config.
        Config,
    }

    /// The character.
    pub struct Character {
        /// The class, if set.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub class: Option<String>,
        /// The ascendancy, if set.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub ascendancy: Option<String>,
        /// The level, if set.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub level: Option<Sourced<u32>>,
        /// The main socket group (1-based), if set.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub main_socket_group: Option<Sourced<u32>>,
        /// Untyped attributes.
        #[serde(default, skip_serializing_if = "is_empty")]
        pub attributes: Vec<Attribute>,
        /// Untyped children.
        #[serde(default, skip_serializing_if = "is_empty")]
        pub children: Vec<OpaqueNode>,
    }

    /// One untyped attribute.
    pub struct Attribute {
        /// Name.
        pub name: String,
        /// Value.
        pub value: String,
    }

    /// A node of untyped content.
    #[serde(rename_all = "kebab-case")]
    pub enum OpaqueNode {
        /// An element.
        Element {
            /// Name.
            name: String,
            /// Attributes, in source order.
            #[serde(default, skip_serializing_if = "is_empty")]
            attributes: Vec<Attribute>,
            /// Children, in source order.
            #[serde(default, skip_serializing_if = "is_empty")]
            children: Vec<OpaqueNode>,
        },
        /// Text.
        Text(String),
        /// A comment.
        Comment(String),
        /// A processing instruction.
        ProcessingInstruction {
            /// Target.
            target: String,
            /// Data, if any.
            #[serde(default, skip_serializing_if = "Option::is_none")]
            data: Option<String>,
        },
    }

    /// One entry of a list with untyped content in place.
    #[serde(rename_all = "kebab-case")]
    #[cfg_attr(feature = "schema", schemars(rename = "{T}Entry"))]
    pub enum Entry<T> {
        /// A typed entry.
        Typed(T),
        /// Untyped content at this position.
        Opaque(OpaqueNode),
    }

    /// A typed value and the text it was read from, if any.
    #[cfg_attr(feature = "schema", schemars(rename = "Sourced_{T}"))]
    pub struct Sourced<T> {
        /// The value.
        pub value: T,
        /// The source text, while the value is unchanged.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub source: Option<String>,
    }

    /// A non-finite number, by its IEEE 754 bit pattern.
    pub struct NonFinite {
        /// The bits as 16 lowercase hexadecimal digits.
        #[cfg_attr(feature = "schema", schemars(regex(pattern = r"^[0-9a-f]{16}$")))]
        pub f64_bits: String,
    }

    /// Passive specs.
    pub struct Passives {
        /// The active spec, if set.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub active: Option<u32>,
        /// The specs.
        pub specs: Vec<Entry<PassiveSpec>>,
        /// Untyped attributes.
        #[serde(default, skip_serializing_if = "is_empty")]
        pub attributes: Vec<Attribute>,
    }

    /// A mastery node and its chosen effect.
    pub struct MasteryEffect {
        /// The node.
        pub node: u32,
        /// The effect.
        pub effect: u32,
    }

    /// A jewel socket and its item.
    pub struct JewelSocket {
        /// The socket node.
        pub node: u32,
        /// The item, if any.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub item: Option<u32>,
        /// Untyped attributes.
        #[serde(default, skip_serializing_if = "is_empty")]
        pub attributes: Vec<Attribute>,
    }

    /// Where a spec's typed child sits among untyped ones.
    #[serde(rename_all = "kebab-case")]
    pub enum SpecPart {
        /// The jewel socket list.
        JewelSockets,
    }

    /// One passive tree spec.
    pub struct PassiveSpec {
        /// The spec's ID.
        pub id: u32,
        /// The title, if any.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub title: Option<String>,
        /// The tree version, if any.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub tree_version: Option<String>,
        /// Allocated nodes, in order.
        pub nodes: Sourced<Vec<u32>>,
        /// Mastery effects, in order.
        pub mastery_effects: Sourced<Vec<MasteryEffect>>,
        /// Jewel sockets.
        pub jewel_sockets: Vec<Entry<JewelSocket>>,
        /// The spec's children.
        pub layout: Vec<Entry<SpecPart>>,
        /// Untyped attributes.
        #[serde(default, skip_serializing_if = "is_empty")]
        pub attributes: Vec<Attribute>,
    }

    /// Skill sets.
    pub struct Skills {
        /// The active set, if set.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub active: Option<u32>,
        /// The sets.
        pub sets: Vec<Entry<SkillSet>>,
        /// Untyped attributes.
        #[serde(default, skip_serializing_if = "is_empty")]
        pub attributes: Vec<Attribute>,
    }

    /// One skill set.
    pub struct SkillSet {
        /// The set's ID.
        pub id: u32,
        /// The title, if any.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub title: Option<String>,
        /// Socket groups.
        pub groups: Vec<Entry<SocketGroup>>,
        /// Untyped attributes.
        #[serde(default, skip_serializing_if = "is_empty")]
        pub attributes: Vec<Attribute>,
    }

    /// A group of linked gems.
    pub struct SocketGroup {
        /// The label, if any.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub label: Option<String>,
        /// Whether the group is enabled.
        pub enabled: bool,
        /// The slot, if any.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub slot: Option<String>,
        /// The main active skill (1-based), if set.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub main_active_skill: Option<Sourced<u32>>,
        /// Gems.
        pub gems: Vec<Entry<Gem>>,
        /// Untyped attributes.
        #[serde(default, skip_serializing_if = "is_empty")]
        pub attributes: Vec<Attribute>,
    }

    /// One gem.
    pub struct Gem {
        /// The gem item, if known.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub gem: Option<String>,
        /// The skill, if known.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub skill: Option<String>,
        /// The name as written, if any.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub name: Option<String>,
        /// Level.
        pub level: Sourced<u32>,
        /// Quality.
        pub quality: Sourced<i32>,
        /// Whether the gem is enabled.
        pub enabled: bool,
        /// Untyped attributes.
        #[serde(default, skip_serializing_if = "is_empty")]
        pub attributes: Vec<Attribute>,
    }

    /// Items and item sets.
    pub struct Items {
        /// The active item set, if set.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub active_set: Option<u32>,
        /// Items and sets in order.
        pub entries: Vec<Entry<ItemsEntry>>,
        /// Untyped attributes.
        #[serde(default, skip_serializing_if = "is_empty")]
        pub attributes: Vec<Attribute>,
    }

    /// An item or an item set.
    #[serde(rename_all = "kebab-case")]
    pub enum ItemsEntry {
        /// An item.
        Item(Item),
        /// An item set.
        Set(ItemSet),
    }

    /// An item set.
    pub struct ItemSet {
        /// The set's ID.
        pub id: u32,
        /// The title, if any.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub title: Option<String>,
        /// Slots.
        pub slots: Vec<Entry<SlotEntry>>,
        /// Untyped attributes.
        #[serde(default, skip_serializing_if = "is_empty")]
        pub attributes: Vec<Attribute>,
    }

    /// One slot of an item set.
    pub struct SlotEntry {
        /// The slot.
        pub slot: String,
        /// The item, if any.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub item: Option<u32>,
        /// Untyped attributes.
        #[serde(default, skip_serializing_if = "is_empty")]
        pub attributes: Vec<Attribute>,
    }

    /// Where an item's body sits among untyped children.
    #[serde(rename_all = "kebab-case")]
    pub enum ItemPart {
        /// The text the item was read from.
        Body,
    }

    /// One item.
    pub struct Item {
        /// The item's ID.
        pub id: u32,
        /// The item as read from its text.
        pub body: ItemBody,
        /// The item's children.
        pub layout: Vec<Entry<ItemPart>>,
        /// Legacy-XML whitespace of the body's text (`lines + 1` entries),
        /// or empty.
        #[serde(default, skip_serializing_if = "is_empty")]
        pub text_spacing: Vec<String>,
        /// Untyped attributes.
        #[serde(default, skip_serializing_if = "is_empty")]
        pub attributes: Vec<Attribute>,
    }

    /// Item rarity.
    #[serde(rename_all = "kebab-case")]
    pub enum Rarity {
        /// Normal.
        Normal,
        /// Magic.
        Magic,
        /// Rare.
        Rare,
        /// Unique.
        Unique,
        /// Relic.
        Relic,
    }

    /// An alternate variant.
    pub struct AltVariant {
        /// Which alternate (0 for the first).
        pub index: u8,
        /// The selected variant, if written.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub selected: Option<u32>,
    }

    /// Variant selection.
    pub struct VariantSelection {
        /// Number of variants.
        pub count: u32,
        /// The selected variant, if written.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub selected: Option<u32>,
        /// Alternate variants.
        #[serde(default, skip_serializing_if = "is_empty")]
        pub alternates: Vec<AltVariant>,
    }

    /// An item as read from its text.
    pub struct ItemBody {
        /// The item class, if written.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub item_class: Option<String>,
        /// Rarity, if read.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub rarity: Option<Rarity>,
        /// The name, if any.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub name: Option<String>,
        /// The base name, if written.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub base: Option<String>,
        /// The implicit count, if written.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub implicits: Option<u64>,
        /// Variant selection.
        pub variants: VariantSelection,
        /// Every source line, in order.
        pub lines: Vec<ItemLine>,
    }

    /// One source line of an item.
    pub struct ItemLine {
        /// The line as written.
        pub source: String,
        /// What it is.
        pub kind: LineKind,
    }

    /// A header line.
    #[serde(rename_all = "kebab-case")]
    pub enum HeaderLine {
        /// The item class.
        ItemClass,
        /// The rarity.
        Rarity,
        /// A separator.
        Separator,
        /// The name.
        Name,
        /// The base.
        Base,
    }

    /// What a line is.
    #[serde(rename_all = "kebab-case")]
    pub enum LineKind {
        /// A header line.
        Header(HeaderLine),
        /// A spec line.
        Spec {
            /// The key.
            key: String,
            /// The value, as written.
            value: String,
            /// Whether the import adapter knows the key.
            known: bool,
        },
        /// A base line of one variant.
        VariantBase {
            /// The base text.
            base: String,
            /// Whether the variant selection includes it.
            selected: bool,
        },
        /// An influence line.
        Influence(String),
        /// An item flag line.
        Flag,
        /// A modifier line.
        Modifier(ModifierLine),
        /// A line not recognized.
        Unrecognized,
    }

    /// A prefix or flag word on a modifier line.
    pub struct Prefix {
        /// The key.
        pub key: String,
        /// The value, if any.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub value: Option<String>,
    }

    /// Which modifier list a line belongs to.
    #[serde(rename_all = "kebab-case")]
    pub enum ModSection {
        /// Enchantments.
        Enchant,
        /// Implicit modifiers.
        Implicit,
        /// Explicit modifiers.
        Explicit,
        /// Another section, by name.
        Other(String),
    }

    /// A modifier line.
    pub struct ModifierLine {
        /// The text without prefixes.
        pub text: String,
        /// Prefixes, in line order.
        #[serde(default, skip_serializing_if = "is_empty")]
        pub prefixes: Vec<Prefix>,
        /// The section.
        pub section: ModSection,
        /// Whether the variant selection includes the line.
        pub selected: bool,
        /// What the line resolved to.
        pub resolution: LineResolution,
    }

    /// The state of a modifier line.
    #[serde(rename_all = "kebab-case")]
    pub enum LineResolution {
        /// Not resolved yet.
        Pending,
        /// Not resolved by design.
        Excluded(Exclusion),
        /// Resolved.
        Resolved {
            /// The result.
            result: Resolved,
            /// The tolerance it needed, if any.
            #[serde(default, skip_serializing_if = "Option::is_none")]
            adapted: Option<Adapted>,
        },
        /// Resolved with an earlier line, which carries the result.
        Continues {
            /// Index of that line among the item's lines.
            line: u64,
        },
        /// Not resolved; quarantined.
        Unresolved(UnresolvedReason),
    }

    /// Why a modifier line is not resolved by design.
    #[serde(rename_all = "kebab-case")]
    pub enum Exclusion {
        /// Not in the variant selection.
        NotSelected,
        /// Marked custom.
        Custom,
        /// Its section is not resolved.
        Section,
    }

    /// An import tolerance a line needed.
    #[serde(rename_all = "kebab-case")]
    pub enum Adapted {
        /// Letter case only.
        CaseFolded,
        /// Resolved with the next line of another section.
        SplitSection,
    }

    /// A resolution result.
    #[serde(rename_all = "kebab-case")]
    pub enum Resolved {
        /// Stat IDs with values.
        Stats(Vec<StatLine>),
        /// A typed result.
        Typed(TypedResolution),
    }

    /// A stat ID and its value.
    pub struct StatLine {
        /// The stat.
        pub stat: String,
        /// Its value.
        pub value: StatValue,
    }

    /// A stat's value as read from a line.
    #[serde(rename_all = "kebab-case")]
    pub enum StatValue {
        /// Shown as one number.
        Exact {
            /// Lowest raw value.
            min: i64,
            /// Highest raw value.
            max: i64,
        },
        /// Shown as a range.
        Ranged {
            /// Lowest raw value.
            min: i64,
            /// Highest raw value.
            max: i64,
        },
        /// Not shown.
        Hidden {
            /// The assumed value, if clear.
            #[serde(default, skip_serializing_if = "Option::is_none")]
            assumed: Option<i64>,
        },
        /// Shown as a name: every key with it, sorted.
        Lookup(Vec<i64>),
    }

    /// A typed result read from game data.
    #[serde(rename_all = "kebab-case")]
    pub enum TypedResolution {
        /// A skill the item grants.
        GrantedSkill {
            /// The data row that grants it.
            source: String,
            /// The gem.
            gem: String,
            /// The level.
            level: u32,
        },
        /// A display property.
        ItemProperty {
            /// The property's data ID.
            property: String,
            /// Values by placeholder, in text order.
            values: Vec<PropertyValue>,
        },
        /// A quality type and its stats.
        CatalystQuality {
            /// The quality type's data ID.
            quality_type: String,
            /// Its stats.
            stats: Vec<StatLine>,
            /// The quality as shown.
            quality: String,
        },
        /// A custom modifier line that is a label or a comment, with no
        /// effect (ADR-015 Amendment 2; additive in v1, P3-close).
        CustomNote,
        /// A custom modifier line read by a curated pattern as PoB's
        /// parser reads it (ADR-015 Amendment 2; additive in v1).
        CustomPattern {
            /// The pattern's name.
            pattern: String,
            /// The value as shown.
            value: String,
        },
    }

    /// One placeholder value of a property line.
    pub struct PropertyValue {
        /// The placeholder's number.
        pub slot: u8,
        /// The value's text.
        pub text: String,
    }

    /// Why a line did not resolve.
    #[serde(rename_all = "kebab-case")]
    pub enum UnresolvedReason {
        /// No description has the text.
        NoTemplate,
        /// None fits the values.
        NoVariantFits,
        /// Only shadowed descriptions fit.
        OnlyShadowed,
        /// More than one fits: the candidates' stats.
        Ambiguous(Vec<Vec<String>>),
        /// The item has no base in the game data.
        NoBase(NoBase),
    }

    /// Why an item has no base.
    #[serde(rename_all = "kebab-case")]
    pub enum NoBase {
        /// The import adapter named none; its reason.
        Unnamed(String),
        /// Not in the game data.
        NotFound,
        /// More than one stat-description scope.
        ScopeConflict,
    }

    /// Config sets.
    pub struct Config {
        /// The active set, if set.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub active: Option<u32>,
        /// The sets.
        pub sets: Vec<Entry<ConfigSet>>,
        /// How the build's format held the sets; left out for one element
        /// per set.
        #[serde(default, skip_serializing_if = "ConfigLayout::is_sets")]
        pub layout: ConfigLayout,
        /// Untyped attributes.
        #[serde(default, skip_serializing_if = "is_empty")]
        pub attributes: Vec<Attribute>,
    }

    /// How a build's format held its config sets.
    #[derive(Default, Copy, PartialEq, Eq)]
    #[serde(rename_all = "kebab-case")]
    pub enum ConfigLayout {
        /// Each set in its own element.
        #[default]
        Sets,
        /// One set (ID 1, no title, no attributes) held directly, with no
        /// set element.
        Flat,
    }

    /// One config set.
    pub struct ConfigSet {
        /// The set's ID.
        pub id: u32,
        /// The title, if any.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub title: Option<String>,
        /// Inputs.
        pub inputs: Vec<Entry<ConfigInput>>,
        /// Custom modifier blocks, after the inputs; left out when none.
        #[serde(default, skip_serializing_if = "is_empty")]
        pub custom_modifiers: Vec<CustomModifierBlock>,
        /// Untyped attributes.
        #[serde(default, skip_serializing_if = "is_empty")]
        pub attributes: Vec<Attribute>,
    }

    /// A block of custom modifier lines.
    pub struct CustomModifierBlock {
        /// The title, if any.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub title: Option<String>,
        /// Whether the block applies.
        pub enabled: bool,
        /// The block's text, as read.
        pub text: String,
        /// The modifier lines read from the text, with their resolutions;
        /// left out when none.
        #[serde(default, skip_serializing_if = "is_empty")]
        pub lines: Vec<CustomModifierLine>,
        /// Untyped attributes.
        #[serde(default, skip_serializing_if = "is_empty")]
        pub attributes: Vec<Attribute>,
    }

    /// One modifier line of a custom modifier block or a config input.
    pub struct CustomModifierLine {
        /// The modifier text.
        pub text: String,
        /// What the line resolved to.
        pub resolution: LineResolution,
    }

    /// One config input.
    pub struct ConfigInput {
        /// The key.
        pub name: String,
        /// The value.
        pub value: ConfigValue,
        /// Whether the value was set or is a placeholder; left out for a
        /// set value.
        #[serde(default, skip_serializing_if = "ConfigInputKind::is_input")]
        pub kind: ConfigInputKind,
        /// The custom modifier lines read from a text value applied as
        /// such (PoB's legacy `customMods`), with their resolutions; left
        /// out when none.
        #[serde(default, skip_serializing_if = "is_empty")]
        pub lines: Vec<CustomModifierLine>,
        /// Untyped attributes.
        #[serde(default, skip_serializing_if = "is_empty")]
        pub attributes: Vec<Attribute>,
    }

    /// Whether a config value was set or is a placeholder.
    #[derive(Default, Copy, PartialEq, Eq)]
    #[serde(rename_all = "kebab-case")]
    pub enum ConfigInputKind {
        /// A value set for the input.
        #[default]
        Input,
        /// A placeholder: the value a planner shows for an input left
        /// unset, which it may also use for that input.
        Placeholder,
    }

    /// A config value.
    #[serde(rename_all = "kebab-case")]
    pub enum ConfigValue {
        /// A flag.
        Boolean(bool),
        /// A number.
        Number(Sourced<Number>),
        /// Text.
        String(String),
    }
}

impl ConfigInputKind {
    /// Whether this is a set value (for `skip_serializing_if`).
    #[expect(
        clippy::trivially_copy_pass_by_ref,
        reason = "serde's skip_serializing_if passes a reference"
    )]
    fn is_input(&self) -> bool {
        *self == Self::Input
    }
}

impl ConfigLayout {
    /// Whether this is the layout with one element per set (for
    /// `skip_serializing_if`).
    #[expect(
        clippy::trivially_copy_pass_by_ref,
        reason = "serde's skip_serializing_if passes a reference"
    )]
    fn is_sets(&self) -> bool {
        *self == Self::Sets
    }
}

/// A number: a JSON number when finite, otherwise its bit pattern (JSON
/// has no NaN or infinity). Finite values are written in the shortest
/// form that reads back to the same bits.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(untagged)]
pub enum Number {
    /// A finite number.
    Finite(f64),
    /// A non-finite number.
    NonFinite(NonFinite),
}
