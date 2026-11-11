//! The Build aggregate (ADR-003): the transaction and persistence
//! boundary. It owns the character, passive specs, skills, items, and
//! config, and keeps untyped content in place.

use crate::config::Config;
use crate::error::DomainError;
use crate::game::{DataVersion, Game, GameSpecific};
use crate::id::{AscendancyId, ClassId};
use crate::item::{Items, LineKind, LineResolution};
use crate::opaque::{Attributes, OpaqueNode, Ordered};
use crate::passive::Passives;
use crate::quarantine::{Completeness, QuarantineEntry, QuarantineReason};
use crate::skill::Skills;
use crate::value::Sourced;

/// The character a build is for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Character {
    /// The class, if set.
    pub class: Option<ClassId>,
    /// The ascendancy, if set.
    pub ascendancy: Option<AscendancyId>,
    /// The level.
    pub level: Sourced<u32>,
    /// The main socket group (1-based, as builds write it), if set.
    pub main_socket_group: Option<Sourced<u32>>,
    /// Attributes not typed.
    pub attributes: Attributes,
    /// Untyped children, in order.
    pub children: Vec<OpaqueNode>,
}

impl Default for Character {
    fn default() -> Self {
        Self {
            class: None,
            ascendancy: None,
            level: Sourced::new(1),
            main_socket_group: None,
            attributes: Attributes::new(),
            children: Vec::new(),
        }
    }
}

/// Where a typed section sits among a build's untyped sections.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuildPart {
    /// [`Build::character`].
    Character,
    /// [`Build::passives`].
    Passives,
    /// [`Build::skills`].
    Skills,
    /// [`Build::items`].
    Items,
    /// [`Build::config`].
    Config,
}

/// A build.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Build {
    game: Game,
    game_specific: GameSpecific,
    /// The data bundle the build was made with, if recorded.
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
    /// The sections in order: typed ones as [`BuildPart`] markers,
    /// untyped ones in place.
    pub layout: Ordered<BuildPart>,
}

impl Build {
    /// An empty build for `game`, with its empty game-specific choices and
    /// the typed sections in their default order.
    #[must_use]
    pub fn new(game: Game) -> Self {
        let mut layout = Ordered::new();
        for part in [
            BuildPart::Character,
            BuildPart::Passives,
            BuildPart::Skills,
            BuildPart::Items,
            BuildPart::Config,
        ] {
            layout.push(part);
        }
        Self {
            game,
            game_specific: GameSpecific::empty(game),
            data: None,
            character: Character::default(),
            passives: Passives::default(),
            skills: Skills::default(),
            items: Items::default(),
            config: Config::default(),
            layout,
        }
    }

    /// The game.
    #[must_use]
    pub const fn game(&self) -> Game {
        self.game
    }

    /// The game-specific choices; always of [`Build::game`].
    #[must_use]
    pub const fn game_specific(&self) -> &GameSpecific {
        &self.game_specific
    }

    /// Replaces the game-specific choices.
    ///
    /// # Errors
    /// [`DomainError::GameMismatch`] when the choices are another game's.
    pub fn set_game_specific(&mut self, choices: GameSpecific) -> Result<(), DomainError> {
        if choices.game() != self.game {
            return Err(DomainError::GameMismatch {
                build: self.game,
                choices: choices.game(),
            });
        }
        self.game_specific = choices;
        Ok(())
    }

    /// Every quarantined item line, by item then line.
    #[must_use]
    pub fn quarantine(&self) -> Vec<QuarantineEntry> {
        let mut out = Vec::new();
        for item in self.items.items() {
            for (line, l) in item.body.lines.iter().enumerate() {
                let reason = match &l.kind {
                    LineKind::Unrecognized => QuarantineReason::UnrecognizedLine,
                    LineKind::Spec { known: false, .. } => QuarantineReason::UnknownSpec,
                    LineKind::Modifier(m) => match &m.resolution {
                        LineResolution::Pending => QuarantineReason::Pending,
                        LineResolution::Unresolved(r) => QuarantineReason::Unresolved(r.clone()),
                        LineResolution::Excluded(_)
                        | LineResolution::Resolved { .. }
                        | LineResolution::Continues { .. } => continue,
                    },
                    _ => continue,
                };
                out.push(QuarantineEntry {
                    item: item.id,
                    line,
                    reason,
                });
            }
        }
        out
    }

    /// [`Completeness::Partial`] when anything is quarantined.
    #[must_use]
    pub fn completeness(&self) -> Completeness {
        if self.quarantine().is_empty() {
            Completeness::Complete
        } else {
            Completeness::Partial
        }
    }
}
