//! Oracle 4: balance drift. Compares each bot's mean metric to the active bands.

use crate::bands::Bands;
use crate::metrics::BotMetrics;

use super::{Finding, OracleKind, Origin, Severity};

/// French number: two decimals at most, comma as the separator.
pub fn fr(value: f64) -> String {
    let text = format!("{value:.2}");
    text.trim_end_matches('0')
        .trim_end_matches('.')
        .replace('.', ",")
}

fn band_text(min: Option<f64>, max: Option<f64>) -> String {
    match (min, max) {
        (Some(min), Some(max)) => format!("[{} ; {}]", fr(min), fr(max)),
        (Some(min), None) => format!("[{} ; ∞[", fr(min)),
        (None, Some(max)) => format!("]-∞ ; {}]", fr(max)),
        (None, None) => "]-∞ ; ∞[".to_owned(),
    }
}

/// One warning per (bot, metric) whose mean is outside an active band.
/// A band still `to_initialise` never raises anything.
pub fn check(bots: &[BotMetrics], bands: &Bands) -> Vec<Finding> {
    let mut findings = Vec::new();
    for bot in bots {
        for (name, aggregate) in &bot.metrics {
            let Some(band) = bands.get(name) else {
                continue;
            };
            if !band.is_violated_by(aggregate.mean) {
                continue;
            }
            let origin = Origin {
                seed: 0,
                bot: bot.bot.clone(),
                faction: "-".to_owned(),
                turn: 0,
                order_log: Vec::new(),
            };
            findings.push(Finding::new(
                OracleKind::BalanceDrift,
                Severity::Warning,
                &origin,
                format!(
                    "Dérive d'équilibre : « {name} » vaut {} en moyenne pour le bot {} (min {}, max {}, {} parties), hors de la bande {}.",
                    fr(aggregate.mean),
                    bot.bot,
                    fr(aggregate.min),
                    fr(aggregate.max),
                    bot.games,
                    band_text(band.min, band.max),
                ),
            ));
        }
    }
    findings
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bands::parse;
    use crate::metrics::{aggregate, GameMetrics};

    fn bots(revolts: u32) -> Vec<BotMetrics> {
        let game = GameMetrics {
            revolts,
            ..Default::default()
        };
        aggregate(&[("chaos".to_owned(), game)])
    }

    #[test]
    fn drift_outside_an_active_band_is_a_warning() {
        let bands =
            parse(r#"{"revolts_per_game": {"min": 1.0, "max": 5.0, "status": "initialised"}}"#)
                .unwrap();
        assert!(check(&bots(3), &bands).is_empty());
        let found = check(&bots(9), &bands);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].oracle, OracleKind::BalanceDrift);
        assert_eq!(found[0].severity, Severity::Warning);
        assert_eq!(found[0].bot, "chaos");
        assert!(found[0].message.contains("revolts_per_game"));
        assert!(found[0].message.contains("9"));
        assert!(found[0].message.contains("[1 ; 5]"), "{}", found[0].message);
    }

    #[test]
    fn band_to_initialise_never_raises() {
        let bands =
            parse(r#"{"revolts_per_game": {"max": 1.0, "status": "to_initialise"}}"#).unwrap();
        assert!(check(&bots(99), &bands).is_empty());
    }

    #[test]
    fn french_numbers() {
        assert_eq!(fr(2.5), "2,5");
        assert_eq!(fr(3.0), "3");
        assert_eq!(fr(0.126), "0,13");
    }
}
