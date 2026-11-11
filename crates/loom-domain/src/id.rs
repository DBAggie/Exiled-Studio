//! Identifiers. Game-data identifiers (classes, gems, stats, ...) are
//! strings or numbers from the game data, never Rust enums (ADR-001), so a
//! new league or the other game needs no code change. Build-local
//! identifiers (items, sets) are numbers the build assigns.

use core::fmt;

/// Declares a string identifier newtype.
macro_rules! string_id {
    ($(#[$doc:meta])* $name:ident) => {
        $(#[$doc])*
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(String);

        impl $name {
            /// Wraps the identifier as the data writes it.
            #[must_use]
            pub fn new(id: impl Into<String>) -> Self {
                Self(id.into())
            }

            /// The identifier.
            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }
    };
}

/// Declares a numeric identifier newtype.
macro_rules! number_id {
    ($(#[$doc:meta])* $name:ident) => {
        $(#[$doc])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(pub u32);

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}", self.0)
            }
        }
    };
}

string_id!(
    /// A stat ID from the game's `Stats` table (ADR-015).
    StatId
);
string_id!(
    /// A character class, as the game data names it.
    ClassId
);
string_id!(
    /// An ascendancy class, as the game data names it.
    AscendancyId
);
string_id!(
    /// A gem, by its base item's metadata ID.
    GemId
);
string_id!(
    /// A skill, by its game-data ID (a granted effect or active skill).
    SkillId
);
string_id!(
    /// A passive tree version.
    TreeVersion
);
string_id!(
    /// An equipment slot name (`Weapon 1`, `Body Armour`, ...).
    SlotId
);
string_id!(
    /// An item influence, by its data name (`Shaper`, ...).
    InfluenceId
);
number_id!(
    /// A passive tree node, by its graph ID.
    NodeId
);
number_id!(
    /// An item of a build; unique within the build.
    ItemId
);
number_id!(
    /// An item set of a build.
    ItemSetId
);
number_id!(
    /// A skill set of a build.
    SkillSetId
);
number_id!(
    /// A config set of a build.
    ConfigSetId
);
number_id!(
    /// A passive tree spec of a build.
    SpecId
);
