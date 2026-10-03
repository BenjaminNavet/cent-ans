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
    BuildingId, CharacterId, CharacterStatus, Faction, FactionId, GameData, HistoricalDate,
    ProvinceId, RelationStatus, UnitTypeId,
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
/// Whether a recorded death falls before the campaign opens in spring 1337:
/// a death later in 1337 (Frederick III of Sicily in June) or dated to the
/// year only leaves the character alive at start.
fn dead_before_start(death: &HistoricalDate) -> bool {
    match death.year() {
        Some(year) if year < 1337 => true,
        Some(1337) => death
            .value
            .get(5..7)
            .and_then(|month| month.parse::<u32>().ok())
            .is_some_and(|month| month < 3),
        _ => false,
    }
}

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
        // A6-L3 (ADR 0179): the duchy's host cost 68 % of its receipts.
        "fac_burgundy" => vec![KNIGHTS, MEN_AT_ARMS, CROSSBOWMEN],
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

/// Balance of the season `faction` would pay from the start, without the
/// idle hoard's share of the court (it melts with the treasury), and its
/// receipts.
fn structural_balance(state: &CampaignState, data: &GameData, faction: &FactionId) -> (i64, i64) {
    let Some(economy) = state.faction_economy(data, faction) else {
        return (0, 0);
    };
    let rules = &data.economy_rules;
    let income = state.faction_income_effective(data, faction);
    let treasury = state.factions.get(faction).map_or(0, |f| f.treasury);
    let opulence =
        (treasury - rules.opulence_seasons * income.max(0)).max(0) * rules.opulence_percent / 100;
    (
        economy.net_income() + opulence,
        economy.projected_income + economy.trade_income,
    )
}

/// Lot JR4b (`settlement_rules.starting_budget`): a great realm whose
/// starting forces outrun its receipts sends home its costliest garrison
/// units, one at a time, until the deficit is within the allowed share —
/// never a settlement's last unit nor the capital's garrison. (The Mamluks
/// paid 4 000 livres of upkeep on 4 000 of receipts and their AI dismissed
/// its whole field army by the eighth season.)
fn fit_starting_garrisons(state: &mut CampaignState, data: &GameData) {
    let Some(rule) = data
        .settlement_rules
        .as_ref()
        .and_then(|r| r.starting_budget.clone())
    else {
        return;
    };
    // JR5: the reference balance ignores the difficulty (whatever level the
    // campaign is or will be set to): measured at the neutral level, where
    // neither the AI nor the player gets any favour.
    let level = state.difficulty;
    state.difficulty = crate::difficulty::Difficulty::Normal;
    let factions: Vec<FactionId> = state.factions.keys().cloned().collect();
    for faction in factions {
        if state.controlled_provinces(&faction).len() < rule.min_provinces
            || crate::crusade::starting_army(state, data, &faction).is_some()
        {
            continue;
        }
        // A6-L3 (ADR 0179): the treasury is capped at a few seasons of income.
        if let Some(seasons) = rule.treasury_max_income_seasons {
            let (_, receipts) = structural_balance(state, data, &faction);
            if let Some(f) = state.factions.get_mut(&faction) {
                f.treasury = f.treasury.min(seasons * receipts.max(0));
            }
        }
        let capital = state.faction_capital_city(&faction).cloned();
        loop {
            let (net, receipts) = structural_balance(state, data, &faction);
            if net * 100 >= -rule.max_deficit_percent * receipts.max(0) {
                break;
            }
            let costliest = state
                .settlements
                .iter()
                .filter(|(id, s)| {
                    s.controller == faction && s.garrison.len() > 1 && Some(*id) != capital.as_ref()
                })
                .flat_map(|(id, s)| {
                    let percent = crate::economy::garrison_upkeep_percent(data, s.kind);
                    s.garrison.iter().enumerate().map(move |(index, unit)| {
                        (
                            crate::economy::unit_upkeep(data, unit) * percent,
                            id.clone(),
                            index,
                        )
                    })
                })
                .filter(|(paid, _, _)| *paid > 0)
                .max_by(|a, b| {
                    a.0.cmp(&b.0)
                        .then_with(|| b.1.cmp(&a.1))
                        .then_with(|| b.2.cmp(&a.2))
                });
            let Some((_, settlement, index)) = costliest else {
                // The fallbacks below only bring a deficit to zero; the
                // surplus asked of the others stops at their garrisons.
                if net >= 0 {
                    break;
                }
                // A6-L3 (ADR 0179): no garrison left to trim, the starting
                // host sends home its costliest unit (never an army's last).
                let costliest_unit = state
                    .armies
                    .iter()
                    .filter(|(_, a)| a.faction == faction && a.units.len() > 1)
                    .flat_map(|(id, a)| {
                        a.units.iter().enumerate().map(move |(index, unit)| {
                            (crate::economy::unit_upkeep(data, unit), id.clone(), index)
                        })
                    })
                    .filter(|(paid, _, _)| *paid > 0)
                    .max_by(|a, b| {
                        a.0.cmp(&b.0)
                            .then_with(|| b.1.cmp(&a.1))
                            .then_with(|| b.2.cmp(&a.2))
                    });
                let Some((_, army, index)) = costliest_unit else {
                    // Last resort: the capital's garrison keeps one unit.
                    let costliest_guard = capital.as_ref().and_then(|id| {
                        let place = state.settlements.get(id)?;
                        let percent = crate::economy::garrison_upkeep_percent(data, place.kind);
                        place
                            .garrison
                            .iter()
                            .enumerate()
                            .filter(|_| place.garrison.len() > 1)
                            .map(|(index, unit)| {
                                (crate::economy::unit_upkeep(data, unit) * percent, index)
                            })
                            .filter(|(paid, _)| *paid > 0)
                            .max_by(|a, b| a.0.cmp(&b.0).then_with(|| b.1.cmp(&a.1)))
                            .map(|(_, index)| index)
                    });
                    if let (Some(index), Some(id)) = (costliest_guard, capital.as_ref()) {
                        if let Some(place) = state.settlements.get_mut(id) {
                            place.garrison.remove(index);
                        }
                        continue;
                    }
                    // Then the dearest building left unpaid for (a realm
                    // whose upkeep alone outruns its receipts).
                    let dearest = state
                        .settlements
                        .iter()
                        .filter(|(_, s)| s.controller == faction)
                        .flat_map(|(id, s)| {
                            let percent = crate::economy::building_upkeep_percent(data, s.kind);
                            s.buildings.iter().enumerate().map(move |(index, b)| {
                                let upkeep = data
                                    .buildings
                                    .get(b)
                                    .map_or(0, |t| i64::from(t.upkeep.unwrap_or(0)));
                                (upkeep * percent, id.clone(), index)
                            })
                        })
                        .filter(|(paid, _, _)| *paid > 0)
                        .max_by(|a, b| {
                            a.0.cmp(&b.0)
                                .then_with(|| b.1.cmp(&a.1))
                                .then_with(|| b.2.cmp(&a.2))
                        });
                    let Some((_, id, index)) = dearest else {
                        break;
                    };
                    if let Some(place) = state.settlements.get_mut(&id) {
                        place.buildings.remove(index);
                    }
                    continue;
                };
                if let Some(a) = state.armies.get_mut(&army) {
                    a.units.remove(index);
                }
                continue;
            };
            if let Some(place) = state.settlements.get_mut(&settlement) {
                place.garrison.remove(index);
            }
        }
    }
    state.difficulty = level;
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
                build_queue: Vec::new(),
                recruit_queue: Vec::new(),
                fortification_level: settlement.fortification_level,
                recruit_pool: Default::default(),
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
            let faction_state = FactionState {
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
                deficit_seasons: 0,
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
                // Lot FE: a view of the title holdings (ADR 0098).
                suzerain: crate::feudal::liege_of(&state, data, id),
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
                research_queue: Vec::new(),
            };
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
                    alive: !character.death.as_ref().is_some_and(dead_before_start),
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
                        .filter(|d| dead_before_start(d))
                        .and_then(|d| d.year()),
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
            // JR1: a faction the crusade rules base in a settlement starts
            // there with the army they list (it holds no city).
            let (station, units) = match crate::crusade::starting_army(&state, data, id) {
                Some(start) => start,
                None => (
                    capital_city,
                    units_from(data, &main_army_composition(faction))?,
                ),
            };
            let army_id = state.allocate_army_id();
            state.armies.insert(
                army_id.clone(),
                Army::new(
                    id.clone(),
                    crate::state::ArmyPosition::Settlement(station),
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
        // JR4b: the great realms start with garrisons they can pay.
        fit_starting_garrisons(&mut state, data);
        // JR1: the crusade opens when its rules and its faction exist.
        crate::crusade::init_crusade(&mut state, data);
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
        // ADR 0114: no alliance between a suzerain and its direct vassal.
        crate::feudal::drop_feudal_alliances(&mut state, data);
        // F8: the felony cases of 1337 (Robert of Artois harboured by Edward III).
        for case in &data.feudal_rules.start_felonies {
            crate::feudal::open_felony_towards(
                &mut state,
                data,
                &case.vassal,
                &case.liege,
                case.reason,
            );
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

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::difficulty::Difficulty;

    #[test]
    fn the_starting_garrisons_do_not_depend_on_the_difficulty() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../data");
        let data = GameData::load(&root).expect("game data loads").0;
        let mut raw_data = data.clone();
        raw_data
            .settlement_rules
            .as_mut()
            .expect("settlement rules")
            .starting_budget = None;
        let player = FactionId::new("fac_france").unwrap();
        let garrisons = |level: Difficulty| {
            let mut state = CampaignState::new_1337(&raw_data, player.clone(), 1).unwrap();
            state.difficulty = level;
            fit_starting_garrisons(&mut state, &data);
            assert_eq!(state.difficulty, level, "the level is restored");
            state
                .settlements
                .iter()
                .map(|(id, s)| (id.clone(), s.garrison.len()))
                .collect::<Vec<_>>()
        };
        let normal = garrisons(Difficulty::Normal);
        assert_eq!(garrisons(Difficulty::Easy), normal);
        assert_eq!(garrisons(Difficulty::VeryHard), normal);
        // And it did trim something.
        let untouched = CampaignState::new_1337(&raw_data, player.clone(), 1).unwrap();
        let raw: Vec<_> = untouched
            .settlements
            .iter()
            .map(|(id, s)| (id.clone(), s.garrison.len()))
            .collect();
        assert_ne!(raw, normal);
    }
}
