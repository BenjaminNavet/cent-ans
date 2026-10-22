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
use crate::ui::kit::icons::icon;
use crate::ui::kit::style;
use crate::ui::kit::tooltip::Tooltip;

/// Radius of the bell medallion (`EndTurnCluster.BUTTON_RADIUS`).
pub const BUTTON_RADIUS: f32 = 62.0;
/// Diameter of the seal (the whole medallion image, ornaments included).
pub const SEAL_SIZE: f32 = BUTTON_RADIUS * 2.0;
/// Share of the radius at which the gold ring of the image sits (shadow and blocked ring).
const RING_SHARE: f32 = 0.86;
/// Parchment band: 100 px wide, its top at half a radius below the centre.
const BAND_WIDTH: f32 = 100.0;
const BAND_TOP: f32 = BUTTON_RADIUS * 1.5;
/// Width of the rubric ring drawn around the disc while a decision blocks.
const BLOCKED_RING: f32 = 3.5;

/// First / second line of the seal band.
#[derive(Component)]
pub struct SealLine(pub u8);

/// The parchment band of the seal.
#[derive(Component)]
pub struct SealBand;

/// The rubric ring shown while a blocking decision waits.
#[derive(Component)]
pub struct SealBlockedRing;

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
        return ("Fin de tour".into(), String::new());
    };
    let mut parts = label.split_whitespace();
    let first = parts.next().unwrap_or("Fin de tour").to_owned();
    let rest = parts.collect::<Vec<_>>().join(" ");
    (first, rest)
}

/// State of the medallion (`_draw_medallion`): one image per state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SealState {
    Normal,
    Hover,
    Pressed,
    Disabled,
}

impl SealState {
    /// `[brightness, contrast, saturation]` of `IconLibrary.medallion_states` (none for normal).
    pub fn bcs(self) -> Option<[f32; 3]> {
        match self {
            Self::Normal => None,
            Self::Hover => Some([1.12, 1.05, 1.08]),
            Self::Pressed => Some([0.82, 1.05, 1.0]),
            Self::Disabled => Some([0.95, 0.85, 0.0]),
        }
    }
}

/// `Image.adjust_bcs` of Godot on RGBA8 pixels (alpha kept): brightness scales, contrast
/// pulls away from mid-grey, saturation mixes from the channel mean. Pure, tested.
pub fn adjust_bcs(pixels: &mut [u8], [brightness, contrast, saturation]: [f32; 3]) {
    for px in pixels.as_chunks_mut::<4>().0 {
        let mut rgb = [px[0], px[1], px[2]].map(|c| f32::from(c) / 255.0 * brightness);
        rgb = rgb.map(|c| 0.5 + (c - 0.5) * contrast);
        let mean = (rgb[0] + rgb[1] + rgb[2]) / 3.0;
        rgb = rgb.map(|c| mean + (c - mean) * saturation);
        for (dst, c) in px.iter_mut().zip(rgb) {
            *dst = (c.clamp(0.0, 1.0) * 255.0).round() as u8;
        }
    }
}

/// The four images of the medallion; the derived ones are built once the source is loaded.
#[derive(Resource, Default)]
pub struct SealImages {
    pub normal: Handle<Image>,
    /// Hover, pressed, disabled.
    pub derived: Option<[Handle<Image>; 3]>,
}

impl SealImages {
    pub fn get(&self, state: SealState) -> Handle<Image> {
        let index = match state {
            SealState::Normal => return self.normal.clone(),
            SealState::Hover => 0,
            SealState::Pressed => 1,
            SealState::Disabled => 2,
        };
        self.derived
            .as_ref()
            .map_or_else(|| self.normal.clone(), |d| d[index].clone())
    }
}

/// Derives the hover / pressed / disabled images from the loaded medallion.
pub fn derive_seal_images(seal: Option<ResMut<SealImages>>, mut images: ResMut<Assets<Image>>) {
    let Some(mut seal) = seal else { return };
    if seal.derived.is_some() || seal.normal == Handle::default() {
        return;
    }
    let Some(source) = images.get(&seal.normal) else {
        return;
    };
    let Some(data) = source.data.clone() else {
        return;
    };
    if source.texture_descriptor.format.block_copy_size(None) != Some(4) {
        warn!("seal: unexpected medallion format, no derived states");
        seal.derived = Some([
            seal.normal.clone(),
            seal.normal.clone(),
            seal.normal.clone(),
        ]);
        return;
    }
    let base = source.clone();
    let derived = [SealState::Hover, SealState::Pressed, SealState::Disabled].map(|state| {
        let mut pixels = data.clone();
        adjust_bcs(&mut pixels, state.bcs().unwrap_or([1.0; 3]));
        let mut image = base.clone();
        image.data = Some(pixels);
        images.add(image)
    });
    seal.derived = Some(derived);
}

/// Builds the seal under `slot`.
pub fn spawn_seal(
    slot: &mut ChildSpawnerCommands,
    theme: &UiTheme,
    assets: &AssetServer,
    keys: &KeyHints,
) {
    let normal: Handle<Image> = theme
        .icons
        .medallion("hud_end_turn")
        .map(|path| assets.load(path.to_owned()))
        .unwrap_or_default();
    slot.commands().insert_resource(SealImages {
        normal: normal.clone(),
        derived: None,
    });
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
    let shadow = BUTTON_RADIUS * RING_SHARE;
    let ring = BUTTON_RADIUS * RING_SHARE + 2.0 + BLOCKED_RING * 0.5;
    slot.spawn((
        Name::new("hud:end_turn"),
        EndTurnButton,
        HudWidget,
        bevy::ui_widgets::Button,
        Hovered::default(),
        Tooltip::new(&tip),
        Node {
            width: Val::Px(SEAL_SIZE),
            height: Val::Px(BAND_TOP + 34.0),
            ..default()
        },
    ))
    .with_children(|seal| {
        // Shadow of the disc, under the gold ring of the image.
        seal.spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(BUTTON_RADIUS - shadow + 2.0),
                top: Val::Px(BUTTON_RADIUS - shadow + 3.0),
                width: Val::Px(shadow * 2.0),
                height: Val::Px(shadow * 2.0),
                border_radius: BorderRadius::MAX,
                ..default()
            },
            BackgroundColor(style::SHADOW),
            Pickable::IGNORE,
        ));
        // The whole medallion image, ornaments included (no clipping, no added ring).
        seal.spawn((
            Name::new("hud:end_turn_medallion"),
            SealDisc,
            ImageNode::new(normal),
            Node {
                position_type: PositionType::Absolute,
                left: Val::ZERO,
                top: Val::ZERO,
                width: Val::Px(SEAL_SIZE),
                height: Val::Px(SEAL_SIZE),
                ..default()
            },
            Pickable::IGNORE,
        ));
        seal.spawn((
            Name::new("hud:end_turn_blocked_ring"),
            SealBlockedRing,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(BUTTON_RADIUS - ring),
                top: Val::Px(BUTTON_RADIUS - ring),
                width: Val::Px(ring * 2.0),
                height: Val::Px(ring * 2.0),
                border: UiRect::all(Val::Px(BLOCKED_RING)),
                border_radius: BorderRadius::MAX,
                ..default()
            },
            BorderColor::all(Color::NONE),
            Visibility::Hidden,
            Pickable::IGNORE,
        ));
        // Parchment band across the lower part of the disc.
        seal.spawn((
            Name::new("hud:end_turn_band"),
            SealBand,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px((SEAL_SIZE - BAND_WIDTH) * 0.5),
                top: Val::Px(BAND_TOP),
                width: Val::Px(BAND_WIDTH),
                height: Val::Px(34.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                padding: UiRect::top(Val::Px(1.0)),
                border: UiRect::all(Val::Px(1.5)),
                ..default()
            },
            BackgroundColor(style::PARCHMENT),
            BorderColor::all(style::INK_SOFT),
            BoxShadow::new(
                style::SHADOW,
                Val::Px(1.5),
                Val::Px(2.0),
                Val::ZERO,
                Val::ZERO,
            ),
            Pickable::IGNORE,
        ))
        .with_children(|band| {
            band.spawn((
                SealLine(0),
                Text::new("Fin de tour"),
                theme.text_font(false, false, 14.0),
                TextColor(style::INK),
                Node {
                    height: Val::Px(16.0),
                    ..default()
                },
                Pickable::IGNORE,
            ));
            band.spawn((
                SealLine(1),
                Text::new(""),
                theme.text_font(false, false, 12.0),
                TextColor(style::INK_SOFT),
                Pickable::IGNORE,
            ));
            // Gold rule along the bottom of the band.
            band.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(1.5),
                    right: Val::Px(1.5),
                    bottom: Val::Px(1.5),
                    height: Val::Px(1.0),
                    ..default()
                },
                BackgroundColor(style::GOLD),
                Pickable::IGNORE,
            ));
        });
    });
}

/// Vertical gold rule between the alert pills and the bell (hidden without pills).
#[derive(Component)]
pub struct AlertRule;

pub fn alert_rule() -> impl Bundle {
    (
        Name::new("hud:alert_rule"),
        AlertRule,
        Node {
            width: Val::Px(1.5),
            height: Val::Px(8.0),
            // Centred on the bell: the seal slot is `SEAL_SIZE + 8` wide with a 4 px right margin.
            margin: UiRect::right(Val::Px(4.0 + (SEAL_SIZE + 8.0) * 0.5 - 0.75)),
            display: Display::None,
            ..default()
        },
        BackgroundColor(style::GOLD),
        Pickable::IGNORE,
    )
}

/// Shows the rule only while the alert column has pills.
pub fn sync_alert_rule(
    alerts: Query<&Children, With<super::AlertList>>,
    mut rule: Query<&mut Node, With<AlertRule>>,
) {
    let (Ok(mut node), Ok(children)) = (rule.single_mut(), alerts.single()) else {
        return;
    };
    let wanted = if children.is_empty() {
        Display::None
    } else {
        Display::Flex
    };
    if node.display != wanted {
        node.display = wanted;
    }
}

/// The medallion image of the seal.
#[derive(Component)]
pub struct SealDisc;

/// Band text and height, medallion state image, rubric ring while blocked.
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub fn update_seal(
    hud: Res<HudData>,
    flow: Res<TurnFlow>,
    images: Option<Res<SealImages>>,
    mut commands: Commands,
    seal: Query<(Entity, &Hovered, Has<Pressed>, Has<InteractionDisabled>), With<EndTurnButton>>,
    mut disc: Query<&mut ImageNode, With<SealDisc>>,
    mut ring: Query<(&mut Visibility, &mut BorderColor), With<SealBlockedRing>>,
    mut band: Query<
        (&mut Node, &mut BackgroundColor, &mut BorderColor),
        (With<SealBand>, Without<SealBlockedRing>),
    >,
    mut lines: Query<(&SealLine, &mut Text, &mut TextColor)>,
    modals: Res<crate::ui::kit::modal::ModalQueue>,
    paused: Res<crate::menu::pause::GamePaused>,
) {
    let Ok((button, hovered, pressed, disabled)) = seal.single() else {
        return;
    };
    let running = flow.running();
    // `end_turn_cluster.gd::refresh_modal`: the bell is also off while a modal window or the
    // pause menu is open.
    let inactive = running || modals.is_busy() || paused.0;
    let blocked = hud.readiness.as_ref().is_some_and(|r| r.blocked);
    if inactive != disabled {
        crate::ui::kit::widgets::set_disabled(&mut commands, button, inactive);
    }
    let hover = hovered.get() && !inactive;
    let date = hud.top.as_ref().map(|t| t.date_label.as_str());
    let (first, second) = seal_lines(date, blocked, running);
    for (line, mut content, mut color) in &mut lines {
        let wanted = if line.0 == 0 { &first } else { &second };
        if &content.0 != wanted {
            content.0 = wanted.clone();
        }
        let ink = match (running, blocked, line.0) {
            (true, _, 0) => style::INK_FADED,
            (_, true, _) => style::RUBRIC,
            (false, false, 0) => style::INK,
            _ => style::INK_SOFT,
        };
        if color.0 != ink {
            color.0 = ink;
        }
    }
    if let Ok((mut node, mut face, mut border)) = band.single_mut() {
        let height = Val::Px(if second.is_empty() { 20.0 } else { 34.0 });
        if node.height != height {
            node.height = height;
        }
        let fill = if hover {
            style::PARCHMENT_LIGHT
        } else {
            style::PARCHMENT
        };
        face.set_if_neq(BackgroundColor(fill));
        let edge = if blocked {
            style::RUBRIC
        } else {
            style::INK_SOFT
        };
        border.set_if_neq(BorderColor::all(edge));
    }
    if let Ok((mut visibility, mut border)) = ring.single_mut() {
        visibility.set_if_neq(if blocked {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        });
        border.set_if_neq(BorderColor::all(style::RUBRIC));
    }
    let state = if inactive {
        SealState::Disabled
    } else if pressed {
        SealState::Pressed
    } else if hover {
        SealState::Hover
    } else {
        SealState::Normal
    };
    if let (Some(images), Ok(mut image)) = (images, disc.single_mut()) {
        let wanted = images.get(state);
        if image.image != wanted {
            image.image = wanted;
        }
    }
}

/// Size of an alert pill (`PILL_WIDTH` × `PILL_HEIGHT`) and gap between pills.
pub const PILL_WIDTH: f32 = 190.0;
pub const PILL_HEIGHT: f32 = 30.0;
pub const PILL_GAP: f32 = 4.0;
/// Radius of the icon disc at the left of a pill (`BADGE_RADIUS * 0.8`).
const PILL_DISC: f32 = 17.0 * 0.8;

/// Face of a pill: parchment, or red wax for a danger kind; lighter on hover.
fn pill_face(danger: bool, quiet: bool, hover: bool) -> Color {
    match (danger, hover) {
        (false, false) if quiet => style::PARCHMENT_DARK,
        (true, true) => style::WAX_LIGHT,
        (true, false) => style::WAX,
        (false, true) => style::PARCHMENT_LIGHT,
        (false, false) => style::PARCHMENT,
    }
}

/// One alert pill (`EndTurnCluster.AlertBadge._draw`): parchment band (wax for a danger),
/// ink frame and a coloured left rule, the kind's icon on a gold-ringed disc, the short label and
/// the count in a cartouche at the right.
pub fn alert_pill(
    theme: &UiTheme,
    assets: &AssetServer,
    name: &str,
    kind: &str,
    label: &str,
    count: usize,
    tone: AlertPill,
) -> impl Bundle {
    let AlertPill { danger, quiet } = tone;
    let ink = if danger {
        style::PARCHMENT_LIGHT
    } else if quiet {
        style::INK_SOFT
    } else {
        style::INK
    };
    let glyph = icon(theme, assets, &alert_icon(kind), PILL_DISC * 1.5);
    (
        Name::new(name.to_owned()),
        bevy::ui_widgets::Button,
        Hovered::default(),
        tone,
        Node {
            width: Val::Px(PILL_WIDTH),
            height: Val::Px(PILL_HEIGHT),
            align_items: AlignItems::Center,
            padding: UiRect::new(Val::Px(8.0), Val::Px(5.0), Val::ZERO, Val::ZERO),
            border: UiRect::new(Val::Px(3.0), Val::Px(1.0), Val::Px(1.0), Val::Px(1.0)),
            ..default()
        },
        BackgroundColor(pill_face(danger, quiet, false)),
        BorderColor {
            left: if danger { style::GOLD } else { style::RUBRIC },
            ..BorderColor::all(if danger {
                style::WAX_DARK
            } else {
                style::INK_SOFT
            })
        },
        BoxShadow::new(
            style::SHADOW,
            Val::Px(1.0),
            Val::Px(2.0),
            Val::ZERO,
            Val::ZERO,
        ),
        children![
            (
                Node {
                    width: Val::Px(PILL_DISC * 2.0),
                    height: Val::Px(PILL_DISC * 2.0),
                    flex_shrink: 0.0,
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    border: UiRect::all(Val::Px(1.0)),
                    border_radius: BorderRadius::MAX,
                    margin: UiRect::left(Val::Px(-3.0)),
                    ..default()
                },
                BackgroundColor(style::PARCHMENT_LIGHT),
                BorderColor::all(style::GOLD),
                Pickable::IGNORE,
                children![(glyph, Pickable::IGNORE)],
            ),
            (
                Text::new(label.to_owned()),
                theme.text_font(false, false, 15.0),
                TextColor(ink),
                TextLayout {
                    linebreak: LineBreak::NoWrap,
                    ..default()
                },
                Node {
                    flex_grow: 1.0,
                    margin: UiRect::left(Val::Px(7.0)),
                    overflow: Overflow::clip(),
                    ..default()
                },
                Pickable::IGNORE,
            ),
            (
                Node {
                    height: Val::Px(PILL_HEIGHT - 10.0),
                    padding: UiRect::axes(Val::Px(4.0), Val::ZERO),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    flex_shrink: 0.0,
                    ..default()
                },
                BackgroundColor(if danger {
                    style::PARCHMENT_LIGHT
                } else {
                    style::INK
                }),
                Pickable::IGNORE,
                children![(
                    Text::new(count.to_string()),
                    theme.text_font(false, false, 15.0),
                    TextColor(if danger {
                        style::WAX_DARK
                    } else {
                        style::PARCHMENT_LIGHT
                    }),
                    Pickable::IGNORE,
                )],
            ),
        ],
    )
}

/// Marker of the alert pills (lighter face on hover).
#[derive(Component)]
pub struct AlertPill {
    pub danger: bool,
    pub quiet: bool,
}

pub fn pill_visuals(mut pills: Query<(&AlertPill, &Hovered, &mut BackgroundColor)>) {
    for (pill, hovered, mut face) in &mut pills {
        face.set_if_neq(BackgroundColor(pill_face(
            pill.danger,
            pill.quiet,
            hovered.get(),
        )));
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
        assert_eq!(seal_lines(None, false, false).0, "Fin de tour");
    }

    #[test]
    fn bcs_matches_godot_adjust_bcs() {
        let mut px = [128u8, 64, 32, 200];
        adjust_bcs(&mut px, SealState::Disabled.bcs().unwrap());
        assert_eq!(px[0], px[1]);
        assert_eq!(px[1], px[2]);
        assert_eq!(px[3], 200, "alpha kept");
        let mut dark = [200u8, 200, 200, 255];
        adjust_bcs(&mut dark, SealState::Pressed.bcs().unwrap());
        assert!(dark[0] < 200);
        let mut light = [100u8, 100, 100, 255];
        adjust_bcs(&mut light, SealState::Hover.bcs().unwrap());
        assert!(light[0] > 100);
    }

    #[test]
    fn alert_icons_follow_godot_aliases() {
        assert_eq!(alert_icon("research_idle"), "hud_research");
        assert_eq!(alert_icon("siege"), "hud_siege");
        assert_eq!(alert_icon("idle_army"), "hud_army");
    }
}
