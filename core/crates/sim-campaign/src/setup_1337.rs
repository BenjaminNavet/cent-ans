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

use data_model::{
    BuildingId, CharacterId, CharacterStatus, Faction, FactionId, GameData, HistoricalDate,
    ProvinceId, RelationStatus, UnitTypeId,
};

use crate::diplomacy::{Claim, FOREVER};
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

/// Composition of the main army of `faction` (unit type ids, may repeat),
/// from `data/rules/starting_armies.json` (A6-L3b): its own entry, or the
/// default one; empty when the file is absent.
fn main_army_composition<'a>(data: &'a GameData, faction: &Faction) -> Vec<&'a str> {
    data.starting_armies
        .as_ref()
        .map(|armies| {
            armies
                .army_of(&faction.id)
                .iter()
                .map(|u| u.as_str())
                .collect()
        })
        .unwrap_or_default()
}

/// Units of a starting garrison, from `data/rules/starting_armies.json`;
/// its size is [`GarrisonRole::garrison_size`].
fn garrison_composition(data: &GameData, role: GarrisonRole) -> Vec<&str> {
    let Some(armies) = data.starting_armies.as_ref() else {
        return Vec::new();
    };
    let units = match role {
        GarrisonRole::Capital => &armies.garrisons.capital,
        GarrisonRole::Frontier => &armies.garrisons.frontier,
        GarrisonRole::Interior => &armies.garrisons.interior,
    };
    units.iter().map(|u| u.as_str()).collect()
}

/// Balance of the season `faction` would pay from the start, without the
/// idle hoard's share of the court (it melts with the treasury), and its
/// receipts.
fn structural_balance(state: &CampaignState, data: &GameData, faction: &FactionId) -> (i64, i64) {
    let Some(economy) = state.faction_economy(data, faction) else {
        return (0, 0);
    };
    let rules = &data.economy_rules;
    let income = state.faction_income(data, faction);
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
/// never a settlement's last unit nor the capital's garrison; then (LR-15)
/// field units down to `min_field_units`, and last the capital's garrison
/// down to `min_capital_units`. (The Mamluks paid 4 000 livres of upkeep on
/// 4 000 of receipts and their AI dismissed its whole field army by the
/// eighth season.)
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
        if state.controlled_provinces(&faction).count() < rule.min_provinces
            || crate::crusade::starting_army(state, data, &faction).is_some()
        {
            continue;
        }
        // A6-L3 (ADR 0183): the treasury is capped at a few seasons of income.
        if let Some(seasons) = rule.treasury_max_income_seasons {
            let (_, receipts) = structural_balance(state, data, &faction);
            if let Some(f) = state.factions.get_mut(&faction) {
                f.treasury = f.treasury.min(seasons * receipts.max(0));
            }
        }
        let capital = state.faction_capital_city(&faction).cloned();
        let is_capital = |id: &data_model::SettlementId| Some(id) == capital.as_ref();
        // 1. Garrisons other than the capital's, down to one unit each.
        send_home_garrisons(state, data, &faction, &rule, false, |id| {
            if is_capital(id) {
                usize::MAX
            } else {
                1
            }
        });
        // 2. LR-15: the starting field armies.
        if let Some(min_units) = rule.min_field_units {
            fit_starting_field_armies(state, data, &faction, min_units);
        }
        // 3. LR-15: the capital's garrison, as a last resort.
        if let Some(min_units) = rule.min_capital_units {
            send_home_garrisons(state, data, &faction, &rule, true, |id| {
                if is_capital(id) {
                    min_units
                } else {
                    usize::MAX
                }
            });
        }
        // 4. A6-L3: then the dearest building left unpaid for (a realm whose
        // upkeep alone outruns its receipts).
        while in_deficit(state, data, &faction) {
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
        }
    }
    state.difficulty = level;
}

/// `true` while `faction` is beyond the deficit `rule` allows (A6-L3: the
/// share may be negative, a surplus is then asked).
fn over_budget(
    state: &CampaignState,
    data: &GameData,
    faction: &FactionId,
    rule: &data_model::StartingBudget,
) -> bool {
    let (net, receipts) = structural_balance(state, data, faction);
    net * 100 < -rule.max_deficit_percent * receipts.max(0)
}

/// `true` while `faction` is still in deficit. The fallbacks below (field
/// armies, the capital's garrison, buildings) only bring a deficit to zero:
/// the surplus asked of the others stops at their garrisons.
fn in_deficit(state: &CampaignState, data: &GameData, faction: &FactionId) -> bool {
    structural_balance(state, data, faction).0 < 0
}

/// Sends home the costliest garrison units of `faction`, one at a time,
/// while it is over budget; a place keeps at least `keep(place)` units.
fn send_home_garrisons(
    state: &mut CampaignState,
    data: &GameData,
    faction: &FactionId,
    rule: &data_model::StartingBudget,
    fallback: bool,
    keep: impl Fn(&data_model::SettlementId) -> usize,
) {
    while if fallback {
        in_deficit(state, data, faction)
    } else {
        over_budget(state, data, faction, rule)
    } {
        let costliest = state
            .settlements
            .iter()
            .filter(|(id, s)| &s.controller == faction && s.garrison.len() > keep(id))
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
            break;
        };
        if let Some(place) = state.settlements.get_mut(&settlement) {
            place.garrison.remove(index);
        }
    }
}

/// Lot LR-15: the second step of [`fit_starting_garrisons`] — with its
/// garrisons at the floor, a realm still beyond the allowed deficit sends
/// home the costliest units of its starting field armies, keeping at least
/// `min_units` of them and never emptying an army. (Serbia and Lithuania
/// paid 1 080 livres for the three coded units on ~1 800 of receipts; their
/// AI dismissed them all within five seasons.)
fn fit_starting_field_armies(
    state: &mut CampaignState,
    data: &GameData,
    faction: &FactionId,
    min_units: usize,
) {
    while in_deficit(state, data, faction) {
        let units: usize = state
            .armies
            .values()
            .filter(|a| &a.faction == faction)
            .map(|a| a.units.len())
            .sum();
        if units <= min_units {
            break;
        }
        let costliest = state
            .armies
            .iter()
            .filter(|(_, a)| &a.faction == faction && a.units.len() > 1)
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
        let Some((_, army, index)) = costliest else {
            break;
        };
        if let Some(army) = state.armies.get_mut(&army) {
            army.units.remove(index);
        }
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
        init_provinces(&mut state, data)?;
        init_settlements(&mut state, data)?;
        init_city_garrisons(&mut state, data)?;

        init_factions(&mut state, data);
        init_relations(&mut state, data);
        init_wars(&mut state);

        init_characters(&mut state, data);
        link_families(&mut state, data);
        init_rulers_and_heirs(&mut state, data);

        init_main_armies(&mut state, data)?;
        crate::economy::resolve_goods(&mut state, data);
        // JR4b: the great realms start with garrisons they can pay.
        fit_starting_garrisons(&mut state, data);
        // JR1: the crusade opens when its rules and its faction exist.
        crate::crusade::init_crusade(&mut state, data);
        init_feudal_ties(&mut state, data);
        Ok(state)
    }
}

fn init_provinces(state: &mut CampaignState, data: &GameData) -> Result<(), CampaignError> {
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
    Ok(())
}

/// The city of every province holds the garrison of its role (capital,
/// frontier, interior).
fn init_city_garrisons(state: &mut CampaignState, data: &GameData) -> Result<(), CampaignError> {
    for (id, province) in &data.provinces {
        let owner = &province.owner;
        let role = if data.factions.get(owner).is_some_and(|f| &f.capital == id) {
            GarrisonRole::Capital
        } else if state.is_frontier(data, owner, id) {
            GarrisonRole::Frontier
        } else {
            GarrisonRole::Interior
        };
        let garrison = units_from(data, &garrison_composition(data, role))?;
        if let Some(city) = state.city_state_mut(id) {
            city.garrison = garrison;
        }
    }
    Ok(())
}

fn init_factions(state: &mut CampaignState, data: &GameData) {
    for (id, faction) in &data.factions {
        let faction_state = FactionState {
            treasury: faction.treasury.map_or(DEFAULT_TREASURY, |t| t as i64),
            ruler: faction.ruler.clone(),
            heir: faction.heir.clone(),
            technologies: faction.starting_technologies.iter().cloned().collect(),
            // Lot FE: a view of the title holdings (ADR 0098).
            suzerain: crate::feudal::liege_of(state, data, id),
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
            religion: Some(faction.religion.clone()),
            ..FactionState::new(faction.capital.clone())
        };
        state.factions.insert(id.clone(), faction_state);
    }
}

/// Applies `edge(a_state, b)` and `edge(b_state, a)`: a symmetric link
/// between two factions of the state.
fn link(
    state: &mut CampaignState,
    a: &FactionId,
    b: &FactionId,
    edge: impl Fn(&mut FactionState, &FactionId),
) {
    for (from, to) in [(a, b), (b, a)] {
        edge(state.factions.get_mut(from).expect("exists"), to);
    }
}

/// Wars, alliances, truces, embargoes and marriage ties of
/// `Faction::relations` (vassal and overlord ties count as alliances).
fn init_relations(state: &mut CampaignState, data: &GameData) {
    for (id, faction) in &data.factions {
        for relation in &faction.relations {
            let other = &relation.faction;
            if !state.factions.contains_key(other) {
                continue;
            }
            match relation.status {
                RelationStatus::War => link(state, id, other, |f, to| {
                    f.at_war_with.insert(to.clone());
                }),
                RelationStatus::Alliance | RelationStatus::Vassal | RelationStatus::Overlord => {
                    link(state, id, other, |f, to| {
                        f.allies.insert(to.clone());
                    });
                }
                RelationStatus::Truce => link(state, id, other, |f, to| {
                    f.truces.insert(to.clone(), INITIAL_TRUCE_TURNS);
                }),
                RelationStatus::Embargo => {
                    let own = state.factions.get_mut(id).expect("exists");
                    own.embargoes.insert(other.clone());
                }
                RelationStatus::MarriageTie => {
                    state.add_modifier(id, other, 15, "Alliance matrimoniale", FOREVER);
                }
                RelationStatus::Peace => {}
            }
        }
    }
}

/// Opens the war clock of every starting war, then drops the wars between
/// allies: a faction never fights its allies at the start.
fn init_wars(state: &mut CampaignState) {
    for faction in state.factions.values_mut() {
        for enemy in faction.at_war_with.clone() {
            faction.war_started.insert(enemy.clone(), 0);
            faction.war_scores.insert(enemy, 0);
        }
        let allies = faction.allies.clone();
        faction.at_war_with.retain(|f| !allies.contains(f));
    }
}

/// Characters. Historical characters not yet born in 1337 stay out of the
/// state: `dynasty::resolve_births` spawns them at their real date if their
/// parents are alive and married (spec M4 § 2).
fn init_characters(state: &mut CampaignState, data: &GameData) {
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
        let died_before_start = character.death.as_ref().filter(|d| dead_before_start(d));
        state.characters.insert(
            id.clone(),
            CharacterState {
                alive: died_before_start.is_none(),
                location,
                captive: character.status == Some(CharacterStatus::Captive),
                traits: character
                    .traits
                    .iter()
                    .filter(|t| data.traits.contains_key(*t))
                    .cloned()
                    .collect(),
                father: family.and_then(|f| f.father.clone()),
                mother: family.and_then(|f| f.mother.clone()),
                piety: character.piety.unwrap_or(50).min(100),
                title: character
                    .titles
                    .iter()
                    .find(|t| t.to.is_none())
                    .or_else(|| character.titles.first())
                    .map(|t| t.title.clone()),
                death_year: died_before_start.and_then(|d| d.year()),
                ..CharacterState::new(
                    character.faction.clone(),
                    character.birth.year().unwrap_or(1300),
                    character.sex,
                    character.house.clone(),
                    character.skills,
                )
            },
        );
    }
}

/// Republics whose data names no ruler (Florence, the Confederates) start
/// with an elected head, like a realm whose dynasty died out; factions whose
/// data names no heir get the one their succession law designates (spec M4
/// § 1: "héritier calculable").
fn init_rulers_and_heirs(state: &mut CampaignState, data: &GameData) {
    let rulerless: Vec<FactionId> = state
        .factions
        .iter()
        .filter(|(id, f)| id.as_str() != REBELS_FACTION && f.ruler.is_none())
        .map(|(id, _)| id.clone())
        .collect();
    for id in rulerless {
        let ruler = crate::dynasty::spawn_ruler(state, data, &id);
        state.factions.get_mut(&id).expect("exists").ruler = Some(ruler);
    }
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
            .and_then(|ruler| crate::dynasty::pick_heir_by_law(state, data, &id, &ruler));
        state.factions.get_mut(&id).expect("exists").heir = heir;
    }
}

/// Main armies (the virtual rebels faction owns no province and never
/// fields troops of its own; it only ever inherits a garrison already
/// weakened by the revolt that hands it a province, spec § 1.1).
fn init_main_armies(state: &mut CampaignState, data: &GameData) -> Result<(), CampaignError> {
    for (id, faction) in &data.factions {
        if id.as_str() == REBELS_FACTION {
            continue;
        }
        // JR1: a faction the crusade rules base in a settlement starts
        // there with the army they list (it holds no city).
        let (station, units) = match crate::crusade::starting_army(state, data, id) {
            Some(start) => start,
            None => {
                // LR-15: its seat (never another realm's city), else,
                // when it holds no place at all, its capital's city.
                let Some(station) = state
                    .faction_seat(id)
                    .or_else(|| state.province_city_id(&faction.capital).cloned())
                else {
                    return Err(CampaignError::MissingData(format!(
                        "capital {} of {id}",
                        faction.capital
                    )));
                };
                (
                    station,
                    units_from(data, &main_army_composition(data, faction))?,
                )
            }
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
        if let Some(general) = pick_general(state, data, faction) {
            state.attach_general(&army_id, &general);
        }
        let allowance = state.army_grid_allowance(data, &state.armies[&army_id]);
        state
            .armies
            .get_mut(&army_id)
            .expect("just created")
            .movement_left = allowance;
    }
    Ok(())
}

/// Vassal loyalty starts at its equilibrium (M5); no alliance between a
/// suzerain and its direct vassal (ADR 0114); the felony cases of 1337
/// (Robert of Artois harboured by Edward III, F8).
fn init_feudal_ties(state: &mut CampaignState, data: &GameData) {
    let vassals: Vec<(FactionId, FactionId)> = state
        .factions
        .iter()
        .filter_map(|(id, f)| f.suzerain.clone().map(|s| (id.clone(), s)))
        .collect();
    for (vassal, suzerain) in vassals {
        let loyalty = crate::diplomacy::loyalty_target(state, data, &vassal, &suzerain);
        state.factions.get_mut(&vassal).expect("exists").loyalty = loyalty;
    }
    crate::feudal::drop_feudal_alliances(state, data);
    for case in &data.feudal_rules.start_felonies {
        crate::feudal::open_felony_towards(state, data, &case.vassal, &case.liege, case.reason);
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
    use data_model::test_support::game_data;

    use super::*;
    use crate::difficulty::Difficulty;

    #[test]
    fn the_starting_garrisons_do_not_depend_on_the_difficulty() {
        let data = game_data();
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
            fit_starting_garrisons(&mut state, data);
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

    /// Garrison sizes and field units of every faction.
    fn forces(state: &CampaignState) -> Vec<(String, usize)> {
        let garrisons = state
            .settlements
            .iter()
            .map(|(id, s)| (id.to_string(), s.garrison.len()));
        let armies = state
            .armies
            .iter()
            .map(|(id, a)| (format!("{id:?}"), a.units.len()));
        garrisons.chain(armies).collect()
    }

    #[test]
    fn the_field_and_capital_levers_only_touch_realms_still_over_budget() {
        // LR-15: the field armies, then the capital's garrison, go lighter
        // only for a realm that the garrisons alone could not bring within
        // the allowed deficit; the others start as under JR4b.
        let data = game_data();
        let rule = data
            .settlement_rules
            .as_ref()
            .and_then(|r| r.starting_budget.clone())
            .expect("starting budget");
        let min_field = rule.min_field_units.expect("field lever");
        let min_capital = rule.min_capital_units.expect("capital lever");
        let mut jr4b_data = data.clone();
        if let Some(r) = jr4b_data
            .settlement_rules
            .as_mut()
            .and_then(|r| r.starting_budget.as_mut())
        {
            r.min_field_units = None;
            r.min_capital_units = None;
        }
        let player = FactionId::new("fac_france").unwrap();
        let state = CampaignState::new_1337(data, player.clone(), 1).unwrap();
        let jr4b = CampaignState::new_1337(&jr4b_data, player, 1).unwrap();
        let mut touched = Vec::new();
        for faction in state.factions.keys() {
            let mine = |s: &CampaignState| {
                let places = s
                    .settlements
                    .iter()
                    .filter(|(_, p)| &p.controller == faction)
                    .map(|(id, p)| (id.to_string(), p.garrison.len()));
                let armies = s
                    .armies
                    .iter()
                    .filter(|(_, a)| &a.faction == faction)
                    .map(|(id, a)| (format!("{id:?}"), a.units.len()));
                places.chain(armies).collect::<Vec<_>>()
            };
            if mine(&state) == mine(&jr4b) {
                continue;
            }
            touched.push(faction.clone());
            assert!(
                over_budget(&jr4b, data, faction, &rule),
                "{faction} was within budget under JR4b"
            );
            let armies: Vec<usize> = state
                .armies
                .values()
                .filter(|a| &a.faction == faction)
                .map(|a| a.units.len())
                .collect();
            assert!(armies.iter().all(|n| *n > 0), "{faction}: empty army");
            assert!(armies.iter().sum::<usize>() >= min_field, "{faction}");
            if let Some(city) = state.faction_capital_city(faction) {
                assert!(state.settlements[city].garrison.len() >= min_capital);
            }
        }
        assert!(!touched.is_empty(), "the levers serve some realm");
        assert_ne!(forces(&state), forces(&jr4b));
        for great in ["fac_france", "fac_england", "fac_castile", "fac_mamluks"] {
            let great = FactionId::new(great).unwrap();
            assert!(!touched.contains(&great), "{great} untouched");
        }
    }

    #[test]
    fn the_realms_of_the_om_start_within_their_budget() {
        // LR-15: the Marinids and Hafsids (Maghreb populations raised),
        // Serbia and Lithuania (field and capital levers) no longer start in
        // structural bankruptcy.
        let data = game_data();
        // A6 x LR: A6-L3 asks a surplus (`max_deficit_percent` -10) that only the
        // garrisons can serve; the LR-15 fallbacks (field armies, capital
        // garrison, buildings) bring a realm to zero and no further. So the
        // invariant kept here is "no structural deficit" (net >= 0), not the
        // 10 % surplus (the Hafsids end at +4 %).
        let state =
            CampaignState::new_1337(data, FactionId::new("fac_france").unwrap(), 1).expect("start");
        for realm in ["fac_marinids", "fac_hafsids", "fac_serbia", "fac_lithuania"] {
            let realm = FactionId::new(realm).unwrap();
            let (net, receipts) = structural_balance(&state, data, &realm);
            assert!(
                !in_deficit(&state, data, &realm),
                "{realm}: {net} on {receipts}"
            );
        }
    }
}
