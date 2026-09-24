//! Historical side changes (G2): Flanders ruined by the wool embargo turns
//! to England (Artevelde, 1338-1340); Burgundy abandons a beaten France for
//! the dominant invader (Troyes, 1420).

use data_model::{FactionId, GameData};
use sim_campaign::diplomacy::{
    claim_stakes, evaluate, war_ready, Proposal, RelationKind, DESERTION_WAR_SCORE, MAX_ALLIANCES,
};
use sim_campaign::{CampaignState, Order};

/// Below this loyalty a vassal ruined by a wool embargo turns to the
/// embargoing crown.
pub const WOOL_REVOLT_LOYALTY: u8 = 60;
/// Provinces of a crown's 1337 realm an invader must control to dominate it.
pub const DOMINANCE_PROVINCES: usize = 8;
/// Below this loyalty an opportunistic vassal or ally changes sides when
/// the invader dominates its suzerain's realm.
pub const DEFECTION_LOYALTY: u8 = 90;
/// Share of its 1337 realm a crown in trouble has lost.
pub const CROWN_IN_TROUBLE_LOSS: f64 = 0.25;

/// Chance (‰) that a campaign sees the wool revolt, a dynastic alliance or
/// a defection at all: history hesitates, so do seeds.
pub const HISTORY_PERMILLE: u64 = 500;

/// Pure per-campaign roll in 0..1000 for `faction` and `salt` (splitmix64
/// of the seed): the AI stays deterministic without touching the RNG.
pub fn campaign_roll(state: &CampaignState, faction: &FactionId, salt: u64) -> u64 {
    let mut x = state.seed ^ salt.wrapping_mul(0x9E37_79B9_7F4A_7C15);
    for b in faction.as_str().bytes() {
        x = x.rotate_left(5) ^ u64::from(b);
    }
    x = (x ^ (x >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    (x ^ (x >> 31)) % 1000
}

/// Planning slot of `faction` (same as `plan_diplomacy`'s).
fn slot(faction: &FactionId) -> u32 {
    faction
        .as_str()
        .bytes()
        .fold(0u32, |a, b| a.wrapping_add(u32::from(b)))
}

/// Provinces of `crown`'s 1337 realm (`data`) controlled by `holder`.
pub fn realm_held_by(
    state: &CampaignState,
    data: &GameData,
    crown: &FactionId,
    holder: &FactionId,
) -> usize {
    data.provinces
        .iter()
        .filter(|(_, p)| &p.owner == crown)
        .filter(|(id, _)| {
            state
                .provinces
                .get(*id)
                .is_some_and(|p| &p.controller == holder)
        })
        .count()
}

/// Does `invader` dominate `crown`'s realm: it holds the 1337 capital or
/// [`DOMINANCE_PROVINCES`] of its provinces, while `crown` has lost a
/// quarter of them or is beaten.
pub fn dominates_realm(
    state: &CampaignState,
    data: &GameData,
    invader: &FactionId,
    crown: &FactionId,
) -> bool {
    let Some(capital) = data.factions.get(crown).map(|f| &f.capital) else {
        return false;
    };
    let seat = state
        .provinces
        .get(capital)
        .is_some_and(|p| &p.controller == invader);
    let held = realm_held_by(state, data, crown, invader);
    let realm = data
        .provinces
        .values()
        .filter(|p| &p.owner == crown)
        .count();
    let kept = realm_held_by(state, data, crown, crown);
    let in_trouble = (kept as f64) < (1.0 - CROWN_IN_TROUBLE_LOSS) * realm as f64
        || state.war_score(data, crown, invader) <= DESERTION_WAR_SCORE;
    (seat || held >= DOMINANCE_PROVINCES) && in_trouble
}

/// The patron `faction` abandons and the crown it turns to, if any: a
/// vassal under the embargo of its lord's enemy (wool revolt), or a vassal
/// or ally of a crown whose realm the enemy dominates (Troyes).
pub fn side_change(
    state: &CampaignState,
    data: &GameData,
    faction: &FactionId,
) -> Option<(FactionId, FactionId)> {
    let me = state.factions.get(faction)?;
    let alive = |f: &FactionId| state.factions.get(f).is_some_and(|s| s.alive);
    if let Some(lord) = me
        .suzerain
        .clone()
        .filter(|_| me.loyalty < WOOL_REVOLT_LOYALTY)
        .filter(|_| campaign_roll(state, faction, 1) < HISTORY_PERMILLE)
    {
        let embargoer = state
            .factions
            .iter()
            .find(|(id, f)| f.alive && f.embargoes.contains(faction) && state.is_at_war(&lord, id));
        if let Some((embargoer, _)) = embargoer {
            return Some((lord, embargoer.clone()));
        }
    }
    let patrons: Vec<FactionId> = match &me.suzerain {
        Some(lord) if me.loyalty < DEFECTION_LOYALTY => vec![lord.clone()],
        Some(_) => Vec::new(),
        None => me.allies.iter().filter(|a| alive(a)).cloned().collect(),
    };
    // One roll per decade: the Armagnac-Burgundian feud is not written.
    if campaign_roll(state, faction, 2 + u64::from(state.turn / 40)) >= HISTORY_PERMILLE {
        return None;
    }
    patrons.into_iter().find_map(|patron| {
        let invader = state.factions[&patron]
            .at_war_with
            .iter()
            .filter(|e| e.as_str() != "fac_rebels" && *e != faction && alive(e))
            // Never towards a crown that claims our lands (or we its).
            .filter(|e| !claim_stakes(state, e, faction).any())
            .filter(|e| !claim_stakes(state, faction, e).any())
            .find(|e| dominates_realm(state, data, e, &patron))?
            .clone();
        Some((patron, invader))
    })
}

/// Orders of `faction` changing sides this turn (G2): white peace with the
/// new patron first, then independence war (vassal) or broken alliance,
/// and an alliance offer to the new patron.
pub fn plan_side_change(state: &CampaignState, data: &GameData, faction: &FactionId) -> Vec<Order> {
    if faction == &state.player_faction || !(state.turn + slot(faction)).is_multiple_of(4) {
        return Vec::new();
    }
    let Some((patron, invader)) = side_change(state, data, faction) else {
        return Vec::new();
    };
    let white = Proposal::Peace {
        provinces: Vec::new(),
        tribute: 0,
    };
    if state.is_at_war(faction, &invader) {
        let accepted = invader != state.player_faction
            && evaluate(state, data, faction, &invader, &white).accept;
        return if accepted {
            vec![Order::ProposePeace {
                target: invader,
                provinces: Vec::new(),
                tribute: 0,
            }]
        } else {
            Vec::new()
        };
    }
    let mut orders = Vec::new();
    let me = &state.factions[faction];
    if me.suzerain.as_ref() == Some(&patron) {
        if !war_ready(state, faction) {
            return orders;
        }
        orders.push(Order::DeclareWar { target: patron });
    } else if state.is_allied(faction, &patron) {
        orders.push(Order::BreakAlliance { target: patron });
    }
    if !state.is_allied(faction, &invader) {
        orders.push(Order::ProposeAlliance { target: invader });
    }
    orders
}

/// Attitude margin a courted prince needs towards us over our enemy.
pub const DYNASTIC_MARGIN: i32 = 10;
/// Attitude towards one of our allies that makes a prince its in-law.
pub const DYNASTIC_IN_LAW_ATTITUDE: i32 = 20;

/// Alliance `faction`, at war, offers a prince who prefers it to its enemy
/// by [`DYNASTIC_MARGIN`] (G2: Edward III's in-laws of Hainaut, Brabant),
/// in about half the campaigns for each prince.
pub fn plan_dynastic_alliance(
    state: &CampaignState,
    data: &GameData,
    faction: &FactionId,
) -> Option<Order> {
    let me = state.factions.get(faction)?;
    // Off the turns of `plan_diplomacy`'s alliances ((turn + slot) % 4 == 1).
    if faction == &state.player_faction || (state.turn + slot(faction)) % 4 != 3 {
        return None;
    }
    let enemies: Vec<&FactionId> = me
        .at_war_with
        .iter()
        .filter(|e| e.as_str() != "fac_rebels")
        .collect();
    let alliances = me
        .allies
        .iter()
        .filter(|a| state.relation(faction, a) == RelationKind::Alliance)
        .count();
    if enemies.is_empty() || alliances >= MAX_ALLIANCES {
        return None;
    }
    state
        .factions
        .iter()
        .filter(|(id, f)| {
            *id != faction && f.alive && f.suzerain.is_none() && id.as_str() != "fac_rebels"
        })
        .filter(|(id, f)| !state.is_allied(faction, id) && !f.at_war_with.contains(faction))
        .filter(|(_, f)| enemies.iter().all(|e| !f.allies.contains(*e)))
        .filter(|(id, _)| campaign_roll(state, id, 7) < HISTORY_PERMILLE)
        .filter_map(|(id, _)| {
            let towards_us = state.attitude(data, id, faction).0;
            let towards_enemy = enemies
                .iter()
                .map(|e| state.attitude(data, id, e).0)
                .max()?;
            // In-laws of our allies need less (Hainaut draws Brabant).
            let in_law = me
                .allies
                .iter()
                .any(|a| state.attitude(data, id, a).0 >= DYNASTIC_IN_LAW_ATTITUDE);
            let margin = if in_law { 0 } else { DYNASTIC_MARGIN };
            (towards_us > towards_enemy + margin && towards_us >= 20)
                .then(|| (towards_us, id.clone()))
        })
        .filter(|(_, id)| evaluate(state, data, faction, id, &Proposal::Alliance).accept)
        .max_by(|a, b| a.0.cmp(&b.0).then_with(|| b.1.cmp(&a.1)))
        .map(|(_, target)| Order::ProposeAlliance { target })
}

/// Embargoes `faction` lifts on realms now fighting its enemies (the wool
/// returns to Ghent once Flanders turns on France); the attitude case is
/// left to `plan_diplomacy`.
pub fn lift_embargoes_on_cobelligerents(
    state: &CampaignState,
    data: &GameData,
    faction: &FactionId,
) -> Vec<Order> {
    let Some(me) = state.factions.get(faction) else {
        return Vec::new();
    };
    me.embargoes
        .iter()
        .filter(|t| !state.is_at_war(faction, t) && state.attitude(data, faction, t).0 <= 10)
        .filter(|t| {
            state.factions.get(*t).is_some_and(|f| {
                f.at_war_with
                    .iter()
                    .any(|e| e.as_str() != "fac_rebels" && me.at_war_with.contains(e))
            })
        })
        .map(|t| Order::SetEmbargo {
            target: t.clone(),
            active: false,
        })
        .collect()
}
