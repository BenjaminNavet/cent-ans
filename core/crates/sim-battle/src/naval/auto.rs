//! Naval auto-resolve, in phases and without positions: volleys (the fleet
//! holding the weather gauge shoots more), fireships, rams, boarding rounds,
//! fire, surrender, then the loser's flight. Every coefficient lives in
//! [`NavalRules`].

use data_model::NavalRules;

use super::combat::{self, climb_factor};
use super::fleet::Fleets;
use super::outcome::NavalOutcome;
use super::setup::NavalSetup;
use super::ship::{Ship, ShipStatus};
use crate::rng::BattleRng;
use crate::setup::SideId;

/// Salt mixed into the battle seed for the auto-resolve dice.
const SEED_SALT: u64 = 0x0A07_0E5E;

/// Resolves a naval battle: a side without ships loses by default.
pub fn auto_resolve(setup: &NavalSetup, seed: u64) -> NavalOutcome {
    let Ok(fleets) = Fleets::deploy(setup, seed) else {
        let winner = if setup.attacker.ships.is_empty() {
            SideId::Defender
        } else {
            SideId::Attacker
        };
        return NavalOutcome::from_ships(setup, &[], Some(winner), 0.0);
    };
    let mut battle = Battle::new(setup, fleets, seed);
    battle.volleys_while_closing();
    battle.fireships();
    battle.boarding_rounds();
    let winner = battle.winner();
    if let Some(winner) = winner {
        battle.flight_of(winner.other(), winner);
    }
    NavalOutcome::from_ships(setup, &battle.ships, winner, battle.elapsed)
}

/// State of one auto-resolved battle.
struct Battle<'a> {
    rules: &'a NavalRules,
    rain: bool,
    ships: Vec<Ship>,
    wind_strength: f64,
    gauge: Option<SideId>,
    rng: BattleRng,
    /// Random spread of each side's blows.
    luck: [f64; 2],
    elapsed: f64,
}

impl<'a> Battle<'a> {
    fn new(setup: &'a NavalSetup, fleets: Fleets, seed: u64) -> Self {
        let rules = &setup.rules;
        let mut rng = BattleRng::from_seed(seed ^ SEED_SALT);
        let luck = [(); 2].map(|()| 1.0 + rng.range(-rules.auto_jitter, rules.auto_jitter));
        Battle {
            rules,
            rain: setup.rain,
            ships: fleets.ships,
            wind_strength: fleets.wind_strength,
            gauge: fleets.gauge,
            rng,
            luck,
            elapsed: 0.0,
        }
    }

    /// Phase 1, volleys while the fleets close: the gauge side shoots first and more.
    fn volleys_while_closing(&mut self) {
        let rules = self.rules;
        let gauge = self.gauge;
        let volleys = |side: SideId| {
            rules.auto_volleys
                + if gauge == Some(side) {
                    rules.auto_gauge_volleys
                } else {
                    0
                }
        };
        let (attacker, defender) = (volleys(SideId::Attacker), volleys(SideId::Defender));
        let order = self.firing_order();
        for round in 0..attacker.max(defender) {
            for side in order {
                if round < volleys(side) {
                    self.volley_round(side);
                }
            }
            self.burn_all(rules.auto_volley_seconds);
            self.settle();
            self.elapsed += rules.auto_volley_seconds;
        }
    }

    /// The gauge side fires first.
    fn firing_order(&self) -> [SideId; 2] {
        match self.gauge {
            Some(SideId::Defender) => [SideId::Defender, SideId::Attacker],
            _ => [SideId::Attacker, SideId::Defender],
        }
    }

    /// Phase 2, fireships drift down on the enemy.
    fn fireships(&mut self) {
        let rules = self.rules;
        for side in SideId::BOTH {
            let chance = match self.gauge {
                Some(g) if g == side => rules.fireship_chance_gauge,
                Some(_) => rules.fireship_chance_lee,
                None => rules.fireship_chance_calm,
            };
            let fireships: Vec<usize> = self
                .ships
                .iter()
                .filter(|s| s.side == side && s.fireship && s.is_afloat())
                .map(|s| s.id as usize)
                .collect();
            for f in fireships {
                if let Some(t) = strongest(&self.ships, side.other()) {
                    if self.rng.unit() < chance {
                        let target = &mut self.ships[t];
                        let resist = target.class.fire_resistance;
                        target.fire = (target.fire + rules.fireship_fire * (1.0 - resist)).min(1.0);
                        target.morale = (target.morale - rules.fireship_morale).max(0.0);
                    }
                }
                let ship = &mut self.ships[f];
                ship.status = ShipStatus::Abandoned;
                ship.fire = 1.0;
                ship.cast_into_sea(0.0, 0.0);
            }
        }
    }

    /// Phase 3, boarding rounds: rams on first contact, melee second by
    /// second, then the shooters between the boardings.
    fn boarding_rounds(&mut self) {
        let rules = self.rules;
        let order = self.firing_order();
        for round in 0..rules.auto_rounds {
            let pairs = pair_ships(&self.ships, rules);
            if pairs.is_empty() {
                break;
            }
            if round == 0 {
                for &(a, b) in &pairs {
                    ram(&mut self.ships, a, b, rules);
                    ram(&mut self.ships, b, a, rules);
                }
            }
            for _ in 0..rules.auto_round_seconds.max(1.0) as usize {
                self.melee_second(&pairs);
            }
            for side in order {
                self.volley_round(side);
            }
            self.settle();
        }
    }

    /// One second of boarding melee between the paired ships, with its
    /// fires and surrenders.
    fn melee_second(&mut self, pairs: &[(usize, usize)]) {
        let rules = self.rules;
        let ships = &mut self.ships;
        let mut losses = vec![0.0; ships.len()];
        let mut engaged = vec![0usize; ships.len()];
        for &(a, b) in pairs {
            engaged[a] += 1;
            engaged[b] += 1;
        }
        for &(a, b) in pairs {
            if !ships[a].is_afloat() || !ships[b].is_afloat() {
                continue;
            }
            // Nobody boards a burning ship: they shoot at it instead.
            if ships[a].fire > rules.burnt_fire || ships[b].fire > rules.burnt_fire {
                continue;
            }
            let (on_b, on_a) = combat::melee_exchange(
                &ships[a],
                &ships[b],
                1.0 / engaged[a].max(1) as f64,
                1.0 / engaged[b].max(1) as f64,
                chain_support(ships, a, rules),
                chain_support(ships, b, rules),
                1.0,
                rules,
            );
            losses[b] += on_b * self.luck[ships[a].side.index()];
            losses[a] += on_a * self.luck[ships[b].side.index()];
        }
        for (ship, loss) in ships.iter_mut().zip(losses) {
            if loss <= 0.0 || !ship.is_afloat() {
                continue;
            }
            let before = ship.fighting_men();
            let killed = ship.take_losses(loss, rules.armor_vs_melee);
            if before > 0.0 {
                ship.morale = (ship.morale
                    - killed / before * 100.0 * rules.morale_per_loss_percent)
                    .max(0.0);
            }
        }
        spread_fire(ships, pairs, rules);
        self.burn_all(1.0);
        capture_broken(&mut self.ships, pairs, rules);
        self.settle();
        self.elapsed += 1.0;
    }

    /// Phase 4, the side that still stands holds the sea (the gauge breaks a tie).
    fn winner(&self) -> Option<SideId> {
        let ships = &self.ships;
        let standing = |side: SideId| {
            ships
                .iter()
                .filter(|s| {
                    s.side == side && s.is_afloat() && !s.fireship && s.fighting_men() >= 1.0
                })
                .map(Ship::melee_power)
                .sum::<f64>()
        };
        let initial = |side: SideId| {
            ships
                .iter()
                .filter(|s| s.side == side && !s.fireship)
                .map(Ship::fighting_initial)
                .sum::<f64>()
                .max(1.0)
        };
        let men = |side: SideId| {
            ships
                .iter()
                .filter(|s| s.side == side && s.is_afloat() && !s.fireship)
                .map(Ship::fighting_men)
                .sum::<f64>()
        };
        let (pa, pd) = (standing(SideId::Attacker), standing(SideId::Defender));
        let ra = men(SideId::Attacker) / initial(SideId::Attacker);
        let rd = men(SideId::Defender) / initial(SideId::Defender);
        let margin = self.rules.auto_decisive_margin;
        if pa <= 0.0 && pd <= 0.0 {
            None
        } else if pd <= 0.0 {
            Some(SideId::Attacker)
        } else if pa <= 0.0 {
            Some(SideId::Defender)
        } else if pa * ra > pd * rd * margin {
            Some(SideId::Attacker)
        } else if pd * rd > pa * ra * margin {
            Some(SideId::Defender)
        } else {
            self.gauge
        }
    }

    /// The loser's ships still afloat run for it, or strike to `winner`.
    fn flight_of(&mut self, loser: SideId, winner: SideId) {
        let rules = self.rules;
        for ship in self
            .ships
            .iter_mut()
            .filter(|s| s.side == loser && s.is_afloat())
        {
            let chance = if ship.class.oar_speed > 0.0 {
                rules.escape_chance_oars
            } else if self.gauge == Some(loser) {
                rules.escape_chance_gauge
            } else {
                rules.escape_chance_other
            };
            if ship.chain.is_none() && self.rng.unit() < chance {
                ship.status = ShipStatus::Escaped;
            } else {
                ship.strike(winner);
            }
        }
    }

    /// One volley of every shooter of `side`, each ship at the enemy ship
    /// facing it in line (round-robin).
    fn volley_round(&mut self, side: SideId) {
        let rules = self.rules;
        let alignment = match self.gauge {
            Some(g) if g == side => rules.auto_volley_alignment,
            Some(_) => -rules.auto_volley_alignment,
            None => 0.0,
        };
        let ships = &mut self.ships;
        let shooters: Vec<usize> = ships
            .iter()
            .filter(|s| s.side == side && s.is_afloat() && !s.fireship && s.ranged_power() > 0.0)
            .map(|s| s.id as usize)
            .collect();
        let targets: Vec<usize> = ships
            .iter()
            .filter(|s| s.side != side && s.is_afloat() && !s.fireship)
            .map(|s| s.id as usize)
            .collect();
        if targets.is_empty() {
            return;
        }
        let luck = self.luck[side.index()];
        for (n, &i) in shooters.iter().enumerate() {
            let t = targets[n % targets.len()];
            for g in 0..ships[i].crew.len() {
                if !ships[i].crew[g].shoots() {
                    continue;
                }
                let distance = ships[i].crew[g].range * rules.auto_range_share;
                let v = combat::volley(
                    &ships[i],
                    g,
                    &ships[t],
                    distance,
                    self.wind_strength,
                    alignment,
                    self.rain,
                    rules,
                );
                let reload = combat::reload_seconds(&ships[i].crew[g], rules);
                let count = (rules.auto_volley_seconds / reload)
                    .max(1.0)
                    .min(ships[i].crew[g].ammo);
                ships[i].crew[g].ammo -= count;
                let before = ships[t].fighting_men();
                let killed = ships[t].take_losses(v.hits * count * luck, rules.armor_vs_ranged);
                if before > 0.0 {
                    let factor = combat::morale_factor(&ships[t], rules);
                    let morale = &mut ships[t].morale;
                    *morale = (*morale
                        - killed / before
                            * 100.0
                            * rules.morale_per_loss_percent
                            * rules.auto_volley_morale
                            * factor)
                        .max(0.0);
                }
                ships[t].fire = (ships[t].fire + v.fire * count).min(1.0);
            }
        }
    }

    /// Fires burn for `seconds`, in whole-second steps at most.
    fn burn_all(&mut self, seconds: f64) {
        let steps = seconds.ceil().max(1.0) as usize;
        for ship in self.ships.iter_mut().filter(|s| !s.is_out()) {
            for _ in 0..steps {
                combat::burn(ship, self.wind_strength, seconds / steps as f64, self.rules);
            }
        }
    }

    /// Abandons burning ships and sends holed ones to the bottom.
    fn settle(&mut self) {
        let rules = self.rules;
        for ship in &mut self.ships {
            let burning = ship.is_afloat() && ship.fire >= rules.fire_abandon;
            let holed = !ship.is_out() && ship.hull <= 0.0;
            if burning || holed {
                ship.cast_into_sea(rules.drown_base, rules.drown_armor);
                ship.status = if holed {
                    ShipStatus::Sinking {
                        since: self.elapsed,
                    }
                } else {
                    ShipStatus::Abandoned
                };
            }
        }
    }
}

/// Fire spreads between lashed ships.
fn spread_fire(ships: &mut [Ship], pairs: &[(usize, usize)], rules: &NavalRules) {
    for &(a, b) in pairs {
        let (fa, fb) = (ships[a].fire, ships[b].fire);
        if fa > rules.fire_spread_threshold && ships[b].is_afloat() {
            ships[b].fire =
                (fb + fa * rules.fire_spread * (1.0 - ships[b].class.fire_resistance)).min(1.0);
        }
        if fb > rules.fire_spread_threshold && ships[a].is_afloat() {
            ships[a].fire =
                (fa + fb * rules.fire_spread * (1.0 - ships[a].class.fire_resistance)).min(1.0);
        }
    }
}

/// Broken crews strike to the ship they fight.
fn capture_broken(ships: &mut [Ship], pairs: &[(usize, usize)], rules: &NavalRules) {
    for &(a, b) in pairs {
        for (loser, winner) in [(a, b), (b, a)] {
            if !ships[loser].is_afloat() || !ships[winner].is_afloat() {
                continue;
            }
            let share = ships[loser].fighting_men() / ships[loser].fighting_initial().max(1.0);
            if ships[loser].morale < rules.surrender_morale || share < rules.surrender_crew_share {
                let by = ships[winner].side;
                ships[loser].strike(by);
            }
        }
    }
}

/// Pairs every afloat ship with an enemy: the strongest meet first, the
/// larger fleet doubles up on the enemy's weakest ships.
fn pair_ships(ships: &[Ship], rules: &NavalRules) -> Vec<(usize, usize)> {
    let fleet = |side: SideId| {
        let mut ids: Vec<usize> = ships
            .iter()
            .filter(|s| s.side == side && s.is_afloat() && !s.fireship && s.fighting_men() >= 1.0)
            .map(|s| s.id as usize)
            .collect();
        ids.sort_by(|&x, &y| {
            ships[y]
                .melee_power()
                .total_cmp(&ships[x].melee_power())
                .then(x.cmp(&y))
        });
        ids
    };
    let (a, d) = (fleet(SideId::Attacker), fleet(SideId::Defender));
    if a.is_empty() || d.is_empty() {
        return Vec::new();
    }
    let (big, small) = if a.len() >= d.len() {
        (&a, &d)
    } else {
        (&d, &a)
    };
    let mut pairs = Vec::new();
    for (n, &i) in big.iter().enumerate() {
        let j = if n < small.len() {
            small[n]
        } else {
            // Extra ships go for the enemy they can climb onto best.
            let appeal = |x: usize| {
                climb_factor(ships[x].deck_height() - ships[i].deck_height(), rules)
                    / ships[x].melee_power().max(rules.auto_min_power)
            };
            *small
                .iter()
                .max_by(|&&x, &&y| appeal(x).total_cmp(&appeal(y)).then(y.cmp(&x)))
                .unwrap_or(&small[0])
        };
        pairs.push((i.min(j), i.max(j)));
    }
    pairs
}

/// Reinforcements over the chains: in the auto-resolve every chained ship is
/// engaged, so only a share of the support counts.
fn chain_support(ships: &[Ship], i: usize, rules: &NavalRules) -> f64 {
    let Some(chain) = ships[i].chain else {
        return 0.0;
    };
    let neighbours = ships
        .iter()
        .filter(|s| {
            s.id as usize != i && s.side == ships[i].side && s.chain == Some(chain) && s.is_afloat()
        })
        .count()
        .min(2);
    neighbours as f64 * rules.chain_support * rules.auto_chain_share
}

/// A galley's spur against a low hull, on first contact.
fn ram(ships: &mut [Ship], a: usize, b: usize, rules: &NavalRules) {
    if !ships[a].is_galley() || !ships[b].is_afloat() {
        return;
    }
    let high = if ships[b].class.freeboard_m >= rules.ram_high_freeboard_m {
        rules.ram_high_factor
    } else {
        1.0
    };
    let damage = ships[a].class.ram * ships[a].class.oar_speed * (1.0 + rules.ram_speed) * high;
    ships[b].hull -= damage;
    ships[b].morale = (ships[b].morale - rules.ram_morale).max(0.0);
}

fn strongest(ships: &[Ship], side: SideId) -> Option<usize> {
    ships
        .iter()
        .filter(|s| s.side == side && s.is_afloat() && !s.fireship)
        .max_by(|a, b| {
            a.melee_power()
                .total_cmp(&b.melee_power())
                .then(b.id.cmp(&a.id))
        })
        .map(|s| s.id as usize)
}
