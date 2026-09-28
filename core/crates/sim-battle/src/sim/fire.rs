//! Siege fires in the battle tick (lot S2, spec `docs/design/s2-incendies.md`).
//!
//! - Ignition: every volley of a besieging regiment may set the house (or
//!   the gate) nearest its point of impact on fire ([`BattleSim::incendiary_volley`]);
//!   the `burn` command puts a torch to a house or the gate.
//! - Each burning house grows, then declines and burns out into a ruin that
//!   no longer blocks movement or pathing; the gate loses HP while it burns.
//! - Spread every `spread.period_s` seconds to the houses (and the gate)
//!   whose edge is close enough, favoured by the wind, slowed by rain/snow.
//! - Heat wounds and shakes the regiments close to a fire; smoke spoils the
//!   aim of the shots that cross it.
//!
//! Every draw comes from a dedicated stream seeded from the battle seed, so
//! the same seed and commands always give the same fire, and the fire never
//! shifts the draws of the other rules.

use std::sync::Arc;

use data_model::UnitCategory;

use super::{BattleSim, DT};
use crate::command::CommandError;
use crate::fire::{Blaze, BurnChoice, FireRules};
use crate::rng::BattleRng;
use crate::setup::SideId;
use crate::siege::{House, SiegeWorks};
use crate::unit::{Unit, UnitState};

/// Salt of the fire's random stream.
const FIRE_SALT: u64 = 0x0F1E_5EED_0000_5202;

/// Fire rules and random stream of a battle (derived from the seed).
#[derive(Debug, Clone)]
pub(crate) struct FireSystem {
    rules: Option<Arc<FireRules>>,
    rng: BattleRng,
    /// The first step ran (the garrison's decision on the suburbs).
    started: bool,
    /// The first fire has been announced in the journal.
    announced: bool,
}

impl FireSystem {
    /// The current rules ([`FireRules::current`]: the data folder's once
    /// loaded) for a siege battle, none for a field battle.
    pub(crate) fn new(seed: u64, siege: bool) -> Self {
        FireSystem {
            rules: siege.then(FireRules::current),
            rng: BattleRng::from_seed(seed ^ FIRE_SALT),
            started: false,
            announced: false,
        }
    }

    /// Draws the wind and lays out the suburbs (called once the walls exist).
    pub(crate) fn prepare(&mut self, works: &mut SiegeWorks) {
        let Some(rules) = self.rules.clone() else {
            return;
        };
        let angle = self.rng.range(0.0, std::f64::consts::TAU);
        let strength = self.rng.range(0.0, rules.spread.wind_strength_max.max(0.0));
        works.wind = (angle.sin() * strength, angle.cos() * strength);
        add_suburbs(works, &rules);
    }
}

fn category_key(category: UnitCategory) -> &'static str {
    match category {
        UnitCategory::Infantry => "infantry",
        UnitCategory::Ranged => "ranged",
        UnitCategory::Cavalry => "cavalry",
        UnitCategory::Siege => "siege",
    }
}

/// Suburb houses outside the side walls (east and west), off the axis of
/// the assault (spec § 2.2): shared round-robin between the two sides,
/// `spacing_m` apart along each wall.
fn add_suburbs(works: &mut SiegeWorks, rules: &FireRules) {
    let s = &rules.suburbs;
    let sides: Vec<usize> = (0..works.pieces.len())
        .filter(|&p| {
            works.pieces[p].kind == crate::siege::PieceKind::Wall
                && works.pieces[p].outward().0.abs() > 0.9
        })
        .collect();
    if sides.is_empty() {
        return;
    }
    for k in 0..s.count as usize {
        let piece = &works.pieces[sides[k % sides.len()]];
        let j = k / sides.len();
        let sign = if j.is_multiple_of(2) { -1.0 } else { 1.0 };
        let along = sign * ((j / 2) as f64 + 0.5) * s.spacing_m;
        let (mx, mz) = piece.midpoint();
        let (nx, nz) = piece.outward();
        let (tx, tz) = piece.tangent();
        // BR3: a rural house along the wall, its rectangle for the figures.
        let town = &crate::town::TownRules::bundled().suburb;
        let mut house = House::block(
            mx + nx * s.distance_m + tx * along,
            mz + nz * s.distance_m + tz * along,
            s.radius_m * town.frontage_factor,
            s.radius_m * town.depth_factor,
            tz.atan2(tx),
        );
        house.suburb = true;
        house.radius = s.radius_m;
        house.rows = 1;
        works.houses.push(house);
    }
}

/// Distance from (px, pz) to the segment `a`-`b`.
fn segment_distance(a: (f64, f64), b: (f64, f64), px: f64, pz: f64) -> f64 {
    let (dx, dz) = (b.0 - a.0, b.1 - a.1);
    let len2 = (dx * dx + dz * dz).max(1e-9);
    let t = (((px - a.0) * dx + (pz - a.1) * dz) / len2).clamp(0.0, 1.0);
    ((a.0 + dx * t - px).powi(2) + (a.1 + dz * t - pz).powi(2)).sqrt()
}

/// What a fire may catch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Fuel {
    House(usize),
    Gate,
}

impl BattleSim {
    // ----- public API -------------------------------------------------------

    /// Fire rules of this battle (`None`: no fire, e.g. a field battle).
    pub fn fire_rules(&self) -> Option<&FireRules> {
        self.fire.rules.as_deref()
    }

    /// Replaces the fire rules (tests, probe); `None` disables fires. The
    /// suburbs and the wind drawn at the start are kept.
    pub fn set_fire_rules(&mut self, rules: Option<FireRules>) {
        self.fire.rules = rules.map(Arc::new);
    }

    /// Sets house `index` on fire (initial intensity of the rules); `false`
    /// when there is no such house, no fire rules, or it already burns.
    pub fn ignite_house(&mut self, index: usize) -> bool {
        let Some(intensity) = self.fire_rules().map(|r| r.house.initial_intensity) else {
            return false;
        };
        self.ignite(Fuel::House(index), intensity)
    }

    /// Sets the gate on fire; `false` when it is already burning or burnt.
    pub fn ignite_gate(&mut self) -> bool {
        let Some(intensity) = self.fire_rules().map(|r| r.gate.initial_intensity) else {
            return false;
        };
        self.ignite(Fuel::Gate, intensity)
    }

    // ----- tick ---------------------------------------------------------------

    /// One step of the fires: the garrison's suburbs, burning, spread, heat.
    pub(super) fn resolve_fire(&mut self) {
        let Some(rules) = self.fire.rules.clone() else {
            return;
        };
        if self.siege.is_none() {
            return;
        }
        let weather = rules.weather(self.weather);
        if !self.fire.started {
            self.fire.started = true;
            self.burn_suburbs(&rules);
        }
        let mut logs: Vec<(String, Option<SideId>)> = Vec::new();
        {
            let works = self.siege.as_mut().expect("siege");
            for house in works.houses.iter_mut() {
                house.fire.advance(&rules.house, &weather, DT);
            }
            if works.gate_fire.burning() {
                works.gate_fire.advance(&rules.gate, &weather, DT);
                let gate = works.gate;
                let piece = &mut works.pieces[gate];
                if piece.intact() {
                    piece.mark_attacked(crate::siege::UNDER_ATTACK_CONTACT_S);
                    piece.hp -= rules.gate.damage_per_s * works.gate_fire.intensity * DT;
                    if piece.hp <= 0.0 {
                        piece.hp = 0.0;
                        logs.push((
                            "La porte, dévorée par les flammes, s'effondre !".to_owned(),
                            Some(SideId::Attacker),
                        ));
                    }
                }
            }
        }
        let period = (rules.spread.period_s / DT).round().max(1.0) as u64;
        if self.ticks.is_multiple_of(period) {
            self.spread_fire(&rules, weather.spread);
        }
        self.fire_heat(&rules);
        for (text, side) in logs {
            self.log(text, side);
        }
    }

    /// An AI garrison burns its suburbs on the first step (each house drawn
    /// separately, weather permitting).
    fn burn_suburbs(&mut self, rules: &FireRules) {
        if !self.ai_enabled[SideId::Defender.index()] {
            return;
        }
        let suburbs: Vec<usize> = self
            .siege
            .as_ref()
            .map(|w| {
                (0..w.houses.len())
                    .filter(|&i| w.houses[i].suburb)
                    .collect()
            })
            .unwrap_or_default();
        if suburbs.is_empty() {
            return;
        }
        let chance = rules.suburbs.ai_burn_chance * rules.weather(self.weather).ignition;
        let mut burnt = 0;
        for i in suburbs {
            if self.fire.rng.unit() < chance
                && self.ignite(Fuel::House(i), rules.torch.initial_intensity)
            {
                burnt += 1;
            }
        }
        if burnt > 0 {
            self.log(
                "La garnison met le feu aux faubourgs pour dégager les abords des murailles."
                    .to_owned(),
                Some(SideId::Defender),
            );
        }
    }

    /// Spread from every burning house to its neighbours and the gate.
    fn spread_fire(&mut self, rules: &FireRules, weather_spread: f64) {
        let Some(works) = self.siege.as_ref() else {
            return;
        };
        let reach = rules.spread.edge_distance_m.max(1e-6);
        let wind = works.wind;
        let gate = &works.pieces[works.gate];
        let gate_open = gate.intact() && works.gate_fire == Blaze::default();
        let wind_factor = |from: (f64, f64), to: (f64, f64)| {
            let (dx, dz) = (to.0 - from.0, to.1 - from.1);
            let len = (dx * dx + dz * dz).sqrt().max(1e-9);
            (1.0 + (wind.0 * dx + wind.1 * dz) / len).max(0.0)
        };
        // (target, chance) in a fixed order: source index, then target index.
        let mut draws: Vec<(Fuel, f64)> = Vec::new();
        for (b, source) in works.houses.iter().enumerate() {
            if !source.fire.burning() {
                continue;
            }
            let base = rules.spread.chance_per_period * source.fire.intensity * weather_spread;
            for (n, target) in works.houses.iter().enumerate() {
                if n == b || target.fire != Blaze::default() {
                    continue;
                }
                // BR3: between the blocks' rectangles.
                let gap = source.gap_to(target);
                if gap > reach {
                    continue;
                }
                let chance = base
                    * (1.0 - gap.max(0.0) / reach)
                    * wind_factor((source.x, source.z), (target.x, target.z));
                draws.push((Fuel::House(n), chance));
            }
            if gate_open {
                let gap = if source.has_footprint() {
                    source.footprint().distance_to_segment(gate.a, gate.b)
                } else {
                    gate.distance(source.x, source.z) - source.radius
                };
                if gap <= reach {
                    let chance = base
                        * (1.0 - gap.max(0.0) / reach)
                        * wind_factor((source.x, source.z), gate.midpoint());
                    draws.push((Fuel::Gate, chance));
                }
            }
        }
        let mut caught: Vec<Fuel> = Vec::new();
        for (fuel, chance) in draws {
            if chance > 0.0 && self.fire.rng.unit() < chance && !caught.contains(&fuel) {
                caught.push(fuel);
            }
        }
        for fuel in caught {
            let intensity = match fuel {
                Fuel::House(_) => rules.house.initial_intensity,
                Fuel::Gate => rules.gate.initial_intensity,
            };
            self.ignite(fuel, intensity);
        }
    }

    /// Heat on `unit`: the intensity of the hottest fire (burning house or
    /// gate) within `heat.radius_m`, 0 when none; BR3b: only
    /// `heat.wall_walk_factor` of it on the wall walk.
    pub fn heat_intensity(&self, unit: &Unit) -> f64 {
        match (self.fire.rules.as_deref(), self.siege.as_ref()) {
            (Some(rules), Some(works)) => heat_on(works, rules, unit),
            _ => 0.0,
        }
    }

    /// Heat: losses and morale for the regiments close to a fire.
    fn fire_heat(&mut self, rules: &FireRules) {
        let Some(works) = self.siege.as_ref() else {
            return;
        };
        let hurt: Vec<(usize, f64)> = self
            .units
            .iter()
            .enumerate()
            .filter(|(_, unit)| unit.present())
            .map(|(i, unit)| (i, heat_on(works, rules, unit)))
            .filter(|&(_, intensity)| intensity > 0.0)
            .collect();
        for (i, intensity) in hurt {
            let unit = &mut self.units[i];
            let loss = (unit.hp * rules.heat.loss_per_s * intensity * DT).min(unit.hp);
            unit.hp -= loss;
            unit.tick_losses += loss;
            if loss > 0.0 {
                unit.loss_cause = crate::impact::LossCause::Fire;
                unit.loss_by = None;
            }
            unit.morale = (unit.morale - rules.heat.morale_per_s * intensity * DT).max(0.0);
            if unit.hp <= 0.0 {
                self.unit_destroyed(i);
            }
        }
    }

    /// Sets `fuel` on fire and announces the first fire of the battle.
    fn ignite(&mut self, fuel: Fuel, intensity: f64) -> bool {
        let Some(works) = self.siege.as_mut() else {
            return false;
        };
        let lit = match fuel {
            Fuel::House(i) => match works.houses.get_mut(i) {
                Some(house) => house.fire.ignite(intensity),
                None => false,
            },
            Fuel::Gate => works.pieces[works.gate].intact() && works.gate_fire.ignite(intensity),
        };
        if !lit {
            return false;
        }
        let suburb = matches!(fuel, Fuel::House(i) if works.houses[i].suburb);
        if fuel == Fuel::Gate {
            self.log("La porte de la ville prend feu !".to_owned(), None);
        } else if !self.fire.announced && !suburb {
            self.fire.announced = true;
            self.log("Le feu prend dans la ville !".to_owned(), None);
        }
        true
    }

    // ----- hooks of the shooting rules ----------------------------------------

    /// A besieger's volley aimed at `aim` may set the house (or the gate)
    /// nearest its point of impact on fire (spec § 2.2).
    /// The regiment looses fire arrows or incendiary stones (BV1, rendering
    /// only): a siege attacker whose type can set fires (`ignition` rules).
    pub(super) fn shoots_fire(&self, shooter: usize) -> bool {
        let Some(rules) = self.fire.rules.as_ref() else {
            return false;
        };
        let unit = &self.units[shooter];
        self.siege.is_some()
            && unit.side == SideId::Attacker
            && rules.ignition_chance(&unit.unit_type, category_key(unit.category)) > 0.0
    }

    pub(super) fn incendiary_volley(&mut self, shooter: usize, aim: (f64, f64)) {
        let Some(rules) = self.fire.rules.clone() else {
            return;
        };
        let unit = &self.units[shooter];
        if unit.side != SideId::Attacker {
            return;
        }
        let chance = rules.ignition_chance(&unit.unit_type, category_key(unit.category))
            * rules.weather(self.weather).ignition;
        if chance <= 0.0 {
            return;
        }
        let Some(works) = self.siege.as_ref() else {
            return;
        };
        let (dx, dz) = (aim.0 - unit.x, aim.1 - unit.z);
        let len = (dx * dx + dz * dz).sqrt().max(1e-9);
        let over = rules.ignition.overshoot_m;
        let (px, pz) = (aim.0 + dx / len * over, aim.1 + dz / len * over);
        let mut best: Option<(Fuel, f64)> = None;
        for (i, house) in works.houses.iter().enumerate() {
            let d = house.edge_distance(px, pz);
            if house.fire == Blaze::default()
                && d <= rules.ignition.reach_m
                && best.is_none_or(|(_, bd)| d < bd)
            {
                best = Some((Fuel::House(i), d));
            }
        }
        let gate = &works.pieces[works.gate];
        let d = gate.distance(px, pz);
        if gate.intact()
            && works.gate_fire == Blaze::default()
            && d <= rules.ignition.gate_reach_m
            && best.is_none_or(|(_, bd)| d < bd)
        {
            best = Some((Fuel::Gate, d));
        }
        let Some((fuel, _)) = best else {
            return;
        };
        if self.fire.rng.unit() < chance {
            let intensity = match fuel {
                Fuel::House(_) => rules.house.initial_intensity,
                Fuel::Gate => rules.gate.initial_intensity,
            };
            self.ignite(fuel, intensity);
        }
    }

    /// Multiplier on a shot from `shooter` at `target` crossing the smoke of
    /// a burning house (engines lob over it).
    pub(super) fn smoke_factor(&self, shooter: &Unit, target: &Unit) -> f64 {
        let (Some(rules), Some(works)) = (self.fire_rules(), self.siege.as_ref()) else {
            return 1.0;
        };
        if shooter.category == UnitCategory::Siege {
            return 1.0;
        }
        let smoky = works.houses.iter().any(|h| {
            h.fire.burning()
                && h.fire.intensity >= rules.smoke.min_intensity
                && segment_distance((shooter.x, shooter.z), (target.x, target.z), h.x, h.z)
                    < h.radius + rules.smoke.margin_m
        });
        if smoky {
            rules.smoke.accuracy_factor
        } else {
            1.0
        }
    }

    // ----- the `burn` command -------------------------------------------------

    /// RS-F: the target the « Incendier » order would give `units` of
    /// `side` (every regiment of the side when empty): the nearest house or
    /// gate that is not on fire and within reach of one of them, by the very
    /// test of the `burn` command. Draws nothing; the error says why the
    /// order is impossible.
    pub fn burn_choice(&self, side: SideId, units: &[u32]) -> Result<BurnChoice, CommandError> {
        if self.finished {
            return Err(CommandError::Finished);
        }
        if self.deploying {
            return Err(CommandError::Deploying);
        }
        let (Some(rules), Some(works)) = (self.fire_rules(), self.siege.as_ref()) else {
            return Err(CommandError::NotASiege);
        };
        let ready =
            |unit: &&Unit| unit.side == side && unit.present() && unit.state != UnitState::Routing;
        let candidates: Vec<&Unit> = if units.is_empty() {
            self.units.iter().filter(ready).collect()
        } else {
            units
                .iter()
                .filter_map(|&id| self.units.get(id as usize))
                .filter(ready)
                .collect()
        };
        if candidates.is_empty() {
            return Err(CommandError::NoUnits);
        }
        let mut fuels: Vec<Fuel> = works
            .houses
            .iter()
            .enumerate()
            .filter(|(_, h)| h.fire == Blaze::default())
            .map(|(i, _)| Fuel::House(i))
            .collect();
        if works.pieces[works.gate].intact() && works.gate_fire == Blaze::default() {
            fuels.push(Fuel::Gate);
        }
        if fuels.is_empty() {
            return Err(CommandError::NothingLeftToBurn);
        }
        let mut best: Option<BurnChoice> = None;
        for &fuel in &fuels {
            for unit in &candidates {
                let Some(distance) = torch_distance(works, rules, unit, fuel) else {
                    continue;
                };
                if best.is_some_and(|b| b.distance_m <= distance) {
                    continue;
                }
                best = Some(match fuel {
                    Fuel::House(i) => BurnChoice {
                        unit: unit.id,
                        house: Some(i),
                        gate: false,
                        suburb: works.houses[i].suburb,
                        distance_m: distance,
                    },
                    Fuel::Gate => BurnChoice {
                        unit: unit.id,
                        house: None,
                        gate: true,
                        suburb: false,
                        distance_m: distance,
                    },
                });
            }
        }
        best.ok_or(CommandError::NothingInReach)
    }

    /// `burn`: a regiment close enough puts a torch to house `house` (or the
    /// gate); the garrison reaches its suburbs from anywhere.
    pub(super) fn command_burn(
        &mut self,
        units: &[u32],
        house: Option<usize>,
        gate: bool,
    ) -> Result<(), CommandError> {
        let Some(rules) = self.fire.rules.clone() else {
            return Err(CommandError::NotASiege);
        };
        let Some(works) = self.siege.as_ref() else {
            return Err(CommandError::NotASiege);
        };
        let fuel = match (house, gate) {
            (Some(i), false) => {
                let target = works.houses.get(i).ok_or(CommandError::UnknownHouse(i))?;
                if target.fire != Blaze::default() {
                    return Err(CommandError::AlreadyBurning);
                }
                Fuel::House(i)
            }
            (None, true) => {
                if !works.pieces[works.gate].intact() || works.gate_fire != Blaze::default() {
                    return Err(CommandError::AlreadyBurning);
                }
                Fuel::Gate
            }
            _ => return Err(CommandError::NothingToBurn),
        };
        let Some(&id) = units
            .iter()
            .find(|&&id| torch_distance(works, &rules, &self.units[id as usize], fuel).is_some())
        else {
            return Err(CommandError::TooFarToBurn(units[0]));
        };
        let chance = rules.torch.chance * rules.weather(self.weather).ignition;
        let label = self.unit_label(id as usize);
        let side = self.units[id as usize].side;
        if self.fire.rng.unit() < chance {
            let text = match fuel {
                Fuel::House(i) if self.siege.as_ref().is_some_and(|w| w.houses[i].suburb) => {
                    format!("Les {label} incendient une maison du faubourg.")
                }
                Fuel::House(_) => format!("Les {label} mettent le feu à une maison."),
                Fuel::Gate => {
                    format!("Les {label} entassent des fagots contre la porte et y mettent le feu.")
                }
            };
            self.log(text, Some(side));
            self.ignite(fuel, rules.torch.initial_intensity);
        } else {
            self.log(
                format!("Les torches des {label} ne prennent pas : le bois est trop mouillé."),
                Some(side),
            );
        }
        Ok(())
    }
}

/// Distance (metres, ≥ 0) from `unit` to `fuel` when it can put a torch to
/// it, `None` when out of reach: engines and synthetic units never can; the
/// garrison reaches its suburbs from anywhere.
fn torch_distance(works: &SiegeWorks, rules: &FireRules, unit: &Unit, fuel: Fuel) -> Option<f64> {
    if unit.synthetic || unit.category == UnitCategory::Siege {
        return None;
    }
    let reach = rules.torch.reach_m;
    match fuel {
        Fuel::House(i) => {
            let h = &works.houses[i];
            let distance = unit.distance_to_rect(h.x, h.z) - h.radius;
            ((h.suburb && unit.side == SideId::Defender) || distance <= reach)
                .then_some(distance.max(0.0))
        }
        Fuel::Gate => {
            let (cx, cz) = works.pieces[works.gate].closest_point(unit.x, unit.z);
            let distance = unit.distance_to_rect(cx, cz);
            (distance <= reach + works.band()).then_some(distance.max(0.0))
        }
    }
}

/// Heat of the fires of `works` on `unit` (see [`BattleSim::heat_intensity`]).
fn heat_on(works: &SiegeWorks, rules: &FireRules, unit: &Unit) -> f64 {
    let mut intensity: f64 = 0.0;
    for house in works.houses.iter().filter(|h| h.fire.burning()) {
        if unit.distance_to_rect(house.x, house.z) - house.radius <= rules.heat.radius_m {
            intensity = intensity.max(house.fire.intensity);
        }
    }
    if works.gate_fire.burning() {
        let (cx, cz) = works.pieces[works.gate].closest_point(unit.x, unit.z);
        if unit.distance_to_rect(cx, cz) <= rules.heat.radius_m {
            intensity = intensity.max(works.gate_fire.intensity);
        }
    }
    if unit.on_wall {
        intensity * rules.heat.wall_walk_factor
    } else {
        intensity
    }
}
