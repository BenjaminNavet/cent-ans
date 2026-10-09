//! Diplomatic AI (pure; M5 § 2.3, F4).

use super::*;

/// Score bonus making a claim war preferred to any opportunistic war.
const CLAIM_WAR_PRIORITY: f64 = 1000.0;
/// Peace reluctance of a pretender towards the crown it claims. (Power
/// ratios of pretenders and co-belligerents: `data/ai/diplomacy.json`.)
pub const PRETENDER_PEACE_RELUCTANCE: i32 = 10;
/// Power ratio of an opportunistic war without claim.
pub const OPPORTUNIST_RATIO: f64 = 1.5;
/// Minimum aggression to press a claim / to wage an opportunistic war.
pub const PRETENDER_AGGRESSION: i32 = 45;
pub const OPPORTUNIST_AGGRESSION: i32 = 60;
/// Turns between two war declarations of the same faction.
pub const WAR_REST_TURNS: u32 = 12;
/// Alliances (vassal ties excluded) an AI seeks at most.
pub const MAX_ALLIANCES: usize = 4;
/// War score below which a beaten realm offers what the enemy holds of it.
pub const SURRENDER_WAR_SCORE: i32 = -25;
/// War score below which a vassal deserts its losing suzerain.
pub const DESERTION_WAR_SCORE: i32 = -25;

/// What `a` claims from `b`: the crown, and how many of `b`'s provinces.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ClaimStakes {
    pub throne: bool,
    pub provinces: usize,
}

impl ClaimStakes {
    pub fn any(self) -> bool {
        self.throne || self.provinces > 0
    }
}

/// Claims `a` holds against `b` (F4).
pub fn claim_stakes(state: &CampaignState, a: &FactionId, b: &FactionId) -> ClaimStakes {
    let mut stakes = ClaimStakes::default();
    let Some(fa) = state.factions.get(a) else {
        return stakes;
    };
    let mut seen = BTreeSet::new();
    for claim in &fa.claims {
        match claim.kind {
            ClaimKind::Throne => stakes.throne |= claim.faction.as_ref() == Some(b),
            ClaimKind::Province => {
                if let Some(p) = &claim.province {
                    if state.province_owner(p) == Some(b) && seen.insert(p) {
                        stakes.provinces += 1;
                    }
                }
            }
        }
    }
    stakes
}

/// Provinces `faction` considers rightfully its own: claimed provinces and
/// every province of a crown it claims (F4: an English army lands anywhere
/// in France).
pub fn claimed_provinces(state: &CampaignState, faction: &FactionId) -> BTreeSet<ProvinceId> {
    let Some(f) = state.factions.get(faction) else {
        return BTreeSet::new();
    };
    let thrones: BTreeSet<&FactionId> = f
        .claims
        .iter()
        .filter(|c| c.kind == ClaimKind::Throne)
        .filter_map(|c| c.faction.as_ref())
        .collect();
    let mut provinces: BTreeSet<ProvinceId> =
        f.claims.iter().filter_map(|c| c.province.clone()).collect();
    provinces.extend(
        state
            .provinces
            .keys()
            .filter(|id| {
                state
                    .province_owner(id)
                    .is_some_and(|o| thrones.contains(o))
            })
            .cloned(),
    );
    provinces
}

/// Factions `faction` quarrels with: current enemies, the targets of its
/// claims and the factions claiming its lands.
pub fn rivals(state: &CampaignState, faction: &FactionId) -> BTreeSet<FactionId> {
    let Some(me) = state.factions.get(faction) else {
        return BTreeSet::new();
    };
    state
        .factions
        .iter()
        .filter(|(id, f)| *id != faction && f.alive && !id.is_rebels())
        .filter(|(id, _)| {
            me.at_war_with.contains(*id)
                || claim_stakes(state, faction, id).any()
                || claim_stakes(state, id, faction).any()
        })
        .map(|(id, _)| id.clone())
        .collect()
}

/// Can `faction` afford a new war: no regency, no debt, a season of upkeep
/// in the chest, a surplus and a free ruler (F4 tempo).
pub fn war_ready(state: &CampaignState, faction: &FactionId) -> bool {
    let Some(me) = state.factions.get(faction) else {
        return false;
    };
    let ruler_free = me
        .ruler
        .as_ref()
        .and_then(|r| state.characters.get(r))
        .is_none_or(|r| !r.captive);
    !me.regency
        && ruler_free
        && me.treasury > 0
        && me.treasury >= 2 * me.last_budget.upkeep().max(0)
        && (me.last_budget.income >= me.last_budget.upkeep()
            || me.treasury >= 8 * me.last_budget.upkeep().max(0))
}

/// Power of the enemies `faction` already fights (rebels excluded).
/// Only fronts that press on us count: bordering enemies and enemies
/// holding our provinces (a distant war of religion does not tie armies).
fn enemy_power(cache: &PlanCache, data: &GameData, faction: &FactionId) -> f64 {
    let state = cache.state();
    state.factions.get(faction).map_or(0.0, |f| {
        f.at_war_with
            .iter()
            .filter(|e| !e.is_rebels())
            .filter(|e| {
                cache.are_neighbors(data, faction, e)
                    || state.provinces.keys().any(|id| {
                        state.province_owner(id) == Some(faction) && state.controls_province(e, id)
                    })
            })
            .map(|e| cache.faction_power(e))
            .sum()
    })
}

/// Alliances proper (vassal ties excluded).
fn alliance_count(state: &CampaignState, faction: &FactionId) -> usize {
    state.factions.get(faction).map_or(0, |f| {
        f.allies
            .iter()
            .filter(|a| state.relation(faction, a) == RelationKind::Alliance)
            .count()
    })
}

/// Weariness above which `faction` declares no war of its own (DP1: a
/// pretender to a throne waits less).
pub fn weariness_to_declare(data: &GameData, faction: &crate::state::FactionState) -> u32 {
    let most = data.ai_diplomacy.negotiation.max_weariness_to_declare;
    let pretender = faction.claims.iter().any(|c| c.kind == ClaimKind::Throne);
    if pretender {
        most + 20
    } else {
        most
    }
}

/// Answer of an ally to a call to arms, as shown by the war preview.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallForecast {
    /// The ally marches.
    Joins,
    /// Not crippled and not hostile (attitude within `call_forecast`
    /// `hesitate_floor`..=0): it stays out, but only just.
    Hesitates,
    /// The ally stays out, for the given reason.
    Refuses(&'static str),
}

/// Would `ally` answer the call to arms of `defender` attacked by
/// `aggressor` (M5 § 2.3, F4)? Vassals follow a loyal tie, overlords
/// protect their vassals, other allies march unless they resent the
/// defender or are crippled (empty treasury).
pub fn answers_call_to_arms(
    state: &CampaignState,
    data: &GameData,
    ally: &FactionId,
    defender: &FactionId,
    aggressor: &FactionId,
) -> bool {
    call_to_arms_forecast(state, data, ally, defender, aggressor) == CallForecast::Joins
}

/// Pure forecast behind [`answers_call_to_arms`] (WH `diploa`): same
/// decision, with the nuance and the reason of a refusal.
pub fn call_to_arms_forecast(
    state: &CampaignState,
    data: &GameData,
    ally: &FactionId,
    defender: &FactionId,
    aggressor: &FactionId,
) -> CallForecast {
    let Some(ally_state) = state.factions.get(ally) else {
        return CallForecast::Refuses("faction disparue");
    };
    if ally_state.suzerain.as_ref() == Some(defender) {
        return if crate::feudal::answers_host(state, data, ally, defender, aggressor) {
            CallForecast::Joins
        } else {
            CallForecast::Refuses("vassal peu loyal")
        };
    }
    if state
        .factions
        .get(defender)
        .is_some_and(|f| f.suzerain.as_ref() == Some(ally))
        || ally == &state.player_faction
    {
        return CallForecast::Joins;
    }
    let attitude = state.attitude(data, ally, defender).0;
    // EQ6: an exhausted realm stays out, as a ruined one.
    let exhausted = data.ai_diplomacy.join_war.weary_stay_out
        && data.ai_diplomacy.negotiation.enabled
        && ally_state.ledger.weariness > weariness_to_declare(data, ally_state);
    if ally_state.treasury < 0 {
        return CallForecast::Refuses("trésorerie vide");
    }
    if exhausted {
        return CallForecast::Refuses("épuisé par la guerre");
    }
    let grudge = rivals(state, ally).contains(aggressor);
    if attitude > 0 || (grudge && attitude > -20) {
        CallForecast::Joins
    } else if attitude >= data.diplomacy_rules.call_forecast.hesitate_floor {
        CallForecast::Hesitates
    } else {
        CallForecast::Refuses("rancune envers le défenseur")
    }
}

/// Diplomatic orders of an AI faction for this turn (spec § 2.3, F4).
pub fn plan_diplomacy(cache: &PlanCache, data: &GameData, faction: &FactionId) -> Vec<Order> {
    let state = cache.state();
    let mut orders = Vec::new();
    if faction.is_rebels() {
        return orders;
    }
    let Some(me) = state.factions.get(faction) else {
        return orders;
    };
    let slot = faction
        .as_str()
        .bytes()
        .fold(0u32, |acc, b| acc.wrapping_add(u32::from(b)));
    let turn = state.turn;
    let aggression = data
        .factions
        .get(faction)
        .and_then(|f| f.ai_personality.as_ref())
        .and_then(|p| p.aggression)
        .map_or(50, i32::from);

    // Peace: offer a white peace when we would accept one ourselves; when
    // clearly winning, ask for the occupied provinces; when losing,
    // cede what the enemy holds rather than lose everything (F4).
    let treaties = data.ai_diplomacy.negotiation.enabled;
    if treaties && ((turn + slot).is_multiple_of(2) || cornered(state, data, faction)) {
        orders.extend(crate::negotiation::plan_peace(cache, data, faction));
    }

    plan_alliances(cache, data, faction, slot, &mut orders);

    let war_rules = &data.ai_diplomacy.war;
    let rest = if war_rules.rest_turns == 0 {
        WAR_REST_TURNS
    } else {
        war_rules.rest_turns
    };
    let rested = me.last_war_declared.is_none_or(|t| t + rest <= turn);
    // RX equil: a realm already fighting its quota of enemies declares no
    // further war of its own, except a pretender pressing its main claim
    // (the Hundred Years' War does not wait for the quota).
    let overstretched = war_rules.max_enemies > 0
        && me.at_war_with.iter().filter(|e| !e.is_rebels()).count() as u32 >= war_rules.max_enemies
        && !crate::negotiation::pretender_ready(state, data, faction);
    // DP1: a weary realm opens no new front; a pretender waits less.
    let weary = treaties && me.ledger.weariness > weariness_to_declare(data, me);
    let able = turn >= 4
        && !weary
        && !overstretched
        && (war_ready(state, faction) || crate::negotiation::pretender_ready(state, data, faction));
    let ready = able && rested;
    // EQ6: the rest after another declaration does not hold back the war
    // for the main crown claimed.
    let main_first = data.ai_diplomacy.war.main_claim_first;
    let mut declared = false;
    // WH `diplob`: a refused ultimatum is followed by war, whatever the rest.
    if let Some(target) = ultimatum_war(state, faction) {
        orders.push(Order::DeclareWar { target });
        declared = true;
    }
    if !declared && able && (rested || main_first) && (turn + slot).is_multiple_of(2) {
        if let Some(target) = war_target(cache, data, faction, aggression, rested) {
            match ultimatum_step(cache, data, faction, &target) {
                UltimatumStep::NotApplicable => orders.push(Order::DeclareWar { target }),
                UltimatumStep::Send(order) => orders.push(order),
                UltimatumStep::Wait => {}
            }
            declared = true;
        }
    }
    if ready && !declared && (turn + slot) % 2 == 1 {
        if let Some(target) = ally_war_to_join(cache, data, faction) {
            orders.push(Order::DeclareWar { target });
            declared = true;
        }
    }
    if ready && !declared {
        orders.extend(desert_losing_suzerain(cache, data, faction, slot));
    }

    // Lift pointless embargoes, drop hated allies.
    for target in &me.embargoes {
        if !state.is_at_war(faction, target) && cache.attitude(data, faction, target).0 > 10 {
            orders.push(Order::SetEmbargo {
                target: target.clone(),
                active: false,
            });
        }
    }
    let hated: BTreeSet<FactionId> = me
        .allies
        .iter()
        .filter(|a| state.relation(faction, a) == RelationKind::Alliance)
        .filter(|a| cache.attitude(data, faction, a).0 < -30)
        .cloned()
        .collect();
    for target in hated {
        orders.push(Order::BreakAlliance { target });
    }

    // Excommunicated and rich: buy back the Pope's favour.
    if religion::is_excommunicated(state, faction) && me.treasury > 8000 {
        orders.push(Order::DonateToChurch { amount: 2000 });
    }
    orders
}

/// A crown down to `peace.cornered_provinces` provinces of its own (or
/// fewer) sues for peace every season, whatever the war score (G5: the
/// Scots after Halidon Hill treat rather than vanish).
pub fn is_cornered(state: &CampaignState, data: &GameData, faction: &FactionId) -> bool {
    cornered(state, data, faction)
}

fn cornered(state: &CampaignState, data: &GameData, faction: &FactionId) -> bool {
    let most = data.ai_diplomacy.peace.cornered_provinces;
    most > 0
        && state
            .provinces
            .keys()
            .filter(|id| {
                state.controls_province(faction, id) && state.province_owner(id) == Some(faction)
            })
            .count()
            <= most
}

/// EQ6: the main crown `faction` claims: the living realm with the most
/// provinces among those whose throne it claims (England: France, not a
/// small Italian lordship inherited through a marriage).
pub fn main_claim(state: &CampaignState, faction: &FactionId) -> Option<FactionId> {
    let me = state.factions.get(faction)?;
    let thrones: BTreeSet<&FactionId> = me
        .claims
        .iter()
        .filter(|c| c.kind == ClaimKind::Throne)
        .filter_map(|c| c.faction.as_ref())
        .filter(|f| *f != faction && state.factions.get(*f).is_some_and(|s| s.alive))
        .collect();
    thrones
        .into_iter()
        .map(|f| {
            let size = state
                .provinces
                .keys()
                .filter(|p| state.province_owner(p) == Some(f))
                .count();
            (size, f)
        })
        .max_by(|a, b| a.0.cmp(&b.0).then_with(|| b.1.cmp(a.1)))
        .map(|(_, f)| f.clone())
}

/// The war `faction` declares this turn, if any: a pretender presses its
/// claim even against a stronger crown when it has allies or a bridgehead;
/// aggressive realms fall on weaker rivals they hold a casus belli against.
/// When `faction` is not `rested` (it declared another war lately), only
/// its main claim is considered (EQ6, `war.main_claim_first`).
fn war_target(
    cache: &PlanCache,
    data: &GameData,
    faction: &FactionId,
    aggression: i32,
    rested: bool,
) -> Option<FactionId> {
    let state = cache.state();
    let my_power = cache.coalition_power(faction);
    let rules = &data.ai_diplomacy.war;
    // Never a new front while the current wars weigh (DP1: a pretender
    // to a throne tolerates a heavier border war, as Edward III kept
    // fighting the Scots while claiming France).
    let pressing = enemy_power(cache, data, faction);
    let own = cache.faction_power(faction);
    let pretender = data.ai_diplomacy.negotiation.enabled
        && state.factions[faction]
            .claims
            .iter()
            .any(|c| c.kind == ClaimKind::Throne);
    let share = if pretender {
        rules.front_share * 2.0
    } else {
        rules.front_share
    };
    if pressing > share * own {
        return None;
    }
    let has_allies = state.factions[faction]
        .allies
        .iter()
        .any(|a| state.factions.get(a).is_some_and(|f| f.alive));
    let main = if rules.main_claim_first || rules.claim_war_ignores_difficulty {
        main_claim(state, faction)
    } else {
        None
    };
    state
        .factions
        .iter()
        .filter(|(id, f)| {
            *id != faction
                && f.alive
                && !id.is_rebels()
                && id.as_str() != PAPACY_FACTION
                && !state.is_allied(faction, id)
                && !state.is_at_war(faction, id)
                && !state.has_truce(faction, id)
                && !state.has_non_aggression(faction, id)
                && (rested || main.as_ref() == Some(*id))
        })
        .filter_map(|(id, _)| {
            let stakes = claim_stakes(state, faction, id);
            let is_main = main.as_ref() == Some(id);
            // EQ6: the main claim war does not depend on the difficulty.
            let neutral = is_main && rules.claim_war_ignores_difficulty;
            // DF1: a harder campaign lowers the odds an AI wants before
            // falling on the player.
            let demand = if neutral {
                1.0
            } else {
                state.difficulty_war_ratio_factor(data, id)
            };
            let attitude = cache.attitude(data, faction, id).0
                - if neutral {
                    state.difficulty_attitude(data, faction, id)
                } else {
                    0
                }
                // EQ6: kinship is the ground of the claim, not a restraint.
                - if is_main && rules.claim_war_ignores_kinship {
                    state.kinship_attitude(data, faction, id)
                } else {
                    0
                };
            // EQ6: the main crown outranks every lesser claim.
            let main_bonus = if is_main && rules.main_claim_first {
                CLAIM_WAR_PRIORITY
            } else {
                0.0
            };
            if stakes.any() && aggression >= PRETENDER_AGGRESSION {
                let ratio = my_power / cache.faction_power(id).max(1.0);
                let supported = has_allies || cache.are_neighbors(data, faction, id);
                let needed = demand
                    * if supported {
                        rules.pretender_ratio
                    } else {
                        rules.pretender_ratio_alone
                    };
                let weight = if stakes.throne { 3.0 } else { 0.0 } + stakes.provinces as f64;
                return (ratio >= needed && attitude < 20)
                    // A claim outranks any opportunistic war.
                    .then(|| (id.clone(), CLAIM_WAR_PRIORITY + main_bonus + weight + ratio));
            }
            if aggression >= OPPORTUNIST_AGGRESSION
                && state.casus_belli(data, faction, id).is_some()
                && cache.attitude(data, faction, id).0 < 0
            {
                let ratio = my_power / cache.coalition_power(id).max(1.0);
                // WH `diplob`: the league finds the hegemon easier to attack.
                let league = &data.ai_diplomacy.league;
                if state.league_target_of(data, faction) == Some(id) {
                    return (ratio >= OPPORTUNIST_RATIO * demand * league.war_ratio_factor)
                        .then(|| (id.clone(), ratio / league.war_ratio_factor.max(0.1)));
                }
                return (ratio >= OPPORTUNIST_RATIO * demand).then(|| (id.clone(), ratio));
            }
            None
        })
        .max_by(|a, b| a.1.total_cmp(&b.1).then_with(|| b.0.cmp(&a.0)))
        .map(|(id, _)| id)
}

/// Seasons after a refused ultimatum during which the war it promised may
/// still be declared (afterwards the matter is forgotten).
const ULTIMATUM_WAR_WINDOW: u32 = 4;

/// What an AI that wants war on the player does first (WH `diplob`).
enum UltimatumStep {
    /// Declare war at once (not the player, off, or too weak to bully).
    NotApplicable,
    /// Send this demand; war follows its refusal.
    Send(Order),
    /// An ultimatum is pending or was answered lately: no war yet.
    Wait,
}

/// The target `faction` declares war on because it refused its ultimatum.
fn ultimatum_war(state: &CampaignState, faction: &FactionId) -> Option<FactionId> {
    let me = state.factions.get(faction)?;
    let player = &state.player_faction;
    let refused = *me.ledger.ultimatum_refused.get(player)?;
    let open = state.turn > refused
        && state.turn <= refused + ULTIMATUM_WAR_WINDOW
        && state.factions.get(player).is_some_and(|f| f.alive)
        && !state.is_at_war(faction, player)
        && !state.is_allied(faction, player)
        && !state.has_truce(faction, player);
    open.then(|| player.clone())
}

/// Before a war on the player, a strong AI demands a province or a tribute
/// (« cédez ou ce sera la guerre », ADR 0283).
fn ultimatum_step(
    cache: &PlanCache,
    data: &GameData,
    faction: &FactionId,
    target: &FactionId,
) -> UltimatumStep {
    let state = cache.state();
    let rules = &data.ai_diplomacy.ultimatum;
    if !rules.enabled || target != &state.player_faction {
        return UltimatumStep::NotApplicable;
    }
    if cache.coalition_power(faction)
        < rules.min_power_ratio * cache.coalition_power(target).max(1.0)
    {
        return UltimatumStep::NotApplicable;
    }
    let sent = state
        .factions
        .get(faction)
        .and_then(|f| f.ledger.ultimatum_sent.get(target))
        .copied();
    if sent.is_some_and(|t| t + rules.cooldown_turns > state.turn) {
        return UltimatumStep::Wait;
    }
    match ultimatum_demand(cache, data, faction, target) {
        Some(articles) => UltimatumStep::Send(Order::ProposeTreaty {
            target: target.clone(),
            articles,
        }),
        None => UltimatumStep::NotApplicable,
    }
}

/// A border province the AI claims and the player holds, else a tribute.
fn ultimatum_demand(
    cache: &PlanCache,
    data: &GameData,
    faction: &FactionId,
    target: &FactionId,
) -> Option<Vec<Article>> {
    let state = cache.state();
    let rules = &data.ai_diplomacy.ultimatum;
    let claimed = claimed_provinces(state, faction);
    let capital = state.factions.get(target).map(|f| f.capital.clone());
    let province = claimed
        .iter()
        .filter(|p| state.province_owner(p) == Some(target) && capital.as_ref() != Some(*p))
        .find(|p| {
            crate::movement::land_neighbors(data, p)
                .iter()
                .any(|n| state.province_owner(n) == Some(faction))
        });
    let articles = match province {
        Some(province) => vec![Article::CedeProvince {
            giver: crate::negotiation::Party::Recipient,
            province: province.clone(),
        }],
        None => {
            let income = cache.faction_income(data, target).max(0);
            let per_season = income * rules.tribute_income_percent / 100;
            if per_season <= 0 {
                return None;
            }
            vec![Article::Tribute {
                giver: crate::negotiation::Party::Recipient,
                per_season,
                seasons: rules.tribute_seasons,
            }]
        }
    };
    crate::negotiation::check_treaty(state, data, faction, target, &articles)
        .is_ok()
        .then_some(articles)
}

/// An ally's war `faction` joins (co-belligerence, F4): the Low Countries
/// follow Edward III into France, Scotland falls on the English border.
fn ally_war_to_join(cache: &PlanCache, data: &GameData, faction: &FactionId) -> Option<FactionId> {
    let state = cache.state();
    let me = state.factions.get(faction)?;
    let front_share = data.ai_diplomacy.war.front_share;
    let rules = &data.ai_diplomacy.join_war;
    if enemy_power(cache, data, faction) > front_share * cache.faction_power(faction) {
        return None;
    }
    let my_power = cache.faction_power(faction);
    for ally in &me.allies {
        let Some(ally_state) = state.factions.get(ally) else {
            continue;
        };
        // WH `diplob`: a defensive alliance does not follow offensive wars.
        if !ally_state.alive
            || state.is_defensive_alliance(faction, ally)
            || cache.attitude(data, faction, ally).0 <= rules.min_attitude
            || cache.faction_power(ally) < rules.min_ally_power_ratio * my_power
        {
            continue;
        }
        for enemy in ally_state.at_war_with.iter().filter(|e| !e.is_rebels()) {
            if enemy == faction
                || state.is_allied(faction, enemy)
                || state.is_at_war(faction, enemy)
                || state.has_truce(faction, enemy)
                || state.has_non_aggression(faction, enemy)
                || !state.factions.get(enemy).is_some_and(|f| f.alive)
            {
                continue;
            }
            // A border suffices for a war of claims (Scotland falls on the
            // English border while Edward III claims France); a border
            // quarrel of our ally's does not spread along every border.
            let claim_war =
                claim_stakes(state, ally, enemy).any() || claim_stakes(state, enemy, ally).any();
            let border = cache.are_neighbors(data, faction, enemy)
                && (claim_war || !rules.border_only_claim_wars);
            let reachable = border || claim_stakes(state, faction, enemy).any();
            let ratio = cache.coalition_power(faction) / cache.coalition_power(enemy).max(1.0);
            if reachable && ratio >= rules.ratio {
                return Some(enemy.clone());
            }
        }
    }
    None
}

/// Alliances against our rivals (F4): partners sharing a rival, or fearing
/// it more than they like it (the counterweight of distant England for the
/// Low Countries, of France for Scotland).
fn plan_alliances(
    cache: &PlanCache,
    data: &GameData,
    faction: &FactionId,
    slot: u32,
    orders: &mut Vec<Order>,
) {
    let state = cache.state();
    // WH `diplob`: a faction standing in the league looks for partners
    // twice as often, and counts the hegemon among its rivals.
    let in_league = state.league_target_of(data, faction).is_some();
    let period = if in_league { 2 } else { 4 };
    let most = if in_league {
        data.ai_diplomacy.league.max_alliances.min(MAX_ALLIANCES)
    } else {
        MAX_ALLIANCES
    };
    if (state.turn + slot) % period != 1 % period || alliance_count(state, faction) >= most {
        return;
    }
    let rivals_of = |id: &FactionId| {
        let mut rivals = cache.rivals(id);
        rivals.extend(state.league_target_of(data, id).cloned());
        rivals
    };
    let my_rivals = rivals_of(faction);
    if my_rivals.is_empty() {
        return;
    }
    let candidate = state
        .factions
        .iter()
        .filter(|(id, f)| {
            *id != faction
                && f.alive
                && !id.is_rebels()
                && id.as_str() != PAPACY_FACTION
                && !state.is_allied(faction, id)
                && !state.is_at_war(faction, id)
                && !my_rivals.contains(*id)
                && f.allies.iter().all(|a| !my_rivals.contains(a))
        })
        .filter(|(id, _)| {
            let theirs = rivals_of(id);
            !theirs.is_disjoint(&my_rivals)
                || my_rivals.iter().any(|r| {
                    let towards_rival = cache.attitude(data, id, r).0;
                    towards_rival < 0 && cache.attitude(data, id, faction).0 > towards_rival + 10
                })
        })
        .map(|(id, _)| (id.clone(), cache.attitude(data, id, faction).0))
        .filter(|(id, _)| {
            let floor = if state.league_peers(data, faction, id) {
                data.ai_diplomacy.league.partner_attitude
            } else {
                0
            };
            cache.attitude(data, faction, id).0 > floor
        })
        .filter_map(|(id, liking)| {
            if id == state.player_faction {
                return Some((id, liking, Article::Alliance));
            }
            let accepts = |article: &Article| {
                crate::negotiation::evaluate_treaty_with(
                    cache,
                    data,
                    faction,
                    &id,
                    std::slice::from_ref(article),
                )
                .accept
            };
            // The league binds its members by a defensive pact only (a
            // military alliance each would drag the whole map into war,
            // ADR 0283).
            let article = if state.league_peers(data, faction, &id) {
                Article::DefensiveAlliance
            } else {
                Article::Alliance
            };
            accepts(&article).then_some((id, liking, article))
        })
        .max_by(|a, b| a.1.cmp(&b.1).then_with(|| b.0.cmp(&a.0)));
    if let Some((target, _, article)) = candidate {
        orders.push(match article {
            Article::Alliance => Order::ProposeAlliance { target },
            other => Order::ProposeTreaty {
                target,
                articles: vec![other],
            },
        });
    }
}

/// An opportunistic vassal (Burgundy) deserts a suzerain that is losing to
/// a stronger coalition: white peace with the winner, then independence.
fn desert_losing_suzerain(
    cache: &PlanCache,
    data: &GameData,
    faction: &FactionId,
    slot: u32,
) -> Vec<Order> {
    let state = cache.state();
    let me = &state.factions[faction];
    let Some(lord) = me.suzerain.clone() else {
        return Vec::new();
    };
    if (state.turn + slot) % 4 != 3 || me.loyalty >= 50 || faction == &state.player_faction {
        return Vec::new();
    }
    let winner = state.factions[&lord]
        .at_war_with
        .iter()
        .filter(|e| !e.is_rebels() && *e != faction)
        .find(|e| {
            state.war_score(data, &lord, e) <= DESERTION_WAR_SCORE
                && cache.coalition_power(e) > cache.coalition_power(&lord)
                && cache.attitude(data, faction, e).0 > -40
        })
        .cloned();
    let Some(winner) = winner else {
        return Vec::new();
    };
    let mut orders = Vec::new();
    if state.is_at_war(faction, &winner) {
        if winner == state.player_faction
            || !crate::negotiation::evaluate_treaty(
                state,
                data,
                faction,
                &winner,
                &[Article::Peace],
            )
            .accept
        {
            return Vec::new();
        }
        orders.push(Order::ProposePeace {
            target: winner.clone(),
            provinces: Vec::new(),
            tribute: 0,
        });
    }
    orders.push(Order::DeclareWar { target: lord });
    orders
}
