//! The stage widget embedded in the council panel (ports the `SubViewportContainer` of
//! `council_chamber.gd`): the room image, the hover card, the « Charge vacante » ghost labels and
//! the mouse picking (hover = light up + card, click = select the office / open the sovereign).

use bevy::prelude::*;
use bevy::ui::RelativeCursorPosition;
use game_api::court::OfficeView;

use super::pick;
use super::{CouncilCamera, CouncilState, STAGE_PIXELS, council_open};
use crate::campaign::panels::court::CourtUi;
use crate::sim::Sim;
use crate::ui::kit::{UiTheme, caption};

/// Displayed size of the stage (same aspect as the 760x480 image).
pub const STAGE_SIZE: Vec2 = Vec2::new(460.0, 460.0 * 480.0 / 760.0);

#[derive(Component)]
pub struct CouncilStage;

/// Ghost label over the empty seat of a vacant office.
#[derive(Component)]
pub struct StageLabel {
    pub key: String,
}

#[derive(Component)]
pub struct StageCard;

#[derive(Component)]
pub struct StageCardText;

/// Spawns the stage under `parent`. Called by the court panel when it builds its offices tab.
pub fn spawn_stage(
    parent: &mut ChildSpawnerCommands,
    theme: &UiTheme,
    state: &CouncilState,
    offices: &[OfficeView],
) {
    parent
        .spawn((
            Name::new("council:stage"),
            CouncilStage,
            ImageNode::new(state.image.clone()),
            RelativeCursorPosition::default(),
            Node {
                width: Val::Px(STAGE_SIZE.x),
                height: Val::Px(STAGE_SIZE.y),
                flex_shrink: 0.0,
                overflow: Overflow::clip(),
                ..default()
            },
        ))
        .with_children(|stage| {
            for key in pick::SEAT_OFFICES {
                let vacant = offices
                    .iter()
                    .find(|o| o.id == key)
                    .is_none_or(|o| o.holder_id.is_empty());
                if !vacant {
                    continue;
                }
                stage.spawn((
                    Name::new(format!("council:vacant:{key}")),
                    StageLabel {
                        key: key.to_owned(),
                    },
                    Text::new("Charge vacante"),
                    TextFont {
                        font_size: bevy::text::FontSize::Px(13.0),
                        ..default()
                    },
                    TextColor(Color::srgba(0.85, 0.8, 0.7, 0.7)),
                    Node {
                        position_type: PositionType::Absolute,
                        ..default()
                    },
                    Pickable::IGNORE,
                    Visibility::Hidden,
                ));
            }
            stage
                .spawn((
                    Name::new("council:card"),
                    StageCard,
                    Node {
                        position_type: PositionType::Absolute,
                        padding: UiRect::axes(Val::Px(8.0), Val::Px(5.0)),
                        display: Display::None,
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.07, 0.06, 0.05, 0.88)),
                    Pickable::IGNORE,
                ))
                .with_children(|card| {
                    card.spawn((StageCardText, caption(theme, "")));
                });
        });
}

/// Hover and click on the stage image.
#[allow(clippy::too_many_arguments)]
pub fn input(
    mut ui: ResMut<CourtUi>,
    mut state: ResMut<CouncilState>,
    sim: Res<Sim>,
    mouse: Res<ButtonInput<MouseButton>>,
    stage: Query<&RelativeCursorPosition, With<CouncilStage>>,
    camera: Query<&Transform, With<CouncilCamera>>,
) {
    if !council_open(&ui) {
        return;
    }
    let (Ok(cursor), Ok(camera)) = (stage.single(), camera.single()) else {
        state.hovered = None;
        state.cursor = None;
        return;
    };
    let over = cursor.cursor_over.then_some(cursor.normalized).flatten();
    let Some(normalized) = over else {
        state.hovered = None;
        state.cursor = None;
        return;
    };
    let uv = normalized + Vec2::splat(0.5);
    state.cursor = Some(uv * STAGE_SIZE);
    let aspect = STAGE_PIXELS.x as f32 / STAGE_PIXELS.y as f32;
    let direction = pick::ray_direction(uv, pick::CAMERA_FOV_DEG, aspect, camera.rotation);
    let key = pick::pick_ray(camera.translation, direction).map(str::to_owned);
    if key != state.hovered {
        if key.is_some() {
            state.hovers += 1;
        }
        state.hovered = key.clone();
    }
    if mouse.just_pressed(MouseButton::Left)
        && let Some(key) = key
    {
        state.clicks += 1;
        state.last_click = Some(key.clone());
        if key == "ruler" {
            let ruler = sim
                .state
                .as_ref()
                .and_then(|s| s.factions.get(&sim.player))
                .and_then(|f| f.ruler.as_ref())
                .map(ToString::to_string);
            if let Some(id) = ruler {
                ui.open_character(&id, false);
            }
        } else if ui.office.as_deref() != Some(key.as_str()) {
            ui.office = Some(key);
            ui.dirty = true;
        }
    }
}

/// Card text of a seat (`show_card`): name, office, skill.
fn card_text(key: &str, ui: &CourtUi, sim: &Sim) -> String {
    if key == "ruler" {
        let faction = sim
            .data
            .as_ref()
            .and_then(|d| d.factions.get(&sim.player))
            .map_or_else(String::new, |f| f.name.display.clone());
        let name = sim
            .data
            .as_ref()
            .zip(sim.state.as_ref())
            .and_then(|(data, state)| {
                let id = state.factions.get(&sim.player)?.ruler.clone()?;
                Some(game_api::court::character_sheet(data, state, &id)?.name)
            })
            .unwrap_or(faction);
        return format!("{name}\nSouverain\nPréside le conseil");
    }
    let Some(office) = ui.snapshot.offices.iter().find(|o| o.id == key) else {
        return String::new();
    };
    if office.holder_id.is_empty() {
        format!("Charge vacante\n{}\nAucun effet", office.name)
    } else {
        format!(
            "{}\n{}\nCompétence {} ({})",
            office.holder_name, office.name, office.holder_skill, office.skill
        )
    }
}

/// Moves the card with the cursor and the vacant labels with the (swaying) camera.
#[allow(clippy::too_many_arguments)]
pub fn update_overlays(
    ui: Res<CourtUi>,
    sim: Res<Sim>,
    mut state: ResMut<CouncilState>,
    camera: Query<&Transform, With<CouncilCamera>>,
    mut cards: Query<&mut Node, With<StageCard>>,
    mut texts: Query<&mut Text, With<StageCardText>>,
    mut labels: Query<
        (&StageLabel, &mut Node, &mut Visibility),
        (Without<StageCard>, Without<CouncilCamera>),
    >,
    mut shown: Local<Option<String>>,
) {
    if !council_open(&ui) {
        return;
    }
    if *shown != state.hovered {
        *shown = state.hovered.clone();
        state.card = shown
            .as_deref()
            .map(|key| card_text(key, &ui, &sim))
            .unwrap_or_default();
        for mut text in &mut texts {
            text.0.clone_from(&state.card);
        }
    }
    for mut node in &mut cards {
        match (state.hovered.is_some(), state.cursor) {
            (true, Some(cursor)) => {
                let at = (cursor + Vec2::new(18.0, 18.0))
                    .clamp(Vec2::ZERO, (STAGE_SIZE - Vec2::new(190.0, 70.0)).max(Vec2::ZERO));
                node.display = Display::Flex;
                node.left = Val::Px(at.x);
                node.top = Val::Px(at.y);
            }
            _ => node.display = Display::None,
        }
    }
    let Ok(camera) = camera.single() else {
        return;
    };
    let aspect = STAGE_PIXELS.x as f32 / STAGE_PIXELS.y as f32;
    for (label, mut node, mut visibility) in &mut labels {
        let point = pick::seat_position(&label.key) + Vec3::new(0.0, 1.05, 0.0);
        match pick::project(point, camera.translation, camera.rotation, pick::CAMERA_FOV_DEG, aspect) {
            Some(uv) => {
                node.left = Val::Px(uv.x * STAGE_SIZE.x - 42.0);
                node.top = Val::Px(uv.y * STAGE_SIZE.y);
                *visibility = Visibility::Inherited;
            }
            None => *visibility = Visibility::Hidden,
        }
    }
}
