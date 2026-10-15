//! Pure helpers of the diplomacy screen: filters, sorts, labels, BBCode of reasons. Tested below.

use bevy::prelude::Color;
use game_api::diplomacy::{FactionRow, ReasonView, VerdictLine};

use crate::ui::kit::fr;

pub const FILTERS: [&str; 4] = ["Toutes", "En guerre", "Alliés et vassaux", "En paix"];
pub const SORTS: [&str; 4] = ["Par position", "Par attitude", "Par puissance", "Voisinage"];

/// Ink colours of the relation status (`STATUS_COLORS`).
pub fn status_color(status: &str) -> Color {
    match status {
        "war" => Color::srgb(0.62, 0.12, 0.10),
        "truce" => Color::srgb(0.66, 0.50, 0.08),
        "peace" => Color::srgb(0.35, 0.33, 0.30),
        "alliance" => Color::srgb(0.15, 0.32, 0.62),
        "vassal" | "suzerain" => Color::srgb(0.42, 0.20, 0.55),
        _ => crate::ui::kit::style::INK,
    }
}

/// French label of a relation status (`STATUS_LABELS`).
pub fn status_label(status: &str) -> &str {
    match status {
        "war" => "En guerre",
        "truce" => "Trêve",
        "peace" => "Paix",
        "alliance" => "Alliance",
        "vassal" => "Notre vassal",
        "suzerain" => "Notre suzerain",
        other => other,
    }
}

fn status_rank(status: &str) -> usize {
    ["war", "suzerain", "vassal", "alliance", "truce", "peace"]
        .iter()
        .position(|s| *s == status)
        .unwrap_or(usize::MAX)
}

fn passes_filter(filter: usize, status: &str) -> bool {
    match filter {
        1 => status == "war",
        2 => matches!(status, "alliance" | "vassal" | "suzerain"),
        3 => matches!(status, "peace" | "truce"),
        _ => true,
    }
}

/// Rows visible under `filter` (index of [`FILTERS`]) in the order `sort` (index of [`SORTS`]).
pub fn visible_rows(rows: &[FactionRow], filter: usize, sort: usize) -> Vec<&FactionRow> {
    let mut out: Vec<&FactionRow> = rows
        .iter()
        .filter(|r| passes_filter(filter, &r.status))
        .collect();
    out.sort_by(|a, b| match sort {
        1 => a
            .attitude
            .cmp(&b.attitude)
            .then_with(|| a.name.cmp(&b.name)),
        2 => b.power.cmp(&a.power).then_with(|| a.name.cmp(&b.name)),
        3 => b
            .neighbour
            .cmp(&a.neighbour)
            .then_with(|| a.attitude.cmp(&b.attitude))
            .then_with(|| a.name.cmp(&b.name)),
        _ => status_rank(&a.status)
            .cmp(&status_rank(&b.status))
            .then_with(|| a.name.cmp(&b.name)),
    });
    out
}

/// « +12 » / « −5 » with the real minus sign.
pub fn signed(value: i32) -> String {
    if value >= 0 {
        format!("+{value}")
    } else {
        format!("{}{}", '\u{2212}', value.abs())
    }
}

/// « Il manque 12 au seuil » / « Marge de 8 » / « Clause impossible ».
pub fn threshold_words(score: i32, blocked: bool) -> String {
    if blocked {
        "Clause impossible".to_owned()
    } else if score < 0 {
        format!("Il manque {} au seuil", -score)
    } else {
        format!("Marge de {score}")
    }
}

/// One line of the opinion block: « +12  Raison (encore 3 tours) ».
pub fn reason_line(reason: &ReasonView) -> String {
    let tail = if reason.turns_left > 0 {
        format!(
            " (encore {})",
            fr::count(i64::from(reason.turns_left), "tour", Some("tours"))
        )
    } else {
        String::new()
    };
    format!("{}  {}{}", signed(reason.value), reason.text, tail)
}

/// « Opinion — Hostile −30 ».
pub fn opinion_total(band: &str, attitude: i32) -> String {
    if band.is_empty() {
        format!("Opinion — {}", signed(attitude))
    } else {
        format!("Opinion — {band} {}", signed(attitude))
    }
}

/// BBCode tooltip of a faction row / the opinion block: total then every reason.
pub fn opinion_tooltip(row: &FactionRow) -> String {
    let mut text = format!(
        "[b]Attitude envers nous : {}{}[/b]",
        if row.attitude_band.is_empty() {
            String::new()
        } else {
            format!("{} ", row.attitude_band)
        },
        signed(row.attitude)
    );
    for reason in &row.reasons {
        text.push('\n');
        text.push_str(&colored_reason(reason.value, &reason_line(reason)));
    }
    text
}

/// BBCode tooltip of one reason.
pub fn reason_tooltip(reason: &ReasonView) -> String {
    colored_reason(reason.value, &reason_line(reason))
}

fn colored_reason(value: i32, line: &str) -> String {
    let color = if value < 0 { "#9e2114" } else { "#2a6a2a" };
    format!("[color={color}]{line}[/color]")
}

/// BBCode of a verdict line « +8  Accord commercial — routes communes ».
pub fn verdict_line_bbcode(line: &VerdictLine) -> String {
    colored_reason(
        line.value,
        &format!("{}  {}", signed(line.value), line.text),
    )
}

/// Tooltip of a clause: the lines of the verdict that belong to clause `index`.
pub fn clause_tooltip(lines: &[VerdictLine], index: usize, blocked: &str) -> String {
    let mut parts: Vec<String> = lines
        .iter()
        .filter(|l| l.article == Some(index))
        .map(verdict_line_bbcode)
        .collect();
    if !blocked.is_empty() {
        parts.push(format!("[b]Impossible :[/b] {blocked}"));
    }
    if parts.is_empty() {
        "Aucun poids particulier.".to_owned()
    } else {
        parts.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(id: &str, status: &str, attitude: i32, power: i64, neighbour: bool) -> FactionRow {
        FactionRow {
            id: id.into(),
            name: id.into(),
            color: "#888888".into(),
            ruler: String::new(),
            status: status.into(),
            stance: "neutral".into(),
            stance_label: String::new(),
            attitude,
            attitude_band: String::new(),
            reasons: Vec::new(),
            allies: Vec::new(),
            enemies: Vec::new(),
            vassals: Vec::new(),
            truce_turns_left: 0,
            embargo_by_us: false,
            embargo_on_us: false,
            war_score: 0,
            casus_belli: String::new(),
            claims: Vec::new(),
            religion_name: String::new(),
            loyalty: None,
            power,
            weariness: 0,
            trade_agreement: false,
            access_given: false,
            access_received: false,
            alliance_kind: String::new(),
            non_aggression_turns_left: 0,
            neighbour,
            hegemon: false,
        }
    }

    #[test]
    fn sorts_and_filters_like_godot() {
        let rows = vec![
            row("b", "peace", -10, 5, false),
            row("a", "war", 20, 1, false),
            row("c", "alliance", 40, 9, true),
        ];
        let ids = |v: Vec<&FactionRow>| v.iter().map(|r| r.id.clone()).collect::<Vec<_>>();
        assert_eq!(ids(visible_rows(&rows, 0, 0)), ["a", "c", "b"]);
        assert_eq!(ids(visible_rows(&rows, 0, 1)), ["b", "a", "c"]);
        assert_eq!(ids(visible_rows(&rows, 0, 2)), ["c", "b", "a"]);
        assert_eq!(ids(visible_rows(&rows, 0, 3)), ["c", "b", "a"]);
        assert_eq!(ids(visible_rows(&rows, 1, 0)), ["a"]);
        assert_eq!(ids(visible_rows(&rows, 2, 0)), ["c"]);
        assert_eq!(ids(visible_rows(&rows, 3, 0)), ["b"]);
    }

    #[test]
    fn words_and_signs() {
        assert_eq!(signed(5), "+5");
        assert_eq!(signed(-5), "\u{2212}5");
        assert_eq!(threshold_words(-12, false), "Il manque 12 au seuil");
        assert_eq!(threshold_words(8, false), "Marge de 8");
        assert_eq!(threshold_words(0, true), "Clause impossible");
        let reason = ReasonView {
            text: "Trahison".into(),
            value: -20,
            turns_left: 3,
        };
        assert!(reason_line(&reason).contains("encore 3 tours"));
    }
}
