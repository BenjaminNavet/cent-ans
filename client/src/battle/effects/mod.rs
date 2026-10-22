//! Battle effects that follow the regiments and the time of day (port of the dust and charge
//! impact parts of game/scripts/battle/battle_effects.gd and of the night glow of
//! battle_smoke.gd):
//! - [`dust`]: dust kicked up by marching, charging and routing regiments (dry ground, no rain
//!   or snow), dust columns of big troops far away, the cloud at the contact of a charge;
//! - [`campfires`]: the camp fire smoke sources (decor camps, else behind the lines);
//! - [`glow`]: the camp fires of the smoke sources light the night (point lights, flicker).
//!
//! The puffs are the player's smoke flipbook sprites (`crate::fx::flipbook`), tinted earth.
//! - [`trail`]: ribbon trails of the arrows in flight (`battle_projectile_trail.gdshader`);
//! - [`ford`]: spray, wakes and entry bursts of the regiments in water;
//! - [`flash`]: bombard muzzle flash (sprites and a short point light);
//! - [`impact`]: ember sparks where melee blows land (lot B2);
//! - [`siege_lights`]: point lights over the burning houses and gate of a siege.
//!
//! Not ported: the clods thrown by the hooves (`Clods` pool).

pub mod campfires;
pub mod dust;
pub mod flash;
pub mod ford;
pub mod glow;
pub mod impact;
pub mod siege_lights;
pub mod trail;

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
            trail::TrailPlugin,
            ford::FordPlugin,
            flash::FlashPlugin,
            impact::ImpactPlugin,
            siege_lights::SiegeLightsPlugin,
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
