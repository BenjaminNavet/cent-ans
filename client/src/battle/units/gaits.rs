//! Cavalry gaits: walk, trot, gallop and turns chosen from the regiment's smoothed speed and
//! heading change (rendering only, no game rule). Port of game/scripts/battle/battle_cavalry_gaits.gd;
//! thresholds from `data/fx/battle_animation.json` (`cavalry_gaits`).

use serde::Deserialize;

/// Thresholds (`cavalry_gaits` of `battle_animation.json`).
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct GaitSettings {
    pub enabled: bool,
    pub trot_min_speed: f32,
    pub gallop_min_speed: f32,
    pub hysteresis_mps: f32,
    pub turn_min_rate: f32,
    pub turn_hysteresis: f32,
}

impl Default for GaitSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            trot_min_speed: 3.4,
            gallop_min_speed: 4.2,
            hysteresis_mps: 0.4,
            turn_min_rate: 0.16,
            turn_hysteresis: 0.05,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum Band {
    #[default]
    Walk,
    Trot,
    Gallop,
}

/// Per-regiment memory giving the hysteresis (`mem` of the GDScript).
#[derive(Debug, Clone, Copy, Default)]
pub struct GaitMemory {
    pub band: Band,
    /// -1 right, 0 straight, 1 left.
    pub turn: i8,
}

/// Render state keys of `BattleSkinned.state_config`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Gait {
    Marching,
    Running,
    Trotting,
    TurnL,
    TurnR,
    TrotTurnL,
    TrotTurnR,
}

impl Gait {
    pub fn key(self) -> &'static str {
        match self {
            Gait::Marching => "marching",
            Gait::Running => "running",
            Gait::Trotting => "trotting",
            Gait::TurnL => "turn_l",
            Gait::TurnR => "turn_r",
            Gait::TrotTurnL => "trot_turn_l",
            Gait::TrotTurnR => "trot_turn_r",
        }
    }
}

/// Gait and turn of a regiment; `speed` in m/s, `turn_rate` in rad/s (> 0 = to the left).
pub fn pick(
    cfg: &GaitSettings,
    mem: &mut GaitMemory,
    speed: f32,
    running: bool,
    turn_rate: f32,
) -> Gait {
    let band = mem.band;
    let hyst = cfg.hysteresis_mps;
    let up_trot = cfg.trot_min_speed + if band >= Band::Trot { 0.0 } else { hyst };
    let up_gallop = cfg.gallop_min_speed + if band >= Band::Gallop { 0.0 } else { hyst };
    let down_trot = cfg.trot_min_speed - if band >= Band::Trot { hyst } else { 0.0 };
    let down_gallop = cfg.gallop_min_speed - if band >= Band::Gallop { hyst } else { 0.0 };
    let mut next = Band::Walk;
    if running
        && speed
            >= if band == Band::Gallop {
                down_gallop
            } else {
                up_gallop
            }
    {
        next = Band::Gallop;
    } else if speed >= if band >= Band::Trot { down_trot } else { up_trot } {
        next = Band::Trot;
    }
    if !running && next == Band::Gallop {
        next = Band::Trot;
    }
    mem.band = next;
    let limit = cfg.turn_min_rate
        - if mem.turn != 0 {
            cfg.turn_hysteresis
        } else {
            0.0
        };
    mem.turn = if turn_rate.abs() >= limit && next != Band::Gallop {
        if turn_rate > 0.0 { 1 } else { -1 }
    } else {
        0
    };
    if next == Band::Gallop {
        return Gait::Running;
    }
    match (mem.turn, next) {
        (0, Band::Trot) => Gait::Trotting,
        (0, _) => Gait::Marching,
        (t, Band::Trot) if t > 0 => Gait::TrotTurnL,
        (_, Band::Trot) => Gait::TrotTurnR,
        (t, _) if t > 0 => Gait::TurnL,
        _ => Gait::TurnR,
    }
}

/// Heading change (rad/s) between two facings, wrapped to -pi..pi per step.
pub fn turn_rate(previous: f32, current: f32, dt: f32) -> f32 {
    if dt <= 0.0 {
        return 0.0;
    }
    let tau = std::f32::consts::TAU;
    let delta = (current - previous + std::f32::consts::PI).rem_euclid(tau) - std::f32::consts::PI;
    delta / dt
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bands_follow_speed_with_hysteresis() {
        let cfg = GaitSettings::default();
        let mut mem = GaitMemory::default();
        assert_eq!(pick(&cfg, &mut mem, 2.0, false, 0.0), Gait::Marching);
        // 3.5 is above trot_min but inside the hysteresis band when coming from walk.
        assert_eq!(pick(&cfg, &mut mem, 3.5, false, 0.0), Gait::Marching);
        assert_eq!(pick(&cfg, &mut mem, 3.9, false, 0.0), Gait::Trotting);
        // Once trotting, 3.2 stays a trot (down threshold 3.0).
        assert_eq!(pick(&cfg, &mut mem, 3.2, false, 0.0), Gait::Trotting);
        assert_eq!(pick(&cfg, &mut mem, 2.9, false, 0.0), Gait::Marching);
    }

    #[test]
    fn gallop_needs_running_and_turns_use_the_band() {
        let cfg = GaitSettings::default();
        let mut mem = GaitMemory::default();
        assert_eq!(pick(&cfg, &mut mem, 5.0, false, 0.0), Gait::Trotting);
        assert_eq!(pick(&cfg, &mut mem, 5.0, true, 0.0), Gait::Running);
        let mut mem = GaitMemory::default();
        assert_eq!(pick(&cfg, &mut mem, 1.0, false, 0.3), Gait::TurnL);
        assert_eq!(pick(&cfg, &mut mem, 1.0, false, -0.3), Gait::TurnR);
        // Turn hysteresis: 0.13 keeps an ongoing turn, a fresh one needs 0.16.
        assert_eq!(pick(&cfg, &mut mem, 1.0, false, -0.13), Gait::TurnR);
        let mut mem = GaitMemory::default();
        assert_eq!(pick(&cfg, &mut mem, 1.0, false, 0.13), Gait::Marching);
        let mut mem = GaitMemory::default();
        pick(&cfg, &mut mem, 4.0, false, 0.0);
        pick(&cfg, &mut mem, 4.0, false, 0.0);
        assert_eq!(pick(&cfg, &mut mem, 4.0, false, 0.5), Gait::TrotTurnL);
    }

    #[test]
    fn turn_rate_wraps() {
        let rate = turn_rate(3.0, -3.0, 1.0);
        assert!(rate > 0.0 && rate < 1.0, "{rate}");
        assert_eq!(turn_rate(0.0, 1.0, 0.0), 0.0);
    }
}
