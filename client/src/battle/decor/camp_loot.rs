//! Looted camps (port of `BattleDecor._loot` / `_process` of game/scripts/battle/battle_decor.gd):
//! the sim's camp state is read once a second; the first time a camp is looted its tents and
//! pavilions fall (tilted, squashed to 28 % of their height) and up to three columns of black
//! smoke rise over its fires (shared `SmokeSources`).

use bevy::prelude::*;
use sim_battle::{DecorPropKind, SideId};

use crate::app::AppState;
use crate::battle::scene::BattleSession;
use crate::battle::units::smoke::SmokeSources;

/// Seconds between two reads of the camp state.
const POLL_S: f32 = 1.0;
/// Smoke columns over a looted camp.
const SMOKE_COLUMNS: usize = 3;

/// A tent or pavilion of the camp of `side`.
#[derive(Component)]
pub struct CampTent {
    pub side: SideId,
}

#[derive(Resource, Default)]
pub struct CampLootStats {
    pub looted_camps: usize,
    pub tents_down: usize,
    pub smoke_columns: usize,
}

#[derive(Resource, Default)]
struct LootState {
    timer: f32,
    looted: [bool; 2],
}

pub struct CampLootPlugin;

impl Plugin for CampLootPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                init.run_if(resource_added::<BattleSession>),
                poll.run_if(resource_exists::<LootState>),
            )
                .chain()
                .run_if(in_state(AppState::Battle)),
        )
        .add_systems(OnExit(AppState::Battle), |mut commands: Commands| {
            commands.remove_resource::<CampLootStats>();
            commands.remove_resource::<LootState>();
        });
    }
}

fn init(mut commands: Commands) {
    commands.insert_resource(CampLootStats::default());
    commands.insert_resource(LootState::default());
}

/// Transform of a fallen tent (`_loot`): tilt about X, squash, sink 5 cm.
pub fn fallen(transform: &Transform) -> Transform {
    let tilt = Quat::from_rotation_x(0.35);
    Transform {
        translation: transform.translation - Vec3::Y * 0.05,
        rotation: transform.rotation * tilt,
        scale: transform.scale * Vec3::new(1.05, 0.28, 1.1),
    }
}

fn side_index(side: SideId) -> usize {
    match side {
        SideId::Attacker => 0,
        SideId::Defender => 1,
    }
}

fn poll(
    time: Res<Time>,
    mut state: ResMut<LootState>,
    mut stats: ResMut<CampLootStats>,
    mut session: ResMut<BattleSession>,
    mut tents: Query<(&CampTent, &mut Transform)>,
    mut smoke: Option<ResMut<SmokeSources>>,
) {
    state.timer += time.delta_secs();
    if state.timer < POLL_S {
        return;
    }
    state.timer = 0.0;
    let sim = session.sim_mut();
    for side in SideId::BOTH {
        let index = side_index(side);
        let looted = sim.camp_state(side).is_some_and(|c| c.looted);
        if !looted || state.looted[index] {
            continue;
        }
        state.looted[index] = true;
        stats.looted_camps += 1;
        for (tent, mut transform) in &mut tents {
            if tent.side == side {
                *transform = fallen(&transform);
                stats.tents_down += 1;
            }
        }
        let field = sim.field();
        let fires: Vec<Vec3> = field
            .decor
            .camp(side)
            .map(|camp| {
                camp.items
                    .iter()
                    .filter(|p| p.kind == DecorPropKind::Campfire)
                    .map(|p| Vec3::new(p.x as f32, field.height(p.x, p.z) as f32, p.z as f32))
                    .collect()
            })
            .unwrap_or_default();
        if let Some(smoke) = smoke.as_deref_mut() {
            for fire in fires.iter().take(SMOKE_COLUMNS) {
                if smoke.add(*fire + Vec3::Y * 1.5, 1.2, "column").is_some() {
                    stats.smoke_columns += 1;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fallen_tent_is_squashed_and_sunk() {
        let tent = Transform::from_xyz(1.0, 2.0, 3.0);
        let down = fallen(&tent);
        assert!((down.scale.y - 0.28).abs() < 1e-6);
        assert!((down.translation.y - 1.95).abs() < 1e-6);
    }
}
