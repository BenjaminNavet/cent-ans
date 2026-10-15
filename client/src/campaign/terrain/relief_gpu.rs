//! GPU side of the exaggerated relief, shared by every campaign material (`relief.wesl`).
//!
//! [`ReliefGpu`] owns the valley-floor texture (Rgba32Float: floor, base, k, massif lift D) and the
//! [`ReliefParams`] uniform, both refreshed from [`ReliefState`]. A material that places geometry
//! by altitude (water, roads, vegetation, markers...) binds them in its own bind group:
//!
//! ```ignore
//! #[uniform(5)] pub relief: ReliefParams,
//! #[texture(6, sample_type = "float", filterable = false)] pub relief_floor: Handle<Image>,
//! ```
//!
//! then copies `ReliefGpu::params` / `ReliefGpu::floor` into itself (system run when
//! `Res<ReliefGpu>` `is_changed()`), and calls `relief_display_height(...)` from
//! `import cent_ans_client::campaign::terrain::relief::...` (see the header of `relief.wesl`).

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{
    Extent3d, ShaderType, TextureDimension, TextureFormat, TextureUsages,
};

use super::relief_floor::FloorGrid;
use super::relief_state::ReliefState;

/// Uniform block `ReliefParams` of `relief.wesl`.
#[derive(Clone, Copy, Debug, Default, PartialEq, ShaderType)]
pub struct ReliefParams {
    /// Vertical scale s, local gain g, squash weight c, 1 when the floor texture is valid.
    pub scale: Vec4,
    /// Cell size (map px), centre offset of cell 0 (map px), unused, unused.
    pub info: Vec4,
}

/// Floor texture + uniform, kept in sync with [`ReliefState`].
#[derive(Resource, Clone, Debug)]
pub struct ReliefGpu {
    pub floor: Handle<Image>,
    pub params: ReliefParams,
    grid_ptr: usize,
}

impl ReliefGpu {
    pub fn params_of(state: &ReliefState) -> ReliefParams {
        let valid = if state.has_floor() { 1.0 } else { 0.0 };
        ReliefParams {
            scale: Vec4::new(state.vertical_scale, state.gain, state.squash, valid),
            info: Vec4::new(state.grid.cell, 0.5 * (state.grid.cell - 1.0), 0.0, 0.0),
        }
    }
}

/// Rgba32Float image of a floor grid (1 x 1 dummy when empty).
pub fn floor_image(grid: &FloorGrid) -> Image {
    let (w, h) = if grid.floor.is_empty() {
        (1, 1)
    } else {
        grid.side
    };
    let mut data = Vec::with_capacity(w * h * 16);
    for i in 0..w * h {
        for channel in [&grid.floor, &grid.base, &grid.squash, &grid.lift] {
            let v = channel.get(i).copied().unwrap_or(0.0);
            data.extend_from_slice(&v.to_le_bytes());
        }
    }
    let mut image = Image::new(
        Extent3d {
            width: w as u32,
            height: h as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba32Float,
        RenderAssetUsages::RENDER_WORLD,
    );
    image.texture_descriptor.usage = TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST;
    image
}

/// Creates / refreshes [`ReliefGpu`] from [`ReliefState`] (texture only when the grid changed).
pub fn sync_relief_gpu(
    mut commands: Commands,
    state: Option<Res<ReliefState>>,
    gpu: Option<ResMut<ReliefGpu>>,
    mut images: ResMut<Assets<Image>>,
) {
    let Some(state) = state else {
        return;
    };
    if gpu.is_some() && !state.is_changed() {
        return;
    }
    let params = ReliefGpu::params_of(&state);
    let grid_ptr = state.grid.floor.as_ptr() as usize;
    match gpu {
        Some(mut gpu) => {
            if gpu.grid_ptr != grid_ptr {
                gpu.floor = images.add(floor_image(&state.grid));
                gpu.grid_ptr = grid_ptr;
            }
            if gpu.params != params {
                gpu.params = params;
            }
        }
        None => commands.insert_resource(ReliefGpu {
            floor: images.add(floor_image(&state.grid)),
            params,
            grid_ptr,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn floor_image_interleaves_the_four_channels() {
        let grid = FloorGrid {
            side: (2, 1),
            cell: 8.0,
            floor: Arc::new(vec![1.0, 5.0]),
            base: Arc::new(vec![2.0, 6.0]),
            squash: Arc::new(vec![3.0, 7.0]),
            lift: Arc::new(vec![4.0, 8.0]),
        };
        let image = floor_image(&grid);
        let data = image.data.as_ref().unwrap();
        let floats: Vec<f32> = data
            .chunks(4)
            .map(|c| f32::from_le_bytes(c.try_into().unwrap()))
            .collect();
        assert_eq!(floats, vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0]);
        assert_eq!(image.texture_descriptor.size.width, 2);
    }

    #[test]
    fn empty_grid_gives_a_dummy_pixel() {
        let image = floor_image(&FloorGrid::default());
        assert_eq!(image.data.as_ref().unwrap().len(), 16);
    }
}

#[cfg(test)]
mod wesl_tests {
    /// `relief.wesl` compiles (WESL -> WGSL) and validates (naga) with a consumer that binds the
    /// shared resources the documented way.
    #[test]
    fn relief_module_compiles_and_validates() {
        use wesl::syntax::{ModulePath, PathOrigin};
        let abs = |name: &str| ModulePath::new(PathOrigin::Absolute, vec![name.to_string()]);
        let mut resolver = wesl::resolver::VirtualResolver::new();
        resolver.add_module(abs("relief"), include_str!("relief.wesl").into());
        resolver.add_module(
            abs("consumer"),
            r#"
import super::relief::{ReliefParams, relief_display_height, relief_display_gradient};
@group(0) @binding(0) var<uniform> relief: ReliefParams;
@group(0) @binding(1) var relief_floor: texture_2d<f32>;
@fragment
fn fs(@location(0) p: vec3<f32>) -> @location(0) vec4<f32> {
    let y = relief_display_height(relief_floor, relief, p.y, p.xz);
    let g = relief_display_gradient(relief_floor, relief, p.y, p.xz, p.xz);
    return vec4<f32>(y, g, 1.0);
}
"#
            .into(),
        );
        let mut compiler = wesl::Compiler::default().with_resolver(resolver);
        compiler.options.keep_main = true;
        let wgsl = compiler
            .compile("consumer")
            .expect("wesl compile")
            .to_string();
        let module = naga::front::wgsl::parse_str(&wgsl).unwrap_or_else(|e| panic!("{e}\n{wgsl}"));
        naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::all(),
        )
        .validate(&module)
        .unwrap_or_else(|e| panic!("{e:?}\n{wgsl}"));
    }
}
