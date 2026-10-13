//! Seeded test bots (spec `docs/superpowers/specs/2026-10-13-seed-bots-design.md`).
//!
//! Eight bots play whole campaigns headless on `core/`, each with its own
//! personality, while oracles watch for panics and hangs, broken rule
//! invariants, legal orders the rules refuse, and balance drift. Every
//! finding carries its seed and order log, so `seed-bots replay` plays the
//! game again exactly up to the faulty turn.

pub mod bot;
pub mod bots;
pub mod candidates;
pub mod oracles;
pub mod report;
pub mod runner;
pub mod view;
