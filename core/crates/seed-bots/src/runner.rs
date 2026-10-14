//! Plays the games: for each (seed, bot), plan → submit → oracles → end of turn → oracles.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use data_model::{FactionId, GameData};
use sim_campaign::agents::derived_rng;
use sim_campaign::{CampaignState, Order};

use crate::bands;
use crate::bot::Bot;
use crate::bots;
use crate::candidates::legal_candidates;
use crate::metrics::{self, BotMetrics, GameMetrics, MetricsTracker};
use crate::oracles::panic::guarded;
use crate::oracles::{balance, invariants, refused, save_roundtrip};
use crate::oracles::{Finding, OracleContext, Origin};
use crate::view::BotView;

/// Factions played, by seed: seed `k` plays `PLAYABLE[k % 3]`, so any three
/// consecutive seeds cover the three great realms.
pub const PLAYABLE: [&str; 3] = ["fac_france", "fac_england", "fac_burgundy"];
/// A save round trip every this many turns (and at the end of the game).
const SAVE_EVERY: u32 = 10;
/// Fixed salt of the bots' random generators.
const BOT_SALT: u32 = 0xB07;

/// Settings of a run.
#[derive(Debug, Clone)]
pub struct RunConfig {
    /// Seeds to play.
    pub seeds: Vec<u64>,
    /// Turns per game.
    pub turns: u32,
    /// Bot names (subset of [`crate::bots::ALL`]).
    pub bots: Vec<String>,
    /// Hard cap in minutes; past it the run stops with a partial report.
    pub max_minutes: u32,
    /// Slow-turn limit, milliseconds.
    pub slow_turn_ms: u64,
    /// Whether to check balance drift (long pass only).
    pub balance: bool,
    /// The `data/` directory.
    pub data_root: PathBuf,
}

/// Wall time spent in each phase, summed over every game and thread.
#[derive(Debug, Clone, Copy, Default)]
pub struct Timings {
    /// `Bot::plan`.
    pub plan: Duration,
    /// `legal_candidates`, recomputed before each order to verify it.
    pub candidates: Duration,
    /// `submit_order`.
    pub submit: Duration,
    /// `end_turn_with(ai::plan_turn)`.
    pub end_turn: Duration,
    /// Invariant oracle.
    pub invariants: Duration,
    /// Save round trip oracle.
    pub save: Duration,
    /// Turns played.
    pub turns: u64,
}

impl Timings {
    fn add(&mut self, other: &Timings) {
        self.plan += other.plan;
        self.candidates += other.candidates;
        self.submit += other.submit;
        self.end_turn += other.end_turn;
        self.invariants += other.invariants;
        self.save += other.save;
        self.turns += other.turns;
    }
}

/// Result of a run.
#[derive(Debug, Clone, Default)]
pub struct RunResult {
    /// Every finding, before deduplication.
    pub findings: Vec<Finding>,
    /// Whether the hard cap stopped the run early.
    pub partial: bool,
    /// Games played.
    pub games: u32,
    /// Wall time, seconds.
    pub seconds: f64,
    /// Time per phase.
    pub timings: Timings,
    /// Balance metrics per bot (only with `config.balance`), bots in name order.
    pub metrics: Vec<BotMetrics>,
}

/// Where the orders of a turn come from.
enum Source<'a> {
    /// A bot plans them.
    Bot(&'a mut dyn Bot),
    /// A recorded log is submitted again (replay).
    Log(&'a [Vec<Order>]),
}

/// Test hook: may corrupt the state after the end of a turn (to provoke a finding).
type Hook<'a> = Option<&'a (dyn Fn(&mut CampaignState, u32) + Sync)>;

/// One game to play.
struct Game<'a> {
    data: &'a GameData,
    seed: u64,
    bot_name: &'a str,
    bot_index: u32,
    faction: FactionId,
    turns: u32,
    slow_turn_ms: u64,
    /// Collect the balance metrics.
    balance: bool,
}

/// What a played game leaves.
struct Played {
    findings: Vec<Finding>,
    timings: Timings,
    /// The deadline stopped the game early.
    cut: bool,
    /// Metrics of a game played to its end (`Game::balance` only).
    metrics: Option<GameMetrics>,
}

/// The faction played with `seed`.
fn faction_for(seed: u64) -> FactionId {
    FactionId::new(PLAYABLE[(seed % PLAYABLE.len() as u64) as usize]).expect("faction id")
}

/// Plays one game. With `stop_after = Some(t)`, stops once turn `t` is done.
fn play(
    game: &Game<'_>,
    mut source: Source<'_>,
    deadline: Option<Instant>,
    stop_after: Option<u32>,
    hook: Hook<'_>,
) -> Played {
    let mut played = Played {
        findings: Vec::new(),
        timings: Timings::default(),
        cut: false,
        metrics: None,
    };
    let Ok(mut state) = CampaignState::new_1337(game.data, game.faction.clone(), game.seed) else {
        return played;
    };
    let mut tracker = game.balance.then(|| MetricsTracker::new(&state));
    let mut order_log: Vec<Vec<Order>> = Vec::new();
    let origin = |turn: u32, log: &[Vec<Order>]| Origin {
        seed: game.seed,
        bot: game.bot_name.to_owned(),
        faction: game.faction.to_string(),
        turn,
        order_log: log.to_vec(),
    };
    for played_turns in 0..game.turns {
        if deadline.is_some_and(|d| Instant::now() >= d) {
            played.cut = true;
            break;
        }
        let turn = state.turn();
        order_log.push(Vec::new());
        played.timings.turns += 1;
        let mut found: Vec<Finding> = Vec::new();

        // Plan and submit, every order verified against the candidates of the moment.
        let planned_origin = origin(turn, &order_log);
        let mut timings = Timings::default();
        // Kept outside the guard: after a panic while submitting, the log still holds
        // every order submitted so far, the faulty one included, so `replay` reproduces it.
        let mut submitted: Vec<Order> = Vec::new();
        let (outcome, mut panics) = guarded(&planned_origin, u64::MAX, || {
            let started = Instant::now();
            let orders = match &mut source {
                Source::Bot(bot) => {
                    let view = BotView {
                        state: &state,
                        faction: &game.faction,
                    };
                    let mut rng = derived_rng(game.seed, turn, game.bot_index, BOT_SALT);
                    bot.plan(&view, game.data, &mut rng)
                }
                Source::Log(log) => log.get(played_turns as usize).cloned().unwrap_or_default(),
            };
            timings.plan += started.elapsed();
            let mut refusals: Vec<(Order, sim_campaign::OrderError)> = Vec::new();
            for order in orders {
                let started = Instant::now();
                let verified = legal_candidates(
                    &BotView {
                        state: &state,
                        faction: &game.faction,
                    },
                    game.data,
                )
                .iter()
                .any(|c| c.order == order);
                timings.candidates += started.elapsed();
                submitted.push(order.clone());
                let started = Instant::now();
                let result = state.submit_order(game.data, order.clone());
                timings.submit += started.elapsed();
                if let (true, Err(error)) = (verified, result) {
                    refusals.push((order, error));
                }
            }
            refusals
        });
        played.timings.add(&timings);
        order_log[played_turns as usize] = submitted;
        for finding in &mut panics {
            finding.order_log = order_log.clone();
        }
        found.extend(panics);
        let Some(refusals) = outcome else {
            // The state is not reliable after a panic: the game stops here.
            played.findings.extend(found);
            return played;
        };
        let with_orders = origin(turn, &order_log);
        for (order, error) in refusals {
            found.extend(refused::check(&with_orders, &order, &Err(error)));
        }

        // End of turn.
        let started = Instant::now();
        let (ended, step_findings) = guarded(&with_orders, game.slow_turn_ms, || {
            state.end_turn_with(game.data, ai::plan_turn)
        });
        played.timings.end_turn += started.elapsed();
        found.extend(step_findings);
        let Some(events) = ended else {
            played.findings.extend(found);
            return played;
        };
        if let Some(tracker) = tracker.as_mut() {
            tracker.observe(&state, &events);
        }
        if let Some(hook) = hook {
            hook(&mut state, turn);
        }

        let context = OracleContext {
            state: &state,
            data: game.data,
            faction: &game.faction,
            turn,
        };
        let started = Instant::now();
        found.extend(invariants::check(&context, &with_orders));
        played.timings.invariants += started.elapsed();

        let last = played_turns + 1 == game.turns || state.outcome.is_some();
        if (played_turns + 1) % SAVE_EVERY == 0 || last {
            let started = Instant::now();
            found.extend(save_roundtrip::check(&state, &with_orders));
            played.timings.save += started.elapsed();
        }
        played.findings.extend(found);
        if state.outcome.is_some() || stop_after == Some(turn) {
            break;
        }
    }
    if !played.cut {
        played.metrics = tracker.map(|tracker| tracker.finish(&state));
    }
    played
}

/// What one bot's thread brings back.
#[derive(Default)]
struct BotRun {
    findings: Vec<Finding>,
    timings: Timings,
    games: u32,
    cut: bool,
    metrics: Vec<(String, GameMetrics)>,
}

/// Plays every (seed, bot) game, one thread per bot.
pub fn run(config: &RunConfig) -> RunResult {
    run_with_hook(config, None)
}

fn run_with_hook(config: &RunConfig, hook: Hook<'_>) -> RunResult {
    let started = Instant::now();
    let deadline = started + Duration::from_secs(u64::from(config.max_minutes) * 60);
    let data = match GameData::load(&config.data_root) {
        Ok((data, _)) => data,
        Err(error) => {
            eprintln!("données illisibles : {error:?}");
            return RunResult::default();
        }
    };
    let mut names: Vec<(u32, &str)> = Vec::new();
    for name in &config.bots {
        match bots::ALL.iter().position(|known| known == name) {
            Some(index) if bots::make(name).is_some() => names.push((index as u32, name)),
            _ => eprintln!("bot {name} ignoré : pas encore écrit ou inconnu"),
        }
    }
    let per_bot: Vec<BotRun> = std::thread::scope(|scope| {
        let handles: Vec<_> = names
            .iter()
            .map(|&(bot_index, name)| {
                let data = &data;
                scope.spawn(move || {
                    let mut result = BotRun::default();
                    for &seed in &config.seeds {
                        let mut bot = bots::make(name).expect("bot checked above");
                        let game = Game {
                            data,
                            seed,
                            bot_name: name,
                            bot_index,
                            faction: faction_for(seed),
                            turns: config.turns,
                            slow_turn_ms: config.slow_turn_ms,
                            balance: config.balance,
                        };
                        let played =
                            play(&game, Source::Bot(bot.as_mut()), Some(deadline), None, hook);
                        result.findings.extend(played.findings);
                        result.timings.add(&played.timings);
                        result.games += 1;
                        if let Some(game_metrics) = played.metrics {
                            result.metrics.push((name.to_owned(), game_metrics));
                        }
                        if played.cut {
                            result.cut = true;
                            break;
                        }
                    }
                    result
                })
            })
            .collect();
        handles
            .into_iter()
            .map(|handle| handle.join().expect("bot thread"))
            .collect()
    });
    let mut result = RunResult::default();
    let mut played_metrics: Vec<(String, GameMetrics)> = Vec::new();
    for run in per_bot {
        result.findings.extend(run.findings);
        result.timings.add(&run.timings);
        result.games += run.games;
        result.partial |= run.cut;
        played_metrics.extend(run.metrics);
    }
    if config.balance {
        result.metrics = metrics::aggregate(&played_metrics);
        match bands::load(&config.data_root) {
            Ok(loaded) => result
                .findings
                .extend(balance::check(&result.metrics, &loaded)),
            Err(error) => eprintln!("{error}"),
        }
    }
    result.seconds = started.elapsed().as_secs_f64();
    result
}

/// Plays a finding's game again and stops at its turn; returns the findings seen then.
pub fn replay(finding: &Finding, data_root: &Path) -> Vec<Finding> {
    replay_with_hook(finding, data_root, None)
}

fn replay_with_hook(finding: &Finding, data_root: &Path, hook: Hook<'_>) -> Vec<Finding> {
    let Ok((data, _)) = GameData::load(data_root) else {
        eprintln!("données illisibles : {}", data_root.display());
        return Vec::new();
    };
    let Some(bot_index) = bots::ALL.iter().position(|b| *b == finding.bot) else {
        eprintln!("bot inconnu : {}", finding.bot);
        return Vec::new();
    };
    let Ok(faction) = FactionId::new(finding.faction.clone()) else {
        eprintln!("faction invalide : {}", finding.faction);
        return Vec::new();
    };
    let game = Game {
        data: &data,
        seed: finding.seed,
        bot_name: &finding.bot,
        bot_index: bot_index as u32,
        faction,
        turns: u32::MAX,
        slow_turn_ms: u64::MAX,
        balance: false,
    };
    let played = play(
        &game,
        Source::Log(&finding.order_log),
        None,
        Some(finding.turn),
        hook,
    );
    played
        .findings
        .into_iter()
        .filter(|f| f.turn == finding.turn)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::oracles::OracleKind;
    use crate::report;

    fn data_root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../data")
    }

    fn config(bots: &[&str], turns: u32) -> RunConfig {
        RunConfig {
            seeds: vec![1],
            turns,
            bots: bots.iter().map(|b| b.to_string()).collect(),
            max_minutes: 10,
            slow_turn_ms: 600_000,
            balance: false,
            data_root: data_root(),
        }
    }

    /// Corrupts the state after turn `at`: an army of a faction that does not exist.
    fn corrupt_at(at: u32) -> impl Fn(&mut CampaignState, u32) + Sync {
        move |state, turn| {
            if turn == at {
                let army = sim_campaign::Army::new(
                    FactionId::new("fac_nowhere").unwrap(),
                    sim_campaign::ArmyPosition::field([1.0, 2.0]),
                    Vec::new(),
                );
                state
                    .armies
                    .insert(sim_campaign::ArmyId::from_index(999_999), army);
            }
        }
    }

    #[test]
    fn seeds_cover_the_three_factions() {
        let names: Vec<String> = (1..=3).map(|s| faction_for(s).to_string()).collect();
        for faction in PLAYABLE {
            assert!(names.iter().any(|n| n == faction), "{faction}");
        }
    }

    /// Slow (a few seconds): two quick runs give the same findings and the same report.json.
    #[test]
    #[ignore = "plays 2 games x 5 turns twice; run with --ignored"]
    fn two_runs_are_identical() {
        let config = config(&["random-legal", "mirror"], 5);
        let (a, b) = (run(&config), run(&config));
        assert_eq!(a.games, 2);
        assert_eq!(a.findings, b.findings);
        let dir = std::env::temp_dir().join(format!("seed-bots-det-{}", std::process::id()));
        let (da, db) = (dir.join("a"), dir.join("b"));
        report::write(&a, &da).unwrap();
        report::write(&b, &db).unwrap();
        assert_eq!(
            std::fs::read_to_string(da.join("report.json")).unwrap(),
            std::fs::read_to_string(db.join("report.json")).unwrap()
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Slow: a violation injected after turn 3 is found at turn 3, and replay finds it again.
    #[test]
    #[ignore = "plays 5 turns then replays 4; run with --ignored"]
    fn injected_violation_is_replayed_at_its_turn() {
        let hook = corrupt_at(3);
        let hook_ref: &(dyn Fn(&mut CampaignState, u32) + Sync) = &hook;
        let cfg = config(&["random-legal"], 5);
        let result = run_with_hook(&cfg, Some(hook_ref));
        let finding = result
            .findings
            .iter()
            .find(|f| f.oracle == OracleKind::Invariant)
            .expect("invariant finding");
        assert_eq!(finding.turn, 3);
        let again = replay_with_hook(finding, &data_root(), Some(hook_ref));
        assert!(
            again
                .iter()
                .any(|f| f.oracle == OracleKind::Invariant && f.turn == 3),
            "{again:?}"
        );
        assert!(again.iter().all(|f| f.turn == 3));
    }

    #[test]
    fn unknown_or_unwritten_bots_are_skipped() {
        let mut cfg = config(&["nope"], 1);
        cfg.seeds = vec![1];
        let result = run(&cfg);
        assert_eq!(result.games, 0);
        assert!(result.findings.is_empty());
    }
}
