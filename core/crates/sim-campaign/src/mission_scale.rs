//! Proportion of the mission offers to the house's real might (ADR 0831).
//!
//! The generator used to draw targets from the map alone: Scotland (100 men,
//! 800 livres of receipts) was asked to take Yorkshire, Navarre to raise three
//! units it could not pay. An offer is now kept only when the house can do
//! it with the men under arms and the gold it has; the limits live in
//! `data/missions.json` (`scale`).

use data_model::{FactionId, GameData, MissionCounter, MissionGoal, ProvinceId};

use crate::missions::{candidates_with, template_open};
use crate::state::CampaignState;

/// What the house can bring to an offer: men under arms and spendable gold.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HouseMight {
    /// Men under arms in the field armies.
    pub men: u32,
    /// Treasury net of debt, never negative.
    pub gold: i64,
}

/// Might of `faction` now.
pub fn house_might(state: &CampaignState, faction: &FactionId) -> HouseMight {
    let gold = state
        .factions
        .get(faction)
        .map_or(0, |f| (f.treasury - f.finance.debt()).max(0));
    HouseMight {
        men: state.men_under_arms(faction),
        gold,
    }
}

/// Men that hold `province` against a taker: the garrisons of the places its
/// controller holds and the armies sheltered in them.
pub fn target_force(state: &CampaignState, province: &ProvinceId) -> u32 {
    let Some(controller) = state.province_controller(province) else {
        return 0;
    };
    let sheltered: u32 = state
        .armies
        .values()
        .filter(|army| &army.faction == controller)
        .filter(|army| {
            army.settlement()
                .and_then(|place| state.settlement_province(place))
                .is_some_and(|home| home == province)
        })
        .map(|army| army.total_strength())
        .sum();
    state.province_garrison_strength(province) + sheltered
}

/// Gold of the cheapest unit the player can raise now.
fn cheapest_levy(state: &CampaignState, data: &GameData, player: &FactionId) -> Option<u32> {
    state
        .provinces
        .keys()
        .filter(|province| state.controls_province(player, province))
        .filter_map(|province| state.province_city_id(province))
        .flat_map(|city| state.recruitable(data, city))
        .filter(|option| option.available)
        .map(|option| option.cost)
        .min()
}

/// Whether a mission of this shape is within the might of `player`.
pub(crate) fn within_might(
    state: &CampaignState,
    data: &GameData,
    player: &FactionId,
    goal: MissionGoal,
    counter: Option<MissionCounter>,
    province: Option<&ProvinceId>,
) -> bool {
    let scale = &data.mission_rules.scale;
    let men = house_might(state, player).men;
    match (goal, counter) {
        (MissionGoal::Control, _) => {
            men >= scale.min_men_for_conquest
                && province.is_none_or(|p| {
                    f64::from(target_force(state, p))
                        <= f64::from(men) * scale.max_target_force_ratio
                })
        }
        (MissionGoal::Count, Some(MissionCounter::BattleWon)) => men >= scale.min_men_for_battle,
        _ => true,
    }
}

/// Units a levy mission can ask for: as many as the template wants, within
/// the share of the treasury the cheapest unit allows (`None`: not even one).
pub(crate) fn affordable_levy(
    state: &CampaignState,
    data: &GameData,
    player: &FactionId,
    wanted: u32,
) -> Option<u32> {
    let cost = f64::from(cheapest_levy(state, data, player)?.max(1));
    let budget =
        house_might(state, player).gold as f64 * data.mission_rules.scale.levy_treasury_share;
    let affordable = (budget / cost).floor() as u32;
    (affordable > 0).then_some(wanted.min(affordable))
}

/// One offer the generator could make, for the proportion check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OfferProbe {
    /// Template id.
    pub template: String,
    /// Target province, if any.
    pub province: Option<ProvinceId>,
    /// Men the target opposes (conquests), else 0.
    pub required_men: u32,
    /// Units a levy asks for, else 0.
    pub levy_units: u32,
    /// Gold the cheapest levy costs per unit (levies), else 0.
    pub levy_unit_cost: u32,
}

/// Every offer the generator may draw for the player now; `scaled: false`
/// ignores the proportion rules (to measure what they remove).
pub fn probe_offers(state: &CampaignState, data: &GameData, scaled: bool) -> Vec<OfferProbe> {
    let player = &state.player_faction;
    let unit_cost = cheapest_levy(state, data, player).unwrap_or(0);
    data.mission_rules
        .templates
        .iter()
        .filter(|template| template_open(state, player, template))
        .flat_map(|template| {
            candidates_with(state, data, player, template, scaled)
                .into_iter()
                .map(move |target| {
                    let levy = template.counter == Some(MissionCounter::UnitsRecruited);
                    OfferProbe {
                        template: template.id.clone(),
                        required_men: if template.goal == MissionGoal::Control {
                            target
                                .province
                                .as_ref()
                                .map_or(0, |p| target_force(state, p))
                        } else {
                            0
                        },
                        levy_units: if levy {
                            target.count.unwrap_or(template.count)
                        } else {
                            0
                        },
                        levy_unit_cost: if levy { unit_cost } else { 0 },
                        province: target.province,
                    }
                })
        })
        .collect()
}
