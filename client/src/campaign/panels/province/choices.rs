//! Choice sections of the province panel: La Table (diets), regional edict, settlement vocation
//! and the possession texts. Ports game/scripts/ui/{province_choice_section, table_section,
//! edict_section, vocation_selector, possession_text}.gd. No rule here: availability, costs and
//! refusals come from `game_api::pprovince`; the orders are sent by `mod.rs`.

use bevy::prelude::*;
use game_api::pprovince::{
    DietOptionView, EdictOptionView, EdictView, NodeEffectView, PossessionView, TableView,
    TierRowView, VocationOptionView, VocationView,
};

use super::labels::{self, UiLabels};
use super::view::Kids;
use crate::ui::kit::icons::icon;
use crate::ui::kit::style::{self, UiType};
use crate::ui::kit::tooltip::Tooltip;
use crate::ui::kit::widgets::{button_small, caption, heading, separator};
use crate::ui::kit::{RichStyle, UiTheme, fr, rich_text, typed};

/// One row of the unfolded option list.
pub struct ChoiceOption {
    pub id: String,
    pub text: String,
    pub tooltip: String,
    pub locked: bool,
    /// Red note next to the option (refusal reason), empty for none.
    pub note: String,
}

/// A section « current option + Changer… + options ».
pub struct Choice<'a> {
    /// `table`, `edict` or `vocation`: button names are `prov:choose:<key>` and `prov:<key>:<id>`.
    pub key: &'a str,
    pub title: &'a str,
    /// Id of the current option (its icon, `IconLibrary` order), empty for none.
    pub current_id: String,
    pub current: String,
    pub description: String,
    /// `description` with Codex words linked (BBCode), set by the view when it has one.
    pub description_rich: Option<String>,
    pub tooltip: String,
    pub change_text: String,
    pub change_tooltip: String,
    pub change_locked: bool,
    pub options: Vec<ChoiceOption>,
}

fn row() -> Node {
    Node {
        column_gap: Val::Px(6.0),
        align_items: AlignItems::Center,
        width: Val::Percent(100.0),
        ..default()
    }
}

/// Text of the effects of a block: « Effet +5 (paysans) ; … ».
pub fn effects_text(labels: &UiLabels, effects: &[NodeEffectView]) -> String {
    effects
        .iter()
        .map(|e| {
            let scope = if e.class.is_empty() {
                String::new()
            } else {
                format!(" ({})", labels.class(&e.class).to_lowercase())
            };
            format!(
                "{} {}{scope}",
                labels.effect(&e.key),
                labels::signed_value(e.value, e.percent)
            )
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn effects_block(labels: &UiLabels, effects: &[NodeEffectView]) -> String {
    if effects.is_empty() {
        return String::new();
    }
    let mut block = String::from("\n\n[b]Effets[/b]");
    for effect in effects {
        block.push('\n');
        block.push_str(&effects_text(labels, std::slice::from_ref(effect)));
    }
    block
}

/// Builds one choice section; `editable`: the player may change it; `open`: list unfolded.
#[allow(clippy::too_many_arguments)]
pub fn choice_section(
    pane: &mut Kids,
    theme: &UiTheme,
    assets: &AssetServer,
    choice: &Choice,
    editable: bool,
    open: bool,
    banner: Option<&str>,
    rich_banner: Option<&str>,
) {
    pane.spawn(heading(theme, choice.title));
    if let Some(source) = rich_banner {
        pane.spawn((
            Name::new(format!("prov:{}_rich_banner", choice.key)),
            Node::default(),
        ))
        .with_children(|wrapper| {
            wrapper.spawn(rich_text(theme, assets, source, &RichStyle::caption(theme)));
        });
    }
    if let Some(banner) = banner {
        pane.spawn((
            Name::new(format!("prov:{}_banner", choice.key)),
            typed(theme, banner, UiType::Caption),
        ));
    }
    pane.spawn((
        Name::new(format!("prov:{}_current", choice.key)),
        Tooltip::new(&choice.tooltip),
        row(),
    ))
    .with_children(|line| {
        if !choice.current_id.is_empty() {
            line.spawn(icon(theme, assets, &choice.current_id, 18.0));
        }
        line.spawn(typed(theme, &choice.current, UiType::Body));
    });
    if let Some(source) = &choice.description_rich {
        pane.spawn((
            Name::new(format!("prov:{}_description", choice.key)),
            Node::default(),
        ))
        .with_children(|wrapper| {
            wrapper.spawn(rich_text(theme, assets, source, &RichStyle::caption(theme)));
        });
    } else if !choice.description.is_empty() {
        pane.spawn(caption(theme, &choice.description));
    }
    if !editable {
        return;
    }
    let mut button = pane.spawn((
        button_small(
            theme,
            &format!("prov:choose:{}", choice.key),
            &choice.change_text,
        ),
        Tooltip::new(&choice.change_tooltip),
    ));
    if choice.change_locked {
        button.insert(bevy::ui::InteractionDisabled);
    }
    if !open {
        return;
    }
    pane.spawn(Node {
        flex_direction: FlexDirection::Column,
        row_gap: Val::Px(2.0),
        width: Val::Percent(100.0),
        ..default()
    })
    .with_children(|list| {
        for option in &choice.options {
            list.spawn((
                Name::new(format!("prov:{}_row:{}", choice.key, option.id)),
                row(),
            ))
            .with_children(|line| {
                let mut button = line.spawn((
                    button_small(
                        theme,
                        &format!("prov:{}:{}", choice.key, option.id),
                        &option.text,
                    ),
                    Tooltip::new(&option.tooltip),
                ));
                let glyph = button
                    .commands()
                    .spawn(icon(theme, assets, &option.id, 16.0))
                    .id();
                button.insert_children(0, &[glyph]);
                if option.locked {
                    button.insert(bevy::ui::InteractionDisabled);
                }
                if !option.note.is_empty() {
                    line.spawn((
                        Node {
                            flex_shrink: 1.0,
                            ..default()
                        },
                        Text::new(option.note.clone()),
                        theme.text_font(false, false, UiType::Caption.size()),
                        TextColor(style::ERROR),
                    ));
                }
            });
        }
    });
    pane.spawn(separator(theme));
}

// ----- La Table --------------------------------------------------------------------------------

fn diet_cost(cost: i64) -> String {
    if cost > 0 {
        format!("{} / saison", fr::amount(cost))
    } else {
        "gratuit".to_owned()
    }
}

fn diet_tooltip(labels: &UiLabels, option: &DietOptionView) -> String {
    let mut tip = format!("[b]{}[/b]\nCoût : {}", option.name, diet_cost(option.cost));
    if !option.description.is_empty() {
        tip.push_str(&format!("\n[i]{}[/i]", option.description));
    }
    tip.push_str(&effects_block(labels, &option.effects));
    for reason in &option.reasons {
        tip.push_str(&format!("\n[color=#8b2a1a]{reason}[/color]"));
    }
    tip
}

pub fn table_choice(labels: &UiLabels, table: &TableView) -> Choice<'static> {
    let current = table.options.iter().find(|o| o.current);
    Choice {
        key: "table",
        title: "La Table",
        current_id: table.diet.clone(),
        current: format!("{} — {}", table.name, diet_cost(table.cost)),
        description: table.description.clone(),
        description_rich: None,
        tooltip: current.map_or_else(String::new, |o| diet_tooltip(labels, o)),
        change_text: "Changer de régime".to_owned(),
        change_tooltip: if table.changed_this_turn {
            "Un seul changement par province et par tour.".to_owned()
        } else {
            "Choisir la table de la province (effet à la fin du tour).".to_owned()
        },
        change_locked: table.changed_this_turn,
        options: table
            .options
            .iter()
            .map(|o| ChoiceOption {
                id: o.id.clone(),
                text: format!(
                    "{} — {}{}",
                    o.name,
                    diet_cost(o.cost),
                    if o.current { "  (actuel)" } else { "" }
                ),
                tooltip: diet_tooltip(labels, o),
                locked: o.current || table.changed_this_turn,
                note: if o.available || o.current {
                    String::new()
                } else {
                    o.reasons
                        .first()
                        .cloned()
                        .unwrap_or_else(|| "indisponible".to_owned())
                },
            })
            .collect(),
    }
}

/// Banner and « déjà changé » line of the Table, `None` when there is nothing to say.
pub fn table_banner(table: &TableView) -> Option<&'static str> {
    if table.changed_this_turn {
        Some("Régime déjà changé ce tour-ci (effet à la fin du tour).")
    } else {
        None
    }
}

/// Lent banner, BBCode: `{rule.*}` values and the `[[cdx_careme|jeûne]]` link are filled by
/// [`lent_text`] (game/scripts/ui/table_section.gd `LENT_TEXT`).
const LENT_TEXT: &str = "[b]Carême[/b] — quarante jours de [[cdx_careme|jeûne]] avant Pâques : une table de viande ou de laitages coûte {rule.lent_piety_penalty} de piété au souverain et fâche le clergé de la province (+{rule.lent_clergy_unrest} de mécontentement) ; le poisson de carême rapporte +{rule.lent_fish_piety} de piété.";

/// The full Lent text with the rule values of the core and its Codex link resolved.
pub fn lent_text(
    store: &crate::codex::store::CodexStore,
    constants: &std::collections::BTreeMap<String, f64>,
) -> String {
    crate::codex::text::format(
        store,
        &crate::codex::rules::fill_rule_values(LENT_TEXT, constants),
    )
}

// ----- Edicts ----------------------------------------------------------------------------------

fn edict_cost(option: &EdictOptionView) -> String {
    let mut parts = Vec::new();
    if option.cost_money > 0 {
        parts.push(format!("{} par saison", fr::amount(option.cost_money)));
    }
    if option.cost_prestige > 0 {
        parts.push(format!("{} de prestige (une fois)", option.cost_prestige));
    }
    parts.join(", ")
}

fn edict_tooltip(labels: &UiLabels, option: &EdictOptionView) -> String {
    let mut tip = format!("[b]{}[/b]", option.name);
    if option.delay_turns > 0 {
        tip.push_str(&format!(
            "\nDélai avant effet : {} tour(s)",
            option.delay_turns
        ));
    }
    let cost = edict_cost(option);
    if !cost.is_empty() {
        tip.push_str(&format!("\nCoût : {cost}"));
    }
    for (text, met) in &option.requirements {
        tip.push_str(&format!("\n{} {text}", if *met { "✓" } else { "✗" }));
    }
    tip.push_str(&effects_block(labels, &option.effects));
    tip
}

pub fn edict_choice(labels: &UiLabels, edict: &EdictView) -> Choice<'static> {
    let current = edict.options.iter().find(|o| o.current);
    Choice {
        key: "edict",
        title: "Édit régional",
        current_id: edict.edict.clone(),
        current: edict.name.clone(),
        description: edict.description.clone(),
        description_rich: None,
        tooltip: current.map_or_else(String::new, |o| edict_tooltip(labels, o)),
        change_text: "Changer d’édit".to_owned(),
        change_tooltip: "Adopter un édit pour la province (effet après un délai).".to_owned(),
        change_locked: false,
        options: edict
            .options
            .iter()
            .map(|o| {
                let cost = edict_cost(o);
                let mut text = o.name.clone();
                if !cost.is_empty() && !o.current {
                    text.push_str(&format!("  — {cost}"));
                }
                if o.current {
                    text.push_str(if o.active {
                        "  (actuel)"
                    } else {
                        "  (en attente)"
                    });
                }
                ChoiceOption {
                    id: o.id.clone(),
                    text,
                    tooltip: edict_tooltip(labels, o),
                    locked: o.current,
                    note: if o.available || o.current {
                        String::new()
                    } else if o.reason.is_empty() {
                        "indisponible".to_owned()
                    } else {
                        o.reason.clone()
                    },
                }
            })
            .collect(),
    }
}

/// « « X » entre en vigueur dans N tour(s)… » when a change is pending.
pub fn edict_pending_line(edict: &EdictView) -> Option<String> {
    edict.pending.then(|| {
        format!(
            "« {} » entre en vigueur dans {} tour(s) ; l’édit actuel s’applique en attendant.",
            edict.pending_name, edict.turns_left
        )
    })
}

// ----- Vocation --------------------------------------------------------------------------------

fn signed_percent(value: f64) -> String {
    let rounded = (value * 10.0).round() / 10.0;
    let sign = if rounded >= 0.0 { '+' } else { fr::MINUS };
    let body = if (rounded - rounded.round()).abs() < 1e-9 {
        format!("{}", rounded.abs().round() as i64)
    } else {
        format!("{:.1}", rounded.abs()).replace('.', ",")
    };
    format!("{sign}{body} %")
}

fn tiers_lines(labels: &UiLabels, tiers: &[TierRowView]) -> String {
    tiers
        .iter()
        .map(|t| {
            format!(
                "\n{} {} ({} bâtiments) : {}",
                if t.reached { "✔" } else { "·" },
                t.label,
                t.min_buildings,
                effects_text(labels, &t.effects)
            )
        })
        .collect()
}

pub fn vocation_button_text(view: &VocationView) -> String {
    if view.vocation.is_empty() {
        "Vocation : aucune".to_owned()
    } else {
        format!(
            "Vocation : {} ({}/{})",
            view.name,
            view.reached_tiers,
            view.tiers.len()
        )
    }
}

pub fn vocation_tooltip(labels: &UiLabels, view: &VocationView) -> String {
    let mut tip = String::new();
    if view.vocation.is_empty() {
        tip.push_str("Aucune vocation : aucun bonus ni surcoût.\nSpécialiser la colonie réduit le coût de ses bâtiments de vocation et renforce leurs effets.");
    } else {
        tip.push_str(&format!(
            "[b]{}[/b] — {}\nBâtiments de la vocation : effets {}, coût {} ; autres bâtiments : coût {}.\nCohérence : {} bâtiment(s) de la vocation.",
            view.name,
            view.description,
            signed_percent(view.effect_bonus_percent),
            signed_percent(-view.cost_reduction_percent),
            signed_percent(view.off_vocation_surcharge_percent),
            view.count
        ));
        tip.push_str(&tiers_lines(labels, &view.tiers));
        if let Some(missing) = view.missing_for_next {
            tip.push_str(&format!(
                "\nProchain palier : encore {missing} bâtiment(s)."
            ));
        }
        if !view.active_effects.is_empty() {
            tip.push_str(&format!(
                "\nEffets actuels : {}",
                effects_text(labels, &view.active_effects)
            ));
        }
    }
    if let Some(until) = view.locked_until {
        tip.push_str(&format!("\nChangement verrouillé jusqu'au tour {until}."));
    } else if let Some(option) = view.options.first() {
        tip.push_str(&format!(
            "\nChanger : {} l.t., puis verrouillé {} tours.",
            fr::digits(i64::from(option.cost)),
            option.lock_turns
        ));
    }
    tip
}

fn vocation_option_tooltip(labels: &UiLabels, option: &VocationOptionView) -> String {
    let mut tip = format!(
        "{}\nEffets des bâtiments concernés {}, coût {} ; hors vocation coût {}.",
        option.description,
        signed_percent(option.effect_bonus_percent),
        signed_percent(-option.cost_reduction_percent),
        signed_percent(option.off_vocation_surcharge_percent)
    );
    tip.push_str(&tiers_lines(labels, &option.tiers));
    if !option.preview_effects.is_empty() {
        tip.push_str(&format!(
            "\nAvec vos bâtiments actuels : {}",
            effects_text(labels, &option.preview_effects)
        ));
    }
    if !option.reason.is_empty() {
        tip.push_str(&format!(
            "\n[color=#8b2a1a]Impossible : {}[/color]",
            option.reason
        ));
    }
    tip
}

pub fn vocation_choice(labels: &UiLabels, view: &VocationView) -> Choice<'static> {
    Choice {
        key: "vocation",
        title: "Vocation",
        current_id: view.vocation.clone(),
        current: vocation_button_text(view),
        description: String::new(),
        description_rich: None,
        tooltip: vocation_tooltip(labels, view),
        change_text: "Changer de vocation".to_owned(),
        change_tooltip: vocation_tooltip(labels, view),
        change_locked: false,
        options: view
            .options
            .iter()
            .map(|o| {
                let mut text = o.name.clone();
                if o.current {
                    text.push_str(" (actuelle)");
                } else {
                    text.push_str(&format!(" — {} l.t.", fr::digits(i64::from(o.cost))));
                    if o.fit > 0 {
                        text.push_str(&format!(
                            " — {} bât. concordant{}",
                            o.fit,
                            if o.fit > 1 { "s" } else { "" }
                        ));
                    }
                }
                ChoiceOption {
                    id: o.id.clone(),
                    text,
                    tooltip: vocation_option_tooltip(labels, o),
                    locked: !o.available,
                    note: String::new(),
                }
            })
            .collect(),
    }
}

// ----- Possession ------------------------------------------------------------------------------

/// Status of a province in one line (`PossessionText.status_line`).
pub fn status_line(possession: &PossessionView) -> String {
    let owner = &possession.owner_name;
    let controller = &possession.controller_name;
    match possession.status.as_str() {
        "own" => "À vous".to_owned(),
        "own_occupied" => format!("À vous — occupée par {controller}"),
        "occupied_by_viewer" => format!(
            "Occupée par vous (propriétaire de droit : {owner}) — rendue à la paix si non cédée"
        ),
        "foreign" => format!("À {owner}"),
        "foreign_occupied" => format!("À {owner} — occupée par {controller}"),
        _ => String::new(),
    }
}

/// Ink of a stance category on parchment.
pub fn cue_color(cue: &str) -> Color {
    match cue {
        "self" => Color::srgb_u8(0x7a, 0x5a, 0x0c),
        "friend" => Color::srgb_u8(0x2a, 0x6a, 0x2a),
        "enemy" => Color::srgb_u8(0x8f, 0x15, 0x10),
        _ => style::INK,
    }
}

/// « de » elided before a vowel (« d’Andalousie »).
fn de(name: &str) -> String {
    if name
        .chars()
        .next()
        .is_some_and(|c| "AEIOUYÉÈÊÂÎÔaeiouyéèêâîô".contains(c))
    {
        format!("d’{name}")
    } else {
        format!("de {name}")
    }
}

/// Tooltip of the possession line: help, occupation reminder and held places.
pub fn possession_tooltip(possession: &PossessionView) -> String {
    let mut tip = format!(
        "La cité {} donne le contrôle de la province ; la possession s’obtient par traité (cession).",
        de(&possession.city_name)
    );
    if possession.occupied {
        tip.push_str("\nL’occupant tient la province tant que la guerre dure ; elle revient à son propriétaire à la paix si elle n’est pas cédée.");
    }
    tip.push_str(&format!(
        "\nPlaces tenues : {} sur {}",
        possession.held, possession.total
    ));
    if !possession.whole_holder.is_empty() {
        tip.push_str(" — province complète : bonus acquis");
    } else if possession.held > 0 && possession.held < possession.total {
        let rest = possession.total - possession.held;
        tip.push_str(&format!(
            " — encore {rest} place{} pour le bonus de province complète",
            if rest > 1 { "s" } else { "" }
        ));
    }
    tip
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(status: &str) -> PossessionView {
        PossessionView {
            status: status.into(),
            owner: "fac_a".into(),
            owner_name: "France".into(),
            controller: "fac_b".into(),
            controller_name: "Angleterre".into(),
            city_name: "Orléans".into(),
            occupied: status.contains("occupied"),
            held: 1,
            total: 3,
            whole_holder: String::new(),
            cue: "enemy".into(),
        }
    }

    #[test]
    fn possession_lines() {
        assert_eq!(status_line(&sample("own")), "À vous");
        assert_eq!(
            status_line(&sample("own_occupied")),
            "À vous — occupée par Angleterre"
        );
        assert_eq!(status_line(&sample("foreign")), "À France");
        let tip = possession_tooltip(&sample("foreign_occupied"));
        assert!(tip.contains("d’Orléans"));
        assert!(tip.contains("encore 2 places"));
    }

    #[test]
    fn percent_signs() {
        assert_eq!(signed_percent(10.0), "+10 %");
        assert_eq!(signed_percent(-2.5), "\u{2212}2,5 %");
    }
}

#[cfg(test)]
mod lent_tests {
    use super::*;

    #[test]
    fn lent_text_fills_rule_values_and_links_the_codex() {
        let store = crate::codex::store::CodexStore::default();
        let constants = [
            ("lent_piety_penalty".to_owned(), 2.0),
            ("lent_clergy_unrest".to_owned(), 1.5),
            ("lent_fish_piety".to_owned(), 1.0),
        ]
        .into_iter()
        .collect();
        let text = lent_text(&store, &constants);
        assert!(text.contains("coûte 2 de piété"), "{text}");
        assert!(text.contains("+1,5 de mécontentement"), "{text}");
        assert!(text.contains("+1 de piété"), "{text}");
        assert!(!text.contains("{rule"), "{text}");
        assert!(text.contains("jeûne"), "{text}");
    }
}
