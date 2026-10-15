//! Infobulles du HUD de bataille : `TooltipSpec` d'un régiment (carte) et textes simples.
//!
//! Replaces `UnitCard._refresh_tooltip` (RichTooltip) of unit_card.gd: name, formation and state
//! in the header, effectif / moral / fatigue / munitions as headline figures, the numbers in the
//! detailed stats.

use super::view::{UnitView, number, state_text};
use crate::ui::kit::tooltip_spec::{Headline, StatRow, TooltipSpec, TooltipTexts};

fn headline(key: &str, icon: &str, label: &str, value: String, sign: i32) -> Headline {
    Headline {
        key: key.into(),
        stat: String::new(),
        icon: icon.into(),
        label: label.into(),
        value,
        sign,
    }
}

fn row(key: &str, label: &str, value: String) -> StatRow {
    StatRow {
        key: key.into(),
        label: label.into(),
        value,
    }
}

/// Spec of a regiment (`display_name` already distinct, « ★ » for the general).
pub fn unit_spec(unit: &UnitView, display_name: &str) -> TooltipSpec {
    let mut headline_figures = vec![
        headline(
            "soldiers",
            "",
            "Effectif",
            format!(
                "{} / {}",
                number(f64::from(unit.soldiers)),
                number(f64::from(unit.initial))
            ),
            0,
        ),
        headline(
            "morale",
            "",
            "Moral",
            number(f64::from(unit.morale)),
            if unit.morale < 35.0 {
                -1
            } else if unit.morale > 75.0 {
                1
            } else {
                0
            },
        ),
    ];
    if unit.fatigue >= 40.0 {
        headline_figures.push(headline(
            "fatigue",
            "",
            "Fatigue",
            number(f64::from(unit.fatigue)),
            -1,
        ));
    }
    let mut stats = vec![
        row("melee", "Mêlée", unit.melee.to_string()),
        row("armor", "Armure", unit.armor.to_string()),
        row("fatigue", "Fatigue", number(f64::from(unit.fatigue))),
        row("experience", "Expérience", unit.experience.to_string()),
        row("kills", "Ennemis abattus", unit.kills.to_string()),
    ];
    if unit.can_shoot {
        stats.insert(1, row("ranged", "Tir", unit.ranged.to_string()));
        stats.push(row(
            "ammo",
            "Munitions",
            format!("{} / {}", unit.ammo, unit.max_ammo),
        ));
    }
    TooltipSpec {
        id: format!("battle_unit_{}", unit.id),
        kind: "unit".into(),
        title: display_name.to_owned(),
        subtitle: format!("{} · {}", state_text(unit), unit.formation),
        icon: unit.unit_type.clone(),
        headline: headline_figures,
        stats,
        ..TooltipSpec::default()
    }
}

/// BBCode of the hover bubble of a regiment.
pub fn unit_bbcode(unit: &UnitView, display_name: &str, texts: &TooltipTexts) -> String {
    unit_spec(unit, display_name).to_bbcode(texts, false)
}

/// Tooltip of a speed button.
pub fn speed_tip(key: &str) -> &'static str {
    match key {
        "pause" => "[b]Pause[/b]\nEspace. Les ordres restent possibles en pause.",
        "0.5" => "[b]Ralenti ×0,5[/b]\nTouches + / −.",
        "1" => "[b]Vitesse ×1[/b]\nTouches + / −.",
        "2" => "[b]Vitesse ×2[/b]\nTouches + / −.",
        _ => "[b]Vitesse ×4[/b]\nTouches + / −.",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::battle::hud::view::{BattleRow, UnitView};
    use data_model::UnitCategory;
    use sim_battle::{UnitFate, UnitState};

    fn archer() -> UnitView {
        UnitView {
            id: 7,
            unit_type: "unit_francs_archers".into(),
            name: "Archers".into(),
            category: UnitCategory::Ranged,
            soldiers: 90,
            initial: 100,
            morale: 30.0,
            fatigue: 50.0,
            ammo: 12,
            max_ammo: 24,
            can_shoot: true,
            is_general: false,
            present: true,
            left_field: false,
            reserve: false,
            state: UnitState::Shooting,
            formation: "Ligne".into(),
            experience: 1,
            kills: 4,
            fate: UnitFate::Held,
            melee: 3,
            ranged: 8,
            armor: 2,
            x: 0.0,
            z: 0.0,
            row: BattleRow::Main,
        }
    }

    #[test]
    fn archer_spec_has_ammo_and_warnings() {
        let spec = unit_spec(&archer(), "Archers II");
        assert_eq!(spec.title, "Archers II");
        assert!(spec.subtitle.contains("tire"));
        assert!(
            spec.stats
                .iter()
                .any(|s| s.key == "ammo" && s.value == "12 / 24")
        );
        assert!(spec.headline.iter().any(|h| h.key == "fatigue"));
        let moral = spec.headline.iter().find(|h| h.key == "morale").unwrap();
        assert_eq!(moral.sign, -1);
    }

    #[test]
    fn bbcode_mentions_name() {
        let text = unit_bbcode(&archer(), "Archers II", &TooltipTexts::default());
        assert!(text.contains("Archers II"));
    }
}
