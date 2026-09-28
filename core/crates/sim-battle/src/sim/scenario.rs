//! EP7: scenario of a historical battle (ADR 0035): the historical
//! deployment, the successive "battles" (waves) held back until released,
//! the regiments posted on their ground, and the changes of weather.
//!
//! Only the battle AI is constrained: a held wave receives no AI order (it
//! still fights whoever reaches it), a posted regiment is never sent farther
//! than its leash from its post. The player's own orders are never
//! filtered.

use super::BattleSim;
use crate::command::Command;
use crate::field::Weather;
use crate::historical::{MapArmies, WeatherChange};
use crate::setup::SideId;
use crate::unit::{Formation, UnitState};

/// A regiment's post: where it stands and how far the AI may take it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Post {
    pub x: f64,
    pub z: f64,
    pub facing: f64,
    pub leash: f64,
}

/// A wave of one side.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct WaveState {
    pub label: String,
    pub release_s: f64,
    pub after: Option<usize>,
    pub after_enemy: Option<usize>,
    pub released: bool,
    /// Once released, the wave's regiments go at the enemy (scripted).
    pub assault: bool,
}

/// Scenario state of a historical battle.
#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct Scenario {
    /// By unit index.
    pub posts: Vec<Option<Post>>,
    /// By unit index: its side's wave.
    pub wave_of: Vec<usize>,
    pub waves: [Vec<WaveState>; 2],
    pub weather_changes: Vec<WeatherChange>,
    pub next_change: usize,
    /// By unit index: where it deployed (assault shooters out of missiles
    /// fall back there).
    pub origin: Vec<(f64, f64)>,
}

/// A posted regiment idle this far from its post walks back to it.
const POST_RETURN: f64 = 0.5;
/// An assault wave breaks into a run this close to its target (m).
const ASSAULT_RUN: f64 = 140.0;

impl BattleSim {
    /// EP7: deploys the regiments of a historical map where the map puts
    /// them (blocks laid side by side with their real frontage), sets their
    /// formation, dismounts the men-at-arms fighting on foot, and starts the
    /// scenario (waves, posts, weather changes).
    pub fn deploy_historical(&mut self, armies: &MapArmies, weather: &[WeatherChange]) {
        let n = self.units.len();
        let mut scenario = Scenario {
            posts: vec![None; n],
            wave_of: vec![0; n],
            origin: vec![(0.0, 0.0); n],
            weather_changes: weather.to_vec(),
            ..Scenario::default()
        };
        let dismount = self
            .setup
            .orders
            .iter()
            .find(|o| o.kind == data_model::BattleOrderKind::Dismount)
            .map(|o| (o.effects.speed_max.unwrap_or(35), o.effects.armor));
        for side in SideId::BOTH {
            let army = armies.side(side);
            scenario.waves[side.index()] = if army.waves.is_empty() {
                vec![WaveState {
                    label: String::new(),
                    release_s: 0.0,
                    after: None,
                    after_enemy: None,
                    released: true,
                    assault: false,
                }]
            } else {
                army.waves
                    .iter()
                    .enumerate()
                    .map(|(k, w)| WaveState {
                        label: w.label.clone(),
                        release_s: w.release_s,
                        after: w.after,
                        after_enemy: w.after_enemy,
                        released: k == 0
                            || w.release_s <= 0.0 && w.after.is_none() && w.after_enemy.is_none(),
                        assault: w.assault,
                    })
                    .collect()
            };
            let mut setup_index = 0usize;
            for block in &army.regiments {
                let count = block.count.max(1) as usize;
                let members: Vec<usize> = (setup_index..setup_index + count)
                    .filter_map(|k| {
                        self.units
                            .iter()
                            .position(|u| u.side == side && u.setup_index == k && !u.synthetic)
                    })
                    .collect();
                setup_index += count;
                let facing = block.facing_deg.to_radians();
                for &i in &members {
                    let unit = &mut self.units[i];
                    if block.dismounted && unit.mounted {
                        let (speed, armor) = dismount.unwrap_or((35, 0));
                        unit.dismount(speed, armor);
                    }
                    if let Some(formation) = block.formation {
                        if formation != Formation::Wedge || unit.mounted {
                            unit.formation = formation;
                        }
                    }
                    if let Some(ammo) = block.ammo {
                        unit.ammo = ammo.min(unit.ammo.max(ammo));
                    }
                    unit.facing = facing;
                    // A historical army deploys whole: no regiment waits off
                    // the field (the map's cap is sized for it).
                    unit.reserve = false;
                }
                // Side by side along the front, centred on the block.
                let right = (facing.cos(), -facing.sin());
                let widths: Vec<f64> = members.iter().map(|&i| self.units[i].extent().0).collect();
                let total =
                    widths.iter().sum::<f64>() + block.gap_m * (members.len().max(1) - 1) as f64;
                let mut offset = -total * 0.5;
                for (&i, w) in members.iter().zip(widths) {
                    let lateral = offset + w * 0.5;
                    offset += w + block.gap_m;
                    let x = (block.x + right.0 * lateral).clamp(5.0, self.field.width - 5.0);
                    let z = (block.z + right.1 * lateral).clamp(5.0, self.field.depth - 5.0);
                    let unit = &mut self.units[i];
                    unit.x = x;
                    unit.z = z;
                    unit.destination = None;
                    unit.target = None;
                    unit.order_queue.clear();
                    scenario.wave_of[i] = block.wave;
                    scenario.origin[i] = (x, z);
                    if block.hold {
                        scenario.posts[i] = Some(Post {
                            x,
                            z,
                            facing,
                            leash: block.leash_m.unwrap_or(crate::historical::DEFAULT_LEASH_M),
                        });
                    }
                }
            }
        }
        self.scenario = Some(Box::new(scenario));
        self.path_cache = Default::default();
        self.obstacle_cache = Default::default();
    }

    /// EP7: replaces the opening line of the journal (the weather drawn
    /// before the map set its own).
    pub fn set_opening_line(&mut self, text: String) {
        match self.events.first_mut() {
            Some(first) if first.time == 0.0 => first.text_fr = text,
            _ => self.log(text, None),
        }
    }

    /// EP7: is regiment `index` in a wave not yet released (no AI order)?
    pub fn scenario_held(&self, index: usize) -> bool {
        self.scenario.as_ref().is_some_and(|s| {
            let unit = &self.units[index];
            let wave = s.wave_of.get(index).copied().unwrap_or(0);
            s.waves[unit.side.index()]
                .get(wave)
                .is_some_and(|w| !w.released)
        })
    }

    /// EP7: is regiment `index` in a released assault wave (it goes at the
    /// enemy on the scenario's orders, not the AI's)?
    pub fn scenario_assault(&self, index: usize) -> bool {
        self.scenario.as_ref().is_some_and(|s| {
            let unit = &self.units[index];
            let wave = s.wave_of.get(index).copied().unwrap_or(0);
            s.waves[unit.side.index()]
                .get(wave)
                .is_some_and(|w| w.released && w.assault)
        })
    }

    /// EP7: post of regiment `index` (x, z, facing, leash), if it holds its
    /// ground.
    pub fn scenario_post(&self, index: usize) -> Option<(f64, f64, f64, f64)> {
        self.scenario
            .as_ref()
            .and_then(|s| s.posts.get(index).copied().flatten())
            .map(|p| (p.x, p.z, p.facing, p.leash))
    }

    /// EP7: is this a historical battle?
    pub fn is_historical(&self) -> bool {
        self.scenario.is_some()
    }

    /// EP7: waves of `side`: `(label, released)`.
    pub fn scenario_waves(&self, side: SideId) -> Vec<(String, bool)> {
        self.scenario.as_ref().map_or_else(Vec::new, |s| {
            s.waves[side.index()]
                .iter()
                .map(|w| (w.label.clone(), w.released))
                .collect()
        })
    }

    /// EP7: an AI command as the scenario lets it through (`None`: dropped).
    pub(super) fn scenario_filter(&self, command: Command) -> Option<Command> {
        let Some(scenario) = &self.scenario else {
            return Some(command);
        };
        let index_of = |id: u32| self.units.iter().position(|u| u.id == id);
        let allowed = |id: u32, to: Option<(f64, f64)>, reach: f64| -> bool {
            let Some(i) = index_of(id) else {
                return true;
            };
            if self.scenario_held(i) || self.scenario_assault(i) {
                return false;
            }
            match (scenario.posts.get(i).copied().flatten(), to) {
                (Some(post), Some((x, z))) => (x - post.x).hypot(z - post.z) <= post.leash + reach,
                _ => true,
            }
        };
        match command {
            Command::Move {
                units,
                x,
                z,
                run,
                facing,
                queue,
                width: None,
                match_speed: false,
                group_tag: None,
            } => {
                let kept: Vec<u32> = units
                    .into_iter()
                    .filter(|&id| allowed(id, Some((x, z)), 0.0))
                    .collect();
                (!kept.is_empty()).then_some(Command::Move {
                    units: kept,
                    x,
                    z,
                    run,
                    facing,
                    queue,
                    width: None,
                    match_speed: false,
                    group_tag: None,
                })
            }
            Command::Attack {
                units,
                target,
                run,
                queue,
            } => {
                let at = index_of(target).map(|j| (self.units[j].x, self.units[j].z));
                let kept: Vec<u32> = units
                    .into_iter()
                    .filter(|&id| {
                        let reach =
                            index_of(id).map_or(0.0, |i| self.effective_range_of(i, at).max(0.0));
                        allowed(id, at, reach)
                    })
                    .collect();
                (!kept.is_empty()).then_some(Command::Attack {
                    units: kept,
                    target,
                    run,
                    queue,
                })
            }
            Command::Withdraw { units } => {
                let kept: Vec<u32> = units
                    .into_iter()
                    .filter(|&id| allowed(id, None, 0.0))
                    .collect();
                (!kept.is_empty()).then_some(Command::Withdraw { units: kept })
            }
            Command::LeaderOrder { side, order, units } if !units.is_empty() => {
                let kept: Vec<u32> = units
                    .into_iter()
                    .filter(|&id| allowed(id, None, 0.0))
                    .collect();
                (!kept.is_empty()).then_some(Command::LeaderOrder {
                    side,
                    order,
                    units: kept,
                })
            }
            // CB4: held and assaulting regiments keep to the script, as
            // under the leader's orders (the Genoese of Crécy never raised
            // their pavises by the order either).
            Command::UseAbility { units, ability } => {
                let kept: Vec<u32> = units
                    .into_iter()
                    .filter(|&id| allowed(id, None, 0.0))
                    .collect();
                (!kept.is_empty()).then_some(Command::UseAbility {
                    units: kept,
                    ability,
                })
            }
            Command::Halt { units } => {
                let kept: Vec<u32> = units
                    .into_iter()
                    .filter(|&id| allowed(id, None, 0.0))
                    .collect();
                (!kept.is_empty()).then_some(Command::Halt { units: kept })
            }
            Command::FireAtWill { units, enabled } => {
                let kept: Vec<u32> = units
                    .into_iter()
                    .filter(|&id| allowed(id, None, 0.0))
                    .collect();
                (!kept.is_empty()).then_some(Command::FireAtWill {
                    units: kept,
                    enabled,
                })
            }
            Command::Formation { units, kind } => {
                let kept: Vec<u32> = units
                    .into_iter()
                    .filter(|&id| allowed(id, None, 0.0))
                    .collect();
                (!kept.is_empty()).then_some(Command::Formation { units: kept, kind })
            }
            other => Some(other),
        }
    }

    /// Range a shooter may fire from within its leash (0 for others).
    fn effective_range_of(&self, i: usize, at: Option<(f64, f64)>) -> f64 {
        let unit = &self.units[i];
        match at {
            Some((x, z)) if unit.can_shoot() && unit.ammo > 0 => self.effective_range(unit, x, z),
            _ => 0.0,
        }
    }

    /// EP7: releases the waves due, applies the changes of weather and walks
    /// idle posted regiments back to their posts (once per AI period).
    pub(super) fn tick_scenario(&mut self) {
        let Some(mut scenario) = self.scenario.take() else {
            return;
        };
        let elapsed = self.elapsed;
        // Changes of weather.
        while let Some(change) = scenario.weather_changes.get(scenario.next_change).copied() {
            if change.at_s > elapsed {
                break;
            }
            scenario.next_change += 1;
            if change.weather != self.weather {
                self.weather = change.weather;
                let text = match change.weather {
                    Weather::Clear => {
                        "Le ciel se dégage ; le soleil revient sur le champ de bataille."
                    }
                    Weather::Rain => "Une averse s'abat sur le champ de bataille.",
                    Weather::Fog => "Le brouillard se lève sur le champ de bataille.",
                    Weather::Snow => "La neige se met à tomber.",
                };
                self.log(text.to_owned(), None);
            }
        }
        // Waves.
        for side in SideId::BOTH {
            let count = scenario.waves[side.index()].len();
            for k in 0..count {
                let wave = &scenario.waves[side.index()][k];
                if wave.released {
                    continue;
                }
                let due = wave.release_s > 0.0 && elapsed >= wave.release_s;
                let triggered = wave
                    .after
                    .is_some_and(|w| self.wave_engaged(&scenario, side, w))
                    || wave
                        .after_enemy
                        .is_some_and(|w| self.wave_engaged(&scenario, side.other(), w));
                if due || triggered {
                    let label = wave.label.clone();
                    scenario.waves[side.index()][k].released = true;
                    if !label.is_empty() {
                        let text = format!("{label} s'ébranle.");
                        self.log(text, Some(side));
                    }
                }
            }
        }
        // Assault waves go at the nearest enemy regiment, at the run once
        // close (the successive charges of Crécy).
        let mut back = Vec::new();
        for i in 0..self.units.len() {
            let unit = &self.units[i];
            if unit.side == SideId::Defender && !self.ai_enabled[1]
                || unit.side == SideId::Attacker && !self.ai_enabled[0]
            {
                continue;
            }
            let wave = scenario.wave_of.get(i).copied().unwrap_or(0);
            let assault = scenario.waves[unit.side.index()]
                .get(wave)
                .is_some_and(|w| w.released && w.assault);
            if !assault || !unit.able() || unit.state == UnitState::Melee {
                continue;
            }
            // Shooters out of missiles fall back behind the next wave
            // instead of charging (the Genoese at Crécy).
            if unit.category == data_model::UnitCategory::Ranged && unit.ammo == 0 {
                let (x, z) = scenario.origin.get(i).copied().unwrap_or((unit.x, unit.z));
                if unit.destination.is_none() && (unit.x - x).hypot(unit.z - z) > 30.0 {
                    back.push((
                        unit.side,
                        Command::Move {
                            units: vec![unit.id],
                            x,
                            z,
                            run: false,
                            facing: Some(unit.facing),
                            queue: false,
                            width: None,
                            match_speed: false,
                            group_tag: None,
                        },
                    ));
                }
                continue;
            }
            if unit
                .target
                .and_then(|t| self.units.get(t as usize))
                .is_some_and(|t| t.able())
            {
                continue;
            }
            let nearest = self
                .units
                .iter()
                .filter(|e| e.side != unit.side && e.able() && !e.synthetic)
                .map(|e| (e.id, (e.x - unit.x).hypot(e.z - unit.z)))
                .min_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)));
            if let Some((target, d)) = nearest {
                back.push((
                    unit.side,
                    Command::Attack {
                        units: vec![unit.id],
                        target,
                        run: d < ASSAULT_RUN,
                        queue: false,
                    },
                ));
            }
        }
        // Posted regiments drift back to their posts when idle.
        for (i, post) in scenario.posts.iter().enumerate() {
            let Some(post) = post else { continue };
            let unit = &self.units[i];
            if !unit.able()
                || unit.state != UnitState::Idle
                || unit.destination.is_some()
                || unit.target.is_some()
                || !self.ai_enabled[unit.side.index()]
            {
                continue;
            }
            if (unit.x - post.x).hypot(unit.z - post.z) > post.leash * POST_RETURN + 5.0 {
                back.push((
                    unit.side,
                    Command::Move {
                        units: vec![unit.id],
                        x: post.x,
                        z: post.z,
                        run: false,
                        facing: Some(post.facing),
                        queue: false,
                        width: None,
                        match_speed: false,
                        group_tag: None,
                    },
                ));
            }
        }
        self.scenario = Some(scenario);
        for (side, command) in back {
            let _ = self.apply_command(command, Some(side));
        }
    }

    /// Half of wave `wave` of `side` in a melee, routing or gone?
    fn wave_engaged(&self, scenario: &Scenario, side: SideId, wave: usize) -> bool {
        let members: Vec<usize> = (0..self.units.len())
            .filter(|&i| {
                self.units[i].side == side
                    && !self.units[i].synthetic
                    && scenario.wave_of.get(i).copied() == Some(wave)
            })
            .collect();
        if members.is_empty() {
            return true;
        }
        let engaged = members
            .iter()
            .filter(|&&i| {
                let u = &self.units[i];
                !u.able() || u.state == UnitState::Melee
            })
            .count();
        engaged * 2 >= members.len()
    }
}
