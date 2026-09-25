//! Direct and indirect shooting in the battle tick (lot R4, ADR 0046; the
//! rules are in [`crate::missile_arc`]).

use data_model::Ability;

use super::BattleSim;
use crate::missile_arc::{arc_clears, FireMode, MissileArcRules};
use crate::unit::{Unit, UnitState};

impl BattleSim {
    /// How `shooter` can reach `target` over the ground, if it can: direct
    /// when it sees the target, otherwise lobbed (volleys of bows only) over
    /// a crest the arrows clear, at a target a friend sees or that was seen
    /// a moment ago. Woods and walls are checked by the caller.
    pub(crate) fn fire_mode(&self, shooter: &Unit, target: &Unit) -> Option<FireMode> {
        let (from, to) = ((shooter.x, shooter.z), (target.x, target.z));
        if !self.field.blocks_sight(from, to) {
            return Some(FireMode::Direct);
        }
        let rules = MissileArcRules::bundled();
        if !shooter.has(Ability::Volley)
            || !rules.lobs(Self::missile_kind(shooter))
            || !arc_clears(
                &self.field,
                from,
                to,
                rules.max_launch_angle_deg,
                rules.clearance_m,
            )
        {
            return None;
        }
        if self.spotted(shooter, target, rules) {
            Some(FireMode::Spotted)
        } else if self.elapsed - target.seen_at <= rules.memory_s {
            Some(FireMode::Remembered)
        } else {
            None
        }
    }

    /// Does a regiment of the shooter's side (other than the shooter) see
    /// `target` from close enough to direct the shooting?
    fn spotted(&self, shooter: &Unit, target: &Unit, rules: &MissileArcRules) -> bool {
        let reach = rules.spotter_range_m * self.range_factor();
        let to = (target.x, target.z);
        self.units.iter().any(|u| {
            u.id != shooter.id
                && u.side == shooter.side
                && u.present()
                && !u.synthetic
                && u.state != UnitState::Routing
                && (u.x - to.0).hypot(u.z - to.1) <= reach
                && !self.field.blocks_sight((u.x, u.z), to)
        })
    }
}
