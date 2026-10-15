//! Widget gallery for visual review: `CENT_ANS_KIT_GALLERY=1` and screen `MainMenu`.
//!
//! Shows every kit widget on one page (typography, rich text, money, buttons in each state,
//! toggles, tabs, slider, list, panels, gauges, lettrine). `CENT_ANS_KIT_GALLERY_DEMO=1`
//! additionally opens a tooltip chain, queues two modals and fires two toasts, so that the dump
//! section `ui_kit` shows them.

use bevy::prelude::*;
use bevy::state::state_scoped::DespawnOnExit;

use super::icons::icon;
use super::modal::{ModalSpec, PRIORITY_DECISION, PRIORITY_REPORT, ToastRequest};
use super::richtext::RichStyle;
use super::style::{UiTier, UiType};
use super::tooltip::{TooltipRegistry, TooltipState};
use super::tooltip_spec::{
    TooltipSpec, TooltipStyle, TooltipTexts, register_rule_texts, tooltip_view,
};
use super::widgets::*;
use super::{UiTheme, fr, heading, rich_text, typed};
use crate::app::AppState;

const ENV_GALLERY: &str = "CENT_ANS_KIT_GALLERY";
const ENV_DEMO: &str = "CENT_ANS_KIT_GALLERY_DEMO";
const ICON: &str = "ui/medallions/agents.png";

fn env_on(name: &str) -> bool {
    std::env::var(name).is_ok_and(|value| value == "1")
}

pub struct GalleryPlugin;

impl Plugin for GalleryPlugin {
    fn build(&self, app: &mut App) {
        if env_on(ENV_GALLERY) {
            app.add_systems(OnEnter(AppState::MainMenu), spawn_gallery)
                .add_systems(Update, demo.run_if(in_state(AppState::MainMenu)));
        }
    }
}

const SAMPLE: &str = "Le [b]moral[/b] des [i]archers[/i] tombe de [color=#8b1a1a]−12[/color] ; voir [url=ib:rule:morale]moral[/url] et [url=ib:rule:fear]peur[/url]. [img=18x18]ui/medallions/agents.png[/img] Solde [color=#2a6a2a]+3 063 ₶[/color].";

const SPEC_JSON: &str = r#"{
  "id": "unit_knights", "kind": "unit", "title": "Chevaliers", "subtitle": "Cavalerie lourde",
  "icon": "unit_knights",
  "headline": [{"key": "strength", "label": "Effectif", "value": "120"},
               {"key": "melee", "stat": "melee", "label": "Mêlée", "value": "9", "sign": 1}],
  "stats": [{"key": "armor", "label": "Armure", "value": "7"}, {"key": "morale", "label": "Moral", "value": "8"}],
  "effects": [{"key": "army_morale", "label": "Moral des armées", "before": 60, "after": 65, "value": "+5", "sign": 1},
              {"text": "Entretien accru en hiver", "sign": -1}],
  "traits": {"strengths": ["charge"], "weaknesses": ["pluie"]},
  "requires": [{"text": "Écurie", "met": true}],
  "flavour": "Fleur de la chevalerie.",
  "footer": {"cost": "300 ₶", "time": "2 saisons"}
}"#;

fn spawn_gallery(
    mut commands: Commands,
    theme: Res<UiTheme>,
    assets: Res<AssetServer>,
    mut registry: ResMut<TooltipRegistry>,
) {
    registry.register(
        "ib:rule:morale",
        "[b]Moral[/b]\nTombe quand les pertes montent ; voir [url=ib:rule:fear]peur[/url].",
    );
    registry.register(
        "ib:rule:fear",
        "[b]Peur[/b]\nUne troupe effrayée rompt la ligne.",
    );
    if let Ok(texts) = TooltipTexts::load() {
        register_rule_texts(&mut registry, &texts, |_| None);
    }
    let theme = theme.clone();
    let body = RichStyle::body(&theme);
    commands
        .spawn((
            Name::new("KitGallery"),
            DespawnOnExit(AppState::MainMenu),
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                padding: UiRect::all(Val::Px(10.0)),
                column_gap: Val::Px(10.0),
                ..default()
            },
            BackgroundColor(Color::srgb(0.13, 0.10, 0.07)),
            UiTier::Panel.index(),
        ))
        .with_children(|root| {
            // Column 1: typography, rich text, money, gauges, separator.
            root.spawn(panel_sized(&theme, 340.0)).with_children(|col| {
                col.spawn(heading(&theme, "Typographie"));
                for variation in [
                    UiType::Title,
                    UiType::Heading,
                    UiType::Body,
                    UiType::Caption,
                ] {
                    col.spawn(typed(
                        &theme,
                        &format!("{} — Cent Ans ₶ ▲ ✓", variation.label()),
                        variation,
                    ));
                }
                col.spawn(separator(&theme));
                col.spawn(heading(&theme, "Texte riche"));
                col.spawn((
                    Node {
                        max_width: Val::Px(280.0),
                        ..default()
                    },
                    children![rich_text(&theme, &assets, SAMPLE, &body)],
                ));
                col.spawn(separator(&theme));
                col.spawn(heading(&theme, "Monnaie"));
                for value in [60_000, 3_063, -1_243, 0] {
                    col.spawn(typed(
                        &theme,
                        &format!(
                            "{} · {} · {}",
                            fr::amount(value),
                            fr::signed(value),
                            fr::delta(value)
                        ),
                        UiType::Body,
                    ));
                }
                col.spawn(typed(
                    &theme,
                    &format!(
                        "{} · {}",
                        fr::count(3, "tour", None),
                        fr::date(2026, 9, 24, Some((23, 19)))
                    ),
                    UiType::Caption,
                ));
                col.spawn(separator(&theme));
                col.spawn(heading(&theme, "Jauges"));
                for ratio in [0.9, 0.5, 0.15] {
                    col.spawn(gauge(&theme, "g:gauge", ratio, 200.0));
                }
                col.spawn((icon_chip(
                    &theme,
                    &assets,
                    ICON,
                    "Agents",
                    "[b]Agents[/b]\nSurvolez 0,5 s ; [url=ib:rule:morale]moral[/url] (Alt).",
                ),));
            });
            // Column 2: controls.
            root.spawn(panel_sized(&theme, 360.0)).with_children(|col| {
                col.spawn(heading(&theme, "Boutons"));
                col.spawn(button_captioned(&theme, "g:normal", "Normal"));
                col.spawn((
                    button_captioned(&theme, "g:disabled", "Désactivé"),
                    bevy::ui::InteractionDisabled,
                ));
                col.spawn(button_primary(&theme, "g:primary", "Confirmer"));
                col.spawn(button_icon(&theme, &assets, "g:icon", ICON, "Avec icône"));
                col.spawn(toggle_button(&theme, "g:toggle", "Bascule", true));
                col.spawn(checkbox(&theme, "g:check_on", "Case cochée", true));
                col.spawn(checkbox(&theme, "g:check_off", "Case vide", false));
                col.spawn(separator(&theme));
                col.spawn(heading(&theme, "Champ de texte"));
                col.spawn(super::text_field::text_field(
                    &theme, "g:field", "", "Nom…", 260.0,
                ));
                col.spawn(separator(&theme));
                col.spawn(heading(&theme, "Curseur"));
                col.spawn(slider(&theme, "g:slider", 0.0, 100.0, 40.0, 1.0, 260.0));
                col.spawn(separator(&theme));
                col.spawn(tab_bar(
                    &theme,
                    "g",
                    &[
                        ("g:tab_a", "Économie"),
                        ("g:tab_b", "Armée"),
                        ("g:tab_c", "Cour"),
                    ],
                    0,
                ));
                col.spawn(panel_inset(&theme)).with_children(|inset| {
                    for (name, text, visible) in [
                        ("g:tab_a", "Recettes et dépenses.", true),
                        ("g:tab_b", "Effectifs, moral.", false),
                        ("g:tab_c", "Conseillers.", false),
                    ] {
                        inset.spawn((
                            tab_pane("g", name, visible),
                            children![typed(&theme, text, UiType::Body)],
                        ));
                    }
                });
            });
            // Column 3: list, panels, lettrine.
            root.spawn(panel_sized(&theme, 340.0)).with_children(|col| {
                col.spawn(lettrine(&theme, "Écus et livres", 22.0));
                col.spawn(heading(&theme, "Liste défilante"));
                col.spawn(scroll_list(&theme, "g:list", 150.0))
                    .with_children(|list| {
                        for index in 0..14 {
                            list.spawn(list_item(
                                &theme,
                                "g:l",
                                &format!("g:item{index}"),
                                &format!("Bataille de démonstration {}", index + 1),
                                index == 1,
                            ));
                        }
                    });
                col.spawn(heading(&theme, "Cadres"));
                col.spawn((
                    panel_note(&theme),
                    children![typed(&theme, "Note marginale (infobulle)", UiType::Caption)],
                ));
                col.spawn((
                    card(&theme),
                    children![typed(
                        &theme,
                        "Carte plate (carte d'unité)",
                        UiType::Caption
                    )],
                ));
                col.spawn((
                    panel_illuminated(&theme),
                    children![typed(&theme, "Page enluminée", UiType::Heading)],
                ));
            });
            // Column 4: icons and the structured tooltip.
            root.spawn(panel_sized(&theme, 360.0)).with_children(|col| {
                col.spawn(heading(&theme, "Icônes"));
                col.spawn(Node {
                    column_gap: Val::Px(6.0),
                    flex_wrap: FlexWrap::Wrap,
                    row_gap: Val::Px(6.0),
                    ..default()
                })
                .with_children(|row| {
                    for name in [
                        "hud_treasury",
                        "unit_knights",
                        "agents",
                        "end_turn",
                        "diplomacy",
                        "no_such_icon",
                    ] {
                        row.spawn(icon(&theme, &assets, name, 32.0));
                    }
                });
                col.spawn(heading(&theme, "Infobulle structurée"));
                let spec = TooltipSpec::from_json(SPEC_JSON).unwrap_or_default();
                let texts = TooltipTexts::load().unwrap_or_default();
                let tooltip_style = TooltipStyle::load().unwrap_or_default();
                col.spawn(tooltip_view(
                    &theme,
                    &assets,
                    &spec,
                    &texts,
                    &TooltipStyle {
                        width_px: 300.0,
                        ..tooltip_style
                    },
                    true,
                ));
            });
        });
}

fn demo(
    mut frames: Local<u32>,
    mut state: ResMut<TooltipState>,
    mut queue: ResMut<super::modal::ModalQueue>,
    mut toasts: MessageWriter<ToastRequest>,
) {
    *frames += 1;
    if *frames != 30 || !env_on(ENV_DEMO) {
        return;
    }
    state.open_at(
        "[b]Archer à pied[/b]\nPortée longue ; [url=ib:rule:morale]moral[/url] fragile.",
        Vec2::new(640.0, 120.0),
        true,
    );
    state.open_child_of(0, "ib:rule:morale");
    queue.request(
        "demo_report",
        PRIORITY_REPORT,
        ModalSpec::info("Rapport de saison", "Le [b]printemps[/b] s'achève."),
    );
    queue.request(
        "demo_decision",
        PRIORITY_DECISION,
        ModalSpec::confirm("Décision", "Lever une taxe ?"),
    );
    toasts.write(ToastRequest::new("Partie sauvegardée"));
    toasts.write(ToastRequest::new("Nouvel avis du conseil"));
}
