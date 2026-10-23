//! Season colour of the rivers and lakes (EAU, ADR 0750): tint and bank ice from
//! `data/map/river_display.json` (`look.season_tint`, `look.season_ice`, order spring, summer,
//! autumn, winter), blended with the smoothed `SeasonWeights` of the ground so the water turns
//! with it. Rendering only.

use bevy::prelude::*;

use super::data::WaterLook;
use super::materials::RiverMaterial;
use crate::campaign::terrain::season::SeasonWeights;

/// Season look of the rivers, read once from the river display file.
#[derive(Resource, Debug, Clone, Copy, PartialEq)]
pub struct WaterSeasonLook {
    pub tint: [Vec3; 4],
    pub ice: Vec4,
}

impl WaterSeasonLook {
    pub fn from_look(look: &WaterLook) -> Self {
        Self {
            tint: look.season_tint.map(Vec3::from_array),
            ice: Vec4::from_array(look.season_ice),
        }
    }

    /// Water tint (rgb) and ice (w) for the season weights `w` (sum 1).
    pub fn blend(&self, w: Vec4) -> Vec4 {
        let tint = self
            .tint
            .iter()
            .enumerate()
            .fold(Vec3::ZERO, |acc, (i, t)| acc + *t * w[i]);
        tint.extend(self.ice.dot(w))
    }
}

/// Writes the blended season look into every river and lake material when it changes.
pub fn sync_water_season(
    look: Option<Res<WaterSeasonLook>>,
    weights: Res<SeasonWeights>,
    mut rivers: ResMut<Assets<RiverMaterial>>,
    mut last: Local<(Vec4, usize)>,
) {
    let Some(look) = look else {
        return;
    };
    let value = look.blend(weights.weights);
    let count = rivers.len();
    if last.0.abs_diff_eq(value, 1e-4) && last.1 == count {
        return;
    }
    *last = (value, count);
    for (_, material) in rivers.iter_mut() {
        material.params.season = value;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blend_follows_the_weights() {
        let look = WaterSeasonLook {
            tint: [
                Vec3::ONE,
                Vec3::splat(2.0),
                Vec3::splat(3.0),
                Vec3::splat(4.0),
            ],
            ice: Vec4::new(0.0, 0.0, 0.2, 0.6),
        };
        let winter = look.blend(Vec4::new(0.0, 0.0, 0.0, 1.0));
        assert_eq!(winter, Vec4::new(4.0, 4.0, 4.0, 0.6));
        let mid = look.blend(Vec4::new(0.0, 0.0, 0.5, 0.5));
        assert!((mid.x - 3.5).abs() < 1e-6 && (mid.w - 0.4).abs() < 1e-6);
    }
}
