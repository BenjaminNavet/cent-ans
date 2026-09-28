//! Spring 1337 start derived from `GameData` (spec § 1.6).
//!
//! Garrisons (lot C4: held by the city of the province): 4 units in
//! capitals, 3 in frontier provinces (ports and provinces next to another
//! faction, [`crate::frontier`]), 2 elsewhere; the other settlements
//! receive the `starting_garrison` of their kind
//! (`data/settlements/rules.json`). Every faction gets one main army in its
//! capital led by its ruler (8 units for France, 6 for England, 4 for
//! Burgundy, 3 for the others so that
//! the AI can act). Treasuries come from `Faction::treasury`; wars, alliances
//! and truces from `Faction::relations`; vassal/overlord ties count as
//! alliances.

use std::collections::BTreeSet;

use data_model::{
    BuildingId, CharacterId, CharacterStatus, Faction, FactionId, GameData, ProvinceId,
    RelationStatus, UnitTypeId,
};

use crate::diplomacy::{Claim, FOREVER};
use crate::economy::TaxRate;
use crate::frontier::GarrisonRole;
use crate::orders::status_allows_command;
use crate::save::CampaignError;
use crate::state::{
    Army, CampaignState, CharacterState, FactionState, ProvinceState, SettlementState, Unit,
};

/// The virtual, non-playable faction provinces fall to on outright revolt
/// (spec § 1.1); it never gets a starting army or garrison.
pub const REBELS_FACTION: &str = "fac_rebels";

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

/// Units of a starting garrison; its size is [`GarrisonRole::garrison_size`].
fn garrison_composition(role: GarrisonRole) -> Vec<&'static str> {
    match role {
        GarrisonRole::Capital => vec![MILITIA, MILITIA, CROSSBOWMEN, MEN_AT_ARMS],
        GarrisonRole::Frontier => vec![MILITIA, MILITIA, CROSSBOWMEN],
        GarrisonRole::Interior => vec![MILITIA, MILITIA],
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

/// One [`SettlementState`] per settlement of a known province (lots C1, C4).
///
/// Owner is the settlement's enclave owner or the province owner; the
/// controller is the owner. The city receives the province's starting
/// buildings (its garrison is set by the caller, P1); the other
/// settlements the `starting_garrison` of their kind from
/// `data/settlements/rules.json` (none when the file is absent).
fn init_settlements(state: &mut CampaignState, data: &GameData) -> Result<(), CampaignError> {
    for (id, settlement) in &data.settlements {
        let Some(province) = data.provinces.get(&settlement.province) else {
            continue;
        };
        if !state.provinces.contains_key(&settlement.province) {
            continue;
        }
        let owner = settlement
            .owner
            .clone()
            .unwrap_or_else(|| province.owner.clone());
        let is_city = state.provinces[&settlement.province].city == *id;
        let garrison = match (&data.settlement_rules, is_city) {
            // The city garrison depends on the province's role (P1),
            // assigned once every settlement exists.
            (_, true) => Vec::new(),
            (None, false) => Vec::new(),
            (Some(rules), false) => {
                let ids: Vec<&str> = rules
                    .starting_garrison
                    .get(&settlement.kind)
                    .map(|units| units.iter().map(|u| u.as_str()).collect())
                    .unwrap_or_default();
                units_from(data, &ids)?
            }
        };
        let mut buildings: Vec<BuildingId> = Vec::new();
        let starting = if is_city {
            province.buildings.as_slice()
        } else {
            &[]
        };
        for building in starting.iter().chain(settlement.buildings.iter()) {
            if !buildings.contains(building) {
                buildings.push(building.clone());
            }
        }
        // EQ2: one step per upgrade chain (the data once stacked tiers).
        let buildings = data.normalize_building_tiers(&buildings);
        state.settlements.insert(
            id.clone(),
            SettlementState {
                province: settlement.province.clone(),
                kind: settlement.kind,
                controller: owner.clone(),
                owner,
                garrison,
                siege: None,
                buildings,
                construction: None,
                recruit_queue: Vec::new(),
                fortification_level: settlement.fortification_level,
            },
        );
    }
    Ok(())
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
        state.feudal = crate::feudal::FeudalState::from_data(data);

        // Provinces, then their settlements, then the city garrisons (P1:
        // frontiers classified by `CampaignState::is_frontier`, like the AI
        // does; lot C4: control is derived from the cities, so the
        // settlements must exist first).
        for (id, province) in &data.provinces {
            let Some(city) = data.province_city(id) else {
                return Err(CampaignError::MissingData(format!("city of {id}")));
            };
            state.provinces.insert(
                id.clone(),
                ProvinceState {
                    city: city.id.clone(),
                    settlements: data
                        .settlements_by_province
                        .get(id)
                        .cloned()
                        .unwrap_or_else(|| vec![city.id.clone()]),
                    unrest: province.population.classes.peasants.unrest / 4,
                    devastation: 0,
                    population: province.population.classes.clone(),
                    revolt_seasons: 0,
                    heresy: 0,
                    heresy_religion: None,
                    diet: None,
                    edict: None,
                },
            );
        }
        init_settlements(&mut state, data)?;
        let mut city_garrisons = std::collections::BTreeMap::new();
        for (id, province) in &data.provinces {
            let owner = &province.owner;
            let role = if data.factions.get(owner).is_some_and(|f| &f.capital == id) {
                GarrisonRole::Capital
            } else if state.is_frontier(data, owner, id) {
                GarrisonRole::Frontier
            } else {
                GarrisonRole::Interior
            };
            let garrison = units_from(data, &garrison_composition(role))?;
            city_garrisons.insert(id.clone(), garrison);
        }
        for (id, garrison) in city_garrisons {
            if let Some(city) = state.city_state_mut(&id) {
                city.garrison = garrison;
            }
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
                tax_rate: TaxRate::Normal,
                goods: Default::default(),
                army_upkeep_last_turn: 0,
                building_upkeep_last_turn: 0,
                projected_income: 0,
                table_upkeep_last_turn: 0,
                coinage: Default::default(),
                price_level: crate::coinage::PRICE_BASE,
                coinage_changed_year: None,
                seigniorage_last_turn: 0,
                recoinage_last_turn: 0,
                ransom_debts: Vec::new(),
                chivalric_order: None,
                trade_income_last_turn: 0,
                budget_history: Vec::new(),
                ledger: Default::default(),
                regency: false,
                embargoes: BTreeSet::new(),
                suzerain: faction.suzerain.clone(),
                loyalty: 100,
                claims: faction
                    .claims
                    .iter()
                    .map(|claim| Claim {
                        kind: claim.kind,
                        faction: claim.faction.clone(),
                        province: claim.province.clone(),
                        text_fr: claim
                            .note
                            .clone()
                            .unwrap_or_else(|| "prétention historique".to_owned()),
                        expires_turn: None,
                    })
                    .collect(),
                modifiers: Vec::new(),
                war_scores: Default::default(),
                war_started: Default::default(),
                religion: Some(faction.religion.clone()),
                papal_favor: 50,
                excommunicated_until: None,
                offers: Vec::new(),
                last_offer_turn: Default::default(),
                last_war_declared: None,
                research: None,
                research_progress: 0,
                research_points_last_turn: 0,
                research_banked: Default::default(),
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
                    RelationStatus::Embargo => {
                        state
                            .factions
                            .get_mut(id)
                            .expect("exists")
                            .embargoes
                            .insert(other.clone());
                    }
                    RelationStatus::MarriageTie => {
                        state.add_modifier(id, other, 15, "Alliance matrimoniale", FOREVER);
                    }
                    RelationStatus::Peace => {}
                }
            }
        }
        for faction in state.factions.values_mut() {
            let enemies: Vec<FactionId> = faction.at_war_with.iter().cloned().collect();
            for enemy in enemies {
                faction.war_started.insert(enemy.clone(), 0);
                faction.war_scores.insert(enemy, 0);
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

        // Characters. Historical characters not yet born in 1337 stay out of
        // the state: `dynasty::resolve_births` spawns them at their real
        // date if their parents are alive and married (spec M4 § 2).
        for (id, character) in &data.characters {
            if character.status == Some(CharacterStatus::Unborn) {
                continue;
            }
            let location = character
                .starting_location
                .clone()
                .filter(|p| state.provinces.contains_key(p))
                .or_else(|| {
                    data.factions
                        .get(&character.faction)
                        .map(|f| f.capital.clone())
                });
            let family = character.family.as_ref();
            state.characters.insert(
                id.clone(),
                CharacterState {
                    name: None,
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
                    captor: None,
                    ransom_terms: None,
                    experience: 0,
                    skill_points: 0,
                    skills_learned: BTreeSet::new(),
                    traits: character
                        .traits
                        .iter()
                        .filter(|t| data.traits.contains_key(*t))
                        .cloned()
                        .collect(),
                    spouse: None,
                    children: Vec::new(),
                    father: family.and_then(|f| f.father.clone()),
                    mother: family.and_then(|f| f.mother.clone()),
                    piety: character.piety.unwrap_or(50).min(100),
                    prestige: 0,
                    loyalty: 100,
                    title: character
                        .titles
                        .iter()
                        .find(|t| t.to.is_none())
                        .or_else(|| character.titles.first())
                        .map(|t| t.title.clone()),
                    governor_of: None,
                    battles_fought: 0,
                    sieges_won: 0,
                    raids_led: 0,
                    death_year: character
                        .death
                        .as_ref()
                        .and_then(|d| d.year())
                        .filter(|y| *y <= 1337),
                    retinue: Vec::new(),
                },
            );
        }
        link_families(&mut state, data);
        // Republics whose data names no ruler (Florence, the Confederates)
        // start with an elected head, like a realm whose dynasty died out.
        let rulerless: Vec<FactionId> = state
            .factions
            .iter()
            .filter(|(id, f)| id.as_str() != REBELS_FACTION && f.ruler.is_none())
            .map(|(id, _)| id.clone())
            .collect();
        for id in rulerless {
            let ruler = crate::dynasty::spawn_ruler(&mut state, data, &id);
            state.factions.get_mut(&id).expect("exists").ruler = Some(ruler);
        }
        // Factions whose data names no heir get the one their succession law
        // designates (spec M4 § 1: "héritier calculable").
        let ids: Vec<FactionId> = state.factions.keys().cloned().collect();
        for id in ids {
            let faction_state = &state.factions[&id];
            let heir_alive = faction_state
                .heir
                .as_ref()
                .is_some_and(|h| state.characters.get(h).is_some_and(|c| c.alive));
            if heir_alive {
                continue;
            }
            let heir = faction_state
                .ruler
                .clone()
                .and_then(|ruler| crate::dynasty::pick_heir_by_law(&state, data, &id, &ruler));
            state.factions.get_mut(&id).expect("exists").heir = heir;
        }

        // Main armies (the virtual rebels faction owns no province and never
        // fields troops of its own; it only ever inherits a garrison already
        // weakened by the revolt that hands it a province, spec § 1.1).
        for (id, faction) in &data.factions {
            if id.as_str() == REBELS_FACTION {
                continue;
            }
            let Some(capital_city) = state.province_city_id(&faction.capital).cloned() else {
                return Err(CampaignError::MissingData(format!(
                    "capital {} of {id}",
                    faction.capital
                )));
            };
            let units = units_from(data, &main_army_composition(faction))?;
            let army_id = state.allocate_army_id();
            state.armies.insert(
                army_id.clone(),
                Army::new(
                    id.clone(),
                    crate::state::ArmyPosition::Settlement(capital_city),
                    units,
                ),
            );
            if let Some(general) = pick_general(&state, data, faction) {
                state.attach_general(&army_id, &general);
            }
            let allowance = state.army_grid_allowance(data, &state.armies[&army_id]);
            state
                .armies
                .get_mut(&army_id)
                .expect("just created")
                .movement_left = allowance;
        }
        crate::economy::resolve_goods(&mut state, data);
        // Vassal loyalty starts at its equilibrium (M5).
        let vassals: Vec<(FactionId, FactionId)> = state
            .factions
            .iter()
            .filter_map(|(id, f)| f.suzerain.clone().map(|s| (id.clone(), s)))
            .collect();
        for (vassal, suzerain) in vassals {
            let loyalty = crate::diplomacy::loyalty_target(&state, data, &vassal, &suzerain);
            state.factions.get_mut(&vassal).expect("exists").loyalty = loyalty;
        }
        Ok(state)
    }
}

/// Resolves family links against the characters actually present in 1337:
/// parents and children must exist in the state, and a spouse is kept only
/// when alive and when the link is mutual (first living spouse listed).
fn link_families(state: &mut CampaignState, data: &GameData) {
    let ids: Vec<CharacterId> = state.characters.keys().cloned().collect();
    for id in &ids {
        let family = data.characters.get(id).and_then(|c| c.family.as_ref());
        let is_alive = |other: &CharacterId| state.characters.get(other).is_some_and(|c| c.alive);
        let spouse = family.and_then(|f| f.spouses.iter().find(|s| is_alive(s)).cloned());
        let character = &state.characters[id];
        let father = character
            .father
            .clone()
            .filter(|f| state.characters.contains_key(f));
        let mother = character
            .mother
            .clone()
            .filter(|m| state.characters.contains_key(m));
        let character = state.characters.get_mut(id).expect("exists");
        character.spouse = if character.alive { spouse } else { None };
        character.father = father;
        character.mother = mother;
    }
    // Spouses must point at each other; children derive from parent links so
    // both directions agree even when the data lists only one side.
    for id in &ids {
        let spouse = state.characters[id].spouse.clone();
        if let Some(spouse) = spouse {
            let mutual = state.characters[&spouse].spouse.as_ref() == Some(id);
            if !mutual {
                let spouse_state = state.characters.get_mut(&spouse).expect("exists");
                if spouse_state.spouse.is_none() {
                    spouse_state.spouse = Some(id.clone());
                } else {
                    state.characters.get_mut(id).expect("exists").spouse = None;
                }
            }
        }
    }
    for id in &ids {
        let (father, mother) = {
            let c = &state.characters[id];
            (c.father.clone(), c.mother.clone())
        };
        for parent in [father, mother].into_iter().flatten() {
            let parent_state = state.characters.get_mut(&parent).expect("exists");
            if !parent_state.children.contains(id) {
                parent_state.children.push(id.clone());
            }
        }
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
