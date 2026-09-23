//! Real-time-with-pause battle simulation (fixed tick).
//!
//! Placeholder for M0: only tracks elapsed time. The real simulation (units,
//! morale, terrain, weather) arrives at M7.

/// State of one battle.
#[derive(Debug, Clone, Default)]
pub struct BattleSim {
    /// Simulated seconds elapsed since the battle started.
    pub elapsed: f64,
    /// Number of ticks processed so far.
    pub ticks: u64,
}

impl BattleSim {
    pub fn new() -> Self {
        Self::default()
    }

    /// Advances the simulation by `dt` seconds.
    pub fn tick(&mut self, dt: f64) {
        self.elapsed += dt;
        self.ticks += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tick_accumulates_time() {
        let mut sim = BattleSim::new();
        sim.tick(0.5);
        sim.tick(0.25);
        assert_eq!(sim.ticks, 2);
        assert!((sim.elapsed - 0.75).abs() < f64::EPSILON);
    }
}
