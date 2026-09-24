//! Subsidies between allies (G2): a rich ally at war against the same enemy
//! pays the debts of an indebted one (French gold for the Scots).

use data_model::FactionId;
use sim_campaign::{CampaignState, Order};

/// Seasons of the ally's deficit the subsidy covers ahead.
pub const SUBSIDY_DEFICIT_SEASONS: i64 = 2;
/// Seasons of gross income the donor keeps before paying anyone.
pub const SUBSIDY_DONOR_RESERVE_SEASONS: i64 = 2;
/// Smallest subsidy worth a courier.
pub const SUBSIDY_MIN: i64 = 100;

/// What `ally` needs to stay solvent for a couple of seasons (0: nothing).
pub fn subsidy_need(state: &CampaignState, ally: &FactionId) -> i64 {
    state.factions.get(ally).map_or(0, |f| {
        let deficit = (f.upkeep_last_turn - f.income_last_turn).max(0);
        (SUBSIDY_DEFICIT_SEASONS * deficit - f.treasury).max(0)
    })
}

/// Subsidies `faction` pays this turn out of `spare` (money beyond its own
/// reserve): allies (not rebels) at war against one of our enemies, neediest
/// first, each gift capped by what is left.
pub fn plan_subsidies(state: &CampaignState, faction: &FactionId, spare: i64) -> Vec<Order> {
    let Some(me) = state.factions.get(faction) else {
        return Vec::new();
    };
    let mut needy: Vec<(i64, FactionId)> = me
        .allies
        .iter()
        .filter(|a| state.factions.get(*a).is_some_and(|f| f.alive))
        .filter(|a| {
            state.factions[*a]
                .at_war_with
                .iter()
                .any(|e| e.as_str() != "fac_rebels" && me.at_war_with.contains(e))
        })
        .map(|a| (subsidy_need(state, a), a.clone()))
        .filter(|(need, _)| *need >= SUBSIDY_MIN)
        .collect();
    needy.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));
    let mut left = spare;
    let mut orders = Vec::new();
    for (need, target) in needy {
        let amount = need.min(left);
        if amount < SUBSIDY_MIN {
            break;
        }
        left -= amount;
        orders.push(Order::SendGift { target, amount });
    }
    orders
}
