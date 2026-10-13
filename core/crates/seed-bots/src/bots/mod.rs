//! The eight bots of the spec.

use crate::bot::Bot;

pub mod mirror;
pub mod random_legal;

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

/// Builds the bot of that name (`None` for the bots not written yet).
pub fn make(name: &str) -> Option<Box<dyn Bot>> {
    match name {
        "random-legal" => Some(Box::new(random_legal::RandomLegal)),
        "mirror" => Some(Box::new(mirror::Mirror)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn makes_the_written_bots_only() {
        assert_eq!(make("random-legal").map(|b| b.name()), Some("random-legal"));
        assert_eq!(make("mirror").map(|b| b.name()), Some("mirror"));
        assert!(make("chaos").is_none());
    }
}
