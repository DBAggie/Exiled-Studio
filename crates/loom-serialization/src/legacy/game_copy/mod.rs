//! The game's own item copy (the text Ctrl+C or Ctrl+Alt+C puts on the
//! clipboard), turned into PoB's item text so the PoB item reader
//! ([`crate::legacy::pob_item`]) reads it. This is what PoB itself does with a
//! pasted item: `ItemClass:ParseRaw` reads the copy and `BuildRaw` writes the
//! text it keeps (`v2.67.2:src/Classes/Item.lua`). Each rule names the
//! `ParseRaw` code it follows; where the converter differs, the module docs
//! of [`poe1`] say so.
//!
//! Every non-blank line of the copy ends in one [`Outcome`]: kept as written,
//! rewritten, or dropped by a named [`Dropped`] rule (ADR-015: nothing is
//! dropped without a name). Lines the converter does not know are kept, so
//! the PoB reader reports them as unrecognized.

pub mod poe1;
