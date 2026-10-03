//! Plays whole campaigns (default 5 seeds × 464 turns, 1337-1453) with the
//! strategic AI for every faction and prints the F4 balance table
//! (`docs/design/v2-finalisation.md` § 2.2): France-England war share and
//! phases, historical alliances, calls to arms, idle treasuries,
//! bankruptcies, marriages, battles, map changes and survivals.
//!
//! Usage: `century_probe [turns] [seed...]`; `VERBOSE=1` prints the
//! France-England wars and peaces; `DIFFICULTY=hard` (any DF1 level id)
//! plays at that level instead of normal.
//!
//! EQ4 adds the combined balance table (`docs/wip/eq4-equilibre-combine.md`):
//! revolts, sieges and their outcome, territorial snowball in 1437 and 1453,
//! factions eliminated and the use of the right of passage (DP2).
//!
//! EQ5 diagnostics (`docs/wip/eq5-ia-banqueroutes-intrusions.md`):
//! `ECON_TRACE=fac_swiss` prints that faction's budget, economic orders,
//! events and unexplained treasury moves every turn; `TRESPASS_TRACE=1`
//! lists the armies standing on foreign lands without right of passage;
//! `DEBUG_ARMY=army_0012` prints that army's orders (and, for France, their
//! result). `DEATH_TRACE=1` prints every faction that vanishes, with the turn
//! and the cause (LR-05).
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::time::Instant;

use data_model::{CharacterId, FactionId, GameData, ProvinceId, SettlementId};
use sim_campaign::{CampaignState, EventKind};

/// First turn of 1350 (the treasury rule applies from then on).
const TURN_1350: u32 = 13 * 4;
/// First turn of 1400 (survival check).
const TURN_1400: u32 = 63 * 4;
/// Treasury limit in seasons of income.
const IDLE_SEASONS: f64 = 8.0;

const PAIRS: &[(&str, &str, &str)] = &[
    ("Auld Alliance", "fac_scotland", "fac_france"),
    ("Angl.-Flandre", "fac_england", "fac_flanders"),
    ("Angl.-Hainaut", "fac_england", "fac_hainaut"),
    ("Angl.-Gueldre", "fac_england", "fac_guelders"),
    ("Angl.-Brabant", "fac_england", "fac_brabant"),
    ("Fr.-Bohême", "fac_france", "fac_bohemia"),
    ("Fr.-Naples", "fac_france", "fac_naples"),
    ("Bourg.-France", "fac_burgundy", "fac_france"),
    ("Bourg.-Angl.", "fac_burgundy", "fac_england"),
];
const MAJORS: &[&str] = &["fac_england", "fac_france", "fac_burgundy", "fac_scotland"];
/// UR2: the 15th-century unit types (lot UR1, `available_from` 1380-1448), checked so that a
/// probe over the whole 1337-1453 span can confirm the AI actually recruits them.
const CENTURY_15: &[(&str, &str)] = &[
    ("ordonnance", "unit_ordonnance_gendarmes"),
    ("francs-archers", "unit_francs_archers"),
    ("coutiliers", "unit_coutiliers"),
    ("couleuvriniers", "unit_culveriners"),
];

fn id(s: &str) -> FactionId {
    FactionId::new(s).expect("well-formed id")
}

#[derive(Default)]
struct Report {
    seed: u64,
    turns: u32,
    war_turns: u32,
    war_phases: u32,
    /// EQ3: France-England peaces (a war phase that ended before the last turn).
    truces: u32,
    /// EQ3: longest France-England war phase, in turns.
    longest_war: u32,
    /// Turns England dominates the French realm (G2).
    dominance_turns: u32,
    /// Most French provinces (of 1337) England held at once.
    max_held: usize,
    alliance_turns: Vec<u32>,
    /// Alliance turns while France and England are at war (G2).
    alliance_war_turns: Vec<u32>,
    /// Turn each alliance was first seen (G4).
    alliance_first: Vec<Option<u32>>,
    calls_honoured: u32,
    calls_refused: u32,
    /// Worst treasury / income ratio after 1350 and its faction.
    idle_ratio: f64,
    idle_faction: String,
    /// Factions above the limit for more than 4 turns after 1350.
    idle_count: usize,
    bankruptcies: BTreeMap<FactionId, u32>,
    factions: usize,
    marriages_cross: u32,
    marriages_same: u32,
    battles_fr_en: u32,
    captures: u32,
    majors_1400: Vec<bool>,
    /// Turn each major first fell (G5).
    majors_fall: Vec<Option<u32>>,
    destroyed: u32,
    issued: u32,
    refused: u32,
    /// EQ4: combined balance measures.
    eq4: Eq4,
    /// CV3-6: living-campaign measures.
    cv3: Cv3,
    /// EQ6: peace turns per England war blocker (`BLOCKERS`).
    blockers: Vec<u32>,
    /// EQ6: France-England wars opened by England / by France.
    declared_by_england: u32,
    declared_by_france: u32,
    /// UR2: whether each of `CENTURY_15` was ever seen in an army or a garrison (any faction).
    recruited_15th: Vec<bool>,
    /// FE8: feudal balance measures.
    fe8: Fe8,
    seconds: f64,
}

/// FE8 (`docs/wip/fe8-equilibre.md`): forfeitures, felonies, the imperial
/// host and the French revenue.
#[derive(Default)]
struct Fe8 {
    /// Turn France first declares the forfeiture of England (Guyenne).
    guyenne_commise: Option<u32>,
    /// Turn of the first France-England war.
    first_war: Option<u32>,
    /// Forfeitures declared / executed at the peace (every faction).
    commises: u32,
    commises_executed: u32,
    felonies: u32,
    /// Vassals answering the host of the Emperor, and against France.
    imperial_host: u32,
    imperial_host_vs_france: u32,
    /// Turns x Italian imperial vassals at war with France (first 50 turns).
    italian_war_turns: u32,
    /// Turns the Emperor is allied with one of its direct vassals.
    empire_allied_vassal_turns: u32,
    /// France's revenue over the first 50 turns.
    france_income_50: i64,
    /// France's treasury at turn 50.
    france_treasury_50: i64,
    /// Bankruptcies of the factions holding at most 2 provinces at start.
    small_bankruptcies: u32,
    small_factions: usize,
    /// Bankruptcies of the 28 factions of before FE (`LEGACY`), the population
    /// the column measured up to RS-B.
    legacy_bankruptcies: u32,
    /// Spec FE § 5 bands: France's direct vassals at the start, after 20
    /// years and at the end; France first realm by population (itself and
    /// its direct vassals) after 20 years and at the end; succession wars
    /// (disputes taken up by a sponsor); most provinces held at the end by a
    /// faction of at most 2 provinces in 1337, and which.
    france_vassals: [usize; 3],
    france_first: [bool; 2],
    succession_wars: usize,
    minor_max: (usize, String),
}

/// FE8: population of `faction`'s realm (its provinces and its direct vassals').
fn realm_population(state: &CampaignState, data: &GameData, faction: &FactionId) -> u64 {
    let mut members = sim_campaign::feudal::direct_vassals(state, data, faction);
    members.push(faction.clone());
    members
        .iter()
        .flat_map(|f| state.owned_provinces(f))
        .filter_map(|p| state.provinces.get(&p))
        .map(|p| p.population.total())
        .sum()
}

/// FE8: whether France is the most populous sovereign realm.
fn france_first(state: &CampaignState, data: &GameData) -> bool {
    let france = id("fac_france");
    let ours = realm_population(state, data, &france);
    state
        .factions
        .iter()
        .filter(|(f, s)| {
            s.alive && s.suzerain.is_none() && **f != france && f.as_str() != "fac_rebels"
        })
        .all(|(f, _)| realm_population(state, data, f) <= ours)
}

/// FE8: the factions of before FE (ae5d94f1), rebels left out.
const LEGACY: &[&str] = &[
    "fac_aragon",
    "fac_austria",
    "fac_bohemia",
    "fac_brabant",
    "fac_brittany",
    "fac_burgundy",
    "fac_castile",
    "fac_empire",
    "fac_england",
    "fac_flanders",
    "fac_florence",
    "fac_france",
    "fac_genoa",
    "fac_granada",
    "fac_guelders",
    "fac_hainaut",
    "fac_holstein",
    "fac_milan",
    "fac_naples",
    "fac_navarre",
    "fac_papacy",
    "fac_portugal",
    "fac_savoy",
    "fac_scotland",
    "fac_sweden",
    "fac_swiss",
    "fac_venice",
    "fac_verona",
];

/// FE8: the imperial vassals of Italy, whose host against France RS measured.
const ITALIANS: &[&str] = &["fac_milan", "fac_savoy", "fac_genoa", "fac_verona"];

/// EQ6: why England, at peace with France, does not declare war (one count
/// per peace turn and blocker; `free` when nothing in the claim-war gate
/// blocks it).
const BLOCKERS: &[&str] = &[
    "truce", "rest", "regency", "captive", "treasury", "weary", "front", "ratio", "attitude",
    "free",
];

/// EQ6: the gates of `war_target` / `plan_diplomacy` England fails against
/// France this turn (a mirror of `sim_campaign::diplomacy`).
fn war_blockers(
    state: &CampaignState,
    data: &GameData,
    england: &FactionId,
    france: &FactionId,
) -> Vec<&'static str> {
    use sim_campaign::diplomacy::{PRETENDER_AGGRESSION, WAR_REST_TURNS};
    let me = &state.factions[england];
    let war_rules = &data.ai_diplomacy.war;
    let is_main = sim_campaign::diplomacy::main_claim(state, england).as_ref() == Some(france);
    let neutral = is_main && war_rules.claim_war_ignores_difficulty;
    let mut out = Vec::new();
    if state.has_truce(england, france) {
        out.push("truce");
    }
    if !(is_main && war_rules.main_claim_first)
        && me
            .last_war_declared
            .is_some_and(|t| t + WAR_REST_TURNS > state.turn)
    {
        out.push("rest");
    }
    if me.regency {
        out.push("regency");
    }
    if me
        .ruler
        .as_ref()
        .and_then(|r| state.characters.get(r))
        .is_some_and(|r| r.captive)
    {
        out.push("captive");
    }
    if me.treasury <= 0 || me.treasury < me.upkeep_last_turn.max(0) {
        out.push("treasury");
    }
    let most = data.ai_diplomacy.negotiation.max_weariness_to_declare;
    if me.ledger.weariness > most + 20 {
        out.push("weary");
    }
    let pressing: f64 = me
        .at_war_with
        .iter()
        .filter(|e| e.as_str() != "fac_rebels")
        .filter(|e| {
            state.are_neighbors(data, england, e)
                || state.provinces.keys().any(|id| {
                    state.province_owner(id) == Some(england) && state.controls_province(e, id)
                })
        })
        .map(|e| state.faction_power(e))
        .sum();
    if pressing > data.ai_diplomacy.war.front_share * 2.0 * state.faction_power(england) {
        out.push("front");
    }
    let aggression = data
        .factions
        .get(england)
        .and_then(|f| f.ai_personality.as_ref())
        .and_then(|p| p.aggression)
        .map_or(50, i32::from);
    let has_allies = me
        .allies
        .iter()
        .any(|a| state.factions.get(a).is_some_and(|f| f.alive));
    let supported = has_allies || state.are_neighbors(data, england, france);
    let rules = &data.ai_diplomacy.war;
    let demand = if neutral {
        1.0
    } else {
        state.difficulty_war_ratio_factor(data, france)
    };
    let needed = demand
        * if supported {
            rules.pretender_ratio
        } else {
            rules.pretender_ratio_alone
        };
    let ratio = state.coalition_power(england) / state.faction_power(france).max(1.0);
    if aggression < PRETENDER_AGGRESSION || ratio < needed {
        out.push("ratio");
    }
    let attitude = state.attitude(data, england, france);
    let offset = if neutral {
        state.difficulty_attitude(data, england, france)
    } else {
        0
    } + if is_main && war_rules.claim_war_ignores_kinship {
        state.kinship_attitude(data, england, france)
    } else {
        0
    };
    if attitude.0 - offset >= 20 {
        out.push("attitude");
    }
    // `WAR_TRACE=1`: England's reasons and gates every 5 years of peace.
    if std::env::var("WAR_TRACE").is_ok() && state.turn.is_multiple_of(20) {
        println!(
            "  t{} {} ratio {ratio:.2} (needed {needed:.2}) weariness {} treasury {} upkeep {} attitude {:?} blockers {out:?} wars {:?}",
            state.turn,
            state.date_label(),
            me.ledger.weariness,
            me.treasury,
            me.upkeep_last_turn,
            attitude,
            me.at_war_with
                .iter()
                .map(|e| format!("{e} score {}", state.war_score(data, england, e)))
                .collect::<Vec<_>>(),
        );
    }
    if out.is_empty() {
        out.push("free");
    }
    out
}

/// CV3-6: living-campaign measures of one campaign (`CV3_STATS=1`).
#[derive(Default)]
struct Cv3 {
    /// Stance orders issued by the planners (every faction, France included).
    ambush_orders: u32,
    forced_march_orders: u32,
    entrenched_orders: u32,
    /// Ambushes sprung / discovered (every faction).
    ambush_sprung: u32,
    ambush_failed: u32,
    /// Encounters resolved (an option applied) and sites that appeared.
    encounters: u32,
    /// Battles (one « Bataille de … » line each).
    battles: u32,
    /// Outcome classes of both sides, by `BattleOutcomeClass::ALL` index
    /// (victory / defeat for the battles without an outcome line).
    classes: [u32; 7],
}

impl Cv3 {
    fn orders(&mut self, orders: &[sim_campaign::Order]) {
        for order in orders {
            if let sim_campaign::Order::SetStance { stance, .. } = order {
                match stance {
                    sim_campaign::Stance::Ambush => self.ambush_orders += 1,
                    sim_campaign::Stance::ForcedMarch => self.forced_march_orders += 1,
                    sim_campaign::Stance::Entrenched => self.entrenched_orders += 1,
                    _ => {}
                }
            }
        }
    }

    fn events(&mut self, data: &GameData, events: &[sim_campaign::GameEvent]) {
        use data_model::BattleOutcomeClass as C;
        let rules = &data.battle_outcome_rules;
        let labels: Vec<(usize, String)> = C::ALL
            .iter()
            .enumerate()
            .map(|(i, c)| (i, rules.consequence(*c).label))
            .collect();
        let index = |c: C| C::ALL.iter().position(|x| *x == c).unwrap_or(0);
        let mut pending_default = 0u32;
        for event in events {
            let text = &event.text_fr;
            if event.kind == EventKind::Battle {
                if text.starts_with("Embuscade !") {
                    self.ambush_sprung += 1;
                } else if text.starts_with("Embuscade éventée") {
                    self.ambush_failed += 1;
                } else if text.starts_with("Bataille ") && text.contains(". Vainqueur : ") {
                    self.battles += 1;
                    pending_default += 1;
                } else if let Some((winner, loser)) = text.split_once(" ; ") {
                    let won = labels
                        .iter()
                        .filter(|(_, l)| winner.starts_with(l.as_str()))
                        .max_by_key(|(_, l)| l.len());
                    let lost = labels
                        .iter()
                        .filter(|(_, l)| {
                            let lower = lowercase_first(l);
                            loser.starts_with(lower.as_str())
                        })
                        .max_by_key(|(_, l)| l.len());
                    if let (Some((w, _)), Some((l, _))) = (won, lost) {
                        self.classes[*w] += 1;
                        self.classes[*l] += 1;
                        pending_default = pending_default.saturating_sub(1);
                    }
                }
            } else if event.kind == EventKind::Chronicle
                && data
                    .encounters
                    .values()
                    .any(|e| text.starts_with(&format!("{} — ", e.title)))
            {
                self.encounters += 1;
            }
        }
        self.classes[index(C::Victory)] += pending_default;
        self.classes[index(C::Defeat)] += pending_default;
    }
}

fn lowercase_first(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => first.to_lowercase().chain(chars).collect(),
        None => String::new(),
    }
}

/// EQ4: combined balance measures of one campaign.
#[derive(Default)]
struct Eq4 {
    /// Revolts proper (not the hand-over of a province to the rebels).
    revolts: u32,
    /// `SiegeStarted` events.
    sieges_started: u32,
    /// Sieges seen at the end of a turn that ended with the besieger holding the place.
    sieges_won: u32,
    /// Sieges seen at the end of a turn that ended without a capture (lifted, relieved).
    sieges_failed: u32,
    /// Settlements taken by a crown without a siege seen at the end of the previous turn
    /// (empty garrison, assault within the turn, relief battle...).
    direct_captures: u32,
    /// (faction, share of provinces controlled) in 1437 and at the end.
    snowball_1437: (String, f64),
    snowball_end: (String, f64),
    /// Factions dead at the end (rebels excluded).
    eliminated: Vec<String>,
    /// DP2: military access treaties granted (new giver -> taker pairs).
    access_grants: u32,
    /// DP2: access pairs in force, summed over turns.
    access_pair_turns: u64,
    /// DP2: seasons of trespass (one per victim-intruder pair and season).
    trespass_seasons: u32,
    /// DP2: casus belli won by a trespass.
    trespass_grievances: u32,
    /// DC3: province cities taken by a crown (rebels excluded).
    city_captures: u32,
    /// DC3: city captures taken back from the rebels.
    city_from_rebels: u32,
    /// DC3: provinces whose every settlement came to be held by one crown that did not hold
    /// them all before, and the turns since that crown's first foothold there.
    provinces_completed: u32,
    conquest_turns: u32,
    /// DC3: (province, crown) -> turn that crown took its first place in a province where
    /// it held none.
    footholds: BTreeMap<(ProvinceId, FactionId), u32>,
    /// DC3 (revolts): devastation summed over provinces and turns, province-turns occupied
    /// (controller != owner) and province-turns above 60 of weighted unrest.
    devastation_sum: u64,
    occupied_turns: u32,
    hot_turns: u32,
    /// DC3: faction-turns (living factions) at high tax, and all faction-turns.
    high_tax_turns: u32,
    faction_turns: u32,
    /// DC3: men under arms (field armies) summed over turns, all factions.
    men_sum: u64,
    /// DC3: province-turns counted in the three sums above.
    province_turns: u32,
    /// DC3: beaten armies that fell back to a neutral refuge, routed and rallied, dispersed.
    retreat_fallback: u32,
    retreat_rout: u32,
    retreat_dispersed: u32,
}

/// Faction controlling the most provinces and its share.
fn snowball(state: &CampaignState) -> (String, f64) {
    let mut counts: BTreeMap<&FactionId, usize> = BTreeMap::new();
    for province in state.provinces.keys() {
        if let Some(controller) = state.province_controller(province) {
            *counts.entry(controller).or_default() += 1;
        }
    }
    let total = state.provinces.len().max(1) as f64;
    counts
        .into_iter()
        .filter(|(f, _)| f.as_str() != "fac_rebels")
        .max_by_key(|(f, n)| (*n, std::cmp::Reverse((*f).clone())))
        .map_or(("-".to_owned(), 0.0), |(f, n)| {
            (
                f.as_str().trim_start_matches("fac_").to_owned(),
                100.0 * n as f64 / total,
            )
        })
}

/// Sieges in place at the end of a turn: settlement -> attacker.
type Sieges = BTreeMap<SettlementId, FactionId>;

fn sieges(state: &CampaignState) -> Sieges {
    state
        .settlements
        .iter()
        .filter_map(|(id, s)| Some((id.clone(), s.siege.as_ref()?.attacker.clone())))
        .collect()
}

/// DP2 snapshot: military access pairs (giver, taker) and trespass records
/// (victim, intruder) -> (total seasons, casus belli until).
type Passage = (
    BTreeSet<(FactionId, FactionId)>,
    BTreeMap<(FactionId, FactionId), (u32, u32)>,
);

fn passage(state: &CampaignState) -> Passage {
    let mut access = BTreeSet::new();
    let mut trespass = BTreeMap::new();
    for (id, f) in &state.factions {
        for taker in &f.ledger.military_access {
            access.insert((id.clone(), taker.clone()));
        }
        for (intruder, t) in &f.ledger.trespassers {
            trespass.insert((id.clone(), intruder.clone()), (t.total, t.grievance_until));
        }
    }
    (access, trespass)
}

fn controllers(state: &CampaignState) -> BTreeMap<SettlementId, FactionId> {
    state
        .settlements
        .iter()
        .map(|(id, s)| (id.clone(), s.controller.clone()))
        .collect()
}

/// End-of-turn snapshot compared by [`track_eq4`].
type Snapshot = (Sieges, Passage, BTreeMap<SettlementId, FactionId>);

fn snapshot(state: &CampaignState) -> Snapshot {
    (sieges(state), passage(state), controllers(state))
}

/// EQ4: compares the state with the previous end-of-turn snapshot.
fn track_eq4(state: &CampaignState, eq4: &mut Eq4, before: &Snapshot) {
    let (old_sieges, (old_access, old_trespass), old_controllers) = before;
    let now = sieges(state);
    for (id, attacker) in old_sieges {
        let controller = &state.settlements[id].controller;
        if controller == attacker {
            eq4.sieges_won += 1;
        } else if now.get(id) != Some(attacker) {
            eq4.sieges_failed += 1;
        }
    }
    for (id, s) in &state.settlements {
        let Some(previous) = old_controllers.get(id) else {
            continue;
        };
        if previous != &s.controller
            && s.controller.as_str() != "fac_rebels"
            && old_sieges.get(id) != Some(&s.controller)
        {
            eq4.direct_captures += 1;
        }
    }
    for f in state.factions.values().filter(|f| f.alive) {
        eq4.faction_turns += 1;
        if f.tax_rate == sim_campaign::TaxRate::High {
            eq4.high_tax_turns += 1;
        }
    }
    eq4.men_sum += state
        .armies
        .values()
        .flat_map(|a| a.units.iter())
        .map(|u| u64::from(u.strength))
        .sum::<u64>();
    // DC3: city captures, footholds and completed conquests.
    for (province_id, province) in &state.provinces {
        let Some(city) = state.settlements.get(&province.city) else {
            continue;
        };
        eq4.province_turns += 1;
        eq4.devastation_sum += u64::from(province.devastation);
        if city.controller != city.owner {
            eq4.occupied_turns += 1;
        }
        if sim_campaign::population::weighted_unrest(&province.population) > 60.0 {
            eq4.hot_turns += 1;
        }
        let previous = old_controllers.get(&province.city);
        if previous != Some(&city.controller) && city.controller.as_str() != "fac_rebels" {
            eq4.city_captures += 1;
            // DC3: `CAPTURE_TRACE=1` lists the city captures.
            if std::env::var("CAPTURE_TRACE").is_ok() {
                println!(
                    "  CAPTURE t{} {} {} <- {} owner {}",
                    state.turn,
                    province_id.as_str(),
                    city.controller.as_str(),
                    previous.map_or("?", |f| f.as_str()),
                    city.owner.as_str(),
                );
            }
            // Retaken from the rebels: a consequence of the revolts, not of the war.
            if previous.is_some_and(|f| f.as_str() == "fac_rebels") {
                eq4.city_from_rebels += 1;
            }
        }
        let places: Vec<&SettlementId> = state
            .settlements
            .iter()
            .filter(|(_, s)| &s.province == province_id)
            .map(|(id, _)| id)
            .collect();
        let holders: BTreeSet<&FactionId> = places
            .iter()
            .map(|id| &state.settlements[*id].controller)
            .collect();
        eq4.footholds
            .retain(|(p, f), _| p != province_id || holders.contains(f));
        for holder in &holders {
            let newly = !places
                .iter()
                .any(|id| old_controllers.get(*id) == Some(*holder));
            if newly && holder.as_str() != "fac_rebels" {
                eq4.footholds
                    .entry((province_id.clone(), (*holder).clone()))
                    .or_insert(state.turn);
            }
        }
        if holders.len() == 1 {
            let holder = *holders.iter().next().expect("one");
            let was_whole = places
                .iter()
                .all(|id| old_controllers.get(*id) == Some(holder));
            if !was_whole && holder.as_str() != "fac_rebels" {
                if let Some(start) = eq4.footholds.remove(&(province_id.clone(), holder.clone())) {
                    eq4.provinces_completed += 1;
                    eq4.conquest_turns += state.turn.saturating_sub(start);
                }
            }
        }
    }
    let (access, trespass) = passage(state);
    eq4.access_grants += access.difference(old_access).count() as u32;
    eq4.access_pair_turns += access.len() as u64;
    for (key, (total, until)) in &trespass {
        let (old_total, old_until) = old_trespass.get(key).copied().unwrap_or((0, 0));
        eq4.trespass_seasons += if *total >= old_total {
            total - old_total
        } else {
            *total
        };
        if *until > old_until && old_until < state.turn {
            eq4.trespass_grievances += 1;
        }
    }
}

/// `TRACE=1`: Burgundy's standing every 5 years (G4).
fn trace_burgundy(state: &CampaignState, data: &GameData, seed: u64) {
    let (france, england, burgundy) = (id("fac_france"), id("fac_england"), id("fac_burgundy"));
    let b = &state.factions[&burgundy];
    let own = |f: &FactionId| {
        state
            .provinces
            .keys()
            .filter(|id| state.province_owner(id) == Some(f))
            .count()
    };
    let score = if state.is_at_war(&france, &england) {
        state.war_score(data, &france, &england)
    } else {
        0
    };
    println!(
        "  [{seed}] {:>16} alive:{} suz:{:?} loy:{} att(fr):{} att(en):{} war(fr/en):{}/{} ally(fr/en):{}/{} en-held:{} prov fr/en/bu:{}/{}/{} fr-en:{} ws(fr,en):{score} claims(en>bu):{:?} grief:{:?}",
        state.date_label(),
        b.alive,
        b.suzerain.as_ref().map(|s| s.as_str()),
        b.loyalty,
        state.attitude(data, &burgundy, &france).0,
        state.attitude(data, &burgundy, &england).0,
        state.is_at_war(&burgundy, &france),
        state.is_at_war(&burgundy, &england),
        state.is_allied(&burgundy, &france),
        state.is_allied(&burgundy, &england),
        ai::alignment::realm_held_by(state, data, &france, &england),
        own(&france),
        own(&england),
        own(&burgundy),
        state.is_at_war(&france, &england),
        sim_campaign::diplomacy::claim_stakes(state, &england, &burgundy).provinces,
        ai::alignment::grievance_change(state, data, &burgundy),
    );
    if std::env::var("TRACE_EVAL").is_ok() {
        let e = sim_campaign::diplomacy::evaluate(
            state,
            data,
            &burgundy,
            &england,
            &sim_campaign::diplomacy::Proposal::Alliance,
        );
        println!(
            "  [{seed}]     side_change {:?} grudge {} alliance en {} {:?}",
            ai::alignment::side_change(state, data, &burgundy),
            ai::alignment::grievance(state, &burgundy, &france),
            e.accept,
            e.reasons
        );
    }
    for low in ["fac_brabant", "fac_hainaut"] {
        let low = id(low);
        let Some(f) = state.factions.get(&low).filter(|f| f.alive) else {
            continue;
        };
        println!(
            "  [{seed}] {:>16}   {} suz:{:?} att(en/fr):{}/{} ally(en/fr):{}/{} war(en/fr):{}/{} allies:{:?}",
            state.date_label(),
            low.as_str(),
            f.suzerain.as_ref().map(|s| s.as_str()),
            state.attitude(data, &low, &england).0,
            state.attitude(data, &low, &france).0,
            state.is_allied(&low, &england),
            state.is_allied(&low, &france),
            state.is_at_war(&low, &england),
            state.is_at_war(&low, &france),
            f.allies.iter().map(|a| a.as_str()).collect::<Vec<_>>(),
        );
    }
}

/// EQ5: one line of `faction`'s budget and economic orders at plan time.
fn trace_economy(
    state: &CampaignState,
    data: &GameData,
    faction: &FactionId,
    orders: &[sim_campaign::Order],
) {
    use sim_campaign::Order;
    let Some(f) = state.factions.get(faction) else {
        return;
    };
    let field: usize = state
        .armies
        .values()
        .filter(|a| &a.faction == faction)
        .map(|a| a.units.len())
        .sum();
    let armies_abroad = state
        .armies
        .values()
        .filter(|a| &a.faction == faction)
        .count();
    let garrison: usize = state
        .settlements
        .values()
        .filter(|s| &s.controller == faction)
        .map(|s| s.garrison.len())
        .sum();
    let unrests: Vec<f64> = state
        .provinces
        .iter()
        .filter(|(id, _)| state.controls_province(faction, id))
        .map(|(_, p)| sim_campaign::population::weighted_unrest(&p.population))
        .collect();
    println!(
        "{:>16}   unrest max {:.0} mean {:.0} price {} coinage {:?} tributes {:?}",
        "",
        unrests.iter().copied().fold(0.0, f64::max),
        unrests.iter().sum::<f64>() / unrests.len().max(1) as f64,
        f.price_level,
        f.coinage,
        f.ledger
            .tributes
            .iter()
            .map(|t| t.per_season)
            .collect::<Vec<_>>(),
    );
    let count = |pred: fn(&Order) -> bool| orders.iter().filter(|o| pred(o)).count();
    println!(
        "{:>16} tr {:>7} inc {:>5} (last {:>5}) adm {:>5} army {:>5} bld {:>5} upk_last {:>5} table {:>4} tax {:?} war {} | armies {} units {} garr {} | disb {} recr {} build {} other {:?}",
        state.date_label(),
        f.treasury,
        state.faction_income_effective(data, faction),
        f.income_last_turn,
        state.faction_administration_upkeep(data, faction),
        state.faction_army_upkeep(data, faction),
        state.faction_building_upkeep(data, faction),
        f.upkeep_last_turn,
        f.table_upkeep_last_turn,
        f.tax_rate,
        f.at_war_with.len(),
        armies_abroad,
        field,
        garrison,
        count(|o| matches!(o, Order::DisbandUnit { .. })),
        count(|o| matches!(o, Order::Recruit { .. })),
        count(|o| matches!(o, Order::Build { .. })),
        orders
            .iter()
            .filter(|o| !matches!(
                o,
                Order::DisbandUnit { .. }
                    | Order::Recruit { .. }
                    | Order::Build { .. }
                    | Order::MoveArmy { .. }
            ))
            .map(|o| format!("{o:?}").chars().take(40).collect::<String>())
            .collect::<Vec<_>>(),
    );
}

/// EQ5: one line per army trespassing at the end of the turn.
fn trace_trespass(state: &CampaignState, data: &GameData) {
    for (id, army) in &state.armies {
        if army.units.is_empty() || army.faction.as_str() == "fac_rebels" {
            continue;
        }
        let Some(province) = state.army_province(data, army) else {
            continue;
        };
        let Some(owner) = sim_campaign::passage::trespassed_owner(state, &army.faction, &province)
        else {
            continue;
        };
        let f = &state.factions[&army.faction];
        println!(
            "TRESPASS t{} {} {} in {} of {} | wars {} | at {:?} | units {} | stance {:?} | mv {} | supply {} | truce {} | may {}",
            state.turn,
            id.as_str(),
            army.faction.as_str(),
            province.as_str(),
            owner.as_str(),
            f.at_war_with.len(),
            army.settlement().map(|s| format!(
                "{} held by {}",
                s.as_str(),
                state.settlements.get(s).map_or("?", |x| x.controller.as_str())
            )),
            army.units.len(),
            army.stance,
            army.movement_left,
            army.supply,
            state.has_truce(&army.faction, &owner),
            sim_campaign::passage::ai_may_trespass(state, data, &army.faction, &owner),
        );
    }
}

fn run(data: &GameData, seed: u64, turns: u32, verbose: bool) -> Report {
    // `TRACE=1` follows Burgundy; any other value filters events by that word.
    let trace_word =
        std::env::var("TRACE")
            .ok()
            .map(|w| if w == "1" { "Bourgogne".to_owned() } else { w });
    let trace = trace_word.is_some();
    let started = Instant::now();
    let france = id("fac_france");
    let england = id("fac_england");
    let mut state = CampaignState::new_1337(data, france.clone(), seed).expect("state");
    // EQ3: `DIFFICULTY=hard` (DF1 level id) plays the campaign at that
    // level, France being the "player"; the default is normal.
    if let Some(level) = std::env::var("DIFFICULTY")
        .ok()
        .and_then(|id| sim_campaign::difficulty::Difficulty::from_id(&id))
    {
        state.set_difficulty(level);
    }
    let mut report = Report {
        seed,
        turns,
        alliance_turns: vec![0; PAIRS.len()],
        alliance_war_turns: vec![0; PAIRS.len()],
        alliance_first: vec![None; PAIRS.len()],
        majors_1400: vec![true; MAJORS.len()],
        majors_fall: vec![None; MAJORS.len()],
        recruited_15th: vec![false; CENTURY_15.len()],
        blockers: vec![0; BLOCKERS.len()],
        ..Report::default()
    };
    // FE8: the small factions (at most 2 provinces in 1337).
    let small: BTreeSet<FactionId> = state
        .factions
        .keys()
        .filter(|f| f.as_str() != "fac_rebels" && state.owned_provinces(f).len() <= 2)
        .cloned()
        .collect();
    report.fe8.small_factions = small.len();
    report.fe8.france_vassals[0] =
        sim_campaign::feudal::direct_vassals(&state, data, &france).len();
    let names: Vec<String> = [&france, &england]
        .iter()
        .map(|f| data.factions[*f].short_or_display_name().to_owned())
        .collect();
    let married = |state: &CampaignState| -> BTreeSet<(CharacterId, CharacterId, bool)> {
        state
            .characters
            .iter()
            .filter(|(_, c)| c.alive)
            .filter_map(|(id, c)| {
                let spouse = c.spouse.as_ref()?;
                let s = state.characters.get(spouse)?;
                (id < spouse).then(|| (id.clone(), spouse.clone(), s.faction != c.faction))
            })
            .collect()
    };
    let mut seen = married(&state);
    let mut over_limit: BTreeMap<FactionId, u32> = BTreeMap::new();
    // Rolling income (last 8 seasons): a season under siege is not the
    // measure of a realm's revenue.
    let mut incomes: BTreeMap<FactionId, Vec<i64>> = BTreeMap::new();
    let mut was_at_war = false;
    let mut war_run = 0u32;
    let mut eq4_before = snapshot(&state);
    // EQ5: `ECON_TRACE=fac_swiss` dumps that faction's budget and economic
    // orders every turn.
    // EQ5: `TRESPASS_TRACE=1` lists every army standing on foreign lands
    // without right of passage at the end of each turn.
    let trespass_trace = std::env::var("TRESPASS_TRACE").is_ok();
    let war_trace = std::env::var("WAR_TRACE").is_ok();
    let death_trace = std::env::var("DEATH_TRACE").is_ok();
    let debug_army = std::env::var("DEBUG_ARMY").ok();
    let econ_trace = std::env::var("ECON_TRACE").ok().map(|f| id(&f));
    for _ in 0..turns {
        // France is the "player": it answers offers as the AI would judge
        // them (otherwise no peace offered to France is ever signed).
        let offers = state.factions[&france].offers.clone();
        for offer in offers {
            let accept = sim_campaign::diplomacy::evaluate(
                &state,
                data,
                &offer.from,
                &france,
                &offer.proposal,
            )
            .accept;
            let _ = state.submit_order(
                data,
                sim_campaign::Order::AnswerOffer {
                    offer: offer.id,
                    accept,
                },
            );
        }
        let france_orders = ai::plan_turn(&state, data, &france);
        report.cv3.orders(&france_orders);
        for order in france_orders {
            let text = debug_army
                .as_ref()
                .map(|army| (army.clone(), format!("{order:?}")))
                .filter(|(army, text)| text.contains(&format!("\"{army}\"")));
            report.issued += 1;
            let result = state.submit_order(data, order);
            if let Some((army, text)) = text {
                println!("t{} {army} order {text} -> {result:?}", state.turn);
            }
            if result.is_err() {
                report.refused += 1;
            }
        }
        let treasuries_before: BTreeMap<FactionId, i64> = state
            .factions
            .iter()
            .map(|(f, s)| (f.clone(), s.treasury))
            .collect();
        let events = if econ_trace.is_some() || debug_army.is_some() {
            state.end_turn_with(data, |s, d, f| {
                let orders = ai::plan_turn(s, d, f);
                if econ_trace.as_ref() == Some(f) {
                    trace_economy(s, d, f, &orders);
                }
                if let Some(army) = &debug_army {
                    let needle = format!("\"{army}\"");
                    for order in orders.iter().map(|o| format!("{o:?}")) {
                        if order.contains(&needle) {
                            println!("t{} {army} order {order}", s.turn);
                        }
                    }
                }
                orders
            })
        } else {
            let counted = std::cell::RefCell::new(Cv3::default());
            let events = state.end_turn_with(data, |s, d, f| {
                let orders = ai::plan_turn(s, d, f);
                counted.borrow_mut().orders(&orders);
                orders
            });
            let counted = counted.into_inner();
            report.cv3.ambush_orders += counted.ambush_orders;
            report.cv3.forced_march_orders += counted.forced_march_orders;
            report.cv3.entrenched_orders += counted.entrenched_orders;
            events
        };
        report.cv3.events(data, &events);
        if let Some(traced) = &econ_trace {
            let residual = |f: &FactionId| {
                let now = &state.factions[f];
                now.treasury
                    - treasuries_before.get(f).copied().unwrap_or(0)
                    - (now.income_last_turn - now.upkeep_last_turn)
            };
            let own = residual(traced);
            if own.abs() > 100 {
                let others: Vec<(String, i64)> = state
                    .factions
                    .keys()
                    .filter(|f| *f != traced && residual(f).abs() > 100)
                    .map(|f| (f.as_str().to_owned(), residual(f)))
                    .collect();
                println!("{:>16}   residual {own} ; others {others:?}", "");
            }
            for event in events.iter().filter(|e| e.faction.as_ref() == Some(traced)) {
                println!("{:>16}   event {:?}: {}", "", event.kind, event.text_fr);
            }
        }
        // NT9: `ARMY_COUNT=1` prints the field armies of France and England each turn.
        if std::env::var("ARMY_COUNT").is_ok() {
            let n = state
                .armies
                .values()
                .filter(|a| {
                    !a.units.is_empty()
                        && matches!(a.faction.as_str(), "fac_france" | "fac_england")
                })
                .count();
            println!("AC [{seed}] {n}");
        }
        for event in &events {
            match event.kind {
                EventKind::Bankruptcy => {
                    if let Some(f) = &event.faction {
                        *report.bankruptcies.entry(f.clone()).or_default() += 1;
                        if small.contains(f) {
                            report.fe8.small_bankruptcies += 1;
                        }
                        if LEGACY.contains(&f.as_str()) {
                            report.fe8.legacy_bankruptcies += 1;
                        }
                    }
                }
                EventKind::Battle
                    if !event.text_fr.starts_with("Bataille en vue")
                        && !event.text_fr.contains("se prépare")
                        && names.iter().any(|n| event.text_fr.contains(n.as_str())) =>
                {
                    report.battles_fr_en += 1;
                    // NT9: `BATTLE_TRACE=1` prints each counted FR/EN battle line.
                    if std::env::var("BATTLE_TRACE").is_ok() {
                        println!("BT [{seed}] {} | {}", state.date_label(), event.text_fr);
                    }
                }
                EventKind::ProvinceCaptured => report.captures += 1,
                EventKind::Revolt if !event.text_fr.contains("passe aux mains") => {
                    report.eq4.revolts += 1;
                    // DC3: `REVOLT_TRACE=1` prints each revolt and the province's state.
                    if std::env::var("REVOLT_TRACE").is_ok() {
                        if let Some(p) =
                            event.province.as_ref().and_then(|p| state.provinces.get(p))
                        {
                            let city = &state.settlements[&p.city];
                            println!(
                                "  [{seed}] {:>16} {} ctrl {} owner {} dev {} disorder {} garrison {} places {}",
                                state.date_label(),
                                event.text_fr,
                                city.controller.as_str(),
                                city.owner.as_str(),
                                p.devastation,
                                p.unrest,
                                state.province_garrison_strength(event.province.as_ref().expect("province")),
                                state.settlements.values().filter(|s| s.province == city.province).count(),
                            );
                        }
                    }
                }
                EventKind::SiegeStarted => report.eq4.sieges_started += 1,
                // DC3: outcomes of a beaten army's retreat (a friendly retreat has no event).
                EventKind::Attrition if event.text_fr.contains("recule et perd") => {
                    report.eq4.retreat_fallback += 1;
                }
                EventKind::Attrition if event.text_fr.contains("avant de se rallier") => {
                    report.eq4.retreat_rout += 1;
                }
                EventKind::ArmyDestroyed if event.text_fr.contains("se disperse") => {
                    report.eq4.retreat_dispersed += 1;
                }
                EventKind::FactionDestroyed => {
                    report.destroyed += 1;
                    // LR-05: `DEATH_TRACE=1` lists every vanished faction and how
                    // (« n'a plus de titre » = absorbed by succession or escheat,
                    // « disparaît de la carte » = lost its last city and army).
                    if death_trace {
                        let year = 1337 + state.turn / 4;
                        println!("DEATH [{seed}] t{} ({year}) {}", state.turn, event.text_fr);
                    }
                }
                EventKind::WarDeclared if event.text_fr.contains("répond à l'appel") => {
                    report.calls_honoured += 1;
                }
                EventKind::AllianceBroken if event.text_fr.contains("refuse de soutenir") => {
                    report.calls_refused += 1;
                }
                _ => {}
            }
            if trace
                && trace_word
                    .as_ref()
                    .is_some_and(|w| event.text_fr.contains(w.as_str()))
                && !matches!(event.kind, EventKind::Battle | EventKind::Marriage)
            {
                println!("  [{seed}] {:>16} * {}", state.date_label(), event.text_fr);
            }
            if verbose
                && matches!(event.kind, EventKind::WarDeclared | EventKind::PeaceSigned)
                && names.iter().all(|n| event.text_fr.contains(n.as_str()))
            {
                println!("  [{seed}] {:>16} {}", state.date_label(), event.text_fr);
            }
        }
        if trespass_trace {
            trace_trespass(&state, data);
        }
        // DC3: `ARMY_TRACE=fac_england` prints that faction's armies every 4 turns.
        if let Ok(who) = std::env::var("ARMY_TRACE") {
            if state.turn.is_multiple_of(4) {
                for (aid, army) in state
                    .armies
                    .iter()
                    .filter(|(_, a)| a.faction.as_str() == who)
                {
                    let men: u32 = army.units.iter().map(|u| u.strength).sum();
                    println!(
                        "  ARMY [{seed}] t{} {} {} men {} units {} at {} owner {}",
                        state.turn,
                        aid.as_str(),
                        state.date_label(),
                        men,
                        army.units.len(),
                        state
                            .army_province(data, army)
                            .map_or("-".into(), |p| p.as_str().to_string()),
                        state
                            .army_province(data, army)
                            .and_then(|p| state.province_controller(&p).cloned())
                            .map_or("-".into(), |f| f.as_str().to_string()),
                    );
                }
            }
        }
        // DC3: `REVOLT_TRACE=1` also lists the provinces above 70 of weighted unrest.
        if std::env::var("REVOLT_TRACE").is_ok() {
            for (pid, p) in &state.provinces {
                let w = sim_campaign::population::weighted_unrest(&p.population);
                if w > 70.0 {
                    let c = &state.settlements[&p.city];
                    let pe = &p.population.peasants;
                    println!(
                        "  [{seed}] {:>16} HOT {} {w:.0} ctrl {} owner {} tax {:?} peasants u{} h{} g{} w{} dev {} dis {} gar {} rs {}",
                        state.date_label(),
                        pid.as_str(),
                        c.controller.as_str(),
                        c.owner.as_str(),
                        state.factions.get(&c.controller).map(|f| f.tax_rate),
                        pe.unrest,
                        pe.health,
                        pe.goods_satisfaction,
                        pe.wealth,
                        p.devastation,
                        p.unrest,
                        state.province_garrison_strength(pid),
                        p.revolt_seasons,
                    );
                }
            }
        }
        // DC6b: `DC6_TRACE=1` prints research points per faction (turns 1, 40, 120)
        // and the heresy of every province each turn.
        if std::env::var("DC6_TRACE").is_ok() {
            if [1, 40, 120, 200].contains(&state.turn) {
                for fid in state.factions.keys().filter(|f| f.as_str() != "fac_rebels") {
                    if state.factions[fid].alive {
                        println!(
                            "DC6R {seed} {} {} {}",
                            state.turn,
                            fid.as_str(),
                            state.research_points_per_turn(data, fid)
                        );
                    }
                }
            }
            let heretic: Vec<u8> = state
                .provinces
                .values()
                .filter(|p| p.heresy_religion.is_some())
                .map(|p| p.heresy)
                .collect();
            let sum: u32 = state.provinces.values().map(|p| u32::from(p.heresy)).sum();
            println!(
                "DC6H {seed} {} {} {} {} {}",
                state.turn,
                heretic.len(),
                sum,
                state.provinces.len(),
                heretic.iter().max().copied().unwrap_or(0)
            );
        }
        // DC6b: `DC6_TRACE=1` also follows revolts, occupations and unrest.
        if std::env::var("DC6_TRACE").is_ok() {
            for event in events.iter().filter(|e| {
                matches!(e.kind, EventKind::Revolt) && !e.text_fr.contains("passe aux mains")
            }) {
                if let Some(pid) = event.province.as_ref() {
                    let p = &state.provinces[pid];
                    let c = &state.settlements[&p.city];
                    println!(
                        "DC6V {seed} {} {} {} {} {} {}",
                        state.turn,
                        pid.as_str(),
                        u8::from(c.controller != c.owner),
                        state.province_garrison_strength(pid),
                        p.devastation,
                        p.unrest
                    );
                }
            }
            let mut occupied = Vec::new();
            let (mut hot60, mut hot75, mut gar, mut w_sum) = (0, 0, 0u64, 0.0);
            for (pid, p) in &state.provinces {
                let c = &state.settlements[&p.city];
                if c.controller != c.owner {
                    occupied.push(pid.as_str().to_owned());
                }
                let w = sim_campaign::population::weighted_unrest(&p.population);
                w_sum += w;
                if w > 60.0 {
                    hot60 += 1;
                }
                if w > 75.0 {
                    hot75 += 1;
                }
                gar += u64::from(state.province_garrison_strength(pid));
            }
            let n = state.provinces.len().max(1);
            println!(
                "DC6P {seed} {} {hot60} {hot75} {} {:.1} {}",
                state.turn,
                gar / n as u64,
                w_sum / n as f64,
                occupied.join(",")
            );
        }
        track_eq4(&state, &mut report.eq4, &eq4_before);
        eq4_before = snapshot(&state);
        if state.year == 1437 && report.eq4.snowball_1437.0.is_empty() {
            report.eq4.snowball_1437 = snowball(&state);
        }
        let at_war = state.is_at_war(&france, &england);
        // EQ6 `WAR_TRACE=1`: both sides' weariness and war score every
        // 5 years of war.
        if at_war && war_trace && state.turn.is_multiple_of(20) {
            let side = |f: &FactionId, e: &FactionId| {
                let s = &state.factions[f];
                format!(
                    "weariness {} score {} treasury {} provinces {}",
                    s.ledger.weariness,
                    state.war_score(data, f, e),
                    s.treasury,
                    state
                        .provinces
                        .keys()
                        .filter(|p| state.controls_province(f, p))
                        .count()
                )
            };
            println!(
                "  t{} {} WAR england [{}] france [{}]",
                state.turn,
                state.date_label(),
                side(&england, &france),
                side(&france, &england)
            );
            let white = [sim_campaign::negotiation::Article::Peace];
            for (from, to) in [(&england, &france), (&france, &england)] {
                let verdict =
                    sim_campaign::negotiation::evaluate_treaty(&state, data, from, to, &white);
                println!(
                    "      white peace {} -> {}: chance {} {:?}",
                    from.as_str(),
                    to.as_str(),
                    verdict.chance,
                    verdict.reasons()
                );
            }
        }
        if at_war {
            report.war_turns += 1;
            war_run += 1;
            report.longest_war = report.longest_war.max(war_run);
            if !was_at_war {
                report.war_phases += 1;
            }
        } else {
            if was_at_war {
                report.truces += 1;
            }
            war_run = 0;
            // Other wars of England at peace with France (what tires it).
            if war_trace && state.turn.is_multiple_of(20) {
                let white = [sim_campaign::negotiation::Article::Peace];
                for enemy in state.factions[&england].at_war_with.clone() {
                    let since = state.factions[&england]
                        .war_started
                        .get(&enemy)
                        .copied()
                        .unwrap_or(0);
                    for (from, to) in [(&england, &enemy), (&enemy, &england)] {
                        let verdict = sim_campaign::negotiation::evaluate_treaty(
                            &state, data, from, to, &white,
                        );
                        println!(
                            "      other war since t{since}: white peace {} -> {}: chance {} {:?}",
                            from.as_str(),
                            to.as_str(),
                            verdict.chance,
                            verdict.reasons()
                        );
                    }
                }
            }
            if state.factions[&england].alive && state.factions[&france].alive {
                for blocker in war_blockers(&state, data, &england, &france) {
                    let index = BLOCKERS
                        .iter()
                        .position(|b| *b == blocker)
                        .expect("blocker");
                    report.blockers[index] += 1;
                }
            }
        }
        if war_trace {
            for e in events.iter().filter(|e| {
                matches!(e.kind, EventKind::WarDeclared) && e.text_fr.contains("Angleterre")
            }) {
                println!("    t{} {}", state.turn, e.text_fr);
            }
        }
        track_fe8(&state, data, &events, &names, &mut report.fe8);
        if state.turn == 80 {
            report.fe8.france_vassals[1] =
                sim_campaign::feudal::direct_vassals(&state, data, &france).len();
            report.fe8.france_first[0] = france_first(&state, data);
        }
        if at_war {
            report.fe8.first_war.get_or_insert(state.turn);
        }
        if at_war && !was_at_war {
            let by_france = events.iter().any(|e| {
                matches!(e.kind, EventKind::WarDeclared)
                    && e.text_fr.starts_with(names[0].as_str())
                    && e.text_fr.contains(names[1].as_str())
            });
            if by_france {
                report.declared_by_france += 1;
            } else {
                report.declared_by_england += 1;
            }
        }
        was_at_war = at_war;
        report.max_held = report.max_held.max(ai::alignment::realm_held_by(
            &state, data, &france, &england,
        ));
        if ai::alignment::dominates_realm(&state, data, &england, &france) {
            report.dominance_turns += 1;
        }
        for (index, (_, a, b)) in PAIRS.iter().enumerate() {
            // FE8 (ADR 0114): a direct feudal tie stands for an alliance.
            if state.is_allied(&id(a), &id(b))
                || sim_campaign::feudal::direct_tie(&state, data, &id(a), &id(b))
            {
                report.alliance_turns[index] += 1;
                report.alliance_first[index].get_or_insert(state.turn);
                if at_war {
                    report.alliance_war_turns[index] += 1;
                }
            }
        }
        let now = married(&state);
        for (_, _, cross) in now.difference(&seen) {
            if *cross {
                report.marriages_cross += 1;
            } else {
                report.marriages_same += 1;
            }
        }
        seen = now;
        for (fid, f) in &state.factions {
            let history = incomes.entry(fid.clone()).or_default();
            history.push(f.income_last_turn);
            if history.len() > 8 {
                history.remove(0);
            }
        }
        if state.turn >= TURN_1350 {
            for (fid, f) in state.factions.iter().filter(|(_, f)| f.alive) {
                if fid.as_str() == "fac_rebels" {
                    continue;
                }
                let history = &incomes[fid];
                let income = history.iter().sum::<i64>() / history.len().max(1) as i64;
                let ratio = f.treasury as f64 / income.max(1) as f64;
                if ratio > IDLE_SEASONS && f.treasury > 10_000 {
                    *over_limit.entry(fid.clone()).or_default() += 1;
                }
                if f.treasury > 10_000 && ratio > report.idle_ratio {
                    report.idle_ratio = ratio;
                    report.idle_faction = fid.to_string();
                }
            }
        }
        if trace && (state.turn.is_multiple_of(20) || (328..372).contains(&state.turn)) {
            trace_burgundy(&state, data, seed);
        }
        for (index, major) in MAJORS.iter().enumerate() {
            if !state.factions[&id(major)].alive {
                report.majors_fall[index].get_or_insert(state.turn);
            }
        }
        if state.turn == TURN_1400 {
            for (index, major) in MAJORS.iter().enumerate() {
                report.majors_1400[index] = state.factions[&id(major)].alive;
            }
        }
        if report.recruited_15th.iter().any(|seen| !seen) {
            for (index, (_, unit_type)) in CENTURY_15.iter().enumerate() {
                if !report.recruited_15th[index]
                    && state
                        .armies
                        .values()
                        .flat_map(|army| army.units.iter())
                        .chain(
                            state
                                .settlements
                                .values()
                                .flat_map(|settlement| settlement.garrison.iter()),
                        )
                        .any(|unit| unit.unit_type.as_str() == *unit_type)
                {
                    report.recruited_15th[index] = true;
                }
            }
        }
    }
    report.eq4.snowball_end = snowball(&state);
    report.eq4.eliminated = state
        .factions
        .iter()
        .filter(|(id, f)| !f.alive && id.as_str() != "fac_rebels")
        .map(|(id, _)| id.as_str().trim_start_matches("fac_").to_owned())
        .collect();
    report.idle_count = over_limit.values().filter(|n| **n > 4).count();
    report.fe8.france_vassals[2] =
        sim_campaign::feudal::direct_vassals(&state, data, &france).len();
    report.fe8.france_first[1] = france_first(&state, data);
    report.fe8.succession_wars = state
        .feudal
        .disputes
        .iter()
        .filter(|d| d.sponsor.is_some())
        .count();
    report.fe8.minor_max = small
        .iter()
        .map(|f| {
            (
                state.owned_provinces(f).len(),
                f.as_str().trim_start_matches("fac_").to_owned(),
            )
        })
        .max()
        .unwrap_or_default();
    report.factions = state
        .factions
        .keys()
        .filter(|f| f.as_str() != "fac_rebels")
        .count();
    report.seconds = started.elapsed().as_secs_f64();
    if verbose {
        let mut worst: Vec<_> = report.bankruptcies.iter().collect();
        worst.sort_by_key(|(_, n)| std::cmp::Reverse(**n));
        println!("  [{seed}] bankruptcies {worst:?}");
        println!("  [{seed}] idle treasuries (turns over limit) {over_limit:?}");
    }
    report
}

fn main() {
    let mut args = std::env::args().skip(1);
    let turns: u32 = args.next().and_then(|s| s.parse().ok()).unwrap_or(464);
    let mut seeds: Vec<u64> = args.filter_map(|s| s.parse().ok()).collect();
    if seeds.is_empty() {
        seeds = vec![1, 2, 3, 4, 5];
    }
    let verbose = std::env::var("VERBOSE").is_ok();
    // DC6b: `CENT_ANS_DATA_DIR` plays another copy of the data (variants of a rule).
    let root = std::env::var("CENT_ANS_DATA_DIR").map_or_else(
        |_| Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../data"),
        std::path::PathBuf::from,
    );
    let (data, _) = GameData::load(&root).expect("data");
    // Seeds run in parallel (each campaign is deterministic on its own).
    let reports: Vec<Report> = std::thread::scope(|scope| {
        let handles: Vec<_> = seeds
            .iter()
            .map(|seed| {
                let data = &data;
                scope.spawn(move || run(data, *seed, turns, verbose))
            })
            .collect();
        handles
            .into_iter()
            .map(|h| h.join().expect("campaign thread"))
            .collect()
    });

    let decades = f64::from(turns) / 40.0;
    println!(
        "\n| graine | guerre FR-EN | phases | batailles FR/EN / déc. | prises | trésor max (saisons) | trésors > 8 | banqueroutes / fac. / déc. | pire faction | mariages (inter / intra) | appels (oui / non) | majeures en 1400 | détruites | refus FR | s |"
    );
    println!("|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|");
    for r in &reports {
        let total: u32 = r.bankruptcies.values().sum();
        let (worst, worst_n) = r
            .bankruptcies
            .iter()
            .max_by_key(|(f, n)| (**n, std::cmp::Reverse((*f).clone())))
            .map_or(("-".to_owned(), 0), |(f, n)| (f.to_string(), *n));
        let majors: usize = r.majors_1400.iter().filter(|a| **a).count();
        println!(
            "| {} | {:.0} % | {} | {:.1} | {} | {:.1} ({}) | {} | {:.2} | {} ({:.1}/déc.) | {} / {} | {} / {} | {}/4 | {} | {:.1} % | {:.1} |",
            r.seed,
            100.0 * f64::from(r.war_turns) / f64::from(r.turns),
            r.war_phases,
            f64::from(r.battles_fr_en) / decades,
            r.captures,
            r.idle_ratio,
            r.idle_faction.trim_start_matches("fac_"),
            r.idle_count,
            f64::from(total) / r.factions as f64 / decades,
            worst.trim_start_matches("fac_"),
            f64::from(worst_n) / decades,
            r.marriages_cross,
            r.marriages_same,
            r.calls_honoured,
            r.calls_refused,
            majors,
            r.destroyed,
            100.0 * f64::from(r.refused) / f64::from(r.issued.max(1)),
            r.seconds,
        );
    }
    println!("\nAlliances (part des tours) :");
    for (index, (label, _, _)) in PAIRS.iter().enumerate() {
        let shares: Vec<String> = reports
            .iter()
            .map(|r| {
                format!(
                    "{:.0} %",
                    100.0 * f64::from(r.alliance_turns[index]) / f64::from(r.turns)
                )
            })
            .collect();
        let war_shares: Vec<String> = reports
            .iter()
            .map(|r| {
                format!(
                    "{:.0} %",
                    100.0 * f64::from(r.alliance_war_turns[index]) / f64::from(r.war_turns.max(1))
                )
            })
            .collect();
        let first: Vec<String> = reports
            .iter()
            .map(|r| {
                r.alliance_first[index].map_or("-".to_owned(), |turn| {
                    format!("{}", 1337 + (turn.saturating_sub(1)) / 4)
                })
            })
            .collect();
        println!(
            "  {label:14} {}   (en guerre FR-EN : {} ; dès {})",
            shares.join(" | "),
            war_shares.join(" | "),
            first.join(" | ")
        );
    }
    let scots: Vec<String> = reports
        .iter()
        .map(|r| {
            let n = r
                .bankruptcies
                .get(&id("fac_scotland"))
                .copied()
                .unwrap_or(0);
            format!("{:.1}", f64::from(n) / decades)
        })
        .collect();
    let dominance: Vec<String> = reports
        .iter()
        .map(|r| {
            format!(
                "{:.0} % (max {} prov.)",
                100.0 * f64::from(r.dominance_turns) / f64::from(r.turns),
                r.max_held
            )
        })
        .collect();
    println!("Angleterre dominant le royaume : {}", dominance.join(" | "));
    println!("Banqueroutes de l'Écosse / déc. : {}", scots.join(" | "));
    let survivors: Vec<String> = MAJORS
        .iter()
        .enumerate()
        .map(|(index, m)| {
            let n = reports.iter().filter(|r| r.majors_1400[index]).count();
            format!("{} {n}/{}", m.trim_start_matches("fac_"), reports.len())
        })
        .collect();
    println!("Survie en 1400 : {}", survivors.join(", "));
    let century_15: Vec<String> = CENTURY_15
        .iter()
        .enumerate()
        .map(|(index, (label, _))| {
            let n = reports.iter().filter(|r| r.recruited_15th[index]).count();
            format!("{label} {n}/{}", reports.len())
        })
        .collect();
    println!("Types du XVe s. recrutés (UR2) : {}", century_15.join(", "));
    print_summary(&reports, decades);
    print_eq4(&reports);
    print_eq6(&reports);
    print_fe8(&reports);
    if std::env::var("CV3_STATS").is_ok() {
        print_cv3(&reports);
    }
}

/// EQ6: who opens the France-England wars and what keeps England at peace.
/// FE8: one turn of feudal measures (after `end_turn`).
fn track_fe8(
    state: &CampaignState,
    data: &GameData,
    events: &[sim_campaign::GameEvent],
    names: &[String],
    fe8: &mut Fe8,
) {
    let france = id("fac_france");
    let empire = id("fac_empire");
    let empire_name = data.factions[&empire].short_or_display_name().to_owned();
    for e in events {
        let text = e.text_fr.as_str();
        if text.contains("prononce la commise") {
            fe8.commises += 1;
            if e.faction.as_ref() == Some(&france) && text.contains(names[1].as_str()) {
                fe8.guyenne_commise.get_or_insert(state.turn);
            }
        } else if text.starts_with("La commise est exécutée") {
            fe8.commises_executed += 1;
        } else if text.starts_with("Félonie de") {
            fe8.felonies += 1;
        } else if text.contains("répond à l'ost de son suzerain")
            && text.contains(&format!("suzerain {empire_name} "))
        {
            fe8.imperial_host += 1;
            if text.ends_with(&format!("contre {}.", names[0])) {
                fe8.imperial_host_vs_france += 1;
            }
        }
    }
    let turn = state.turn;
    if turn <= 50 {
        fe8.italian_war_turns += ITALIANS
            .iter()
            .filter(|f| state.is_at_war(&id(f), &france))
            .count() as u32;
        if let Some(f) = state.factions.get(&france) {
            fe8.france_income_50 += f.income_last_turn;
            if turn == 50 {
                fe8.france_treasury_50 = f.treasury;
            }
        }
    }
    let empire_allies = &state.factions[&empire].allies;
    if sim_campaign::feudal::direct_vassals(state, data, &empire)
        .iter()
        .any(|v| empire_allies.contains(v))
    {
        fe8.empire_allied_vassal_turns += 1;
    }
}

fn print_fe8(reports: &[Report]) {
    println!("\nFE8 — féodalité :");
    println!(
        "| graine | commise Guyenne (tour) | 1re guerre FR-EN | commises / exécutées | félonies | ost impérial (dont c. France) | Italiens en guerre c. France (tours×fac., t≤50) | Empire allié à son vassal (tours) | recettes France t≤50 | trésor France t50 | banqueroutes petites fac. / fac. / déc. | banqueroutes 28 fac. d'avant FE / fac. / déc. |"
    );
    println!("|---|---|---|---|---|---|---|---|---|---|---|---|");
    for r in reports {
        let f = &r.fe8;
        let small_rate = f64::from(f.small_bankruptcies)
            / f.small_factions.max(1) as f64
            / (f64::from(r.turns) / 40.0);
        let legacy_rate =
            f64::from(f.legacy_bankruptcies) / LEGACY.len() as f64 / (f64::from(r.turns) / 40.0);
        println!(
            "| {} | {} | {} | {} / {} | {} | {} ({}) | {} | {} | {} | {} | {:.2} | {:.2} |",
            r.seed,
            f.guyenne_commise.map_or("-".to_owned(), |t| t.to_string()),
            f.first_war.map_or("-".to_owned(), |t| t.to_string()),
            f.commises,
            f.commises_executed,
            f.felonies,
            f.imperial_host,
            f.imperial_host_vs_france,
            f.italian_war_turns,
            f.empire_allied_vassal_turns,
            f.france_income_50,
            f.france_treasury_50,
            small_rate,
            legacy_rate,
        );
    }
    println!(
        "| graine | vassaux directs de la France (1337 / t80 / fin) | France 1er royaume par population (t80 / fin) | guerres de succession | mineure la plus grande à la fin (provinces) |"
    );
    println!("|---|---|---|---|---|");
    for r in reports {
        let f = &r.fe8;
        println!(
            "| {} | {} / {} / {} | {} / {} | {} | {} ({}) |",
            r.seed,
            f.france_vassals[0],
            f.france_vassals[1],
            f.france_vassals[2],
            if f.france_first[0] { "oui" } else { "non" },
            if f.france_first[1] { "oui" } else { "non" },
            f.succession_wars,
            f.minor_max.1,
            f.minor_max.0,
        );
    }
    let n = reports.len();
    let guyenne = reports
        .iter()
        .filter(|r| r.fe8.guyenne_commise.is_some())
        .count();
    let trigger = reports
        .iter()
        .filter(|r| r.fe8.guyenne_commise.is_some() && r.fe8.guyenne_commise == r.fe8.first_war)
        .count();
    println!("  Commise de Guyenne : {guyenne}/{n} graines, dont {trigger}/{n} ouvrant la 1re guerre FR-EN.");
    let income: Vec<f64> = reports
        .iter()
        .map(|r| r.fe8.france_income_50 as f64)
        .collect();
    let (mean, min, max) = spread(&income);
    println!("  Recettes France t≤50 : moy. {mean:.0} [{min:.0}-{max:.0}]");
}

fn print_eq6(reports: &[Report]) {
    println!("\nEQ6 — tours de paix FR-EN, obstacles à la déclaration anglaise :");
    println!(
        "| graine | guerre | ouvertes par Angl. / Fr. | tours de paix | {} |",
        BLOCKERS.join(" | ")
    );
    println!("|---|---|---|---|{}", "---|".repeat(BLOCKERS.len()));
    for r in reports {
        let peace = r.turns - r.war_turns;
        let cells: Vec<String> = r.blockers.iter().map(|n| n.to_string()).collect();
        println!(
            "| {} | {:.0} % | {} / {} | {} | {} |",
            r.seed,
            100.0 * f64::from(r.war_turns) / f64::from(r.turns),
            r.declared_by_england,
            r.declared_by_france,
            peace,
            cells.join(" | ")
        );
    }
}

/// Bankruptcies per faction and decade.
fn bankruptcy_rate(r: &Report) -> f64 {
    let total: u32 = r.bankruptcies.values().sum();
    f64::from(total) / r.factions as f64 / (f64::from(r.turns) / 40.0)
}

/// Share of the sieges seen at the end of a turn that ended in a capture.
fn siege_success(r: &Report) -> f64 {
    let resolved = (r.eq4.sieges_won + r.eq4.sieges_failed).max(1);
    100.0 * f64::from(r.eq4.sieges_won) / f64::from(resolved)
}

/// One EQ4 measure of a campaign.
type Measure = fn(&Report) -> f64;

/// CV3-6 (`CV3_STATS=1`): stances, ambushes, encounters and outcome classes
/// per campaign, with the rates per 20 turns (the pilot game of spec § 0).
fn print_cv3(reports: &[Report]) {
    use data_model::BattleOutcomeClass as C;
    println!(
        "\nCV3 — campagne vivante ({}) :",
        std::env::var("DIFFICULTY").unwrap_or_else(|_| "normal".to_owned())
    );
    let keys: Vec<&str> = C::ALL.iter().map(|c| c.key()).collect();
    println!(
        "| graine | guerre FR-EN | révoltes | embuscades (ordres / réussies / éventées) | marches forcées | camps retranchés | rencontres | batailles | classes ({}) |",
        keys.join(" / ")
    );
    println!("|---|---|---|---|---|---|---|---|---|");
    for r in reports {
        let c = &r.cv3;
        let classes: Vec<String> = c.classes.iter().map(|n| n.to_string()).collect();
        println!(
            "| {} | {:.0} % | {} | {} / {} / {} | {} | {} | {} | {} | {} |",
            r.seed,
            100.0 * f64::from(r.war_turns) / f64::from(r.turns),
            r.eq4.revolts,
            c.ambush_orders,
            c.ambush_sprung,
            c.ambush_failed,
            c.forced_march_orders,
            c.entrenched_orders,
            c.encounters,
            c.battles,
            classes.join(" / "),
        );
    }
    let per20 = |f: fn(&Cv3) -> u32| -> f64 {
        let turns: u32 = reports.iter().map(|r| r.turns).sum();
        let total: u32 = reports.iter().map(|r| f(&r.cv3)).sum();
        20.0 * f64::from(total) / f64::from(turns.max(1))
    };
    println!(
        "  Par 20 tours (toutes factions) : embuscades {:.2} ordres, {:.2} réussies, {:.2} éventées ; marches forcées {:.2} ; camps retranchés {:.2} ; rencontres {:.2} ; batailles {:.1}",
        per20(|c| c.ambush_orders),
        per20(|c| c.ambush_sprung),
        per20(|c| c.ambush_failed),
        per20(|c| c.forced_march_orders),
        per20(|c| c.entrenched_orders),
        per20(|c| c.encounters),
        per20(|c| c.battles),
    );
    let mut totals = [0u32; 7];
    for r in reports {
        for (t, n) in totals.iter_mut().zip(r.cv3.classes) {
            *t += n;
        }
    }
    let sum: u32 = totals.iter().sum::<u32>().max(1);
    let shares: Vec<String> = keys
        .iter()
        .zip(totals)
        .map(|(k, n)| format!("{k} {:.1} %", 100.0 * f64::from(n) / f64::from(sum)))
        .collect();
    println!("  Classes de résultat (camps) : {}", shares.join(", "));
    let revolts: Vec<f64> = reports.iter().map(|r| f64::from(r.eq4.revolts)).collect();
    let (mean, min, max) = spread(&revolts);
    println!("  Révoltes / partie : moy. {mean:.1} [{min:.0}-{max:.0}]");
}

/// EQ4: the combined balance table, one row per seed and the means.
fn print_eq4(reports: &[Report]) {
    println!(
        "\nEQ4 — tableau combiné ({}) :",
        std::env::var("DIFFICULTY").unwrap_or_else(|_| "normal".to_owned())
    );
    println!(
        "| graine | guerre FR-EN | trêves | révoltes (/200 t.) | banqueroutes / fac. / déc. | sièges engagés | sièges réussis | prises directes | 1437 : 1re faction | fin : 1re faction | éliminées | accès militaires (accords, paires moy.) | intrusions (saisons, casus belli) |"
    );
    println!("|---|---|---|---|---|---|---|---|---|---|---|---|---|");
    for r in reports {
        let e = &r.eq4;
        println!(
            "| {} | {:.0} % | {} | {} ({:.1}) | {:.2} | {} | {:.0} % ({}/{}) | {} | {} {:.0} % | {} {:.0} % | {} ({}) | {} ({:.1}) | {} ({}) |",
            r.seed,
            100.0 * f64::from(r.war_turns) / f64::from(r.turns),
            r.truces,
            e.revolts,
            f64::from(e.revolts) * 200.0 / f64::from(r.turns),
            bankruptcy_rate(r),
            e.sieges_started,
            siege_success(r),
            e.sieges_won,
            e.sieges_won + e.sieges_failed,
            e.direct_captures,
            e.snowball_1437.0,
            e.snowball_1437.1,
            e.snowball_end.0,
            e.snowball_end.1,
            e.eliminated.len(),
            e.eliminated.join(", "),
            e.access_grants,
            e.access_pair_turns as f64 / f64::from(r.turns),
            e.trespass_seasons,
            e.trespass_grievances,
        );
    }
    let rows: &[(&str, Measure)] = &[
        ("Guerre FR-EN (%)", |r| {
            100.0 * f64::from(r.war_turns) / f64::from(r.turns)
        }),
        ("Trêves FR-EN / siècle", |r| f64::from(r.truces)),
        ("Révoltes / 200 tours", |r| {
            f64::from(r.eq4.revolts) * 200.0 / f64::from(r.turns)
        }),
        ("Banqueroutes / fac. / déc.", bankruptcy_rate),
        ("Sièges engagés", |r| f64::from(r.eq4.sieges_started)),
        ("Sièges réussis (%)", siege_success),
        ("Prises directes", |r| f64::from(r.eq4.direct_captures)),
        ("1re faction 1437 (% prov.)", |r| r.eq4.snowball_1437.1),
        ("1re faction fin (% prov.)", |r| r.eq4.snowball_end.1),
        ("Factions éliminées", |r| r.eq4.eliminated.len() as f64),
        ("Accès militaires accordés", |r| {
            f64::from(r.eq4.access_grants)
        }),
        ("Saisons d'intrusion", |r| f64::from(r.eq4.trespass_seasons)),
        ("Casus belli d'intrusion", |r| {
            f64::from(r.eq4.trespass_grievances)
        }),
        ("DC3 cités prises / déc.", |r| {
            f64::from(r.eq4.city_captures) * 40.0 / f64::from(r.turns)
        }),
        ("DC3 impôt haut (% fac.-tours)", |r| {
            100.0 * f64::from(r.eq4.high_tax_turns) / f64::from(r.eq4.faction_turns.max(1))
        }),
        ("DC3 hommes en campagne (moy. / tour)", |r| {
            r.eq4.men_sum as f64 / f64::from(r.turns.max(1))
        }),
        ("DC3 dont reprises aux rebelles / déc.", |r| {
            f64::from(r.eq4.city_from_rebels) * 40.0 / f64::from(r.turns)
        }),
        ("DC3 provinces conquises en entier / déc.", |r| {
            f64::from(r.eq4.provinces_completed) * 40.0 / f64::from(r.turns)
        }),
        ("DC3 dévastation moy. (par province)", |r| {
            r.eq4.devastation_sum as f64 / f64::from(r.eq4.province_turns.max(1))
        }),
        ("DC3 provinces occupées (% prov.-tours)", |r| {
            100.0 * f64::from(r.eq4.occupied_turns) / f64::from(r.eq4.province_turns.max(1))
        }),
        ("DC3 mécontentement > 60 (‰ prov.-tours)", |r| {
            1000.0 * f64::from(r.eq4.hot_turns) / f64::from(r.eq4.province_turns.max(1))
        }),
        ("DC3 replis vers un refuge neutre", |r| {
            f64::from(r.eq4.retreat_fallback)
        }),
        ("DC3 débandades ralliées", |r| {
            f64::from(r.eq4.retreat_rout)
        }),
        ("DC3 armées dispersées", |r| {
            f64::from(r.eq4.retreat_dispersed)
        }),
        ("DC3 durée moy. d'une conquête (tours)", |r| {
            f64::from(r.eq4.conquest_turns) / f64::from(r.eq4.provinces_completed.max(1))
        }),
    ];
    println!("Moyennes EQ4 :");
    for (label, f) in rows {
        let (m, lo, hi) = spread(&reports.iter().map(f).collect::<Vec<_>>());
        println!("  {label} : moy. {m:.2} [{lo:.2}-{hi:.2}]");
    }
}

/// Mean, minimum and maximum of `values`.
fn spread(values: &[f64]) -> (f64, f64, f64) {
    let n = values.len().max(1) as f64;
    let mean = values.iter().sum::<f64>() / n;
    let min = values.iter().copied().fold(f64::INFINITY, f64::min);
    let max = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    (mean, min, max)
}

/// G5 synthesis: the indicators of `docs/status.md` (G2, G4) over all seeds.
fn print_summary(reports: &[Report], decades: f64) {
    let pair = |label: &str| {
        PAIRS
            .iter()
            .position(|(l, _, _)| *l == label)
            .expect("pair")
    };
    let war_share = |r: &Report, index: usize| {
        100.0 * f64::from(r.alliance_war_turns[index]) / f64::from(r.war_turns.max(1))
    };
    let war: Vec<f64> = reports
        .iter()
        .map(|r| 100.0 * f64::from(r.war_turns) / f64::from(r.turns))
        .collect();
    let (mean, min, max) = spread(&war);
    let in_band = war.iter().filter(|w| (55.0..=75.0).contains(*w)).count();
    println!("\nSynthèse ({} graines) :", reports.len());
    println!(
        "  Guerre FR-EN : moy. {mean:.0} % [{min:.0}-{max:.0}], {in_band}/{} graines dans 55-75 %",
        reports.len()
    );
    let truces: Vec<String> = reports
        .iter()
        .map(|r| format!("{} ({} ans)", r.truces, r.longest_war / 4))
        .collect();
    println!(
        "  Paix FR-EN par siècle (plus longue guerre) : {}",
        truces.join(" | ")
    );
    for label in ["Auld Alliance", "Angl.-Brabant", "Bourg.-France"] {
        let index = pair(label);
        let shares: Vec<f64> = reports.iter().map(|r| war_share(r, index)).collect();
        let (mean, min, max) = spread(&shares);
        println!("  {label} (tours de guerre FR-EN) : moy. {mean:.0} % [{min:.0}-{max:.0}]");
    }
    let burgundy = pair("Bourg.-Angl.");
    // Turn 253 is the first of 1400.
    let fifteenth = reports
        .iter()
        .filter(|r| r.alliance_first[burgundy].is_some_and(|t| t > TURN_1400))
        .count();
    let any = reports
        .iter()
        .filter(|r| r.alliance_first[burgundy].is_some())
        .count();
    println!(
        "  Bourg.-Angl. : {any}/{n} graines, dont {fifteenth}/{n} au XVe s.",
        n = reports.len()
    );
    let bankruptcies: Vec<f64> = reports
        .iter()
        .map(|r| {
            let total: u32 = r.bankruptcies.values().sum();
            f64::from(total) / r.factions as f64 / decades
        })
        .collect();
    let (mean, min, max) = spread(&bankruptcies);
    println!("  Banqueroutes / fac. / déc. : moy. {mean:.2} [{min:.2}-{max:.2}]");
    let calls: Vec<f64> = reports
        .iter()
        .map(|r| f64::from(r.calls_honoured))
        .collect();
    let (mean, min, max) = spread(&calls);
    println!("  Appels aux armes honorés : moy. {mean:.0} [{min:.0}-{max:.0}]");
    let all_alive = reports
        .iter()
        .filter(|r| r.majors_1400.iter().all(|a| *a))
        .count();
    println!(
        "  4 majeures en vie en 1400 : {all_alive}/{}",
        reports.len()
    );
    for r in reports {
        for (index, major) in MAJORS.iter().enumerate() {
            if let Some(turn) = r.majors_fall[index] {
                println!(
                    "    graine {} : {} tombe en {}{}",
                    r.seed,
                    major.trim_start_matches("fac_"),
                    1337 + turn.saturating_sub(1) / 4,
                    if r.majors_1400[index] {
                        ""
                    } else {
                        " (avant 1400)"
                    }
                );
            }
        }
    }
}
