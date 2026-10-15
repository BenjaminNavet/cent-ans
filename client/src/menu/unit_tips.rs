//! Tooltip of a unit type for the custom battle rosters (`TooltipHost.set_tooltip(button, "unit",
//! id)` in `custom_battle_screen.gd`): the figures of `data/unit_types/<id>.json` as the
//! rich tooltip of the kit shows them.

use data_model::{UnitCategory, UnitType};

use crate::ui::kit::fr::digits;
use crate::ui::kit::tooltip_spec::{Footer, Headline, StatRow, TooltipSpec, TooltipTexts};

fn category_label(category: UnitCategory) -> &'static str {
    match category {
        UnitCategory::Infantry => "Infanterie",
        UnitCategory::Ranged => "Tireurs",
        UnitCategory::Cavalry => "Cavalerie",
        UnitCategory::Siege => "Siège",
    }
}

fn row(key: &str, label: &str, value: String) -> StatRow {
    StatRow {
        key: key.to_owned(),
        label: label.to_owned(),
        value,
    }
}

/// Spec of unit type `unit` (name, category, size, cost, combat figures, description).
pub fn unit_type_spec(unit: &UnitType) -> TooltipSpec {
    let stats = &unit.stats;
    let mut rows = vec![
        row("melee", "Mêlée", stats.melee.to_string()),
        row("armor", "Armure", stats.armor.to_string()),
        row("morale", "Moral", stats.morale.to_string()),
        row("speed", "Vitesse", stats.speed.to_string()),
    ];
    if let Some(charge) = stats.charge {
        rows.insert(1, row("charge", "Charge", charge.to_string()));
    }
    if stats.ranged > 0 {
        rows.insert(1, row("ranged", "Tir", stats.ranged.to_string()));
        rows.push(row("range", "Portée", format!("{} m", stats.range)));
        rows.push(row("ammo", "Munitions", stats.ammo.to_string()));
    }
    if let Some(attack) = stats.siege_attack {
        rows.push(row("siege_attack", "Contre les murs", attack.to_string()));
    }
    TooltipSpec {
        id: unit.id.to_string(),
        kind: "unit".to_owned(),
        title: unit.name.display.clone(),
        subtitle: category_label(unit.category).to_owned(),
        icon: unit.id.to_string(),
        headline: vec![Headline {
            key: "soldiers".to_owned(),
            stat: String::new(),
            icon: String::new(),
            label: "Effectif".to_owned(),
            value: digits(i64::from(unit.soldiers)),
            sign: 0,
        }],
        stats: rows,
        flavour: unit.description.clone().unwrap_or_default(),
        footer: Footer {
            cost: format!("{} points", digits(i64::from(unit.cost.money))),
            ..Footer::default()
        },
        ..TooltipSpec::default()
    }
}

/// BBCode of the hover bubble of `unit`.
pub fn unit_type_bbcode(unit: &UnitType, texts: &TooltipTexts) -> String {
    unit_type_spec(unit).to_bbcode(texts, false)
}
