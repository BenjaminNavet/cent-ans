//! Render scale of the 3D world (« Mise à l'échelle », ADR 0123 ported to Bevy, ADR 0740): a
//! world camera marked [`ScaledWorldCamera`] renders into an image of `scale` x the final target
//! (window or `OffscreenTarget`), shown full screen under the UI by the UI camera (bilinear).
//!
//! The image is declared with the matching `scale_factor`, so the camera's *logical* viewport stays
//! the window size: cursor rays, `world_to_viewport` and every UI anchor keep their coordinates.
//! The scale comes from `data/fx/render_quality.json` § `upscale`: « Automatique » keeps the pixel
//! count of the 1080p reference (`reference_pixels`, at least `min_scale`), the other choices are
//! fixed scales; `CENT_ANS_RENDER_SCALE=<s>` forces one (benches).

use std::collections::HashMap;

use bevy::camera::{ImageRenderTarget, RenderTarget};
use bevy::image::{ImageSampler, ImageSamplerDescriptor};
use bevy::picking::Pickable;
use bevy::prelude::*;
use bevy::render::render_resource::TextureFormat;
use bevy::window::PrimaryWindow;
use serde_json::Value;

use super::Settings;
use crate::agent::OffscreenTarget;

/// Scale below which the world is rendered to its own image (above it, straight to the target).
const FULL_SCALE: f32 = 0.98;

/// Marks a 3D camera whose world is rendered at the « Mise à l'échelle » scale.
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct ScaledWorldCamera;

/// `upscale` block of `render_quality.json`.
#[derive(Resource, Debug, Clone, PartialEq)]
pub struct UpscaleTable {
    pub reference_pixels: f32,
    pub min_scale: f32,
    /// Fixed scale of each non-automatic choice (`off`, `quality`, `performance`).
    pub choices: HashMap<String, f32>,
}

impl Default for UpscaleTable {
    fn default() -> Self {
        UpscaleTable {
            reference_pixels: 1920.0 * 1080.0,
            min_scale: 0.4,
            choices: HashMap::from([
                ("off".to_owned(), 1.0),
                ("quality".to_owned(), 0.75),
                ("performance".to_owned(), 0.5),
            ]),
        }
    }
}

impl UpscaleTable {
    pub fn from_json(block: &Value) -> UpscaleTable {
        let d = UpscaleTable::default();
        let num = |key: &str, fallback: f32| block[key].as_f64().map_or(fallback, |v| v as f32);
        let mut choices = d.choices.clone();
        if let Some(map) = block["player_choices"].as_object() {
            for (name, choice) in map {
                let scale = if choice["mode"].as_str() == Some("off") {
                    1.0
                } else {
                    choice["scale"].as_f64().unwrap_or(1.0) as f32
                };
                choices.insert(name.clone(), scale.clamp(0.25, 1.0));
            }
        }
        UpscaleTable {
            reference_pixels: num("reference_pixels", d.reference_pixels).max(1.0),
            min_scale: num("min_scale", d.min_scale).clamp(0.25, 1.0),
            choices,
        }
    }

    fn load() -> UpscaleTable {
        crate::paths::find_root()
            .ok()
            .and_then(|root| std::fs::read_to_string(root.join("data/fx/render_quality.json")).ok())
            .and_then(|text| serde_json::from_str::<Value>(&text).ok())
            .map_or_else(UpscaleTable::default, |tree| {
                UpscaleTable::from_json(&tree["upscale"])
            })
    }

    /// Scale of the world for the setting `choice` on a target of `pixels` physical pixels.
    pub fn scale(&self, choice: &str, pixels: f32) -> f32 {
        match self.choices.get(choice) {
            Some(scale) => *scale,
            // « Automatique » (and any unknown value): the pixel count of the reference.
            None => (self.reference_pixels / pixels.max(1.0))
                .sqrt()
                .clamp(self.min_scale, 1.0),
        }
    }
}

/// The scaled image of a camera and what it replaced.
#[derive(Component)]
struct ScaledTarget {
    image: Handle<Image>,
    size: UVec2,
    original: RenderTarget,
    display: Entity,
}

/// Full-screen node showing a scaled world image.
#[derive(Component)]
pub struct WorldUpscaleNode;

/// What the last update chose, for the dump.
#[derive(Resource, Debug, Default, Clone, Copy, PartialEq)]
pub struct RenderScaleState {
    pub scale: f32,
    pub size: UVec2,
}

pub struct RenderScalePlugin;

impl Plugin for RenderScalePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(UpscaleTable::load())
            .init_resource::<RenderScaleState>()
            .add_systems(
                PostUpdate,
                (sync_scaled_cameras, cleanup_orphans)
                    .chain()
                    .before(bevy::camera::CameraUpdateSystems),
            );
    }
}

/// Forced scale (`CENT_ANS_RENDER_SCALE`), for benches.
fn forced_scale() -> Option<f32> {
    std::env::var("CENT_ANS_RENDER_SCALE")
        .ok()?
        .parse::<f32>()
        .ok()
        .map(|s| s.clamp(0.25, 1.0))
}

#[allow(clippy::too_many_arguments)]
fn sync_scaled_cameras(
    mut commands: Commands,
    settings: Option<Res<Settings>>,
    table: Res<UpscaleTable>,
    offscreen: Option<Res<OffscreenTarget>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut images: ResMut<Assets<Image>>,
    mut cameras: Query<
        (Entity, &mut RenderTarget, Option<&mut ScaledTarget>),
        With<ScaledWorldCamera>,
    >,
    mut nodes: Query<&mut ImageNode, With<WorldUpscaleNode>>,
    mut state: ResMut<RenderScaleState>,
) {
    if cameras.is_empty() {
        return;
    }
    // Physical size of the final target.
    let final_size = match &offscreen {
        Some(target) => images.get(&target.0).map(|image| image.size()),
        None => windows
            .single()
            .ok()
            .map(|w| UVec2::new(w.physical_width(), w.physical_height())),
    };
    let Some(final_size) = final_size.filter(|s| s.x > 0 && s.y > 0) else {
        return;
    };
    let choice = settings.as_ref().map_or("auto", |s| s.upscale.as_str());
    let scale =
        forced_scale().unwrap_or_else(|| table.scale(choice, (final_size.x * final_size.y) as f32));
    let want = (final_size.as_vec2() * scale)
        .round()
        .as_uvec2()
        .max(UVec2::ONE);
    let new_state = RenderScaleState {
        scale,
        size: if scale < FULL_SCALE { want } else { final_size },
    };
    if *state != new_state {
        *state = new_state;
    }
    for (entity, mut target, scaled) in &mut cameras {
        if scale >= FULL_SCALE {
            if let Some(scaled) = scaled {
                *target = scaled.original.clone();
                commands.entity(scaled.display).despawn();
                commands.entity(entity).remove::<ScaledTarget>();
            }
            continue;
        }
        if scaled.as_ref().is_some_and(|s| s.size == want) {
            continue;
        }
        let mut image =
            Image::new_target_texture(want.x, want.y, TextureFormat::Rgba8UnormSrgb, None);
        image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor::linear());
        let handle = images.add(image);
        let render_target = RenderTarget::Image(ImageRenderTarget {
            handle: handle.clone(),
            // Logical size of the image = physical size of the final target (window logical
            // pixels with the scale factor override of 1): cursor rays stay in window pixels.
            scale_factor: want.x as f32 / final_size.x as f32,
        });
        match scaled {
            Some(mut scaled) => {
                if let Ok(mut node) = nodes.get_mut(scaled.display) {
                    node.image = handle.clone();
                }
                scaled.image = handle;
                scaled.size = want;
            }
            None => {
                let display = commands
                    .spawn((
                        Name::new("WorldUpscale"),
                        WorldUpscaleNode,
                        Node {
                            position_type: PositionType::Absolute,
                            left: Val::Px(0.0),
                            top: Val::Px(0.0),
                            width: Val::Percent(100.0),
                            height: Val::Percent(100.0),
                            ..default()
                        },
                        ImageNode::new(handle.clone()),
                        // Under every other node; never hovered (world input tests the hover map).
                        GlobalZIndex(i32::MIN),
                        Pickable::IGNORE,
                    ))
                    .id();
                // The display goes with the camera (leaving the screen despawns both).
                commands.entity(entity).insert(ScaledTarget {
                    image: handle,
                    size: want,
                    original: target.clone(),
                    display,
                });
            }
        }
        *target = render_target;
    }
}

/// Despawns the display node of a camera that went away.
fn cleanup_orphans(
    mut commands: Commands,
    nodes: Query<Entity, With<WorldUpscaleNode>>,
    cameras: Query<&ScaledTarget>,
) {
    for node in &nodes {
        if !cameras.iter().any(|c| c.display == node) {
            commands.entity(node).despawn();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auto_keeps_the_reference_pixel_count() {
        let table = UpscaleTable::default();
        let s = table.scale("auto", 3456.0 * 2234.0);
        let pixels = 3456.0 * 2234.0 * s * s;
        assert!((pixels - table.reference_pixels).abs() / table.reference_pixels < 0.01);
        assert_eq!(table.scale("auto", 1280.0 * 800.0), 1.0);
        assert_eq!(table.scale("auto", 8000.0 * 8000.0), table.min_scale);
    }

    #[test]
    fn fixed_choices_from_the_data() {
        let block = serde_json::json!({
            "reference_pixels": 2073600, "min_scale": 0.4,
            "player_choices": {
                "off": {"mode": "off", "scale": 1.0},
                "quality": {"mode": "metalfx_spatial", "scale": 0.75},
                "performance": {"mode": "metalfx_spatial", "scale": 0.5}
            }
        });
        let table = UpscaleTable::from_json(&block);
        assert_eq!(table.scale("off", 1e7), 1.0);
        assert_eq!(table.scale("quality", 1e7), 0.75);
        assert_eq!(table.scale("performance", 1e7), 0.5);
    }
}
