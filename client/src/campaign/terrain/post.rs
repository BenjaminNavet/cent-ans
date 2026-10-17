//! Post-tonemap adjustment of the Godot campaign environment (`campaign_map.tscn`:
//! `adjustment_contrast` 1.06, `adjustment_saturation` 1.08). Godot's `apply_bcs` runs on display
//! values after the tonemapper; the terrain shader used to approximate it before AgX. It is now a
//! fullscreen pass scheduled after Bevy's tonemapping on the campaign camera, so it also covers
//! water, armies and markers like the Godot environment did. It also carries the season colour
//! grade (saturation, contrast, lift, highlights of `campaign_lighting`): one grade for the whole
//! frame instead of the ground and the foliage grading themselves in their own shaders.

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
    /// Environment grade (`CampaignLight`): x saturation, y contrast, z shadow saturation, w strength.
    pub grade: Vec4,
    pub lift: Vec4,
    pub highlights: Vec4,
}

impl Default for CampaignAdjust {
    fn default() -> Self {
        CampaignAdjust {
            params: Vec4::new(CONTRAST, SATURATION, 0.0, 0.0),
            grade: Vec4::new(1.0, 1.0, 1.0, 0.0),
            lift: Vec4::ZERO,
            highlights: Vec4::ONE,
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
    app.add_systems(Update, (attach, sync_grade).chain());
}

/// Puts the pass on every campaign camera that lacks it.
fn attach(
    mut commands: Commands,
    cameras: Query<Entity, (With<CampaignCamera>, Without<CampaignAdjust>)>,
) {
    if crate::campaign::perf_switch::off("post") {
        return;
    }
    for camera in &cameras {
        commands
            .entity(camera)
            .try_insert(CampaignAdjust::default());
    }
}

/// Feeds the season / turn grade of the environment to the pass (the ground and the foliage no
/// longer grade themselves: `TerrainParams::grade.w` stays 0, see `lighting::apply_light`).
fn sync_grade(
    light: Res<super::lighting::CampaignLight>,
    mut passes: Query<&mut CampaignAdjust, With<CampaignCamera>>,
) {
    let grade = Vec4::new(
        light.saturation,
        light.contrast,
        light.shadow_saturation,
        light.strength,
    );
    let lift = light.lift.extend(0.0);
    let highlights = light.highlights.extend(0.0);
    for mut pass in &mut passes {
        if pass.grade != grade || pass.lift != lift || pass.highlights != highlights {
            pass.grade = grade;
            pass.lift = lift;
            pass.highlights = highlights;
        }
    }
}
