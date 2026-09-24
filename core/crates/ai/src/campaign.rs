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
/// Value kept by a claimed target reached through a sea crossing (landings
/// are costly); unclaimed provinces are never invaded by sea.
pub const SEA_INVASION_FACTOR: f64 = 0.6;
/// An army defends a province when it is at least this strong relative to the threat.
pub const DEFENCE_RATIO: f64 = 0.7;
/// Share of income spent on armies at war / at peace.
pub const WAR_MILITARY_SHARE: f64 = 0.7;
pub const PEACE_MILITARY_SHARE: f64 = 0.4;
/// A debt must be repaid within this many turns, or units are dismissed.
const DEBT_REPAYMENT_TURNS: i64 = 8;
/// Units dismissed at most per turn to cut a debt.
const MAX_DISBANDS_PER_TURN: usize = 12;
/// Recruitment orders per turn: one per this much seasonal income (1 to 8).
pub const INCOME_PER_RECRUIT: i64 = 6000;
/// Minimum estimated odds (%) before the AI storms a besieged town.
pub const ASSAULT_ODDS: u32 = 65;
/// Armies below this share of their maximum strength fall back.
pub const RETREAT_STRENGTH: f64 = 0.4;

/// Seasons of gross income kept in the treasury; above, money is spent (F4).
pub const RESERVE_SEASONS: i64 = 3;
/// Turns over which a hoard above the reserve is spent (army, buildings).
pub const HOARD_SPENDING_TURNS: i64 = 8;
/// Seasons of deficit a treasury must cover before units are dismissed
/// ahead of bankruptcy, at war / at peace (F4).
pub const WAR_RUNWAY_TURNS: i64 = 8;
pub const PEACE_RUNWAY_TURNS: i64 = 3;
/// Share of the hoard given to the Church each year while papal favour is
/// below [`DONATION_FAVOR`].
pub const DONATION_PERCENT: i64 = 5;
pub const DONATION_FAVOR: u8 = 70;

/// A realm down to this many free provinces is not besieged there by an
/// enemy without a claim on them: peace decides its fate (F4).
pub const LAST_BASTIONS: usize = 2;

const REBELS: &str = "fac_rebels";

/// Everything the planner derives once per faction and turn.
struct Context<'a> {
    state: &'a CampaignState,
    data: &'a GameData,
    faction: &'a FactionId,
    enemies: BTreeSet<FactionId>,
    aggression: i32,
    income: i64,
    /// Gross income of the season (the unit of « seasons of income »).
    gross_income: i64,
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
            // Net of court and administration (M10 balance) and of the
            // tribute owed to a suzerain.
            income: {
                let gross = state.faction_income_effective(data, faction);
                let tribute = if me.suzerain.is_some() {
                    (gross * sim_campaign::diplomacy::VASSAL_TRIBUTE_PERCENT / 100).max(0)
                } else {
                    0
                };
                gross - state.faction_administration_upkeep(data, faction) - tribute
            },
            gross_income: state.faction_income_effective(data, faction),
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

    /// Seasonal surplus (negative: deficit) at the current upkeep.
    fn surplus(&self) -> i64 {
        self.income - self.upkeep()
    }

    /// Treasury above [`RESERVE_SEASONS`] of gross income: idle money the
    /// faction spends over [`HOARD_SPENDING_TURNS`] (F4).
    fn hoard(&self) -> i64 {
        (self.treasury - RESERVE_SEASONS * self.gross_income.max(0)).max(0)
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
    orders.extend(sim_campaign::table::ai_choose_diets(state, data, faction));
    orders.extend(sim_campaign::coinage::ai_choose_coinage(
        state, data, faction,
    ));
    orders.extend(sim_campaign::ransom::ai_ransom_orders(state, data, faction));
    orders.extend(sim_campaign::chivalry::ai_found_order(state, data, faction));
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
    // Taxes: heavy in war or deficit if public order allows, light when
    // the realm grumbles (F4: a rich treasury is spent, not untaxed).
    let in_debt = ctx.treasury < 0;
    let rate = if unrest > 55.0 {
        TaxRate::Low
    } else if (ctx.at_war() || in_debt || ctx.surplus() < 0) && unrest < 30.0 {
        TaxRate::High
    } else {
        TaxRate::Normal
    };
    if rate != me.tax_rate {
        orders.push(Order::SetTaxRate { rate });
    }

    // Debt: dismiss the costliest unit until the surplus repays the debt
    // within `DEBT_REPAYMENT_TURNS`. F4: dismiss ahead of bankruptcy when the
    // treasury no longer covers the deficit for a few seasons.
    let runway = if ctx.at_war() {
        WAR_RUNWAY_TURNS
    } else {
        PEACE_RUNWAY_TURNS
    };
    let wanted_surplus = if ctx.treasury < 0 {
        -ctx.treasury / DEBT_REPAYMENT_TURNS
    } else if ctx.surplus() < 0
        && (ctx.treasury < -ctx.surplus() * runway || (!ctx.at_war() && ctx.hoard() == 0))
    {
        0
    } else {
        i64::MIN
    };
    if ctx.surplus() < wanted_surplus {
        let savings = wanted_surplus - ctx.surplus();
        orders.extend(disband_for_debt(ctx, savings));
        return;
    }

    // Idle money (F4): a yearly gift to the Church while its favour is low.
    let hoard = ctx.hoard();
    let catholic = sim_campaign::religion::is_catholic(state, data, ctx.faction);
    if hoard > 0
        && catholic
        && me.papal_favor < DONATION_FAVOR
        && state.season == Season::Autumn
        && ctx.faction.as_str() != sim_campaign::diplomacy::PAPACY_FACTION
    {
        let amount = hoard * DONATION_PERCENT / 100;
        if amount >= 200 {
            orders.push(Order::DonateToChurch { amount });
        }
    }

    let mut budget = ctx.treasury - ctx.reserve();
    let share = if ctx.at_war() {
        WAR_MILITARY_SHARE
    } else {
        PEACE_MILITARY_SHARE
    };
    // F4: armies are paid from what buildings leave of the net income, plus
    // the hoard spent over `HOARD_SPENDING_TURNS`; small realms live within
    // their means.
    let target_upkeep = ((ctx.income - ctx.building_upkeep).max(0) as f64 * share) as i64
        + hoard / HOARD_SPENDING_TURNS;
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
    let max_recruits =
        ((ctx.income + hoard / HOARD_SPENDING_TURNS) / INCOME_PER_RECRUIT).clamp(1, 8) as usize;
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
        // G1: no more than the province's free recruitment slots.
        let mut free_slots = state.recruit_slots_free(data, site);
        while recruits < max_recruits
            && free_slots > 0
            && planned_upkeep + i64::from(option.upkeep)
                <= if planned_upkeep == 0 && ctx.surplus() >= i64::from(option.upkeep) {
                    target_upkeep.max(i64::from(option.upkeep))
                } else {
                    target_upkeep
                }
            && budget >= i64::from(option.cost)
        {
            orders.push(Order::Recruit {
                province: site.clone(),
                unit_type: option.unit_type.clone(),
            });
            budget -= i64::from(option.cost);
            planned_upkeep += i64::from(option.upkeep);
            recruits += 1;
            free_slots -= 1;
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
    // F4: a new building's upkeep must fit in the surplus left by the army
    // (or in the hoard being spent).
    let mut spare =
        ctx.surplus() - (planned_upkeep - ctx.army_upkeep) + hoard / HOARD_SPENDING_TURNS;
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
        let upkeep = data
            .buildings
            .get(&building)
            .map_or(0, |b| i64::from(b.upkeep.unwrap_or(0)));
        if used.len() >= builds || used.contains(&province) || budget < cost || upkeep > spare {
            continue;
        }
        spare -= upkeep;
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

/// In debt, dismisses the costliest units until `savings` livres of upkeep
/// are saved (at most [`MAX_DISBANDS_PER_TURN`]): field units first, then
/// garrisons. The capital keeps its last unit and besieged places keep theirs;
/// an army losing its last unit is disbanded.
fn disband_for_debt(ctx: &Context, savings: i64) -> Vec<Order> {
    use sim_campaign::economy::{unit_upkeep, GARRISON_UPKEEP_PERCENT};
    enum Holder {
        Army(ArmyId),
        Garrison(ProvinceId),
    }
    let state = ctx.state;
    let capital = &state.factions[ctx.faction].capital;
    // (upkeep, is_garrison, holder index, unit index)
    let mut holders: Vec<Holder> = Vec::new();
    let mut candidates: Vec<(i64, bool, usize, usize)> = Vec::new();
    for (id, army) in state
        .armies
        .iter()
        .filter(|(_, a)| &a.faction == ctx.faction)
    {
        let holder = holders.len();
        holders.push(Holder::Army(id.clone()));
        for (index, unit) in army.units.iter().enumerate() {
            candidates.push((unit_upkeep(ctx.data, unit), false, holder, index));
        }
    }
    for (id, province) in state
        .provinces
        .iter()
        .filter(|(_, p)| &p.controller == ctx.faction && p.siege.is_none())
    {
        let keep = usize::from(id == capital);
        if province.garrison.len() <= keep {
            continue;
        }
        let holder = holders.len();
        holders.push(Holder::Garrison(id.clone()));
        let mut units: Vec<(i64, usize)> = province
            .garrison
            .iter()
            .enumerate()
            .map(|(index, unit)| {
                (
                    unit_upkeep(ctx.data, unit) * GARRISON_UPKEEP_PERCENT / 100,
                    index,
                )
            })
            .collect();
        // The cheapest `keep` units stay.
        units.sort_by_key(|(upkeep, index)| (std::cmp::Reverse(*upkeep), *index));
        units.truncate(province.garrison.len() - keep);
        for (upkeep, index) in units {
            candidates.push((upkeep, true, holder, index));
        }
    }
    // Field units before garrisons, costliest first.
    candidates.sort_by_key(|(upkeep, garrison, holder, index)| {
        (*garrison, std::cmp::Reverse(*upkeep), *holder, *index)
    });
    let mut saved = 0;
    let mut chosen: Vec<(usize, usize)> = Vec::new();
    for (upkeep, _, holder, index) in candidates {
        if saved >= savings || chosen.len() >= MAX_DISBANDS_PER_TURN {
            break;
        }
        saved += upkeep;
        chosen.push((holder, index));
    }
    // Highest indices first so that earlier removals keep later indices valid.
    chosen.sort_by_key(|(holder, index)| (*holder, std::cmp::Reverse(*index)));
    chosen
        .into_iter()
        .map(|(holder, unit_index)| match &holders[holder] {
            Holder::Army(army) => Order::DisbandUnit {
                army: Some(army.clone()),
                province: None,
                unit_index,
            },
            Holder::Garrison(province) => Order::DisbandUnit {
                army: None,
                province: Some(province.clone()),
                unit_index,
            },
        })
        .collect()
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
    // F4: the ruler and the heir first (a dynasty needs sons), then the
    // eldest of the house.
    let heir = me.heir.clone();
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
        .min_by_key(|(id, c)| {
            let rank = if Some(*id) == ruler.as_ref() {
                0
            } else if Some(*id) == heir.as_ref() {
                1
            } else {
                2
            };
            (rank, c.birth_year, (*id).clone())
        })
        .map(|(id, _)| id.clone());
    let Some(single) = single else {
        return;
    };
    let partner = marriage_partner(ctx, &single);
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

/// Best spouse for `single` (F4): a partner of child-bearing age close in
/// years, preferably from a ruling house of an ally or of a friendly realm
/// (diplomatic marriage), who would accept; the player receives an offer.
fn marriage_partner(ctx: &Context, single: &CharacterId) -> Option<(CharacterId, FactionId)> {
    let state = ctx.state;
    let data = ctx.data;
    let year = state.year();
    let me = state.characters.get(single)?;
    let ruling = |c: &sim_campaign::CharacterState| {
        state
            .factions
            .get(&c.faction)
            .and_then(|f| f.ruler.as_ref())
            .and_then(|r| state.characters.get(r))
            .is_some_and(|r| r.house == c.house)
    };
    state
        .marriage_candidates(data, single)
        .into_iter()
        .filter_map(|id| state.characters.get(&id).map(|c| (id, c)))
        .filter(|(_, c)| !c.captive && c.faction.as_str() != REBELS)
        .filter(|(_, c)| (c.age(year) - me.age(year)).abs() <= 15)
        .filter(|(_, c)| {
            let wife = if c.sex == data_model::Sex::Female {
                c
            } else {
                me
            };
            wife.age(year) <= 35
        })
        .filter_map(|(id, c)| {
            let own = &c.faction == ctx.faction;
            let attitude = if own {
                0
            } else {
                state.attitude(data, ctx.faction, &c.faction).0
            };
            if !own && (attitude < 0 || state.is_at_war(ctx.faction, &c.faction)) {
                return None;
            }
            let mut value = c.prestige + attitude;
            if !own && ruling(c) {
                value += 40;
            }
            if state.is_allied(ctx.faction, &c.faction) && !own {
                value += 20;
            }
            Some((value, id, c.faction.clone()))
        })
        .filter(|(_, id, faction)| {
            faction == ctx.faction
                || faction == &state.player_faction
                || sim_campaign::diplomacy::evaluate(
                    state,
                    data,
                    ctx.faction,
                    faction,
                    &sim_campaign::diplomacy::Proposal::Marriage {
                        character: single.clone(),
                        spouse: id.clone(),
                    },
                )
                .accept
        })
        .max_by(|a, b| a.0.cmp(&b.0).then_with(|| b.1.cmp(&a.1)))
        .map(|(_, id, faction)| (id, faction))
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

/// True when the planned path to `target` includes a sea crossing.
fn crosses_sea(
    data: &GameData,
    table: &BTreeMap<ProvinceId, sim_campaign::movement::Reach>,
    target: &ProvinceId,
) -> bool {
    let mut current = target.clone();
    while let Some(previous) = table.get(&current).and_then(|r| r.previous.clone()) {
        if sim_campaign::movement::is_sea_crossing(data, &previous, &current) {
            return true;
        }
        current = previous;
    }
    false
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

    // F4: a throne claim makes every province of that crown a claimed target.
    let claims = sim_campaign::diplomacy::claimed_provinces(state, ctx.faction);
    let last_bastions: BTreeSet<ProvinceId> = ctx
        .enemies
        .iter()
        .filter(|e| e.as_str() != REBELS)
        .flat_map(|enemy| {
            let held: Vec<ProvinceId> = state
                .provinces
                .iter()
                .filter(|(_, p)| &p.owner == enemy && &p.controller == enemy)
                .map(|(id, _)| id.clone())
                .collect();
            if held.len() <= LAST_BASTIONS {
                held
            } else {
                Vec::new()
            }
        })
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
                // Landings only for claimed provinces (England in France, not
                // the reverse).
                .filter(|(id, _)| claims.contains(*id) || !crosses_sea(data, &table, id))
                // F4: the last strongholds of a realm we hold no claim on are
                // left to the peace table (Scotland survives Edward III).
                .filter(|(id, _)| claims.contains(*id) || !last_bastions.contains(*id))
                .map(|(id, reach)| {
                    let mut value = ctx.province_income(id) / 100.0 + 10.0;
                    if enemy_capitals.contains(id) {
                        value += 50.0;
                    }
                    if claims.contains(id) {
                        value += 30.0;
                    }
                    if crosses_sea(data, &table, id) {
                        value *= SEA_INVASION_FACTOR;
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
                        && (claims.contains(*id) || !last_bastions.contains(*id))
                        && state.defensive_power(data, id) < power * 2.0
                        && ctx.threat(id) < power
                        && !crosses_sea(data, &table, id)
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
