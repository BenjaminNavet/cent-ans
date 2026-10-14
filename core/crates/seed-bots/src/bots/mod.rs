//! The eight bots of the spec.

use data_model::GameData;
use sim_campaign::rng::CampaignRng;
use sim_campaign::{MoveOrderTarget, Order};

use crate::bot::Bot;
use crate::candidates::Candidate;
use crate::view::BotView;

pub mod aggressor;
pub mod builder;
pub mod chaos;
pub mod diplomat;
pub mod mirror;
pub mod random_legal;
pub mod sailor;
pub mod turtle;

/// Most orders any bot submits in one turn.
pub const MAX_ORDERS: u32 = 10;

/// Draws `1..=max_orders` candidates without replacement, each pick
/// proportional to `weight` (weight 0 = never). Deterministic given `rng`.
pub(crate) fn draw_weighted(
    mut pool: Vec<Candidate>,
    weight: impl Fn(&Candidate) -> u32,
    max_orders: u32,
    rng: &mut CampaignRng,
) -> Vec<Order> {
    let max_orders = max_orders.clamp(1, MAX_ORDERS);
    let wanted = 1 + rng.below(max_orders) as usize;
    let mut weights: Vec<u32> = pool.iter().map(&weight).collect();
    let mut orders = Vec::new();
    while orders.len() < wanted {
        let total: u64 = weights.iter().map(|w| u64::from(*w)).sum();
        if total == 0 {
            break;
        }
        let mut ticket = rng.next_u64() % total;
        let index = weights
            .iter()
            .position(|w| {
                let w = u64::from(*w);
                if ticket < w {
                    true
                } else {
                    ticket -= w;
                    false
                }
            })
            .unwrap_or(0);
        weights.remove(index);
        orders.push(pool.remove(index).order);
    }
    orders
}

/// Map point a `MoveArmy` order heads for, when it is a plain point.
pub(crate) fn move_point(order: &Order) -> Option<(&sim_campaign::ArmyId, [f32; 2])> {
    match order {
        Order::MoveArmy {
            army,
            target: MoveOrderTarget::Point { x, y },
        } => Some((army, [*x, *y])),
        _ => None,
    }
}

/// Current map point of one of the bot's armies.
pub(crate) fn army_point(
    view: &BotView,
    data: &GameData,
    army: &sim_campaign::ArmyId,
) -> Option<[f32; 2]> {
    let entry = view.state.armies.get(army)?;
    Some(view.state.army_cell(data, entry).center(data.navgrid()))
}

/// Squared distance between two map points.
pub(crate) fn dist2(a: [f32; 2], b: [f32; 2]) -> f32 {
    (a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2)
}

/// The point of `points` nearest to `from`.
pub(crate) fn nearest(from: [f32; 2], points: &[[f32; 2]]) -> Option<[f32; 2]> {
    points
        .iter()
        .copied()
        .min_by(|a, b| dist2(from, *a).total_cmp(&dist2(from, *b)))
}

/// Whether a `MoveArmy` point order brings its army closer to the nearest
/// of `targets` than it stands now (false when unknown).
pub(crate) fn moves_closer(
    view: &BotView,
    data: &GameData,
    order: &Order,
    targets: &[[f32; 2]],
) -> bool {
    let Some((army, to)) = move_point(order) else {
        return false;
    };
    let Some(from) = army_point(view, data, army) else {
        return false;
    };
    let Some(goal) = nearest(from, targets) else {
        return false;
    };
    dist2(to, goal) < dist2(from, goal)
}

/// Names of every bot, in report order.
pub const ALL: &[&str] = &[
    "random-legal",
    "aggressor",
    "builder",
    "diplomat",
    "turtle",
    "sailor",
    "chaos",
    "mirror",
];

/// Builds the bot of that name (`None` for the bots not written yet).
pub fn make(name: &str) -> Option<Box<dyn Bot>> {
    match name {
        "random-legal" => Some(Box::new(random_legal::RandomLegal)),
        "aggressor" => Some(Box::new(aggressor::Aggressor)),
        "builder" => Some(Box::new(builder::Builder)),
        "diplomat" => Some(Box::new(diplomat::Diplomat)),
        "turtle" => Some(Box::new(turtle::Turtle)),
        "sailor" => Some(Box::new(sailor::Sailor)),
        "chaos" => Some(Box::new(chaos::Chaos)),
        "mirror" => Some(Box::new(mirror::Mirror)),
        _ => None,
    }
}

#[cfg(test)]
pub(crate) mod test_support {
    use data_model::GameData;
    use sim_campaign::rng::CampaignRng;
    use sim_campaign::{CampaignState, Order};

    use crate::bot::Bot;
    use crate::view::{fixtures, BotView};

    /// Plays `turns` turns of the 1337 France fixture with `bot`; returns
    /// the orders emitted, turn by turn.
    pub fn play(bot: &mut dyn Bot, seed: u64, turns: u32) -> Vec<Vec<Order>> {
        let data: &GameData = fixtures::data();
        let mut state: CampaignState = fixtures::state();
        let faction = fixtures::france();
        let mut rng = CampaignRng::from_seed(seed);
        let mut log = Vec::new();
        for _ in 0..turns {
            let orders = {
                let view = BotView {
                    state: &state,
                    faction: &faction,
                };
                bot.plan(&view, data, &mut rng)
            };
            for order in &orders {
                let _ = state.submit_order(data, order.clone());
            }
            log.push(orders);
            state.end_turn_with(data, ai::plan_turn);
        }
        log
    }

    /// Checks the bot emits a characteristic order, stays within bounds and
    /// is deterministic per seed.
    pub fn check(make: impl Fn() -> Box<dyn Bot>, characteristic: impl Fn(&Order) -> bool) {
        let first = play(make().as_mut(), 11, 5);
        let second = play(make().as_mut(), 11, 5);
        assert_eq!(first, second, "not deterministic");
        assert!(first.iter().all(|t| t.len() <= super::MAX_ORDERS as usize));
        assert!(
            first.iter().flatten().any(&characteristic),
            "no characteristic order in {first:?}"
        );
        assert!(first
            .iter()
            .flatten()
            .all(|o| !format!("{o:?}").starts_with("Debug")));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn makes_every_bot() {
        for name in ALL {
            assert_eq!(make(name).map(|b| b.name()), Some(*name));
        }
        assert!(make("nope").is_none());
    }
}
