//! Melee impact sparks (lot B2). When the core reports casualties of a melee, charge or pike
//! exchange (`Unit::loss_cause`), the figure nearest to the killers throws a few ember sprites
//! (the player's fire flipbook, additive) where the blow lands. No new asset; the sprites share
//! the particle budget of `fx::flipbook` and are refused above `sparks.sprite_room`, so that
//! fires and smoke keep their room. Settings: `data/fx/battle_impact.json`.

use bevy::prelude::*;
use serde::Deserialize;

use crate::agent::dump::DumpAppExt;
use crate::app::AppState;
use crate::battle::scene::BattleSession;
use crate::fx::flipbook::{FlipbookKind, FlipbookLooks, SmokeSprite, spawn_fire_sprite};

#[derive(Debug, Clone, Deserialize)]
pub struct Engage {
    pub range_m: f32,
    #[allow(dead_code)]
    pub min_men: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Sparks {
    pub causes: Vec<String>,
    pub per_impact: u32,
    pub max_impacts_per_step: usize,
    pub lifetime_s: [f32; 2],
    pub speed_m_s: [f32; 2],
    pub size_m: [f32; 2],
    pub height_m: f32,
    pub sprite_room: usize,
    pub min_interval_s: f32,
}

/// `data/fx/battle_impact.json`.
#[derive(Resource, Debug, Clone, Deserialize)]
pub struct ImpactSettings {
    pub engage: Engage,
    pub sparks: Sparks,
}

impl Default for ImpactSettings {
    fn default() -> Self {
        Self {
            engage: Engage {
                range_m: 7.0,
                min_men: 2,
            },
            sparks: Sparks {
                causes: vec!["melee".into(), "charge".into(), "pikes".into()],
                per_impact: 3,
                max_impacts_per_step: 4,
                lifetime_s: [0.25, 0.5],
                speed_m_s: [1.5, 4.5],
                size_m: [0.12, 0.28],
                height_m: 1.2,
                sprite_room: 600,
                min_interval_s: 0.12,
            },
        }
    }
}

impl ImpactSettings {
    pub fn load(data_dir: &std::path::Path) -> Self {
        std::fs::read_to_string(data_dir.join("fx/battle_impact.json"))
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default()
    }
}

/// Blows of the last steps waiting for their sparks, and what was done with them.
#[derive(Resource, Default)]
pub struct MeleeImpacts {
    pending: Vec<(Vec3, usize)>,
    pub spawned: usize,
    pub dropped: usize,
    since_last_s: f32,
    serial: u64,
}

impl MeleeImpacts {
    /// A blow landed at `point`; `men` fell.
    pub fn push(&mut self, point: Vec3, men: usize) {
        if self.pending.len() < 64 {
            self.pending.push((point, men));
        }
    }

    pub fn is_empty(&self) -> bool {
        self.pending.is_empty()
    }
}

fn lerp(range: [f32; 2], t: f32) -> f32 {
    range[0] + (range[1] - range[0]) * t
}

/// Sparks thrown by one impact: `(velocity, life, size)` of each, deterministic from `seed`.
pub fn spark_set(settings: &Sparks, seed: u64) -> Vec<(Vec3, f32, f32)> {
    (0..settings.per_impact)
        .map(|k| {
            let h = |n: u64| super::hash01(seed.wrapping_mul(16).wrapping_add(k as u64 * 4 + n));
            let angle = h(0) * std::f32::consts::TAU;
            let speed = lerp(settings.speed_m_s, h(1));
            let velocity = Vec3::new(angle.cos() * speed, 1.0 + 1.5 * h(2), angle.sin() * speed);
            (
                velocity,
                lerp(settings.lifetime_s, h(3)),
                lerp(settings.size_m, h(1)),
            )
        })
        .collect()
}

fn spawn_sparks(
    mut commands: Commands,
    session: Res<BattleSession>,
    time: Res<Time>,
    settings: Res<ImpactSettings>,
    mut impacts: ResMut<MeleeImpacts>,
    looks: Option<ResMut<FlipbookLooks>>,
) {
    let dt = if session.paused {
        0.0
    } else {
        time.delta_secs().min(0.1) * session.speed as f32
    };
    impacts.since_last_s += dt;
    if impacts.pending.is_empty() {
        return;
    }
    let Some(mut looks) = looks else {
        impacts.pending.clear();
        return;
    };
    let wanted: Vec<(Vec3, usize)> = std::mem::take(&mut impacts.pending);
    let mut spawned_here = 0;
    for (point, _men) in wanted {
        if spawned_here >= settings.sparks.max_impacts_per_step
            || impacts.since_last_s < settings.sparks.min_interval_s
            || looks.live() >= settings.sparks.sprite_room
        {
            impacts.dropped += 1;
            continue;
        }
        impacts.serial += 1;
        let seed = impacts.serial;
        for (k, (velocity, life, size)) in spark_set(&settings.sparks, seed).into_iter().enumerate()
        {
            let sprite = SmokeSprite {
                position: point,
                velocity,
                accel: Vec3::new(0.0, -9.0, 0.0),
                damping: 1.5,
                life,
                size,
                spin: 0.0,
                spin_rate: 0.0,
                tint: None,
            };
            let phase = super::hash01(seed * 7 + k as u64);
            if let Some(entity) = spawn_fire_sprite(
                &mut commands,
                &mut looks,
                FlipbookKind::Ember,
                &sprite,
                phase,
            ) {
                commands
                    .entity(entity)
                    .insert(DespawnOnExit(AppState::Battle));
                impacts.spawned += 1;
            }
        }
        spawned_here += 1;
        impacts.since_last_s = 0.0;
    }
}

fn dump_impacts(world: &World) -> (String, serde_json::Value) {
    let impacts = world.get_resource::<MeleeImpacts>();
    let frames = world.get_resource::<crate::battle::units::PoseFrames>();
    (
        "battle_melee_impact".into(),
        serde_json::json!({
            "sparks_spawned": impacts.map(|i| i.spawned),
            "impacts_dropped": impacts.map(|i| i.dropped),
            "figures_engaged": frames.map(|f| f.engaged),
        }),
    )
}

pub struct ImpactPlugin;

impl Plugin for ImpactPlugin {
    fn build(&self, app: &mut App) {
        let settings = crate::paths::find_root()
            .map(|root| ImpactSettings::load(&crate::paths::data_dir(&root)))
            .unwrap_or_default();
        app.insert_resource(settings)
            .init_resource::<MeleeImpacts>()
            .register_dump(dump_impacts)
            .add_systems(
                Update,
                spawn_sparks
                    .run_if(in_state(AppState::Battle))
                    .run_if(resource_exists::<BattleSession>)
                    .after(crate::battle::scene::StepSet),
            );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_data_file_loads_and_sparks_stay_inside_their_ranges() {
        let root = crate::paths::find_root().expect("root");
        let settings = ImpactSettings::load(&crate::paths::data_dir(&root));
        assert!(settings.engage.range_m > 0.0);
        assert!(settings.sparks.sprite_room < crate::fx::flipbook::MAX_PARTICLES);
        let set = spark_set(&settings.sparks, 42);
        assert_eq!(set.len(), settings.sparks.per_impact as usize);
        for (velocity, life, size) in &set {
            assert!(
                *life >= settings.sparks.lifetime_s[0] && *life <= settings.sparks.lifetime_s[1]
            );
            assert!(*size >= settings.sparks.size_m[0] && *size <= settings.sparks.size_m[1]);
            assert!(velocity.y > 0.0);
        }
        assert_eq!(set, spark_set(&settings.sparks, 42), "deterministic");
    }

    #[test]
    fn pending_blows_are_capped() {
        let mut impacts = MeleeImpacts::default();
        for _ in 0..200 {
            impacts.push(Vec3::ZERO, 1);
        }
        assert_eq!(impacts.pending.len(), 64);
    }
}
