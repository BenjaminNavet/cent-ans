//! French tooltip texts of the top bar (port of `forecast_text.gd`, `income_lines.gd` and the
//! budget tooltip of `map_ui.gd`). Pure formatting of values computed by `game-api`.

use bevy::prelude::*;
use game_api::hud::{
    EconomyView, FigureTrend, ForecastView, GaugeView, ProvinceIncome, RecruitReserveView,
};

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
        "household" => "Maison du roi",
        "wages" => "Gages des officiers",
        "siege" => "Coût des sièges",
        "obligations" => "Dot, tribut et saisies",
        "interest" => "Intérêts de la dette",
        "levies" => "Impôts exceptionnels",
        "loans" => "Emprunts et remboursements",
        "other" => "Autres mouvements",
        other => other,
    }
}

/// XP-POURQUOI: the trend of a figure of the top bar since last season, as it opens the
/// tooltip: the change, then the heaviest terms (three at most, `game-api` closes their sum with
/// « Autres éléments »). Empty before the first season is resolved.
pub fn trend_tooltip(trend: Option<&FigureTrend>, money: bool) -> String {
    let Some((trend, delta)) = trend.and_then(|t| t.delta_since_last_season.map(|d| (t, d))) else {
        return String::new();
    };
    let show = |value: i64| {
        if money {
            fr::signed(value)
        } else if value == 0 {
            "0".to_owned()
        } else {
            format!("{}{}", if value > 0 { "+" } else { "" }, fr::digits(value))
        }
    };
    let ui = game_api::causes::ui_text;
    let mut lines = vec![format!(
        "[b]Depuis la saison passée :[/b] [color={}]{}[/color]",
        hex(fr::color_of(delta)),
        show(delta)
    )];
    if !trend.top_terms.is_empty() {
        lines.push(
            ui(if trend.terms_last_season {
                "terms_last_season"
            } else {
                "terms_next_season"
            }),
        );
        for term in &trend.top_terms {
            lines.push(format!(
                "{} : [color={}]{}[/color]",
                term.label,
                hex(fr::color_of(term.amount)),
                show(term.amount)
            ));
        }
    }
    lines.join("\n") + "\n"
}

/// « ▲+1 200 » / « ▼−2 140 » after a figure: its change since last season (empty at zero or
/// before the first season is resolved).
pub fn past_arrow(trend: Option<&FigureTrend>) -> String {
    match trend.and_then(|t| t.delta_since_last_season) {
        None | Some(0) => String::new(),
        Some(d) if d > 0 => format!(" ▲+{}", fr::digits(d)),
        Some(d) => format!(" ▼{}{}", fr::MINUS, fr::digits(-d)),
    }
}

/// The trend of figure `key` (`treasury`, `net_income`, `men`, `prestige`, `unrest`).
pub fn trend_of<'a>(top: &'a game_api::TopBar, key: &str) -> Option<&'a FigureTrend> {
    top.trends.iter().find(|t| t.figure == key)
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
    for line in economy
        .budget_lines
        .iter()
        .filter(|l| !matches!(l.key.as_str(), "other" | "levies" | "loans"))
    {
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

/// One line on the recruitment reserve (recruitment panel header): what the realm can raise now.
pub fn reserve_line(reserve: &RecruitReserveView) -> String {
    format!(
        "Réserve de recrues : {} sur {} · {} sur {} · +{} la saison prochaine",
        fr::count(i64::from(reserve.available), "unité", None),
        reserve.cap,
        fr::count(
            i64::from(reserve.ready_settlements),
            "place prête",
            Some("places prêtes")
        ),
        reserve.settlements,
        reserve.gain_next_season,
    )
}

/// Tooltip paragraph on the recruitment reserve (the real bottleneck of raising troops, not the
/// purse, ADR 0793). Appended to the « Hommes » chip of the top bar.
pub fn reserve_tooltip(reserve: &RecruitReserveView) -> String {
    let verdict = if reserve.available == 0 {
        "Aucune levée possible avant la prochaine saison."
    } else {
        "Ouvrez le recrutement pour les lever."
    };
    format!(
        "[b]Réserve de recrues[/b]\n{} levables aujourd’hui, pour un plafond de {}.\n{} sur {} de vos places ont au moins une unité prête ; {} de plus la saison prochaine.\nC’est cette réserve, non la bourse, qui borne les levées : chaque place la reconstitue peu à peu. {verdict}",
        fr::count(i64::from(reserve.available), "unité", None),
        reserve.cap,
        reserve.ready_settlements,
        reserve.settlements,
        fr::count(i64::from(reserve.gain_next_season), "unité", None),
    )
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
    fn reserve_texts_state_the_figures() {
        let reserve = RecruitReserveView {
            settlements: 9,
            available: 14,
            cap: 32,
            ready_settlements: 6,
            gain_next_season: 5,
        };
        let tip = reserve_tooltip(&reserve);
        assert!(tip.contains("14 unités"), "{tip}");
        assert!(tip.contains("plafond de 32"));
        assert!(tip.contains("6 sur 9"));
        assert!(tip.contains("non la bourse"));
        let line = reserve_line(&reserve);
        assert!(
            line.contains("14 unités sur 32") && line.contains("6 places prêtes sur 9"),
            "{line}"
        );
        let empty = RecruitReserveView {
            available: 0,
            ..reserve
        };
        assert!(reserve_tooltip(&empty).contains("Aucune levée possible"));
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
