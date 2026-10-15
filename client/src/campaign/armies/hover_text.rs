//! Text of the rich hover bubbles of the map (army, colony, army lost from sight). Port of
//! game/scripts/map/map_hover_text.gd: pure functions over plain inputs, no game rule and no
//! read outside what the caller says is visible (`visible`, `is_player`). The ambush of an enemy
//! army is never shown. Output is BBCode for `ui::kit::rich_text`.

use data_model::UnitCategory;

use super::markers::plate_text;
use crate::ui::kit::fr;

/// Category label of a regiment group (`CATEGORY_NAMES`).
pub fn category_name(category: UnitCategory) -> &'static str {
    match category {
        UnitCategory::Infantry => "fantassins",
        UnitCategory::Ranged => "archers",
        UnitCategory::Cavalry => "cavaliers",
        UnitCategory::Siege => "engins",
    }
}

const CATEGORY_ORDER: [UnitCategory; 4] = [
    UnitCategory::Infantry,
    UnitCategory::Ranged,
    UnitCategory::Cavalry,
    UnitCategory::Siege,
];

/// Men per category in display order (`composition`); empty categories are left out.
pub fn composition(units: &[(UnitCategory, u32)]) -> Vec<(UnitCategory, u32)> {
    CATEGORY_ORDER
        .iter()
        .filter_map(|category| {
            let total: u32 = units
                .iter()
                .filter(|(c, _)| c == category)
                .map(|(_, men)| *men)
                .sum();
            units
                .iter()
                .any(|(c, _)| c == category)
                .then_some((*category, total))
        })
        .collect()
}

/// French label of a stance key worth showing (`visible_stance`): never « normal », never the
/// ambush of an enemy.
pub fn visible_stance(stance: &str, is_player: bool) -> Option<&'static str> {
    match stance {
        "raid" => Some("Chevauchée"),
        "siege" => Some("Siège"),
        "ambush" if is_player => Some("Embuscade"),
        "forced_march" => Some("Marche forcée"),
        "entrenched" => Some("Retranchée"),
        _ => None,
    }
}

/// Everything the army bubble reads.
#[derive(Debug, Clone, Default)]
pub struct ArmyBubble {
    pub general_name: String,
    pub faction_name: String,
    pub liege_name: Option<String>,
    pub men: u32,
    pub composition: Vec<(UnitCategory, u32)>,
    /// `Stance::key()`.
    pub stance: String,
    pub is_player: bool,
    /// Sailing a voyage.
    pub embarked: bool,
    /// A march is planned (several turns) or the army has a destination.
    pub moving: bool,
    pub destination_name: Option<String>,
    pub supply: u8,
    /// `Some(n)`: army lost from sight, last known situation `n` seasons ago.
    pub seen_ago: Option<u32>,
}

fn state_line(bubble: &ArmyBubble) -> Option<String> {
    if bubble.embarked {
        return Some("À bord".into());
    }
    if bubble.stance == "siege" {
        return Some("Met le siège".into());
    }
    Some(if bubble.moving {
        match &bubble.destination_name {
            Some(name) => format!("En marche vers {name}"),
            None => "En marche".into(),
        }
    } else {
        "Au repos".into()
    })
}

/// Bubble of an army (`army_text`).
pub fn army_text(bubble: &ArmyBubble) -> String {
    let general = bubble.general_name.as_str();
    let mut title = if general.is_empty() {
        format!("Ost de {}", bubble.faction_name)
    } else {
        format!("Ost de {general}")
    };
    if bubble.seen_ago.is_some() {
        title = format!("Dernière position connue — {title}");
    }
    let mut lines = vec![format!("[b]{title}[/b]")];
    let mut faction_line = bubble.faction_name.clone();
    if let Some(liege) = bubble.liege_name.as_deref().filter(|l| !l.is_empty()) {
        faction_line.push_str(&format!(" (vassal de {liege})"));
    }
    if !general.is_empty() && !faction_line.is_empty() {
        lines.push(faction_line);
    }
    lines.push(format!("{} hommes", plate_text(bubble.men, "")));
    if let Some(ago) = bubble.seen_ago {
        lines.push(if ago == 0 {
            "[i]Vue ce tour-ci[/i]".into()
        } else {
            format!(
                "[i]Vue il y a {}[/i]",
                fr::count(i64::from(ago), "saison", None)
            )
        });
        return lines.join("\n");
    }
    if !bubble.composition.is_empty() {
        let parts: Vec<String> = bubble
            .composition
            .iter()
            .map(|(category, men)| format!("{} {}", plate_text(*men, ""), category_name(*category)))
            .collect();
        lines.push(parts.join(" · "));
    }
    if let Some(stance) = visible_stance(&bubble.stance, bubble.is_player) {
        lines.push(format!("Posture : {stance}"));
    }
    if let Some(state) = state_line(bubble) {
        lines.push(state);
    }
    if bubble.is_player {
        lines.push(format!("Vivres : {}", bubble.supply));
    }
    lines.join("\n")
}

/// Walls of a colony from its buildings and fortification (`wall_text`).
pub fn wall_text(buildings: &[String], fortification: u32) -> String {
    for (id, name) in [
        ("bld_stone_walls", "murailles de pierre"),
        ("bld_walls", "murailles"),
        ("bld_palisade", "palissade"),
    ] {
        if buildings.iter().any(|b| b == id) {
            return name.into();
        }
    }
    if fortification >= 2 {
        format!("enceinte (fortification {fortification})")
    } else {
        "sans enceinte".into()
    }
}

/// Siege of a colony as the bubble reads it.
#[derive(Debug, Clone, Default)]
pub struct SiegeLine {
    pub attacker_name: String,
    pub turns_elapsed: u32,
    pub supplies: u8,
    pub breach: u8,
}

/// Everything the colony bubble reads.
#[derive(Debug, Clone, Default)]
pub struct SettlementBubble {
    pub name: String,
    pub owner_name: String,
    pub controller_name: String,
    /// The province is in sight or ours: the rest is shown.
    pub visible: bool,
    pub is_player: bool,
    /// Visual level (0 village .. 3 city), `None` when unknown.
    pub level: Option<usize>,
    pub buildings: Vec<String>,
    pub fortification: u32,
    pub siege: Option<SiegeLine>,
    pub garrison_units: usize,
    pub garrison_men: u32,
}

const LEVEL_TITLES: [&str; 4] = ["Village", "Bourg", "Ville", "Cité"];

fn capitalised(text: &str) -> String {
    let mut chars = text.chars();
    chars.next().map_or_else(String::new, |first| {
        first.to_uppercase().chain(chars).collect()
    })
}

/// Bubble of a colony (`settlement_text`); out of sight: name and owner only.
pub fn settlement_text(bubble: &SettlementBubble) -> String {
    let mut lines = vec![
        format!("[b]{}[/b]", bubble.name),
        format!("Propriétaire : {}", bubble.owner_name),
    ];
    if !bubble.controller_name.is_empty() && bubble.controller_name != bubble.owner_name {
        lines.push(format!("Occupée par {}", bubble.controller_name));
    }
    if !bubble.visible {
        return lines.join("\n");
    }
    let walls = wall_text(&bubble.buildings, bubble.fortification);
    lines.push(match bubble.level.and_then(|l| LEVEL_TITLES.get(l)) {
        Some(title) => format!("{title} · {walls}"),
        None => capitalised(&walls),
    });
    if let Some(siege) = &bubble.siege {
        lines.push(format!(
            "[color=#8f1510]Assiégée par {} depuis {}[/color]",
            siege.attacker_name,
            fr::count(i64::from(siege.turns_elapsed), "tour", None)
        ));
        lines.push(format!(
            "Vivres de la place : {} · brèche : {}",
            siege.supplies, siege.breach
        ));
    }
    if bubble.is_player || bubble.garrison_units > 0 {
        lines.push(if bubble.garrison_men > 0 {
            format!("Garnison : {} hommes", plate_text(bubble.garrison_men, ""))
        } else {
            "Garnison : aucune".into()
        });
    }
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bubble() -> ArmyBubble {
        ArmyBubble {
            general_name: "Philippe".into(),
            faction_name: "France".into(),
            liege_name: Some("Le Roi".into()),
            men: 1500,
            composition: composition(&[
                (UnitCategory::Cavalry, 300),
                (UnitCategory::Infantry, 900),
                (UnitCategory::Infantry, 100),
            ]),
            stance: "normal".into(),
            is_player: true,
            supply: 80,
            ..Default::default()
        }
    }

    #[test]
    fn composition_is_ordered_and_summed() {
        let rows = composition(&[
            (UnitCategory::Cavalry, 300),
            (UnitCategory::Infantry, 900),
            (UnitCategory::Infantry, 100),
        ]);
        assert_eq!(
            rows,
            vec![(UnitCategory::Infantry, 1000), (UnitCategory::Cavalry, 300)]
        );
    }

    #[test]
    fn own_army_shows_supply_and_rest() {
        let text = army_text(&bubble());
        assert!(text.starts_with("[b]Ost de Philippe[/b]\nFrance (vassal de Le Roi)\n"));
        assert!(text.contains("1\u{202f}000 fantassins · 300 cavaliers"));
        assert!(text.ends_with("Au repos\nVivres : 80"));
    }

    #[test]
    fn enemy_ambush_is_hidden_but_not_ours() {
        let mut b = bubble();
        b.stance = "ambush".into();
        assert!(army_text(&b).contains("Posture : Embuscade"));
        b.is_player = false;
        assert!(!army_text(&b).contains("Posture"));
        assert!(!army_text(&b).contains("Vivres"));
    }

    #[test]
    fn marching_and_siege_lines() {
        let mut b = bubble();
        b.moving = true;
        b.destination_name = Some("Paris".into());
        assert!(army_text(&b).contains("En marche vers Paris"));
        b.stance = "siege".into();
        assert!(army_text(&b).contains("Met le siège"));
        b.embarked = true;
        assert!(army_text(&b).contains("À bord"));
    }

    #[test]
    fn ghost_has_no_detail() {
        let mut b = bubble();
        b.seen_ago = Some(2);
        let text = army_text(&b);
        assert!(text.contains("Dernière position connue — Ost de Philippe"));
        assert!(text.contains("Vue il y a 2 saisons"));
        assert!(!text.contains("fantassins"));
        b.seen_ago = Some(0);
        assert!(army_text(&b).contains("Vue ce tour-ci"));
    }

    #[test]
    fn settlement_out_of_sight_is_short() {
        let b = SettlementBubble {
            name: "Rouen".into(),
            owner_name: "France".into(),
            controller_name: "Angleterre".into(),
            ..Default::default()
        };
        assert_eq!(
            settlement_text(&b),
            "[b]Rouen[/b]\nPropriétaire : France\nOccupée par Angleterre"
        );
    }

    #[test]
    fn settlement_in_sight_shows_walls_siege_and_garrison() {
        let b = SettlementBubble {
            name: "Rouen".into(),
            owner_name: "France".into(),
            controller_name: "France".into(),
            visible: true,
            is_player: true,
            level: Some(2),
            buildings: vec!["bld_stone_walls".into()],
            siege: Some(SiegeLine {
                attacker_name: "Angleterre".into(),
                turns_elapsed: 3,
                supplies: 40,
                breach: 10,
            }),
            garrison_units: 2,
            garrison_men: 800,
            ..Default::default()
        };
        let text = settlement_text(&b);
        assert!(text.contains("Ville · murailles de pierre"));
        assert!(text.contains("Assiégée par Angleterre depuis 3 tours"));
        assert!(text.contains("Vivres de la place : 40 · brèche : 10"));
        assert!(text.ends_with("Garnison : 800 hommes"));
        let open = SettlementBubble {
            visible: true,
            fortification: 3,
            ..Default::default()
        };
        assert!(settlement_text(&open).contains("Enceinte (fortification 3)"));
    }
}
