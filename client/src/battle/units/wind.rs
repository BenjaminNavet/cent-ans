//! Wind of the battle: a direction fixed by the battle seed and a strength by weather
//! (`wind_for` of game/scripts/battle/battle_standards.gd, strengths of `data/fx/battle_finish.json`).
//! Banners lie downwind when seen from close, smoke drifts with it.

use bevy::prelude::*;

use crate::battle::scene::BattleSession;

#[derive(Resource, Clone, Copy, Debug)]
pub struct BattleWind {
    /// Unit direction the wind blows towards, on the ground plane: `(x, z)`.
    pub dir: Vec2,
    pub strength: f32,
    pub gust: f32,
}

impl Default for BattleWind {
    fn default() -> Self {
        Self {
            dir: Vec2::X,
            strength: 0.5,
            gust: 0.2,
        }
    }
}

impl BattleWind {
    /// Direction from the seed (`absi(seed * 2654435761) % 3600 / 3600 * TAU`).
    pub fn direction_for(seed: u64) -> Vec2 {
        let hashed = (seed as i64).wrapping_mul(2_654_435_761).unsigned_abs();
        let angle = (hashed % 3600) as f32 / 3600.0 * std::f32::consts::TAU;
        Vec2::new(angle.cos(), angle.sin())
    }

    pub fn load(data_dir: &std::path::Path, weather: &str, seed: u64) -> Self {
        let parsed = std::fs::read_to_string(data_dir.join("fx/battle_finish.json"))
            .ok()
            .and_then(|text| serde_json::from_str::<serde_json::Value>(&text).ok());
        let entry = parsed.as_ref().and_then(|v| v.get("wind")).and_then(|w| {
            w.get("by_weather")
                .and_then(|b| b.get(weather))
                .or_else(|| w.get("default"))
        });
        let read = |key: &str, fallback: f32| {
            entry
                .and_then(|e| e.get(key))
                .and_then(|v| v.as_f64())
                .map_or(fallback, |v| v as f32)
        };
        Self {
            dir: Self::direction_for(seed),
            strength: read("strength", 0.5),
            gust: read("gust", 0.2),
        }
    }

    /// Yaw (rotation about Y) that lays a cloth running along local +X in the wind's direction.
    pub fn downwind_yaw(&self) -> f32 {
        (-self.dir.y).atan2(self.dir.x)
    }
}

/// Reads the wind when a battle opens.
pub fn init_wind(mut commands: Commands, session: Res<BattleSession>) {
    let wind = crate::paths::find_root()
        .map(|root| {
            BattleWind::load(
                &crate::paths::data_dir(&root),
                &session.config.weather,
                session.seed,
            )
        })
        .unwrap_or_default();
    commands.insert_resource(wind);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn direction_is_deterministic_and_unit() {
        let a = BattleWind::direction_for(42);
        assert_eq!(a, BattleWind::direction_for(42));
        assert!((a.length() - 1.0).abs() < 1e-5);
        assert_ne!(a, BattleWind::direction_for(43));
    }

    #[test]
    fn cloth_points_downwind() {
        let wind = BattleWind {
            dir: Vec2::new(0.0, 1.0),
            ..default()
        };
        let along_x = Quat::from_rotation_y(wind.downwind_yaw()) * Vec3::X;
        assert!((along_x - Vec3::Z).length() < 1e-5, "{along_x}");
    }
}
