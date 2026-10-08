//! Economic planning: taxes, demolitions and debt relief, the Church's
//! share, recruitment, the release of surplus garrisons and construction.

use std::collections::{BTreeMap, BTreeSet};

use data_model::{EffectKind, FactionId, GameData, SettlementId, SettlementKind, UnitTypeId};
use sim_campaign::population::weighted_unrest;
use sim_campaign::{ArmyId, CampaignState, Order, Season, TaxRate};

use super::Context;

/// What the recruitment leaves for the buildings.
struct Spending {
    /// Livres still free to spend.
    budget: i64,
    /// Seasonal upkeep of the armies once this turn's recruits are paid.
    planned_upkeep: i64,
}

/// The faction's economy for the turn, stage by stage.
pub(super) fn plan_economy(ctx: &Context, orders: &mut Vec<Order>) {
    plan_tax_rate(ctx, orders);
    // RS-C: a realm whose buildings outgrew its income raze the least useful.
    orders.extend(plan_demolitions(ctx));
    if relieve_debt(ctx, orders) {
        return;
    }
    donate_to_church(ctx, orders);
    // JR1: a host that holds no city at all (based in a town of another
    // realm's province) recruits in, and musters from, the places it holds.
    let cityless = !ctx
        .state
        .settlements
        .iter()
        .any(|(id, s)| ctx.holds(s) && ctx.is_city(id));
    // PB3f: the free supply before this turn's recruits, computed once for
    // the recruitment and building options of every settlement (all held
    // by the faction: `owns_settlement`).
    let free_supply = ctx.state.free_supply(ctx.data, ctx.faction);
    let spending = recruit(ctx, cityless, &free_supply, orders);
    release_surplus_garrisons(ctx, cityless, orders);
    construct(ctx, spending, &free_supply, orders);
}

/// Turns of deficit the treasury must cover before units are dismissed.
fn runway_turns(ctx: &Context) -> i64 {
    if ctx.at_war() {
        ctx.rules.economy.war_runway_turns
    } else {
        ctx.rules.economy.peace_runway_turns
    }
}

/// Taxes: heavy in war or deficit if public order allows, light when the
/// realm grumbles (F4: a rich treasury is spent, not untaxed).
fn plan_tax_rate(ctx: &Context, orders: &mut Vec<Order>) {
    let (state, data, eco) = (ctx.state, ctx.data, &ctx.rules.economy);
    let me = &state.factions[ctx.faction];
    let (weighted, population) = state
        .provinces
        .iter()
        .filter(|(id, _)| state.controls_province(ctx.faction, id))
        .map(|(_, p)| p)
        .fold((0.0, 0.0), |(w, n), p| {
            let size = p.population.total() as f64;
            (w + weighted_unrest(&p.population) * size, n + size)
        });
    let unrest = if population > 0.0 {
        weighted / population
    } else {
        0.0
    };
    let in_debt = ctx.treasury < 0;
    // EQ1: no heavy taxes while a province is on the brink of revolt.
    let brink = data.population_rules.revolt_unrest_threshold - eco.revolt_margin;
    let revolt_risk = state
        .provinces
        .iter()
        .filter(|(id, _)| state.controls_province(ctx.faction, id))
        .any(|(_, p)| weighted_unrest(&p.population) > brink);
    // EQ2: a war is paid from the reserve first; heavy taxes come when the
    // treasury falls below `reserve_seasons` of income, or when a deficit
    // would empty it within the runway (82 % of the « High » samples were
    // wars, most with a full treasury: 42-45 % of the samples).
    let low_treasury = ctx.treasury < eco.reserve_seasons * ctx.gross_income.max(0);
    let uncovered_deficit = ctx.surplus() < 0 && ctx.treasury < -ctx.surplus() * runway_turns(ctx);
    // EQ5: heavy taxes already levied stay while the budget would fall back
    // into deficit at the normal rate and the treasury holds less than a
    // season of income, and (with the debt) while the realm only grumbles:
    // without this the rate flipped every season around the threshold and a
    // realm whose buildings outgrew its income never left the red.
    let levied = me.tax_rate == TaxRate::High;
    let normal_surplus = if levied {
        ctx.surplus()
            - (ctx.gross_income.max(0) as f64
                * (1.0 - 1.0 / TaxRate::High.multiplier(&ctx.data.economy_rules)))
                as i64
    } else {
        ctx.surplus()
    };
    let stay_high = levied && ctx.treasury < ctx.gross_income.max(0) && normal_surplus < 0;
    let needs_money = in_debt || uncovered_deficit || stay_high || (ctx.at_war() && low_treasury);
    let max_unrest = if levied && (in_debt || stay_high) {
        eco.high_tax_max_unrest + eco.high_tax_hysteresis
    } else {
        eco.high_tax_max_unrest
    };
    let rate = if unrest > eco.low_tax_min_unrest {
        TaxRate::Low
    } else if needs_money && unrest < max_unrest && !revolt_risk {
        TaxRate::High
    } else {
        TaxRate::Normal
    };
    if rate != me.tax_rate {
        orders.push(Order::SetTaxRate { rate });
    }
}

/// Debt: dismiss the costliest unit until the surplus repays the debt
/// within `debt_repayment_turns`. F4: dismiss ahead of bankruptcy when the
/// treasury no longer covers the deficit for a few seasons. `true` when
/// units are dismissed (the rest of the economy waits).
fn relieve_debt(ctx: &Context, orders: &mut Vec<Order>) -> bool {
    let runway = runway_turns(ctx);
    let wanted_surplus = if ctx.treasury < 0 {
        -ctx.treasury / ctx.rules.economy.debt_repayment_turns
    } else if ctx.surplus() < 0
        && (ctx.treasury < -ctx.surplus() * runway || (!ctx.at_war() && ctx.hoard() == 0))
    {
        0
    } else {
        i64::MIN
    };
    if ctx.surplus() >= wanted_surplus {
        return false;
    }
    let savings = wanted_surplus - ctx.surplus();
    orders.extend(disband_for_debt(ctx, savings));
    true
}

/// Idle money (F4): a yearly gift to the Church while its favour is low.
fn donate_to_church(ctx: &Context, orders: &mut Vec<Order>) {
    let (state, eco) = (ctx.state, &ctx.rules.economy);
    let hoard = ctx.hoard();
    let catholic = sim_campaign::religion::is_catholic(state, ctx.data, ctx.faction);
    if hoard > 0
        && catholic
        && state.factions[ctx.faction].papal_favor < eco.donation_favor
        && state.season == Season::Autumn
        && ctx.faction.as_str() != sim_campaign::diplomacy::PAPACY_FACTION
    {
        let amount = hoard * eco.donation_percent / 100;
        if amount >= eco.donation_min {
            orders.push(Order::DonateToChurch { amount });
        }
    }
}

/// Where the faction recruits, in order: the capital's city first, then the
/// cities (and castles) of threatened border provinces. A cityless host
/// recruits in every place it holds that is not a village.
fn recruit_sites(ctx: &Context, cityless: bool) -> Vec<SettlementId> {
    let state = ctx.state;
    let me = &state.factions[ctx.faction];
    if cityless {
        return state
            .settlements
            .iter()
            .filter(|(_, s)| ctx.holds(s) && s.kind != SettlementKind::Village)
            .map(|(id, _)| id.clone())
            .collect();
    }
    // LR-15: the seat, never another realm's city (cityless faction).
    let mut sites: Vec<SettlementId> = state.faction_seat(ctx.faction).into_iter().collect();
    let mut borders: Vec<(SettlementId, i64)> = state
        .provinces
        .keys()
        .filter(|id| ctx.owns(id) && **id != me.capital && ctx.is_border(id))
        .flat_map(|id| {
            let threat = -(ctx.threat(id) as i64);
            state
                .settlements_of(id)
                .filter(|(_, s)| {
                    ctx.holds(s) && matches!(s.kind, SettlementKind::City | SettlementKind::Castle)
                })
                .map(move |(sid, _)| (sid.clone(), threat))
                .collect::<Vec<_>>()
        })
        .collect();
    borders.sort_by(|a, b| a.1.cmp(&b.1).then_with(|| a.0.cmp(&b.0)));
    let seat = sites.first().cloned();
    sites.extend(
        borders
            .into_iter()
            .map(|(id, _)| id)
            .filter(|id| Some(id) != seat.as_ref()),
    );
    sites
}

/// Seasonal upkeep the armies may reach: armies are paid from what
/// buildings leave of the net income, plus the hoard spent over
/// `hoard_spending_turns`; small realms live within their means.
/// Lot C7a: garrisons are a fixed cost of the realm like buildings (a
/// hundred places since the settlements); the share applies to what both
/// leave, and the garrisons are paid on top.
/// LR-04: the capital city's garrison is where recruits muster, not a fixed
/// cost: counted on top, every recruit kept there raised the target by as
/// much as it cost, and a county's garrison grew until it ate the whole net
/// income (no margin left for an event or a war).
fn target_upkeep(ctx: &Context, hoard: i64) -> i64 {
    let eco = &ctx.rules.economy;
    let share = if ctx.at_war() {
        eco.war_military_share
    } else {
        eco.peace_military_share
    };
    let garrisons = garrison_upkeep_outside_capital(ctx).min(ctx.army_upkeep);
    garrisons
        + ((ctx.income - ctx.building_upkeep - garrisons).max(0) as f64 * share) as i64
        + hoard / eco.hoard_spending_turns
}

/// The running state of this turn's recruitment.
struct Recruiting<'c, 'a> {
    ctx: &'c Context<'a>,
    budget: i64,
    planned_upkeep: i64,
    /// Seasonal upkeep the armies may reach.
    target_upkeep: i64,
    recruits: usize,
    max_recruits: usize,
    /// Regiments by unit type, for the doctrine's shares.
    composition: BTreeMap<UnitTypeId, u32>,
    /// SV2: the resource units this turn's recruits draw (siege engines:
    /// wood, iron) leave the faction's free supply; the next ones of the
    /// same kind are priced with their import (B7c rule, ADR 0053).
    supply: BTreeMap<data_model::ResourceId, u32>,
}

impl<'c, 'a> Recruiting<'c, 'a> {
    fn new(ctx: &'c Context<'a>, free_supply: &BTreeMap<data_model::ResourceId, u32>) -> Self {
        let (state, data, eco) = (ctx.state, ctx.data, &ctx.rules.economy);
        let hoard = ctx.hoard();
        // B7b: recruits still training (`recruit_time_turns` > 1) are already
        // paid for and will soon cost their upkeep; count them like this
        // turn's recruits (same measure: the unit type's `upkeep`).
        let training_upkeep: i64 = state
            .settlements
            .iter()
            .filter(|(_, s)| ctx.holds(s))
            .flat_map(|(_, s)| s.recruit_queue.iter())
            .filter_map(|r| data.unit_types.get(&r.unit_type))
            .map(|t| i64::from(t.upkeep))
            .sum();
        let mut composition = crate::doctrine::field_composition(state, ctx.faction);
        // A6-L3b: garrisons count too, for the share caps (a faction with no
        // field army raised militia into its garrisons turn after turn).
        for settlement in state.settlements.values().filter(|s| ctx.holds(s)) {
            for unit in &settlement.garrison {
                *composition.entry(unit.unit_type.clone()).or_default() += 1;
            }
        }
        Recruiting {
            ctx,
            budget: ctx.treasury - ctx.reserve(),
            planned_upkeep: ctx.army_upkeep + training_upkeep,
            target_upkeep: target_upkeep(ctx, hoard),
            recruits: 0,
            // G2: a hoard buys troops at its own pace (a crushed realm
            // sitting on ransoms and loot raises companies, it does not
            // bank them).
            max_recruits: (ctx.income / eco.income_per_recruit
                + hoard / eco.hoard_spending_turns / eco.hoard_livres_per_recruit)
                .clamp(1, eco.max_recruits_per_turn) as usize,
            composition,
            supply: free_supply.clone(),
        }
    }

    /// Recruits at `site` among `options` while the budget, the upkeep and
    /// the slots allow. At peace, two recruits end the turn's effort there.
    fn fill_site(
        &mut self,
        site: &SettlementId,
        options: &[sim_campaign::RecruitOption],
        orders: &mut Vec<Order>,
    ) {
        let ctx = self.ctx;
        let (state, data) = (ctx.state, ctx.data);
        let Some(recruit_context) =
            sim_campaign::RecruitContext::new(state, data, ctx.faction, site)
        else {
            return;
        };
        // G1: no more than the settlement's free recruitment slots.
        let mut free_slots = recruit_context.slots_free();
        // TW2-T2: no more of a unit type than the settlement's reserve.
        let mut drawn: BTreeMap<UnitTypeId, u32> = BTreeMap::new();
        while self.recruits < self.max_recruits && free_slots > 0 {
            let upkeep_cap = |upkeep: i64| {
                if self.planned_upkeep == 0 && ctx.surplus() >= upkeep {
                    self.target_upkeep.max(upkeep)
                } else {
                    self.target_upkeep
                }
            };
            let repriced = recruit_context.reprice(options, &self.supply);
            let fitting: Vec<&sim_campaign::RecruitOption> = repriced
                .iter()
                .filter(|o| {
                    self.planned_upkeep + i64::from(o.upkeep) <= upkeep_cap(i64::from(o.upkeep))
                        && self.budget >= i64::from(o.cost)
                        && drawn.get(&o.unit_type).copied().unwrap_or(0) < o.pool.available
                })
                .collect();
            let Some(option) = crate::doctrine::pick_recruit(
                data,
                ctx.faction,
                &fitting,
                &self.composition,
                |o| unit_value(data, &o.unit_type, o.cost),
            ) else {
                break;
            };
            orders.push(Order::Recruit {
                settlement: site.into(),
                unit_type: option.unit_type.clone(),
            });
            *self
                .composition
                .entry(option.unit_type.clone())
                .or_default() += 1;
            *drawn.entry(option.unit_type.clone()).or_default() += 1;
            draw_supply(&mut self.supply, &option.resources);
            self.budget -= i64::from(option.cost);
            self.planned_upkeep += i64::from(option.upkeep);
            self.recruits += 1;
            free_slots -= 1;
            if !ctx.at_war() && self.recruits.is_multiple_of(2) {
                return;
            }
        }
    }
}

/// Recruitment, site by site, within the upkeep and the budget.
fn recruit(
    ctx: &Context,
    cityless: bool,
    free_supply: &BTreeMap<data_model::ResourceId, u32>,
    orders: &mut Vec<Order>,
) -> Spending {
    let (state, data) = (ctx.state, ctx.data);
    let mut recruiting = Recruiting::new(ctx, free_supply);
    let sites = recruit_sites(ctx, cityless);
    // PB3f: the recruitment options of every site (a read of the state),
    // on the planner's pool; the loop below spends the budget in order.
    let site_options: Vec<Vec<sim_campaign::RecruitOption>> = ctx.mode.map(&sites, |site| {
        if !ctx.owns_settlement(site) {
            return Vec::new();
        }
        state
            .recruitable_with_supply(data, site, free_supply)
            .into_iter()
            .filter(|o| o.available)
            .collect()
    });
    for (site, options) in sites.iter().zip(site_options) {
        // E1: the doctrine's mix decides, among what fits the budget.
        if ctx.owns_settlement(site) && !options.is_empty() {
            recruiting.fill_site(site, &options, orders);
        }
    }
    Spending {
        budget: recruiting.budget,
        planned_upkeep: recruiting.planned_upkeep,
    }
}

/// Garrisons of cities beyond need become field armies (merged next turn);
/// the need grows with the threat around the city (lot C4). The small
/// garrisons of the other settlements stay where they are.
fn release_surplus_garrisons(ctx: &Context, cityless: bool, orders: &mut Vec<Order>) {
    let (state, data) = (ctx.state, ctx.data);
    for (id, settlement) in &state.settlements {
        if !ctx.holds(settlement) || settlement.siege.is_some() {
            continue;
        }
        let city = ctx.is_city(id);
        if !city && !cityless {
            continue;
        }
        // P1: one unit less than the garrison of the same role at the 1337
        // start, so the starting garrisons stay put; one more under threat.
        // JR1: a cityless host keeps the starting garrison of the place's
        // kind (recruits and landed contingents march out).
        let mut keep = if city {
            state
                .garrison_role(data, ctx.faction, &settlement.province)
                .garrison_size()
                - 1
        } else {
            data.settlement_rules
                .as_ref()
                .and_then(|r| r.starting_garrison.get(&settlement.kind))
                .map_or(1, |units| units.len().max(1))
        };
        if ctx.threat_at(id) > 0.0 {
            keep += 1;
        }
        if settlement.garrison.len() > keep + 1 {
            // NT5 (N6): at most `max_units` per army; the rest forms other
            // armies (the garrison shifts down as each one marches out).
            let cap = data.army_rules.cap().max(1);
            let mut left = settlement.garrison.len() - keep;
            while left > 0 {
                let size = left.min(cap);
                orders.push(Order::CreateArmy {
                    settlement: id.into(),
                    units_from_garrison: (keep..keep + size).collect(),
                    general: None,
                });
                left -= size;
            }
        }
    }
}

/// Construction: best yield per livre; the richer, the more sites at once.
fn construct(
    ctx: &Context,
    spending: Spending,
    free_supply: &BTreeMap<data_model::ResourceId, u32>,
    orders: &mut Vec<Order>,
) {
    let (data, eco) = (ctx.data, &ctx.rules.economy);
    let Spending {
        mut budget,
        planned_upkeep,
    } = spending;
    let hoard = ctx.hoard();
    let builds =
        (1 + (budget / eco.livres_per_extra_build).max(0) as usize).min(eco.max_builds_per_turn);
    // F4: a new building's upkeep must fit in the surplus left by the army
    // (or in the hoard being spent).
    // EQ5: with the safety margin kept, and within a share of the gross
    // income (the net one shrinks with a hoard's opulence): buildings
    // cannot be dismissed when times turn bad.
    let mut spare = ctx.surplus() - (planned_upkeep - ctx.army_upkeep)
        + hoard / eco.hoard_spending_turns
        - ctx.safety_margin();
    // (Buildings that pay for themselves in taxes or trade escape the cap.)
    let mut upkeep_room =
        ctx.gross_income * eco.max_building_upkeep_percent / 100 - ctx.building_upkeep;
    let options = ranked_builds(ctx, free_supply);
    let mut used = BTreeSet::new();
    for (_, settlement, building, cost) in options {
        let upkeep = data
            .buildings
            .get(&building)
            .map_or(0, |b| i64::from(b.upkeep.unwrap_or(0)));
        if used.len() >= builds || used.contains(&settlement) || budget < cost || upkeep > spare {
            continue;
        }
        let pays_for_itself = building_income(ctx, &settlement, &building) >= upkeep as f64;
        if !pays_for_itself {
            if upkeep > upkeep_room {
                continue;
            }
            upkeep_room -= upkeep;
        }
        spare -= upkeep;
        budget -= cost;
        used.insert(settlement.clone());
        orders.push(Order::Build {
            settlement: settlement.into(),
            building,
        });
    }
}

/// Every building that could be started on a settlement held with no works
/// under way, best yield per livre first: (yield, settlement, building, cost).
fn ranked_builds(
    ctx: &Context,
    free_supply: &BTreeMap<data_model::ResourceId, u32>,
) -> Vec<(f64, SettlementId, data_model::BuildingId, i64)> {
    let (state, data) = (ctx.state, ctx.data);
    // PB3f: each settlement's options are valued on the planner's pool, then
    // gathered in the settlements' order (the sort below is stable).
    let idle: Vec<(&SettlementId, &sim_campaign::SettlementState)> = state
        .settlements
        .iter()
        .filter(|(_, s)| ctx.holds(s) && s.construction.is_none())
        .collect();
    let mut options: Vec<_> = ctx
        .mode
        .map(&idle, |(id, settlement)| {
            let Some(province) = state.provinces.get(&settlement.province) else {
                return Vec::new();
            };
            let unrest = weighted_unrest(&province.population);
            let health = f64::from(province.population.peasants.health);
            state
                .buildable_with_supply(data, id, free_supply)
                .iter()
                .filter(|o| o.available)
                .filter_map(|option| {
                    let value = building_value(ctx, id, &option.building, unrest, health);
                    (value > 0.0).then(|| {
                        (
                            value / f64::from(option.cost.max(1)),
                            (*id).clone(),
                            option.building.clone(),
                            i64::from(option.cost),
                        )
                    })
                })
                .collect::<Vec<_>>()
        })
        .into_iter()
        .flatten()
        .collect();
    options.sort_by(|a, b| {
        b.0.total_cmp(&a.0)
            .then_with(|| a.1.cmp(&b.1))
            .then_with(|| a.2.cmp(&b.2))
    });
    options
}

/// Seasonal upkeep of the faction's garrisons (share paid by the crown by
/// settlement kind; reliefs, technologies and coinage left out).
pub(super) fn garrison_upkeep(ctx: &Context) -> i64 {
    use sim_campaign::economy::{garrison_share, unit_upkeep};
    let capital = ctx.state.faction_capital_city(ctx.faction);
    ctx.state
        .settlements
        .iter()
        .filter(|(_, s)| &s.controller == ctx.faction)
        .map(|(id, s)| {
            garrison_share(
                ctx.data,
                s.kind,
                capital == Some(id),
                s.garrison.iter().map(|u| unit_upkeep(ctx.data, u)),
            )
        })
        .sum()
}

/// [`garrison_upkeep`] without the capital city's garrison (LR-04).
fn garrison_upkeep_outside_capital(ctx: &Context) -> i64 {
    use sim_campaign::economy::{garrison_share, unit_upkeep};
    let capital = ctx.state.faction_capital_city(ctx.faction);
    ctx.state
        .settlements
        .iter()
        .filter(|(id, s)| &s.controller == ctx.faction && capital != Some(*id))
        .map(|(_, s)| {
            garrison_share(
                ctx.data,
                s.kind,
                false,
                s.garrison.iter().map(|u| unit_upkeep(ctx.data, u)),
            )
        })
        .sum()
}

/// Power per livre of a unit type.
fn unit_value(data: &GameData, unit_type: &UnitTypeId, cost: u32) -> f64 {
    data.unit_types.get(unit_type).map_or(0.0, |t| {
        let attack = f64::from(t.stats.melee.max(t.stats.ranged));
        let defence = f64::from(t.stats.armor) / 2.0 + f64::from(t.stats.morale) / 4.0;
        f64::from(t.soldiers) * (attack + defence) / f64::from(cost.max(1))
    })
}

/// EQ5: seasonal taxes and trade `building` adds in `settlement`.
fn building_income(
    ctx: &Context,
    settlement: &SettlementId,
    building: &data_model::BuildingId,
) -> f64 {
    let Some(def) = ctx.data.buildings.get(building) else {
        return 0.0;
    };
    let income = ctx
        .province_of(settlement)
        .map_or(0.0, |p| ctx.province_income(p));
    def.effects
        .iter()
        .filter(|e| matches!(e.effect, EffectKind::TaxIncome | EffectKind::TradeIncome))
        .map(|e| {
            if matches!(e.mode, data_model::EffectMode::Percent) {
                income * e.value / 100.0
            } else {
                e.value
            }
        })
        .sum()
}

/// Seasonal value (livres-equivalent) of building `building` in `settlement`.
fn building_value(
    ctx: &Context,
    settlement: &SettlementId,
    building: &data_model::BuildingId,
    unrest: f64,
    health: f64,
) -> f64 {
    let Some(def) = ctx.data.buildings.get(building) else {
        return 0.0;
    };
    let Some(province) = ctx.province_of(settlement) else {
        return 0.0;
    };
    let income = ctx.province_income(province);
    // RS-B: a secondary place's buildings weigh on the whole province and on
    // research as the rules count them (`province_effect_percent`, `research_percent`).
    let kind = ctx.state.settlement_kind(settlement);
    let province_weight = f64::from(sim_campaign::buildings::province_effect_percent(
        ctx.data, kind,
    )) / 100.0;
    let research_weight =
        f64::from(sim_campaign::buildings::research_percent(ctx.data, kind)) / 100.0;
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
            EffectKind::Unrest if unrest > 30.0 => -v * province_weight * income / 50.0,
            EffectKind::Health if health < 50.0 => v * province_weight * income / 80.0,
            EffectKind::Growth | EffectKind::Wealth => v * province_weight * income / 150.0,
            EffectKind::ResearchPoints => v * research_weight * 60.0,
            EffectKind::Garrison | EffectKind::FortificationLevel if ctx.is_border(province) => {
                v * 20.0
            }
            _ => 0.0,
        };
    }
    value - f64::from(def.upkeep.unwrap_or(0)) * 1.5
}

/// RS-C: after `ai_demolition.deficit_seasons` seasons of deficit in a row,
/// razes the buildings that do not pay for themselves, least value per livre
/// of upkeep first, while their upkeep exceeds `max_upkeep_percent` of the
/// gross income (at most `max_per_turn`). Buildings built before the
/// construction cap (EQ5) otherwise sank the Swiss into bankruptcy.
fn plan_demolitions(ctx: &Context) -> Vec<Order> {
    let state = ctx.state;
    let data = ctx.data;
    let rules = &data.economy_rules.ai_demolition;
    // OMR R3 (ADR 0117): also in a debt the net surplus cannot repay within
    // `debt_repayment_turns` — the simulation's deficit count leaves out the
    // tribute to the suzerain and the agents, which kept small vassals in
    // the red for decades without ever razing a building.
    let stuck =
        ctx.treasury < 0 && ctx.surplus() < -ctx.treasury / ctx.rules.economy.debt_repayment_turns;
    if state.factions[ctx.faction].deficit_seasons < rules.deficit_seasons && !stuck {
        return Vec::new();
    }
    // (value per livre of upkeep, upkeep, settlement, building)
    let mut candidates: Vec<(f64, i64, SettlementId, data_model::BuildingId)> = Vec::new();
    let mut burden = 0;
    for (id, settlement) in state
        .settlements
        .iter()
        .filter(|(_, s)| ctx.holds(s) && s.siege.is_none())
    {
        let Some(province) = state.provinces.get(&settlement.province) else {
            continue;
        };
        let unrest = weighted_unrest(&province.population);
        let health = f64::from(province.population.peasants.health);
        let percent = sim_campaign::economy::building_upkeep_percent(data, settlement.kind);
        for building in &settlement.buildings {
            let upkeep = data
                .buildings
                .get(building)
                .map_or(0, |b| i64::from(b.upkeep.unwrap_or(0)))
                * percent
                / 100;
            if upkeep <= 0 || building_income(ctx, id, building) >= upkeep as f64 {
                continue;
            }
            burden += upkeep;
            if sim_campaign::buildings::demolition_blocker(state, data, id, building).is_some() {
                continue;
            }
            let value = building_value(ctx, id, building, unrest, health);
            candidates.push((value / upkeep as f64, upkeep, id.clone(), building.clone()));
        }
    }
    let limit = ctx.gross_income.max(0) * rules.max_upkeep_percent / 100;
    if burden <= limit {
        return Vec::new();
    }
    candidates.sort_by(|a, b| {
        a.0.total_cmp(&b.0)
            .then_with(|| a.2.cmp(&b.2))
            .then_with(|| a.3.cmp(&b.3))
    });
    let mut orders = Vec::new();
    for (_, upkeep, settlement, building) in candidates {
        if burden <= limit || orders.len() >= rules.max_per_turn {
            break;
        }
        burden -= upkeep;
        orders.push(Order::Demolish {
            settlement: settlement.into(),
            building,
        });
    }
    orders
}

/// In debt, dismisses the costliest units until `savings` livres of upkeep
/// are saved (at most `max_disbands_per_turn`): field units first, then
/// garrisons. The capital keeps its last unit and besieged places keep theirs;
/// an army losing its last unit is disbanded.
fn disband_for_debt(ctx: &Context, savings: i64) -> Vec<Order> {
    use sim_campaign::economy::{garrison_upkeep_percent, unit_upkeep};
    enum Holder {
        Army(ArmyId),
        Garrison(SettlementId),
    }
    let state = ctx.state;
    let seat = state.faction_seat(ctx.faction);
    let capital = seat.as_ref();
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
    for (id, settlement) in state
        .settlements
        .iter()
        .filter(|(_, s)| &s.controller == ctx.faction && s.siege.is_none())
    {
        let keep = usize::from(Some(id) == capital);
        if settlement.garrison.len() <= keep {
            continue;
        }
        let holder = holders.len();
        holders.push(Holder::Garrison(id.clone()));
        let mut units: Vec<(i64, usize)> = settlement
            .garrison
            .iter()
            .enumerate()
            .map(|(index, unit)| {
                (
                    unit_upkeep(ctx.data, unit)
                        * garrison_upkeep_percent(ctx.data, settlement.kind)
                        / 100,
                    index,
                )
            })
            .collect();
        // The cheapest `keep` units stay.
        units.sort_by_key(|(upkeep, index)| (std::cmp::Reverse(*upkeep), *index));
        units.truncate(settlement.garrison.len() - keep);
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
        if saved >= savings || chosen.len() >= ctx.rules.economy.max_disbands_per_turn {
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
                settlement: None,
                unit_index,
            },
            Holder::Garrison(settlement) => Order::DisbandUnit {
                army: None,
                settlement: Some(settlement.into()),
                unit_index,
            },
        })
        .collect()
}

/// SV2: `options` priced against the free resource `supply` left by the
/// recruits already planned this turn — a second trebuchet imports the wood
/// the first one drew (B7c rule, ADR 0053). Options without resources keep
/// their price.
pub fn reprice_recruits(
    state: &CampaignState,
    data: &GameData,
    faction: &FactionId,
    settlement: &SettlementId,
    options: &[sim_campaign::RecruitOption],
    supply: &BTreeMap<data_model::ResourceId, u32>,
) -> Vec<sim_campaign::RecruitOption> {
    sim_campaign::RecruitContext::new(state, data, faction, settlement).map_or_else(
        || options.to_vec(),
        |context| context.reprice(options, supply),
    )
}

/// SV2: removes `needed` from the running `supply` (what is lacking is
/// imported, not owed).
pub fn draw_supply(
    supply: &mut BTreeMap<data_model::ResourceId, u32>,
    needed: &BTreeMap<data_model::ResourceId, u32>,
) {
    for (resource, amount) in needed {
        if let Some(left) = supply.get_mut(resource) {
            *left = left.saturating_sub(*amount);
        }
    }
}
