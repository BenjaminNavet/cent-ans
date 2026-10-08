//! Salts of the AI's pure rolls: every roll hashes the campaign seed with a
//! salt, so two decisions never share a roll. Values are part of the saved
//! games' reproducibility: never renumber one.

/// An army lies in ambush (`stances`).
pub(crate) const AMBUSH: u64 = 0xA3B0;
/// An army detours to an encounter site (`stances`).
pub(crate) const DETOUR: u64 = 0xD370;
/// Per-turn feudal rolls: revolt, homage (`feudal`).
pub(crate) const FEUDAL: u64 = 0xFE05;
/// The wool revolt roll (`alignment`).
pub(crate) const WOOL: u64 = 1;
/// The defection rolls, one per decade from this one (`alignment`).
pub(crate) const DEFECTION: u64 = 2;
/// The dynastic alliance and money fief roll (`alignment`).
pub(crate) const DYNASTIC: u64 = 7;
