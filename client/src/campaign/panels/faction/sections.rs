//! Sections of the faction panel ported from Godot: money (`coinage_section.gd`), order of
//! chivalry (`chivalry_section.gd`), fervour (`crusade_section.gd`), feudal obligations
//! (`feudal_section.gd`), goods by category (`faction_panel.gd`) and the economy trade tab
//! (`economy_trade_tab.gd`). The texts are pure functions so they are unit-tested; the builders
//! only lay them out. No rule here: every figure comes from `game_api::faction_panel`.

use bevy::prelude::*;
use game_api::faction_panel::{
    ChivalryOption, ChivalrySection, CoinageOption, CoinageSection, FactionPanelView,
    GoodsCategory, PartnerRow,
};
use game_api::feudal::{DutiesView, FeudalSheet};
use sim_campaign::crusade::{CrusadeView, PendingPassageView};

use super::FactionPanel;
use super::view::{Kids, ink, pair, wrapped};
use crate::ui::kit::tooltip::Tooltip;
use crate::ui::kit::{
    UiTheme, UiType, button_small, caption, fr, heading, separator, style, toggle_button,
};

/// Price-level gauge bounds (Godot `PRICE_MIN` / `PRICE_MAX`).
pub const PRICE_MIN: f32 = 100.0;
pub const PRICE_MAX: f32 = 400.0;
const GAUGE_WIDTH: f32 = 300.0;
const MAX_PARTNERS: usize = 12;

const COINAGE_EXPLANATION: &str = "Les rois « muent » la monnaie : moins d’argent fin dans chaque pièce rapporte un seigneuriage immédiat, mais les prix montent et les rentes fixes des bourgeois et du clergé fondent. Nicole Oresme plaide pour une monnaie forte, stable, comptée en livres tournois.";

/// Fill of the price gauge.
#[derive(Component)]
pub struct PriceFill;

/// Fill of the fervour gauge.
#[derive(Component)]
pub struct FervorFill;

/// A tick of the fervour gauge (threshold or floor).
#[derive(Component)]
pub struct FervorTick;

/// A goods chip.
#[derive(Component)]
pub struct GoodsChip;

// ---------------------------------------------------------------- texts

/// `strong` -> « Forte » (Godot trims the « Monnaie » prefix and capitalises).
pub fn coinage_short(label: &str) -> String {
    let short = label.trim_start_matches("Monnaie ");
    let mut chars = short.chars();
    chars.next().map_or(String::new(), |first| {
        first.to_uppercase().collect::<String>() + chars.as_str()
    })
}

/// Price multiplier text: 250 -> « ×2,5 ».
pub fn price_factor_text(price: u32) -> String {
    let factor = (price as f64 / 100.0 * 100.0).round() / 100.0;
    format!("×{}", format!("{factor}").replace('.', ","))
}

/// Tooltip of one coinage level (`RichTooltip.coinage`).
pub fn coinage_tooltip(option: &CoinageOption, changed_this_year: bool) -> String {
    let mut lines = vec![format!(
        "[b]{}[/b]{}",
        option.label,
        if option.current {
            " — monnaie actuelle"
        } else {
            ""
        }
    )];
    lines.push(format!(
        "Seigneuriage : {} / saison",
        fr::signed(option.seigniorage)
    ));
    if option.recoinage > 0 {
        lines.push(format!(
            "Refonte des espèces : {}{} / saison (administration)",
            fr::MINUS,
            fr::amount(option.recoinage)
        ));
    }
    if option.inflation > 0 {
        lines.push(format!(
            "Prix : +{} / saison (recrutement, entretien et constructions renchérissent)",
            option.inflation
        ));
    } else if option.deflation > 0 {
        lines.push(format!(
            "Prix : {}{} / saison (jusqu’aux prix de 1337)",
            fr::MINUS,
            option.deflation
        ));
    } else {
        lines.push("Prix : stables".to_owned());
    }
    if option.burgher_unrest.abs() > f64::EPSILON {
        lines.push(format!(
            "Mécontentement des bourgeois : {}{} (cible)",
            if option.burgher_unrest > 0.0 {
                "+"
            } else {
                "\u{2212}"
            },
            option.burgher_unrest.abs()
        ));
    }
    if option.prestige != 0 {
        lines.push(format!(
            "Prestige du souverain : {} / saison",
            fr::signed(i64::from(option.prestige))
        ));
    }
    lines.push("[i]Un seul changement de monnaie par année civile.[/i]".to_owned());
    if changed_this_year && !option.current {
        lines.push(
            "Refusé cette année : la monnaie a déjà été changée ; prochain changement possible l’an prochain."
                .to_owned(),
        );
    }
    lines.join("\n")
}

/// Tooltip of a foundable order (`RichTooltip.chivalric_order`).
pub fn chivalry_tooltip(option: &ChivalryOption) -> String {
    let mut lines = vec![format!(
        "[b]{}[/b] — {}",
        option.name,
        if option.available {
            "disponible"
        } else {
            "indisponible"
        }
    )];
    lines.push(format!(
        "Coût : {} · prestige requis : {}",
        fr::amount(option.cost),
        option.prestige_required
    ));
    lines.push(format!(
        "Membres : {} (historiquement : {})",
        option.members,
        if option.historical_members.is_empty() {
            "—"
        } else {
            &option.historical_members
        }
    ));
    lines.push(format!(
        "Membres : loyauté +{}, moral des armées qu’ils mènent +{}",
        option.member_loyalty, option.member_morale
    ));
    lines.push(format!(
        "Souverain : prestige +{} à la fondation, +{} par an",
        option.founder_prestige, option.yearly_prestige
    ));
    if option.min_year > 0 {
        lines.push(format!("Fondation possible dès {}", option.min_year));
    }
    if !option.available && !option.reason.is_empty() {
        lines.push(format!("Refus : {}", option.reason));
    }
    lines.push(strip_codex_links(&option.description));
    lines.join("\n")
}

/// `[[cdx_x|texte]]` -> `texte` (codex links are not clickable yet in the Bevy client).
pub fn strip_codex_links(text: &str) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(start) = rest.find("[[") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        let Some(end) = after.find("]]") else {
            out.push_str(&rest[start..]);
            return out;
        };
        let inner = &after[..end];
        out.push_str(inner.rsplit('|').next().unwrap_or(inner));
        rest = &after[end + 2..];
    }
    out.push_str(rest);
    out
}

fn count(value: i64, singular: &str, plural: &str) -> String {
    format!("{value} {}", if value <= 1 { singular } else { plural })
}

fn signed_int(value: i32) -> String {
    match value {
        v if v > 0 => format!("+{v}"),
        v if v < 0 => format!("{}{}", fr::MINUS, -v),
        _ => "0".to_owned(),
    }
}

/// « Élan de la Croix : moral +10 », « débandade… » (`CrusadeSection.status_text`).
pub fn crusade_status_text(view: &CrusadeView) -> String {
    let mut parts = Vec::new();
    if view.zeal_morale > 0 {
        parts.push(format!(
            "Élan de la Croix : moral {}",
            signed_int(view.zeal_morale)
        ));
    } else if view.zeal_morale < 0 {
        parts.push(format!(
            "Ferveur chancelante : moral {}",
            signed_int(view.zeal_morale)
        ));
    }
    if view.desertion_percent > 0 {
        parts.push(format!(
            "Débandade : {} % des hommes rentrent chez eux chaque tour",
            view.desertion_percent
        ));
    }
    if parts.is_empty() {
        "Ost résolu : ni élan ni découragement.".to_owned()
    } else {
        parts.join(" · ") + "."
    }
}

/// « 3 unités de volontaires débarqueront dans 2 tours… ».
pub fn crusade_promise_text(view: &CrusadeView) -> String {
    format!(
        "{} de volontaires débarqueront dans {} dans un port que vous tenez.",
        count(i64::from(view.passage_units), "unité", "unités"),
        count(i64::from(view.passage_delay), "tour", "tours")
    )
}

/// « Limassol : 3 unités, dans 2 tours ».
pub fn pending_text(passage: &PendingPassageView) -> String {
    let when = if passage.turns_left > 0 {
        format!(
            "dans {}",
            count(i64::from(passage.turns_left), "tour", "tours")
        )
    } else {
        "ce tour".to_owned()
    };
    format!(
        "{} : {}, {when}",
        passage.port_name,
        count(i64::from(passage.units), "unité", "unités")
    )
}

/// « Débandade sous 20 · élan à partir de 70 · plancher 50 ».
pub fn crusade_marks_text(view: &CrusadeView) -> String {
    let mut parts = vec![
        format!("Débandade sous {}", view.desertion_threshold),
        format!("élan à partir de {}", view.zeal_high_threshold),
    ];
    if view.floor > 0 {
        parts.push(format!("plancher {}", view.floor));
    }
    parts.join(" · ")
}

/// Tooltip of the gauge: value, causes of the turn, thresholds.
pub fn crusade_tooltip(view: &CrusadeView) -> String {
    let mut lines = vec![
        format!("[b]Ferveur : {} / 100[/b]", view.fervor),
        "Elle paie l’ost par les aumônes, le renforce par le passage et le disperse quand elle s’éteint."
            .to_owned(),
        String::new(),
        "[b]Ce tour[/b]".to_owned(),
    ];
    let mut listed = 0;
    for change in view.changes.iter().filter(|c| c.delta != 0) {
        lines.push(format!("• {} : {}", change.cause, signed_int(change.delta)));
        listed += 1;
    }
    if listed == 0 {
        lines.push("Aucun mouvement.".to_owned());
    }
    lines.push(String::new());
    lines.push("[b]État[/b]".to_owned());
    lines.push(crusade_status_text(view));
    lines.push(format!("Aumônes : {} par tour.", fr::amount(view.alms)));
    lines.push(String::new());
    lines.push("[b]Seuils[/b]".to_owned());
    lines.push(format!(
        "• {} et plus : élan de la Croix (moral {})",
        view.zeal_high_threshold,
        signed_int(view.zeal_high_morale)
    ));
    lines.push(format!(
        "• Sous {} : ferveur chancelante (moral {})",
        view.zeal_low_threshold,
        signed_int(view.zeal_low_morale)
    ));
    lines.push(format!(
        "• Sous {} : débandade ({} % des hommes par tour, le double à 0)",
        view.desertion_threshold, view.desertion_men_percent
    ));
    if view.floor > 0 {
        lines.push(format!(
            "• Plancher : {} tant que {} est tenue",
            view.floor, view.target_name
        ));
    }
    lines.join("\n")
}

/// Fill colour of the fervour gauge (Godot `FervorGauge.fill_color`).
pub fn fervor_colour(view: &CrusadeView) -> Color {
    if view.fervor < view.desertion_threshold {
        style::POOR
    } else if view.fervor < view.zeal_low_threshold {
        style::FAIR
    } else if view.fervor >= view.zeal_high_threshold {
        style::GOLD
    } else {
        style::GOOD
    }
}

/// Lines « obligations » of the feudal section: `(text, colour)` (`FeudalSection.lines`).
pub fn feudal_lines(sheet: &FeudalSheet, duties: &DutiesView) -> Vec<(String, Color)> {
    let names = |refs: &[game_api::common::FactionRef]| {
        refs.iter()
            .map(|r| r.name.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    };
    let mut lines = Vec::new();
    match &sheet.liege {
        None => lines.push((
            "Souverain : ne doit l’hommage à personne.".to_owned(),
            style::INK,
        )),
        Some(liege) => {
            let above = if sheet.liege_chain.len() > 1 {
                format!(
                    " ; au-dessus : {}",
                    sheet.liege_chain[1..]
                        .iter()
                        .map(|l| l.name.as_str())
                        .collect::<Vec<_>>()
                        .join(" › ")
                )
            } else {
                String::new()
            };
            lines.push((
                format!(
                    "Suzerain : {} (loyauté {}, {}){above}.",
                    liege.name,
                    sheet.loyalty.unwrap_or(0),
                    sheet.status_label
                ),
                style::INK,
            ));
            lines.push((
                format!(
                    "Tribut : {} % des revenus, {} cette saison à {}.",
                    duties.tribute_percent,
                    fr::amount(duties.tribute),
                    liege.name
                ),
                style::INK_SOFT,
            ));
            if duties.host_against.is_empty() {
                lines.push((
                    "Ost : votre suzerain n’est en guerre contre personne.".to_owned(),
                    style::INK_SOFT,
                ));
            } else {
                lines.push((
                    format!("Ost dû contre : {}.", names(&duties.host_against)),
                    style::POOR,
                ));
            }
        }
    }
    if sheet.direct_vassals.is_empty() {
        lines.push(("Aucun vassal direct.".to_owned(), style::INK_SOFT));
    } else {
        lines.push((
            format!("Vassaux directs : {}.", names(&sheet.direct_vassals)),
            style::INK_SOFT,
        ));
    }
    if !sheet.title_vassals.is_empty() {
        lines.push((
            format!(
                "Tiennent aussi un fief de vous : {} (double allégeance).",
                names(&sheet.title_vassals)
            ),
            style::INK_SOFT,
        ));
    }
    if !duties.protect.is_empty() {
        let due: Vec<String> = duties
            .protect
            .iter()
            .map(|p| format!("{} (attaqué par {})", p.vassal.name, names(&p.attackers)))
            .collect();
        lines.push((format!("Protection due : {}.", due.join(", ")), style::POOR));
    }
    for case in &duties.felons {
        lines.push((
            format!(
                "Félonie de {} ({}) : commise possible encore {}.",
                case.vassal.name,
                case.reason,
                count(i64::from(case.turns_left), "tour", "tours")
            ),
            style::RUBRIC,
        ));
    }
    for case in &duties.own_felonies {
        lines.push((
            format!(
                "Vous êtes félon envers {} ({}) : la commise peut être prononcée contre vous ({}).",
                case.liege.name,
                case.reason,
                count(i64::from(case.turns_left), "tour", "tours")
            ),
            style::POOR,
        ));
    }
    let streaks = &sheet.streaks;
    if sheet.liege.is_none() {
        lines.push((
            format!(
                "○ Indépendance tenue : {} / {} tours.",
                streaks.independent_turns, streaks.independence_turns
            ),
            style::INK_SOFT,
        ));
    } else {
        lines.push((
            format!(
                "○ Premier vassal du royaume : {} / {} tours.",
                streaks.first_vassal_turns, streaks.ascension_turns
            ),
            style::INK_SOFT,
        ));
    }
    if !streaks.start_crown_name.is_empty() {
        lines.push((
            format!("○ Ceindre la couronne : {}.", streaks.start_crown_name),
            style::INK_SOFT,
        ));
    }
    lines
}

/// « Angleterre — +420 ₶ par saison pour nous » (agreement in force).
pub fn agreement_text(partner: &PartnerRow) -> String {
    format!(
        "{} — {} par saison pour nous",
        partner.name,
        fr::signed(partner.current_self)
    )
}

/// « Angleterre — +420 ₶ par saison pour nous, +300 ₶ pour eux » (before signing).
pub fn partner_text(partner: &PartnerRow) -> String {
    format!(
        "{} — {} par saison pour nous, {} pour eux",
        partner.name,
        fr::signed(partner.gain_self),
        fr::signed(partner.gain_partner)
    )
}

// ---------------------------------------------------------------- builders

/// Section « Monnaie »: current money, price gauge, seigniorage, recoinage, selector.
pub fn coinage(pane: &mut Kids, theme: &UiTheme, section: &CoinageSection, editable: bool) {
    pane.spawn(separator(theme));
    pair(pane, theme, "Monnaie", &section.label, style::INK);
    let price_tip = format!(
        "Prix de 1337 = 100. Recrutement, entretien et constructions coûtent {} ; les rentes fixes des bourgeois et du clergé en souffrent.",
        price_factor_text(section.price_level)
    );
    pane.spawn((
        Name::new("faction:price_level"),
        Node {
            align_items: AlignItems::Center,
            column_gap: Val::Px(8.0),
            ..default()
        },
        Tooltip::new(&price_tip),
    ))
    .with_children(|row| {
        row.spawn(ink(
            theme,
            &format!("Niveau des prix : {}", section.price_level),
            UiType::Caption,
            style::INK,
        ));
        let ratio =
            ((section.price_level as f32 - PRICE_MIN) / (PRICE_MAX - PRICE_MIN)).clamp(0.0, 1.0);
        let tint = style::GOOD.mix(&style::POOR, ((ratio * 300.0) / 150.0).clamp(0.0, 1.0));
        row.spawn((
            Node {
                width: Val::Px(GAUGE_WIDTH * 0.5),
                height: Val::Px(10.0),
                ..default()
            },
            BackgroundColor(style::PARCHMENT_DARK),
        ))
        .with_children(|bar| {
            bar.spawn((
                PriceFill,
                Node {
                    width: Val::Percent(ratio * 100.0),
                    height: Val::Percent(100.0),
                    ..default()
                },
                BackgroundColor(tint),
            ));
        });
    });
    pane.spawn(ink(
        theme,
        &format!(
            "Seigneuriage : {} / saison (saison passée : {})",
            fr::signed(section.seigniorage),
            fr::signed(section.seigniorage_last_turn)
        ),
        UiType::Caption,
        style::INK,
    ));
    pane.spawn(ink(
        theme,
        &format!(
            "Refonte : {} / saison (saison passée : {})",
            fr::amount(section.recoinage),
            fr::amount(section.recoinage_last_turn)
        ),
        UiType::Caption,
        style::INK,
    ));
    if editable {
        if section.changed_this_year {
            pane.spawn(ink(
                theme,
                "Monnaie déjà changée cette année : prochain changement l’an prochain.",
                UiType::Caption,
                style::MUTED,
            ));
        }
        pane.spawn(Node {
            flex_wrap: FlexWrap::Wrap,
            column_gap: Val::Px(4.0),
            row_gap: Val::Px(2.0),
            ..default()
        })
        .with_children(|grid| {
            for option in &section.options {
                // A level refused this year stays clickable: the sim says why.
                let name = format!("faction:coinage:{}", option.level);
                let mut button = grid.spawn(toggle_button(
                    theme,
                    &name,
                    &coinage_short(&option.label),
                    option.current,
                ));
                button.insert(Tooltip::new(&coinage_tooltip(
                    option,
                    section.changed_this_year,
                )));
            }
        });
    }
    pane.spawn(wrapped(ink(
        theme,
        COINAGE_EXPLANATION,
        UiType::Caption,
        style::MUTED,
    )));
}

/// Section « Ordre de chevalerie ».
pub fn chivalry(pane: &mut Kids, theme: &UiTheme, section: &ChivalrySection) {
    pane.spawn(heading(theme, "Ordre de chevalerie"));
    if let Some(order) = &section.founded {
        let status = if order.collapsed { " — brisé" } else { "" };
        pane.spawn(ink(
            theme,
            &format!("{}{status}", order.name),
            UiType::Body,
            if order.collapsed {
                style::POOR
            } else {
                style::INK
            },
        ));
        let bonus = if order.collapsed {
            "Plus aucun bonus depuis la perte de la moitié de ses membres.".to_owned()
        } else {
            format!(
                "Membres : loyauté +{}, moral +{} · souverain : prestige +{} par an",
                order.member_loyalty, order.member_morale, order.yearly_prestige
            )
        };
        pane.spawn(wrapped(ink(theme, &bonus, UiType::Caption, style::MUTED)));
        let of = if order.members_max > 0 {
            format!("/{}", order.members_max)
        } else {
            String::new()
        };
        pane.spawn(ink(
            theme,
            &format!("Membres ({}{of}) :", order.members.len()),
            UiType::Caption,
            style::MUTED,
        ));
        pane.spawn(Node {
            flex_wrap: FlexWrap::Wrap,
            column_gap: Val::Px(8.0),
            ..default()
        })
        .with_children(|flow| {
            for member in &order.members {
                flow.spawn((
                    Name::new(format!("faction:member:{}", member.id)),
                    ink(theme, &member.name, UiType::Caption, style::RUBRIC),
                ));
            }
        });
        return;
    }
    if section.options.is_empty() {
        pane.spawn(caption(theme, "Aucun ordre ne peut être fondé."));
    }
    for option in &section.options {
        pane.spawn(wrapped(ink(
            theme,
            &format!(
                "{} — {}, prestige requis {}",
                option.name,
                fr::amount(option.cost),
                option.prestige_required
            ),
            UiType::Caption,
            style::INK,
        )));
        // An unavailable order stays clickable: the sim says why.
        let mut button = pane.spawn(button_small(
            theme,
            &format!("faction:found:{}", option.id),
            "Fonder l’ordre",
        ));
        button.insert(Tooltip::new(&chivalry_tooltip(option)));
        if !option.available && !option.reason.is_empty() {
            pane.spawn(wrapped(ink(
                theme,
                &format!("Impossible : {}", option.reason),
                UiType::Caption,
                style::ERROR,
            )));
        }
    }
}

/// Section « Ferveur » (crusader faction only).
pub fn crusade(pane: &mut Kids, theme: &UiTheme, view: &CrusadeView) {
    pane.spawn(separator(theme));
    pane.spawn(Node {
        justify_content: JustifyContent::SpaceBetween,
        ..default()
    })
    .with_children(|head| {
        head.spawn(heading(theme, "Ferveur"));
        head.spawn(ink(
            theme,
            &format!("{} / 100", view.fervor),
            UiType::Body,
            style::INK,
        ));
    });
    let pct = |value: u8| Val::Percent(f32::from(value.min(100)));
    pane.spawn((
        Name::new("faction:fervor_gauge"),
        Node {
            width: Val::Px(GAUGE_WIDTH),
            height: Val::Px(14.0),
            margin: UiRect::vertical(Val::Px(3.0)),
            ..default()
        },
        BackgroundColor(style::PARCHMENT_DARK),
        Tooltip::new(&crusade_tooltip(view)),
    ))
    .with_children(|bar| {
        bar.spawn((
            FervorFill,
            Node {
                width: pct(view.fervor),
                height: Val::Percent(100.0),
                ..default()
            },
            BackgroundColor(fervor_colour(view)),
        ));
        for threshold in [
            view.desertion_threshold,
            view.zeal_low_threshold,
            view.zeal_high_threshold,
        ] {
            bar.spawn((
                FervorTick,
                Node {
                    position_type: PositionType::Absolute,
                    left: pct(threshold),
                    top: Val::Px(-3.0),
                    width: Val::Px(1.0),
                    height: Val::Px(20.0),
                    ..default()
                },
                BackgroundColor(style::INK),
            ));
        }
        if view.floor > 0 {
            bar.spawn((
                FervorTick,
                Node {
                    position_type: PositionType::Absolute,
                    left: pct(view.floor),
                    margin: UiRect::left(Val::Px(-4.0)),
                    top: Val::Px(3.0),
                    width: Val::Px(8.0),
                    height: Val::Px(8.0),
                    ..default()
                },
                BackgroundColor(style::GOLD_PALE),
                UiTransform::from_rotation(Rot2::degrees(45.0)),
            ));
        }
    });
    pane.spawn(wrapped(ink(
        theme,
        &crusade_marks_text(view),
        UiType::Caption,
        style::INK_SOFT,
    )));
    let mut alms = pane.spawn(ink(
        theme,
        &format!("Aumônes : {} par tour", fr::amount(view.alms)),
        UiType::Caption,
        style::INK,
    ));
    alms.insert(Tooltip::new(&format!(
        "[b]Aumônes[/b]\nDons de la chrétienté, versés au trésor chaque tour : plus la ferveur est haute, plus ils sont larges.\nTour passé : {}",
        fr::amount(view.alms_last_turn)
    )));
    let colour = if view.desertion_percent > 0 || view.zeal_morale < 0 {
        style::POOR
    } else if view.zeal_morale > 0 {
        style::GOOD
    } else {
        style::INK_SOFT
    };
    pane.spawn(wrapped(ink(
        theme,
        &crusade_status_text(view),
        UiType::Caption,
        colour,
    )));
    if !view.target_name.is_empty() {
        let text = if view.target_taken {
            format!(
                "{} est tenue : la ferveur ne descend plus sous {}.",
                view.target_name, view.floor
            )
        } else {
            format!("But du vœu : {}.", view.target_name)
        };
        pane.spawn(wrapped(ink(theme, &text, UiType::Caption, style::INK_SOFT)));
    }
    let mut button = pane.spawn(button_small(
        theme,
        "faction:preach",
        &format!("Prêcher le passage ({})", fr::amount(view.passage_cost)),
    ));
    let body = if view.passage_available {
        crusade_promise_text(view)
    } else {
        format!("Impossible : {}", view.passage_blocker)
    };
    button.insert(Tooltip::new(&format!("[b]Prêcher le passage[/b]\n{body}")));
    if !view.passage_available && !view.passage_blocker.is_empty() {
        pane.spawn(wrapped(ink(
            theme,
            &format!("Impossible : {}", view.passage_blocker),
            UiType::Caption,
            style::POOR,
        )));
    }
    if !view.pending.is_empty() {
        pane.spawn(ink(
            theme,
            "Contingents attendus :",
            UiType::Caption,
            style::INK_SOFT,
        ));
        for passage in &view.pending {
            pane.spawn(ink(
                theme,
                &format!("• {}", pending_text(passage)),
                UiType::Caption,
                style::INK,
            ));
        }
    }
}

/// Feudal obligations (« Féodalité » section).
pub fn feudal(pane: &mut Kids, theme: &UiTheme, view: &FactionPanelView) {
    pane.spawn(separator(theme));
    pane.spawn(heading(theme, "Obligations féodales"));
    let Some(sheet) = &view.feudal else {
        pane.spawn(caption(theme, "Cette faction ne tient aucun titre."));
        return;
    };
    for (text, colour) in feudal_lines(sheet, &view.duties) {
        pane.spawn(wrapped(ink(theme, &text, UiType::Caption, colour)));
    }
}

/// Goods list, by category, with one chip per resource.
pub fn goods(pane: &mut Kids, theme: &UiTheme, categories: &[GoodsCategory]) {
    pane.spawn(separator(theme));
    pane.spawn(heading(theme, "Biens"));
    if categories.is_empty() {
        pane.spawn(caption(theme, "—"));
    }
    for category in categories {
        pane.spawn(ink(
            theme,
            &format!("• {}", category.label),
            UiType::Body,
            style::INK,
        ));
        pane.spawn(Node {
            flex_wrap: FlexWrap::Wrap,
            column_gap: Val::Px(10.0),
            margin: UiRect::left(Val::Px(12.0)),
            ..default()
        })
        .with_children(|flow| {
            for item in &category.items {
                flow.spawn((
                    GoodsChip,
                    Name::new(format!("faction:good:{}", item.id)),
                    ink(
                        theme,
                        &format!("{} ({})", item.name, item.count),
                        UiType::Caption,
                        style::INK,
                    ),
                    Tooltip::new(&format!(
                        "[b]{}[/b]\nProduit par le royaume : {}.",
                        item.name, item.count
                    )),
                ));
            }
        });
    }
}

/// Economy trade tab: agreements, new routes (partners), goods produced.
pub fn partners(pane: &mut Kids, theme: &UiTheme, panel: &FactionPanel, view: &FactionPanelView) {
    pane.spawn(separator(theme));
    pane.spawn(heading(theme, "Accords en cours"));
    let signed: Vec<&PartnerRow> = view.partners.iter().filter(|p| p.signed).collect();
    if signed.is_empty() {
        pane.spawn(caption(theme, "Aucun accord commercial en vigueur."));
    }
    for partner in signed {
        pane.spawn(Node {
            align_items: AlignItems::Center,
            column_gap: Val::Px(8.0),
            ..default()
        })
        .with_children(|row| {
            row.spawn((
                Node {
                    flex_grow: 1.0,
                    ..default()
                },
                ink(theme, &agreement_text(partner), UiType::Caption, style::INK),
            ));
            if view.is_player {
                let armed = panel.break_armed.as_deref() == Some(partner.id.as_str());
                row.spawn(button_small(
                    theme,
                    &format!("faction:break:{}", partner.id),
                    if armed { "Confirmer ?" } else { "Rompre" },
                ));
            }
        });
    }
    pane.spawn(separator(theme));
    pane.spawn(heading(theme, "Nouvelles routes"));
    let open: Vec<&PartnerRow> = view
        .partners
        .iter()
        .filter(|p| !p.signed && p.gain_self > 0)
        .take(MAX_PARTNERS)
        .collect();
    if open.is_empty() {
        pane.spawn(caption(
            theme,
            "Aucun nouveau partenaire : routes coupées, guerre ou accords déjà signés.",
        ));
    }
    for partner in open {
        pane.spawn(Node {
            align_items: AlignItems::Center,
            column_gap: Val::Px(8.0),
            ..default()
        })
        .with_children(|row| {
            row.spawn((
                Node {
                    flex_grow: 1.0,
                    flex_basis: Val::Px(0.0),
                    ..default()
                },
                ink(theme, &partner_text(partner), UiType::Caption, style::INK),
            ));
            if view.is_player {
                row.spawn(button_small(
                    theme,
                    &format!("faction:propose:{}", partner.id),
                    "Proposer l’accord",
                ));
            }
        });
    }
    pane.spawn(separator(theme));
    pane.spawn(heading(theme, "Échanges de ressources"));
    pane.spawn(wrapped(ink(
        theme,
        "Les biens voyagent avec les routes ci-dessus ; le royaume ne tient pas de stocks, donc pas d’achat ni de vente directs de ressources.",
        UiType::Caption,
        style::MUTED,
    )));
    let produced: Vec<String> = view
        .goods
        .iter()
        .flat_map(|c| &c.items)
        .map(|g| format!("{} × {}", g.name, g.count))
        .collect();
    if !produced.is_empty() {
        pane.spawn(wrapped(ink(
            theme,
            &produced.join(" · "),
            UiType::Caption,
            style::INK,
        )));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sim_campaign::crusade::FervorChange;

    fn crusade_view() -> CrusadeView {
        CrusadeView {
            fervor: 75,
            floor: 50,
            alms: 120,
            alms_last_turn: 100,
            changes: vec![
                FervorChange {
                    cause: "Bataille gagnée".into(),
                    delta: 5,
                },
                FervorChange {
                    cause: "Rien".into(),
                    delta: 0,
                },
            ],
            zeal_morale: 10,
            desertion_percent: 0,
            zeal_high_threshold: 70,
            zeal_low_threshold: 40,
            zeal_high_morale: 10,
            zeal_low_morale: -10,
            desertion_threshold: 20,
            desertion_men_percent: 5,
            target_taken: false,
            target_name: "Jérusalem".into(),
            passage_cost: 500,
            passage_cooldown: 0,
            passage_available: true,
            passage_blocker: String::new(),
            passage_units: 3,
            passage_delay: 2,
            pending: vec![PendingPassageView {
                turns_left: 1,
                port: data_model::SettlementId::new("set_x").unwrap(),
                port_name: "Limassol".into(),
                units: 3,
            }],
        }
    }

    #[test]
    fn crusade_texts() {
        let view = crusade_view();
        assert_eq!(crusade_status_text(&view), "Élan de la Croix : moral +10.");
        assert_eq!(
            crusade_marks_text(&view),
            "Débandade sous 20 · élan à partir de 70 · plancher 50"
        );
        assert_eq!(
            pending_text(&view.pending[0]),
            "Limassol : 3 unités, dans 1 tour"
        );
        assert!(crusade_promise_text(&view).starts_with("3 unités de volontaires"));
        let tip = crusade_tooltip(&view);
        assert!(tip.contains("Bataille gagnée : +5") && !tip.contains("Rien :"));
        assert_eq!(fervor_colour(&view), style::GOLD);
        let mut weak = view;
        weak.fervor = 10;
        assert_eq!(fervor_colour(&weak), style::POOR);
    }

    #[test]
    fn small_texts() {
        assert_eq!(
            coinage_short("Monnaie fortement affaiblie"),
            "Fortement affaiblie"
        );
        assert_eq!(price_factor_text(250), "×2,5");
        assert_eq!(price_factor_text(100), "×1");
        assert_eq!(
            strip_codex_links("Voir [[cdx_a|la Jarretière]] et [[cdx_b]]."),
            "Voir la Jarretière et cdx_b."
        );
    }
}
