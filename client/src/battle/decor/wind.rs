//! Wind sway: an `ExtendedMaterial` whose vertex stage (`wind.wesl`) moves grass cards and tree
//! crowns with the time (two sines and a slow gust, values in `battle_decor_look.json`); `wind_prepass.wesl` gives the
//! prepass and the shadows the same sway (`wind_passes`), both over `wind_common.wesl`.
//! Port of the vertex part of game/shaders/battle_grass.gdshader and the wind of
//! battle_vegetation.gd.

use bevy::asset::embedded_asset;
use bevy::pbr::{ExtendedMaterial, MaterialExtension};
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, ShaderType};
use bevy::shader::ShaderRef;

use super::look::{WindLook, WindPasses, wind_direction};

#[derive(Clone, Copy, Debug, ShaderType)]
pub struct WindParams {
    pub sway: Vec4,
    pub shape: Vec4,
    /// Rectangle (x, z, width, depth) covered by the flatten map; zero width = no map.
    pub flatten_rect: Vec4,
}

impl WindParams {
    /// `mode`: 0 grass card (weight from the card's v), 1 crown (weight from the height).
    pub fn new(wind: &WindLook, mode: f32, height_ref: f32) -> Self {
        let dir = wind_direction(wind);
        WindParams {
            sway: Vec4::new(
                wind.amplitude_m,
                wind.speed,
                wind.gust_amplitude_m,
                wind.gust_speed,
            ),
            shape: Vec4::new(dir.x, dir.y, mode, height_ref),
            flatten_rect: Vec4::ZERO,
        }
    }
}

impl Default for WindParams {
    fn default() -> Self {
        WindParams {
            sway: Vec4::ZERO,
            shape: Vec4::new(1.0, 0.0, 0.0, 1.0),
            flatten_rect: Vec4::ZERO,
        }
    }
}

#[derive(Asset, TypePath, AsBindGroup, Clone, Debug, Default)]
pub struct WindExtension {
    #[uniform(100)]
    pub params: WindParams,
    /// RG8 map of the flattened (R) and bloodied (G) grass, see `flatten.rs`.
    #[texture(101)]
    #[sampler(102)]
    pub flatten: Option<Handle<Image>>,
}

impl MaterialExtension for WindExtension {
    fn vertex_shader() -> ShaderRef {
        // Dev A/B switch (`CENT_ANS_PERF_OFF=wind`): Bevy's own vertex stage, no sway.
        if wind_switched_off() {
            return ShaderRef::Default;
        }
        "embedded://cent_ans_client/battle/decor/wind.wesl".into()
    }
    /// Depth/normal/motion prepass (SSAO, depth of field, motion blur): off by default, because
    /// the base prepass vertex stage does not sway and the still cards' depth hid the ground
    /// behind the swaying ones. On only with `wind_in_prepass` (the swaying stage below then
    /// serves it, so the depths agree). Shadows are independent of this switch.
    fn enable_prepass() -> bool {
        WindPasses::current().wind_in_prepass
    }
    /// Vertex stage of the prepass AND of the shadow passes (Bevy runs the same one): the swaying
    /// one, so the shadows and the prepass follow the foliage. The fragment stage stays the base
    /// material's (alpha mask discard).
    fn prepass_vertex_shader() -> ShaderRef {
        prepass_vertex_ref(WindPasses::current(), wind_switched_off())
    }
}

/// Which prepass/shadow vertex stage the wind material uses.
fn prepass_vertex_ref(passes: WindPasses, switched_off: bool) -> ShaderRef {
    if switched_off || !passes.uses_wind_vertex() {
        return ShaderRef::Default;
    }
    "embedded://cent_ans_client/battle/decor/wind_prepass.wesl".into()
}

pub type WindMaterial = ExtendedMaterial<StandardMaterial, WindExtension>;

/// True when `CENT_ANS_PERF_OFF` lists `wind` (dev A/B bench of the sway's vertex cost).
fn wind_switched_off() -> bool {
    std::env::var("CENT_ANS_PERF_OFF")
        .is_ok_and(|list| list.split(',').any(|name| name.trim() == "wind"))
}

pub struct WindPlugin;

impl Plugin for WindPlugin {
    fn build(&self, app: &mut App) {
        bevy::shader::load_shader_library!(app, "wind_common.wesl");
        embedded_asset!(app, "wind.wesl");
        embedded_asset!(app, "wind_prepass.wesl");
        app.add_plugins(MaterialPlugin::<WindMaterial>::default());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn is_default(shader: &ShaderRef) -> bool {
        matches!(shader, ShaderRef::Default)
    }

    #[test]
    fn prepass_vertex_follows_the_settings() {
        let both_off = WindPasses {
            wind_in_shadows: false,
            wind_in_prepass: false,
        };
        assert!(is_default(&prepass_vertex_ref(both_off, false)));
        assert!(!is_default(&prepass_vertex_ref(
            WindPasses::default(),
            false
        )));
        let prepass_only = WindPasses {
            wind_in_shadows: false,
            wind_in_prepass: true,
        };
        assert!(!is_default(&prepass_vertex_ref(prepass_only, false)));
        // The dev switch wins over everything.
        assert!(is_default(&prepass_vertex_ref(WindPasses::default(), true)));
    }

    #[test]
    fn shipped_data_keeps_the_camera_prepass_off() {
        assert!(!<WindExtension as MaterialExtension>::enable_prepass());
    }

    /// `wind_common` compiles (WESL -> WGSL) and validates (naga) with a consumer.
    #[test]
    fn wind_common_compiles_and_validates() {
        use wesl::syntax::{ModulePath, PathOrigin};
        let abs = |name: &str| ModulePath::new(PathOrigin::Absolute, vec![name.to_string()]);
        let mut resolver = wesl::resolver::VirtualResolver::new();
        resolver.add_module(abs("wind"), include_str!("wind_common.wesl").into());
        resolver.add_module(
            abs("consumer"),
            r#"
import super::wind::{WindParams, wind_displace};
@group(0) @binding(0) var<uniform> wind: WindParams;
@group(0) @binding(1) var flatten_tex: texture_2d<f32>;
@group(0) @binding(2) var flatten_samp: sampler;
@vertex
fn vs(@location(0) p: vec3<f32>, @location(1) uv: vec2<f32>) -> @builtin(position) vec4<f32> {
    let r = wind_displace(wind, vec4<f32>(p, 1.0), p.y, uv.y, 1.5, flatten_tex, flatten_samp);
    return r.world * r.tint;
}
"#
            .into(),
        );
        let mut compiler = wesl::Compiler::default().with_resolver(resolver);
        compiler.options.keep_main = true;
        let wgsl = compiler.compile("consumer").expect("wesl").to_string();
        let module = naga::front::wgsl::parse_str(&wgsl).unwrap_or_else(|e| panic!("{e}\n{wgsl}"));
        naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::all(),
        )
        .validate(&module)
        .unwrap_or_else(|e| panic!("{e:?}\n{wgsl}"));
    }
}
