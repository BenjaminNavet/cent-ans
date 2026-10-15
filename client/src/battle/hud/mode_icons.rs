//! Unit modes of the cards and the help: key, French label, tooltip with the numbers of the core
//! rules. Ports game/scripts/battle/battle_mode_icons.gd (`MODES`, `field_of`, `label_of`,
//! `tip_of`, `active_modes`). The drawn fallback glyphs of Godot (`draw_mode`, `draw_state`) are
//! not ported: the card badges draw the ink icon `battle_mode_<mode>` / `battle_state_<kind>`
//! through the UI kit icon table and fall back to a text glyph ([`super::view::badge_glyph`]),
//! see [`super::cards::badge_icon_id`].

use sim_battle::UnitModeRules;

/// One mode, in the order of the buttons.
pub struct ModeInfo {
    pub mode: &'static str,
    pub label: &'static str,
    /// Tooltip template; `{name}` placeholders are filled by [`rule_value`].
    pub tip: &'static str,
}

pub const MODES: [ModeInfo; 5] = [
    ModeInfo {
        mode: "run",
        label: "Course",
        tip: "Tous les déplacements au pas de course (le double clic droit fait courir un seul ordre).",
    },
    ModeInfo {
        mode: "guard",
        label: "Garde",
        tip: "Tenir sa position : pas de poursuite d’un ennemi en fuite ni d’un adversaire qui rompt le contact.",
    },
    ModeInfo {
        mode: "skirmish",
        label: "Escarmouche",
        tip: "Les tireurs reculent de {unit_modes_skirmish_retreat_m} m devant une troupe de mêlée qui approche à moins de {unit_modes_skirmish_trigger_m} m.",
    },
    ModeInfo {
        mode: "melee",
        label: "Mêlée",
        tip: "Les tireurs rangent leurs armes de trait et engagent au corps à corps.",
    },
    ModeInfo {
        mode: "breach",
        label: "Battre en brèche",
        tip: "Engins de siège : tir concentré sur les murs et les portes (+{unit_modes_breach_damage_percent} % de dégâts, mangonneau +{unit_modes_breach_mangonel_percent} %, recharge +{unit_modes_breach_reload_percent} %), jamais sur les hommes.",
    },
];

/// The numbers `RuleValues` serves in Godot (`data_store_rules.rs`), read from the core rules.
pub fn rule_value(key: &str) -> Option<f64> {
    let rules = UnitModeRules::bundled();
    Some(match key {
        "unit_modes_skirmish_trigger_m" => rules.skirmish.trigger_m,
        "unit_modes_skirmish_retreat_m" => rules.skirmish.retreat_m,
        "unit_modes_breach_damage_percent" => (rules.breach.wall_damage("default") - 1.0) * 100.0,
        "unit_modes_breach_mangonel_percent" => {
            (rules.breach.wall_damage("unit_mangonel") - 1.0) * 100.0
        }
        "unit_modes_breach_reload_percent" => (rules.breach.reload_multiplier - 1.0) * 100.0,
        _ => return None,
    })
}

fn quote(value: f64) -> String {
    if (value - value.round()).abs() < 0.05 {
        format!("{}", value.round() as i64)
    } else {
        format!("{value:.1}").replace('.', ",")
    }
}

/// Label of `mode` (the key itself when unknown).
pub fn label_of(mode: &str) -> &str {
    MODES
        .iter()
        .find(|m| m.mode == mode)
        .map_or(mode, |m| m.label)
}

/// Tooltip of `mode` with the rule numbers filled in; empty when unknown.
pub fn tip_of(mode: &str) -> String {
    let Some(info) = MODES.iter().find(|m| m.mode == mode) else {
        return String::new();
    };
    crate::ui::kit::tooltip_spec::fill_rules(&info.tip.replace("{unit_", "{rule.unit_"), |key| {
        rule_value(key).map(quote)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_and_order() {
        assert_eq!(label_of("breach"), "Battre en brèche");
        assert_eq!(label_of("zzz"), "zzz");
        assert_eq!(
            MODES.map(|m| m.mode),
            ["run", "guard", "skirmish", "melee", "breach"]
        );
    }

    #[test]
    fn tips_have_no_placeholder_left() {
        for info in &MODES {
            let tip = tip_of(info.mode);
            assert!(!tip.is_empty() && !tip.contains('{'), "{tip}");
        }
        assert!(tip_of("skirmish").contains(" m "));
    }

    #[test]
    fn modes_match_the_status_view() {
        // The order of the buttons is the order `status_view` lists the switched-on modes in.
        assert_eq!(MODES[0].mode, "run");
        assert_eq!(MODES[4].mode, "breach");
    }
}
