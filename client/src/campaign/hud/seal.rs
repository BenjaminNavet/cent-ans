//! End-of-season seal and alert pills (bottom right), port of `ui/end_turn_cluster.gd`: a round
//! illuminated bell medallion with a parchment band « Printemps / 1337 » (« Décision / en
//! attente » with a rubric ring while a blocking alert waits), above it the column of alert
//! pills (icon, short label, count). The click is the kit's `ButtonPressed { "hud:end_turn" }`,
//! read by `flow.rs` as before.

use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::ui::{InteractionDisabled, Pressed};

use super::bar::KeyHints;
use super::flow::TurnFlow;
use super::{EndTurnButton, HudData, HudWidget};
use crate::ui::kit::UiTheme;
use crate::ui::kit::icons::icon_tinted;
use crate::ui::kit::style::{self, UiType};
use crate::ui::kit::tooltip::Tooltip;
use crate::ui::kit::widgets::panel_note;

/// Diameter of the seal.
pub const SEAL_SIZE: f32 = 104.0;
/// Width of the parchment band.
const BAND_WIDTH: f32 = 104.0;

/// First / second line of the seal band.
#[derive(Component)]
pub struct SealLine(pub u8);

/// Icon aliases of alert kinds (`EndTurnCluster.ICON_ALIASES`).
pub fn alert_icon(kind: &str) -> String {
    let alias = match kind {
        "diplomacy_offer" => "diplomacy",
        "research_idle" => "research",
        "mission_due" => "chronicle",
        "ransom" | "coinage" => "treasury",
        "idle_army" => "army",
        "free_slot" => "construction_done",
        "other" => "chronicle",
        other => other,
    };
    format!("hud_{alias}")
}

/// Two lines of the seal band: season / year, « Décision / en attente » when a blocking alert
/// waits, « Tour / en cours… » while the turn runs. Pure, tested.
pub fn seal_lines(date_label: Option<&str>, blocked: bool, running: bool) -> (String, String) {
    if running {
        return ("Tour".into(), "en cours…".into());
    }
    if blocked {
        return ("Décision".into(), "en attente".into());
    }
    let Some(label) = date_label else {
        return ("Fin de".into(), "saison".into());
    };
    let mut parts = label.split_whitespace();
    let first = parts.next().unwrap_or("Fin de saison").to_owned();
    let rest = parts.collect::<Vec<_>>().join(" ");
    (first, rest)
}

/// Builds the seal under `slot`.
pub fn spawn_seal(
    slot: &mut ChildSpawnerCommands,
    theme: &UiTheme,
    assets: &AssetServer,
    keys: &KeyHints,
) {
    let image = theme
        .icons
        .medallion("hud_end_turn")
        .map(|path| ImageNode::new(assets.load(path.to_owned())))
        .unwrap_or_default();
    let key = keys.label("campaign_end_turn");
    let fast = keys.label("campaign_end_turn_fast");
    let mut tip = String::from("[b]Fin de saison[/b]");
    if !key.is_empty() {
        tip.push_str(&format!(" ({key})"));
    }
    tip.push_str("\nLes ordres sont exécutés et la saison suivante commence.");
    if !fast.is_empty() {
        tip.push_str(&format!("\n{fast} : sans confirmation ni rejeu."));
    }
    slot.spawn((
        Name::new("hud:end_turn"),
        EndTurnButton,
        HudWidget,
        bevy::ui_widgets::Button,
        Hovered::default(),
        Tooltip::new(&tip),
        Node {
            width: Val::Px(SEAL_SIZE),
            height: Val::Px(SEAL_SIZE + 18.0),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            ..default()
        },
    ))
    .with_children(|seal| {
        seal.spawn((
            Name::new("hud:end_turn_medallion"),
            SealDisc,
            image,
            Node {
                width: Val::Px(SEAL_SIZE),
                height: Val::Px(SEAL_SIZE),
                border_radius: BorderRadius::MAX,
                ..default()
            },
            Outline::new(Val::Px(3.0), Val::Px(-2.0), Color::NONE),
            Pickable::IGNORE,
        ));
        // Band of parchment across the lower part of the disc.
        seal.spawn((panel_note(theme), Pickable::IGNORE))
            .insert(Node {
                position_type: PositionType::Absolute,
                top: Val::Px(SEAL_SIZE * 0.70),
                width: Val::Px(BAND_WIDTH),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                padding: UiRect::axes(Val::Px(4.0), Val::Px(3.0)),
                ..default()
            })
            .with_children(|band| {
                band.spawn((
                    SealLine(0),
                    Text::new("Fin de"),
                    theme.text_font(true, false, UiType::Caption.size()),
                    TextColor(style::INK),
                    Pickable::IGNORE,
                ));
                band.spawn((
                    SealLine(1),
                    Text::new("saison"),
                    theme.text_font(false, false, UiType::Caption.size() - 2.0),
                    TextColor(style::INK_SOFT),
                    Pickable::IGNORE,
                ));
            });
    });
}

/// The medallion image of the seal.
#[derive(Component)]
pub struct SealDisc;

/// Band text, disabled state while the turn runs, rubric ring while blocked, hover halo.
#[allow(clippy::type_complexity)]
pub fn update_seal(
    hud: Res<HudData>,
    flow: Res<TurnFlow>,
    mut commands: Commands,
    seal: Query<(Entity, &Hovered, Has<Pressed>, Has<InteractionDisabled>), With<EndTurnButton>>,
    mut disc: Query<(&mut ImageNode, &mut Outline), With<SealDisc>>,
    mut lines: Query<(&SealLine, &mut Text, &mut TextColor)>,
) {
    let Ok((button, hovered, pressed, disabled)) = seal.single() else {
        return;
    };
    let running = flow.running();
    let blocked = hud.readiness.as_ref().is_some_and(|r| r.blocked);
    if running != disabled {
        crate::ui::kit::widgets::set_disabled(&mut commands, button, running);
    }
    let date = hud.top.as_ref().map(|t| t.date_label.as_str());
    let (first, second) = seal_lines(date, blocked, running);
    for (line, mut content, mut color) in &mut lines {
        let wanted = if line.0 == 0 { &first } else { &second };
        if &content.0 != wanted {
            content.0 = wanted.clone();
        }
        let ink = match (running, blocked, line.0) {
            (true, _, _) => style::INK_FADED,
            (false, true, _) => style::RUBRIC,
            (false, false, 0) => style::INK,
            _ => style::INK_SOFT,
        };
        if color.0 != ink {
            color.0 = ink;
        }
    }
    if let Ok((mut image, mut outline)) = disc.single_mut() {
        let tint = if running {
            Color::srgb(0.55, 0.53, 0.50)
        } else if pressed {
            Color::srgb(0.82, 0.80, 0.76)
        } else {
            Color::WHITE
        };
        if image.color != tint {
            image.color = tint;
        }
        let ring = if blocked && !running {
            style::RUBRIC
        } else if hovered.get() && !running {
            style::GOLD_PALE
        } else {
            Color::NONE
        };
        if outline.color != ring {
            outline.color = ring;
        }
    }
}

/// One alert pill: framed parchment, icon (rubric when dangerous), short label, count box.
pub fn alert_pill(
    theme: &UiTheme,
    assets: &AssetServer,
    name: &str,
    kind: &str,
    label: &str,
    count: usize,
    danger: bool,
) -> impl Bundle {
    let ink = if danger { style::RUBRIC } else { style::INK };
    let icon_id = alert_icon(kind);
    let glyph = icon_tinted(theme, assets, &icon_id, 18.0, ink);
    (
        Name::new(name.to_owned()),
        bevy::ui_widgets::Button,
        Hovered::default(),
        AlertPill,
        Node {
            align_items: AlignItems::Center,
            column_gap: Val::Px(6.0),
            padding: UiRect::new(Val::Px(6.0), Val::Px(3.0), Val::Px(2.0), Val::Px(2.0)),
            min_width: Val::Px(156.0),
            border: UiRect::all(Val::Px(1.0)),
            border_radius: BorderRadius::all(Val::Px(2.0)),
            ..default()
        },
        BackgroundColor(style::PARCHMENT),
        BorderColor::all(if danger {
            style::RUBRIC
        } else {
            style::INK_SOFT
        }),
        BoxShadow::new(
            style::SHADOW,
            Val::Px(1.5),
            Val::Px(2.0),
            Val::ZERO,
            Val::Px(2.0),
        ),
        Outline::new(Val::Px(1.5), Val::ZERO, Color::NONE),
        children![
            (glyph, Pickable::IGNORE),
            (
                Text::new(label.to_owned()),
                theme.text_font(false, false, UiType::Caption.size()),
                TextColor(ink),
                Node {
                    flex_grow: 1.0,
                    ..default()
                },
                Pickable::IGNORE,
            ),
            (
                Node {
                    min_width: Val::Px(18.0),
                    padding: UiRect::axes(Val::Px(4.0), Val::Px(0.0)),
                    justify_content: JustifyContent::Center,
                    ..default()
                },
                BackgroundColor(style::INK),
                Pickable::IGNORE,
                children![(
                    Text::new(count.to_string()),
                    theme.text_font(true, false, UiType::Caption.size() - 1.0),
                    TextColor(style::PARCHMENT_LIGHT),
                    Pickable::IGNORE,
                )],
            ),
        ],
    )
}

/// Marker of the alert pills (hover halo).
#[derive(Component)]
pub struct AlertPill;

pub fn pill_visuals(mut pills: Query<(&Hovered, &mut Outline), With<AlertPill>>) {
    for (hovered, mut outline) in &mut pills {
        let color = if hovered.get() {
            style::GOLD
        } else {
            Color::NONE
        };
        if outline.color != color {
            outline.color = color;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn band_splits_the_date_and_shows_states() {
        assert_eq!(
            seal_lines(Some("Printemps 1337"), false, false),
            ("Printemps".into(), "1337".into())
        );
        assert_eq!(
            seal_lines(Some("Printemps 1337"), true, false).0,
            "Décision"
        );
        assert_eq!(seal_lines(Some("Printemps 1337"), true, true).0, "Tour");
        assert_eq!(seal_lines(None, false, false).0, "Fin de");
    }

    #[test]
    fn alert_icons_follow_godot_aliases() {
        assert_eq!(alert_icon("research_idle"), "hud_research");
        assert_eq!(alert_icon("siege"), "hud_siege");
        assert_eq!(alert_icon("idle_army"), "hud_army");
    }
}
