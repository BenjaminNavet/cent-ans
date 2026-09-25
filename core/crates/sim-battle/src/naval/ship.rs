//! A ship in a naval battle (lot NV1): hull, fire, crew groups, orders.

use data_model::{Ability, Propulsion, ShipClass, UnitCategory};
use serde::Serialize;

use crate::setup::{SideId, UnitSetup};
use crate::shot::MissileKind;

/// Men of one embarked regiment aboard a ship.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Crew {
    /// Index of the regiment in its side's units.
    pub unit: usize,
    pub unit_type: String,
    pub category: UnitCategory,
    pub men: f64,
    pub initial: f64,
    /// 0-100 stats of the regiment.
    pub melee: f64,
    pub ranged: f64,
    pub armor: f64,
    /// Bow or crossbow range at sea (0 = no missile).
    pub range: f64,
    /// Volleys left.
    pub ammo: f64,
    pub missile: MissileKind,
    pub reload: f64,
    /// Bows slacken in the rain.
    pub rain_penalty: bool,
}

impl Crew {
    pub fn from_unit(unit_index: usize, unit: &UnitSetup, men: u32, rules: &RangeRules) -> Crew {
        let bolt = unit.unit_type.contains("crossbow") || unit.abilities.contains(&Ability::Pavise);
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
            ammo: f64::from(unit.stats.ammo.min(60)),
            missile: if bolt {
                MissileKind::Bolt
            } else {
                MissileKind::Arrow
            },
            reload: 0.0,
            rain_penalty: unit.abilities.contains(&Ability::RainPenalty),
        }
    }

    pub fn shoots(&self) -> bool {
        self.range > 0.0 && self.ammo >= 1.0 && self.men >= 1.0
    }
}

/// Ranges of bows and crossbows at sea.
#[derive(Debug, Clone, Copy)]
pub struct RangeRules {
    pub bow: f64,
    pub crossbow: f64,
}

/// What a ship has been told to do.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ShipOrder {
    /// Keep station, shoot at will, fight back.
    Hold,
    MoveTo {
        x: f64,
        z: f64,
    },
    /// Close, throw grapples and board.
    Board {
        target: u32,
    },
    /// Close to bow range and shoot.
    Shoot {
        target: u32,
    },
    /// Galleys: drive the spur into the target.
    Ram {
        target: u32,
    },
    /// Cut the grapples and sail away.
    Disengage,
}

impl ShipOrder {
    pub fn key(self) -> &'static str {
        match self {
            ShipOrder::Hold => "hold",
            ShipOrder::MoveTo { .. } => "move",
            ShipOrder::Board { .. } => "board",
            ShipOrder::Shoot { .. } => "shoot",
            ShipOrder::Ram { .. } => "ram",
            ShipOrder::Disengage => "disengage",
        }
    }

    pub fn target(self) -> Option<u32> {
        match self {
            ShipOrder::Board { target }
            | ShipOrder::Shoot { target }
            | ShipOrder::Ram { target } => Some(target),
            _ => None,
        }
    }
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

impl ShipStatus {
    pub fn key(self) -> &'static str {
        match self {
            ShipStatus::Afloat => "afloat",
            ShipStatus::Captured { .. } => "captured",
            ShipStatus::Abandoned => "abandoned",
            ShipStatus::Sinking { .. } => "sinking",
            ShipStatus::Sunk => "sunk",
            ShipStatus::Escaped => "escaped",
        }
    }
}

/// A ship during the battle.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Ship {
    pub id: u32,
    pub side: SideId,
    /// Index in its side's `ships` setup.
    pub index: usize,
    pub name: String,
    pub class: ShipClass,
    pub x: f64,
    pub z: f64,
    /// Radians, direction (cos, sin) in the x-z plane.
    pub heading: f64,
    pub speed: f64,
    pub hull: f64,
    /// 0-1: share of the ship aflame.
    pub fire: f64,
    pub status: ShipStatus,
    pub crew: Vec<Crew>,
    pub sailors: f64,
    pub sailors_initial: f64,
    /// Rowers' stamina, seconds of full stroke left.
    pub stamina: f64,
    /// 0-100.
    pub morale: f64,
    pub order: ShipOrder,
    /// Ids of the ships grappled to this one.
    pub grappled: Vec<u32>,
    pub chain: Option<u32>,
    pub fireship: bool,
    pub fire_arrows: bool,
    pub flagship: bool,
    /// Running away (broken morale): no more orders.
    pub fleeing: bool,
    /// Men in the water after the ship went down or burnt out: rescued if
    /// their side holds the sea, lost otherwise.
    pub swimmers: Vec<f64>,
    /// Drowned men per crew group.
    pub drowned: Vec<f64>,
    /// Men taken prisoner (crew of a captured ship).
    pub prisoners: Vec<f64>,
    /// Last shooting target (rendering).
    pub last_target: Option<u32>,
    /// Seconds spent in melee (rendering, AI).
    pub melee_time: f64,
}

impl Ship {
    pub fn is_afloat(&self) -> bool {
        self.status == ShipStatus::Afloat
    }

    /// In the fight: afloat, not running away.
    pub fn is_active(&self) -> bool {
        self.is_afloat() && !self.fleeing
    }

    pub fn soldiers(&self) -> f64 {
        self.crew.iter().map(|c| c.men).sum()
    }

    pub fn soldiers_initial(&self) -> f64 {
        self.crew.iter().map(|c| c.initial).sum()
    }

    /// Soldiers and sailors (rowers do not fight).
    pub fn fighting_men(&self) -> f64 {
        self.soldiers() + self.sailors
    }

    pub fn fighting_initial(&self) -> f64 {
        self.soldiers_initial() + self.sailors_initial
    }

    pub fn deck_height(&self) -> f64 {
        self.class.freeboard_m
    }

    pub fn castle_height(&self) -> f64 {
        self.class.castle_height()
    }

    /// Radius of the circle used for spacing and grapples.
    pub fn radius(&self) -> f64 {
        0.3 * self.class.length_m + 0.25 * self.class.beam_m
    }

    pub fn is_galley(&self) -> bool {
        self.class.ram > 0.0 && self.class.propulsion == Propulsion::Oars
    }

    pub fn distance_to(&self, other: &Ship) -> f64 {
        ((self.x - other.x).powi(2) + (self.z - other.z).powi(2)).sqrt()
    }

    /// Gap between the two spacing circles.
    pub fn gap_to(&self, other: &Ship) -> f64 {
        self.distance_to(other) - self.radius() - other.radius()
    }

    /// Average armour of the men aboard (sailors count as 10).
    pub fn average_armor(&self) -> f64 {
        let men = self.fighting_men();
        if men <= 0.0 {
            return 0.0;
        }
        (self.crew.iter().map(|c| c.men * c.armor).sum::<f64>() + self.sailors * 10.0) / men
    }

    /// Melee power: men × melee / 100 × morale factor (shooters with arrows
    /// left fight at half strength, sailors at a third).
    pub fn melee_power(&self) -> f64 {
        let morale = 0.5 + self.morale / 200.0;
        let soldiers: f64 = self
            .crew
            .iter()
            .map(|c| {
                let share = if c.shoots() { 0.5 } else { 1.0 };
                c.men * c.melee / 100.0 * share
            })
            .sum();
        (soldiers + self.sailors * 0.5 * 0.25) * morale
    }

    /// Shooting power: shooters × ranged / 100.
    pub fn ranged_power(&self) -> f64 {
        self.crew
            .iter()
            .filter(|c| c.shoots())
            .map(|c| c.men * c.ranged / 100.0)
            .sum()
    }

    /// Kills `amount` men spread over the soldiers and sailors in
    /// proportion to their numbers, the armour of each group stopping
    /// `armor_share` per 100 points. Returns the men killed.
    pub fn take_losses(&mut self, amount: f64, armor_share: f64) -> f64 {
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
        let loss = (amount * share * (1.0 - 0.1 * armor_share)).min(self.sailors);
        self.sailors -= loss;
        killed + loss
    }

    /// Kills the same share of every group (fire, drowning).
    pub fn take_share(&mut self, share: f64) -> f64 {
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

    pub fn forward(&self) -> (f64, f64) {
        (self.heading.cos(), self.heading.sin())
    }
}
