//! Feudal AI (lot FE5, spec FE § 5, ADR 0110).
//!
//! Every decision is a score of the weights of `data/ai/feudal.json`,
//! shifted by the faction's `ai_personality`:
//!
//! - **reactive** decisions, taken inside the resolution of another
//!   faction's order, are the [`POLICY`] the core calls
//!   (`sim_campaign::feudal::FeudalPolicy`): answer to a call for
//!   protection, arbitration of a private war, answer to the host;
//! - **proactive** decisions are orders of [`plan_feudal`]: forfeiture on
//!   an open felony case, grant of a spare title, revolt, homage to
//!   another lord (a disloyal vassal, or a threatened county seeking a
//!   protector);
//! - [`filter_suicidal_wars`] applies the « survival first » doctrine of
//!   `data/ai/doctrines.json` (`rank_strategies`) to the declarations of
//!   war, and [`demand_titles`] adds the titles a winner demands to its
//!   peace treaties (`Article::DemandTitle`).

use data_model::{AiFeudal, FactionId, GameData, TitleId, TitleRank};
use sim_campaign::feudal::{
    self as fe, direct_vassals, liege_of, primary_rank, Arbitration, FeudalPolicy, Likelihood,
};
use sim_campaign::negotiation::{check_treaty, evaluate_treaty, Article, Party};
use sim_campaign::{CampaignState, Order};

use crate::alignment::campaign_roll;

/// Salt of the per-turn feudal rolls (revolt, homage).
const FEUDAL_SALT: u64 = 0xFE05;

/// The feudal decisions of this crate, as the core calls them.
pub const POLICY: FeudalPolicy = FeudalPolicy {
    protection: protection_score,
    arbitration,
    answers_host,
    certainty_margin,
    planned_revolts: true,
};

/// Registers [`POLICY`] with the core (idempotent). [`crate::plan_turn`]
/// calls it; a caller that resolves feudal events before any AI planning
/// (the bridge at load, a test) calls it first.
pub fn install() {
    let _ = fe::install_policy(POLICY);
}

fn weights(data: &GameData) -> &AiFeudal {
    &data.ai_feudal
}

fn alive(state: &CampaignState, faction: &FactionId) -> bool {
    state.factions.get(faction).is_some_and(|f| f.alive)
}

/// `ai_personality.aggression` (0-100, 50 when unset).
fn aggression(data: &GameData, faction: &FactionId) -> i32 {
    data.factions
        .get(faction)
        .and_then(|f| f.ai_personality.as_ref())
        .and_then(|p| p.aggression)
        .map_or(50, i32::from)
}

/// `ai_personality.diplomacy` (0-100, 50 when unset).
fn diplomacy(data: &GameData, faction: &FactionId) -> i32 {
    data.factions
        .get(faction)
        .and_then(|f| f.ai_personality.as_ref())
        .and_then(|p| p.diplomacy)
        .map_or(50, i32::from)
}

/// `(trait - 50) * weight / 50`.
fn temper(value: i32, weight: i32) -> i32 {
    (value - 50) * weight / 50
}

fn wars(state: &CampaignState, faction: &FactionId) -> usize {
    state.factions.get(faction).map_or(0, |f| {
        f.at_war_with.iter().filter(|e| !is_rebels(e)).count()
    })
}

/// Power of `faction` and of its living allies, `except` left out.
fn coalition_power(state: &CampaignState, faction: &FactionId, except: &[&FactionId]) -> f64 {
    let own = state.faction_power(faction);
    let allies: f64 = state.factions.get(faction).map_or(0.0, |f| {
        f.allies
            .iter()
            .filter(|a| !except.contains(a) && alive(state, a))
            .map(|a| state.faction_power(a))
            .sum()
    });
    own + allies
}

fn label(data: &GameData, faction: &FactionId) -> String {
    data.factions
        .get(faction)
        .map_or_else(|| faction.to_string(), |f| f.name.display.clone())
}

fn is_rebels(faction: &FactionId) -> bool {
    faction.as_str() == sim_campaign::diplomacy::REBELS_FACTION
}

// =========================================================================
// Reactive decisions (the core's call points)
// =========================================================================

/// Score of `liege` called to protect `vassal` against `aggressor`,
/// relative to `protection.threshold`, with its main reason (French).
pub fn protection_score(
    state: &CampaignState,
    data: &GameData,
    liege: &FactionId,
    vassal: &FactionId,
    aggressor: &FactionId,
) -> (i32, String) {
    let w = &weights(data).protection;
    let mut terms: Vec<(i32, String)> = vec![(
        w.base,
        format!("devoir de protection envers {}", label(data, vassal)),
    )];
    // The liege's side: itself and its loyal direct vassals (the host it
    // would summon), against the aggressor and its allies.
    let host_pivot = weights(data).host.loyalty_pivot;
    let host: f64 = direct_vassals(state, data, liege)
        .iter()
        .filter(|v| *v != vassal && *v != aggressor)
        .filter(|v| {
            state
                .factions
                .get(*v)
                .is_some_and(|f| f.loyalty >= host_pivot)
        })
        .map(|v| state.faction_power(v))
        .sum();
    let ours = state.faction_power(liege) + state.faction_power(vassal) + host;
    let theirs = coalition_power(state, aggressor, &[liege, vassal]).max(1.0);
    let doublings = (ours.max(1.0) / theirs)
        .log2()
        .clamp(-w.max_power_doublings, w.max_power_doublings);
    let power = (doublings * f64::from(w.power_weight)).round() as i32;
    terms.push((
        power,
        if power >= 0 {
            "plus fort que l'agresseur".to_owned()
        } else {
            "plus faible que l'agresseur".to_owned()
        },
    ));
    let attitude = state.attitude(data, liege, vassal).0 / w.attitude_divisor.max(1);
    terms.push((
        attitude,
        if attitude >= 0 {
            "bonne entente avec le vassal".to_owned()
        } else {
            "mauvaise entente avec le vassal".to_owned()
        },
    ));
    let loyalty = state
        .factions
        .get(vassal)
        .map_or(0, |f| i32::from(f.loyalty));
    let fidelity = (loyalty - i32::from(w.loyalty_pivot)) / w.loyalty_divisor.max(1);
    terms.push((
        fidelity,
        if fidelity >= 0 {
            "vassal fidèle".to_owned()
        } else {
            "vassal peu fidèle".to_owned()
        },
    ));
    if state.factions.get(liege).is_some_and(|f| f.treasury < 0) {
        terms.push((w.empty_treasury, "trésor vide".to_owned()));
    }
    let wars = wars(state, liege);
    if wars > 0 {
        terms.push((
            w.per_ongoing_war * wars as i32,
            format!("déjà engagé dans {wars} guerre(s)"),
        ));
    }
    if state.is_allied(liege, aggressor) {
        terms.push((w.allied_with_aggressor, "allié de l'agresseur".to_owned()));
    }
    let temperament = temper(aggression(data, liege), w.aggression_weight)
        + temper(diplomacy(data, liege), w.diplomacy_weight);
    if temperament != 0 {
        terms.push((
            temperament,
            if temperament > 0 {
                "tempérament belliqueux et loyal".to_owned()
            } else {
                "tempérament prudent".to_owned()
            },
        ));
    }
    let score = terms.iter().map(|(v, _)| v).sum::<i32>() - w.threshold;
    let main = if score >= 0 {
        terms.iter().max_by_key(|(v, _)| *v)
    } else {
        terms.iter().min_by_key(|(v, _)| *v)
    };
    (score, main.map_or_else(String::new, |(_, r)| r.clone()))
}

fn certainty_margin(data: &GameData) -> i32 {
    weights(data).protection.certainty_margin
}

/// Verdict of `lord` on the private war of `attacker` against `target`
/// (§ 4.3.5), with its reason (French).
pub fn arbitration(
    state: &CampaignState,
    data: &GameData,
    lord: &FactionId,
    attacker: &FactionId,
    target: &FactionId,
) -> (Arbitration, String) {
    let w = &weights(data).arbitration;
    let gap = w.take_side_attitude_gap - temper(aggression(data, lord), w.aggression_weight);
    let towards_attacker = state.attitude(data, lord, attacker).0;
    let towards_target = state.attitude(data, lord, target).0;
    for (favoured, margin) in [
        (target, towards_target - towards_attacker),
        (attacker, towards_attacker - towards_target),
    ] {
        if margin >= gap {
            return (
                Arbitration::TakeSide {
                    side: favoured.clone(),
                },
                format!("préfère {}", label(data, favoured)),
            );
        }
    }
    let ratio = state.faction_power(lord) / state.faction_power(attacker).max(1.0);
    if wars(state, lord) > w.max_wars_to_impose {
        (
            Arbitration::LetBe,
            "trop occupé par ses propres guerres".to_owned(),
        )
    } else if ratio >= w.impose_peace_power_ratio {
        (
            Arbitration::ImposePeace,
            "assez fort pour imposer la paix".to_owned(),
        )
    } else {
        (
            Arbitration::LetBe,
            "trop faible pour imposer la paix".to_owned(),
        )
    }
}

/// Score of `vassal` summoned to the host of `liege` against `enemy`
/// (§ 4.1), relative to `host.threshold`.
pub fn host_score(
    state: &CampaignState,
    data: &GameData,
    vassal: &FactionId,
    liege: &FactionId,
    enemy: &FactionId,
) -> i32 {
    let w = &weights(data).host;
    let loyalty = state
        .factions
        .get(vassal)
        .map_or(0, |f| i32::from(f.loyalty));
    let mut score = (loyalty - i32::from(w.loyalty_pivot)) * w.loyalty_weight;
    let liege_power = state.faction_power(liege).max(1.0);
    if liege_power >= w.fear_power_ratio * state.faction_power(vassal).max(1.0) {
        score += w.fear_bonus;
    }
    if coalition_power(state, enemy, &[liege, vassal]) >= w.lost_cause_power_ratio * liege_power {
        score += w.lost_cause;
    }
    score -= state.attitude(data, vassal, enemy).0 / w.enemy_attitude_divisor.max(1);
    if state.is_allied(vassal, enemy) {
        score += w.allied_with_enemy;
    }
    score += temper(aggression(data, vassal), w.aggression_weight);
    score - w.threshold
}

fn answers_host(
    state: &CampaignState,
    data: &GameData,
    vassal: &FactionId,
    liege: &FactionId,
    enemy: &FactionId,
) -> bool {
    host_score(state, data, vassal, liege, enemy) >= 0
}

// =========================================================================
// Proactive decisions (orders)
// =========================================================================

/// Feudal orders of `faction` this turn: at most one forfeiture, then a
/// revolt or a homage, then a grant.
pub fn plan_feudal(state: &CampaignState, data: &GameData, faction: &FactionId) -> Vec<Order> {
    let mut orders = Vec::new();
    if !alive(state, faction) || !state.feudal.primary.contains_key(faction) {
        return orders;
    }
    let light = &weights(data).light_evaluation;
    if light.enabled && light.period > 1 && idle(state, faction) {
        let slot = crate::alignment::slot_of(faction);
        if !(state.turn + slot).is_multiple_of(light.period) {
            return orders;
        }
    }
    orders.extend(plan_commise(state, data, faction));
    if let Some(order) =
        plan_revolt(state, data, faction).or_else(|| plan_homage(state, data, faction))
    {
        orders.push(order);
    }
    orders.extend(plan_grant(state, data, faction));
    orders
}

/// No army raised and no war: the light evaluation applies.
fn idle(state: &CampaignState, faction: &FactionId) -> bool {
    wars(state, faction) == 0 && !state.armies.values().any(|a| &a.faction == faction)
}

/// Forfeiture against a felon under an open case, when the odds allow it.
pub fn plan_commise(state: &CampaignState, data: &GameData, faction: &FactionId) -> Option<Order> {
    let w = &weights(data).commise;
    let me = state.factions.get(faction)?;
    let cases: Vec<&FactionId> = state
        .feudal
        .felonies
        .iter()
        .filter(|c| &c.liege == faction && c.expires_turn >= state.turn)
        .map(|c| &c.vassal)
        .filter(|v| alive(state, v) && !fe::has_forfeiture(state, faction, v))
        .collect();
    if cases.is_empty() || me.treasury < w.min_treasury {
        return None;
    }
    let factor = 1.0 + f64::from(aggression(data, faction) - 50) / 50.0 * w.aggression_shift;
    let needed = w.min_power_ratio / factor.max(0.1);
    let mine = state.faction_power(faction);
    cases
        .into_iter()
        .filter(|v| {
            let other_wars = wars(state, faction) - usize::from(state.is_at_war(faction, v));
            other_wars <= w.max_wars
        })
        .find(|v| {
            let vassals: f64 = direct_vassals(state, data, v)
                .iter()
                .map(|x| state.faction_power(x))
                .sum();
            let felon = coalition_power(state, v, &[faction]) + vassals;
            mine >= needed * felon.max(1.0)
        })
        .map(|v| Order::DeclareCommise { vassal: v.clone() })
}

fn roll(state: &CampaignState, faction: &FactionId, kind: u64) -> u64 {
    campaign_roll(
        state,
        faction,
        FEUDAL_SALT ^ (kind << 32) ^ u64::from(state.turn),
    )
}

/// A disloyal AI vassal revolts when it and its allies weigh enough
/// against its suzerain.
pub fn plan_revolt(state: &CampaignState, data: &GameData, faction: &FactionId) -> Option<Order> {
    let w = &weights(data).revolt;
    let me = state.factions.get(faction)?;
    if me.loyalty >= data.feudal_rules.rebellion_loyalty || faction == &state.player_faction {
        return None;
    }
    let liege = liege_of(state, data, faction)?;
    if state.is_at_war(faction, &liege) {
        return None;
    }
    let ours = coalition_power(state, faction, &[&liege]);
    if ours < w.min_power_ratio * state.faction_power(&liege).max(1.0) {
        return None;
    }
    let chance = i64::from(w.chance_permille)
        + i64::from(aggression(data, faction) - 50) * i64::from(w.aggression_permille);
    (i64::try_from(roll(state, faction, 1)).unwrap_or(i64::MAX) < chance).then_some(Order::Revolt)
}

/// Homage to another lord: a disloyal vassal leaving a weak or faithless
/// suzerain, or a sovereign county at war with a far stronger neighbour
/// seeking a protector (« survival first »).
pub fn plan_homage(state: &CampaignState, data: &GameData, faction: &FactionId) -> Option<Order> {
    let w = &weights(data).allegiance;
    let me = state.factions.get(faction)?;
    let own_rank = primary_rank(state, data, faction)?;
    let liege = liege_of(state, data, faction);
    let threatened_by: Vec<&FactionId> = match &liege {
        Some(_) if me.loyalty < data.feudal_rules.disloyal_threshold => Vec::new(),
        Some(_) => return None,
        None => {
            let ratio = strategy(data, own_rank)?.seek_protection_power_ratio;
            let ours = coalition_power(state, faction, &[]).max(1.0);
            let threats: Vec<&FactionId> = me
                .at_war_with
                .iter()
                .filter(|e| !is_rebels(e))
                .filter(|e| coalition_power(state, e, &[faction]) >= ratio * ours)
                .collect();
            if threats.is_empty() {
                return None;
            }
            threats
        }
    };
    if (roll(state, faction, 2) as i64) >= i64::from(w.chance_permille) {
        return None;
    }
    let floor = liege
        .as_ref()
        .map_or(0.0, |l| w.min_power_ratio * state.faction_power(l));
    state
        .factions
        .iter()
        .filter(|(id, f)| {
            f.alive
                && *id != faction
                && Some(*id) != liege.as_ref()
                && **id != state.player_faction
                && !is_rebels(id)
        })
        .filter(|(id, _)| primary_rank(state, data, id).is_some_and(|r| r > own_rank))
        .filter(|(id, _)| !state.is_at_war(faction, id))
        .filter(|(id, f)| {
            threatened_by
                .iter()
                .all(|t| !f.allies.contains(*t) && *t != *id)
        })
        .filter(|(id, _)| !fe::liege_chain(state, data, id).contains(faction))
        .filter(|(id, _)| state.attitude(data, faction, id).0 >= w.min_attitude)
        .filter(|(id, _)| state.are_neighbors(data, faction, id))
        .map(|(id, _)| (id, state.faction_power(id)))
        .filter(|(_, power)| *power >= floor)
        .max_by(|a, b| a.1.total_cmp(&b.1).then_with(|| b.0.cmp(a.0)))
        .map(|(id, _)| Order::SwitchAllegiance { lord: id.clone() })
}

/// A lord holding more spare titles than it keeps grants the least of
/// them to its most loyal direct vassal of sufficient rank.
pub fn plan_grant(state: &CampaignState, data: &GameData, faction: &FactionId) -> Option<Order> {
    let w = &weights(data).grant;
    let period = w.period_turns.max(1);
    if !(state.turn + crate::alignment::slot_of(faction)).is_multiple_of(period) {
        return None;
    }
    let primary = state.feudal.primary.get(faction)?;
    let mut spare: Vec<(TitleRank, usize, &TitleId)> = state
        .feudal
        .holders
        .iter()
        .filter(|(t, h)| *h == faction && *t != primary)
        .filter_map(|(t, _)| {
            let title = data.titles.get(t)?;
            Some((title.rank, title.de_jure_provinces.len(), t))
        })
        .collect();
    if spare.len() <= w.max_kept_titles {
        return None;
    }
    spare.sort();
    let (rank, _, title) = spare.first()?;
    direct_vassals(state, data, faction)
        .into_iter()
        .filter(|v| v != &state.player_faction)
        .filter(|v| primary_rank(state, data, v).is_some_and(|r| r >= *rank))
        .filter_map(|v| {
            let loyalty = state.factions.get(&v)?.loyalty;
            (loyalty >= w.min_vassal_loyalty).then_some((loyalty, v))
        })
        .max_by(|a, b| a.0.cmp(&b.0).then_with(|| b.1.cmp(&a.1)))
        .map(|(_, grantee)| Order::GrantTitle {
            title: (*title).clone(),
            grantee,
        })
}

// =========================================================================
// Doctrine « survival first », titles demanded at the peace
// =========================================================================

fn strategy(data: &GameData, rank: TitleRank) -> Option<&data_model::RankStrategy> {
    data.ai_doctrines.as_ref()?.strategy(rank.key())
}

/// Power a declaration of war by `attacker` on `target` would face: the
/// target, its allies and the suzerains likely or maybe coming to its
/// help (the escalation preview of the core).
pub fn war_opposition(
    state: &CampaignState,
    data: &GameData,
    attacker: &FactionId,
    target: &FactionId,
) -> f64 {
    let mut total = coalition_power(state, target, &[attacker]);
    for step in fe::war_escalation_preview(state, data, attacker, target) {
        if step.likelihood != Likelihood::Unlikely
            && &step.faction != attacker
            && !state.is_allied(target, &step.faction)
        {
            total += state.faction_power(&step.faction);
        }
    }
    total
}

/// Drops the declarations of war the « survival first » doctrine of the
/// faction's rank forbids: our side must outweigh the opposition by
/// `min_war_power_ratio`.
pub fn filter_suicidal_wars(
    state: &CampaignState,
    data: &GameData,
    faction: &FactionId,
    orders: &mut Vec<Order>,
) {
    let Some(rank) = primary_rank(state, data, faction) else {
        return;
    };
    let Some(strategy) = strategy(data, rank) else {
        return;
    };
    if !orders.iter().any(|o| matches!(o, Order::DeclareWar { .. })) {
        return;
    }
    let ours = coalition_power(state, faction, &[]);
    orders.retain(|order| match order {
        Order::DeclareWar { target } => {
            ours >= strategy.min_war_power_ratio * war_opposition(state, data, faction, target)
        }
        _ => true,
    });
}

/// Adds to each winner's peace treaty the titles of the loser whose own
/// provinces it mostly controls (§ 4.6), as long as the treaty stays valid
/// and likely to be signed.
pub fn demand_titles(
    state: &CampaignState,
    data: &GameData,
    faction: &FactionId,
    orders: &mut [Order],
) {
    let w = &weights(data).demand_title;
    if !w.enabled {
        return;
    }
    let min_chance = data.ai_diplomacy.negotiation.ai_min_chance;
    let demand_score = data.ai_diplomacy.negotiation.demand_score;
    for order in orders.iter_mut() {
        let Order::ProposeTreaty { target, articles } = order else {
            continue;
        };
        if !state.is_at_war(faction, target)
            || state.war_score(data, faction, target) < demand_score
        {
            continue;
        }
        let primary = state.feudal.primary.get(target);
        let wanted: Vec<TitleId> = state
            .feudal
            .holders
            .iter()
            .filter(|(t, h)| *h == target && Some(*t) != primary)
            .filter(|(t, _)| {
                let Some(title) = data.titles.get(*t) else {
                    return false;
                };
                let own = &title.de_jure_provinces;
                let held = own
                    .iter()
                    .filter(|p| state.controls_province(faction, p))
                    .count();
                !own.is_empty() && held * 100 >= own.len() * w.min_controlled_percent as usize
            })
            .map(|(t, _)| t.clone())
            .collect();
        for title in wanted {
            articles.push(Article::DemandTitle {
                giver: Party::Recipient,
                title,
            });
            let ok = check_treaty(state, data, faction, target, articles).is_ok()
                && (target == &state.player_faction
                    || evaluate_treaty(state, data, faction, target, articles).chance
                        >= min_chance);
            if !ok {
                articles.pop();
            }
        }
    }
}
