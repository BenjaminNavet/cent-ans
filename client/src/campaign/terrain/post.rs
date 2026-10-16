//! Post-tonemap adjustment of the Godot campaign environment (`campaign_map.tscn`:
//! `adjustment_contrast` 1.06, `adjustment_saturation` 1.08). Godot's `apply_bcs` runs on display
//! values after the tonemapper; the terrain shader used to approximate it before AgX. It is now a
//! fullscreen pass scheduled after Bevy's tonemapping on the campaign camera, so it also covers
//! water, armies and markers like the Godot environment did.

use bevy::core_pipeline::fullscreen_material::{FullscreenMaterial, FullscreenMaterialPlugin};
use bevy::prelude::*;
use bevy::render::RenderApp;
use bevy::render::extract_component::ExtractComponent;
use bevy::render::render_resource::ShaderType;
use bevy::shader::ShaderRef;

use crate::campaign::camera::CampaignCamera;

/// `adjustment_contrast` of the campaign environment.
pub const CONTRAST: f32 = 1.06;
/// `adjustment_saturation` of the campaign environment.
pub const SATURATION: f32 = 1.08;

/// Post-tonemap contrast / saturation pass of the campaign camera.
#[derive(Component, ExtractComponent, Clone, Copy, Debug, PartialEq, ShaderType)]
#[extract_app(RenderApp)]
pub struct CampaignAdjust {
    /// x: contrast, y: saturation, zw: unused.
    pub params: Vec4,
}

impl Default for CampaignAdjust {
    fn default() -> Self {
        CampaignAdjust {
            params: Vec4::new(CONTRAST, SATURATION, 0.0, 0.0),
        }
    }
}

impl FullscreenMaterial for CampaignAdjust {
    fn fragment_shader() -> ShaderRef {
        "embedded://cent_ans_client/campaign/terrain/post.wesl".into()
    }
    fn schedule_configs(
        system: bevy::ecs::schedule::ScheduleConfigs<bevy::ecs::system::BoxedSystem>,
    ) -> bevy::ecs::schedule::ScheduleConfigs<bevy::ecs::system::BoxedSystem> {
        system
            .in_set(bevy::core_pipeline::Core3dSystems::PostProcess)
            .after(bevy::core_pipeline::tonemapping::tonemapping)
    }
}

pub fn add_plugin(app: &mut App) {
    bevy::asset::embedded_asset!(app, "post.wesl");
    app.add_plugins(FullscreenMaterialPlugin::<CampaignAdjust>::default());
    app.add_systems(Update, attach);
}

/// Puts the pass on every campaign camera that lacks it.
fn attach(
    mut commands: Commands,
    cameras: Query<Entity, (With<CampaignCamera>, Without<CampaignAdjust>)>,
) {
    for camera in &cameras {
        commands
            .entity(camera)
            .try_insert(CampaignAdjust::default());
    }
}
