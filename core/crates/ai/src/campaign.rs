//! Strategic campaign AI (M9 spec § 1).
//!
//! [`plan_turn`] is pure and deterministic: it reads the state and returns the
//! faction's orders. It layers, in order: diplomacy and research (reused from
//! `sim-campaign`), economy (taxes, debt, recruitment, construction),
//! characters (governors, generals, skills, marriages) and one military
//! objective per army (defend, besiege, raid, regroup, retreat).
//!
//! Lot C4: objectives are settlements. Taking a city gives the province, so
//! hostile cities come first, then the other settlements by weight and
//! fortification; garrisons are kept according to the threat around each
//! settlement. Balancing is left to lot C7.

use std::collections::{BTreeMap, BTreeSet};

use data_model::{
    CharacterId, EffectKind, FactionId, GameData, ProvinceId, SettlementId, SettlementKind,
    SkillBranch, SkillId, UnitTypeId,
};
use sim_campaign::coinage::CoinageLevel;
use sim_campaign::movement::{edges, points_per_step};
use sim_campaign::passage;
use sim_campaign::population::weighted_unrest;
use sim_campaign::{ArmyId, CampaignState, Order, Season, Stance, TaxRate};

use crate::parallel::Mode;

/// Maximum path cost considered for an objective, in province steps (times
/// `MovementRules::points_per_step`). Lot DC3 (ADR 0082): 5 → 10 when the step went
/// from 140 to 70 km, so that the AI weighs the same 700 km as before (at 5 steps, half
/// the cities it used to besiege fell out of its sight and conquests slowed by half).
pub const PLANNING_RANGE: u32 = 10;
/// Siege value bonus of a city (it hands over the province, lot C4).
pub const CITY_TARGET_BONUS: f64 = 25.0;
/// Siege value lost per fortification level of the target.
pub const FORTIFICATION_TARGET_PENALTY: f64 = 3.0;
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
/// Weighted unrest above which the AI no longer raises taxes to « Haut »
/// (lot G1/E2: heavy taxes now bite; 18 after E2, 30 again with EQ1's
/// higher baseline unrest).
pub const HIGH_TAX_MAX_UNREST: f64 = 30.0;
/// EQ1: a province this close to the revolt threshold keeps the realm off
/// « Haut » taxes.
pub const REVOLT_MARGIN: f64 = 10.0;
/// EQ5: unrest tolerated above [`HIGH_TAX_MAX_UNREST`] while « Haut » taxes
/// already levied are still needed.
pub const HIGH_TAX_HYSTERESIS: f64 = 10.0;
/// EQ5: seasonal surplus kept as a margin against events, in percent of
/// gross income.
pub const SAFETY_MARGIN_PERCENT: i64 = 5;
/// EQ5: buildings may not take more than this share (percent) of the gross
/// income in upkeep: they cannot be dismissed when times turn bad (a
/// plague, a lost province).
pub const MAX_BUILDING_UPKEEP_PERCENT: i64 = 30;
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
/// EQ5: an army trespassing with no place of its own within the planning
/// range looks this many times farther for the way home.
pub const HOMEWARD_RANGE_FACTOR: u32 = 4;
/// EQ5: places of its own tried (nearest first) before giving up a way home.
const HOMEWARD_CANDIDATES: usize = 3;
/// Siege value bonus of a settlement the faction owns de jure but an enemy
/// holds (lot C7a: win back lost places first, above a throne claim's 30).
pub const RECLAIM_TARGET_BONUS: f64 = 35.0;
/// A siege of a fortress (level [`FORTRESS_LEVEL`] or more) still far from
/// starving the garrison after this many turns is given up (lot C7a).
pub const SIEGE_PATIENCE_TURNS: u32 = 12;
pub const FORTRESS_LEVEL: u32 = 4;
/// An army leaves one unit as the garrison of an empty place it holds on a
/// border or took from an enemy, when it has at least this many (lot C7a).
pub const GARRISON_MIN_ARMY_UNITS: usize = 3;

/// Seasons of gross income kept in the treasury; above, money is spent (F4).
pub const RESERVE_SEASONS: i64 = 3;
/// Turns over which a hoard above the reserve is spent (army, buildings).
pub const HOARD_SPENDING_TURNS: i64 = 4;
/// Hoard spent per extra recruit and turn, and the recruitment cap (G2).
pub const HOARD_LIVRES_PER_RECRUIT: i64 = 1500;
pub const MAX_RECRUITS_PER_TURN: i64 = 16;
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
    /// Lot M2: the settlement standing for every army on the settlement
    /// graph (its own, or the nearest one in the field).
    anchors: BTreeMap<ArmyId, SettlementId>,
    /// Lot M3: routes on the settlement graph and orders on the grid.
    grid: crate::grid::GridPlanner<'a>,
    /// PB3f: sequential reference or planner's pool (ADR 0091).
    mode: Mode,
}

impl<'a> Context<'a> {
    /// PB3f: `upkeep` is (army, building) upkeep, computed once by the
    /// caller; the grid planner and the army anchors are built concurrently
    /// in [`Mode::Parallel`].
    fn new(
        mode: Mode,
        state: &'a CampaignState,
        data: &'a GameData,
        faction: &'a FactionId,
        upkeep: (i64, i64),
    ) -> Option<Self> {
        let me = state.factions.get(faction)?;
        let aggression = data
            .factions
            .get(faction)
            .and_then(|f| f.ai_personality.as_ref())
            .and_then(|p| p.aggression)
            .map_or(50, i32::from);
        let (grid, (anchors, gross_income)) = mode.join(
            || crate::grid::GridPlanner::with_mode(mode, state, data, faction),
            || {
                let armies: Vec<(&ArmyId, &sim_campaign::Army)> = state.armies.iter().collect();
                let anchors: BTreeMap<ArmyId, SettlementId> = mode
                    .map(&armies, |(id, a)| {
                        Some(((*id).clone(), state.army_anchor(data, a)?))
                    })
                    .into_iter()
                    .flatten()
                    .collect();
                (anchors, state.faction_income_effective(data, faction))
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
            state,
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

    /// EQ5: surplus kept against events (fines, fires, lost harvests).
    fn safety_margin(&self) -> i64 {
        self.gross_income.max(0) * SAFETY_MARGIN_PERCENT / 100
    }

    /// Treasury above [`RESERVE_SEASONS`] of gross income: idle money the
    /// faction spends over [`HOARD_SPENDING_TURNS`] (F4).
    fn hoard(&self) -> i64 {
        (self.treasury - RESERVE_SEASONS * self.gross_income.max(0)).max(0)
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
            sim_campaign::economy::province_income_with(
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
        let mut provinces = vec![province.clone()];
        provinces.extend(self.data.province_land_neighbors(province).iter().cloned());
        provinces
            .iter()
            .flat_map(|p| self.state.hostile_armies_in(self.data, self.faction, p))
            .map(|id| self.state.army_power(self.data, &id))
            .sum()
    }

    /// Hostile army power anchored on `settlement` or one edge away (lot
    /// M2: an army in the field counts at its nearest settlement).
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
            .map(|(id, _)| self.state.army_power(self.data, id))
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
    state: &CampaignState,
    data: &GameData,
    faction: &FactionId,
    upkeep: (i64, i64),
) -> StatePlans {
    let ((diplomacy, (money_fief, embargoes, subsidies)), ((research, diets, edicts), rest)) = mode
        .join(
            || {
                mode.join(
                    || {
                        let mut orders =
                            sim_campaign::diplomacy::plan_diplomacy(state, data, faction);
                        // FE5: « survival first » for the counties, titles
                        // demanded at the peace, feudal acts.
                        crate::feudal::filter_suicidal_wars(state, data, faction, &mut orders);
                        crate::feudal::demand_titles(state, data, faction, &mut orders);
                        orders.extend(crate::feudal::plan_feudal(state, data, faction));
                        // DP1: trade agreements and military access (ADR 0025).
                        orders.extend(crate::diplomacy_eval::plan_treaties(state, data, faction));
                        // G2: historical side changes (Artevelde, Troyes).
                        orders.extend(crate::alignment::plan_side_change(state, data, faction));
                        orders.extend(crate::alignment::plan_dynastic_alliance(
                            state, data, faction,
                        ));
                        orders
                    },
                    || {
                        let money_fief = crate::alignment::plan_money_fief(state, data, faction);
                        let embargoes = crate::alignment::lift_embargoes_on_cobelligerents(
                            state, data, faction,
                        );
                        // G2: subsidies first, out of what the donor would
                        // otherwise hoard.
                        let treasury = state.factions.get(faction).map_or(0, |f| f.treasury)
                            - money_fief.as_ref().map_or(0, gift_amount);
                        let spare = (treasury - upkeep.0 - upkeep.1).max(0)
                            / crate::support::SUBSIDY_SPARE_DIVISOR;
                        let subsidies = crate::support::plan_subsidies(state, data, faction, spare);
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
                            sim_campaign::coinage::ai_choose_coinage(state, data, faction),
                            sim_campaign::ransom::ai_ransom_orders(state, data, faction),
                            sim_campaign::chivalry::ai_found_order(state, data, faction),
                            // C6: spies, heralds and preachers (recruitment
                            // keeps a reserve).
                            sim_campaign::agents::plan_agents(state, data, faction),
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

fn plan_turn_in(
    mode: Mode,
    state: &CampaignState,
    data: &GameData,
    faction: &FactionId,
) -> Vec<Order> {
    // FE5: the core asks the feudal decisions of this crate from now on.
    crate::feudal::install();
    // OMR R1: the state is read-only for the whole plan; repeated questions
    // (faction power, neighbours) are answered from indexes built once.
    let _scope = state.planning_scope();
    if faction.as_str() == REBELS || !state.factions.get(faction).is_some_and(|f| f.alive) {
        return Vec::new();
    }
    let upkeep = (
        state.faction_army_upkeep(data, faction),
        state.faction_building_upkeep(data, faction),
    );
    let (ctx, plans) = mode.join(
        || Context::new(mode, state, data, faction, upkeep),
        || state_plans(mode, state, data, faction, upkeep),
    );
    let Some(mut ctx) = ctx else {
        return Vec::new();
    };
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
    let fixed_upkeep = ctx.building_upkeep + garrison_upkeep(&ctx).min(ctx.army_upkeep);
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
    plan_economy(&ctx, &mut orders);
    plan_characters(&ctx, &mut orders);
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
    plan_armies(&ctx, &mut orders);
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
    // Taxes: heavy in war or deficit if public order allows, light when
    // the realm grumbles (F4: a rich treasury is spent, not untaxed).
    let in_debt = ctx.treasury < 0;
    // EQ1: no heavy taxes while a province is on the brink of revolt.
    let brink = data.population_rules.revolt_unrest_threshold - REVOLT_MARGIN;
    let revolt_risk = state
        .provinces
        .iter()
        .filter(|(id, _)| state.controls_province(ctx.faction, id))
        .any(|(_, p)| weighted_unrest(&p.population) > brink);
    // EQ2: a war is paid from the reserve first; heavy taxes come when the
    // treasury falls below `RESERVE_SEASONS` of income, or when a deficit
    // would empty it within the runway (82 % of the « High » samples were
    // wars, most with a full treasury: 42-45 % of the samples).
    let low_treasury = ctx.treasury < RESERVE_SEASONS * ctx.gross_income.max(0);
    let runway = if ctx.at_war() {
        WAR_RUNWAY_TURNS
    } else {
        PEACE_RUNWAY_TURNS
    };
    let uncovered_deficit = ctx.surplus() < 0 && ctx.treasury < -ctx.surplus() * runway;
    // EQ5: heavy taxes already levied stay while the budget would fall back
    // into deficit at the normal rate and the treasury holds less than a
    // season of income, and (with the debt) while the realm only grumbles:
    // without this the rate
    // flipped every season around the threshold and a realm whose
    // buildings outgrew its income never left the red.
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
        HIGH_TAX_MAX_UNREST + HIGH_TAX_HYSTERESIS
    } else {
        HIGH_TAX_MAX_UNREST
    };
    let rate = if unrest > 55.0 {
        TaxRate::Low
    } else if needs_money && unrest < max_unrest && !revolt_risk {
        TaxRate::High
    } else {
        TaxRate::Normal
    };
    if rate != me.tax_rate {
        orders.push(Order::SetTaxRate { rate });
    }
    // RS-C: a realm whose buildings outgrew its income raze the least useful.
    orders.extend(plan_demolitions(ctx));

    // Debt: dismiss the costliest unit until the surplus repays the debt
    // within `DEBT_REPAYMENT_TURNS`. F4: dismiss ahead of bankruptcy when the
    // treasury no longer covers the deficit for a few seasons.
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
    // Lot C7a: garrisons are a fixed cost of the realm like buildings (a
    // hundred places since the settlements); the share applies to what
    // both leave, and the garrisons are paid on top.
    let garrisons = garrison_upkeep(ctx).min(ctx.army_upkeep);
    let target_upkeep = garrisons
        + ((ctx.income - ctx.building_upkeep - garrisons).max(0) as f64 * share) as i64
        + hoard / HOARD_SPENDING_TURNS;
    // B7b: recruits still training (`recruit_time_turns` > 1) are already
    // paid for and will soon cost their upkeep; count them like this turn's
    // recruits (same measure: the unit type's `upkeep`).
    let training_upkeep: i64 = state
        .settlements
        .iter()
        .filter(|(_, s)| ctx.holds(s))
        .flat_map(|(_, s)| s.recruit_queue.iter())
        .filter_map(|r| data.unit_types.get(&r.unit_type))
        .map(|t| i64::from(t.upkeep))
        .sum();
    let mut planned_upkeep = ctx.army_upkeep + training_upkeep;

    // Recruitment: the capital's city first, then the cities (and castles)
    // of threatened border provinces.
    let mut sites: Vec<SettlementId> = state
        .province_city_id(&me.capital)
        .cloned()
        .into_iter()
        .collect();
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
    sites.extend(borders.into_iter().map(|(id, _)| id));
    let mut recruits = 0;
    // G2: a hoard buys troops at its own pace (a crushed realm sitting on
    // ransoms and loot raises companies, it does not bank them).
    let max_recruits = (ctx.income / INCOME_PER_RECRUIT
        + hoard / HOARD_SPENDING_TURNS / HOARD_LIVRES_PER_RECRUIT)
        .clamp(1, MAX_RECRUITS_PER_TURN) as usize;
    let mut composition = crate::doctrine::field_composition(state, ctx.faction);
    // SV2: the resource units this turn's recruits draw (siege engines:
    // wood, iron) leave the faction's free supply; the next ones of the
    // same kind are priced with their import (B7c rule, ADR 0053).
    let mut supply = state.free_supply(data, ctx.faction);
    // PB3f: the free supply before this turn's recruits, computed once for
    // the recruitment and building options of every settlement (all held
    // by the faction: `owns_settlement`).
    let free_supply = supply.clone();
    // PB3f: the recruitment options of every site (a read of the state),
    // on the planner's pool; the loop below spends the budget in order.
    let site_options: Vec<Vec<sim_campaign::RecruitOption>> = ctx.mode.map(&sites, |site| {
        if !ctx.owns_settlement(site) {
            return Vec::new();
        }
        state
            .recruitable_with_supply(data, site, &free_supply)
            .into_iter()
            .filter(|o| o.available)
            .collect()
    });
    'sites: for (site, options) in sites.iter().zip(site_options) {
        if !ctx.owns_settlement(site) {
            continue;
        }
        // E1: the doctrine's mix decides, among what fits the budget.
        if options.is_empty() {
            continue;
        }
        // G1: no more than the settlement's free recruitment slots.
        let mut free_slots = state.recruit_slots_free(data, site);
        // TW2-T2: no more of a unit type than the settlement's reserve.
        let mut drawn: BTreeMap<data_model::UnitTypeId, u32> = BTreeMap::new();
        while recruits < max_recruits && free_slots > 0 {
            let upkeep_cap = |upkeep: i64| {
                if planned_upkeep == 0 && ctx.surplus() >= upkeep {
                    target_upkeep.max(upkeep)
                } else {
                    target_upkeep
                }
            };
            let repriced = reprice_recruits(state, data, ctx.faction, site, &options, &supply);
            let fitting: Vec<&sim_campaign::RecruitOption> = repriced
                .iter()
                .filter(|o| {
                    planned_upkeep + i64::from(o.upkeep) <= upkeep_cap(i64::from(o.upkeep))
                        && budget >= i64::from(o.cost)
                        && drawn.get(&o.unit_type).copied().unwrap_or(0) < o.pool.available
                })
                .collect();
            let Some(option) =
                crate::doctrine::pick_recruit(data, ctx.faction, &fitting, &composition, |o| {
                    unit_value(data, &o.unit_type, o.cost)
                })
            else {
                break;
            };
            orders.push(Order::Recruit {
                settlement: site.into(),
                unit_type: option.unit_type.clone(),
            });
            *composition.entry(option.unit_type.clone()).or_default() += 1;
            *drawn.entry(option.unit_type.clone()).or_default() += 1;
            draw_supply(&mut supply, &option.resources);
            budget -= i64::from(option.cost);
            planned_upkeep += i64::from(option.upkeep);
            recruits += 1;
            free_slots -= 1;
            if !ctx.at_war() && recruits % 2 == 0 {
                continue 'sites;
            }
        }
    }

    // Garrisons of cities beyond need become field armies (merged next
    // turn); the need grows with the threat around the city (lot C4). The
    // small garrisons of the other settlements stay where they are.
    for (id, settlement) in &state.settlements {
        if !ctx.holds(settlement) || settlement.siege.is_some() || !ctx.is_city(id) {
            continue;
        }
        // P1: one unit less than the garrison of the same role at the 1337
        // start, so the starting garrisons stay put; one more under threat.
        let mut keep = state
            .garrison_role(data, ctx.faction, &settlement.province)
            .garrison_size()
            - 1;
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

    // Construction: best yield per livre; the richer, the more sites at once.
    let builds = (1 + (budget / 15_000).max(0) as usize).min(6);
    // F4: a new building's upkeep must fit in the surplus left by the army
    // (or in the hoard being spent).
    // EQ5: with the safety margin kept, and within a share of the gross
    // income (the net one shrinks with a hoard's opulence): buildings
    // cannot be dismissed when times turn bad.
    let mut spare = ctx.surplus() - (planned_upkeep - ctx.army_upkeep)
        + hoard / HOARD_SPENDING_TURNS
        - ctx.safety_margin();
    // (Buildings that pay for themselves in taxes or trade escape the cap.)
    let mut upkeep_room =
        ctx.gross_income * MAX_BUILDING_UPKEEP_PERCENT / 100 - ctx.building_upkeep;
    // PB3f: each settlement's options are valued on the planner's pool, then
    // gathered in the settlements' order (the sort below is stable).
    let idle: Vec<(&SettlementId, &sim_campaign::SettlementState)> = state
        .settlements
        .iter()
        .filter(|(_, s)| ctx.holds(s) && s.construction.is_none())
        .collect();
    let mut options: Vec<(f64, SettlementId, data_model::BuildingId, i64)> = ctx
        .mode
        .map(&idle, |(id, settlement)| {
            let Some(province) = state.provinces.get(&settlement.province) else {
                return Vec::new();
            };
            let unrest = weighted_unrest(&province.population);
            let health = f64::from(province.population.peasants.health);
            state
                .buildable_with_supply(data, id, &free_supply)
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

/// Seasonal upkeep of the faction's garrisons (share paid by the crown by
/// settlement kind; reliefs, technologies and coinage left out).
fn garrison_upkeep(ctx: &Context) -> i64 {
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
    // `DEBT_REPAYMENT_TURNS` — the simulation's deficit count leaves out the
    // tribute to the suzerain and the agents, which kept small vassals in
    // the red for decades without ever razing a building.
    let stuck = ctx.treasury < 0 && ctx.surplus() < -ctx.treasury / DEBT_REPAYMENT_TURNS;
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
/// are saved (at most [`MAX_DISBANDS_PER_TURN`]): field units first, then
/// garrisons. The capital keeps its last unit and besieged places keep theirs;
/// an army losing its last unit is disbanded.
fn disband_for_debt(ctx: &Context, savings: i64) -> Vec<Order> {
    use sim_campaign::economy::{garrison_upkeep_percent, unit_upkeep};
    enum Holder {
        Army(ArmyId),
        Garrison(SettlementId),
    }
    let state = ctx.state;
    let capital = state.province_city_id(&state.factions[ctx.faction].capital);
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
        let army_province = state.army_province(ctx.data, army);
        let best = state
            .characters
            .iter()
            .filter(|(id, c)| {
                available(id)
                    && !busy.contains(*id)
                    && c.location.is_some()
                    && c.location == army_province
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

/// Lot C7a: one unit (the cheapest to keep) left as the garrison of an
/// empty, unbesieged place the army holds, when the place was taken from
/// an enemy or lies on a threatened border, and the army can spare it.
fn garrison_order(ctx: &Context, army_id: &ArmyId) -> Option<Order> {
    let state = ctx.state;
    let army = state.armies.get(army_id)?;
    if army.units.len() < GARRISON_MIN_ARMY_UNITS {
        return None;
    }
    let place = state.settlements.get(army.settlement()?)?;
    if &place.controller != ctx.faction
        || place.siege.is_some()
        || !place.garrison.is_empty()
        || place.kind == SettlementKind::Village
    {
        return None;
    }
    let conquered = &place.owner != ctx.faction;
    let exposed = ctx.is_border(&place.province) && ctx.threat(&place.province) > 0.0;
    if !conquered && !exposed {
        return None;
    }
    let index = army
        .units
        .iter()
        .enumerate()
        .min_by_key(|(i, u)| (sim_campaign::economy::unit_upkeep(ctx.data, u), *i))
        .map(|(i, _)| i)?;
    Some(Order::GarrisonUnits {
        army: army_id.clone(),
        unit_indices: vec![index],
    })
}

/// EQ5: at peace, an army standing in a place of its own inside the lands
/// of another realm (a castle held in a foreign province) joins the
/// garrison when the walls can hold it all, instead of camping there as a
/// field army without right of passage.
fn trespasser_garrison_order(ctx: &Context, army_id: &ArmyId) -> Option<Order> {
    let state = ctx.state;
    let army = state.armies.get(army_id)?;
    let place = state.settlements.get(army.settlement()?)?;
    if &place.controller != ctx.faction || place.siege.is_some() || army.units.is_empty() {
        return None;
    }
    let cap = ctx
        .data
        .settlement_rules
        .as_ref()
        .and_then(|r| r.garrison_cap.get(&place.kind))
        .copied()
        .unwrap_or(usize::MAX);
    if place.garrison.len() + army.units.len() > cap {
        return None;
    }
    Some(Order::GarrisonUnits {
        army: army_id.clone(),
        unit_indices: (0..army.units.len()).collect(),
    })
}

/// True when the planned path to `target` includes a sea crossing.
fn crosses_sea(
    data: &GameData,
    table: &BTreeMap<SettlementId, sim_campaign::movement::Reach>,
    target: &SettlementId,
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

    // Merge armies stationed together into the strongest one (NT5, N6:
    // within the unit cap; an army that does not fit stays apart and takes
    // the next ones).
    let cap = data.army_rules.cap();
    let mut merged: BTreeSet<ArmyId> = BTreeSet::new();
    let mut by_location: BTreeMap<SettlementId, Vec<(ArmyId, usize)>> = BTreeMap::new();
    for (id, _) in &armies {
        let Some(location) = state.armies[id].settlement().cloned() else {
            continue;
        };
        let size = state.armies[id].units.len();
        let targets = by_location.entry(location).or_default();
        match targets.iter_mut().find(|(_, units)| *units + size <= cap) {
            Some((target, units)) => {
                orders.push(Order::MergeArmies {
                    source: id.clone(),
                    target: target.clone(),
                });
                *units += size;
                merged.insert(id.clone());
            }
            None => targets.push((id.clone(), size)),
        }
    }
    let power_at = |settlement: &SettlementId| -> f64 {
        armies
            .iter()
            .filter(|(id, _)| ctx.anchors.get(id) == Some(settlement))
            .map(|(_, p)| p)
            .sum()
    };
    let step = points_per_step(data).max(1.0);
    let range = (f64::from(PLANNING_RANGE) * step).round() as u32;
    // Province of a node of the Dijkstra table.
    let province = |s: &SettlementId| state.settlement_province(s).cloned();

    // F4: a throne claim makes every province of that crown a claimed target.
    let claims = sim_campaign::diplomacy::claimed_provinces(state, ctx.faction);
    let last_bastions: BTreeSet<ProvinceId> = ctx
        .enemies
        .iter()
        .filter(|e| e.as_str() != REBELS)
        .flat_map(|enemy| {
            let held: Vec<ProvinceId> = state
                .provinces
                .keys()
                .filter(|id| state.holds_province(enemy, id))
                .cloned()
                .collect();
            if held.len() <= LAST_BASTIONS {
                held
            } else {
                Vec::new()
            }
        })
        .collect();
    let enemy_capitals: BTreeSet<SettlementId> = ctx
        .enemies
        .iter()
        .filter_map(|e| state.factions.get(e))
        .filter_map(|f| state.province_city_id(&f.capital).cloned())
        .collect();
    let my_capital = state
        .factions
        .get(ctx.faction)
        .and_then(|f| state.province_city_id(&f.capital))
        .cloned();
    let largest = armies.first().map(|(id, p)| (id.clone(), *p));
    let mut defended: BTreeSet<SettlementId> = BTreeSet::new();
    let mut targeted: BTreeSet<SettlementId> = BTreeSet::new();
    // CV3-6: a siege of ours or against us in progress (no encounter detours).
    let realm_besieged = state.settlements.values().any(|s| {
        s.siege
            .as_ref()
            .is_some_and(|siege| &s.controller == ctx.faction || &siege.attacker == ctx.faction)
    });

    // Armies whose units are already being dismissed this turn keep their
    // unit indices untouched.
    let disbanding: BTreeSet<ArmyId> = orders
        .iter()
        .filter_map(|o| match o {
            Order::DisbandUnit {
                army: Some(army), ..
            } => Some(army.clone()),
            _ => None,
        })
        .collect();
    // Armies losing every unit this turn: gone before any later order.
    let emptied: BTreeSet<&ArmyId> = disbanding
        .iter()
        .filter(|army| {
            let dismissed = orders
                .iter()
                .filter(|o| matches!(o, Order::DisbandUnit { army: Some(a), .. } if a == *army))
                .count();
            state
                .armies
                .get(*army)
                .is_some_and(|a| dismissed >= a.units.len())
        })
        .collect();

    // PB3f: the route tables the loop below asks for, computed ahead on
    // the planner's pool (a memo of the grid planner: same tables).
    let table_keys: Vec<(SettlementId, u32, u32, f64)> = armies
        .iter()
        .filter(|(id, _)| !merged.contains(id))
        .filter_map(|(id, _)| {
            let anchor = ctx.anchors.get(id)?;
            let cap = state.army_movement_allowance(data, &state.armies[id]);
            Some((anchor.clone(), range, cap, power_at(anchor)))
        })
        .collect();
    ctx.grid.prefetch_tables(ctx.mode, &table_keys);

    for (army_id, _) in armies
        .iter()
        .filter(|(id, _)| !merged.contains(id) && !emptied.contains(id))
    {
        let army = &state.armies[army_id];
        // EQ5: standing without right of passage in the lands of a realm at
        // peace (after a peace, or in a place of its own inside a foreign
        // province).
        let trespassing = state
            .army_province(data, army)
            .and_then(|p| passage::trespassed_owner(state, ctx.faction, &p));
        if trespassing.is_some() && !disbanding.contains(army_id) && !ctx.at_war() {
            if let Some(order) = trespasser_garrison_order(ctx, army_id) {
                orders.push(order);
                continue;
            }
        }
        if !disbanding.contains(army_id) {
            if let Some(order) = garrison_order(ctx, army_id) {
                orders.push(order);
            }
        }
        let Some(anchor) = ctx.anchors.get(army_id).cloned() else {
            continue;
        };
        let power = power_at(&anchor);
        let strength: u32 = army.units.iter().map(|u| u.strength).sum();
        let max_strength: u32 = army.units.iter().map(|u| u.max_strength).sum();
        let cap = state.army_movement_allowance(data, army);
        // Lot M3: cached for the turn; stronger enemy armies are avoided.
        let table = ctx.grid.table(&anchor, range, cap, power);
        let steps = |cost: u32| f64::from(cost) / step;
        let besieging = army
            .settlement()
            .and_then(|s| state.settlements.get(s))
            .and_then(|s| s.siege.as_ref())
            .is_some_and(|s| &s.attacker == ctx.faction);
        let here = state.army_province(data, army);

        let mut choice: Option<(Objective, SettlementId)> = None;

        // 5. Retreat when broken or starving in winter.
        let broken =
            max_strength > 0 && f64::from(strength) < RETREAT_STRENGTH * f64::from(max_strength);
        let starving = state.season == Season::Winter
            && army.supply < 30
            && here
                .as_ref()
                .is_some_and(|p| state.is_hostile_territory(ctx.faction, p));
        if (broken || starving) && !besieging {
            let home = table
                .iter()
                .filter(|(id, _)| ctx.owns_settlement(id) && ctx.threat_at(id) < power)
                .min_by_key(|(id, reach)| (reach.cost, (*id).clone()))
                .map(|(id, _)| id.clone());
            if let Some(home) = home {
                choice = Some((Objective::Retreat, home));
            }
        }

        // Lot C7a: give up a fortress that holds out far beyond patience.
        let hopeless = besieging
            && state.fortification_level(data, &anchor) >= FORTRESS_LEVEL
            && state
                .settlements
                .get(&anchor)
                .and_then(|s| s.siege.as_ref())
                .is_some_and(|s| s.turns_elapsed >= SIEGE_PATIENCE_TURNS && s.turns_left > 2)
            && !state
                .assault_odds(data, army_id)
                .is_some_and(|(odds, _)| odds >= ASSAULT_ODDS);
        if hopeless {
            targeted.insert(anchor.clone());
        }

        // Lot M3: engage an enemy army within the bubble when the odds are
        // good. The strategic orders below still follow: they fail harmlessly
        // once the battle has spent the army's movement, and apply when the
        // attack was refused (target out of reach).
        // CV3-6: an army in ambush waits while its prey still comes and it
        // is not discovered; it does not attack by itself.
        if army.stance == Stance::Ambush {
            if crate::stances::keep_ambush(state, data, ctx.faction, army_id) {
                continue;
            }
            orders.push(Order::SetStance {
                army: army_id.clone(),
                stance: Stance::Normal,
            });
        }
        // NT6c: a weakened army on friendly ground with no enemy near rests
        // to be replenished; it leaves the rest camp once rebuilt.
        if choice.is_none() && !broken && !besieging {
            match crate::stances::rest_plan(state, data, ctx.faction, army_id) {
                crate::stances::RestPlan::Rest { entrench } => {
                    if entrench && army.stance != Stance::Entrenched {
                        orders.push(Order::SetStance {
                            army: army_id.clone(),
                            stance: Stance::Entrenched,
                        });
                    }
                    continue;
                }
                crate::stances::RestPlan::Leave => {
                    let threat = here.as_ref().map_or(0.0, |p| ctx.threat(p));
                    if !crate::stances::should_entrench(state, data, ctx.faction, army_id, threat) {
                        orders.push(Order::SetStance {
                            army: army_id.clone(),
                            stance: Stance::Normal,
                        });
                    }
                }
                crate::stances::RestPlan::None => {}
            }
        }
        let mut attacked = false;
        if !broken && !besieging {
            if let Some(order) = ctx.grid.attack_order(army_id) {
                orders.push(order);
                attacked = true;
            }
        }
        // CV3-6: lie in wait for a stronger enemy marching on our lands.
        if choice.is_none() && !broken && !besieging && !attacked && ctx.at_war() {
            if let Some(ambush) =
                crate::stances::ambush_orders(state, data, ctx.faction, army_id, ctx.aggression)
            {
                orders.extend(ambush);
                continue;
            }
        }

        // Keep a siege that is going our way.
        if choice.is_none() && besieging && !hopeless && ctx.threat_at(&anchor) < power * 1.2 {
            targeted.insert(anchor.clone());
            // Storm the walls when the odds are good (M8) and, behind
            // standing walls, an engine is ready (NT5, N7: built meanwhile).
            if state
                .assault_odds(data, army_id)
                .is_some_and(|(odds, _)| odds >= ASSAULT_ODDS)
                && state.assault_blocker(data, army_id).is_none()
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

        // 1. Defend a threatened friendly settlement.
        if choice.is_none() {
            choice = table
                .iter()
                .filter(|(id, _)| {
                    state.is_friendly_settlement(ctx.faction, id) && !defended.contains(*id)
                })
                .filter_map(|(id, reach)| {
                    let threat = ctx.threat_at(id);
                    let local = state.settlement_defensive_power(data, id);
                    (threat > local * 0.8 && power >= DEFENCE_RATIO * threat).then(|| {
                        let weight = if Some(id) == my_capital.as_ref() {
                            3.0
                        } else if ctx.is_city(id) {
                            2.0
                        } else {
                            1.0
                        };
                        let value = weight * (ctx.settlement_income(id) + threat)
                            / (1.0 + steps(reach.cost));
                        (value, id.clone())
                    })
                })
                .max_by(|a, b| a.0.total_cmp(&b.0).then_with(|| b.1.cmp(&a.1)))
                .map(|(_, id)| (Objective::Defend, id));
        }

        // 2. Besiege the most valuable weak enemy settlement: cities first
        // (they hand over the province), then the others by weight and
        // fortification (lot C4).
        if choice.is_none() {
            choice = table
                .iter()
                .filter(|(id, _)| {
                    state.is_hostile_settlement(ctx.faction, id) && !targeted.contains(*id)
                })
                .filter(|(id, _)| {
                    state.settlement_defensive_power(data, id) * SIEGE_SUPERIORITY < power
                })
                .filter_map(|(id, reach)| province(id).map(|p| (id, reach, p)))
                // Landings only for claimed provinces (England in France, not
                // the reverse).
                .filter(|(id, _, p)| {
                    claims.contains(p) || ctx.owned_de_jure(id) || !crosses_sea(data, &table, id)
                })
                // F4: the last strongholds of a realm we hold no claim on are
                // left to the peace table (Scotland survives Edward III).
                .filter(|(id, _, p)| {
                    claims.contains(p) || ctx.owned_de_jure(id) || !last_bastions.contains(p)
                })
                .map(|(id, reach, p)| {
                    let mut value = ctx.settlement_income(id) / 100.0 + 10.0;
                    if ctx.is_city(id) {
                        value += CITY_TARGET_BONUS;
                    }
                    if ctx.owned_de_jure(id) {
                        value += RECLAIM_TARGET_BONUS;
                    }
                    value -= FORTIFICATION_TARGET_PENALTY
                        * f64::from(state.fortification_level(data, id));
                    if enemy_capitals.contains(id) {
                        value += 50.0;
                    }
                    if claims.contains(&p) {
                        value += 30.0;
                    }
                    if crosses_sea(data, &table, id) {
                        value *= SEA_INVASION_FACTOR;
                    }
                    (value.max(1.0) / (1.0 + steps(reach.cost) / 2.0), id.clone())
                })
                .max_by(|a, b| a.0.total_cmp(&b.0).then_with(|| b.1.cmp(&a.1)))
                .map(|(_, id)| (Objective::Siege, id));
        }

        // 3. Chevauchée when too weak to besiege (aggressive factions).
        if choice.is_none() && ctx.aggression >= 65 && ctx.at_war() {
            choice = table
                .iter()
                .filter_map(|(id, reach)| province(id).map(|p| (id, reach, p)))
                .filter(|(_, _, p)| state.is_hostile_territory(ctx.faction, p))
                .filter(|(id, _, p)| {
                    state.provinces[p].devastation < 50
                        && (claims.contains(p) || !last_bastions.contains(p))
                        && state.defensive_power(data, p) < power * 2.0
                        && ctx.threat(p) < power
                        && !crosses_sea(data, &table, id)
                })
                .map(|(id, reach, _)| (id, reach))
                .min_by_key(|(id, reach)| (reach.cost, (*id).clone()))
                .map(|(id, _)| (Objective::Raid, id.clone()));
        }

        // EQ5: with nothing to fight for here, an army trespassing on the
        // lands of a realm at peace goes home: a place of its own outside
        // closed lands first (within the planning range, then farther
        // across those lands), else any place of its own, and only a place
        // the grid can actually reach (a road over water is no way home).
        if choice.is_none() && !besieging && trespassing.is_some() {
            let far = ctx
                .grid
                .homeward_table(&anchor, range * HOMEWARD_RANGE_FACTOR, cap, power);
            let homes = |table: &crate::grid::Table, abroad_ok: bool| -> Vec<SettlementId> {
                let mut homes: Vec<(u32, SettlementId)> = table
                    .iter()
                    .filter(|(id, _)| {
                        ctx.owns_settlement(id)
                            && state.hostile_armies_at(ctx.faction, id).is_empty()
                            && (abroad_ok
                                || ctx.province_of(id).is_none_or(|p| {
                                    passage::trespassed_owner(state, ctx.faction, p).is_none()
                                }))
                    })
                    .map(|(id, reach)| (reach.cost, id.clone()))
                    .collect();
                homes.sort();
                homes.into_iter().map(|(_, id)| id).collect()
            };
            // Same land mass on the grid (a cheap check: no path search).
            let grid = data.navgrid();
            let here = state.army_cell(data, army);
            let land = grid.component(i64::from(here.x), i64::from(here.y));
            let reachable = |target: &SettlementId| {
                data.settlement_point(target).is_some_and(|p| {
                    let cell = sim_campaign::Cell::of_point(grid, p);
                    let (x, y) = (i64::from(cell.x), i64::from(cell.y));
                    grid.passable(x, y) && grid.component(x, y) == land
                })
            };
            let found = [(&table, false), (&far, false), (&table, true), (&far, true)]
                .into_iter()
                .find_map(|(route, abroad_ok)| {
                    homes(route, abroad_ok)
                        .into_iter()
                        .take(HOMEWARD_CANDIDATES)
                        .find(|id| reachable(id))
                        .map(|id| (route, id))
                });
            if let Some((route, target)) = found {
                if army.stance != Stance::Normal {
                    orders.push(Order::SetStance {
                        army: army_id.clone(),
                        stance: Stance::Normal,
                    });
                }
                orders.extend(
                    ctx.grid
                        .march_orders(army_id, army, &anchor, &target, route),
                );
                continue;
            }
        }

        // 4. Regroup with the main army when much weaker.
        if choice.is_none() {
            if let Some((main, main_power)) = &largest {
                if main != army_id && power < 0.5 * main_power {
                    if let Some(location) = ctx.anchors.get(main).cloned() {
                        if table.contains_key(&location) {
                            choice = Some((Objective::Regroup, location));
                        }
                    }
                }
            }
        }

        // Lot C7a: an idle army outside friendly places (after a peace, a
        // lost siege...) goes back to the nearest place of its own.
        if choice.is_none()
            && !besieging
            && army
                .settlement()
                .is_none_or(|s| !state.is_friendly_settlement(ctx.faction, s))
        {
            choice = table
                .iter()
                .filter(|(id, _)| {
                    ctx.owns_settlement(id) && state.hostile_armies_at(ctx.faction, id).is_empty()
                })
                .min_by_key(|(id, reach)| (reach.cost, (*id).clone()))
                .map(|(id, _)| (Objective::Regroup, id.clone()));
        }

        // CV3-6: outnumbered in the field on a threatened border, with no
        // shelter within this turn's reach: an entrenched camp.
        let sheltered = choice.as_ref().is_some_and(|(objective, target)| {
            !matches!(objective, Objective::Regroup | Objective::Retreat)
                || table.get(target).is_some_and(|r| r.cost <= cap)
        });
        if !sheltered && !besieging && !attacked {
            let threat = here.as_ref().map_or(0.0, |p| ctx.threat(p));
            if crate::stances::should_entrench(state, data, ctx.faction, army_id, threat) {
                if army.stance != Stance::Entrenched {
                    orders.push(Order::SetStance {
                        army: army_id.clone(),
                        stance: Stance::Entrenched,
                    });
                }
                continue;
            }
        }

        // CV3-6: with nothing urgent (at peace, or at war when the tuning
        // allows it with no threat here; no siege of ours or against us), a
        // short detour to an encounter site.
        let idle = choice
            .as_ref()
            .is_none_or(|(objective, _)| *objective == Objective::Regroup);
        if idle && !besieging && !attacked && !realm_besieged {
            let quiet = !ctx.at_war()
                || (data.ai_grid.encounters.detour_at_war
                    && here.as_ref().is_none_or(|p| ctx.threat(p) <= 0.0));
            if quiet {
                if let Some(order) =
                    crate::stances::encounter_detour(state, data, ctx.faction, army_id)
                {
                    if army.stance != Stance::Normal {
                        orders.push(Order::SetStance {
                            army: army_id.clone(),
                            stance: Stance::Normal,
                        });
                    }
                    orders.push(order);
                    continue;
                }
            }
        }

        let Some((objective, target)) = choice else {
            // Idle: normal stance at home.
            if army.stance != Stance::Normal
                && !here
                    .as_ref()
                    .is_some_and(|p| state.is_hostile_territory(ctx.faction, p))
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
        // CV3-6: a forced march to relieve a besieged place or join a siege
        // just beyond normal reach.
        if matches!(objective, Objective::Defend | Objective::Siege) && !attacked {
            if let Some(cost) = table.get(&target).map(|r| r.cost) {
                let forced = crate::stances::forced_march_orders(
                    state,
                    data,
                    ctx.faction,
                    army_id,
                    &target,
                    (cost, cap),
                    |c| {
                        ctx.grid
                            .march_orders_with_cap(army_id, army, &anchor, &target, &table, c)
                    },
                );
                if let Some(forced) = forced {
                    orders.extend(forced);
                    continue;
                }
            }
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
        orders.extend(
            ctx.grid
                .march_orders(army_id, army, &anchor, &target, &table),
        );
    }
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
    options
        .iter()
        .map(|option| {
            let mut option = option.clone();
            if !option.resources.is_empty() {
                if let Some(price) =
                    state.recruit_price(data, faction, settlement, &option.unit_type, supply)
                {
                    option.cost = price.cost;
                    option.import_cost = price.import_cost;
                    option.imported = price.draw.imported;
                }
            }
            option
        })
        .collect()
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
