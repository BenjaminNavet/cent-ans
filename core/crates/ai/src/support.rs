//! Subsidies between allies (G2): a rich ally at war against the same enemy
//! pays the debts of an indebted one (French gold for the Scots).

use data_model::{FactionId, GameData};
use sim_campaign::diplomacy::rivals;
use sim_campaign::{CampaignState, Order};

/// Seasons of the ally's deficit the subsidy covers ahead.
pub const SUBSIDY_DEFICIT_SEASONS: i64 = 2;
/// The donor keeps a season of upkeep and gives at most this fraction
/// (1/n) of the rest per turn.
pub const SUBSIDY_SPARE_DIVISOR: i64 = 3;
/// Only a realm this many times richer (gross income) pays subsidies.
pub const SUBSIDY_WEALTH_RATIO: i64 = 3;
/// Smallest subsidy worth a courier.
pub const SUBSIDY_MIN: i64 = 100;

/// What `ally` needs to stay solvent for a couple of seasons (0: nothing).
pub fn subsidy_need(state: &CampaignState, ally: &FactionId) -> i64 {
    state.factions.get(ally).map_or(0, |f| {
        let deficit = (f.last_budget.upkeep() - f.last_budget.income).max(0);
        (SUBSIDY_DEFICIT_SEASONS * deficit - f.treasury).max(0)
    })
}

/// Subsidies `faction` pays this turn out of `spare` (money beyond its own
/// reserve): allies sharing one of our rivals, neediest first, each gift capped by what is left.
pub fn plan_subsidies(
    state: &CampaignState,
    data: &GameData,
    faction: &FactionId,
    spare: i64,
) -> Vec<Order> {
    let wealth = |f: &FactionId| state.faction_income(data, f).max(0);
    let Some(me) = state.factions.get(faction) else {
        return Vec::new();
    };
    // Our enemies and pretenders (or the realms we claim): the Scots and the
    // French share England, in war as in truce.
    let my_rivals = rivals(state, faction);
    // DC3: the incomes (a walk over every place) are weighed last, on the few
    // needy allies left, and ours only once.
    let mut my_wealth = None;
    let mut needy: Vec<(i64, FactionId)> = me
        .allies
        .iter()
        .filter(|a| state.factions.get(*a).is_some_and(|f| f.alive))
        .filter(|a| !rivals(state, a).is_disjoint(&my_rivals))
        .map(|a| (subsidy_need(state, a), a.clone()))
        .filter(|(need, _)| *need >= SUBSIDY_MIN)
        .filter(|(_, a)| {
            let mine = *my_wealth.get_or_insert_with(|| wealth(faction));
            mine >= SUBSIDY_WEALTH_RATIO * wealth(a)
        })
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
