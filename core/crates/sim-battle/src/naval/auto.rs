//! Naval auto-resolve (lot NV1, ADR 0028), the phased logic of lot N1
//! (ADR 0013) at sea: volleys (the fleet holding the weather gauge shoots
//! more), fireships, rams, boarding rounds fought with the very formulas of
//! the real-time battle ([`super::combat`]), fire, surrender, then the
//! loser's flight. Calibrated on [`super::NavalSim`] by
//! `tests/nv1_naval.rs`.

use super::combat::{self, climb_factor};
use super::outcome::NavalOutcome;
use super::setup::NavalSetup;
use super::ship::{Ship, ShipStatus};
use super::sim::{outcome_of, NavalSim};
use crate::rng::BattleRng;
use crate::setup::SideId;

/// Seconds of shooting one auto-resolve volley stands for.
const VOLLEY_SECONDS: f64 = 30.0;

/// Resolves a naval battle without the real-time simulation.
pub fn auto_resolve(setup: &NavalSetup, seed: u64) -> NavalOutcome {
    let Ok(sim) = NavalSim::new(setup.clone(), seed) else {
        // A side without ships: the other holds the sea unopposed.
        let winner = if setup.attacker.ships.is_empty() {
            Some(SideId::Defender)
        } else {
            Some(SideId::Attacker)
        };
        return outcome_of(setup, &[], winner, 0.0, true);
    };
    let rules = setup.rules.clone();
    let gauge = sim.gauge();
    let wind = sim.wind;
    let mut ships = sim.ships;
    let mut rng = BattleRng::from_seed(seed ^ 0x0A07_0E5E);
    let luck = [
        1.0 + rng.range(-rules.auto_jitter, rules.auto_jitter),
        1.0 + rng.range(-rules.auto_jitter, rules.auto_jitter),
    ];
    let mut elapsed = 0.0;

    // 1. Volleys while the fleets close: the gauge side shoots first and more.
    let volleys = |side: SideId| {
        rules.auto_volleys
            + if gauge == Some(side) {
                rules.auto_gauge_volleys
            } else {
                0
            }
    };
    let most = volleys(SideId::Attacker).max(volleys(SideId::Defender));
    let order = match gauge {
        Some(SideId::Defender) => [SideId::Defender, SideId::Attacker],
        _ => [SideId::Attacker, SideId::Defender],
    };
    for round in 0..most {
        for side in order {
            if round < volleys(side) {
                volley_round(
                    &mut ships,
                    side,
                    gauge,
                    wind,
                    luck[side.index()],
                    setup.rain,
                    &rules,
                );
            }
        }
        for ship in &mut ships {
            burn_for(ship, wind, VOLLEY_SECONDS, &rules);
        }
        settle(&mut ships, &rules, elapsed);
        elapsed += VOLLEY_SECONDS;
    }

    // 2. Fireships drift down on the enemy.
    for side in SideId::BOTH {
        let chance = match gauge {
            Some(g) if g == side => 0.85,
            Some(_) => 0.35,
            None => 0.6,
        };
        let fireships: Vec<usize> = ships
            .iter()
            .filter(|s| s.side == side && s.fireship && s.is_afloat())
            .map(|s| s.id as usize)
            .collect();
        for f in fireships {
            let target = strongest(&ships, side.other());
            if let Some(t) = target {
                if rng.unit() < chance {
                    let resist = ships[t].class.fire_resistance;
                    ships[t].fire = (ships[t].fire + rules.fireship_fire * (1.0 - resist)).min(1.0);
                    ships[t].morale = (ships[t].morale - 15.0).max(0.0);
                }
            }
            let s = &mut ships[f];
            s.status = ShipStatus::Abandoned;
            s.fire = 1.0;
            for (k, crew) in s.crew.iter_mut().enumerate() {
                s.swimmers[k] += crew.men;
                crew.men = 0.0;
            }
            s.sailors = 0.0;
        }
    }

    // 3. Boarding rounds.
    for round in 0..rules.auto_rounds {
        let pairs = pair_ships(&ships, &rules);
        if pairs.is_empty() {
            break;
        }
        if round == 0 {
            for &(a, b) in &pairs {
                ram(&mut ships, a, b, &rules);
                ram(&mut ships, b, a, &rules);
            }
        }
        let seconds = rules.auto_round_seconds.max(1.0) as usize;
        for _ in 0..seconds {
            let mut losses = vec![0.0; ships.len()];
            let mut engaged = vec![0usize; ships.len()];
            for &(a, b) in &pairs {
                engaged[a] += 1;
                engaged[b] += 1;
            }
            for &(a, b) in &pairs {
                if !ships[a].is_afloat() || !ships[b].is_afloat() {
                    continue;
                }
                // Nobody boards a burning ship: they shoot at it instead.
                if ships[a].fire > 0.3 || ships[b].fire > 0.3 {
                    continue;
                }
                let (on_b, on_a) = combat::melee_exchange(
                    &ships[a],
                    &ships[b],
                    1.0 / engaged[a].max(1) as f64,
                    1.0 / engaged[b].max(1) as f64,
                    chain_support(&ships, a, &rules),
                    chain_support(&ships, b, &rules),
                    1.0,
                    &rules,
                );
                losses[b] += on_b * luck[ships[a].side.index()];
                losses[a] += on_a * luck[ships[b].side.index()];
            }
            for (i, loss) in losses.into_iter().enumerate() {
                if loss <= 0.0 || !ships[i].is_afloat() {
                    continue;
                }
                let before = ships[i].fighting_men();
                let killed = ships[i].take_losses(loss, rules.armor_vs_melee);
                if before > 0.0 {
                    let m = &mut ships[i].morale;
                    *m = (*m - killed / before * 100.0 * rules.morale_per_loss_percent).max(0.0);
                }
            }
            // Fire spreads between lashed ships.
            for &(a, b) in &pairs {
                let (fa, fb) = (ships[a].fire, ships[b].fire);
                if fa > 0.2 && ships[b].is_afloat() {
                    ships[b].fire = (fb
                        + fa * rules.fire_spread * (1.0 - ships[b].class.fire_resistance))
                        .min(1.0);
                }
                if fb > 0.2 && ships[a].is_afloat() {
                    ships[a].fire = (fa
                        + fb * rules.fire_spread * (1.0 - ships[a].class.fire_resistance))
                        .min(1.0);
                }
            }
            for ship in &mut ships {
                burn_for(ship, wind, 1.0, &rules);
            }
            capture_broken(&mut ships, &pairs, &rules);
            settle(&mut ships, &rules, elapsed);
            elapsed += 1.0;
        }
        // Shooters keep shooting between the boardings.
        for side in order {
            volley_round(
                &mut ships,
                side,
                gauge,
                wind,
                luck[side.index()],
                setup.rain,
                &rules,
            );
        }
        settle(&mut ships, &rules, elapsed);
    }

    // 4. The side that still stands holds the sea; the loser runs.
    let standing = |ships: &[Ship], side: SideId| {
        ships
            .iter()
            .filter(|s| s.side == side && s.is_afloat() && !s.fireship && s.fighting_men() >= 1.0)
            .map(Ship::melee_power)
            .sum::<f64>()
    };
    let initial = |ships: &[Ship], side: SideId| {
        ships
            .iter()
            .filter(|s| s.side == side && !s.fireship)
            .map(|s| s.fighting_initial())
            .sum::<f64>()
            .max(1.0)
    };
    let men = |ships: &[Ship], side: SideId| {
        ships
            .iter()
            .filter(|s| s.side == side && s.is_afloat() && !s.fireship)
            .map(Ship::fighting_men)
            .sum::<f64>()
    };
    let (pa, pd) = (
        standing(&ships, SideId::Attacker),
        standing(&ships, SideId::Defender),
    );
    let ra = men(&ships, SideId::Attacker) / initial(&ships, SideId::Attacker);
    let rd = men(&ships, SideId::Defender) / initial(&ships, SideId::Defender);
    let winner = if pa <= 0.0 && pd <= 0.0 {
        None
    } else if pd <= 0.0 {
        Some(SideId::Attacker)
    } else if pa <= 0.0 {
        Some(SideId::Defender)
    } else if pa * ra > pd * rd * 1.1 {
        Some(SideId::Attacker)
    } else if pd * rd > pa * ra * 1.1 {
        Some(SideId::Defender)
    } else {
        gauge
    };
    if let Some(winner) = winner {
        let loser = winner.other();
        for ship in ships
            .iter_mut()
            .filter(|s| s.side == loser && s.is_afloat())
        {
            let chance = if ship.class.oar_speed > 0.0 {
                0.7
            } else if gauge == Some(loser) {
                0.5
            } else {
                0.3
            };
            if ship.chain.is_none() && rng.unit() < chance {
                ship.status = ShipStatus::Escaped;
            } else {
                for (k, crew) in ship.crew.iter_mut().enumerate() {
                    ship.prisoners[k] += crew.men;
                    crew.men = 0.0;
                }
                ship.sailors = 0.0;
                ship.status = ShipStatus::Captured { by: winner };
            }
        }
    }
    outcome_of(setup, &ships, winner, elapsed, true)
}

/// One volley of every shooter of `side`, each ship at the enemy ship
/// facing it in line (round-robin).
fn volley_round(
    ships: &mut [Ship],
    side: SideId,
    gauge: Option<SideId>,
    wind: combat::Wind,
    luck: f64,
    rain: bool,
    rules: &data_model::NavalRules,
) {
    let alignment = match gauge {
        Some(g) if g == side => 0.8,
        Some(_) => -0.8,
        None => 0.0,
    };
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
    for (n, &i) in shooters.iter().enumerate() {
        let t = targets[n % targets.len()];
        for g in 0..ships[i].crew.len() {
            if !ships[i].crew[g].shoots() {
                continue;
            }
            let distance = ships[i].crew[g].range * 0.55;
            let v = combat::volley(
                &ships[i], g, &ships[t], distance, wind, alignment, rain, rules,
            );
            let reload = combat::reload_seconds(&ships[i].crew[g], rules);
            let count = (VOLLEY_SECONDS / reload)
                .max(1.0)
                .min(ships[i].crew[g].ammo);
            ships[i].crew[g].ammo -= count;
            let before = ships[t].fighting_men();
            let killed = ships[t].take_losses(v.hits * count * luck, rules.armor_vs_ranged);
            if before > 0.0 {
                let factor = combat::morale_factor(&ships[t], rules);
                let m = &mut ships[t].morale;
                *m = (*m - killed / before * 100.0 * rules.morale_per_loss_percent * 0.6 * factor)
                    .max(0.0);
            }
            ships[t].fire = (ships[t].fire + v.fire * count).min(1.0);
        }
    }
}

fn burn_for(ship: &mut Ship, wind: combat::Wind, seconds: f64, rules: &data_model::NavalRules) {
    if matches!(
        ship.status,
        ShipStatus::Sunk | ShipStatus::Escaped | ShipStatus::Sinking { .. }
    ) {
        return;
    }
    let steps = seconds.ceil().max(1.0) as usize;
    for _ in 0..steps {
        combat::burn(ship, wind, seconds / steps as f64, rules);
    }
}

/// Abandons burning ships and sends holed ones to the bottom.
fn settle(ships: &mut [Ship], rules: &data_model::NavalRules, elapsed: f64) {
    for ship in ships.iter_mut() {
        let burning = ship.is_afloat() && ship.fire >= rules.fire_abandon;
        let holed = !matches!(
            ship.status,
            ShipStatus::Sunk | ShipStatus::Escaped | ShipStatus::Sinking { .. }
        ) && ship.hull <= 0.0;
        if burning || holed {
            for (k, crew) in ship.crew.iter_mut().enumerate() {
                let drown = (rules.drown_base + crew.armor / 100.0 * rules.drown_armor).min(1.0);
                ship.drowned[k] += crew.men * drown;
                ship.swimmers[k] += crew.men * (1.0 - drown);
                crew.men = 0.0;
            }
            ship.sailors = 0.0;
            ship.status = if holed {
                ShipStatus::Sinking { since: elapsed }
            } else {
                ShipStatus::Abandoned
            };
        }
    }
}

/// Broken crews strike to the ship they fight.
fn capture_broken(ships: &mut [Ship], pairs: &[(usize, usize)], rules: &data_model::NavalRules) {
    for &(a, b) in pairs {
        for (loser, winner) in [(a, b), (b, a)] {
            if !ships[loser].is_afloat() || !ships[winner].is_afloat() {
                continue;
            }
            let share = ships[loser].fighting_men() / ships[loser].fighting_initial().max(1.0);
            if ships[loser].morale < rules.surrender_morale || share < rules.surrender_crew_share {
                let by = ships[winner].side;
                let s = &mut ships[loser];
                for (k, crew) in s.crew.iter_mut().enumerate() {
                    s.prisoners[k] += crew.men;
                    crew.men = 0.0;
                }
                s.sailors = 0.0;
                s.status = ShipStatus::Captured { by };
            }
        }
    }
}

/// Pairs every afloat ship with an enemy: the strongest meet first, the
/// larger fleet doubles up on the enemy's weakest ships.
fn pair_ships(ships: &[Ship], rules: &data_model::NavalRules) -> Vec<(usize, usize)> {
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
            *small
                .iter()
                .max_by(|&&x, &&y| {
                    let fx = climb_factor(ships[x].deck_height() - ships[i].deck_height(), rules)
                        / ships[x].melee_power().max(0.1);
                    let fy = climb_factor(ships[y].deck_height() - ships[i].deck_height(), rules)
                        / ships[y].melee_power().max(0.1);
                    fx.total_cmp(&fy).then(y.cmp(&x))
                })
                .unwrap_or(&small[0])
        };
        pairs.push((i.min(j), i.max(j)));
    }
    pairs
}

fn chain_support(ships: &[Ship], i: usize, rules: &data_model::NavalRules) -> f64 {
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
    // In the auto-resolve every chained ship is engaged: half the support.
    neighbours as f64 * rules.chain_support * 0.5
}

/// A galley's spur against a low hull, on first contact.
fn ram(ships: &mut [Ship], a: usize, b: usize, rules: &data_model::NavalRules) {
    if !ships[a].is_galley() || !ships[b].is_afloat() {
        return;
    }
    let high = if ships[b].class.freeboard_m >= 2.5 {
        rules.ram_high_factor
    } else {
        1.0
    };
    let damage = ships[a].class.ram * ships[a].class.oar_speed * (1.0 + rules.ram_speed) * high;
    ships[b].hull -= damage;
    ships[b].morale = (ships[b].morale - 8.0).max(0.0);
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
