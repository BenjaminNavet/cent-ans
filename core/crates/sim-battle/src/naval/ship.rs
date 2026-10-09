//! A ship in a naval battle: hull, fire, crew groups.

use data_model::{Ability, NavalRules, Propulsion, ShipClass, UnitCategory};
use serde::Serialize;

use crate::setup::{SideId, UnitSetup};
use crate::shot::MissileKind;

/// Men of one embarked regiment aboard a ship.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Crew {
    /// Index of the regiment in its side's units.
    pub(crate) unit: usize,
    pub(crate) unit_type: String,
    pub(crate) category: UnitCategory,
    pub(crate) men: f64,
    pub(crate) initial: f64,
    /// 0-100 stats of the regiment.
    pub(crate) melee: f64,
    pub(crate) ranged: f64,
    pub(crate) armor: f64,
    /// Bow or crossbow range at sea (0 = no missile).
    pub(crate) range: f64,
    /// Volleys left.
    pub(crate) ammo: f64,
    pub(crate) missile: MissileKind,
    pub(crate) reload: f64,
    /// Bows slacken in the rain.
    pub(crate) rain_penalty: bool,
}

impl Crew {
    pub(crate) fn from_unit(
        unit_index: usize,
        unit: &UnitSetup,
        men: u32,
        rules: &RangeRules,
    ) -> Crew {
        // The missile declared by the unit type wins over the id heuristic.
        let bolt = match unit.missile {
            Some(missile) => missile == data_model::Missile::Bolt,
            None => {
                unit.unit_type.contains("crossbow") || unit.abilities.contains(&Ability::Pavise)
            }
        };
        let shooter = unit.stats.ranged > 0 && unit.stats.range > 0 && unit.stats.ammo > 0;
        let range = if !shooter {
            0.0
        } else if bolt {
            rules.crossbow
        } else {
            rules.bow
        };
        Crew {
            unit: unit_index,
            unit_type: unit.unit_type.clone(),
            category: unit.category,
            men: f64::from(men),
            initial: f64::from(men),
            melee: f64::from(unit.stats.melee),
            ranged: f64::from(unit.stats.ranged),
            armor: f64::from(unit.stats.armor),
            range,
            ammo: f64::from(unit.stats.ammo.min(rules.ammo_cap)),
            missile: if bolt {
                MissileKind::Bolt
            } else {
                MissileKind::Arrow
            },
            reload: 0.0,
            rain_penalty: unit.abilities.contains(&Ability::RainPenalty),
        }
    }

    pub(crate) fn shoots(&self) -> bool {
        self.range > 0.0 && self.ammo >= 1.0 && self.men >= 1.0
    }
}

/// Ranges of bows and crossbows at sea.
#[derive(Debug, Clone, Copy)]
pub(crate) struct RangeRules {
    pub(crate) bow: f64,
    pub(crate) crossbow: f64,
    /// Most volleys a group carries aboard.
    pub(crate) ammo_cap: u32,
}

/// Where a ship stands in the battle.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum ShipStatus {
    Afloat,
    /// Struck its colours to `by`: a prize.
    Captured {
        by: SideId,
    },
    /// Crew gone (burnt out, abandoned): drifts until it sinks or is taken.
    Abandoned,
    /// Going under since `since` (seconds).
    Sinking {
        since: f64,
    },
    Sunk,
    /// Left the battle.
    Escaped,
}

/// A ship during the battle.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Ship {
    pub(crate) id: u32,
    pub(crate) side: SideId,
    /// Index in its side's `ships` setup.
    pub(crate) index: usize,
    pub(crate) name: String,
    pub(crate) class: ShipClass,
    pub(crate) hull: f64,
    /// 0-1: share of the ship aflame.
    pub(crate) fire: f64,
    pub(crate) status: ShipStatus,
    pub(crate) crew: Vec<Crew>,
    pub(crate) sailors: f64,
    pub(crate) sailors_initial: f64,
    /// 0-100.
    pub(crate) morale: f64,
    pub(crate) chain: Option<u32>,
    pub(crate) fireship: bool,
    pub(crate) fire_arrows: bool,
    /// Running away (broken morale): no more orders.
    pub(crate) fleeing: bool,
    /// Men in the water after the ship went down or burnt out: rescued if
    /// their side holds the sea, lost otherwise.
    pub(crate) swimmers: Vec<f64>,
    /// Drowned men per crew group.
    pub(crate) drowned: Vec<f64>,
    /// Men taken prisoner (crew of a captured ship).
    pub(crate) prisoners: Vec<f64>,
}

impl Ship {
    pub(crate) fn is_afloat(&self) -> bool {
        self.status == ShipStatus::Afloat
    }

    /// Gone from the battle (sunk, sinking or escaped): nothing burns or fights.
    pub(crate) fn is_out(&self) -> bool {
        matches!(
            self.status,
            ShipStatus::Sunk | ShipStatus::Escaped | ShipStatus::Sinking { .. }
        )
    }

    /// The crew leaves the ship for the water: each group drowns
    /// `base + armour / 100 × armor` of its men (at most all), the rest swim.
    pub(crate) fn cast_into_sea(&mut self, base: f64, armor: f64) {
        for (k, crew) in self.crew.iter_mut().enumerate() {
            let drown = (base + crew.armor / 100.0 * armor).min(1.0);
            self.drowned[k] += crew.men * drown;
            self.swimmers[k] += crew.men * (1.0 - drown);
            crew.men = 0.0;
        }
        self.sailors = 0.0;
    }

    /// The crew strikes its colours: the men are prisoners of `by`.
    pub(crate) fn strike(&mut self, by: SideId) {
        for (k, crew) in self.crew.iter_mut().enumerate() {
            self.prisoners[k] += crew.men;
            crew.men = 0.0;
        }
        self.sailors = 0.0;
        self.status = ShipStatus::Captured { by };
    }

    pub(crate) fn soldiers(&self) -> f64 {
        self.crew.iter().map(|c| c.men).sum()
    }

    pub(crate) fn soldiers_initial(&self) -> f64 {
        self.crew.iter().map(|c| c.initial).sum()
    }

    /// Soldiers and sailors (rowers do not fight).
    pub(crate) fn fighting_men(&self) -> f64 {
        self.soldiers() + self.sailors
    }

    pub(crate) fn fighting_initial(&self) -> f64 {
        self.soldiers_initial() + self.sailors_initial
    }

    pub(crate) fn deck_height(&self) -> f64 {
        self.class.freeboard_m
    }

    pub(crate) fn castle_height(&self) -> f64 {
        self.class.castle_height()
    }

    pub(crate) fn is_galley(&self) -> bool {
        self.class.ram > 0.0 && self.class.propulsion == Propulsion::Oars
    }

    /// Melee power: men × melee / 100 × morale factor (shooters with arrows
    /// left fight at `shooter_melee_share`, sailors at `sailor_melee`).
    pub(crate) fn melee_power(&self, rules: &NavalRules) -> f64 {
        let floor = rules.melee_morale_floor;
        let morale = floor + (1.0 - floor) * self.morale / 100.0;
        let soldiers: f64 = self
            .crew
            .iter()
            .map(|c| {
                let share = if c.shoots() {
                    rules.shooter_melee_share
                } else {
                    1.0
                };
                c.men * c.melee / 100.0 * share
            })
            .sum();
        (soldiers + self.sailors * rules.sailor_melee) * morale
    }

    /// Shooting power: shooters × ranged / 100.
    pub(crate) fn ranged_power(&self) -> f64 {
        self.crew
            .iter()
            .filter(|c| c.shoots())
            .map(|c| c.men * c.ranged / 100.0)
            .sum()
    }

    /// Kills `amount` men spread over the soldiers and sailors in
    /// proportion to their numbers, the armour of each group stopping
    /// `armor_share` per 100 points. Returns the men killed.
    pub(crate) fn take_losses(&mut self, amount: f64, rules: &NavalRules, armor_share: f64) -> f64 {
        let men = self.fighting_men();
        if men <= 0.0 || amount <= 0.0 {
            return 0.0;
        }
        let mut killed = 0.0;
        for crew in &mut self.crew {
            let share = crew.men / men;
            let loss = (amount * share * (1.0 - crew.armor / 100.0 * armor_share)).min(crew.men);
            crew.men -= loss;
            killed += loss;
        }
        let share = self.sailors / men;
        let loss =
            (amount * share * (1.0 - rules.sailor_armor / 100.0 * armor_share)).min(self.sailors);
        self.sailors -= loss;
        killed + loss
    }

    /// Kills the same share of every group (fire, drowning).
    pub(crate) fn take_share(&mut self, share: f64) -> f64 {
        let share = share.clamp(0.0, 1.0);
        let mut killed = 0.0;
        for crew in &mut self.crew {
            let loss = crew.men * share;
            crew.men -= loss;
            killed += loss;
        }
        let loss = self.sailors * share;
        self.sailors -= loss;
        killed + loss
    }
}
