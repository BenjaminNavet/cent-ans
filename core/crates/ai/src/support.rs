//! Subsidies between allies (G2): a rich ally at war against the same enemy
//! pays the debts of an indebted one (French gold for the Scots).

use data_model::{AiCampaign, FactionId, GameData};

use sim_campaign::plan_cache::PlanCache;
use sim_campaign::{CampaignState, Order};

// B1: the subsidy constants (deficit seasons, spare divisor, wealth ratio,
// minimum) live in `data/ai/campaign.json` (`tuning`).

/// What `ally` needs to stay solvent for a couple of seasons (0: nothing).
pub fn subsidy_need(state: &CampaignState, ally: &FactionId) -> i64 {
    state.factions.get(ally).map_or(0, |f| {
        let deficit = (f.last_budget.upkeep() - f.last_budget.income).max(0);
        (AiCampaign::bundled().tuning.subsidy_deficit_seasons * deficit - f.treasury).max(0)
    })
}

/// Subsidies `faction` pays this turn out of `spare` (money beyond its own
/// reserve): allies sharing one of our rivals, neediest first, each gift capped by what is left.
pub fn plan_subsidies(
    cache: &PlanCache,
    data: &GameData,
    faction: &FactionId,
    spare: i64,
) -> Vec<Order> {
    let state = cache.state();
    let wealth = |f: &FactionId| cache.faction_income(data, f).max(0);
    let Some(me) = state.factions.get(faction) else {
        return Vec::new();
    };
    // Our enemies and pretenders (or the realms we claim): the Scots and the
    // French share England, in war as in truce.
    let my_rivals = cache.rivals(faction);
    // DC3: the incomes (a walk over every place) are weighed last, on the few
    // needy allies left, and ours only once.
    let mut my_wealth = None;
    let mut needy: Vec<(i64, FactionId)> = me
        .allies
        .iter()
        .filter(|a| state.factions.get(*a).is_some_and(|f| f.alive))
        .filter(|a| !cache.rivals(a).is_disjoint(&my_rivals))
        .map(|a| (subsidy_need(state, a), a.clone()))
        .filter(|(need, _)| *need >= AiCampaign::bundled().tuning.subsidy_min)
        .filter(|(_, a)| {
            let mine = *my_wealth.get_or_insert_with(|| wealth(faction));
            mine >= AiCampaign::bundled().tuning.subsidy_wealth_ratio * wealth(a)
        })
        .collect();
    needy.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));
    let mut left = spare;
    let mut orders = Vec::new();
    for (need, target) in needy {
        let amount = need.min(left);
        if amount < AiCampaign::bundled().tuning.subsidy_min {
            break;
        }
        left -= amount;
        orders.push(Order::SendGift { target, amount });
    }
    orders
}
