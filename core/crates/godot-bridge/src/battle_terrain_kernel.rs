//! Lot SC PF-08 / BT7 / GB6: Godot handle on the battle ground computed in
//! Rust (`battle_terrain_field.rs`). `battle_terrain.gd` hands over the
//! simulation grid, the river, the coast, the horizon relief and the noise
//! resources once at build time, then asks for heights, grids and meshes.
//! Rendering only: no game rule here.

use crate::battle_terrain_field::{
    extend_river, relief_from_heights, Biome, Coast, Horizon, MeshData, Noise, Terrain,
};
use godot::classes::mesh::{ArrayType, PrimitiveType};
use godot::classes::{ArrayMesh, FastNoiseLite, RefCounted};
use godot::prelude::*;

/// The three `FastNoiseLite` resources of the decorative hills.
struct GodotNoise {
    hills: Option<Gd<FastNoiseLite>>,
    ridges: Option<Gd<FastNoiseLite>>,
    rolls: Option<Gd<FastNoiseLite>>,
}

fn sample(noise: &Option<Gd<FastNoiseLite>>, x: f32, z: f32) -> f32 {
    noise.as_ref().map_or(0.0, |n| n.get_noise_2d(x, z))
}

impl Noise for GodotNoise {
    fn hills(&self, x: f32, z: f32) -> f32 {
        sample(&self.hills, x, z)
    }
    fn ridges(&self, x: f32, z: f32) -> f32 {
        sample(&self.ridges, x, z)
    }
    fn rolls(&self, x: f32, z: f32) -> f32 {
        sample(&self.rolls, x, z)
    }
}

#[derive(GodotClass)]
#[class(base = RefCounted)]
pub struct BattleTerrainKernel {
    terrain: Terrain,
    noise: GodotNoise,
    base: Base<RefCounted>,
}

#[godot_api]
impl IRefCounted for BattleTerrainKernel {
    fn init(base: Base<RefCounted>) -> Self {
        BattleTerrainKernel {
            terrain: Terrain::default(),
            noise: GodotNoise {
                hills: None,
                ridges: None,
                rolls: None,
            },
            base,
        }
    }
}

fn float_of(dict: &VarDictionary, key: &str, default: f64) -> f64 {
    // GDScript hands over `int` or `float` indifferently.
    dict.get(key).map_or(default, |v| {
        v.try_to::<f64>()
            .or_else(|_| v.try_to::<i64>().map(|i| i as f64))
            .unwrap_or(default)
    })
}

fn mesh_of(data: &MeshData) -> Gd<ArrayMesh> {
    let vertices: PackedVector3Array = data
        .vertices
        .iter()
        .map(|v| Vector3::new(v[0], v[1], v[2]))
        .collect();
    let normals: PackedVector3Array = data
        .normals
        .iter()
        .map(|v| Vector3::new(v[0], v[1], v[2]))
        .collect();
    let indices = PackedInt32Array::from(data.indices.as_slice());
    let mut arrays = VarArray::new();
    arrays.resize(ArrayType::MAX.ord() as usize, &Variant::nil());
    arrays.set(ArrayType::VERTEX.ord() as usize, &vertices.to_variant());
    arrays.set(ArrayType::NORMAL.ord() as usize, &normals.to_variant());
    arrays.set(ArrayType::INDEX.ord() as usize, &indices.to_variant());
    let mut mesh = ArrayMesh::new_gd();
    mesh.add_surface_from_arrays(PrimitiveType::TRIANGLES, &arrays);
    mesh
}

#[godot_api]
impl BattleTerrainKernel {
    /// The simulation grid (`nx × nz`, `resolution` m) and the field size.
    #[func]
    fn set_field(
        &mut self,
        heights: PackedFloat32Array,
        nx: i64,
        nz: i64,
        resolution: f64,
        width: f64,
        depth: f64,
    ) {
        let t = &mut self.terrain;
        t.heights = heights.to_vec();
        t.nx = nx.max(0) as usize;
        t.nz = nz.max(0) as usize;
        t.resolution = resolution;
        t.width = width;
        t.depth = depth;
        if t.heights.len() < t.nx * t.nz {
            t.nx = 0;
            t.nz = 0;
        }
        // GDScript sums in double then divides by the packed array size.
        let sum: f64 = t.heights.iter().map(|h| f64::from(*h)).sum();
        t.mean_height = if t.heights.is_empty() {
            0.0
        } else {
            sum / t.heights.len() as f64
        };
    }

    /// Mean height of the grid (centre of the decorative hills).
    #[func]
    fn mean_height(&self) -> f64 {
        self.terrain.mean_height
    }

    /// Noise resources of the hills beyond the field.
    #[func]
    fn set_noise(
        &mut self,
        hills: Option<Gd<FastNoiseLite>>,
        ridges: Option<Gd<FastNoiseLite>>,
        rolls: Option<Gd<FastNoiseLite>>,
    ) {
        self.noise = GodotNoise {
            hills,
            ridges,
            rolls,
        };
    }

    /// Hills scale, ridge share and roll amplitude of the biome.
    #[func]
    fn set_biome(&mut self, relief: f64, ridges: f64, rolls: f64) {
        self.terrain.biome = Biome {
            relief,
            ridges,
            rolls,
        };
    }

    /// River course extended beyond the field (a point every 10 m), widths per point.
    #[func]
    fn set_river(
        &mut self,
        points: PackedVector2Array,
        widths: PackedFloat32Array,
        default_width: f64,
    ) {
        let t = &mut self.terrain;
        t.river_points = points.as_slice().iter().map(|p| (p.x, p.y)).collect();
        t.river_widths = widths.to_vec();
        t.river_default_width = default_width;
    }

    /// The simulation river extended `reach` m each side by mirror images:
    /// `{points: PackedVector2Array, widths: PackedFloat32Array}`.
    #[func]
    fn extend_river(
        &self,
        points: PackedVector2Array,
        widths: PackedFloat32Array,
        default_width: f64,
        reach: f64,
    ) -> VarDictionary {
        let pts: Vec<(f32, f32)> = points.as_slice().iter().map(|p| (p.x, p.y)).collect();
        let (out_points, out_widths) = extend_river(&pts, widths.as_slice(), default_width, reach);
        let mut result = VarDictionary::new();
        let packed: PackedVector2Array =
            out_points.iter().map(|p| Vector2::new(p.0, p.1)).collect();
        result.set("points", &packed.to_variant());
        result.set(
            "widths",
            &PackedFloat32Array::from(out_widths.as_slice()).to_variant(),
        );
        result
    }

    /// Sea on the west or east flank (`shore_x`), with the sea level.
    #[func]
    fn set_coast(&mut self, west: bool, shore_x: f64, sea_level: f64) {
        self.terrain.coast = Some(Coast {
            west,
            shore_x,
            sea_level,
        });
    }

    #[func]
    fn clear_coast(&mut self) {
        self.terrain.coast = None;
    }

    /// Pools dug into the ground: `[{x, z, radius}]`.
    #[func]
    fn set_pools(&mut self, pools: VarArray) {
        self.terrain.pools = pools
            .iter_shared()
            .map(|p| {
                let d = p.to::<VarDictionary>();
                (
                    float_of(&d, "x", 0.0) as f32,
                    float_of(&d, "z", 0.0) as f32,
                    float_of(&d, "radius", 0.0),
                )
            })
            .collect();
    }

    /// Real relief of the place (EP2): see `BattleHorizon.kernel_params`.
    #[func]
    fn set_horizon(&mut self, params: VarDictionary) {
        let heights = params
            .get("heights")
            .map(|v| v.to::<PackedFloat32Array>().to_vec())
            .unwrap_or_default();
        let n = float_of(&params, "n", 0.0) as usize;
        if n < 2 || heights.len() < n * n {
            self.terrain.horizon = None;
            return;
        }
        let pair = |key: &str| {
            params
                .get(key)
                .map_or((0.0, 0.0), |v| (v.to::<Vector2>().x, v.to::<Vector2>().y))
        };
        self.terrain.horizon = Some(Horizon {
            heights,
            n,
            step: float_of(&params, "step", 100.0),
            half: float_of(&params, "half", 13000.0),
            centre: pair("centre"),
            field_size: pair("field_size"),
            cos: float_of(&params, "cos", 1.0),
            sin: float_of(&params, "sin", 0.0),
            offset_y: float_of(&params, "offset_y", 0.0),
            blend_start: float_of(&params, "blend_start", 700.0),
            blend_end: float_of(&params, "blend_end", 3000.0),
            river_keep: float_of(&params, "river_keep", 450.0),
        });
    }

    #[func]
    fn clear_horizon(&mut self) {
        self.terrain.horizon = None;
    }

    /// Bilinear height of the grid, clamped to the field.
    #[func]
    fn height_at(&self, x: f64, z: f64) -> f64 {
        self.terrain.height_at(x, z)
    }

    /// Decorative height everywhere (field, hills, river valley, horizon, coast).
    #[func]
    fn world_height(&self, x: f64, z: f64) -> f64 {
        self.terrain.world_height(&self.noise, x, z)
    }

    /// `world_height` over an `nx × nz` grid of `step` m from `origin`.
    #[func]
    fn height_grid(&self, origin: Vector2, step: f64, nx: i64, nz: i64) -> PackedFloat32Array {
        let grid = self.terrain.height_grid(
            &self.noise,
            (f64::from(origin.x), f64::from(origin.y)),
            step,
            nx.max(0) as usize,
            nz.max(0) as usize,
        );
        PackedFloat32Array::from(grid.as_slice())
    }

    /// Distance to the river bed (infinite without river).
    #[func]
    fn river_distance(&self, x: f64, z: f64) -> f64 {
        self.terrain.river_distance(x, z)
    }

    #[func]
    fn river_width_at(&self, x: f64) -> f64 {
        self.terrain.river_width_at(x)
    }

    #[func]
    fn river_span_at(&self, x: f64) -> f64 {
        self.terrain.river_span_at(x)
    }

    /// The simulation grid as a mesh with a 3 m skirt (pools dug in).
    #[func]
    fn field_mesh(&self) -> Gd<ArrayMesh> {
        mesh_of(&self.terrain.field_mesh())
    }

    /// Terrain ring of grid `step` around `hole`, sunk by `sink` inside the hole.
    #[func]
    fn ring_mesh(&self, rect: Rect2, step: f64, hole: Rect2, sink: f64) -> Gd<ArrayMesh> {
        let r = |r: Rect2| (r.position.x, r.position.y, r.size.x, r.size.y);
        mesh_of(
            &self
                .terrain
                .ring_mesh(&self.noise, r(rect), step, r(hole), sink),
        )
    }

    /// Relief map (RGBA8 bytes) of a `hw × hh` height grid of `texel` m.
    #[func]
    fn relief_from_heights(
        &self,
        hdata: PackedFloat32Array,
        hw: i64,
        hh: i64,
        texel: f64,
    ) -> PackedByteArray {
        let (hw, hh) = (hw.max(0) as usize, hh.max(0) as usize);
        if hdata.len() < hw * hh {
            return PackedByteArray::new();
        }
        PackedByteArray::from(relief_from_heights(hdata.as_slice(), hw, hh, texel).as_slice())
    }
}
