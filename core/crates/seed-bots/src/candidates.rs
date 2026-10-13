//! Legal candidate orders: the orders the requirement and preview queries
//! declare possible for the player this turn.

use data_model::{GameData, SettlementId};
use sim_campaign::orders::Place;
use sim_campaign::{negotiation, posture, research, royal_acts};
use sim_campaign::{Order, RelationKind, TaxRate, TechStatus};

use crate::view::BotView;

/// Family of an order, used by bots to weigh candidates by personality.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Family {
    /// Moves, attacks, sieges, embarking.
    Military,
    /// Recruitment and army organisation.
    Recruitment,
    /// Buildings, taxes, edicts, research.
    Economy,
    /// War, peace, alliances, treaties, marriages, vassals.
    Diplomacy,
    /// Characters, agents, council, captives, events.
    Court,
}

/// A candidate order with its family.
#[derive(Debug, Clone)]
pub struct Candidate {
    /// The order, which the queries say the rules should accept.
    pub order: Order,
    /// Its family.
    pub family: Family,
}

/// Settlements examined per call for buildings and recruits.
const SETTLEMENTS_PER_CALL: usize = 10;
/// Destinations sampled per army.
const MOVES_PER_ARMY: usize = 4;
/// Buildings offered per settlement.
const BUILDS_PER_SETTLEMENT: usize = 3;
/// Unit types offered per settlement.
const RECRUITS_PER_SETTLEMENT: usize = 3;
/// Edicts or diets offered per province.
const POLICIES_PER_PROVINCE: usize = 2;
/// Gift the diplomat candidates offer, livres.
const GIFT_AMOUNT: i64 = 50;

/// `count` items spread evenly over `items` (deterministic, in order).
fn spread<T: Clone>(items: &[T], count: usize) -> Vec<T> {
    if items.len() <= count {
        return items.to_vec();
    }
    (0..count)
        .map(|i| items[i * items.len() / count].clone())
        .collect()
}

/// Every order the queries declare possible for the bot's faction this turn.
///
/// Built only from the queries the interface uses (`buildable`,
/// `recruitable`, `reachable_cells`, `edict_options`, `diplomacy_view`,
/// `check_treaty`, pending decisions…); `Debug*` orders never appear. The
/// list is bounded: a few buildings, units and destinations per place.
/// Orders without a prerequisite query are listed in `docs/wip/seed-bots.md`.
pub fn legal_candidates(view: &BotView, data: &GameData) -> Vec<Candidate> {
    let mut out = Vec::new();
    economy(view, data, &mut out);
    recruitment(view, data, &mut out);
    military(view, data, &mut out);
    diplomacy(view, data, &mut out);
    court(view, data, &mut out);
    out
}

fn push(out: &mut Vec<Candidate>, family: Family, order: Order) {
    out.push(Candidate { order, family });
}

/// At most [`SETTLEMENTS_PER_CALL`] settlements the faction owns and
/// controls, a window that moves with the turn so every place comes up in
/// turn (bounded cost on a large realm).
fn own_settlements<'a>(view: &'a BotView) -> Vec<&'a SettlementId> {
    let all: Vec<&SettlementId> = view
        .state
        .settlements
        .iter()
        .filter(|(_, s)| &s.owner == view.faction && &s.controller == view.faction)
        .map(|(id, _)| id)
        .collect();
    if all.len() <= SETTLEMENTS_PER_CALL {
        return all;
    }
    let start = view.state.turn() as usize * SETTLEMENTS_PER_CALL % all.len();
    (0..SETTLEMENTS_PER_CALL)
        .map(|i| all[(start + i) % all.len()])
        .collect()
}

fn economy(view: &BotView, data: &GameData, out: &mut Vec<Candidate>) {
    let state = view.state;
    let Some(faction) = state.factions.get(view.faction) else {
        return;
    };
    for id in own_settlements(view) {
        let options = state.buildable(data, id);
        if !state.can_build_from_options(data, id, &options).ok {
            continue;
        }
        let available: Vec<_> = options.iter().filter(|o| o.available).collect();
        for option in spread(&available, BUILDS_PER_SETTLEMENT) {
            push(
                out,
                Family::Economy,
                Order::Build {
                    settlement: Place::Settlement(id.clone()),
                    building: option.building.clone(),
                },
            );
        }
    }
    for rate in [TaxRate::Low, TaxRate::Normal, TaxRate::High] {
        if rate != faction.tax_rate {
            push(out, Family::Economy, Order::SetTaxRate { rate });
        }
    }
    for technology in data.technologies.values() {
        if research::tech_status(state, view.faction, technology) == TechStatus::Available {
            push(
                out,
                Family::Economy,
                Order::Research {
                    technology: technology.id.clone(),
                },
            );
        }
    }
    for province in state.controlled_provinces(view.faction) {
        let edicts: Vec<_> = state
            .edict_options(data, province)
            .into_iter()
            .filter(|o| o.available && !o.current)
            .collect();
        for option in spread(&edicts, POLICIES_PER_PROVINCE) {
            push(
                out,
                Family::Economy,
                Order::SetEdict {
                    province: province.clone(),
                    edict: option.edict.clone(),
                },
            );
        }
        let diets: Vec<_> = state
            .diet_options(data, province)
            .into_iter()
            .filter(|o| o.available && !o.current)
            .collect();
        for option in spread(&diets, POLICIES_PER_PROVINCE) {
            push(
                out,
                Family::Economy,
                Order::SetDiet {
                    province: province.clone(),
                    diet: option.diet.clone(),
                },
            );
        }
    }
    for option in royal_acts::royal_act_options(state, data, view.faction) {
        if option.available {
            push(out, Family::Economy, Order::RoyalAct { act: option.id });
        }
    }
}

fn recruitment(view: &BotView, data: &GameData, out: &mut Vec<Candidate>) {
    let state = view.state;
    for id in own_settlements(view) {
        if state.recruit_slots_free(data, id) == 0 {
            continue;
        }
        let available: Vec<_> = state
            .recruitable(data, id)
            .into_iter()
            .filter(|o| o.available)
            .collect();
        for option in spread(&available, RECRUITS_PER_SETTLEMENT) {
            push(
                out,
                Family::Recruitment,
                Order::Recruit {
                    settlement: Place::Settlement(id.clone()),
                    unit_type: option.unit_type.clone(),
                },
            );
        }
    }
}

fn military(view: &BotView, data: &GameData, out: &mut Vec<Candidate>) {
    let state = view.state;
    let grid = data.navgrid();
    for (army_id, army) in state
        .armies
        .iter()
        .filter(|(_, a)| &a.faction == view.faction)
    {
        let start = state.army_cell(data, army);
        let reach = state.reachable_cells(data, army_id);
        let cells: Vec<_> = reach
            .this_turn
            .iter()
            .filter(|(cell, cost)| *cell != start && *cost > 0)
            .map(|(cell, _)| *cell)
            .collect();
        for cell in spread(&cells, MOVES_PER_ARMY) {
            push(
                out,
                Family::Military,
                Order::move_to_point(army_id.clone(), cell.center(grid)),
            );
        }
        for (stance, allowed) in posture::stance_options(state, data, army_id) {
            if allowed.is_ok() && stance != army.stance {
                push(
                    out,
                    Family::Military,
                    Order::SetStance {
                        army: army_id.clone(),
                        stance,
                    },
                );
            }
        }
        let besieging = army.settlement().is_some_and(|place| {
            state
                .settlements
                .get(place)
                .is_some_and(|s| s.siege.is_some() && &s.controller != view.faction)
        });
        if besieging && state.assault_blocker(data, army_id).is_none() {
            push(
                out,
                Family::Military,
                Order::Assault {
                    army: army_id.clone(),
                },
            );
        }
    }
}

fn diplomacy(view: &BotView, data: &GameData, out: &mut Vec<Candidate>) {
    let state = view.state;
    let treasury = state.factions.get(view.faction).map_or(0, |f| f.treasury);
    // Neighbours and enemies only: other realms are out of reach, and each
    // treaty check is costly.
    let mut targets = state.neighbour_factions(data, view.faction);
    if let Some(faction) = state.factions.get(view.faction) {
        targets.extend(faction.at_war_with.iter().cloned());
    }
    for target in targets {
        let alive = state
            .factions
            .get(&target)
            .is_some_and(|f| f.alive && !target.is_rebels());
        if !alive || &target == view.faction {
            continue;
        }
        let relation = state.relation(view.faction, &target);
        let proposals = match relation {
            RelationKind::War => vec![Order::ProposePeace {
                target: target.clone(),
                provinces: Vec::new(),
                tribute: 0,
            }],
            RelationKind::Peace | RelationKind::Truce => vec![Order::ProposeAlliance {
                target: target.clone(),
            }],
            _ => Vec::new(),
        };
        for order in proposals {
            let accepted = order
                .proposal(state, view.faction)
                .is_some_and(|(to, treaty)| {
                    negotiation::check_treaty(state, data, view.faction, &to, &treaty.articles)
                        .is_ok()
                });
            if accepted {
                push(out, Family::Diplomacy, order);
            }
        }
        if relation == RelationKind::Peace {
            push(
                out,
                Family::Diplomacy,
                Order::DeclareWar {
                    target: target.clone(),
                },
            );
        }
        if treasury >= GIFT_AMOUNT && relation != RelationKind::War {
            push(
                out,
                Family::Diplomacy,
                Order::SendGift {
                    target: target.clone(),
                    amount: GIFT_AMOUNT,
                },
            );
        }
    }
    for offer in state.valid_offers(data, view.faction) {
        for accept in [true, false] {
            push(
                out,
                Family::Diplomacy,
                Order::AnswerOffer {
                    offer: offer.id,
                    accept,
                },
            );
        }
    }
}

fn court(view: &BotView, data: &GameData, out: &mut Vec<Candidate>) {
    let state = view.state;
    for decision in state.decision_views(data, view.faction) {
        for option in decision.options.iter().filter(|o| o.allowed) {
            push(
                out,
                Family::Court,
                Order::ChooseEventOption {
                    decision: decision.id,
                    option: option.index,
                },
            );
        }
    }
    for encounter in state.pending_encounter_views(data, view.faction) {
        for option in encounter.options.iter().filter(|o| o.available) {
            push(
                out,
                Family::Court,
                Order::ChooseEncounterOption {
                    army: encounter.army.clone(),
                    site: encounter.site,
                    option: option.index,
                },
            );
        }
    }
    for capture in state.capture_decision_views(data, view.faction) {
        for option in capture.options.iter().filter(|o| o.allowed) {
            push(
                out,
                Family::Court,
                Order::ChooseCaptureOutcome {
                    decision: capture.id,
                    outcome: option.outcome,
                },
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use std::time::Instant;

    use super::*;
    use crate::view::fixtures;

    #[test]
    fn candidates_cover_families_and_are_accepted() {
        let data = fixtures::data();
        let state = fixtures::state();
        let faction = fixtures::france();
        let view = BotView {
            state: &state,
            faction: &faction,
        };
        let started = Instant::now();
        let candidates = legal_candidates(&view, data);
        let first = started.elapsed();
        let started = Instant::now();
        let _ = legal_candidates(&view, data);
        eprintln!("first call {first:?}, second {:?}", started.elapsed());
        let started = Instant::now();
        eprintln!(
            "legal_candidates: {:?}, {} candidates",
            started.elapsed(),
            candidates.len()
        );
        assert!(!candidates.is_empty());
        let families: HashSet<_> = candidates.iter().map(|c| c.family).collect();
        for family in [
            Family::Military,
            Family::Recruitment,
            Family::Economy,
            Family::Diplomacy,
        ] {
            assert!(families.contains(&family), "no {family:?} candidate");
        }
        assert!(candidates
            .iter()
            .all(|c| !format!("{:?}", c.order).starts_with("Debug")));
        // Each Economy/Recruitment candidate is accepted on its own on a copy.
        for candidate in candidates
            .iter()
            .filter(|c| matches!(c.family, Family::Economy | Family::Recruitment))
        {
            let mut copy = state.clone();
            let result = copy.submit_order(data, candidate.order.clone());
            assert!(result.is_ok(), "{:?} refused: {result:?}", candidate.order);
        }
    }
}
