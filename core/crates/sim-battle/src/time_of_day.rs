//! Time of day of a battle (lot EP8, ADR 0055): dawn, morning, midday,
//! afternoon, dusk and night (`data/rules/battle_time_of_day.json`, schema
//! `data/schemas/battle_time_of_day_rules.schema.json`).
//!
//! The day moves on while the battle lasts
//! ([`TimeOfDayRules::minutes_per_battle_second`]). Each phase carries a
//! visibility factor on the effective range of the shooters, multiplied with
//! the weather's ([`crate::field::Weather::range_factor`]): the low light of
//! dawn and dusk shortens it. The campaign draws the starting phase from the
//! battle (see [`TimeOfDayRules::campaign_hour`]); quick battles choose it.
//! Without either, the battle starts at [`TimeOfDayRules::default_hour`]
//! (midday: full visibility, so the older battles are unchanged).

use data_model::util::splitmix64;
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

/// One phase of the day.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DayPhase {
    pub key: String,
    /// French display name.
    pub label: String,
    pub from_hour: f64,
    /// End (excluded); below `from_hour` when the phase spans midnight.
    pub to_hour: f64,
    /// Starting hour of a battle fought in this phase.
    pub start_hour: f64,
    /// Factor on the effective range of the shooters.
    pub visibility: f64,
    /// Journal line when the phase begins.
    pub announce: String,
    /// Offered by the quick battle settings (default true).
    #[serde(default = "yes", skip_serializing_if = "is_true")]
    pub selectable: bool,
}

fn yes() -> bool {
    true
}

fn is_true(value: &bool) -> bool {
    *value
}

impl DayPhase {
    /// `hour` (in `[0, 24)`) falls in this phase.
    pub fn contains(&self, hour: f64) -> bool {
        if self.from_hour <= self.to_hour {
            hour >= self.from_hour && hour < self.to_hour
        } else {
            hour >= self.from_hour || hour < self.to_hour
        }
    }
}

/// Weight of a starting phase in the campaign draw.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PhaseWeight {
    pub phase: String,
    pub weight: u32,
}

/// Contents of `data/rules/battle_time_of_day.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TimeOfDayRules {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Minutes of the day per simulated battle second.
    pub minutes_per_battle_second: f64,
    /// Starting hour when nobody gives one.
    pub default_hour: f64,
    pub phases: Vec<DayPhase>,
    pub campaign_draw: Vec<PhaseWeight>,
}

const BUNDLED: &str = include_str!("../../../../data/rules/battle_time_of_day.json");

/// `hour` brought back to `[0, 24)`.
pub fn wrap_hour(hour: f64) -> f64 {
    if !hour.is_finite() {
        return 0.0;
    }
    let h = hour.rem_euclid(24.0);
    if h >= 24.0 {
        0.0
    } else {
        h
    }
}

impl TimeOfDayRules {
    /// `data/rules/battle_time_of_day.json` as compiled into the crate.
    pub fn bundled() -> &'static TimeOfDayRules {
        static RULES: OnceLock<TimeOfDayRules> = OnceLock::new();
        RULES.get_or_init(|| {
            serde_json::from_str(BUNDLED).expect("data/rules/battle_time_of_day.json is valid")
        })
    }

    /// Hour of the day `elapsed_s` battle seconds after `start_hour`.
    pub fn hour_after(&self, start_hour: f64, elapsed_s: f64) -> f64 {
        wrap_hour(start_hour + elapsed_s.max(0.0) * self.minutes_per_battle_second / 60.0)
    }

    /// The phase `hour` falls in (the first phase if the file leaves a gap).
    pub fn phase_at(&self, hour: f64) -> &DayPhase {
        let hour = wrap_hour(hour);
        self.phases
            .iter()
            .find(|p| p.contains(hour))
            .unwrap_or(&self.phases[0])
    }

    /// The phase named `key`.
    pub fn phase(&self, key: &str) -> Option<&DayPhase> {
        self.phases.iter().find(|p| p.key == key)
    }

    /// Factor on the shooters' range at `hour`.
    pub fn visibility_at(&self, hour: f64) -> f64 {
        self.phase_at(hour).visibility.clamp(0.05, 1.0)
    }

    /// Starting hour of a quick battle fought in phase `key`.
    pub fn start_hour_of(&self, key: &str) -> Option<f64> {
        self.phase(key).map(|p| p.start_hour)
    }

    /// Starting hour of a campaign battle, drawn from `key` (a hash of the
    /// battle: turn, province, index). Pure: it consumes no random stream,
    /// so neither the campaign nor the battle draws move.
    pub fn campaign_hour(&self, key: u64) -> f64 {
        let total: u64 = self.campaign_draw.iter().map(|w| u64::from(w.weight)).sum();
        if total == 0 {
            return self.default_hour;
        }
        let mut roll = splitmix64(key) % total;
        for entry in &self.campaign_draw {
            let weight = u64::from(entry.weight);
            if roll < weight {
                return self
                    .start_hour_of(&entry.phase)
                    .unwrap_or(self.default_hour);
            }
            roll -= weight;
        }
        self.default_hour
    }
}

/// Key of a campaign battle for [`TimeOfDayRules::campaign_hour`]: turn,
/// index of the pending battle and province id (FNV-1a).
pub fn campaign_battle_key(turn: u32, index: usize, province: &str) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    let mut feed = |byte: u8| {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0100_0000_01b3);
    };
    for b in turn.to_le_bytes() {
        feed(b);
    }
    for b in (index as u64).to_le_bytes() {
        feed(b);
    }
    for b in province.bytes() {
        feed(b);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rules() -> &'static TimeOfDayRules {
        TimeOfDayRules::bundled()
    }

    #[test]
    fn phases_cover_the_whole_day_without_overlap() {
        let r = rules();
        for step in 0..(24 * 12) {
            let hour = f64::from(step) / 12.0;
            let count = r.phases.iter().filter(|p| p.contains(hour)).count();
            assert_eq!(count, 1, "hour {hour} is in {count} phases");
        }
    }

    #[test]
    fn default_hour_has_full_visibility() {
        let r = rules();
        assert_eq!(r.phase_at(r.default_hour).key, "midday");
        assert_eq!(r.visibility_at(r.default_hour), 1.0);
    }

    #[test]
    fn dawn_and_dusk_shorten_the_range() {
        let r = rules();
        let dawn = r.start_hour_of("dawn").unwrap();
        let dusk = r.start_hour_of("dusk").unwrap();
        assert!(r.visibility_at(dawn) < 1.0);
        assert!(r.visibility_at(dusk) < 1.0);
        assert!(r.visibility_at(r.start_hour_of("morning").unwrap()) > r.visibility_at(dawn));
        // The night is darker still.
        assert!(r.visibility_at(23.0) < r.visibility_at(dusk));
    }

    #[test]
    fn the_day_moves_on_during_the_battle() {
        let r = rules();
        // 0.2 min per battle second: 1800 s of battle = 360 min = 6 h.
        let h = r.hour_after(12.0, 1800.0);
        assert!((h - 18.0).abs() < 1e-9, "{h}");
        assert_eq!(r.phase_at(r.hour_after(5.5, 0.0)).key, "dawn");
        // Dawn turns into morning after 1.5 h = 450 s.
        assert_eq!(r.phase_at(r.hour_after(5.5, 460.0)).key, "morning");
        // Wraps around midnight.
        assert!((r.hour_after(23.0, 600.0) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn campaign_draw_is_deterministic_and_varied() {
        let r = rules();
        let a = r.campaign_hour(campaign_battle_key(12, 0, "prov_ponthieu"));
        let b = r.campaign_hour(campaign_battle_key(12, 0, "prov_ponthieu"));
        assert_eq!(a, b);
        let mut seen = std::collections::BTreeSet::new();
        for turn in 0..200 {
            let hour = r.campaign_hour(campaign_battle_key(turn, 0, "prov_guyenne"));
            seen.insert(r.phase_at(hour).key.clone());
        }
        for key in ["dawn", "morning", "midday", "afternoon", "dusk"] {
            assert!(seen.contains(key), "{key} never drawn");
        }
        assert!(!seen.contains("night"));
    }

    #[test]
    fn azincourt_report_matches_the_documented_compression() {
        // EP8b: bug report said the battle-day clock "runs much faster than the battle" on
        // the Agincourt map (ADR 0035: start 10 h 30) — the banner read "Midi" after 2 min 50 s
        // (170 s) of simulated battle. That is exactly the documented compression (0.2 minute of
        // day per simulated second: 20 min of battle = 4 h of day), applied once, not a bug.
        // Pinned so a change to the factor, or a regression that applies it twice, is caught.
        let r = rules();
        let hour = r.hour_after(10.5, 170.0);
        assert!((hour - (10.5 + 170.0 * 0.2 / 60.0)).abs() < 1e-9, "{hour}");
        assert!((hour - 11.066_666_666_666_666).abs() < 1e-9, "{hour}");
        assert_eq!(r.phase_at(hour).key, "midday");
    }

    #[test]
    fn wrap_hour_stays_in_range() {
        assert_eq!(wrap_hour(24.0), 0.0);
        assert_eq!(wrap_hour(-1.0), 23.0);
        assert_eq!(wrap_hour(f64::NAN), 0.0);
    }
}
