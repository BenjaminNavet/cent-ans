//! Strategic campaign AI (M9 spec § 1).
//!
//! [`plan_turn`] is pure and deterministic: it reads the state and returns the
//! faction's orders. It layers, in order: diplomacy and research (reused from
//! `sim-campaign`), economy (taxes, debt, recruitment, construction),
//! characters (governors, generals, skills, marriages) and one military
//! objective per army (defend, besiege, raid, regroup, retreat).

use std::collections::{BTreeMap, BTreeSet};

use data_model::{
    CharacterId, EffectKind, FactionId, GameData, ProvinceId, SkillBranch, SkillId, UnitTypeId,
};
use sim_campaign::movement::{dijkstra, edges, path_to};
use sim_campaign::population::weighted_unrest;
use sim_campaign::{ArmyId, CampaignState, Order, Season, Stance, TaxRate};

/// Maximum path cost considered for an objective.
pub const PLANNING_RANGE: u32 = 8;
/// An army besieges when its power exceeds the defence by this factor.
pub const SIEGE_SUPERIORITY: f64 = 1.5;
/// An army defends a province when it is at least this strong relative to the threat.
pub const DEFENCE_RATIO: f64 = 0.7;
/// Share of income spent on armies at war / at peace.
pub const WAR_MILITARY_SHARE: f64 = 0.7;
pub const PEACE_MILITARY_SHARE: f64 = 0.4;
/// Recruitment orders per turn: one per this much seasonal income (1 to 8).
pub const INCOME_PER_RECRUIT: i64 = 6000;
/// Minimum estimated odds (%) before the AI storms a besieged town.
pub const ASSAULT_ODDS: u32 = 65;
/// Armies below this share of their maximum strength fall back.
pub const RETREAT_STRENGTH: f64 = 0.4;

const REBELS: &str = "fac_rebels";

/// Everything the planner derives once per faction and turn.
struct Context<'a> {
    state: &'a CampaignState,
    data: &'a GameData,
    faction: &'a FactionId,
    enemies: BTreeSet<FactionId>,
    aggression: i32,
    income: i64,
    army_upkeep: i64,
    building_upkeep: i64,
    treasury: i64,
}

impl<'a> Context<'a> {
    fn new(state: &'a CampaignState, data: &'a GameData, faction: &'a FactionId) -> Option<Self> {
        let me = state.factions.get(faction)?;
        let aggression = data
            .factions
            .get(faction)
            .and_then(|f| f.ai_personality.as_ref())
            .and_then(|p| p.aggression)
            .map_or(50, i32::from);
        Some(Context {
            state,
            data,
            faction,
            enemies: me.at_war_with.clone(),
            aggression,
            income: state.faction_income_effective(data, faction),
            army_upkeep: state.faction_army_upkeep(data, faction),
            building_upkeep: state.faction_building_upkeep(data, faction),
            treasury: me.treasury,
        })
    }

    fn at_war(&self) -> bool {
        self.enemies.iter().any(|e| e.as_str() != REBELS)
    }

    fn upkeep(&self) -> i64 {
        self.army_upkeep + self.building_upkeep
    }

    /// Treasury kept aside: two turns of upkeep.
    fn reserve(&self) -> i64 {
        2 * self.upkeep()
    }

    fn owns(&self, province: &ProvinceId) -> bool {
        self.state
            .provinces
            .get(province)
            .is_some_and(|p| &p.controller == self.faction && &p.owner == self.faction)
    }

    fn province_income(&self, province: &ProvinceId) -> f64 {
        self.state.provinces.get(province).map_or(0.0, |p| {
            sim_campaign::economy::province_income_effective(self.data, p, TaxRate::Normal)
        })
    }

    /// Hostile army power inside or next to `province`.
    fn threat(&self, province: &ProvinceId) -> f64 {
        let mut provinces = vec![province.clone()];
        provinces.extend(edges(self.data, province).into_iter().map(|(p, _)| p));
        provinces
            .iter()
            .flat_map(|p| self.state.hostile_armies_in(self.faction, p))
            .map(|id| self.state.army_power(self.data, &id))
            .sum()
    }

    fn is_border(&self, province: &ProvinceId) -> bool {
        edges(self.data, province)
            .iter()
            .any(|(n, _)| self.state.is_hostile_territory(self.faction, n))
    }
}

/// Orders of `faction` for this turn.
pub fn plan_turn(state: &CampaignState, data: &GameData, faction: &FactionId) -> Vec<Order> {
    if faction.as_str() == REBELS || !state.factions.get(faction).is_some_and(|f| f.alive) {
        return Vec::new();
    }
    let Some(ctx) = Context::new(state, data, faction) else {
        return Vec::new();
    };
    let mut orders = sim_campaign::diplomacy::plan_diplomacy(state, data, faction);
    if let Some(technology) = sim_campaign::research::ai_choose_research(state, data, faction) {
        orders.push(Order::Research { technology });
    }
    plan_economy(&ctx, &mut orders);
    plan_characters(&ctx, &mut orders);
    plan_armies(&ctx, &mut orders);
    orders
}

// =========================================================================
// Economy
// =========================================================================

fn plan_economy(ctx: &Context, orders: &mut Vec<Order>) {
    let state = ctx.state;
    let data = ctx.data;
    let me = &state.factions[ctx.faction];

    // Taxes follow war needs and public order.
    let (weighted, population) = state
        .provinces
        .values()
        .filter(|p| &p.controller == ctx.faction)
        .fold((0.0, 0.0), |(w, n), p| {
            let size = p.population.total() as f64;
            (w + weighted_unrest(&p.population) * size, n + size)
        });
    let unrest = if population > 0.0 {
        weighted / population
    } else {
        0.0
    };
    // A treasury worth ten seasons of income is idle money: spend it, and
    // lighten taxes at peace.
    let rich = ctx.treasury > 10 * ctx.income.max(1);
    let rate = if unrest > 55.0 || (rich && !ctx.at_war()) {
        TaxRate::Low
    } else if ctx.at_war() && unrest < 30.0 {
        TaxRate::High
    } else {
        TaxRate::Normal
    };
    if rate != me.tax_rate {
        orders.push(Order::SetTaxRate { rate });
    }

    // Debt: dismiss the costliest unit.
    if ctx.treasury < 0 && ctx.income < ctx.upkeep() {
        if let Some(order) = disband_costliest(ctx) {
            orders.push(order);
        }
        return;
    }

    let mut budget = ctx.treasury - ctx.reserve();
    let share = if ctx.at_war() {
        WAR_MILITARY_SHARE
    } else {
        PEACE_MILITARY_SHARE
    };
    let share = if rich { share + 0.3 } else { share };
    let target_upkeep = (ctx.income as f64 * share) as i64;
    let mut planned_upkeep = ctx.army_upkeep;

    // Recruitment: capital first, then threatened border provinces.
    let mut sites: Vec<ProvinceId> = vec![me.capital.clone()];
    let mut borders: Vec<(ProvinceId, i64)> = state
        .provinces
        .keys()
        .filter(|id| ctx.owns(id) && **id != me.capital && ctx.is_border(id))
        .map(|id| (id.clone(), -(ctx.threat(id) as i64)))
        .collect();
    borders.sort_by(|a, b| a.1.cmp(&b.1).then_with(|| a.0.cmp(&b.0)));
    sites.extend(borders.into_iter().map(|(id, _)| id));
    let mut recruits = 0;
    let max_recruits = (ctx.income / INCOME_PER_RECRUIT).clamp(1, 8) as usize;
    'sites: for site in &sites {
        if !ctx.owns(site) {
            continue;
        }
        let best = state
            .recruitable(data, site)
            .into_iter()
            .filter(|o| o.available)
            .max_by(|a, b| {
                unit_value(data, &a.unit_type, a.cost)
                    .total_cmp(&unit_value(data, &b.unit_type, b.cost))
                    .then_with(|| b.unit_type.cmp(&a.unit_type))
            });
        let Some(option) = best else {
            continue;
        };
        while recruits < max_recruits
            && planned_upkeep + i64::from(option.upkeep)
                <= target_upkeep.max(i64::from(option.upkeep))
            && budget >= i64::from(option.cost)
        {
            orders.push(Order::Recruit {
                province: site.clone(),
                unit_type: option.unit_type.clone(),
            });
            budget -= i64::from(option.cost);
            planned_upkeep += i64::from(option.upkeep);
            recruits += 1;
            if !ctx.at_war() && recruits % 2 == 0 {
                continue 'sites;
            }
        }
    }

    // Garrisons beyond need become field armies (merged next turn).
    for (id, province) in &state.provinces {
        if !ctx.owns(id) {
            continue;
        }
        let keep = if *id == me.capital {
            3
        } else if ctx.is_border(id) {
            2
        } else {
            1
        };
        if province.garrison.len() > keep + 1 && province.siege.is_none() {
            orders.push(Order::CreateArmy {
                province: id.clone(),
                units_from_garrison: (keep..province.garrison.len()).collect(),
                general: None,
            });
        }
    }

    // Construction: best yield per livre; the richer, the more sites at once.
    let builds = (1 + (budget / 15_000).max(0) as usize).min(6);
    let mut options: Vec<(f64, ProvinceId, data_model::BuildingId, i64)> = Vec::new();
    for id in state.provinces.keys().filter(|id| ctx.owns(id)) {
        let Some(city) = state.province_city(data, id) else {
            continue;
        };
        if city.construction.is_some() {
            continue;
        }
        let unrest = weighted_unrest(&city.classes);
        let health = f64::from(city.classes.peasants.health);
        for option in city.buildable.iter().filter(|o| o.available) {
            let value = building_value(ctx, id, &option.building, unrest, health);
            if value > 0.0 {
                options.push((
                    value / f64::from(option.cost.max(1)),
                    id.clone(),
                    option.building.clone(),
                    i64::from(option.cost),
                ));
            }
        }
    }
    options.sort_by(|a, b| {
        b.0.total_cmp(&a.0)
            .then_with(|| a.1.cmp(&b.1))
            .then_with(|| a.2.cmp(&b.2))
    });
    let mut used = BTreeSet::new();
    for (_, province, building, cost) in options {
        if used.len() >= builds || used.contains(&province) || budget < cost {
            continue;
        }
        budget -= cost;
        used.insert(province.clone());
        orders.push(Order::Build { province, building });
    }
}

/// Power per livre of a unit type.
fn unit_value(data: &GameData, unit_type: &UnitTypeId, cost: u32) -> f64 {
    data.unit_types.get(unit_type).map_or(0.0, |t| {
        let attack = f64::from(t.stats.melee.max(t.stats.ranged));
        let defence = f64::from(t.stats.armor) / 2.0 + f64::from(t.stats.morale) / 4.0;
        f64::from(t.soldiers) * (attack + defence) / f64::from(cost.max(1))
    })
}

/// Seasonal value (livres-equivalent) of building `building` in `province`.
fn building_value(
    ctx: &Context,
    province: &ProvinceId,
    building: &data_model::BuildingId,
    unrest: f64,
    health: f64,
) -> f64 {
    let Some(def) = ctx.data.buildings.get(building) else {
        return 0.0;
    };
    let income = ctx.province_income(province);
    let mut value = 0.0;
    for effect in &def.effects {
        let v = effect.value;
        value += match effect.effect {
            EffectKind::TaxIncome | EffectKind::TradeIncome => {
                if matches!(effect.mode, data_model::EffectMode::Percent) {
                    income * v / 100.0
                } else {
                    v
                }
            }
            EffectKind::Unrest if unrest > 30.0 => -v * income / 50.0,
            EffectKind::Health if health < 50.0 => v * income / 80.0,
            EffectKind::Growth | EffectKind::Wealth => v * income / 150.0,
            EffectKind::ResearchPoints => v * 60.0,
            EffectKind::Garrison | EffectKind::FortificationLevel if ctx.is_border(province) => {
                v * 20.0
            }
            _ => 0.0,
        };
    }
    value - f64::from(def.upkeep.unwrap_or(0)) * 1.5
}

/// Dismisses the costliest unit of the largest army, or else of the largest
/// garrison (never the last unit of either).
fn disband_costliest(ctx: &Context) -> Option<Order> {
    let upkeep = |t: &UnitTypeId| ctx.data.unit_types.get(t).map_or(0, |u| u.upkeep);
    let army = ctx
        .state
        .armies
        .iter()
        .filter(|(_, a)| &a.faction == ctx.faction && a.units.len() > 1)
        .max_by_key(|(id, a)| (a.units.len(), std::cmp::Reverse((*id).clone())));
    if let Some((id, army)) = army {
        let (index, _) = army
            .units
            .iter()
            .enumerate()
            .max_by_key(|(i, u)| (upkeep(&u.unit_type), std::cmp::Reverse(*i)))?;
        return Some(Order::DisbandUnit {
            army: Some(id.clone()),
            province: None,
            unit_index: index,
        });
    }
    let (province, garrison) = ctx
        .state
        .provinces
        .iter()
        .filter(|(_, p)| &p.controller == ctx.faction && p.garrison.len() > 1)
        .max_by_key(|(id, p)| (p.garrison.len(), std::cmp::Reverse((*id).clone())))?;
    let (index, _) = garrison
        .garrison
        .iter()
        .enumerate()
        .max_by_key(|(i, u)| (upkeep(&u.unit_type), std::cmp::Reverse(*i)))?;
    Some(Order::DisbandUnit {
        army: None,
        province: Some(province.clone()),
        unit_index: index,
    })
}

// =========================================================================
// Characters
// =========================================================================

fn plan_characters(ctx: &Context, orders: &mut Vec<Order>) {
    let state = ctx.state;
    let data = ctx.data;
    let year = state.year();
    let me = &state.factions[ctx.faction];
    let ruler = me.ruler.clone();
    let available = |id: &CharacterId| {
        state
            .characters
            .get(id)
            .is_some_and(|c| c.alive && !c.captive && &c.faction == ctx.faction && c.is_major(year))
    };
    let mut busy: BTreeSet<CharacterId> = state
        .characters
        .iter()
        .filter(|(_, c)| c.army.is_some() || c.governor_of.is_some())
        .map(|(id, _)| id.clone())
        .collect();

    // Generals: the best available commander standing with each leaderless army.
    for (army_id, army) in state
        .armies
        .iter()
        .filter(|(_, a)| &a.faction == ctx.faction)
    {
        if army.general.is_some() {
            continue;
        }
        let best = state
            .characters
            .iter()
            .filter(|(id, c)| {
                available(id) && !busy.contains(*id) && c.location.as_ref() == Some(&army.location)
            })
            .max_by_key(|(id, c)| (c.skills.command, std::cmp::Reverse((*id).clone())))
            .map(|(id, _)| id.clone());
        if let Some(general) = best {
            busy.insert(general.clone());
            orders.push(Order::AssignGeneral {
                army: army_id.clone(),
                character: general,
            });
        }
    }

    // Governors: best administrators to the richest ungoverned provinces.
    let mut provinces: Vec<(ProvinceId, i64)> = state
        .provinces
        .keys()
        .filter(|id| ctx.owns(id) && state.province_governor(id).is_none())
        .map(|id| (id.clone(), ctx.province_income(id) as i64))
        .collect();
    provinces.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    let mut candidates: Vec<(CharacterId, u8)> = state
        .characters
        .iter()
        .filter(|(id, _)| available(id) && !busy.contains(*id) && Some(*id) != ruler.as_ref())
        .map(|(id, c)| (id.clone(), c.skills.governance))
        .collect();
    candidates.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    for ((province, _), (character, _)) in provinces.into_iter().zip(candidates).take(3) {
        orders.push(Order::AssignGovernor {
            province,
            character,
        });
    }

    // Skill points: the branch of the character's job.
    for (id, c) in state
        .characters
        .iter()
        .filter(|(_, c)| c.alive && &c.faction == ctx.faction && c.skill_points > 0)
    {
        let branch = if c.army.is_some() {
            SkillBranch::Command
        } else if c.governor_of.is_some() {
            SkillBranch::Governance
        } else {
            SkillBranch::Court
        };
        let learnable: Vec<SkillId> = state
            .learnable_skills(data, id)
            .into_iter()
            .filter(|s| data.skills.get(s).is_some_and(|d| d.cost <= c.skill_points))
            .collect();
        let pick = learnable
            .iter()
            .filter(|s| data.skills[*s].branch == branch)
            .min_by_key(|s| (data.skills[*s].tier, (*s).clone()))
            .or_else(|| {
                learnable
                    .iter()
                    .min_by_key(|s| (data.skills[*s].tier, (*s).clone()))
            });
        if let Some(skill) = pick {
            orders.push(Order::LearnSkill {
                character: id.clone(),
                skill: skill.clone(),
            });
        }
    }

    // Marriages: once a year, one unmarried adult of the ruling house.
    if state.season != Season::Spring {
        return;
    }
    let house = ruler
        .as_ref()
        .and_then(|r| state.characters.get(r))
        .map(|r| r.house.clone());
    let Some(house) = house else {
        return;
    };
    let single = state
        .characters
        .iter()
        .filter(|(id, c)| {
            available(id)
                && c.spouse.is_none()
                && c.house == house
                && c.age(year) >= 16
                && c.age(year) <= 45
        })
        .min_by_key(|(id, c)| (c.birth_year, (*id).clone()))
        .map(|(id, _)| id.clone());
    let Some(single) = single else {
        return;
    };
    let partner = state
        .marriage_candidates(data, &single)
        .into_iter()
        .filter_map(|id| state.characters.get(&id).map(|c| (id, c)))
        .filter(|(_, c)| &c.faction == ctx.faction || state.is_allied(ctx.faction, &c.faction))
        .filter(|(_, c)| c.faction != ctx.state.player_faction)
        .max_by_key(|(id, c)| (c.prestige, std::cmp::Reverse(id.clone())))
        .map(|(id, c)| (id, c.faction.clone()));
    match partner {
        Some((spouse, spouse_faction)) if &spouse_faction == ctx.faction => {
            orders.push(Order::ProposeMarriage {
                character: single,
                spouse,
            });
        }
        Some((spouse, spouse_faction)) => orders.push(Order::ProposeFactionMarriage {
            target: spouse_faction,
            character: single,
            spouse,
        }),
        None => {}
    }
}

// =========================================================================
// Armies
// =========================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Objective {
    Defend,
    Siege,
    Raid,
    Regroup,
    Retreat,
}

fn plan_armies(ctx: &Context, orders: &mut Vec<Order>) {
    let state = ctx.state;
    let data = ctx.data;
    let mut armies: Vec<(ArmyId, f64)> = state
        .armies
        .iter()
        .filter(|(_, a)| &a.faction == ctx.faction)
        .map(|(id, _)| (id.clone(), state.army_power(data, id)))
        .collect();
    armies.sort_by(|a, b| b.1.total_cmp(&a.1).then_with(|| a.0.cmp(&b.0)));

    // Merge armies standing together into the strongest one.
    let mut merged: BTreeSet<ArmyId> = BTreeSet::new();
    let mut by_location: BTreeMap<ProvinceId, ArmyId> = BTreeMap::new();
    for (id, _) in &armies {
        let location = state.armies[id].location.clone();
        match by_location.get(&location) {
            Some(target) => {
                orders.push(Order::MergeArmies {
                    source: id.clone(),
                    target: target.clone(),
                });
                merged.insert(id.clone());
            }
            None => {
                by_location.insert(location, id.clone());
            }
        }
    }
    let power_at = |province: &ProvinceId| -> f64 {
        armies
            .iter()
            .filter(|(id, _)| &state.armies[id].location == province)
            .map(|(_, p)| p)
            .sum()
    };

    let claims: BTreeSet<ProvinceId> = state.factions[ctx.faction]
        .claims
        .iter()
        .filter_map(|c| c.province.clone())
        .collect();
    let enemy_capitals: BTreeSet<ProvinceId> = ctx
        .enemies
        .iter()
        .filter_map(|e| state.factions.get(e).map(|f| f.capital.clone()))
        .collect();
    let largest = armies.first().map(|(id, p)| (id.clone(), *p));
    let mut defended: BTreeSet<ProvinceId> = BTreeSet::new();
    let mut targeted: BTreeSet<ProvinceId> = BTreeSet::new();

    for (army_id, _) in armies.iter().filter(|(id, _)| !merged.contains(id)) {
        let army = &state.armies[army_id];
        let power = power_at(&army.location);
        let strength: u32 = army.units.iter().map(|u| u.strength).sum();
        let max_strength: u32 = army.units.iter().map(|u| u.max_strength).sum();
        let table = dijkstra(
            state,
            data,
            ctx.faction,
            &army.location,
            Some(PLANNING_RANGE),
        );
        let besieging = state
            .provinces
            .get(&army.location)
            .and_then(|p| p.siege.as_ref())
            .is_some_and(|s| &s.attacker == ctx.faction);

        let mut choice: Option<(Objective, ProvinceId)> = None;

        // 5. Retreat when broken or starving in winter.
        let broken =
            max_strength > 0 && f64::from(strength) < RETREAT_STRENGTH * f64::from(max_strength);
        let starving = state.season == Season::Winter
            && army.supply < 30
            && state.is_hostile_territory(ctx.faction, &army.location);
        if (broken || starving) && !besieging {
            let home = table
                .iter()
                .filter(|(id, _)| ctx.owns(id) && ctx.threat(id) < power)
                .min_by_key(|(id, reach)| (reach.cost, (*id).clone()))
                .map(|(id, _)| id.clone());
            if let Some(home) = home {
                choice = Some((Objective::Retreat, home));
            }
        }

        // Keep a siege that is going our way.
        if choice.is_none() && besieging && ctx.threat(&army.location) < power * 1.2 {
            targeted.insert(army.location.clone());
            // Storm the walls when the odds are good (M8).
            if state
                .assault_odds(data, army_id)
                .is_some_and(|(odds, _)| odds >= ASSAULT_ODDS)
            {
                orders.push(Order::Assault {
                    army: army_id.clone(),
                });
                continue;
            }
            if army.stance != Stance::Siege {
                orders.push(Order::SetStance {
                    army: army_id.clone(),
                    stance: Stance::Siege,
                });
            }
            continue;
        }

        // 1. Defend a threatened friendly province.
        if choice.is_none() {
            choice = table
                .iter()
                .filter(|(id, _)| {
                    state.is_friendly_territory(ctx.faction, id) && !defended.contains(*id)
                })
                .filter_map(|(id, reach)| {
                    let threat = ctx.threat(id);
                    let local = state.defensive_power(data, id);
                    (threat > local * 0.8 && power >= DEFENCE_RATIO * threat).then(|| {
                        let weight =
                            if Some(id) == state.factions.get(ctx.faction).map(|f| &f.capital) {
                                3.0
                            } else {
                                1.0
                            };
                        let value = weight * (ctx.province_income(id) + threat)
                            / (1.0 + f64::from(reach.cost));
                        (value, id.clone())
                    })
                })
                .max_by(|a, b| a.0.total_cmp(&b.0).then_with(|| b.1.cmp(&a.1)))
                .map(|(_, id)| (Objective::Defend, id));
        }

        // 2. Besiege the most valuable weak enemy province.
        if choice.is_none() {
            choice = table
                .iter()
                .filter(|(id, _)| {
                    state.is_hostile_territory(ctx.faction, id) && !targeted.contains(*id)
                })
                .filter(|(id, _)| state.defensive_power(data, id) * SIEGE_SUPERIORITY < power)
                .map(|(id, reach)| {
                    let mut value = ctx.province_income(id) / 100.0 + 10.0;
                    if enemy_capitals.contains(id) {
                        value += 50.0;
                    }
                    if claims.contains(id) {
                        value += 30.0;
                    }
                    (value / (1.0 + f64::from(reach.cost) / 2.0), id.clone())
                })
                .max_by(|a, b| a.0.total_cmp(&b.0).then_with(|| b.1.cmp(&a.1)))
                .map(|(_, id)| (Objective::Siege, id));
        }

        // 3. Chevauchée when too weak to besiege (aggressive factions).
        if choice.is_none() && ctx.aggression >= 65 && ctx.at_war() {
            choice = table
                .iter()
                .filter(|(id, _)| state.is_hostile_territory(ctx.faction, id))
                .filter(|(id, _)| {
                    state.provinces[*id].devastation < 50
                        && state.defensive_power(data, id) < power * 2.0
                        && ctx.threat(id) < power
                })
                .min_by_key(|(id, reach)| (reach.cost, (*id).clone()))
                .map(|(id, _)| (Objective::Raid, id.clone()));
        }

        // 4. Regroup with the main army when much weaker.
        if choice.is_none() {
            if let Some((main, main_power)) = &largest {
                if main != army_id && power < 0.5 * main_power {
                    let location = state.armies[main].location.clone();
                    if table.contains_key(&location) {
                        choice = Some((Objective::Regroup, location));
                    }
                }
            }
        }

        let Some((objective, target)) = choice else {
            // Idle: normal stance at home.
            if army.stance != Stance::Normal
                && !state.is_hostile_territory(ctx.faction, &army.location)
            {
                orders.push(Order::SetStance {
                    army: army_id.clone(),
                    stance: Stance::Normal,
                });
            }
            continue;
        };
        match objective {
            Objective::Defend => {
                defended.insert(target.clone());
            }
            Objective::Siege => {
                targeted.insert(target.clone());
            }
            _ => {}
        }
        let stance = match objective {
            Objective::Siege => Stance::Siege,
            Objective::Raid => Stance::Raid,
            _ => Stance::Normal,
        };
        if army.stance != stance {
            orders.push(Order::SetStance {
                army: army_id.clone(),
                stance,
            });
        }
        if target != army.location {
            if let Some(path) = path_to(&table, &target) {
                orders.push(Order::MoveArmy {
                    army: army_id.clone(),
                    path,
                });
            }
        }
    }
}
