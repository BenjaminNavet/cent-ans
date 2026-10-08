//! Strategic campaign AI (M9 spec § 1).
//!
//! [`plan_turn`] is pure and deterministic: it reads the state and returns the
//! faction's orders. It layers, in order: diplomacy and research (reused from
//! `sim-campaign`), economy (taxes, debt, recruitment, construction),
//! characters (governors, generals, skills, marriages) and one military
//! objective per army (defend, besiege, raid, regroup, retreat).
//!
//! Objectives are settlements. Taking a city gives the province, so hostile
//! cities come first, then the other settlements by weight and
//! fortification; garrisons are kept according to the threat around each
//! settlement. The numeric thresholds live in `data/ai/campaign.json`
//! ([`data_model::AiCampaign`]). Economy, characters and armies are planned
//! in the submodules.

use std::collections::{BTreeMap, BTreeSet};

use data_model::{AiCampaign, FactionId, GameData, ProvinceId, SettlementId, SettlementKind};
use sim_campaign::coinage::CoinageLevel;
use sim_campaign::movement::edges;
use sim_campaign::plan_cache::PlanCache;
use sim_campaign::{ArmyId, CampaignState, Order, TaxRate};

use crate::parallel::Mode;

mod army;
mod characters;
mod economy;

pub use economy::{draw_supply, reprice_recruits};

/// A realm down to this many free provinces is not besieged there by an
/// enemy without a claim on them: peace decides its fate (F4).
pub const LAST_BASTIONS: usize = sim_campaign::negotiation::LAST_BASTIONS;

/// Everything the planner derives once per faction and turn.
struct Context<'a> {
    state: &'a CampaignState,
    /// Memoised faction power, borders, income and rivals of `state`.
    cache: &'a PlanCache<'a>,
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
    /// Lot M2: the settlement standing for every army on the settlement
    /// graph (its own, or the nearest one in the field).
    anchors: BTreeMap<ArmyId, SettlementId>,
    /// Lot M3: routes on the settlement graph and orders on the grid.
    grid: crate::grid::GridPlanner<'a>,
    /// PB3f: sequential reference or planner's pool (ADR 0091).
    mode: Mode,
    rules: &'static AiCampaign,
    /// AD12: power of the hostile armies standing in each province, in army
    /// order, built once (the threat questions are asked hundreds of times
    /// per plan).
    hostile_power: BTreeMap<ProvinceId, Vec<f64>>,
}

impl<'a> Context<'a> {
    /// PB3f: `upkeep` is (army, building) upkeep, computed once by the
    /// caller; the grid planner and the army anchors are built concurrently
    /// in [`Mode::Parallel`].
    fn new(
        mode: Mode,
        cache: &'a PlanCache<'a>,
        data: &'a GameData,
        faction: &'a FactionId,
        upkeep: (i64, i64),
    ) -> Option<Self> {
        let state = cache.state();
        let me = state.factions.get(faction)?;
        let aggression = data
            .factions
            .get(faction)
            .and_then(|f| f.ai_personality.as_ref())
            .and_then(|p| p.aggression)
            .map_or(50, i32::from);
        let (grid, (anchors, gross_income)) = mode.join(
            || crate::grid::GridPlanner::with_mode(mode, cache, data, faction),
            || {
                let armies: Vec<(&ArmyId, &sim_campaign::Army)> = state.armies.iter().collect();
                let anchors: BTreeMap<ArmyId, SettlementId> = mode
                    .map(&armies, |(id, a)| {
                        Some(((*id).clone(), state.army_anchor(data, a)?))
                    })
                    .into_iter()
                    .flatten()
                    .collect();
                (anchors, cache.faction_income(data, faction))
            },
        );
        // Net of court and administration (M10 balance) and of the tribute
        // owed to a suzerain.
        let tribute = if me.suzerain.is_some() {
            (gross_income * data.feudal_rules.vassal_tribute_percent / 100).max(0)
        } else {
            0
        };
        let income = gross_income
            - state.administration_upkeep_for(data, faction, gross_income)
            - tribute
            - commitments(state, faction);
        Some(Context {
            anchors,
            grid,
            mode,
            rules: AiCampaign::bundled(),
            hostile_power: hostile_power_by_province(state, data, faction),
            state,
            cache,
            data,
            faction,
            enemies: me.at_war_with.clone(),
            aggression,
            income,
            gross_income,
            army_upkeep: upkeep.0,
            building_upkeep: upkeep.1,
            treasury: me.treasury,
        })
    }

    fn at_war(&self) -> bool {
        self.enemies.iter().any(|e| !e.is_rebels())
    }

    fn upkeep(&self) -> i64 {
        self.army_upkeep + self.building_upkeep
    }

    /// Treasury kept aside: two turns of upkeep (JR4: and, for the
    /// crusade, the price of the next passage once it can be preached).
    fn reserve(&self) -> i64 {
        2 * self.upkeep()
            + sim_campaign::crusade::ai_passage_reserve(self.state, self.data, self.faction)
    }

    /// Seasonal surplus (negative: deficit) at the current upkeep.
    fn surplus(&self) -> i64 {
        self.income - self.upkeep()
    }

    /// EQ5: surplus kept against events (fines, fires, lost harvests).
    fn safety_margin(&self) -> i64 {
        self.gross_income.max(0) * self.rules.economy.safety_margin_percent / 100
    }

    /// Treasury above `reserve_seasons` of gross income: idle money the
    /// faction spends over `hoard_spending_turns` (F4).
    fn hoard(&self) -> i64 {
        (self.treasury - self.rules.economy.reserve_seasons * self.gross_income.max(0)).max(0)
    }

    fn owns(&self, province: &ProvinceId) -> bool {
        self.state.holds_province(self.faction, province)
    }

    /// `settlement` is owned and held by the faction.
    fn owns_settlement(&self, settlement: &SettlementId) -> bool {
        self.state
            .settlements
            .get(settlement)
            .is_some_and(|s| self.holds(s))
    }

    /// [`Context::owns_settlement`] on a settlement already read (OMR R1:
    /// the walks over the settlements do not look each one up again).
    fn holds(&self, settlement: &sim_campaign::SettlementState) -> bool {
        &settlement.owner == self.faction && &settlement.controller == self.faction
    }

    /// `settlement` is owned de jure by the faction (held or lost).
    fn owned_de_jure(&self, settlement: &SettlementId) -> bool {
        self.state
            .settlements
            .get(settlement)
            .is_some_and(|s| &s.owner == self.faction)
    }

    /// Estimated seasonal tax of `province` at the normal rate, its places'
    /// buildings weighing their kind's `province_effect_percent` (lot RS-B:
    /// a market in a village no longer counts as one in the city).
    fn province_income(&self, province: &ProvinceId) -> f64 {
        self.state.provinces.get(province).map_or(0.0, |p| {
            sim_campaign::economy::province_income(
                self.data,
                p,
                &[],
                TaxRate::Normal,
                &self.state.province_building_effects(self.data, province),
            )
        })
    }

    /// Tax share of a settlement (its province's income times its weight).
    fn settlement_income(&self, settlement: &SettlementId) -> f64 {
        self.state
            .settlement_province(settlement)
            .map_or(0.0, |p| self.province_income(p))
            * sim_campaign::settlements::weight_share(self.data, settlement)
    }

    /// Hostile army power inside or next to `province`.
    fn threat(&self, province: &ProvinceId) -> f64 {
        std::iter::once(province)
            .chain(self.data.province_land_neighbors(province).iter())
            .filter_map(|p| self.hostile_power.get(p))
            .flatten()
            .sum()
    }

    /// Hostile army power anchored on `settlement` or one edge away (lot
    /// M2: an army in the field counts at its nearest settlement).
    /// Lot SL1: an army across the sea counts for `sea_threat_factor`.
    fn threat_at(&self, settlement: &SettlementId) -> f64 {
        let mut nodes = vec![settlement.clone()];
        nodes.extend(edges(self.data, settlement).into_iter().map(|(s, _)| s));
        self.anchors
            .iter()
            .filter(|(_, anchor)| nodes.contains(anchor))
            .filter(|(id, _)| {
                self.state
                    .armies
                    .get(*id)
                    .is_some_and(|a| self.state.is_at_war(self.faction, &a.faction))
            })
            .map(|(id, anchor)| {
                let power = self.state.army_power(self.data, id);
                if sim_campaign::movement::is_sea_crossing(self.data, anchor, settlement) {
                    power * self.rules.military.sea_threat_factor
                } else {
                    power
                }
            })
            .sum()
    }

    /// P1: the frontier classification of the campaign (ports and
    /// provinces next to another faction), shared with the 1337 setup.
    fn is_border(&self, province: &ProvinceId) -> bool {
        self.state.is_frontier(self.data, self.faction, province)
    }

    /// `settlement` is the city of its province.
    fn is_city(&self, settlement: &SettlementId) -> bool {
        self.state.settlement_kind(settlement) == SettlementKind::City
    }

    fn province_of(&self, settlement: &SettlementId) -> Option<&ProvinceId> {
        self.state.settlement_province(settlement)
    }
}

/// Power of the armies at war with `faction`, by the province they stand in
/// (armies in id order, like [`CampaignState::hostile_armies_in`]).
fn hostile_power_by_province(
    state: &CampaignState,
    data: &GameData,
    faction: &FactionId,
) -> BTreeMap<ProvinceId, Vec<f64>> {
    let mut by_province: BTreeMap<ProvinceId, Vec<f64>> = BTreeMap::new();
    for (id, army) in &state.armies {
        if !state.is_at_war(faction, &army.faction) {
            continue;
        }
        if let Some(province) = state.army_province(data, army) {
            by_province
                .entry(province)
                .or_default()
                .push(state.army_power(data, id));
        }
    }
    by_province
}

/// EQ5: seasonal charges the treasury pays whatever it holds, besides the
/// army, buildings and administration: tributes of a lost war (a Granada
/// paying England, Scotland and Holstein went bankrupt for decades, its
/// budget blind to them) and agents. Ransom installments and the diets
/// (« Table ») are left out: they are only paid when the treasury can.
fn commitments(state: &CampaignState, faction: &FactionId) -> i64 {
    let tributes: i64 = state.factions.get(faction).map_or(0, |me| {
        me.ledger.tributes.iter().map(|t| t.per_season).sum()
    });
    let agents = state
        .agents
        .upkeep_last_turn
        .get(faction)
        .copied()
        .unwrap_or(0);
    tributes + agents
}

/// Orders of `faction` for this turn (PB3f: independent read-only work on
/// the planner's pool, same orders as [`plan_turn_sequential`]).
pub fn plan_turn(state: &CampaignState, data: &GameData, faction: &FactionId) -> Vec<Order> {
    plan_turn_in(Mode::Parallel, state, data, faction)
}

/// [`plan_turn`] entirely on the calling thread: the reference of the
/// equality tests (ADR 0091).
pub fn plan_turn_sequential(
    state: &CampaignState,
    data: &GameData,
    faction: &FactionId,
) -> Vec<Order> {
    plan_turn_in(Mode::Sequential, state, data, faction)
}

/// The planners that read the state alone (diplomacy, treaties, gifts,
/// research, diets, edicts, coinage, ransoms, chivalry, agents), in the
/// order their orders are issued.
struct StatePlans {
    diplomacy: Vec<Order>,
    money_fief: Option<Order>,
    embargoes: Vec<Order>,
    subsidies: Vec<Order>,
    research: Option<Order>,
    diets: Vec<Order>,
    edicts: Vec<Order>,
    coinage: Vec<Order>,
    ransoms: Vec<Order>,
    chivalry: Vec<Order>,
    agents: Vec<Order>,
}

fn gift_amount(order: &Order) -> i64 {
    match order {
        Order::SendGift { amount, .. } => *amount,
        _ => 0,
    }
}

/// PB3f: [`StatePlans`] in four concurrent groups. The subsidies are paid
/// out of the treasury left by the money fief (`upkeep`: army, buildings).
fn state_plans(
    mode: Mode,
    cache: &PlanCache,
    data: &GameData,
    faction: &FactionId,
    upkeep: (i64, i64),
) -> StatePlans {
    let state = cache.state();
    let ((diplomacy, (money_fief, embargoes, subsidies)), ((research, diets, edicts), rest)) = mode
        .join(
            || {
                mode.join(
                    || {
                        let mut orders =
                            sim_campaign::diplomacy::plan_diplomacy(cache, data, faction);
                        // FE5: « survival first » for the counties, titles
                        // demanded at the peace, feudal acts.
                        crate::feudal::filter_suicidal_wars(cache, data, faction, &mut orders);
                        crate::feudal::demand_titles(cache, data, faction, &mut orders);
                        orders.extend(crate::feudal::plan_feudal(cache, data, faction));
                        // DP1: trade agreements and military access (ADR 0025).
                        orders.extend(crate::diplomacy_eval::plan_treaties(cache, data, faction));
                        // G2: historical side changes (Artevelde, Troyes).
                        orders.extend(crate::alignment::plan_side_change(cache, data, faction));
                        orders.extend(crate::alignment::plan_dynastic_alliance(
                            cache, data, faction,
                        ));
                        orders
                    },
                    || {
                        let money_fief = crate::alignment::plan_money_fief(cache, data, faction);
                        let embargoes = crate::alignment::lift_embargoes_on_cobelligerents(
                            cache, data, faction,
                        );
                        // G2: subsidies first, out of what the donor would
                        // otherwise hoard.
                        let treasury = state.factions.get(faction).map_or(0, |f| f.treasury)
                            - money_fief.as_ref().map_or(0, gift_amount);
                        let spare = (treasury - upkeep.0 - upkeep.1).max(0)
                            / crate::support::SUBSIDY_SPARE_DIVISOR;
                        let subsidies = crate::support::plan_subsidies(cache, data, faction, spare);
                        (money_fief, embargoes, subsidies)
                    },
                )
            },
            || {
                mode.join(
                    || {
                        (
                            sim_campaign::research::ai_choose_research(state, data, faction)
                                .map(|technology| Order::Research { technology }),
                            sim_campaign::table::ai_choose_diets(state, data, faction),
                            sim_campaign::edicts::ai_choose_edicts(state, data, faction),
                        )
                    },
                    || {
                        (
                            sim_campaign::coinage::ai_choose_coinage(cache, data, faction),
                            sim_campaign::ransom::ai_ransom_orders(cache, data, faction),
                            {
                                // JR1: the crusader faction preaches the
                                // passage as soon as it can.
                                let mut orders =
                                    sim_campaign::chivalry::ai_found_order(cache, data, faction);
                                orders
                                    .extend(sim_campaign::crusade::ai_preach(state, data, faction));
                                orders
                            },
                            // C6: spies, heralds and preachers (recruitment
                            // keeps a reserve).
                            sim_campaign::agents::plan_agents(cache, data, faction),
                        )
                    },
                )
            },
        );
    let (coinage, ransoms, chivalry, agents) = rest;
    StatePlans {
        diplomacy,
        money_fief,
        embargoes,
        subsidies,
        research,
        diets,
        edicts,
        coinage,
        ransoms,
        chivalry: chivalry.into_iter().collect(),
        agents,
    }
}

/// The orders of the state-only planners, in the order they are issued; the
/// gifts among them leave the treasury the economy then plans with.
fn state_orders(ctx: &mut Context, plans: StatePlans) -> Vec<Order> {
    let mut orders = plans.diplomacy;
    if let Some(order) = plans.money_fief {
        ctx.treasury -= gift_amount(&order);
        orders.push(order);
    }
    orders.extend(plans.embargoes);
    for order in plans.subsidies {
        ctx.treasury -= gift_amount(&order);
        orders.push(order);
    }
    orders.extend(plans.research);
    orders.extend(plans.diets);
    orders.extend(plans.edicts);
    // G2: a realm whose buildings eat half its income does not debase: the
    // inflation of their upkeep outweighs the seigniorage (Scots spiral).
    // EQ5: a third is enough: the prices stay up after the money is sound
    // again (Swiss buildings 158 → 201 after two years of debasement).
    // F8: the garrisons are a fixed cost inflated alike (a county whose
    // single garrison eats its income sank into a spiral of debasement and
    // debt: Connacht, prices 100 -> 360 in 50 turns).
    let fixed_upkeep = ctx.building_upkeep + economy::garrison_upkeep(ctx).min(ctx.army_upkeep);
    let upkeep_heavy = 3 * fixed_upkeep > ctx.gross_income;
    orders.extend(plans.coinage.into_iter().filter(|o| {
        !upkeep_heavy
            || !matches!(
                o,
                Order::SetCoinage {
                    level: CoinageLevel::Debased | CoinageLevel::HeavilyDebased
                }
            )
    }));
    orders.extend(plans.ransoms);
    orders.extend(plans.chivalry);
    orders.extend(plans.agents);
    orders
}

fn plan_turn_in(
    mode: Mode,
    state: &CampaignState,
    data: &GameData,
    faction: &FactionId,
) -> Vec<Order> {
    // FE5: the core asks the feudal decisions of this crate from now on.
    crate::feudal::install();
    // The state is read-only for the whole plan: repeated questions
    // (faction power, neighbours, income, rivals) are answered once.
    let cache = PlanCache::new(state);
    if faction.is_rebels() || !state.factions.get(faction).is_some_and(|f| f.alive) {
        return Vec::new();
    }
    let upkeep = (
        state.faction_upkeep(data, faction),
        state.faction_building_upkeep(data, faction),
    );
    let (ctx, plans) = mode.join(
        || Context::new(mode, &cache, data, faction, upkeep),
        || state_plans(mode, &cache, data, faction, upkeep),
    );
    let Some(mut ctx) = ctx else {
        return Vec::new();
    };
    let mut orders = state_orders(&mut ctx, plans);
    economy::plan_economy(&ctx, &mut orders);
    characters::plan_characters(&ctx, &mut orders);
    // TW2-T3: companies for the threatened armies of a rich realm, hired
    // where they stand before they march.
    let treasury = ctx.treasury - planned_spending(data, &orders);
    orders.extend(crate::mercenaries::plan_hires(
        state,
        data,
        faction,
        treasury,
        ctx.gross_income,
        |army| {
            ctx.anchors
                .get(army)
                .map_or(0.0, |anchor| ctx.threat_at(anchor))
        },
    ));
    // TW2-T5: ranks of the armies spent on traditions.
    orders.extend(crate::traditions::plan_traditions(
        ctx.state,
        ctx.data,
        ctx.faction,
    ));
    army::plan_armies(&ctx, &mut orders);
    orders
}

/// TW2-T3: livres this turn's recruitments and constructions will spend
/// (base prices: an estimate for the mercenary budget).
fn planned_spending(data: &GameData, orders: &[Order]) -> i64 {
    orders
        .iter()
        .map(|order| match order {
            Order::Recruit { unit_type, .. } => data
                .unit_types
                .get(unit_type)
                .map_or(0, |t| i64::from(t.cost.money)),
            Order::Build { building, .. } => data
                .buildings
                .get(building)
                .map_or(0, |b| i64::from(b.cost.money)),
            _ => 0,
        })
        .sum()
}
