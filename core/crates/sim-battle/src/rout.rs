//! EP10 (ADR 0067): direction of the rout and contagion of morale. Rules in
//! `data/rules/battle_rout.json` (schema
//! `data/schemas/battle_rout_rules.schema.json`). Field battles only: the
//! sieges keep their rule (flight away from the nearest enemy, every routing
//! friend counts).
//!
//! - **Flight.** A routing regiment runs to the rear of its army, away from
//!   the front ([`FlightRules::direction`]); an able enemy standing on that
//!   way makes it swerve (it does not run through the enemy), and so does an
//!   able friend (it runs round its own line rather than through it). The
//!   nearest enemy no longer sets the direction: an enemy on the flank of a
//!   wing used to send the fugitives along the line.
//! - **Contagion.** Every routing friend within
//!   [`ContagionRules::radius_m`] weighs on the morale of a regiment
//!   ([`ContagionRules::weight`]): fully when it gives way beside it or in
//!   front of it, [`ContagionRules::behind_weight`] only once it is
//!   [`ContagionRules::behind_depth_m`] behind it, running away from the
//!   enemy. The morale loss is [`ContagionRules::morale_per_second`] per unit
//!   of weight, at most [`ContagionRules::max_weight`].

use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

/// Contents of `data/rules/battle_rout.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RoutRules {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub contagion: ContagionRules,
    pub flight: FlightRules,
}

/// How a routing friend shakes the regiments near it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContagionRules {
    /// A routing friend farther than this does not count (metres).
    pub radius_m: f64,
    /// Morale lost per second per unit of weight.
    pub morale_per_second: f64,
    /// Cap on the sum of the weights.
    pub max_weight: f64,
    /// Up to this far behind the regiment (along the axis of the front) a
    /// routing friend still counts as beside it: weight 1 (metres).
    pub beside_depth_m: f64,
    /// Beyond this far behind, it weighs [`Self::behind_weight`]; linear in
    /// between (metres).
    pub behind_depth_m: f64,
    /// Weight of a routing friend already behind the regiment.
    pub behind_weight: f64,
}

/// Where a routing regiment runs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FlightRules {
    /// An able enemy on the way, closer than this ahead of the fugitive,
    /// makes it swerve (metres).
    pub lookahead_m: f64,
    /// Lateral offset under which an enemy is on the way (metres).
    pub enemy_clearance_m: f64,
    /// Strength of the swerve before an enemy (the rear counts 1).
    pub enemy_weight: f64,
    /// An able friend on the way, closer than this ahead, is gone round
    /// (metres).
    pub friend_lookahead_m: f64,
    /// Lateral offset under which a friend is on the way (metres).
    pub friend_clearance_m: f64,
    /// Strength of the swerve before a friend (the rear counts 1).
    pub friend_weight: f64,
}

const BUNDLED: &str = include_str!("../../../../data/rules/battle_rout.json");

impl RoutRules {
    /// `data/rules/battle_rout.json` as compiled into the crate.
    pub fn bundled() -> &'static RoutRules {
        static RULES: OnceLock<RoutRules> = OnceLock::new();
        RULES.get_or_init(|| {
            // TEMP EP10 tuning: remove before merge.
            if let Ok(path) = std::env::var("EP10_RULES") {
                let text = std::fs::read_to_string(path).unwrap();
                return serde_json::from_str(&text).unwrap();
            }
            serde_json::from_str(BUNDLED).expect("data/rules/battle_rout.json is valid")
        })
    }
}

impl ContagionRules {
    /// Weight of a routing friend at offset `(dx, dz)` from a regiment whose
    /// army faces `forward` (unit vector towards the enemy): 0 out of reach,
    /// 1 beside or in front, [`Self::behind_weight`] well behind.
    pub fn weight(&self, dx: f64, dz: f64, forward: (f64, f64)) -> f64 {
        if dx * dx + dz * dz >= self.radius_m * self.radius_m {
            return 0.0;
        }
        let behind = -(dx * forward.0 + dz * forward.1);
        if behind <= self.beside_depth_m {
            return 1.0;
        }
        if behind >= self.behind_depth_m {
            return self.behind_weight;
        }
        let t = (behind - self.beside_depth_m) / (self.behind_depth_m - self.beside_depth_m);
        1.0 + (self.behind_weight - 1.0) * t
    }

    /// Morale lost per second for a total weight of routing friends.
    pub fn morale_rate(&self, weight: f64) -> f64 {
        weight.min(self.max_weight) * self.morale_per_second
    }
}

impl FlightRules {
    /// Unit direction of flight of a regiment at `pos` whose army's rear is
    /// `rear` (unit vector away from the front), with the able enemies and
    /// the able friends around it.
    pub fn direction(
        &self,
        pos: (f64, f64),
        rear: (f64, f64),
        enemies: impl IntoIterator<Item = (f64, f64)>,
        friends: impl IntoIterator<Item = (f64, f64)>,
    ) -> (f64, f64) {
        // Lateral axis of the flight (left of `rear`).
        let side = (-rear.1, rear.0);
        let push = |p: (f64, f64), lookahead: f64, clearance: f64, weight: f64| {
            let (ox, oz) = (p.0 - pos.0, p.1 - pos.1);
            let ahead = ox * rear.0 + oz * rear.1;
            let lateral = ox * side.0 + oz * side.1;
            if ahead <= 0.0 || ahead >= lookahead || lateral.abs() >= clearance {
                return 0.0;
            }
            // Away from the obstacle; one straight ahead is passed on the left.
            let away = if lateral > 0.0 { -1.0 } else { 1.0 };
            away * weight * (1.0 - lateral.abs() / clearance) * (1.0 - ahead / lookahead)
        };
        let enemy: f64 = enemies
            .into_iter()
            .map(|e| {
                push(
                    e,
                    self.lookahead_m,
                    self.enemy_clearance_m,
                    self.enemy_weight,
                )
            })
            .sum();
        // Going round friends stays a light swerve, however many of them
        // (regiments piled on one spot must not send the flight sideways).
        let friend: f64 = friends
            .into_iter()
            .map(|f| {
                push(
                    f,
                    self.friend_lookahead_m,
                    self.friend_clearance_m,
                    self.friend_weight,
                )
            })
            .sum::<f64>()
            .clamp(-self.friend_weight, self.friend_weight);
        let steer = enemy + friend;
        let (x, z) = (rear.0 + side.0 * steer, rear.1 + side.1 * steer);
        let norm = (x * x + z * z).sqrt();
        (x / norm, z / norm)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_friend_behind_weighs_less_than_one_beside() {
        let c = &RoutRules::bundled().contagion;
        let forward = (0.0, 1.0);
        let beside = c.weight(60.0, 0.0, forward);
        let ahead = c.weight(0.0, 60.0, forward);
        let behind = c.weight(0.0, -100.0, forward);
        let midway = c.weight(0.0, -(c.beside_depth_m + c.behind_depth_m) / 2.0, forward);
        assert_eq!(beside, 1.0);
        assert_eq!(ahead, 1.0);
        assert_eq!(behind, c.behind_weight);
        assert!(behind < midway && midway < beside);
        assert_eq!(c.weight(0.0, -(c.radius_m + 1.0), forward), 0.0);
        assert!(c.morale_rate(10.0) <= c.max_weight * c.morale_per_second + 1e-12);
    }

    #[test]
    fn the_flight_runs_to_the_rear_whatever_the_nearest_enemy() {
        let f = &RoutRules::bundled().flight;
        let rear = (0.0, 1.0);
        // An enemy on the flank does not turn the flight along the line.
        let d = f.direction((0.0, 0.0), rear, [(-10.0, 0.0)], []);
        assert!((d.0 - 0.0).abs() < 1e-9 && (d.1 - 1.0).abs() < 1e-9);
        // An enemy on the way: swerve, still away from the front.
        let d = f.direction((0.0, 0.0), rear, [(10.0, 20.0)], []);
        assert!(d.0 < -0.3 && d.1 > 0.0, "{d:?}");
        // A friend on the way is gone round.
        let d = f.direction((0.0, 0.0), rear, [], [(-5.0, 15.0)]);
        assert!(d.0 > 0.1 && d.1 > 0.5, "{d:?}");
    }
}
