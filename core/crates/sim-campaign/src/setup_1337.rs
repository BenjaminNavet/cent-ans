//! Spring 1337 start derived from `GameData` (spec § 1.6).
//!
//! Garrisons: 4 units in capitals, 3 in ports and border provinces, 2
//! elsewhere. Every faction gets one main army in its capital led by its ruler
//! (8 units for France, 6 for England, 4 for Burgundy, 3 for the others so that
//! the AI can act). Treasuries come from `Faction::treasury`; wars, alliances
//! and truces from `Faction::relations`; vassal/overlord ties count as
//! alliances.

use std::collections::BTreeSet;

use data_model::{
    CharacterId, CharacterStatus, Faction, FactionId, GameData, ProvinceId, RelationStatus,
    UnitTypeId,
};

use crate::orders::status_allows_command;
use crate::save::CampaignError;
use crate::state::{
    Army, CampaignState, CharacterState, FactionState, ProvinceState, Stance, Unit,
};

/// Treasury for factions whose data gives none.
pub const DEFAULT_TREASURY: i64 = 5_000;
/// Duration in turns of the truces active at the start.
pub const INITIAL_TRUCE_TURNS: u32 = 8;

const MILITIA: &str = "unit_urban_militia";
const CROSSBOWMEN: &str = "unit_crossbowmen";
const MEN_AT_ARMS: &str = "unit_men_at_arms_foot";
const KNIGHTS: &str = "unit_knights";
const MOUNTED_SERGEANTS: &str = "unit_mounted_sergeants";
const LONGBOWMEN: &str = "unit_longbowmen";
const MOUNTED_ARCHERS: &str = "unit_mounted_archers";

/// Composition of the main army of `faction` (unit type ids, may repeat).
fn main_army_composition(faction: &Faction) -> Vec<&'static str> {
    match faction.id.as_str() {
        "fac_france" => vec![
            KNIGHTS,
            KNIGHTS,
            KNIGHTS,
            MEN_AT_ARMS,
            MEN_AT_ARMS,
            CROSSBOWMEN,
            CROSSBOWMEN,
            MOUNTED_SERGEANTS,
        ],
        "fac_england" => vec![
            KNIGHTS,
            KNIGHTS,
            MEN_AT_ARMS,
            LONGBOWMEN,
            LONGBOWMEN,
            MOUNTED_ARCHERS,
        ],
        "fac_burgundy" => vec![KNIGHTS, KNIGHTS, MEN_AT_ARMS, CROSSBOWMEN],
        _ => vec![KNIGHTS, MEN_AT_ARMS, CROSSBOWMEN],
    }
}

fn garrison_composition(is_capital: bool, is_port: bool, is_border: bool) -> Vec<&'static str> {
    if is_capital {
        vec![MILITIA, MILITIA, CROSSBOWMEN, MEN_AT_ARMS]
    } else if is_port || is_border {
        vec![MILITIA, MILITIA, CROSSBOWMEN]
    } else {
        vec![MILITIA, MILITIA]
    }
}

fn units_from(data: &GameData, ids: &[&str]) -> Result<Vec<Unit>, CampaignError> {
    ids.iter()
        .map(|raw| {
            let id = UnitTypeId::new(*raw).map_err(CampaignError::MissingData)?;
            data.unit_types
                .get(&id)
                .map(Unit::fresh)
                .ok_or_else(|| CampaignError::MissingData(format!("unit type {raw}")))
        })
        .collect()
}

impl CampaignState {
    /// Builds the spring 1337 campaign for `player` from the game data.
    pub fn new_1337(
        data: &GameData,
        player: FactionId,
        seed: u64,
    ) -> Result<CampaignState, CampaignError> {
        if !data.factions.contains_key(&player) {
            return Err(CampaignError::UnknownFaction(player));
        }
        let mut state = CampaignState::empty(player, seed);

        // Provinces and garrisons.
        for (id, province) in &data.provinces {
            let owner = &province.owner;
            let is_capital = data.factions.get(owner).is_some_and(|f| &f.capital == id);
            let is_border = province
                .neighbors
                .iter()
                .any(|n| data.provinces.get(n).is_some_and(|p| &p.owner != owner));
            let garrison = units_from(
                data,
                &garrison_composition(is_capital, province.has_port(), is_border),
            )?;
            state.provinces.insert(
                id.clone(),
                ProvinceState {
                    owner: owner.clone(),
                    controller: owner.clone(),
                    garrison,
                    siege: None,
                    unrest: province.population.classes.peasants.unrest / 4,
                    devastation: 0,
                    population: province.population.classes.clone(),
                    recruit_queue: Vec::new(),
                },
            );
        }

        // Factions and diplomacy.
        for (id, faction) in &data.factions {
            let mut faction_state = FactionState {
                treasury: faction.treasury.map_or(DEFAULT_TREASURY, |t| t as i64),
                income_last_turn: 0,
                upkeep_last_turn: 0,
                at_war_with: BTreeSet::new(),
                allies: BTreeSet::new(),
                truces: Default::default(),
                alive: true,
                ruler: faction.ruler.clone(),
                heir: faction.heir.clone(),
                capital: faction.capital.clone(),
                technologies: faction.starting_technologies.iter().cloned().collect(),
            };
            if let Some(suzerain) = &faction.suzerain {
                faction_state.allies.insert(suzerain.clone());
            }
            state.factions.insert(id.clone(), faction_state);
        }
        for (id, faction) in &data.factions {
            for relation in &faction.relations {
                let other = &relation.faction;
                if !state.factions.contains_key(other) {
                    continue;
                }
                match relation.status {
                    RelationStatus::War => {
                        state
                            .factions
                            .get_mut(id)
                            .expect("exists")
                            .at_war_with
                            .insert(other.clone());
                        state
                            .factions
                            .get_mut(other)
                            .expect("exists")
                            .at_war_with
                            .insert(id.clone());
                    }
                    RelationStatus::Alliance
                    | RelationStatus::Vassal
                    | RelationStatus::Overlord => {
                        state
                            .factions
                            .get_mut(id)
                            .expect("exists")
                            .allies
                            .insert(other.clone());
                        state
                            .factions
                            .get_mut(other)
                            .expect("exists")
                            .allies
                            .insert(id.clone());
                    }
                    RelationStatus::Truce => {
                        let until = INITIAL_TRUCE_TURNS;
                        state
                            .factions
                            .get_mut(id)
                            .expect("exists")
                            .truces
                            .insert(other.clone(), until);
                        state
                            .factions
                            .get_mut(other)
                            .expect("exists")
                            .truces
                            .insert(id.clone(), until);
                    }
                    RelationStatus::Peace
                    | RelationStatus::Embargo
                    | RelationStatus::MarriageTie => {}
                }
            }
        }
        // A faction never fights its allies at the start.
        let ids: Vec<FactionId> = state.factions.keys().cloned().collect();
        for id in &ids {
            let allies = state.factions[id].allies.clone();
            state
                .factions
                .get_mut(id)
                .expect("exists")
                .at_war_with
                .retain(|f| !allies.contains(f));
        }

        // Characters.
        for (id, character) in &data.characters {
            let location = character
                .starting_location
                .clone()
                .filter(|p| state.provinces.contains_key(p))
                .or_else(|| {
                    data.factions
                        .get(&character.faction)
                        .map(|f| f.capital.clone())
                });
            state.characters.insert(
                id.clone(),
                CharacterState {
                    faction: character.faction.clone(),
                    alive: character
                        .death
                        .as_ref()
                        .and_then(|d| d.year())
                        .is_none_or(|y| y > 1337),
                    birth_year: character.birth.year().unwrap_or(1300),
                    sex: character.sex,
                    house: character.house.clone(),
                    location,
                    army: None,
                    skills: character.skills,
                    captive: character.status == Some(CharacterStatus::Captive),
                },
            );
        }

        // Main armies.
        for (id, faction) in &data.factions {
            if !state.provinces.contains_key(&faction.capital) {
                return Err(CampaignError::MissingData(format!(
                    "capital {} of {id}",
                    faction.capital
                )));
            }
            let units = units_from(data, &main_army_composition(faction))?;
            let army_id = state.allocate_army_id();
            state.armies.insert(
                army_id.clone(),
                Army {
                    faction: id.clone(),
                    general: None,
                    location: faction.capital.clone(),
                    units,
                    movement_points: state.season.movement_points(),
                    supply: 100,
                    stance: Stance::Normal,
                    path: Vec::new(),
                },
            );
            if let Some(general) = pick_general(&state, data, faction) {
                state.attach_general(&army_id, &general);
            }
        }
        Ok(state)
    }
}

/// The ruler when able to command, else the best available commander of the faction.
fn pick_general(state: &CampaignState, data: &GameData, faction: &Faction) -> Option<CharacterId> {
    let commands = |id: &CharacterId| -> bool {
        let Some(character) = data.characters.get(id) else {
            return false;
        };
        let Some(dynamic) = state.characters.get(id) else {
            return false;
        };
        dynamic.alive
            && !dynamic.captive
            && character.faction == faction.id
            && status_allows_command(character.status)
            && character.status != Some(CharacterStatus::InExile)
    };
    if let Some(ruler) = &faction.ruler {
        if commands(ruler) {
            return Some(ruler.clone());
        }
    }
    let capital: &ProvinceId = &faction.capital;
    data.characters
        .iter()
        .filter(|(id, c)| {
            commands(id)
                && state
                    .characters
                    .get(*id)
                    .is_some_and(|d| d.location.as_ref() == Some(capital))
                && c.skills.command > 0
        })
        .max_by_key(|(id, c)| (c.skills.command, std::cmp::Reverse((*id).clone())))
        .map(|(id, _)| id.clone())
}
