//! Campaign AI quality probe: how *well* the AI plays (not balance), AI
//! against AI from 1337 (`docs/wip/ia-quality-probe.md`).
//!
//! France is answered by the AI like in `settlements_probe`; every other
//! faction plays through `end_turn_with(data, planner)`, where the planner
//! is `ai::plan_turn` wrapped by a [`Recorder`]: at the very moment a
//! faction plans, the recorder notes what that faction could see (its
//! armies, the empty enemy places within reach, its besieged places and
//! who could relieve them, its attacks). The state right after `end_turn`
//! then tells what the AI actually did with it.
//!
//! Metrics (all living non-rebel factions, per turn):
//! 1. **Missed captures**: an army ends the turn idle while an enemy place
//!    with no garrison and no enemy army on it or one edge away was within
//!    this turn's reach. Reach proxy: the AI's own route table
//!    (`GridPlanner::table` from the army's anchor, budget = movement
//!    points left when the faction planned, stronger enemies avoided), no
//!    sea crossing on the path, the place still held by the same enemy
//!    after the turn.
//! 2. **Undefended losses**: a place lost this turn that already had an
//!    enemy army on it when its holder planned, while an army of the holder
//!    at least as strong as the besiegers could reach it this turn (same
//!    table) and neither ended the turn there nor headed for it nor fought.
//! 3. **Idle at war**: share of army-turns (factions at war with a
//!    non-rebel enemy) where the army ends idle — did not move since its
//!    faction planned, no destination, not besieging, no battle — and is
//!    neither guarding a threatened own place (enemy army within 2
//!    provinces), nor resting (`stances::rest_plan`, supply < 30, strength
//!    < 40 % of full), nor in ambush or entrenched.
//! 4. **Suicide battles**: field battles (event `Battle` "… Vainqueur …",
//!    whose `army` is the attacker and `faction` the winner) the attacker
//!    lost with its power < 0.8 × the defender's before the battle. Powers
//!    at the attacker's planning time (`army_power`); the defender is the
//!    `Attack` target when the AI ordered one, else the strongest hostile
//!    army in the battle's province (a march into an enemy).
//! 5. **Fragmentation**: mean number of armies per faction at war whose
//!    power is under 25 % of the faction's largest army.
//! 6. **Sieges**: started, captured, abandoned (lifted without capture),
//!    mean turns to capture; captures seen without a siege are "direct".
//! 7. **Treasury**: share of faction-turns with a negative treasury, and
//!    with a treasury above 12 seasons of `income_last_turn` (hoarding).
//! 8. **Cost**: simulation ms per turn (the recorder's own time excluded)
//!    and probe ms per turn.
//!
//! Usage: `ia_quality_probe [turns] [seed...]` (default 60 turns, seeds
//! 1-4). `VERBOSE=1` lists the top examples of metrics 1, 2 and 4.
use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::time::{Duration, Instant};

use data_model::{FactionId, GameData, ProvinceId, SettlementId, SettlementKind};
use sim_campaign::movement::{edges, is_sea_crossing, path_to};
use sim_campaign::{ArmyId, CampaignState, EventKind, GameEvent, MoveTarget, Order, Stance};

const REBELS: &str = "fac_rebels";
/// Attacker power below this share of the defender's: a suicide battle.
const SUICIDE_RATIO: f64 = 0.8;
/// Armies under this share of the faction's largest one are fragments.
const FRAGMENT_SHARE: f64 = 0.25;
/// A treasury above this many seasons of income is hoarding.
const HOARD_SEASONS: i64 = 12;
/// Resting proxies besides `stances::rest_plan` (as `RETREAT_STRENGTH`).
const LOW_SUPPLY: u8 = 30;
const LOW_STRENGTH: f64 = 0.4;
/// An army that moved less than this (map pixels) did not move.
const MOVED_PX: f32 = 0.5;
/// Examples listed per metric with `VERBOSE=1`.
const EXAMPLES: usize = 5;

fn distance(a: [f32; 2], b: [f32; 2]) -> f32 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2)).sqrt()
}

/// Examples sort cities first.
fn kind_rank(kind: SettlementKind) -> u8 {
    match kind {
        SettlementKind::City => 0,
        SettlementKind::Town => 1,
        SettlementKind::Castle => 2,
        SettlementKind::Abbey => 3,
        SettlementKind::Village => 4,
    }
}

/// An army as its faction saw it when planning.
struct ArmySnap {
    faction: FactionId,
    power: f64,
    point: [f32; 2],
    /// Resting, low supply or broken: idleness is legitimate.
    resting: bool,
    /// Standing in a place of its own with an enemy army within 2 provinces.
    guarding: bool,
    /// Besieging an enemy place.
    besieging: bool,
    /// Enemy places empty and within this turn's reach (metric 1).
    capturable: Vec<SettlementId>,
}

/// A place of the planning faction with an enemy army on it (metric 2).
struct Threatened {
    faction: FactionId,
    /// Armies of the holder at least as strong as the besiegers, within
    /// this turn's reach.
    relievers: Vec<ArmyId>,
}

/// What each faction saw when it planned this turn.
#[derive(Default)]
struct Recorder {
    armies: BTreeMap<ArmyId, ArmySnap>,
    threatened: BTreeMap<SettlementId, Threatened>,
    /// `Attack` orders: attacker → target power at planning time.
    attacks: BTreeMap<ArmyId, f64>,
    /// Per planning faction: strongest hostile army by province.
    hostile_max: BTreeMap<FactionId, BTreeMap<ProvinceId, f64>>,
    /// Time spent recording (excluded from the simulation time).
    spent: Duration,
}

impl Recorder {
    /// Notes what `faction` sees before it plans.
    fn record(&mut self, state: &CampaignState, data: &GameData, faction: &FactionId) {
        let started = Instant::now();
        let Some(me) = state.factions.get(faction) else {
            return;
        };
        let enemies: BTreeSet<&FactionId> = me.at_war_with.iter().collect();
        // Hostile armies: anchors, provinces, strongest per province.
        let mut hostile_anchor: BTreeSet<SettlementId> = BTreeSet::new();
        let mut hostile_provinces: BTreeSet<ProvinceId> = BTreeSet::new();
        let mut max_by_province: BTreeMap<ProvinceId, f64> = BTreeMap::new();
        for (id, army) in &state.armies {
            if !enemies.contains(&army.faction) {
                continue;
            }
            if let Some(anchor) = state.army_anchor(data, army) {
                hostile_anchor.insert(anchor);
            }
            if let Some(p) = state.army_province(data, army) {
                let power = state.army_power(data, id);
                let slot = max_by_province.entry(p.clone()).or_insert(0.0);
                *slot = slot.max(power);
                hostile_provinces.insert(p);
            }
        }
        // Empty enemy places with no enemy army on them or one edge away.
        let empty: BTreeSet<SettlementId> = state
            .settlements
            .iter()
            .filter(|(_, s)| enemies.contains(&s.controller))
            .filter(|(_, s)| s.garrison.iter().all(|u| u.strength == 0))
            .filter(|(id, _)| {
                !hostile_anchor.contains(*id)
                    && edges(data, id)
                        .iter()
                        .all(|(n, _)| !hostile_anchor.contains(n))
            })
            .map(|(id, _)| id.clone())
            .collect();
        // Own places with an enemy army on them, and the besiegers' power.
        let threatened: BTreeMap<SettlementId, f64> = state
            .settlements
            .iter()
            .filter(|(_, s)| &s.controller == faction)
            .filter_map(|(sid, _)| {
                let besiegers: f64 = state
                    .hostile_armies_at(faction, sid)
                    .iter()
                    .map(|id| state.army_power(data, id))
                    .sum();
                (besiegers > 0.0).then(|| (sid.clone(), besiegers))
            })
            .collect();
        let near_threat = |p: &ProvinceId| {
            let mut ring: BTreeSet<ProvinceId> = BTreeSet::from([p.clone()]);
            for n in data.province_land_neighbors(p) {
                ring.insert(n.clone());
                ring.extend(data.province_land_neighbors(n).iter().cloned());
            }
            ring.iter().any(|q| hostile_provinces.contains(q))
        };
        let needs_routes = !empty.is_empty() || !threatened.is_empty();
        let grid = needs_routes.then(|| ai::grid::GridPlanner::new(state, data, faction));
        let mut relievers: BTreeMap<SettlementId, Vec<ArmyId>> = BTreeMap::new();
        for (id, army) in state.armies.iter().filter(|(_, a)| &a.faction == faction) {
            let power = state.army_power(data, id);
            let strength: u32 = army.units.iter().map(|u| u.strength).sum();
            let full: u32 = army.units.iter().map(|u| u.max_strength).sum();
            let resting = army.supply < LOW_SUPPLY
                || (full > 0 && f64::from(strength) < LOW_STRENGTH * f64::from(full))
                || matches!(
                    ai::stances::rest_plan(state, data, faction, id),
                    ai::stances::RestPlan::Rest { .. }
                );
            let guarding = army
                .settlement()
                .is_some_and(|s| state.is_friendly_settlement(faction, s))
                && state
                    .army_province(data, army)
                    .is_some_and(|p| near_threat(&p));
            let mut capturable = Vec::new();
            if let (Some(grid), Some(anchor)) = (&grid, state.army_anchor(data, army)) {
                if army.movement_left > 0 && power > 0.0 {
                    let cap = state.army_movement_allowance(data, army);
                    let table = grid.table(&anchor, army.movement_left, cap, power);
                    let by_land = |target: &SettlementId| {
                        path_to(&table, target).is_some_and(|path| {
                            let mut previous = anchor.clone();
                            path.into_iter().all(|node| {
                                let land = !is_sea_crossing(data, &previous, &node);
                                previous = node;
                                land
                            })
                        })
                    };
                    capturable = empty
                        .iter()
                        .filter(|s| table.contains_key(*s) && by_land(s))
                        .cloned()
                        .collect();
                    for (sid, besiegers) in &threatened {
                        if power >= *besiegers && !army.is_at(sid) && table.contains_key(sid) {
                            relievers.entry(sid.clone()).or_default().push(id.clone());
                        }
                    }
                }
            }
            self.armies.insert(
                id.clone(),
                ArmySnap {
                    faction: faction.clone(),
                    power,
                    point: state.army_point(data, army),
                    resting,
                    guarding,
                    besieging: army
                        .settlement()
                        .and_then(|s| state.settlements.get(s))
                        .and_then(|s| s.siege.as_ref())
                        .is_some_and(|s| &s.attacker == faction),
                    capturable,
                },
            );
        }
        for sid in threatened.into_keys() {
            let relievers = relievers.remove(&sid).unwrap_or_default();
            self.threatened.insert(
                sid,
                Threatened {
                    faction: faction.clone(),
                    relievers,
                },
            );
        }
        self.hostile_max.insert(faction.clone(), max_by_province);
        self.spent += started.elapsed();
    }

    /// Notes the `Attack` orders of a plan.
    fn record_orders(&mut self, state: &CampaignState, data: &GameData, orders: &[Order]) {
        for order in orders {
            if let Order::Attack { army, target_army } = order {
                self.attacks
                    .insert(army.clone(), state.army_power(data, target_army));
            }
        }
    }
}

/// One example: (turn, faction, army, place, kind rank, what the army was
/// doing when its faction planned).
type Example = (u32, FactionId, ArmyId, SettlementId, u8, &'static str);

impl ArmySnap {
    /// What the army was doing when its faction planned (examples).
    fn note(&self) -> &'static str {
        if self.besieging {
            "assiégeait"
        } else if self.resting {
            "au repos"
        } else if self.guarding {
            "gardait une place menacée"
        } else {
            "libre"
        }
    }
}

/// Counters of one seed.
#[derive(Default)]
struct Report {
    turns: u32,
    missed_army_turns: u32,
    missed_examples: Vec<Example>,
    losses_seen: u32,
    undefended_losses: u32,
    undefended_examples: Vec<Example>,
    war_army_turns: u32,
    idle_army_turns: u32,
    battles: u32,
    battles_lost: u32,
    suicide: u32,
    suicide_examples: Vec<String>,
    war_faction_turns: u32,
    fragments: u32,
    sieges_started: u32,
    sieges_captured: u32,
    sieges_abandoned: u32,
    siege_turns: u32,
    direct_captures: u32,
    faction_turns: u32,
    negative_treasury: u32,
    hoarding: u32,
    sim: Duration,
    probe: Duration,
}

/// Column headers of the tables, in [`Report::row`] order.
const HEADERS: &[&str] = &[
    "prises manquées",
    "/tour",
    "pertes sans secours",
    "pertes sous siège vu",
    "% oisives en guerre",
    "batailles",
    "perdues (att.)",
    "suicidaires",
    "fragments/faction",
    "sièges lancés",
    "pris",
    "abandonnés",
    "tours/prise",
    "prises directes",
    "% trésor < 0",
    "% thésaurisation",
    "ms/tour sim",
    "ms/tour sonde",
];

impl Report {
    fn pct(n: u32, d: u32) -> f64 {
        if d == 0 {
            0.0
        } else {
            100.0 * f64::from(n) / f64::from(d)
        }
    }

    fn per(n: u32, d: u32) -> f64 {
        f64::from(n) / f64::from(d.max(1))
    }

    /// The values, in [`HEADERS`] order.
    fn row(&self) -> Vec<f64> {
        let ms_per_turn = |d: Duration| d.as_secs_f64() * 1000.0 / f64::from(self.turns.max(1));
        vec![
            f64::from(self.missed_army_turns),
            Self::per(self.missed_army_turns, self.turns),
            f64::from(self.undefended_losses),
            f64::from(self.losses_seen),
            Self::pct(self.idle_army_turns, self.war_army_turns),
            f64::from(self.battles),
            f64::from(self.battles_lost),
            f64::from(self.suicide),
            Self::per(self.fragments, self.war_faction_turns),
            f64::from(self.sieges_started),
            f64::from(self.sieges_captured),
            f64::from(self.sieges_abandoned),
            Self::per(self.siege_turns, self.sieges_captured),
            f64::from(self.direct_captures),
            Self::pct(self.negative_treasury, self.faction_turns),
            Self::pct(self.hoarding, self.faction_turns),
            ms_per_turn(self.sim),
            ms_per_turn(self.probe),
        ]
    }
}

/// Sieges before the turn (place → attacker, first turn) and controllers.
struct Before {
    sieges: BTreeMap<SettlementId, (FactionId, u32)>,
    controllers: BTreeMap<SettlementId, FactionId>,
}

fn run(data: &GameData, seed: u64, turns: u32) -> Report {
    let france = FactionId::new("fac_france").expect("well-formed id");
    let mut state = CampaignState::new_1337(data, france.clone(), seed).expect("state");
    state.interactive_battles = false;
    let mut report = Report {
        turns,
        ..Report::default()
    };
    for _ in 1..=turns {
        let turn = state.turn;
        let started = Instant::now();
        let recorder = RefCell::new(Recorder::default());
        // France is the "player": it answers offers as the AI would.
        for offer in state.factions[&france].offers.clone() {
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
                Order::AnswerOffer {
                    offer: offer.id,
                    accept,
                },
            );
        }
        recorder.borrow_mut().record(&state, data, &france);
        let orders = ai::plan_turn(&state, data, &france);
        recorder.borrow_mut().record_orders(&state, data, &orders);
        for order in orders {
            let _ = state.submit_order(data, order);
        }
        let before = Before {
            sieges: state
                .settlements
                .iter()
                .filter_map(|(id, s)| {
                    let siege = s.siege.as_ref()?;
                    Some((id.clone(), (siege.attacker.clone(), siege.started_turn)))
                })
                .collect(),
            controllers: state
                .settlements
                .iter()
                .map(|(id, s)| (id.clone(), s.controller.clone()))
                .collect(),
        };
        let events = state.end_turn_with(data, |s, d, f| {
            recorder.borrow_mut().record(s, d, f);
            let orders = ai::plan_turn(s, d, f);
            recorder.borrow_mut().record_orders(s, d, &orders);
            orders
        });
        let recorder = recorder.into_inner();
        report.sim += started.elapsed().saturating_sub(recorder.spent);
        let measured = Instant::now();
        measure(&mut report, data, &state, &recorder, &events, turn, &before);
        report.probe += recorder.spent + measured.elapsed();
    }
    report
}

/// Reads the state right after the turn against what the factions saw.
fn measure(
    report: &mut Report,
    data: &GameData,
    state: &CampaignState,
    rec: &Recorder,
    events: &[GameEvent],
    turn: u32,
    before: &Before,
) {
    let playing =
        |f: &FactionId| f.as_str() != REBELS && state.factions.get(f).is_some_and(|fs| fs.alive);
    let at_real_war = |f: &FactionId| {
        state
            .factions
            .get(f)
            .is_some_and(|fs| fs.at_war_with.iter().any(|e| e.as_str() != REBELS))
    };
    let kind = |s: &SettlementId| state.settlements.get(s).map_or(9, |s| kind_rank(s.kind));
    // Idle after the turn: did not move since planning, no destination,
    // not besieging, no battle.
    let idle = |id: &ArmyId, snap: &ArmySnap| -> bool {
        let Some(army) = state.armies.get(id) else {
            return false;
        };
        let besieging = army
            .settlement()
            .and_then(|s| state.settlements.get(s))
            .and_then(|s| s.siege.as_ref())
            .is_some_and(|s| s.attacker == army.faction);
        army.destination.is_none()
            && !besieging
            && army.fought_turn != Some(turn)
            && distance(state.army_point(data, army), snap.point) < MOVED_PX
    };

    // 1 and 3: idle armies.
    for (id, snap) in &rec.armies {
        if !playing(&snap.faction) || !state.armies.contains_key(id) {
            continue;
        }
        let is_idle = idle(id, snap);
        let still_open = snap
            .capturable
            .iter()
            .filter(|s| {
                state.settlements.get(*s).is_some_and(|st| {
                    before.controllers.get(*s) == Some(&st.controller)
                        && state.is_at_war(&snap.faction, &st.controller)
                })
            })
            .min_by_key(|s| (kind(s), (*s).clone()));
        if let (true, Some(place)) = (is_idle, still_open) {
            report.missed_army_turns += 1;
            report.missed_examples.push((
                turn,
                snap.faction.clone(),
                id.clone(),
                place.clone(),
                kind(place),
                snap.note(),
            ));
        }
        if at_real_war(&snap.faction) {
            report.war_army_turns += 1;
            let stance = state.armies[id].stance;
            if is_idle
                && !snap.resting
                && !snap.guarding
                && !matches!(stance, Stance::Ambush | Stance::Entrenched)
            {
                report.idle_army_turns += 1;
            }
        }
    }

    // 2: places lost while an able army stood by.
    for (sid, seen) in &rec.threatened {
        let Some(s) = state.settlements.get(sid) else {
            continue;
        };
        if s.controller == seen.faction || !playing(&seen.faction) {
            continue;
        }
        report.losses_seen += 1;
        let target = MoveTarget::Settlement(sid.clone());
        let stood_by = seen.relievers.iter().find(|id| {
            state.armies.get(*id).is_some_and(|a| {
                !a.is_at(sid)
                    && a.destination.as_ref() != Some(&target)
                    && a.fought_turn != Some(turn)
            })
        });
        if let Some(army) = stood_by {
            report.undefended_losses += 1;
            report.undefended_examples.push((
                turn,
                seen.faction.clone(),
                army.clone(),
                sid.clone(),
                kind(sid),
                rec.armies.get(army).map_or("?", ArmySnap::note),
            ));
        }
    }

    // 4: field battles (the event's army is the attacker, its faction the
    // winner).
    for event in events.iter().filter(|e| {
        e.kind == EventKind::Battle
            && e.text_fr.starts_with("Bataille ")
            && e.text_fr.contains("Vainqueur")
    }) {
        let (Some(attacker), Some(winner)) = (&event.army, &event.faction) else {
            continue;
        };
        let Some(snap) = rec.armies.get(attacker) else {
            continue;
        };
        if snap.faction.as_str() == REBELS {
            continue;
        }
        report.battles += 1;
        if winner == &snap.faction {
            continue;
        }
        report.battles_lost += 1;
        let ordered = rec.attacks.get(attacker).copied();
        let defender = ordered.or_else(|| {
            let p = event.province.as_ref()?;
            rec.hostile_max.get(&snap.faction)?.get(p).copied()
        });
        if let Some(defender) = defender.filter(|d| snap.power < SUICIDE_RATIO * d) {
            report.suicide += 1;
            report.suicide_examples.push(format!(
                "t{turn} {} {} ({:.0} contre {:.0}, {}) : {}",
                snap.faction,
                attacker,
                snap.power,
                defender,
                if ordered.is_some() {
                    "ordre Attack"
                } else {
                    "marche"
                },
                event.text_fr
            ));
        }
    }

    // 5 and 7: per faction.
    for (fid, f) in &state.factions {
        if !playing(fid) {
            continue;
        }
        report.faction_turns += 1;
        if f.treasury < 0 {
            report.negative_treasury += 1;
        }
        if f.income_last_turn > 0 && f.treasury > HOARD_SEASONS * f.income_last_turn {
            report.hoarding += 1;
        }
        if at_real_war(fid) {
            report.war_faction_turns += 1;
            let powers: Vec<f64> = state
                .armies
                .iter()
                .filter(|(_, a)| &a.faction == fid)
                .map(|(id, _)| state.army_power(data, id))
                .collect();
            let largest = powers.iter().copied().fold(0.0, f64::max);
            report.fragments += powers
                .iter()
                .filter(|p| **p < FRAGMENT_SHARE * largest)
                .count() as u32;
        }
    }

    // 6: sieges and captures (non-rebel attackers).
    let mut sieges_after: BTreeMap<&SettlementId, (&FactionId, u32)> = BTreeMap::new();
    for (sid, s) in &state.settlements {
        let Some(siege) = &s.siege else {
            continue;
        };
        sieges_after.insert(sid, (&siege.attacker, siege.started_turn));
        if siege.attacker.as_str() != REBELS
            && before
                .sieges
                .get(sid)
                .is_none_or(|(a, t)| a != &siege.attacker || *t != siege.started_turn)
        {
            report.sieges_started += 1;
        }
    }
    for (sid, (attacker, started)) in &before.sieges {
        if attacker.as_str() == REBELS
            || sieges_after
                .get(sid)
                .is_some_and(|(a, t)| *a == attacker && t == started)
        {
            continue;
        }
        let controller = &state.settlements[sid].controller;
        if state.is_allied(attacker, controller) {
            report.sieges_captured += 1;
            report.siege_turns += state.turn.saturating_sub(*started);
        } else {
            report.sieges_abandoned += 1;
        }
    }
    for (sid, previous) in &before.controllers {
        let s = &state.settlements[sid];
        if &s.controller != previous
            && s.controller.as_str() != REBELS
            && before
                .sieges
                .get(sid)
                .is_none_or(|(a, _)| !state.is_allied(a, &s.controller))
        {
            report.direct_captures += 1;
        }
    }
}

/// Prints the examples grouped by (faction, army, place): the most
/// repeated first, then cities first.
fn print_examples(title: &str, examples: &[Example]) {
    let mut groups: BTreeMap<(&FactionId, &ArmyId, &SettlementId), (Vec<u32>, u8, &str)> =
        BTreeMap::new();
    for (turn, faction, army, place, kind, note) in examples {
        let group = groups
            .entry((faction, army, place))
            .or_insert((Vec::new(), *kind, note));
        group.0.push(*turn);
    }
    let mut groups: Vec<_> = groups.into_iter().collect();
    groups.sort_by(|a, b| (b.1 .0.len(), a.1 .1).cmp(&(a.1 .0.len(), b.1 .1)));
    println!(
        "\n{title} ({} cas, {} groupes)",
        examples.len(),
        groups.len()
    );
    for ((faction, army, place), (turns, _, note)) in groups.iter().take(EXAMPLES) {
        let first = turns.first().copied().unwrap_or(0);
        println!(
            "- {faction} {army} → {place} : {} tour(s) dès t{first} ({note})",
            turns.len()
        );
    }
}

fn main() {
    let mut args = std::env::args().skip(1);
    let turns: u32 = args.next().and_then(|s| s.parse().ok()).unwrap_or(60);
    let mut seeds: Vec<u64> = args.filter_map(|s| s.parse().ok()).collect();
    if seeds.is_empty() {
        seeds = vec![1, 2, 3, 4];
    }
    let verbose = std::env::var("VERBOSE").is_ok_and(|v| v == "1");
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../data");
    let (data, _) = GameData::load(&root).expect("data");
    let mut rows: Vec<Vec<f64>> = Vec::new();
    for seed in &seeds {
        let report = run(&data, *seed, turns);
        let row = report.row();
        println!("\n## Graine {seed} ({turns} tours)\n");
        println!("| mesure | valeur |");
        println!("|---|---|");
        for (header, value) in HEADERS.iter().zip(&row) {
            println!("| {header} | {value:.2} |");
        }
        if verbose {
            print_examples("Prises manquées", &report.missed_examples);
            print_examples("Pertes sans secours", &report.undefended_examples);
            println!("\nBatailles suicidaires :");
            for line in report.suicide_examples.iter().take(EXAMPLES) {
                println!("- {line}");
            }
        }
        rows.push(row);
    }
    println!("\n## Synthèse ({turns} tours)\n");
    println!("| graine | {} |", HEADERS.join(" | "));
    println!("|---|{}", "---|".repeat(HEADERS.len()));
    for (seed, row) in seeds.iter().zip(&rows) {
        let cells: Vec<String> = row.iter().map(|v| format!("{v:.1}")).collect();
        println!("| {seed} | {} |", cells.join(" | "));
    }
    let n = rows.len().max(1) as f64;
    let mean: Vec<String> = (0..HEADERS.len())
        .map(|i| format!("{:.1}", rows.iter().map(|r| r[i]).sum::<f64>() / n))
        .collect();
    println!("| moyenne | {} |", mean.join(" | "));
}
