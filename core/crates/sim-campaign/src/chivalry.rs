//! H6 « Ordres de chevalerie » : Jarretière, Étoile, Toison d'or...
//!
//! A faction founds at most one order (`found_chivalric_order`, or the
//! « fonder » option of `evt_ordre_de_la_jarretiere` /
//! `evt_ordre_de_l_etoile`): it pays the cost, needs the ruler's prestige,
//! and its best adult characters are named automatically up to the order's
//! (reduced) strength. Members gain loyalty once; the units a member leads
//! fight with more morale; the ruler gains prestige at foundation and every
//! year. Dead or lost members are replaced every season.
//!
//! Mauron (1352): when an order loses half of its members within one season
//! (killed or captured, typically in a disastrous battle), it is broken: it
//! stops giving its bonuses and the ruler loses prestige
//! ([`COLLAPSE_PRESTIGE`]). Simplification: only generals fight as named
//! characters, so the rule counts the season's losses rather than those of
//! one battle.

use data_model::{CharacterId, ChivalricOrder, ChivalricOrderId, FactionId, GameData, Sex};
use serde::{Deserialize, Serialize};

use crate::events::{EventKind, GameEvent};
use crate::orders::Order;
use crate::state::CampaignState;

/// Ruler prestige lost when the order collapses.
pub const COLLAPSE_PRESTIGE: i32 = 15;

/// The order a faction has founded.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrderState {
    pub order: ChivalricOrderId,
    pub founded_turn: u32,
    /// Current members, best first.
    #[serde(default)]
    pub members: Vec<CharacterId>,
    /// Broken by a disaster (Mauron): no more bonuses.
    #[serde(default)]
    pub collapsed: bool,
}

/// Why `found_chivalric_order` was refused.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ChivalryError {
    #[error("ordre de chevalerie inconnu : {0}")]
    UnknownOrder(ChivalricOrderId),
    #[error("votre faction a déjà fondé un ordre")]
    AlreadyFounded,
    #[error("cet ordre appartient à une autre faction")]
    OtherFaction,
    #[error("cet ordre est déjà fondé par une autre faction")]
    TakenByOther,
    #[error("cet ordre ne peut être fondé avant {0}")]
    TooEarly(i32),
    #[error("prestige insuffisant : {needed} requis, {current} actuellement")]
    NotEnoughPrestige { needed: i32, current: i32 },
    #[error("trésor insuffisant : {needed} livres nécessaires, {available} disponibles")]
    InsufficientFunds { needed: i64, available: i64 },
    #[error("votre faction n'a pas de souverain")]
    NoRuler,
}

/// Faction that founded `order`, if any.
pub fn founder_of(state: &CampaignState, order: &ChivalricOrderId) -> Option<FactionId> {
    state
        .factions
        .iter()
        .find(|(_, f)| f.chivalric_order.as_ref().is_some_and(|o| &o.order == order))
        .map(|(id, _)| id.clone())
}

/// Orders `faction` may consider: its historical order(s), or the generic
/// ones (no `faction`) when it has none. Id order.
pub fn orders_for<'a>(data: &'a GameData, faction: &FactionId) -> Vec<&'a ChivalricOrder> {
    let own: Vec<&ChivalricOrder> = data
        .chivalric_orders
        .values()
        .filter(|o| o.faction.as_ref() == Some(faction))
        .collect();
    if !own.is_empty() {
        return own;
    }
    data.chivalric_orders
        .values()
        .filter(|o| o.faction.is_none())
        .collect()
}

/// Foundation cost at the faction's current prices (H5).
pub fn foundation_cost(state: &CampaignState, faction: &FactionId, order: &ChivalricOrder) -> i64 {
    crate::coinage::priced(state, faction, i64::from(order.cost))
}

/// Why `faction` cannot found `order` now (`None`: it can).
pub fn found_blocker(
    state: &CampaignState,
    data: &GameData,
    faction: &FactionId,
    order: &ChivalricOrderId,
) -> Option<ChivalryError> {
    let Some(definition) = data.chivalric_orders.get(order) else {
        return Some(ChivalryError::UnknownOrder(order.clone()));
    };
    let f = state.factions.get(faction)?;
    if f.chivalric_order.is_some() {
        return Some(ChivalryError::AlreadyFounded);
    }
    if !orders_for(data, faction).iter().any(|o| &o.id == order) {
        return Some(ChivalryError::OtherFaction);
    }
    if definition.faction.is_some() && founder_of(state, order).is_some() {
        return Some(ChivalryError::TakenByOther);
    }
    if let Some(year) = definition.min_year.filter(|y| state.year < *y) {
        return Some(ChivalryError::TooEarly(year));
    }
    let Some(ruler) = f.ruler.as_ref().and_then(|r| state.characters.get(r)) else {
        return Some(ChivalryError::NoRuler);
    };
    if ruler.prestige < definition.prestige_required {
        return Some(ChivalryError::NotEnoughPrestige {
            needed: definition.prestige_required,
            current: ruler.prestige,
        });
    }
    let cost = foundation_cost(state, faction, definition);
    if f.treasury < cost {
        return Some(ChivalryError::InsufficientFunds {
            needed: cost,
            available: f.treasury,
        });
    }
    None
}

/// `found_chivalric_order`: pays, founds, names the members; the ruler
/// gains the order's `founder_prestige`.
pub fn found_order(
    state: &mut CampaignState,
    data: &GameData,
    faction: &FactionId,
    order: &ChivalricOrderId,
) -> Result<(), ChivalryError> {
    if let Some(error) = found_blocker(state, data, faction, order) {
        return Err(error);
    }
    let definition = &data.chivalric_orders[order];
    let cost = foundation_cost(state, faction, definition);
    state.factions.get_mut(faction).expect("checked").treasury -= cost;
    if let Some(ruler) = state.factions[faction].ruler.clone() {
        if let Some(r) = state.characters.get_mut(&ruler) {
            r.prestige += definition.founder_prestige;
        }
    }
    let mut events = Vec::new();
    establish(state, data, faction, order, &mut events);
    state.pending_events.extend(events);
    Ok(())
}

/// Event effect `found_chivalric_order` (« fonder » options of the Garter
/// and Star events): the event's own effects carry the cost and the
/// prestige, so this only founds the order and names its members; no-op if
/// the faction already has an order or another faction holds this one.
pub fn found_order_by_event(
    state: &mut CampaignState,
    data: &GameData,
    faction: &FactionId,
    order: &ChivalricOrderId,
    events: &mut Vec<GameEvent>,
) {
    let free = state
        .factions
        .get(faction)
        .is_some_and(|f| f.alive && f.chivalric_order.is_none());
    let taken = founder_of(state, order).is_some();
    if free && !taken && data.chivalric_orders.contains_key(order) {
        establish(state, data, faction, order, events);
    }
}

fn establish(
    state: &mut CampaignState,
    data: &GameData,
    faction: &FactionId,
    order: &ChivalricOrderId,
    events: &mut Vec<GameEvent>,
) {
    let turn = state.turn;
    state.factions.get_mut(faction).expect("checked").chivalric_order = Some(OrderState {
        order: order.clone(),
        founded_turn: turn,
        members: Vec::new(),
        collapsed: false,
    });
    let definition = &data.chivalric_orders[order];
    events.push(
        GameEvent::new(
            EventKind::Chivalry,
            format!(
                "{} fonde {}.",
                faction_label(data, faction),
                definition.name.display
            ),
        )
        .faction(faction),
    );
    fill_members(state, data, faction, events);
}

fn faction_label(data: &GameData, faction: &FactionId) -> String {
    data.factions
        .get(faction)
        .map_or_else(|| faction.to_string(), |f| f.name.display.clone())
}

/// A character who may sit in the order: alive, free, adult man of the
/// faction, not its ruler (the ruler is the order's sovereign).
fn eligible(state: &CampaignState, faction: &FactionId, id: &CharacterId) -> bool {
    let is_ruler = state
        .factions
        .get(faction)
        .is_some_and(|f| f.ruler.as_ref() == Some(id));
    state.characters.get(id).is_some_and(|c| {
        &c.faction == faction
            && c.alive
            && !c.captive
            && c.sex == Sex::Male
            && c.is_major(state.year)
            && !is_ruler
    })
}

/// Merit used to pick the members: prestige, command and battles fought.
pub fn merit(state: &CampaignState, id: &CharacterId) -> i32 {
    state.characters.get(id).map_or(0, |c| {
        c.prestige + 5 * i32::from(c.skills.command) + 2 * c.battles_fought as i32
    })
}

/// Drops the members who died, were captured or left, then names the best
/// eligible characters up to the order's strength (merit, then id). A
/// newly named member gains the order's loyalty once.
fn fill_members(
    state: &mut CampaignState,
    data: &GameData,
    faction: &FactionId,
    events: &mut Vec<GameEvent>,
) {
    let Some(current) = state.factions.get(faction).and_then(|f| f.chivalric_order.clone()) else {
        return;
    };
    let Some(definition) = data.chivalric_orders.get(&current.order) else {
        return;
    };
    let mut members: Vec<CharacterId> = current
        .members
        .iter()
        .filter(|m| eligible(state, faction, m))
        .cloned()
        .collect();
    let mut candidates: Vec<CharacterId> = state
        .characters
        .keys()
        .filter(|id| eligible(state, faction, id) && !members.contains(id))
        .cloned()
        .collect();
    // Stable sort: equal merit keeps the id order (determinism).
    candidates.sort_by_key(|id| std::cmp::Reverse(merit(state, id)));
    let mut named = Vec::new();
    for id in candidates {
        if members.len() >= definition.members as usize {
            break;
        }
        members.push(id.clone());
        named.push(id);
    }
    for id in &named {
        if let Some(c) = state.characters.get_mut(id) {
            c.loyalty = c.loyalty.saturating_add(definition.member_loyalty).min(100);
        }
    }
    if !named.is_empty() && faction == &state.player_faction {
        let names: Vec<String> = named.iter().map(|id| state.character_name(data, id)).collect();
        events.push(
            GameEvent::new(
                EventKind::Chivalry,
                format!(
                    "{} : nouveaux compagnons — {}.",
                    definition.name.display,
                    names.join(", ")
                ),
            )
            .faction(faction),
        );
    }
    if let Some(order) = state
        .factions
        .get_mut(faction)
        .and_then(|f| f.chivalric_order.as_mut())
    {
        order.members = members;
    }
}

/// Morale bonus a member general gives the units he leads (0 for a
/// non-member or a broken order); read by `skills::character_effects`.
pub fn member_morale(state: &CampaignState, data: &GameData, id: &CharacterId) -> f64 {
    let Some(c) = state.characters.get(id) else {
        return 0.0;
    };
    state
        .factions
        .get(&c.faction)
        .and_then(|f| f.chivalric_order.as_ref())
        .filter(|o| !o.collapsed && o.members.contains(id))
        .and_then(|o| data.chivalric_orders.get(&o.order))
        .map_or(0.0, |d| f64::from(d.member_morale))
}

/// Seasonal phase: a disaster breaks the order (half of the members lost
/// since the last season, dead or captured); otherwise the ranks are
/// refilled, and each winter the ruler gains the order's yearly prestige.
pub(crate) fn resolve_chivalry(
    state: &mut CampaignState,
    data: &GameData,
    events: &mut Vec<GameEvent>,
) {
    let ids: Vec<FactionId> = state
        .factions
        .iter()
        .filter(|(_, f)| f.alive && f.chivalric_order.as_ref().is_some_and(|o| !o.collapsed))
        .map(|(id, _)| id.clone())
        .collect();
    for faction in ids {
        let order = state.factions[&faction]
            .chivalric_order
            .clone()
            .expect("filtered above");
        let Some(definition) = data.chivalric_orders.get(&order.order) else {
            continue;
        };
        let lost = order
            .members
            .iter()
            .filter(|m| {
                state
                    .characters
                    .get(*m)
                    .is_none_or(|c| !c.alive || c.captive)
            })
            .count();
        if order.members.len() >= 2 && lost * 2 >= order.members.len() {
            if let Some(o) = state
                .factions
                .get_mut(&faction)
                .and_then(|f| f.chivalric_order.as_mut())
            {
                o.collapsed = true;
            }
            if let Some(ruler) = state.factions[&faction].ruler.clone() {
                if let Some(r) = state.characters.get_mut(&ruler) {
                    r.prestige -= COLLAPSE_PRESTIGE;
                }
            }
            events.push(
                GameEvent::new(
                    EventKind::Chivalry,
                    format!(
                        "{} a perdu la moitié de ses compagnons : l'ordre ne s'en relèvera pas.",
                        definition.name.display
                    ),
                )
                .faction(&faction),
            );
            continue;
        }
        fill_members(state, data, &faction, events);
        if state.season == crate::state::Season::Winter {
            if let Some(ruler) = state.factions[&faction].ruler.clone() {
                if let Some(r) = state.characters.get_mut(&ruler) {
                    r.prestige += definition.yearly_prestige;
                }
            }
        }
    }
}

/// AI: founds the first available order as soon as it can while keeping
/// four seasons of income in the treasury. Deterministic.
pub fn ai_found_order(state: &CampaignState, data: &GameData, faction: &FactionId) -> Vec<Order> {
    let Some(f) = state.factions.get(faction).filter(|f| f.alive) else {
        return Vec::new();
    };
    if f.chivalric_order.is_some() {
        return Vec::new();
    }
    let reserve = 4 * state.faction_income_effective(data, faction).max(0);
    orders_for(data, faction)
        .into_iter()
        .find(|o| {
            found_blocker(state, data, faction, &o.id).is_none()
                && f.treasury - foundation_cost(state, faction, o) >= reserve
        })
        .map(|o| {
            vec![Order::FoundChivalricOrder {
                order: o.id.clone(),
            }]
        })
        .unwrap_or_default()
}
