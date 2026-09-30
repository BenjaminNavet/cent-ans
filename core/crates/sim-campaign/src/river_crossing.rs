//! Battles at a river crossing of the campaign map (chantier RC, ADR 0117).
//!
//! A field battle becomes a *crossing battle* when a bridge, ford or ferry of
//! `data/map/crossings_px.json` lies within `search_radius_km` of the fight
//! (the defender's position, or the midpoint of both armies) and the two
//! armies stand on opposite banks: opposite sides of the straight river line
//! through the crossing along its `dir`. The structure then sets the
//! attacker's damage and the defender's ranged damage
//! (`data/rules/river_crossings.json`), instead of the province river flag.

use data_model::{CrossingFactors, GameData, MapCrossing};
use sim_battle::{BattleCrossing, CrossingStructure};

use crate::march::px_per_km;
use crate::state::{ArmyId, CampaignState};

/// Below this distance to the river line (map pixels) a position counts as
/// on neither bank.
const BANK_EPSILON_PX: f32 = 1e-3;

/// The crossing a battle is fought at.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrossingSite {
    pub structure: CrossingStructure,
    /// Name of the crossing, empty for an anonymous road bridge.
    pub name: String,
    /// French name of the river, empty when unknown.
    pub river_display: String,
}

impl CrossingSite {
    /// The `BattleSetup.crossing` of a tactical battle fought here.
    pub fn battle_crossing(&self) -> BattleCrossing {
        BattleCrossing {
            structure: self.structure,
            name: self.name.clone(),
            river: self.river_display.clone(),
        }
    }
}

/// Structure of a map crossing: `ford`+`ford` is a ford, `ferry` a ferry,
/// `boats` a boat bridge, `stone` a stone bridge, anything else wood.
pub fn structure_of(crossing: &MapCrossing) -> CrossingStructure {
    match crossing.structure.as_str() {
        "ford" if crossing.kind == "ford" => CrossingStructure::Ford,
        "ferry" => CrossingStructure::Ferry,
        "boats" => CrossingStructure::BoatBridge,
        "stone" => CrossingStructure::StoneBridge,
        _ => CrossingStructure::WoodBridge,
    }
}

/// The coefficient of `structure` in `factors`.
pub fn factor(factors: &CrossingFactors, structure: CrossingStructure) -> f64 {
    match structure {
        CrossingStructure::StoneBridge => factors.stone_bridge,
        CrossingStructure::WoodBridge => factors.wood_bridge,
        CrossingStructure::BoatBridge => factors.boat_bridge,
        CrossingStructure::Ferry => factors.ferry,
        CrossingStructure::Ford => factors.ford,
    }
}

fn distance(a: [f32; 2], b: [f32; 2]) -> f32 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2)).sqrt()
}

/// Signed distance of `point` to the river line of `crossing` (sign = bank).
fn bank_side(crossing: &MapCrossing, point: [f32; 2]) -> f32 {
    let [dx, dy] = crossing.dir;
    let len = (dx * dx + dy * dy).sqrt();
    if len <= f32::EPSILON {
        return 0.0;
    }
    let (rx, ry) = (point[0] - crossing.px[0], point[1] - crossing.px[1]);
    (dx * ry - dy * rx) / len
}

/// Do `a` and `b` stand on opposite banks at `crossing`?
fn opposite_banks(crossing: &MapCrossing, a: [f32; 2], b: [f32; 2]) -> bool {
    let (sa, sb) = (bank_side(crossing, a), bank_side(crossing, b));
    sa.abs() > BANK_EPSILON_PX && sb.abs() > BANK_EPSILON_PX && (sa > 0.0) != (sb > 0.0)
}

/// The crossing of a battle between an attacker at map pixel `attacker` and
/// a defender at `defender` (see the module docs); the nearest one wins.
pub fn crossing_between(
    data: &GameData,
    attacker: [f32; 2],
    defender: [f32; 2],
) -> Option<CrossingSite> {
    if data.crossings.is_empty() {
        return None;
    }
    let radius = data.river_crossing_rules.search_radius_km as f32 * px_per_km(data);
    let middle = [
        (attacker[0] + defender[0]) / 2.0,
        (attacker[1] + defender[1]) / 2.0,
    ];
    data.crossings
        .iter()
        .filter_map(|c| {
            let d = distance(c.px, defender).min(distance(c.px, middle));
            (d <= radius && opposite_banks(c, attacker, defender)).then_some((d, c))
        })
        .min_by(|a, b| a.0.total_cmp(&b.0).then_with(|| a.1.id.cmp(&b.1.id)))
        .map(|(_, c)| CrossingSite {
            structure: structure_of(c),
            name: c.name.clone(),
            river_display: data.river_display_name(&c.river).to_owned(),
        })
}

/// The crossing of a field battle between the lead armies `attacker` and
/// `defender`, when they fight across a river near a bridge, ford or ferry.
pub fn crossing_site(
    state: &CampaignState,
    data: &GameData,
    attacker: &ArmyId,
    defender: &ArmyId,
) -> Option<CrossingSite> {
    if data.crossings.is_empty() {
        return None;
    }
    let a = state.armies.get(attacker)?;
    let d = state.armies.get(defender)?;
    crossing_between(data, state.army_point(data, a), state.army_point(data, d))
}

fn starts_with_vowel(word: &str) -> bool {
    word.chars().next().is_some_and(|c| {
        "AEIOUYHÀÂÄÉÈÊËÎÏÔÖÙÛÜaeiouyhàâäéèêëîïôöùûü".contains(c)
    })
}

/// « de X » with the French contractions (« du pont de Blois », « des ponts
/// de Nantes », « d'Anvers »).
fn of_name(name: &str) -> String {
    for (prefix, replacement) in [
        ("Pont ", "du pont "),
        ("Ponts ", "des ponts "),
        ("Bac ", "du bac "),
        ("Gué ", "du gué "),
        ("Les ", "des "),
        ("Le ", "du "),
        ("La ", "de la "),
    ] {
        if let Some(rest) = name.strip_prefix(prefix) {
            return format!("{replacement}{rest}");
        }
    }
    if name.starts_with("Pont-") || name.starts_with("Grand-") || name.starts_with("Grandpont") {
        return format!("du {name}");
    }
    if starts_with_vowel(name) {
        format!("d'{name}")
    } else {
        format!("de {name}")
    }
}

/// The river with its article (« la Loire », « le Rhône », « l'Escaut »);
/// a guess from the first word: feminine when it ends in -e or -a, save a
/// few masculine rivers.
fn river_with_article(river: &str) -> String {
    const MASCULINE: [&str; 8] = [
        "Danube", "Tage", "Tibre", "Rhône", "Tigre", "Gave", "Canal", "Bras",
    ];
    if river.is_empty() {
        return "une rivière".to_owned();
    }
    if starts_with_vowel(river) {
        return format!("l'{river}");
    }
    let first = river.split([' ', '-']).next().unwrap_or(river);
    let feminine = !MASCULINE.contains(&first) && (first.ends_with('e') || first.ends_with('a'));
    if feminine {
        format!("la {river}")
    } else {
        format!("le {river}")
    }
}

/// Signed percentage of a coefficient (0.6 → « −40 % », 1.25 → « +25 % »).
fn percent(coefficient: f64) -> String {
    let p = ((coefficient - 1.0) * 100.0).round() as i64;
    if p < 0 {
        format!("−{} %", -p)
    } else {
        format!("+{p} %")
    }
}

/// The forecast line of a crossing battle, in French: « Passage en force du
/// pont des Tourelles (Orléans) sur la Loire (−40 %, tireurs du défenseur
/// +25 %) ».
pub fn forecast_line(data: &GameData, site: &CrossingSite) -> String {
    let rules = &data.river_crossing_rules;
    let attacker = percent(factor(&rules.attacker_factor, site.structure));
    let ranged = percent(factor(&rules.defender_ranged_factor, site.structure));
    let river = river_with_article(&site.river_display);
    let head = if !site.name.is_empty() {
        format!("Passage en force {} sur {river}", of_name(&site.name))
    } else {
        match site.structure {
            CrossingStructure::StoneBridge => {
                format!("Passage en force d'un pont de pierre sur {river}")
            }
            CrossingStructure::WoodBridge => {
                format!("Passage en force d'un pont de bois sur {river}")
            }
            CrossingStructure::BoatBridge => {
                format!("Passage en force d'un pont de bateaux sur {river}")
            }
            CrossingStructure::Ferry => format!("Passage d'un bac sur {river}"),
            CrossingStructure::Ford => {
                let of_river = if river.starts_with("l'") || river.starts_with("une ") {
                    format!("d'{}", river.trim_start_matches("l'"))
                } else if let Some(rest) = river.strip_prefix("le ") {
                    format!("du {rest}")
                } else {
                    format!("de {river}")
                };
                format!("Passage à gué {of_river}")
            }
        }
    };
    format!("{head} ({attacker}, tireurs du défenseur {ranged})")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn crossing(structure: &str, kind: &str) -> MapCrossing {
        MapCrossing {
            id: "c".into(),
            name: String::new(),
            kind: kind.into(),
            structure: structure.into(),
            river: String::new(),
            px: [100.0, 100.0],
            dir: [1.0, 0.0],
        }
    }

    #[test]
    fn structures_follow_the_map_fields() {
        assert_eq!(
            structure_of(&crossing("ford", "ford")),
            CrossingStructure::Ford
        );
        assert_eq!(
            structure_of(&crossing("ferry", "ford")),
            CrossingStructure::Ferry
        );
        assert_eq!(
            structure_of(&crossing("boats", "bridge")),
            CrossingStructure::BoatBridge
        );
        assert_eq!(
            structure_of(&crossing("stone", "road")),
            CrossingStructure::StoneBridge
        );
        assert_eq!(
            structure_of(&crossing("wood", "road")),
            CrossingStructure::WoodBridge
        );
    }

    #[test]
    fn banks_are_the_sides_of_the_river_line() {
        let c = crossing("stone", "bridge");
        assert!(opposite_banks(&c, [100.0, 90.0], [100.0, 110.0]));
        assert!(!opposite_banks(&c, [90.0, 90.0], [110.0, 95.0]));
        assert!(!opposite_banks(&c, [90.0, 100.0], [110.0, 110.0]));
    }

    #[test]
    fn french_names() {
        assert_eq!(
            of_name("Pont des Tourelles (Orléans)"),
            "du pont des Tourelles (Orléans)"
        );
        assert_eq!(of_name("Ponts de Nantes"), "des ponts de Nantes");
        assert_eq!(of_name("Bac d'Anvers"), "du bac d'Anvers");
        assert_eq!(of_name("London Bridge"), "de London Bridge");
        assert_eq!(river_with_article("Loire"), "la Loire");
        assert_eq!(river_with_article("Rhône"), "le Rhône");
        assert_eq!(river_with_article("Rhin"), "le Rhin");
        assert_eq!(river_with_article("Escaut"), "l'Escaut");
        assert_eq!(river_with_article("Volga"), "la Volga");
        assert_eq!(percent(0.6), "−40 %");
        assert_eq!(percent(1.25), "+25 %");
    }
}
