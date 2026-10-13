//! The eight bots of the spec.

use crate::bot::Bot;

/// Names of every bot, in report order.
pub const ALL: &[&str] = &[
    "random-legal",
    "aggressor",
    "builder",
    "diplomat",
    "turtle",
    "sailor",
    "chaos",
    "mirror",
];

/// Builds the bot of that name.
pub fn make(_name: &str) -> Option<Box<dyn Bot>> {
    todo!("lots B and E")
}
