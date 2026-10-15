//! WGSL (WESL) materials of the water lot, ports of the Godot spatial shaders:
//! `water.gdshader` -> [`SeaMaterial`], `river_water.gdshader` -> [`RiverMaterial`] (rivers and
//! lake sheets), `terrain_line.gdshader` + `road_line.gdshader` -> [`LineMaterial`],
//! `road.gdshader` -> [`RoadRibbonMaterial`]. Shaders are embedded and registered under fixed
//! handles by [`register_shaders`].

use bevy::asset::uuid_handle;
use bevy::prelude::*;
use bevy::reflect::TypePath;
use bevy::render::render_resource::{AsBindGroup, ShaderType};
use bevy::shader::{Shader, ShaderRef};

use super::drape::Drape;
use crate::campaign::terrain::relief_gpu::ReliefParams;

const LINE_SHADER: Handle<Shader> = uuid_handle!("6f0b1a52-3c1e-4d0a-9d31-0a5f7c1e0001");
const ROAD_SHADER: Handle<Shader> = uuid_handle!("6f0b1a52-3c1e-4d0a-9d31-0a5f7c1e0002");
const RIVER_SHADER: Handle<Shader> = uuid_handle!("6f0b1a52-3c1e-4d0a-9d31-0a5f7c1e0003");
const SEA_SHADER: Handle<Shader> = uuid_handle!("6f0b1a52-3c1e-4d0a-9d31-0a5f7c1e0004");

/// Registers the embedded shaders (no-op without the render plugin, e.g. in tests).
pub fn register_shaders(app: &mut App) {
    let Some(mut shaders) = app.world_mut().get_resource_mut::<Assets<Shader>>() else {
        return;
    };
    let sources: [(Handle<Shader>, &'static str, &'static str); 4] = [
        (
            LINE_SHADER,
            include_str!("shaders/line.wesl"),
            "water/line.wesl",
        ),
        (
            ROAD_SHADER,
            include_str!("shaders/road.wesl"),
            "water/road.wesl",
        ),
        (
            RIVER_SHADER,
            include_str!("shaders/river.wesl"),
            "water/river.wesl",
        ),
        (
            SEA_SHADER,
            include_str!("shaders/sea.wesl"),
            "water/sea.wesl",
        ),
    ];
    for (handle, source, path) in sources {
        let source = super::drape::with_relief(source);
        if let Err(error) = shaders.insert(handle.id(), Shader::from_wesl(source, path)) {
            error!("water: shader {path}: {error:?}");
        }
    }
}

/// sRGB triple (Godot `source_color`) to a linear `Vec4`.
pub fn linear(rgb: [f32; 3], alpha: f32) -> Vec4 {
    let c = Color::srgba(rgb[0], rgb[1], rgb[2], alpha).to_linear();
    Vec4::new(c.red, c.green, c.blue, c.alpha)
}

// --- Lines (coast, roads) ----------------------------------------------------------------

#[derive(ShaderType, Debug, Clone, Copy)]
pub struct LineParams {
    pub fill_color: Vec4,
    pub casing_color: Vec4,
    pub alpha: f32,
    pub near_px: f32,
    pub far_px: f32,
    pub near_distance: f32,
    pub far_distance: f32,
    pub far_alpha: f32,
    pub casing_px: f32,
    pub thin_fade: f32,
    /// World units above the draped ground.
    pub lift: f32,
}

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct LineMaterial {
    #[uniform(0)]
    pub params: LineParams,
    #[uniform(1)]
    pub relief: ReliefParams,
    #[texture(2, sample_type = "float", filterable = false)]
    pub relief_floor: Handle<Image>,
}

impl Drape for LineMaterial {
    fn set_relief(&mut self, params: ReliefParams, floor: &Handle<Image>) {
        self.relief = params;
        self.relief_floor = floor.clone();
    }
    fn relief(&self) -> (ReliefParams, &Handle<Image>) {
        (self.relief, &self.relief_floor)
    }
}

impl LineMaterial {
    /// `CoastRenderer`: dark discreet hairline of constant screen width (`terrain_line`).
    pub fn coast() -> Self {
        Self {
            params: LineParams {
                fill_color: linear([0.10, 0.09, 0.07], 0.28),
                casing_color: linear([0.10, 0.09, 0.07], 0.28),
                alpha: 1.0,
                near_px: 1.0,
                far_px: 1.0,
                near_distance: 150.0,
                far_distance: 620.0,
                far_alpha: 1.0,
                casing_px: 0.0,
                thin_fade: 0.7,
                lift: 0.05,
            },
            relief: ReliefParams::default(),
            relief_floor: Handle::default(),
        }
    }

    /// Main roads: light packed earth with a discreet casing (`RoadRenderer.main_*`).
    pub fn main_road() -> Self {
        Self {
            params: LineParams {
                fill_color: linear([0.66, 0.56, 0.41], 1.0),
                casing_color: linear([0.36, 0.28, 0.18], 0.45),
                alpha: 1.0,
                near_px: 3.4,
                far_px: 1.6,
                near_distance: 150.0,
                far_distance: 620.0,
                far_alpha: 0.25,
                casing_px: 0.8,
                thin_fade: 0.0,
                lift: 0.1,
            },
            relief: ReliefParams::default(),
            relief_floor: Handle::default(),
        }
    }

    /// Secondary and computed roads: thin, pale, fading before the high tier.
    pub fn minor_road() -> Self {
        Self {
            params: LineParams {
                fill_color: linear([0.36, 0.25, 0.13], 0.55),
                casing_color: linear([0.36, 0.25, 0.13], 0.55),
                alpha: 1.0,
                near_px: 1.5,
                far_px: 0.9,
                near_distance: 150.0,
                far_distance: 420.0,
                far_alpha: 0.0,
                casing_px: 0.0,
                thin_fade: 0.0,
                lift: 0.1,
            },
            relief: ReliefParams::default(),
            relief_floor: Handle::default(),
        }
    }
}

impl Material for LineMaterial {
    fn vertex_shader() -> ShaderRef {
        LINE_SHADER.into()
    }
    fn fragment_shader() -> ShaderRef {
        LINE_SHADER.into()
    }
    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Blend
    }
    fn enable_prepass() -> bool {
        false
    }
    fn enable_shadows() -> bool {
        false
    }
    fn depth_bias(&self) -> f32 {
        // Godot `render_priority = 1`: drawn over the ground and the lines under it.
        1.0
    }

    fn specialize(
        _pipeline: &bevy::pbr::MaterialPipeline,
        descriptor: &mut bevy::render::render_resource::RenderPipelineDescriptor,
        _layout: &bevy::mesh::MeshVertexBufferLayoutRef,
        _key: bevy::pbr::MaterialPipelineKey<Self>,
    ) -> Result<(), bevy::render::render_resource::SpecializedMeshPipelineError> {
        // Ribbons have no fixed winding after draping: two-sided.
        descriptor.primitive.cull_mode = None;
        Ok(())
    }
}

// --- Dirt road ribbons -------------------------------------------------------------------

#[derive(ShaderType, Debug, Clone, Copy)]
pub struct RoadParams {
    pub dirt_color: Vec4,
    pub rut_color: Vec4,
    pub grass_edge: Vec4,
    pub alpha: f32,
    pub lift: f32,
}

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct RoadRibbonMaterial {
    #[uniform(0)]
    pub params: RoadParams,
    #[uniform(1)]
    pub relief: ReliefParams,
    #[texture(2, sample_type = "float", filterable = false)]
    pub relief_floor: Handle<Image>,
}

impl Drape for RoadRibbonMaterial {
    fn set_relief(&mut self, params: ReliefParams, floor: &Handle<Image>) {
        self.relief = params;
        self.relief_floor = floor.clone();
    }
    fn relief(&self) -> (ReliefParams, &Handle<Image>) {
        (self.relief, &self.relief_floor)
    }
}

impl Default for RoadRibbonMaterial {
    fn default() -> Self {
        Self {
            params: RoadParams {
                dirt_color: linear([0.47, 0.38, 0.25], 1.0),
                rut_color: linear([0.34, 0.26, 0.16], 1.0),
                grass_edge: linear([0.30, 0.31, 0.16], 1.0),
                alpha: 1.0,
                lift: 0.06,
            },
            relief: ReliefParams::default(),
            relief_floor: Handle::default(),
        }
    }
}

impl Material for RoadRibbonMaterial {
    fn vertex_shader() -> ShaderRef {
        ROAD_SHADER.into()
    }
    fn fragment_shader() -> ShaderRef {
        ROAD_SHADER.into()
    }
    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Blend
    }
    fn enable_prepass() -> bool {
        false
    }
    fn enable_shadows() -> bool {
        false
    }

    fn specialize(
        _pipeline: &bevy::pbr::MaterialPipeline,
        descriptor: &mut bevy::render::render_resource::RenderPipelineDescriptor,
        _layout: &bevy::mesh::MeshVertexBufferLayoutRef,
        _key: bevy::pbr::MaterialPipelineKey<Self>,
    ) -> Result<(), bevy::render::render_resource::SpecializedMeshPipelineError> {
        // Ribbons have no fixed winding after draping: two-sided.
        descriptor.primitive.cull_mode = None;
        Ok(())
    }
}

// --- Rivers and lakes --------------------------------------------------------------------

#[derive(ShaderType, Debug, Clone, Copy)]
pub struct RiverParams {
    pub shallow_color: Vec4,
    pub deep_color: Vec4,
    pub sky_color: Vec4,
    pub foam_color: Vec4,
    pub bank_ink_color: Vec4,
    pub importance_px_lo: Vec4,
    pub importance_px_hi: Vec4,
    /// Detail texture: x = strength (0 = off), y = tiles per unit, z = flow speed, w = turbidity.
    pub detail: Vec4,
    /// Linear mean colour of the detail albedo (w unused).
    pub detail_mean: Vec4,
    pub silt_color: Vec4,
    pub map_size: Vec2,
    pub distance_fade: Vec2,
    pub min_px: f32,
    pub thin_fade: f32,
    pub far_alpha: f32,
    pub bank_ink_strength: f32,
    pub bank_ink_px: f32,
    pub edge_soft_px: f32,
    pub sky_reflect: f32,
    pub lift_base: f32,
    pub lift_per_distance: f32,
    pub flow_speed: f32,
    pub ripple_scale: f32,
    pub ripple_strength: f32,
    pub sheet: f32,
    pub sheet_alpha: f32,
    pub gen_width_scale: f32,
    pub gen_width_ref: f32,
    pub has_coast_dist: f32,
    pub coast_dist_scale: f32,
}

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct RiverMaterial {
    #[uniform(0)]
    pub params: RiverParams,
    #[texture(1)]
    #[sampler(2)]
    pub coast_dist: Option<Handle<Image>>,
    #[uniform(3)]
    pub relief: ReliefParams,
    #[texture(4, sample_type = "float", filterable = false)]
    pub relief_floor: Handle<Image>,
    /// Detail normal (`water_detail_normal`, repeating, mip-mapped) and its sampler.
    #[texture(5)]
    #[sampler(6)]
    pub detail_normal: Option<Handle<Image>>,
    /// Detail albedo; alpha is the foam streak mask.
    #[texture(7)]
    pub detail_albedo: Option<Handle<Image>>,
}

impl RiverParams {
    /// Binds a detail layer (`WaterDetail.apply` + `flow_speed` + `turbidity`).
    pub fn set_detail(&mut self, layer: &super::detail::Layer) {
        self.detail = Vec4::new(
            layer.strength,
            layer.scale,
            layer.flow_speed,
            layer.turbidity,
        );
        self.detail_mean = layer.mean.extend(0.0);
    }
}

impl Drape for RiverMaterial {
    fn set_relief(&mut self, params: ReliefParams, floor: &Handle<Image>) {
        self.relief = params;
        self.relief_floor = floor.clone();
    }
    fn relief(&self) -> (ReliefParams, &Handle<Image>) {
        (self.relief, &self.relief_floor)
    }
}

/// Camera distance (world units) at which the campaign legibility of the river ribbons is checked.
pub const LEGIBILITY_DISTANCE: f32 = 260.0;

impl RiverParams {
    /// What the vertex shader guarantees for a river at eye distance `distance`: the on-screen
    /// width floor in pixels per importance tier (`min_px x importance scale`) and the opacity of
    /// the far fade (`river.wesl`: ribbons are never thinner than the floor, only faded).
    pub fn legibility(&self, distance: f32) -> ([f32; 7], f32) {
        let scale = |i: usize| {
            if i < 4 {
                self.importance_px_lo[i]
            } else {
                self.importance_px_hi[i - 4]
            }
        };
        let floors = std::array::from_fn(|i| self.min_px * scale(i));
        let alpha = if self.distance_fade.y > 0.0 {
            let t = ((distance - self.distance_fade.x)
                / (self.distance_fade.y - self.distance_fade.x))
                .clamp(0.0, 1.0);
            let smooth = t * t * (3.0 - 2.0 * t);
            1.0 + (self.far_alpha - 1.0) * smooth
        } else {
            1.0
        };
        (floors, alpha)
    }

    /// River ribbons from `river_display.json` (`RiversRenderer._apply_display`).
    pub fn river(
        display: &super::data::RiverDisplay,
        min_px: f32,
        map_size: Vec2,
        extent: f32,
        minor: bool,
        meters_per_px: f32,
    ) -> Self {
        let scale = &display.importance_min_px_scale;
        let at = |i: usize| scale.get(i).copied().unwrap_or(1.0);
        Self {
            shallow_color: linear(display.shallow_color, 1.0),
            deep_color: linear(display.deep_color, 1.0),
            sky_color: linear(display.sky_color, 1.0),
            foam_color: linear([0.80, 0.82, 0.78], 1.0),
            bank_ink_color: linear(display.bank_ink_color, 1.0),
            importance_px_lo: Vec4::new(at(0), at(1), at(2), at(3)),
            importance_px_hi: Vec4::new(at(4), at(5), at(6), 1.0),
            detail: Vec4::ZERO,
            detail_mean: Vec4::ONE,
            silt_color: linear([0.34, 0.31, 0.22], 1.0),
            map_size,
            distance_fade: if minor {
                Vec2::new(display.minor_fade[0], display.minor_fade[1]) * extent
            } else {
                Vec2::ZERO
            },
            min_px,
            thin_fade: display.thin_fade,
            far_alpha: if minor { display.minor_far_alpha } else { 1.0 },
            bank_ink_strength: display.bank_ink_strength,
            bank_ink_px: display.bank_ink_px,
            edge_soft_px: display.edge_soft_px,
            sky_reflect: display.sky_reflect,
            lift_base: 0.012,
            lift_per_distance: 0.0009,
            flow_speed: 0.35,
            ripple_scale: 3.2,
            ripple_strength: 0.08,
            sheet: 0.0,
            sheet_alpha: 0.88,
            gen_width_scale: display.generalised.width_scale.max(0.0),
            gen_width_ref: if display.generalised.width_ref_m > 0.0 {
                display.generalised.width_ref_m / meters_per_px.max(1.0)
            } else {
                0.14
            },
            has_coast_dist: 0.0,
            coast_dist_scale: 2.0,
        }
    }

    /// Lake sheets (`LakesRenderer._water_material`, colours in sRGB there).
    pub fn lake(map_size: Vec2) -> Self {
        let mut p = Self::river(
            &super::data::RiverDisplay::default(),
            1.0,
            map_size,
            1.0,
            false,
            719.0,
        );
        p.gen_width_scale = 0.0;
        p.shallow_color = linear([0.26, 0.46, 0.52], 1.0);
        p.deep_color = linear([0.13, 0.31, 0.44], 1.0);
        p.sky_reflect = 0.55;
        p.sheet = 1.0;
        p.sheet_alpha = 0.88;
        p.sky_color = linear([0.60, 0.72, 0.82], 1.0);
        p
    }
}

impl Material for RiverMaterial {
    fn vertex_shader() -> ShaderRef {
        RIVER_SHADER.into()
    }
    fn fragment_shader() -> ShaderRef {
        RIVER_SHADER.into()
    }
    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Blend
    }
    fn enable_prepass() -> bool {
        false
    }
    fn enable_shadows() -> bool {
        false
    }

    fn specialize(
        _pipeline: &bevy::pbr::MaterialPipeline,
        descriptor: &mut bevy::render::render_resource::RenderPipelineDescriptor,
        _layout: &bevy::mesh::MeshVertexBufferLayoutRef,
        _key: bevy::pbr::MaterialPipelineKey<Self>,
    ) -> Result<(), bevy::render::render_resource::SpecializedMeshPipelineError> {
        // Ribbons have no fixed winding after draping: two-sided.
        descriptor.primitive.cull_mode = None;
        Ok(())
    }
}

// --- Sea ---------------------------------------------------------------------------------

#[derive(ShaderType, Debug, Clone, Copy)]
pub struct SeaParams {
    pub shallow_color: Vec4,
    pub mid_color: Vec4,
    pub deep_color: Vec4,
    pub foam_color: Vec4,
    pub edge_haze_color: Vec4,
    pub season_tint: Vec4,
    pub basin_tint: [Vec4; 5],
    pub basin_wave: [Vec4; 5],
    pub basin_water: [Vec4; 5],
    pub map_size: Vec2,
    pub wave_scale: f32,
    pub wave_speed: f32,
    pub wave_strength: f32,
    pub foam_px: f32,
    pub coast_dist_scale: f32,
    pub depth_per_px_m: f32,
    pub max_depth_m: f32,
    pub depth_falloff_m: f32,
    pub mid_depth_m: f32,
    pub alpha_shallow: f32,
    pub alpha_deep: f32,
    pub edge_haze_px: f32,
    pub lake_pond_px: f32,
    pub basin_on: f32,
    pub sun_spec_power: f32,
    pub sun_spec_intensity: f32,
    /// Living sea (`sea_life.rs`): x = on, y = brightness, z = max saturation, w = surf min px.
    pub life_a: Vec4,
    /// Shallows colour, depth (m) in w.
    pub shallows_a: Vec4,
    /// Shallows amount, caustic, caustic scale.
    pub shallows_b: Vec4,
    /// Crest colour, amount in w.
    pub crest_a: Vec4,
    /// Crest depth range (m), frequency, speed.
    pub crest_b: Vec4,
    pub crest_dir: Vec4,
    /// Tide period (s), max px, high cover, mud alpha.
    pub tide_a: Vec4,
    pub tide_mud: Vec4,
    pub tide_wet: Vec4,
    /// Coastal sea detail: x = strength (0 = off), y = tiles per unit.
    pub detail_sea: Vec4,
    /// Linear mean colour of the coastal detail albedo.
    pub detail_sea_mean: Vec4,
    /// Open-ocean detail: strength, tiles per unit.
    pub detail_ocean: Vec4,
    pub detail_ocean_mean: Vec4,
    /// Depths (m) of the coastal -> ocean transition in x, y; z = per-basin detail on.
    pub detail_depth: Vec4,
    /// Per basin slot: (array layer or -1, strength, tiles per unit, 0).
    pub basin_detail: [Vec4; 5],
    /// Per basin slot: linear mean colour of the layer albedo.
    pub basin_detail_mean: [Vec4; 5],
}

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct SeaMaterial {
    #[uniform(0)]
    pub params: SeaParams,
    #[texture(1)]
    #[sampler(2)]
    pub coast_dist: Option<Handle<Image>>,
    #[texture(3)]
    #[sampler(4)]
    pub basin_weights: Option<Handle<Image>>,
    /// Tidal flats (`sea_life.rs`), sampled with the basin sampler.
    #[texture(5)]
    pub tide_zones: Option<Handle<Image>>,
    /// Coastal sea detail normal + shared repeating sampler (`water_detail_*`).
    #[texture(6)]
    #[sampler(7)]
    pub detail_sea_normal: Option<Handle<Image>>,
    #[texture(8)]
    pub detail_sea_albedo: Option<Handle<Image>>,
    /// Open-ocean detail (`ocean_detail_*`).
    #[texture(9)]
    pub detail_ocean_normal: Option<Handle<Image>>,
    #[texture(10)]
    pub detail_ocean_albedo: Option<Handle<Image>>,
    /// Per-basin generated surfaces (`sea_basin_normal` / `sea_basin_albedo` arrays).
    #[texture(11, dimension = "2d_array")]
    pub basin_detail_normal: Option<Handle<Image>>,
    #[texture(12, dimension = "2d_array")]
    pub basin_detail_albedo: Option<Handle<Image>>,
}

impl SeaParams {
    /// Binds the coastal and ocean detail layers (strength 0 when a layer is missing).
    pub fn set_detail(
        &mut self,
        sea: Option<&super::detail::Layer>,
        ocean: Option<&super::detail::Layer>,
        basins: Option<&super::detail::BasinArrays>,
    ) {
        if let Some(l) = sea {
            self.detail_sea = Vec4::new(l.strength, l.scale, 0.0, 0.0);
            self.detail_sea_mean = l.mean.extend(0.0);
        }
        if let Some(l) = ocean {
            self.detail_ocean = Vec4::new(l.strength, l.scale, 0.0, 0.0);
            self.detail_ocean_mean = l.mean.extend(0.0);
        }
        if let Some(b) = basins {
            self.detail_depth.z = 1.0;
            self.basin_detail = b.params;
            self.basin_detail_mean = b.means;
        }
    }
}

impl SeaParams {
    /// Defaults of `water.gdshader` (colours are linear there), basins filled by the caller.
    pub fn new(map_size: Vec2, basins: &super::data::SeaBasins) -> Self {
        use super::data::{BasinLook, MAX_BASINS};
        let slot = |look: &BasinLook| {
            (
                Vec4::new(look.tint[0], look.tint[1], look.tint[2], 0.0),
                Vec4::new(look.swell, look.long_swell, look.foam, look.whitecaps),
                Vec4::new(look.clarity, look.grey_scale, 0.0, 0.0),
            )
        };
        let mut tint = [Vec4::ONE; 5];
        let mut wave = [Vec4::new(1.0, 0.0, 1.0, 0.0); 5];
        let mut water = [Vec4::new(1.0, 1.0, 0.0, 0.0); 5];
        (tint[0], wave[0], water[0]) = slot(&basins.default);
        for (i, basin) in basins.basins.iter().take(MAX_BASINS).enumerate() {
            (tint[i + 1], wave[i + 1], water[i + 1]) = slot(&basin.look);
        }
        Self {
            shallow_color: Vec4::new(0.045, 0.14, 0.14, 1.0),
            mid_color: Vec4::new(0.018, 0.075, 0.095, 1.0),
            deep_color: Vec4::new(0.006, 0.028, 0.050, 1.0),
            foam_color: Vec4::new(0.62, 0.66, 0.66, 1.0),
            edge_haze_color: linear([0.74, 0.76, 0.76], 1.0),
            season_tint: Vec4::ONE,
            basin_tint: tint,
            basin_wave: wave,
            basin_water: water,
            map_size,
            wave_scale: 0.6,
            wave_speed: 0.35,
            wave_strength: 0.08,
            foam_px: 1.4,
            coast_dist_scale: 2.0,
            // No heightmap here: depth is estimated from the distance to the coast.
            depth_per_px_m: 12.0,
            max_depth_m: 220.0,
            depth_falloff_m: 70.0,
            mid_depth_m: 25.0,
            alpha_shallow: 0.7,
            alpha_deep: 0.97,
            edge_haze_px: 160.0,
            lake_pond_px: 3.0,
            basin_on: if basins.basins.is_empty() { 0.0 } else { 1.0 },
            sun_spec_power: 180.0,
            sun_spec_intensity: 0.1,
            life_a: Vec4::new(0.0, 1.0, 1.0, 0.0),
            shallows_a: Vec4::new(0.2, 0.4, 0.38, 40.0),
            shallows_b: Vec4::new(0.0, 0.0, 0.9, 0.0),
            crest_a: Vec4::new(0.46, 0.55, 0.58, 0.0),
            crest_b: Vec4::new(40.0, 140.0, 2.4, 0.55),
            crest_dir: Vec4::new(0.8, 0.6, 0.0, 0.0),
            tide_a: Vec4::new(150.0, 8.0, 0.9, 0.96),
            tide_mud: Vec4::new(0.13, 0.115, 0.085, 0.0),
            tide_wet: Vec4::new(0.22, 0.22, 0.19, 0.0),
            detail_sea: Vec4::ZERO,
            detail_sea_mean: Vec4::ONE,
            detail_ocean: Vec4::ZERO,
            detail_ocean_mean: Vec4::ONE,
            detail_depth: Vec4::new(30.0, 120.0, 0.0, 0.0),
            basin_detail: [Vec4::new(-1.0, 0.0, 1.0, 0.0); 5],
            basin_detail_mean: [Vec4::ONE; 5],
        }
    }
}

impl Material for SeaMaterial {
    fn vertex_shader() -> ShaderRef {
        SEA_SHADER.into()
    }
    fn fragment_shader() -> ShaderRef {
        SEA_SHADER.into()
    }
    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Blend
    }
    fn enable_prepass() -> bool {
        false
    }
    fn enable_shadows() -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn river_floor_and_far_fade_at_the_legibility_distance() {
        let display = super::super::data::RiverDisplay::default();
        let major = RiverParams::river(
            &display,
            1.6,
            Vec2::new(7080.0, 6104.0),
            7080.0,
            false,
            719.0,
        );
        let (floors, alpha) = major.legibility(LEGIBILITY_DISTANCE);
        assert!(floors.iter().all(|f| *f >= 1.0), "{floors:?}");
        assert_eq!(alpha, 1.0, "major rivers do not fade");
        let minor = RiverParams::river(
            &display,
            1.0,
            Vec2::new(7080.0, 6104.0),
            7080.0,
            true,
            719.0,
        );
        let (_, near) = minor.legibility(0.0);
        let (_, far) = minor.legibility(1.0e9);
        assert!(near >= far);
    }
}
