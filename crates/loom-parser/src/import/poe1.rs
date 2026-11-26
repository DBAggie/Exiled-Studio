//! PoE1-only import content (plan gate 3c: PoE1-only names live in a
//! `poe1` module): PoB's custom modifier lines read as its PoE1 parser
//! reads them ([`custom`], ADR-015 Amendment 2); older item wordings read
//! through the reviewed rewording table ([`reword`], ADR-015 Amendment 3).

pub mod custom;
pub mod reword;
