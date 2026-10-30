//! Builders of the journal of causes (ADR 0748): one pure function per [`CauseKind`], fed with
//! the facts the producers gather where the event happens (battle, siege, economy, revolt,
//! diplomacy, succession, end of the game). Keys only; the sentences are data.

use data_model::{FactionId, ProvinceId};

use crate::causes::{Cause, CauseKind};

fn pct(part: u64, whole: u64) -> i64 {
    part.saturating_mul(100)
        .checked_div(whole)
        .map_or(0, |share| share as i64)
}

/// What a battle's cause is built from (the loser's point of view).
pub struct BattleFacts<'a> {
    pub winner: &'a FactionId,
    pub loser: &'a FactionId,
    pub province: &'a ProvinceId,
    pub winner_strength: u32,
    pub loser_strength: u32,
    pub loser_losses: u32,
    /// Estimated damage dealt per side before the battle (situation modifiers included).
    pub winner_power: f64,
    pub loser_power: f64,
    pub loser_general_lost: bool,
    pub loser_routed: bool,
    pub loser_morale_delta: i32,
    pub winner_defended: bool,
    /// Terrain key of the province when it favours the defender (rough ground).
    pub rough_terrain: Option<&'static str>,
}

/// Cause of a battle: the loser's numbers, the factors that decided it.
pub fn battle_cause(f: &BattleFacts<'_>) -> Cause {
    let own_share = pct(
        u64::from(f.loser_losses),
        u64::from(f.loser_strength.max(1)),
    );
    let ratio = pct(
        u64::from(f.loser_strength),
        u64::from(f.winner_strength.max(1)),
    );
    let mut severity = 35 + own_share / 2;
    if f.loser_general_lost {
        severity += 15;
    }
    if f.loser_routed {
        severity += 10;
    }
    let mut cause = Cause::new(CauseKind::Battle, severity.clamp(0, 100) as u8)
        .subject(f.loser)
        .actor(f.winner)
        .province(f.province)
        .factor("heavy_losses", own_share, own_share);
    if ratio < 100 {
        cause = cause.factor("outnumbered", ratio, 100 - ratio);
    }
    if f.loser_power > 0.0 && f.winner_power > f.loser_power {
        let edge = ((f.winner_power / f.loser_power - 1.0) * 100.0).round() as i64;
        cause = cause.factor("firepower_edge", edge, edge / 2);
    }
    if f.loser_general_lost {
        cause = cause.factor("general_lost", 1, 70);
    }
    if f.loser_routed {
        cause = cause.factor("morale_broken", i64::from(-f.loser_morale_delta), 60);
    } else if f.loser_morale_delta < 0 {
        let points = i64::from(-f.loser_morale_delta);
        cause = cause.factor("morale_shaken", points, points * 2);
    }
    if let (true, Some(terrain)) = (f.winner_defended, f.rough_terrain) {
        cause = cause.factor(&format!("terrain_{terrain}"), 1, 35);
    }
    cause.ranked()
}

/// What the cause of a place taken is built from.
pub struct PlaceFacts<'a> {
    pub previous: &'a FactionId,
    pub new_holder: &'a FactionId,
    pub province: &'a ProvinceId,
    pub is_city: bool,
    /// `(seasons elapsed, supplies left, breach)` of the siege, if one was under way.
    pub siege: Option<(u32, u8, u8)>,
    /// Men of the garrison before the fall, and of the besiegers.
    pub garrison: u32,
    pub besiegers: u32,
}

/// Cause of a place changing hands.
pub fn place_cause(f: &PlaceFacts<'_>) -> Cause {
    let mut cause = Cause::new(CauseKind::PlaceTaken, if f.is_city { 55 } else { 30 })
        .subject(f.previous)
        .actor(f.new_holder)
        .province(f.province);
    match f.siege {
        Some((seasons, supplies, breach)) => {
            cause = cause.factor("siege_seasons", i64::from(seasons), i64::from(seasons) * 8);
            if supplies == 0 {
                cause = cause.factor("starved", 0, 80);
            }
            if breach >= 50 {
                cause = cause.factor("walls_breached", i64::from(breach), i64::from(breach) / 2);
            }
        }
        None => {
            cause = cause.factor("no_siege", 0, 25);
        }
    }
    if f.garrison == 0 {
        cause = cause.factor("undefended", 0, 70);
    } else if f.besiegers > f.garrison {
        let ratio = pct(u64::from(f.besiegers), u64::from(f.garrison));
        cause = cause.factor("garrison_outnumbered", ratio, (ratio - 100) / 4);
    }
    cause.ranked()
}

/// What the cause of a bankruptcy is built from (livres, per season).
pub struct BankruptcyFacts<'a> {
    pub faction: &'a FactionId,
    pub treasury: i64,
    pub income: i64,
    pub army_upkeep: i64,
    pub army_surcharge: i64,
    pub building_upkeep: i64,
    pub administration: i64,
    pub table: i64,
    pub household: i64,
    pub wages: i64,
    pub siege: i64,
    pub obligations: i64,
    pub interest: i64,
    pub deficit_seasons: u32,
}

/// Cause of an empty treasury: the budget lines ranked by their share of the upkeep.
pub fn bankruptcy_cause(f: &BankruptcyFacts<'_>) -> Cause {
    let upkeep = (f.army_upkeep
        + f.building_upkeep
        + f.administration
        + f.table
        + f.household
        + f.wages
        + f.siege
        + f.obligations
        + f.interest)
        .max(1);
    let share = |line: i64| pct(line.max(0) as u64, upkeep as u64);
    let gap = (upkeep - f.income).max(0);
    let severity = (60 + i64::from(f.deficit_seasons) * 5).clamp(0, 100) as u8;
    let mut cause = Cause::new(CauseKind::Bankruptcy, severity)
        .subject(f.faction)
        .factor("army_upkeep", f.army_upkeep, share(f.army_upkeep))
        .factor(
            "army_surcharge",
            f.army_surcharge,
            share(f.army_surcharge) + 1,
        )
        .factor(
            "building_upkeep",
            f.building_upkeep,
            share(f.building_upkeep),
        )
        .factor("administration", f.administration, share(f.administration))
        .factor("table", f.table, share(f.table))
        .factor("household", f.household, share(f.household))
        .factor("wages", f.wages, share(f.wages))
        .factor("siege_costs", f.siege, share(f.siege))
        .factor("obligations", f.obligations, share(f.obligations))
        .factor("interest", f.interest, share(f.interest) + 2)
        .factor("income_short", f.income, pct(gap as u64, upkeep as u64));
    if f.deficit_seasons > 1 {
        let seasons = i64::from(f.deficit_seasons);
        cause = cause.factor("deficit_seasons", seasons, seasons * 15);
    }
    if f.treasury < 0 {
        cause = cause.factor("debt", f.treasury, 10);
    }
    cause.ranked()
}

/// What the cause of an extraordinary tax is built from.
pub struct LevyFacts<'a> {
    pub faction: &'a FactionId,
    pub yield_livres: i64,
    pub unrest: i64,
    pub treasury_before: i64,
}

/// Cause of an extraordinary tax: the yield, the unrest it feeds and the empty purse behind it.
pub fn levy_cause(f: &LevyFacts<'_>) -> Cause {
    let mut cause = Cause::new(CauseKind::ExtraordinaryTax, 40)
        .subject(f.faction)
        .factor("levy_yield", f.yield_livres, 40)
        .factor("unrest_levy", f.unrest, f.unrest * 6);
    if f.treasury_before < 1500 {
        cause = cause.factor("levy_shortfall", f.treasury_before, 70);
    }
    cause.ranked()
}

/// What the cause of a royal default is built from.
pub struct DefaultFacts<'a> {
    pub faction: &'a FactionId,
    /// Debt, per cent of one season of gross income.
    pub debt_seasons_percent: i64,
    pub written_off_percent: i64,
    pub freeze_turns: i64,
    pub seizure: i64,
    pub treasury: i64,
}

/// Cause of a royal default: the load of debt, then what the bankers take in return.
pub fn default_cause(f: &DefaultFacts<'_>) -> Cause {
    Cause::new(CauseKind::RoyalDefault, 85)
        .subject(f.faction)
        .factor("debt_seasons", f.debt_seasons_percent, 90)
        .factor("treasury_negative", f.treasury, 60)
        .factor("write_off", f.written_off_percent, 40)
        .factor("credit_frozen", f.freeze_turns, 35)
        .factor("seizure", f.seizure, 30)
        .ranked()
}

/// What the cause of a debasement is built from.
pub struct DebasementFacts<'a> {
    pub faction: &'a FactionId,
    pub seigniorage_gain: i64,
    pub price_level: i64,
    pub treasury: i64,
}

/// Cause of a debasement: the seigniorage wanted, the empty purse, the inflation already there.
pub fn debasement_cause(f: &DebasementFacts<'_>) -> Cause {
    let mut cause = Cause::new(CauseKind::Debasement, 35)
        .subject(f.faction)
        .factor("seigniorage_gain", f.seigniorage_gain, 50)
        .factor(
            "inflation_ahead",
            f.price_level,
            (f.price_level - 100).max(0),
        );
    if f.treasury < 0 {
        cause = cause.factor("treasury_negative", f.treasury, 75);
    }
    cause.ranked()
}

/// Cause of an uprising or of a warning of rising tension: the unrest terms of the province
/// (points of discontent each pulls, positive ones only), heaviest first.
pub fn unrest_cause(
    kind: CauseKind,
    lord: &FactionId,
    province: &ProvinceId,
    tension: u32,
    terms: &[(&str, f64)],
) -> Cause {
    let mut cause = Cause::new(kind, if kind == CauseKind::Revolt { 70 } else { 35 })
        .subject(lord)
        .province(province)
        .factor("tension", i64::from(tension), 5);
    for (key, points) in terms {
        let points = points.round() as i64;
        if points > 0 {
            cause = cause.factor(&format!("unrest_{key}"), points, points * 3);
        }
    }
    cause.ranked()
}

/// What the cause of a war declaration is built from.
pub struct WarFacts<'a> {
    pub attacker: &'a FactionId,
    pub target: &'a FactionId,
    pub casus_belli: bool,
    pub truce_broken: bool,
    pub pact_broken: bool,
    pub sworn_bond: bool,
    pub attacker_power: f64,
    pub target_power: f64,
    /// The AI's named reason (doctrine or objective), `None` for the player's own war.
    pub reason: Option<&'a str>,
}

/// Cause of a war declared.
pub fn war_cause(f: &WarFacts<'_>) -> Cause {
    let mut cause = Cause::new(CauseKind::WarDeclared, 50)
        .subject(f.target)
        .actor(f.attacker);
    if f.sworn_bond {
        cause = cause.factor("sworn_bond", 1, 90);
    }
    if f.truce_broken {
        cause = cause.factor("truce_broken", 1, 85);
    }
    if f.pact_broken {
        cause = cause.factor("pact_broken", 1, 85);
    }
    if f.casus_belli {
        cause = cause.factor("casus_belli", 1, 40);
    } else if !(f.truce_broken || f.pact_broken || f.sworn_bond) {
        cause = cause.factor("no_casus_belli", 1, 45);
    }
    if f.target_power > 0.0 && f.attacker_power > f.target_power {
        let edge = ((f.attacker_power / f.target_power - 1.0) * 100.0).round() as i64;
        cause = cause.factor("stronger_aggressor", edge, edge / 3);
    } else if f.attacker_power > 0.0 && f.target_power > f.attacker_power {
        let edge = ((f.target_power / f.attacker_power - 1.0) * 100.0).round() as i64;
        cause = cause.factor("stronger_target", edge, edge / 3);
    }
    if let Some(reason) = f.reason {
        cause = cause.reason(reason);
    }
    cause.ranked()
}

/// Cause of an alliance broken.
pub fn alliance_cause(
    breaker: &FactionId,
    abandoned: &FactionId,
    opinion_penalty: i64,
    reason: Option<&str>,
) -> Cause {
    let mut cause = Cause::new(CauseKind::AllianceBroken, 40)
        .subject(abandoned)
        .actor(breaker)
        .factor("betrayal", opinion_penalty, 50);
    if let Some(reason) = reason {
        cause = cause.reason(reason);
    }
    cause.ranked()
}

/// What the cause of a succession is built from.
pub struct SuccessionFacts<'a> {
    pub faction: &'a FactionId,
    pub dead_ruler_age: Option<i32>,
    pub designated_heir: bool,
    pub contested: bool,
    pub successor_age: Option<i32>,
    pub successor_minor: bool,
    pub line_extinct: bool,
    pub elective: bool,
}

/// Cause of a succession (the realm's setback, if any).
pub fn succession_cause(f: &SuccessionFacts<'_>) -> Cause {
    let mut cause =
        Cause::new(CauseKind::Succession, if f.line_extinct { 85 } else { 55 }).subject(f.faction);
    if let Some(age) = f.dead_ruler_age {
        cause = cause.factor("ruler_age", i64::from(age), i64::from(age - 40));
    }
    if !f.designated_heir && !f.elective {
        cause = cause.factor("no_designated_heir", 1, 50);
    }
    if f.contested {
        cause = cause.factor("contested", 1, 65);
    }
    if let (Some(age), true) = (f.successor_age, f.successor_minor) {
        cause = cause.factor("minor_successor", i64::from(age), 60);
    }
    if f.line_extinct {
        cause = cause.factor("line_extinct", 1, 90);
    }
    if f.elective {
        cause = cause.factor("elective", 1, 20);
    }
    cause.ranked()
}

/// Cause of the end of the player's campaign: a few figures of the realm.
pub fn game_end_cause(
    kind: CauseKind,
    faction: &FactionId,
    provinces_held: u32,
    objectives: (u32, u32),
    score: i64,
) -> Cause {
    let (done, total) = objectives;
    let mut cause = Cause::new(kind, 100)
        .subject(faction)
        .factor("score", score, 20)
        .factor(
            "objectives_done",
            i64::from(done),
            pct(u64::from(done), u64::from(total.max(1))),
        );
    cause = if provinces_held == 0 {
        cause.factor("no_land_left", 0, 100)
    } else {
        cause.factor("provinces_held", i64::from(provinces_held), 30)
    };
    cause.ranked()
}

/// Prefix of the factor keys that carry their own French label (attitude reasons, whose
/// labels the diplomacy core already writes): `attitude:<label>`.
pub const LABELLED_PREFIX: &str = "attitude:";

/// Names of the attitude tiers, lowest first (keys of the `attitude_to_*` factor texts).
pub const ATTITUDE_TIERS: [&str; 5] = ["hostile", "wary", "neutral", "friendly", "allied"];

/// XP-POURQUOI: the treasury of a faction moved over a season. `terms` are the signed livres
/// of the budget lines (`receipts`, `armies`, `buildings`, ..., `other`); the weight of a
/// factor is its share of the total movement, so the heaviest terms come first.
pub fn treasury_swing_cause(
    faction: &FactionId,
    before: i64,
    after: i64,
    terms: &[(&str, i64)],
) -> Cause {
    let change = after - before;
    let moved: u64 = terms
        .iter()
        .map(|(_, v)| v.unsigned_abs())
        .sum::<u64>()
        .max(1);
    let share = (change.unsigned_abs().saturating_mul(100) / before.unsigned_abs().max(1)) as i64;
    let severity = (20 + share / 2).clamp(20, 70) as u8;
    let mut cause = Cause::new(CauseKind::TreasurySwing, severity)
        .subject(faction)
        .factor("treasury_change", change, 5);
    for (key, amount) in terms {
        // A term that pushes the same way as the change is a cause; one that pushes against it
        // only softens the change and weighs a quarter.
        let share = pct(amount.unsigned_abs(), moved);
        let weight = if amount.signum() == change.signum() {
            share
        } else {
            share / 4
        };
        cause = cause.factor(&format!("treasury_{key}"), *amount, weight);
    }
    cause.ranked()
}

/// XP-POURQUOI: the unrest of a province moved by `after - before` points. `terms` are the
/// province's unrest terms (positive ones become factors, as for a revolt).
pub fn unrest_swing_cause(
    lord: &FactionId,
    province: &ProvinceId,
    before: f64,
    after: f64,
    terms: &[(&str, f64)],
) -> Cause {
    let delta = (after - before).round() as i64;
    let severity = (20 + delta.abs() * 2).clamp(20, 65) as u8;
    let direction = if delta >= 0 {
        "unrest_rise"
    } else {
        "unrest_fall"
    };
    let mut cause = Cause::new(CauseKind::UnrestSwing, severity)
        .subject(lord)
        .province(province)
        .factor(direction, delta.abs(), 5)
        .factor("unrest_level", after.round() as i64, 4);
    for (key, points) in terms {
        let points = points.round() as i64;
        if points > 0 {
            cause = cause.factor(&format!("unrest_{key}"), points, points * 3);
        }
    }
    cause.ranked()
}

/// XP-POURQUOI: the attitude of AI faction `ai` towards the player moved from `before` to
/// `after`, into tier `tier_after`. `reasons` are the labelled terms of the attitude (the
/// diplomacy core writes their French labels): the heaviest become factors.
pub fn attitude_shift_cause(
    ai: &FactionId,
    player: &FactionId,
    (before, after): (i32, i32),
    tier_after: usize,
    reasons: &[(String, i32)],
) -> Cause {
    let tier = ATTITUDE_TIERS[tier_after.min(ATTITUDE_TIERS.len() - 1)];
    let delta = i64::from(after - before);
    let severity = (20 + delta.abs() / 2).clamp(20, 60) as u8;
    let direction = if delta >= 0 {
        "attitude_warming"
    } else {
        "attitude_cooling"
    };
    let mut cause = Cause::new(CauseKind::AttitudeShift, severity)
        .subject(player)
        .actor(ai)
        .factor(&format!("attitude_to_{tier}"), i64::from(after), 90)
        .factor(direction, delta.abs(), 85);
    for (label, value) in reasons {
        cause = cause.factor(
            &format!("{LABELLED_PREFIX}{label}"),
            i64::from(*value),
            i64::from(value.abs()),
        );
    }
    cause.ranked()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn f(id: &str) -> FactionId {
        FactionId::new(id).unwrap()
    }

    fn keys(cause: &Cause) -> Vec<&str> {
        cause.factors.iter().map(|x| x.key.as_str()).collect()
    }

    #[test]
    fn battle_factors_are_ranked_and_complete() {
        let (winner, loser) = (f("fac_england"), f("fac_france"));
        let province = ProvinceId::new("prov_picardie").unwrap();
        let cause = battle_cause(&BattleFacts {
            winner: &winner,
            loser: &loser,
            province: &province,
            winner_strength: 3000,
            loser_strength: 6000,
            loser_losses: 2040,
            winner_power: 150.0,
            loser_power: 100.0,
            loser_general_lost: true,
            loser_routed: true,
            loser_morale_delta: -30,
            winner_defended: true,
            rough_terrain: Some("forest"),
        });
        assert_eq!(cause.kind, CauseKind::Battle);
        let first = &cause.factors[0];
        assert_eq!(first.key, "general_lost");
        assert!(keys(&cause).contains(&"heavy_losses"));
        assert!(keys(&cause).contains(&"firepower_edge"));
        assert!(keys(&cause).contains(&"terrain_forest"));
        // 2040 of 6000 men lost: 34 %.
        let losses = cause
            .factors
            .iter()
            .find(|x| x.key == "heavy_losses")
            .unwrap();
        assert_eq!(losses.value, 34);
        // Not outnumbered (loser had 200 % of the winner's men).
        assert!(!keys(&cause).contains(&"outnumbered"));
        assert!(cause.severity > 50);
    }

    #[test]
    fn outnumbered_loser_names_its_ratio() {
        let (winner, loser) = (f("fac_england"), f("fac_france"));
        let province = ProvinceId::new("prov_picardie").unwrap();
        let cause = battle_cause(&BattleFacts {
            winner: &winner,
            loser: &loser,
            province: &province,
            winner_strength: 4000,
            loser_strength: 1000,
            loser_losses: 400,
            winner_power: 0.0,
            loser_power: 0.0,
            loser_general_lost: false,
            loser_routed: false,
            loser_morale_delta: 0,
            winner_defended: false,
            rough_terrain: Some("hills"),
        });
        assert_eq!(cause.factors[0].key, "outnumbered");
        assert_eq!(cause.factors[0].value, 25);
        assert!(!keys(&cause).contains(&"terrain_hills"));
    }

    #[test]
    fn bankruptcy_ranks_the_heaviest_budget_line_first() {
        let faction = f("fac_france");
        let cause = bankruptcy_cause(&BankruptcyFacts {
            faction: &faction,
            treasury: -120,
            income: 400,
            army_upkeep: 600,
            army_surcharge: 100,
            building_upkeep: 100,
            administration: 100,
            table: 0,
            household: 0,
            wages: 0,
            siege: 0,
            obligations: 0,
            interest: 0,
            deficit_seasons: 3,
        });
        assert_eq!(cause.factors[0].key, "army_upkeep");
        assert!(keys(&cause).contains(&"deficit_seasons"));
        assert!(keys(&cause).contains(&"debt"));
        assert!(!keys(&cause).contains(&"table"));
    }

    #[test]
    fn unrest_keeps_positive_terms_only() {
        let lord = f("fac_france");
        let province = ProvinceId::new("prov_picardie").unwrap();
        let cause = unrest_cause(
            CauseKind::Revolt,
            &lord,
            &province,
            100,
            &[("tax", 12.0), ("garrison", -8.0), ("occupation", 5.4)],
        );
        assert_eq!(cause.factors[0].key, "unrest_tax");
        assert!(!keys(&cause).contains(&"unrest_garrison"));
    }

    #[test]
    fn war_names_the_broken_oath_and_the_reason() {
        let (a, t) = (f("fac_england"), f("fac_france"));
        let cause = war_cause(&WarFacts {
            attacker: &a,
            target: &t,
            casus_belli: false,
            truce_broken: true,
            pact_broken: false,
            sworn_bond: false,
            attacker_power: 200.0,
            target_power: 100.0,
            reason: Some("ambition"),
        });
        assert_eq!(cause.factors[0].key, "truce_broken");
        assert!(!keys(&cause).contains(&"no_casus_belli"));
        assert_eq!(cause.reason.as_ref().unwrap().key, "ambition");
    }

    #[test]
    fn succession_flags_a_minor_and_an_extinct_line() {
        let faction = f("fac_france");
        let cause = succession_cause(&SuccessionFacts {
            faction: &faction,
            dead_ruler_age: Some(63),
            designated_heir: false,
            contested: false,
            successor_age: Some(9),
            successor_minor: true,
            line_extinct: false,
            elective: false,
        });
        assert!(keys(&cause).contains(&"minor_successor"));
        assert!(keys(&cause).contains(&"no_designated_heir"));
        assert!(keys(&cause).contains(&"ruler_age"));
    }
}
