//! Builders of the journal of causes (ADR 0748): one pure function per [`CauseKind`], fed with
//! the facts the producers gather where the event happens (battle, siege, economy, revolt,
//! diplomacy, succession, end of the game). Keys only; the sentences are data.

use data_model::{FactionId, ProvinceId};

use crate::causes::{Cause, CauseKind};

fn pct(part: u64, whole: u64) -> i64 {
    if whole == 0 {
        0
    } else {
        (part.saturating_mul(100) / whole) as i64
    }
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
    pub deficit_seasons: u32,
}

/// Cause of an empty treasury: the budget lines ranked by their share of the upkeep.
pub fn bankruptcy_cause(f: &BankruptcyFacts<'_>) -> Cause {
    let upkeep = (f.army_upkeep + f.building_upkeep + f.administration + f.table).max(1);
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
