//! French tooltip texts of the top bar (port of `forecast_text.gd`, `income_lines.gd` and the
//! budget tooltip of `map_ui.gd`). Pure formatting of values computed by `game-api`.

use bevy::prelude::*;
use game_api::hud::{EconomyView, ForecastView, GaugeView, ProvinceIncome};

use crate::ui::kit::fr;

/// Number of detailed terms before « Autres ».
const TOP: usize = 5;

/// `#rrggbb` of a colour.
pub fn hex(color: Color) -> String {
    let c = color.to_srgba().to_u8_array();
    format!("#{:02x}{:02x}{:02x}", c[0], c[1], c[2])
}

/// Signed amount in its tone colour (BBCode).
pub fn tinted(value: i64) -> String {
    format!(
        "[color={}]{}[/color]",
        hex(fr::color_of(value)),
        fr::signed(value)
    )
}

/// `IncomeLines.LABELS`.
pub fn income_label(key: &str) -> &str {
    match key {
        "poll_peasants" => "Taille des paysans",
        "poll_burghers" => "Taille des bourgeois",
        "poll_clergy" => "Taille du clergé",
        "poll_nobility" => "Taille de la noblesse",
        "production" => "Production (moulins, ateliers)",
        "tax_rate" => "Taux d'imposition",
        "building_tax" => "Bâtiments (recettes)",
        "building_trade" => "Bâtiments (commerce)",
        "devastation" => "Dévastation",
        "full_province" => "Province entière",
        "embargo" => "Embargos",
        "domain" => "Domaine du seigneur",
        "difficulty" => "Difficulté",
        "alms" => "Aumônes de la croisade",
        "seigniorage" => "Seigneuriage",
        "trade_routes" => "Routes commerciales",
        other => other,
    }
}

/// `BudgetTable.RUBRICS`.
pub fn rubric_label(key: &str) -> &str {
    match key {
        "receipts" => "Impôts et commerce",
        "armies" => "Entretien des armées",
        "buildings" => "Entretien des bâtiments",
        "table" => "Table des provinces",
        "administration" => "Cour et administration",
        "other" => "Autres mouvements",
        other => other,
    }
}

/// Label of a forecast term without label of its own (`ForecastText.LABELS`).
fn term_label<'a>(key: &'a str, label: &'a str) -> &'a str {
    if !label.is_empty() {
        return label;
    }
    match key {
        "attrition" => "Disette et usure en campagne",
        "replenish" => "Renforts des armées au repos",
        "mercenaries" => "Désertion des compagnies impayées",
        "crusade" => "Croisade (ferveur, désertion)",
        "table" => "Table de la cour",
        "tech" => "Techniques (bilan de l'hiver)",
        "ruler" => "Souverain : traits et talents (hiver)",
        "retinue" => "Compagnons du souverain (hiver)",
        "inertia" => "Mécontentement actuel qui retombe",
        other => other,
    }
}

fn signed_f(value: f64, decimals: bool) -> String {
    let text = if decimals {
        format!("{:.1}", value.abs()).replace('.', ",")
    } else {
        fr::digits(value.abs().round() as i64)
    };
    let scaled = if decimals {
        value.abs() * 10.0
    } else {
        value.abs()
    };
    if scaled.round() == 0.0 {
        return "0".to_owned();
    }
    format!("{}{text}", if value > 0.0 { '+' } else { fr::MINUS })
}

fn tone(delta: f64, bad_when_up: bool) -> Color {
    if delta.abs() < 0.05 {
        fr::INK_COLOR
    } else if (delta > 0.0) == bad_when_up {
        fr::LOSS_COLOR
    } else {
        fr::GAIN_COLOR
    }
}

/// « ▲+3 », « ▼−2 », « = ».
pub fn arrow(gauge: &GaugeView) -> String {
    match gauge.delta {
        0 => "=".to_owned(),
        d if d > 0 => format!("▲+{}", fr::digits(d)),
        d => format!("▼{}{}", fr::MINUS, fr::digits(-d)),
    }
}

/// `ForecastText.tooltip`: « Actuel → Prochain tour » then the biggest terms.
pub fn gauge_tooltip(gauge: &GaugeView, unit: &str, bad_when_up: bool, decimals: bool) -> String {
    let mut lines = vec![format!(
        "[b]Actuel → Prochain tour :[/b] {}{unit} → {}{unit} ([color={}]{}{unit}[/color])",
        fr::digits(gauge.current),
        fr::digits(gauge.next),
        hex(tone(gauge.delta as f64, bad_when_up)),
        signed_f(gauge.delta as f64, false),
    )];
    let mut rest = gauge.other;
    for (index, term) in gauge.contributions.iter().enumerate() {
        if index >= TOP {
            rest += term.amount;
            continue;
        }
        lines.push(format!(
            "{} : [color={}]{}{unit}[/color]",
            term_label(&term.key, &term.label),
            hex(tone(term.amount, bad_when_up)),
            signed_f(term.amount, decimals),
        ));
    }
    if rest.abs() >= if decimals { 0.05 } else { 0.5 } {
        lines.push(format!(
            "Autres : [color={}]{}{unit}[/color]",
            hex(tone(rest, bad_when_up)),
            signed_f(rest, decimals),
        ));
    }
    lines.push(
        "[i]Prévision sans hasard : les événements imprévus et les décisions de l'IA n'y figurent pas.[/i]"
            .to_owned(),
    );
    lines.join("\n")
}

/// Forecast tooltips `(prestige, men, unrest)`; empty before the forecast exists.
pub fn forecast_tooltips(forecast: Option<&ForecastView>) -> [String; 3] {
    match forecast {
        Some(f) => [
            gauge_tooltip(&f.prestige, "", false, false),
            gauge_tooltip(&f.men, "", false, false),
            gauge_tooltip(&f.unrest, "%", true, true),
        ],
        None => Default::default(),
    }
}

/// `budget_tooltip` of `map_ui.gd`: signed budget rubrics and the change against last season.
pub fn budget_tooltip(economy: &EconomyView) -> String {
    let mut lines = vec!["[b]Solde prévu pour la prochaine saison[/b]".to_owned()];
    let has_past = economy.net_change.is_some();
    let mut biggest_key = "";
    let mut biggest = 0i64;
    for line in economy.budget_lines.iter().filter(|l| l.key != "other") {
        let mut text = format!("{} : {}", rubric_label(&line.key), tinted(line.projected));
        if let (true, Some(last)) = (has_past, line.last) {
            text.push_str(&format!(" (saison passée {})", fr::signed(last)));
        }
        lines.push(text);
        let delta = line.delta.unwrap_or(0);
        if has_past && delta.abs() > biggest.abs() {
            biggest = delta;
            biggest_key = &line.key;
        }
    }
    lines.push(format!("[b]Solde : {}[/b]", tinted(economy.net_income)));
    if let Some(change) = economy.net_change {
        let mut sentence = format!("{} par rapport à la saison passée", fr::signed(change));
        if change != 0 && !biggest_key.is_empty() && biggest != 0 {
            sentence.push_str(&format!(
                ", dont {} sur « {} »",
                fr::signed(biggest),
                rubric_label(biggest_key).to_lowercase()
            ));
        }
        lines.push(format!(
            "[color={}]{sentence}.[/color]",
            hex(fr::color_of(change))
        ));
    } else {
        lines.push("Premier tour : pas encore de saison passée.".to_owned());
    }
    lines.join("\n")
}

/// `ForecastText.income_drivers`: main receipts and the richest provinces.
pub fn income_drivers(economy: &EconomyView, provinces: &[ProvinceIncome]) -> String {
    let mut sorted: Vec<_> = economy.income_lines.iter().collect();
    sorted.sort_by_key(|l| std::cmp::Reverse(l.value.abs()));
    let mut out = Vec::new();
    if !sorted.is_empty() {
        let rows: Vec<String> = sorted
            .iter()
            .take(TOP)
            .map(|l| format!("{} : {}", income_label(&l.key), tinted(l.value)))
            .collect();
        out.push(format!("[b]Principales recettes[/b]\n{}", rows.join("\n")));
    }
    if !provinces.is_empty() {
        let rows: Vec<String> = provinces
            .iter()
            .take(3)
            .map(|p| format!("{} : {}", p.name, fr::signed(p.value)))
            .collect();
        out.push(format!(
            "[b]Provinces qui rapportent le plus[/b]\n{}",
            rows.join("\n")
        ));
    }
    out.join("\n")
}

/// Whole treasury tooltip: current → next season, budget, drivers.
pub fn treasury_tooltip(economy: &EconomyView, provinces: &[ProvinceIncome]) -> String {
    let mut parts = vec![format!(
        "[b]Actuel → Prochain tour :[/b] {} → {} ({})",
        fr::amount(economy.treasury),
        fr::amount(economy.treasury + economy.net_income),
        tinted(economy.net_income)
    )];
    parts.push(budget_tooltip(economy));
    let drivers = income_drivers(economy, provinces);
    if !drivers.is_empty() {
        parts.push(drivers);
    }
    parts.join("\n")
}

/// French season name from `TopBar::season`.
pub fn season_fr(season: &str) -> &'static str {
    match season {
        "spring" => "Printemps",
        "summer" => "Été",
        "autumn" => "Automne",
        "winter" => "Hiver",
        _ => "",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signed_floats() {
        assert_eq!(signed_f(0.01, false), "0");
        assert_eq!(signed_f(3.0, false), "+3");
        assert_eq!(signed_f(-1.4, true), format!("{}1,4", fr::MINUS));
    }

    #[test]
    fn hex_of_colour() {
        assert_eq!(hex(Color::srgb(1.0, 0.0, 0.0)), "#ff0000");
    }

    #[test]
    fn labels_fall_back_to_key() {
        assert_eq!(income_label("production"), "Production (moulins, ateliers)");
        assert_eq!(income_label("zzz"), "zzz");
    }
}
