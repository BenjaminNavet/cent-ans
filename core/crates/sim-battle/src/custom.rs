//! NT2: custom battle of the main menu (spec NT § NT2).
//!
//! Two sides, each a playable faction with a budget of points spent on
//! regiments of the faction's roster (a unit costs its recruitment price in
//! livres), on a field chosen by the player (terrain, season, weather, hour)
//! or, as a siege, with side 2 holding a walled place. The rules (budget
//! bounds, unit cap, generic captain) live in `data/rules/custom_battle.json`
//! ([`CustomBattleRules`]); this module checks a composition
//! ([`CustomBattle::validate`]) and turns it into a [`BattleSetup`]
//! ([`CustomBattle::battle_setup`]). The weather, when forced, is carried by
//! the replay start (`ReplayStart::weather`), not by the setup.

use std::collections::BTreeMap;
use std::sync::OnceLock;

use data_model::{
    BattleAbility, BattleOrder, BattleStandardRules, Faction, FactionId, Terrain, UnitType,
    UnitTypeId,
};
use serde::{Deserialize, Serialize};

use crate::field::Weather;
use crate::setup::{
    BattleSeason, BattleSetup, GeneralSetup, SideId, SideSetup, SiegeSetup, UnitSetup,
};
use crate::time_of_day::TimeOfDayRules;

/// `data/rules/custom_battle.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CustomBattleRules {
    #[serde(default)]
    pub description: String,
    /// Points offered to each side.
    pub default_budget: u32,
    pub min_budget: u32,
    pub max_budget: u32,
    /// Step of the budget setting in the screen.
    pub budget_step: u32,
    /// Regiments at most in one side's army.
    pub max_units_per_side: usize,
    /// Command (0-10) of each side's generic captain.
    pub general_command: u8,
    /// Fortification level (0-3) of the besieged place, by default.
    pub default_fortification: u32,
    pub max_fortification: u32,
}

const BUNDLED: &str = include_str!("../../../../data/rules/custom_battle.json");

impl CustomBattleRules {
    /// `data/rules/custom_battle.json` as compiled into the crate.
    pub fn bundled() -> &'static CustomBattleRules {
        static RULES: OnceLock<CustomBattleRules> = OnceLock::new();
        RULES.get_or_init(|| {
            serde_json::from_str(BUNDLED).expect("data/rules/custom_battle.json is valid")
        })
    }
}

/// One side of a custom battle as composed in the screen.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CustomSide {
    pub faction: String,
    /// Points of the side; `0` means the default budget.
    #[serde(default)]
    pub budget: u32,
    /// Unit type ids bought, one entry per regiment.
    #[serde(default)]
    pub units: Vec<String>,
}

/// A custom battle composition (the dictionary of the screen). Field
/// settings are keys; an empty string lets the battle draw it (weather from
/// the season, hour as in the campaign).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CustomBattle {
    pub attacker: CustomSide,
    pub defender: CustomSide,
    /// `Terrain` key (`plains`, `hills`...); empty: plains.
    #[serde(default)]
    pub terrain: String,
    /// `BattleSeason` key; empty: spring.
    #[serde(default)]
    pub season: String,
    /// `Weather` key; empty: drawn from the season.
    #[serde(default)]
    pub weather: String,
    /// Day phase key of `battle_time_of_day.json`; empty: drawn.
    #[serde(default)]
    pub hour: String,
    /// Siege: the defender (side 2) holds a walled place.
    #[serde(default)]
    pub siege: bool,
    /// Fortification level (0-3) of the place; `None`: the default.
    #[serde(default)]
    pub fortification: Option<u32>,
    /// NT1 kind of place (city, borough, castle); `None`: a ring city.
    #[serde(default)]
    pub place: Option<crate::siege_layouts::PlaceKind>,
    /// Side commanded by the player (`attacker`, `defender`); empty: AI
    /// against AI.
    #[serde(default)]
    pub player_side: String,
}

/// Cost and size of one side, for the screen.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct CustomSideReport {
    pub cost: u32,
    pub budget: u32,
    pub units: usize,
    pub max_units: usize,
}

/// Result of [`CustomBattle::validate`]: French messages, empty when the
/// battle can be fought.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct CustomReport {
    pub ok: bool,
    pub errors: Vec<String>,
    pub attacker: CustomSideReport,
    pub defender: CustomSideReport,
}

/// The data a custom battle is built from.
pub struct CustomData<'a> {
    pub unit_types: &'a BTreeMap<UnitTypeId, UnitType>,
    pub factions: &'a BTreeMap<FactionId, Faction>,
}

/// Short French name of a faction ("France", "Angleterre").
pub fn faction_name(faction: &Faction) -> String {
    faction
        .short_name
        .clone()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| faction.name.display.clone())
}

/// Whether `faction` may field `unit_type` in a custom battle: the
/// campaign's faction and culture restrictions (period and technology are
/// ignored: the battle is out of time).
pub fn in_roster(unit_type: &UnitType, faction: &Faction) -> bool {
    (unit_type.required_faction.is_empty() || unit_type.required_faction.contains(&faction.id))
        && (unit_type.required_culture.is_empty()
            || unit_type.required_culture.contains(&faction.culture))
}

/// The roster of `faction`: unit types it may buy, by category then cost.
pub fn roster<'a>(
    unit_types: &'a BTreeMap<UnitTypeId, UnitType>,
    faction: &Faction,
) -> Vec<&'a UnitType> {
    let mut out: Vec<&UnitType> = unit_types
        .values()
        .filter(|u| in_roster(u, faction))
        .collect();
    out.sort_by_key(|u| (u.category as u8, u.cost.money, u.id.as_str().to_owned()));
    out
}

/// Parses a snake_case key into a serde enum.
fn parse_key<T: serde::de::DeserializeOwned>(key: &str) -> Option<T> {
    serde_json::from_value(serde_json::Value::String(key.to_owned())).ok()
}

/// Groups thousands with a narrow space (« 6 000 »).
fn points(value: u32) -> String {
    let digits = value.to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push('\u{202f}');
        }
        out.push(c);
    }
    out
}

impl CustomBattle {
    pub fn side(&self, side: SideId) -> &CustomSide {
        match side {
            SideId::Attacker => &self.attacker,
            SideId::Defender => &self.defender,
        }
    }

    /// Budget of `side` (the default when unset).
    pub fn budget(&self, side: SideId, rules: &CustomBattleRules) -> u32 {
        match self.side(side).budget {
            0 => rules.default_budget,
            b => b,
        }
    }

    /// Forced terrain (plains when unset).
    pub fn terrain(&self) -> Option<Terrain> {
        if self.terrain.is_empty() {
            Some(Terrain::Plains)
        } else {
            parse_key(&self.terrain)
        }
    }

    pub fn season(&self) -> Option<BattleSeason> {
        if self.season.is_empty() {
            Some(BattleSeason::Spring)
        } else {
            parse_key(&self.season)
        }
    }

    /// Forced weather: `Ok(None)` drawn from the season, `Err` unknown key.
    pub fn weather(&self) -> Result<Option<Weather>, String> {
        if self.weather.is_empty() {
            return Ok(None);
        }
        parse_key(&self.weather)
            .map(Some)
            .ok_or_else(|| format!("météo inconnue : {}", self.weather))
    }

    /// Forced starting hour: `Ok(None)` drawn, `Err` unknown phase.
    pub fn start_hour(&self) -> Result<Option<f64>, String> {
        if self.hour.is_empty() {
            return Ok(None);
        }
        TimeOfDayRules::bundled()
            .start_hour_of(&self.hour)
            .map(Some)
            .ok_or_else(|| format!("heure inconnue : {}", self.hour))
    }

    pub fn player_side(&self) -> Option<SideId> {
        SideId::parse(&self.player_side)
    }

    /// Fortification level of the besieged place.
    pub fn fortification(&self, rules: &CustomBattleRules) -> u32 {
        self.fortification
            .unwrap_or(rules.default_fortification)
            .min(rules.max_fortification)
    }

    /// Checks the composition: factions, rosters, budgets, unit cap, field
    /// keys. `ok` is true when [`Self::battle_setup`] will succeed.
    pub fn validate(&self, data: &CustomData, rules: &CustomBattleRules) -> CustomReport {
        let mut report = CustomReport::default();
        for side in SideId::BOTH {
            let label = match side {
                SideId::Attacker => "Camp 1",
                SideId::Defender => "Camp 2",
            };
            let composed = self.side(side);
            let budget = self.budget(side, rules);
            let mut side_report = CustomSideReport {
                cost: 0,
                budget,
                units: composed.units.len(),
                max_units: rules.max_units_per_side,
            };
            let faction = FactionId::new(composed.faction.as_str())
                .ok()
                .and_then(|id| data.factions.get(&id))
                .filter(|f| f.playable);
            let Some(faction) = faction else {
                report.errors.push(format!(
                    "{label} : faction non jouable ou inconnue ({})",
                    composed.faction
                ));
                *report.side_mut(side) = side_report;
                continue;
            };
            let name = faction_name(faction);
            if budget < rules.min_budget || budget > rules.max_budget {
                report.errors.push(format!(
                    "{label} : budget de {} points hors des bornes ({} à {})",
                    points(budget),
                    points(rules.min_budget),
                    points(rules.max_budget)
                ));
            }
            for id in &composed.units {
                let unit_type = UnitTypeId::new(id.as_str())
                    .ok()
                    .and_then(|id| data.unit_types.get(&id));
                match unit_type {
                    Some(u) if in_roster(u, faction) => side_report.cost += u.cost.money,
                    Some(u) => report.errors.push(format!(
                        "{label} : {} n'est pas levée par {name}",
                        u.name.display
                    )),
                    None => report
                        .errors
                        .push(format!("{label} : unité inconnue ({id})")),
                }
            }
            if composed.units.is_empty() {
                report
                    .errors
                    .push(format!("{label} : l'armée n'a aucune unité"));
            }
            if composed.units.len() > rules.max_units_per_side {
                report.errors.push(format!(
                    "{label} : {} unités ({} au plus)",
                    composed.units.len(),
                    rules.max_units_per_side
                ));
            }
            if side_report.cost > budget {
                report.errors.push(format!(
                    "{label} : {} points dépensés pour un budget de {}",
                    points(side_report.cost),
                    points(budget)
                ));
            }
            *report.side_mut(side) = side_report;
        }
        if self.terrain().is_none() {
            report
                .errors
                .push(format!("terrain inconnu : {}", self.terrain));
        }
        if self.season().is_none() {
            report
                .errors
                .push(format!("saison inconnue : {}", self.season));
        }
        if let Err(error) = self.weather() {
            report.errors.push(error);
        }
        if let Err(error) = self.start_hour() {
            report.errors.push(error);
        }
        if !self.player_side.is_empty() && self.player_side().is_none() {
            report
                .errors
                .push(format!("camp du joueur inconnu : {}", self.player_side));
        }
        report.ok = report.errors.is_empty();
        report
    }

    /// The battle setup of a valid composition (the first error of
    /// [`Self::validate`] otherwise). Each side gets a generic captain on
    /// its most expensive regiment (a mounted one first).
    pub fn battle_setup(
        &self,
        data: &CustomData,
        rules: &CustomBattleRules,
        orders: Vec<BattleOrder>,
        standards: Option<BattleStandardRules>,
        abilities: Vec<BattleAbility>,
    ) -> Result<BattleSetup, String> {
        let report = self.validate(data, rules);
        if let Some(error) = report.errors.first() {
            return Err(error.clone());
        }
        let side = |side: SideId| -> Result<SideSetup, String> {
            let composed = self.side(side);
            let faction = FactionId::new(composed.faction.as_str())
                .ok()
                .and_then(|id| data.factions.get(&id))
                .ok_or_else(|| format!("unknown faction {}", composed.faction))?;
            let mut units = Vec::new();
            let mut best: Option<(bool, u32, usize)> = None;
            for (index, id) in composed.units.iter().enumerate() {
                let unit_type = UnitTypeId::new(id.as_str())
                    .ok()
                    .and_then(|id| data.unit_types.get(&id))
                    .ok_or_else(|| format!("unknown unit type {id}"))?;
                let rank = (unit_type.mounted, unit_type.cost.money, usize::MAX - index);
                if best.is_none_or(|b| rank > b) {
                    best = Some(rank);
                }
                units.push(UnitSetup::from_unit_type(
                    unit_type,
                    unit_type.soldiers,
                    unit_type.stats.morale,
                    0,
                ));
            }
            let name = faction_name(faction);
            let general = best.map(|(_, _, rev)| GeneralSetup {
                character: format!("custom_{}", side.key()),
                name: format!("Capitaine ({name})"),
                command: rules.general_command,
                unit_index: usize::MAX - rev,
                morale_bonus: 0.0,
                charge_percent: 0.0,
                ranged_percent: 0.0,
                defense_percent: 0.0,
                sovereign: false,
            });
            Ok(SideSetup {
                faction: faction.id.to_string(),
                faction_name: name,
                army: String::new(),
                units,
                general,
                forced_march: false,
                entrenched: false,
                start_fatigue: 0.0,
            })
        };
        let terrain = self.terrain().unwrap_or(Terrain::Plains);
        Ok(BattleSetup {
            province: "custom".to_owned(),
            province_name: "Bataille personnalisée".to_owned(),
            terrain,
            river: false,
            season: self.season().unwrap_or_default(),
            coastal: false,
            village: None,
            attacker: side(SideId::Attacker)?,
            defender: side(SideId::Defender)?,
            player_side: self.player_side(),
            siege: self.siege.then(|| SiegeSetup {
                fortification: self.fortification(rules),
                breach: 0,
                place: self.place.unwrap_or_default(),
            }),
            siege_layout: None,
            orders,
            abilities,
            standards,
            decor_plan: None,
            opening: Default::default(),
        })
    }
}

impl CustomReport {
    fn side_mut(&mut self, side: SideId) -> &mut CustomSideReport {
        match side {
            SideId::Attacker => &mut self.attacker,
            SideId::Defender => &mut self.defender,
        }
    }
}
