//! Season banner (`season_banner.gd`): an illuminated cartouche with the new season and year
//! ("Printemps 1338") enters, rests and leaves in 1.2 s above the middle of the screen, without
//! catching the mouse. Announced when a turn is resolved (`TurnAdvanced`). "Reduce motion": no
//! slide. Bevy UI has no per-node opacity, so there is no fade (slide and visibility only).

use bevy::prelude::*;
use bevy::state::state_scoped::DespawnOnExit;
use serde_json::{Value, json};

use super::model::{BANNER_TOTAL_SECONDS, banner_phase};
use crate::app::AppState;
use crate::campaign::hud::flow::TurnAdvanced;
use crate::sim::Sim;
use crate::ui::kit::modal::ReduceMotion;
use crate::ui::kit::style::{self, UiTier, UiType};
use crate::ui::kit::widgets::panel_illuminated;
use crate::ui::kit::{UiTheme, typed};

/// Position of the cartouche centre (share of the screen height), `CENTER_Y`.
const TOP_PERCENT: f32 = 26.0;

/// Banner state: the last announced text and the running animation.
#[derive(Resource, Debug, Default)]
pub struct SeasonBanner {
    pub last_text: String,
    pub announcements: u32,
    elapsed: Option<f32>,
}

#[derive(Component)]
struct BannerRoot;

#[derive(Component)]
struct BannerPanel;

fn announce(
    mut commands: Commands,
    mut advanced: MessageReader<TurnAdvanced>,
    sim: Res<Sim>,
    theme: Option<Res<UiTheme>>,
    mut banner: ResMut<SeasonBanner>,
    roots: Query<Entity, With<BannerRoot>>,
) {
    if advanced.read().count() == 0 {
        return;
    }
    let (Some(state), Some(theme)) = (sim.state.as_ref(), theme) else {
        return;
    };
    let text = state.date_label();
    banner.last_text.clone_from(&text);
    banner.announcements += 1;
    banner.elapsed = Some(0.0);
    for root in &roots {
        commands.entity(root).despawn();
    }
    commands
        .spawn((
            Name::new("SeasonBanner"),
            BannerRoot,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                width: Val::Percent(100.0),
                top: Val::Percent(TOP_PERCENT),
                justify_content: JustifyContent::Center,
                ..default()
            },
            Visibility::Hidden,
            UiTier::Banner.index(),
            DespawnOnExit(AppState::Campaign),
        ))
        .with_children(|root| {
            root.spawn((
                BannerPanel,
                panel_illuminated(&theme),
                UiTransform::default(),
            ))
            .with_children(|panel| {
                panel
                    .spawn(typed(&theme, &text, UiType::Title))
                    .insert(TextColor(style::RUBRIC));
            });
        });
}

fn animate(
    time: Res<Time<Real>>,
    reduce: Res<ReduceMotion>,
    mut banner: ResMut<SeasonBanner>,
    mut roots: Query<&mut Visibility, With<BannerRoot>>,
    mut panels: Query<&mut UiTransform, With<BannerPanel>>,
) {
    let Some(elapsed) = banner.elapsed else {
        return;
    };
    let elapsed = elapsed + time.delta_secs();
    let (visible, offset) = banner_phase(elapsed, reduce.0);
    for mut visibility in &mut roots {
        *visibility = if visible {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
    for mut transform in &mut panels {
        transform.translation = Val2::px(0.0, offset);
    }
    banner.elapsed = (elapsed < BANNER_TOTAL_SECONDS).then_some(elapsed);
}

pub fn dump(world: &World) -> Value {
    let banner = world.resource::<SeasonBanner>();
    json!({
        "last_text": banner.last_text,
        "announcements": banner.announcements,
        "running": banner.elapsed.is_some(),
    })
}

pub(super) fn build(app: &mut App) {
    app.init_resource::<SeasonBanner>().add_systems(
        Update,
        (announce, animate)
            .chain()
            .run_if(in_state(AppState::Campaign)),
    );
}
