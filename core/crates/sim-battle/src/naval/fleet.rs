//! Deployment of a naval battle: the ships of both fleets with their crews,
//! the wind drawn from the seed and the weather gauge it gives.

use super::setup::NavalSetup;
use super::ship::{Crew, RangeRules, Ship, ShipStatus};
use crate::rng::BattleRng;
use crate::setup::SideId;

/// Why a naval setup is refused.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum NavalSetupError {
    #[error("la flotte {0} n'a aucun navire")]
    NoShips(&'static str),
    #[error("le navire {ship} embarque un régiment inconnu n°{unit}")]
    UnknownUnit { ship: String, unit: usize },
}

/// Both fleets afloat, with the wind over the sea.
#[derive(Debug, Clone, PartialEq)]
pub struct Fleets {
    pub ships: Vec<Ship>,
    /// Wind strength, 0-1.
    pub wind_strength: f64,
    /// Weather gauge: the side whose fleet lies upwind of the other.
    pub gauge: Option<SideId>,
}

impl Fleets {
    /// Both fleets afloat; wind and weather gauge drawn from the seed.
    pub fn deploy(setup: &NavalSetup, seed: u64) -> Result<Fleets, NavalSetupError> {
        if setup.attacker.ships.is_empty() {
            return Err(NavalSetupError::NoShips("assaillante"));
        }
        if setup.defender.ships.is_empty() {
            return Err(NavalSetupError::NoShips("en défense"));
        }
        let mut rng = BattleRng::from_seed(seed);
        let (wind_strength, gauge) = draw_wind(setup, &mut rng);
        let ranges = RangeRules {
            bow: setup.rules.bow_range_m,
            crossbow: setup.rules.crossbow_range_m,
        };
        let mut ships = Vec::new();
        for side in SideId::BOTH {
            let fleet = setup.side(side);
            let morale = (fleet.units.iter().map(|u| f64::from(u.morale)).sum::<f64>()
                / fleet.units.len().max(1) as f64)
                .clamp(30.0, 100.0);
            for (index, spec) in fleet.ships.iter().enumerate() {
                let mut crew = Vec::new();
                for c in &spec.crew {
                    let unit =
                        fleet
                            .units
                            .get(c.unit)
                            .ok_or_else(|| NavalSetupError::UnknownUnit {
                                ship: spec.name.clone(),
                                unit: c.unit,
                            })?;
                    if c.men > 0 {
                        crew.push(Crew::from_unit(c.unit, unit, c.men, &ranges));
                    }
                }
                let class = spec.class.clone();
                let sailors = f64::from(class.sailors);
                ships.push(Ship {
                    id: ships.len() as u32,
                    side,
                    index,
                    name: spec.name.clone(),
                    hull: f64::from(class.hull),
                    class,
                    fire: 0.0,
                    status: ShipStatus::Afloat,
                    swimmers: vec![0.0; crew.len()],
                    drowned: vec![0.0; crew.len()],
                    prisoners: vec![0.0; crew.len()],
                    crew,
                    sailors,
                    sailors_initial: sailors,
                    morale,
                    chain: spec.chain,
                    fireship: spec.fireship,
                    fire_arrows: spec.fire_arrows,
                    fleeing: false,
                });
            }
        }
        Ok(Fleets {
            ships,
            wind_strength,
            gauge,
        })
    }
}

/// Wind of the battle: its strength, and the weather gauge. The gauge goes
/// to the setup's `gauge` side or to a coin toss; the wind blows from that
/// fleet towards the other, 20-40° off the axis (the fleets face each other
/// along x, the attacker to the west), so a wind too near the cross-axis
/// gives no gauge.
fn draw_wind(setup: &NavalSetup, rng: &mut BattleRng) -> (f64, Option<SideId>) {
    let rules = &setup.rules;
    let strength = rng
        .range(rules.wind_strength_min, rules.wind_strength_max)
        .clamp(0.0, 1.0);
    let holder = setup.gauge.unwrap_or(if rng.unit() < 0.5 {
        SideId::Attacker
    } else {
        SideId::Defender
    });
    let base = match holder {
        SideId::Attacker => 0.0,
        SideId::Defender => std::f64::consts::PI,
    };
    let off = rng
        .range(rules.gauge_angle_min_deg, rules.gauge_angle_max_deg)
        .to_radians()
        * if rng.unit() < 0.5 { 1.0 } else { -1.0 };
    let alignment = (base + off).cos();
    let gauge = if alignment > rules.gauge_alignment {
        Some(SideId::Attacker)
    } else if alignment < -rules.gauge_alignment {
        Some(SideId::Defender)
    } else {
        None
    };
    (strength, gauge)
}
