//! Roles a phase gives to a division: hold, shoot, reserve.

use serde::{Deserialize, Serialize};

use super::division::DivisionKind;

/// What a division does during a phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    /// Keep the ground and receive the enemy (counter-charge at close range).
    Hold,
    /// Shooters trade volleys in front of the line.
    Shoot,
    /// Kept back; its foot plugs a gap in the line.
    Reserve,
}

/// The role of each division in a phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PhaseRoles {
    pub vanguard: Role,
    pub battle: Role,
    pub rear: Role,
}

impl PhaseRoles {
    pub fn of(&self, kind: DivisionKind) -> Role {
        match kind {
            DivisionKind::Vanguard => self.vanguard,
            DivisionKind::Battle => self.battle,
            DivisionKind::Rear => self.rear,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roles_are_looked_up_by_division() {
        let roles = PhaseRoles {
            vanguard: Role::Shoot,
            battle: Role::Hold,
            rear: Role::Reserve,
        };
        assert_eq!(roles.of(DivisionKind::Vanguard), Role::Shoot);
        assert_eq!(roles.of(DivisionKind::Rear), Role::Reserve);
    }
}
