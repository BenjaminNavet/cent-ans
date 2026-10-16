//! Battle effects that follow the regiments and the time of day (port of the dust and charge
//! impact parts of game/scripts/battle/battle_effects.gd and of the night glow of
//! battle_smoke.gd):
//! - [`dust`]: dust kicked up by marching, charging and routing regiments (dry ground, no rain
//!   or snow), dust columns of big troops far away, the cloud at the contact of a charge;
//! - [`campfires`]: the camp fire smoke sources (decor camps, else behind the lines);
//! - [`glow`]: the camp fires of the smoke sources light the night (point lights, flicker).
//!
//! The puffs are the player's smoke flipbook sprites (`crate::fx::flipbook`), tinted earth.
//! Not ported: the splashes and wakes at fords, the clods thrown by hooves, the cannon flash and
//! the legacy arrow layer with its ribbon trails (`battle_projectile_trail.gdshader`: volleys
//! draw the arrows, siege stones have their own smoke trail in `siege/fx.rs`).

pub mod campfires;
pub mod dust;
pub mod glow;

use bevy::prelude::*;

/// Deterministic value in `[0, 1)` from an integer seed (no random stream).
pub fn hash01(n: u64) -> f32 {
    let mut z = n
        .wrapping_mul(0x9E37_79B9_7F4A_7C15)
        .wrapping_add(0x1234_5678);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^= z >> 31;
    (z >> 40) as f32 / (1u64 << 24) as f32
}

pub struct BattleEffectsPlugin;

impl Plugin for BattleEffectsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            campfires::CampfirePlugin,
            dust::DustPlugin,
            glow::GlowPlugin,
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_is_deterministic_and_in_range() {
        assert_eq!(hash01(7), hash01(7));
        assert_ne!(hash01(7), hash01(8));
        assert!((0..200).all(|n| (0.0..1.0).contains(&hash01(n))));
    }
}
