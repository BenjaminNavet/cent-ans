//! Pure text of the feudal panel (port of `FeudalSection.lines` / `objective_lines` and the
//! status glyphs of `feudal_controller.gd`). Tested below.

use bevy::prelude::Color;
use game_api::feudal::{DutiesView, FeudalSheet, FelonyRow};

use crate::ui::kit::{fr, style};

/// Ink of a line (`HudStyle.INK`, `INK_SOFT`, `POOR`, `GOOD`, `RUBRIC`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    Ink,
    Soft,
    Poor,
    Good,
    Rubric,
}

impl Tone {
    pub fn color(self) -> Color {
        match self {
            Tone::Ink => style::INK,
            Tone::Soft => style::INK_SOFT,
            Tone::Poor => style::POOR,
            Tone::Good => style::GOOD,
            Tone::Rubric => style::RUBRIC,
        }
    }
}

/// `(text, tone, node name)`.
pub type Line = (String, Tone, String);

const MET: &str = "✓";
const OPEN: &str = "○";

/// Ink of a vassal status (`STATUS_COLORS`).
pub fn status_color(status: &str) -> Color {
    match status {
        "loyal" => Color::srgb(0.20, 0.45, 0.22),
        "discontent" => Color::srgb(0.70, 0.45, 0.08),
        "felon" => Color::srgb(0.62, 0.10, 0.08),
        "in_revolt" => Color::srgb(0.45, 0.05, 0.30),
        _ => style::INK,
    }
}

/// Glyph of a vassal status (`STATUS_GLYPHS`).
pub fn status_glyph(status: &str) -> &'static str {
    match status {
        "loyal" => "●",
        "discontent" => "◐",
        "felon" => "✖",
        "in_revolt" => "⚑",
        _ => "",
    }
}

fn names(refs: &[game_api::FactionRef]) -> String {
    refs.iter()
        .map(|f| f.name.as_str())
        .collect::<Vec<_>>()
        .join(", ")
}

fn turns(n: u32) -> String {
    fr::count(i64::from(n), "tour", Some("tours"))
}

fn line(text: String, tone: Tone, name: &str) -> Line {
    (text, tone, name.to_owned())
}

fn felony_text(case: &FelonyRow, own: bool) -> String {
    if own {
        format!(
            "Vous êtes félon envers {} ({}) : la commise peut être prononcée contre vous ({}).",
            case.liege.name,
            case.reason,
            turns(case.turns_left)
        )
    } else {
        format!(
            "Félonie de {} ({}) : commise possible encore {}.",
            case.vassal.name,
            case.reason,
            turns(case.turns_left)
        )
    }
}

/// « Position et obligations » lines.
pub fn position_lines(sheet: &FeudalSheet, duties: &DutiesView) -> Vec<Line> {
    let mut out = Vec::new();
    let titles: Vec<&str> = sheet.titles.iter().map(|t| t.name.as_str()).collect();
    out.push(line(
        format!("Titres : {}.", titles.join(", ")),
        Tone::Ink,
        "Titles",
    ));
    match &sheet.liege {
        None => out.push(line(
            "Souverain : ne doit l’hommage à personne.".to_owned(),
            Tone::Ink,
            "Liege",
        )),
        Some(liege) => {
            let above = if sheet.liege_chain.len() > 1 {
                let chain: Vec<&str> = sheet.liege_chain[1..]
                    .iter()
                    .map(|f| f.name.as_str())
                    .collect();
                format!(" ; au-dessus : {}", chain.join(" › "))
            } else {
                String::new()
            };
            out.push(line(
                format!(
                    "Suzerain : {} (loyauté {}, {}){}.",
                    liege.name,
                    sheet.loyalty.unwrap_or(0),
                    sheet.status_label,
                    above
                ),
                Tone::Ink,
                "Liege",
            ));
            out.push(line(
                format!(
                    "Tribut : {} % des revenus, {} cette saison à {}.",
                    duties.tribute_percent,
                    fr::amount(duties.tribute),
                    liege.name
                ),
                Tone::Soft,
                "Tribute",
            ));
            let host = names(&duties.host_against);
            if host.is_empty() {
                out.push(line(
                    "Ost : votre suzerain n’est en guerre contre personne.".to_owned(),
                    Tone::Soft,
                    "Host",
                ));
            } else {
                out.push(line(format!("Ost dû contre : {host}."), Tone::Poor, "Host"));
            }
        }
    }
    let vassals = names(&sheet.direct_vassals);
    out.push(line(
        if vassals.is_empty() {
            "Aucun vassal direct.".to_owned()
        } else {
            format!("Vassaux directs : {vassals}.")
        },
        Tone::Soft,
        "Vassals",
    ));
    let extra = names(&sheet.title_vassals);
    if !extra.is_empty() {
        out.push(line(
            format!("Tiennent aussi un fief de vous : {extra} (double allégeance)."),
            Tone::Soft,
            "TitleVassals",
        ));
    }
    if !duties.protect.is_empty() {
        let due: Vec<String> = duties
            .protect
            .iter()
            .map(|p| format!("{} (attaqué par {})", p.vassal.name, names(&p.attackers)))
            .collect();
        out.push(line(
            format!("Protection due : {}.", due.join(", ")),
            Tone::Poor,
            "Protect",
        ));
    }
    for case in &duties.felons {
        out.push(line(felony_text(case, false), Tone::Rubric, "Felon"));
    }
    for case in &duties.own_felonies {
        out.push(line(felony_text(case, true), Tone::Poor, "OwnFelony"));
    }
    out
}

/// Objective lines: historical titles first, then the generic ways (§ 4.8).
pub fn objective_lines(sheet: &FeudalSheet) -> Vec<Line> {
    let mut out = Vec::new();
    for (index, o) in sheet.objectives.iter().enumerate() {
        let mut text = format!("{} {}", if o.met { MET } else { OPEN }, o.title);
        if !o.description.is_empty() {
            text.push_str(" — ");
            text.push_str(&o.description);
        }
        out.push((
            text,
            if o.met { Tone::Good } else { Tone::Ink },
            format!("Objective{index}"),
        ));
    }
    let s = &sheet.streaks;
    if sheet.liege.is_none() {
        out.push(line(
            format!(
                "{OPEN} Indépendance tenue : {} / {} tours.",
                s.independent_turns, s.independence_turns
            ),
            Tone::Soft,
            "Independence",
        ));
    } else {
        out.push(line(
            format!(
                "{OPEN} Premier vassal du royaume : {} / {} tours.",
                s.first_vassal_turns, s.ascension_turns
            ),
            Tone::Soft,
            "FirstVassal",
        ));
    }
    if !s.start_crown_name.is_empty() {
        out.push(line(
            format!("{OPEN} Ceindre la couronne : {}.", s.start_crown_name),
            Tone::Soft,
            "Crown",
        ));
    }
    out
}

/// « Duché de Guyenne » → rank label capitalised, for the possessions list.
pub fn rank_heading(rank_label: &str) -> String {
    let mut chars = rank_label.chars();
    chars
        .next()
        .map(|c| c.to_uppercase().collect::<String>() + chars.as_str())
        .unwrap_or_default()
}

/// Banner of a call kind: yes / no captions.
pub fn call_captions(kind: &str) -> (&'static str, &'static str) {
    match kind {
        "protection" => ("Intervenir", "Se dérober"),
        "peace_summons" => ("Obéir", "Passer outre"),
        _ => ("Imposer la paix", "Laisser faire"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use game_api::feudal::{ObjectiveRow, StreaksView, TitleView};
    use game_api::FactionRef;

    fn fref(id: &str, name: &str) -> FactionRef {
        FactionRef {
            id: id.into(),
            name: name.into(),
        }
    }

    fn sheet(liege: Option<FactionRef>) -> FeudalSheet {
        FeudalSheet {
            id: "fac_a".into(),
            name: "A".into(),
            ruler: "R".into(),
            house: "Valois".into(),
            primary_name: "Duché".into(),
            rank: "duchy".into(),
            rank_label: "duché".into(),
            titles: vec![TitleView {
                id: "tit_a".into(),
                name: "Duché de A".into(),
                rank: "duchy".into(),
                rank_label: "duché".into(),
                provinces: vec![],
                primary: true,
            }],
            liege_chain: liege.iter().cloned().collect(),
            liege,
            sovereign: fref("fac_k", "K"),
            direct_vassals: vec![],
            title_vassals: vec![fref("fac_t", "T")],
            loyalty: Some(60),
            status: "loyal".into(),
            status_label: "loyal".into(),
            objectives: vec![ObjectiveRow {
                title: "Tenir".into(),
                description: "Le fief.".into(),
                met: true,
            }],
            streaks: StreaksView {
                independent_turns: 3,
                independence_turns: 20,
                first_vassal_turns: 1,
                ascension_turns: 10,
                start_crown_name: "Couronne".into(),
            },
        }
    }

    fn duties() -> DutiesView {
        DutiesView {
            tribute: 1200,
            tribute_percent: 10,
            host_against: vec![],
            protect: vec![],
            felons: vec![],
            own_felonies: vec![],
            grantable_titles: vec![],
            homage_candidates: vec![],
        }
    }

    #[test]
    fn sovereign_lines() {
        let lines = position_lines(&sheet(None), &duties());
        assert!(lines[1].0.starts_with("Souverain"));
        assert!(lines.iter().any(|l| l.0.starts_with("Aucun vassal direct")));
        assert!(lines.iter().any(|l| l.0.contains("double allégeance")));
        let obj = objective_lines(&sheet(None));
        assert!(obj[0].0.starts_with("✓ Tenir — Le fief."));
        assert!(obj.iter().any(|l| l.0.contains("Indépendance tenue : 3 / 20")));
    }

    #[test]
    fn vassal_lines() {
        let s = sheet(Some(fref("fac_k", "Roi")));
        let mut d = duties();
        d.host_against = vec![fref("fac_e", "Angleterre")];
        let lines = position_lines(&s, &d);
        assert!(lines[1].0.contains("Suzerain : Roi (loyauté 60, loyal)"));
        assert!(lines[2].0.contains("10 %"));
        assert!(lines[3].0.starts_with("Ost dû contre : Angleterre"));
        assert_eq!(lines[3].1, Tone::Poor);
        assert!(objective_lines(&s)
            .iter()
            .any(|l| l.0.contains("Premier vassal du royaume : 1 / 10")));
    }

    #[test]
    fn glyphs_and_captions() {
        assert_eq!(status_glyph("felon"), "✖");
        assert_eq!(status_glyph(""), "");
        assert_eq!(call_captions("protection").0, "Intervenir");
        assert_eq!(rank_heading("duché"), "Duché");
    }
}
