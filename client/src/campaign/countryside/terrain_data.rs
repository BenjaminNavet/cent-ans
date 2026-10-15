//! The map as the countryside placement sees it: rasters read once off the main thread
//! (splat, biomes, river bed, wetlands, agrarian landscapes), the heightmap, the land mask and
//! the lakes of the living map, and the spatial indexes of towns, villages and road segments.

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use bevy::prelude::*;
use serde_json::Value;

use super::placement::{RoadSeg, Terrain, Village, cell_of};
use crate::campaign::life::LifeData;
use crate::campaign::terrain::relief_floor::HeightGrid;
use crate::campaign::vegetation::data::{Plane, read_png};

/// Side (px) of the town clearance grid.
const GRID_PX: f32 = 8.0;
/// Reduction of the splat and biome rasters (7080 px -> 1770).
const SHRINK: usize = 4;
/// First row of the regional landscapes in the ground table (`HbGround.FIRST_LANDSCAPE_ROW`).
const FIRST_LANDSCAPE_ROW: i32 = 8;

/// A raster reduced by an integer factor (nearest sample), `ch` channels.
fn shrink(plane: Plane, factor: usize) -> Plane {
    if plane.is_empty() || factor <= 1 {
        return plane;
    }
    let (w, h) = (plane.w / factor, plane.h / factor);
    let mut data = Vec::with_capacity(w * h * plane.ch);
    for y in 0..h {
        for x in 0..w {
            let src = ((y * factor + factor / 2) * plane.w + x * factor + factor / 2) * plane.ch;
            data.extend_from_slice(&plane.data[src..src + plane.ch]);
        }
    }
    Plane {
        w,
        h,
        ch: plane.ch,
        data,
    }
}

pub struct MapTerrain {
    pub size: Vec2,
    pub heights: Arc<HeightGrid>,
    splat: Plane,
    biomes: Plane,
    river: Plane,
    wet: Plane,
    agri: Plane,
    agri_rows: HashMap<String, i32>,
    parents: Vec<i32>,
    land: Option<(usize, usize, Arc<Vec<u8>>)>,
    lakes: Vec<Vec<Vec2>>,
    towns: HashMap<IVec2, Vec<Vec2>>,
    villages: HashMap<IVec2, Vec<Village>>,
    roads: HashMap<IVec2, Vec<RoadSeg>>,
    pub town_count: usize,
    pub village_count: usize,
    pub road_segments: usize,
    pub rasters: Vec<(&'static str, bool)>,
}

fn point_in_polygon(p: Vec2, poly: &[Vec2]) -> bool {
    let mut inside = false;
    let mut j = poly.len().wrapping_sub(1);
    for i in 0..poly.len() {
        let (a, b) = (poly[i], poly[j]);
        if (a.y > p.y) != (b.y > p.y) && p.x < (b.x - a.x) * (p.y - a.y) / (b.y - a.y) + a.x {
            inside = !inside;
        }
        j = i;
    }
    inside
}

/// What the loader needs from the main thread.
pub struct Inputs {
    pub root: std::path::PathBuf,
    pub heights: Arc<HeightGrid>,
    pub life: LifeData,
    pub places: Vec<(Vec2, String)>,
    pub cell_px: f32,
}

fn read_json(path: &Path) -> Option<Value> {
    serde_json::from_str(&std::fs::read_to_string(path).ok()?).ok()
}

impl MapTerrain {
    pub fn load(inputs: Inputs) -> MapTerrain {
        let map = inputs.root.join("data/map");
        let load = |name: &str| read_png(&map.join(name)).unwrap_or_else(|_| Plane::empty());
        let splat = shrink(load("splat.png"), SHRINK);
        let biomes = shrink(load("biomes.png"), SHRINK);
        let river = load("river_bed.png");
        let wet = shrink(load("wetlands.png"), 2);
        let agri = load("agri_regions.png");
        let rasters = vec![
            ("splat", !splat.is_empty()),
            ("biomes", !biomes.is_empty()),
            ("river_bed", !river.is_empty()),
            ("wetlands", !wet.is_empty()),
            ("agri_regions", !agri.is_empty()),
        ];
        let mut agri_rows = HashMap::new();
        if let Some(doc) = read_json(&map.join("agri_landscapes.json"))
            && let Some(landscapes) = doc["landscapes"].as_object()
        {
            for (n, id) in landscapes.keys().enumerate() {
                agri_rows.insert(id.clone(), FIRST_LANDSCAPE_ROW + n as i32);
            }
        }
        let parents = read_json(&map.join("biome_parents.json"))
            .and_then(|v| {
                v["parents"].as_array().map(|a| {
                    a.iter()
                        .map(|x| x.as_i64().unwrap_or(0) as i32)
                        .collect::<Vec<_>>()
                })
            })
            .unwrap_or_default();
        let size = Vec2::new(inputs.heights.width as f32, inputs.heights.height as f32);
        let mut towns: HashMap<IVec2, Vec<Vec2>> = HashMap::new();
        let mut villages: HashMap<IVec2, Vec<Village>> = HashMap::new();
        let mut add_town = |p: Vec2, kind: &str| {
            towns.entry(cell_of(p, GRID_PX)).or_default().push(p);
            villages
                .entry(cell_of(p, inputs.cell_px))
                .or_default()
                .push(Village {
                    px: p,
                    kind: kind.to_string(),
                });
        };
        for (p, kind) in &inputs.places {
            add_town(*p, kind);
        }
        for (p, _, _) in &inputs.life.hamlets {
            add_town(*p, "hamlet");
        }
        let town_count = inputs.places.len() + inputs.life.hamlets.len();
        let mut roads: HashMap<IVec2, Vec<RoadSeg>> = HashMap::new();
        let mut road_segments = 0;
        for (run_index, run) in inputs.life.road_runs.iter().enumerate() {
            for n in 0..run.points.len().saturating_sub(1) {
                let (a, b) = (run.points[n], run.points[n + 1]);
                if a.distance_squared(b) < 1e-6 {
                    continue;
                }
                roads
                    .entry(cell_of((a + b) * 0.5, inputs.cell_px))
                    .or_default()
                    .push(RoadSeg {
                        a,
                        b,
                        main: run.main,
                        id: (run_index * 1000 + n) as u32,
                    });
                road_segments += 1;
            }
        }
        let village_count = villages.values().map(Vec::len).sum();
        MapTerrain {
            size,
            heights: inputs.heights,
            splat,
            biomes,
            river,
            wet,
            agri,
            agri_rows,
            parents,
            land: inputs
                .life
                .land
                .as_ref()
                .map(|l| (l.width, l.height, l.bytes.clone())),
            lakes: inputs.life.lakes.clone(),
            towns,
            villages,
            roads,
            town_count,
            village_count,
            road_segments,
            rasters,
        }
    }

    fn texel(&self, plane: &Plane, p: Vec2, c: usize) -> f32 {
        plane.texel(
            (self.size.x as f64, self.size.y as f64),
            p.x as f64,
            p.y as f64,
            c,
        )
    }
}

impl Terrain for MapTerrain {
    fn size(&self) -> Vec2 {
        self.size
    }

    fn is_land(&self, p: Vec2) -> bool {
        match &self.land {
            Some((w, h, bytes)) => {
                p.x >= 0.0
                    && p.y >= 0.0
                    && (p.x as usize) < *w
                    && (p.y as usize) < *h
                    && bytes[p.y as usize * w + p.x as usize] > 127
            }
            None => true,
        }
    }

    fn river_sd(&self, p: Vec2) -> f32 {
        if self.river.is_empty() {
            return 8.0;
        }
        let x = (p.x as i64).clamp(0, self.river.w as i64 - 1) as usize;
        let y = (p.y as i64).clamp(0, self.river.h as i64 - 1) as usize;
        (self.river.data[y * self.river.w + x] as f32 - 128.0) / 16.0
    }

    fn in_lake(&self, p: Vec2) -> bool {
        self.lakes.iter().any(|l| point_in_polygon(p, l))
    }

    fn height_m(&self, p: Vec2) -> f32 {
        self.heights.height_m_at(p.x, p.y)
    }

    fn splat(&self, p: Vec2) -> [f32; 4] {
        if self.splat.is_empty() {
            return [0.0; 4];
        }
        std::array::from_fn(|c| self.texel(&self.splat, p, c))
    }

    fn wet(&self, p: Vec2) -> [f32; 2] {
        if self.wet.is_empty() {
            return [0.0; 2];
        }
        [self.texel(&self.wet, p, 0), self.texel(&self.wet, p, 1)]
    }

    fn biome(&self, p: Vec2) -> i32 {
        if self.biomes.is_empty() {
            return -1;
        }
        (self.texel(&self.biomes, p, 0) * 255.0).round() as i32
    }

    fn biome_allows(&self, mask: u32, biome: i32) -> bool {
        if self.parents.is_empty() {
            return false;
        }
        let count = self.parents.len() as i32;
        let mut b = biome.clamp(0, count - 1);
        for _ in 0..3 {
            if (mask >> b) & 1 == 1 {
                return true;
            }
            let parent = self.parents[b as usize];
            if parent == b {
                return false;
            }
            b = parent;
        }
        false
    }

    fn landscape_row(&self, p: Vec2) -> i32 {
        if self.agri.is_empty() {
            return -1;
        }
        (self.texel(&self.agri, p, 0) * 255.0).round() as i32
    }

    fn landscape_id_row(&self, id: &str) -> i32 {
        self.agri_rows.get(id).copied().unwrap_or(-2)
    }

    fn near_town(&self, p: Vec2, clearance: f32) -> bool {
        let key = cell_of(p, GRID_PX);
        let reach = ((clearance / GRID_PX).ceil() as i32).max(1);
        for dy in -reach..=reach {
            for dx in -reach..=reach {
                if let Some(list) = self.towns.get(&(key + IVec2::new(dx, dy)))
                    && list
                        .iter()
                        .any(|t| t.distance_squared(p) < clearance * clearance)
                {
                    return true;
                }
            }
        }
        false
    }

    fn villages_in(&self, cell: IVec2) -> &[Village] {
        self.villages.get(&cell).map_or(&[], Vec::as_slice)
    }

    fn roads_in(&self, cell: IVec2) -> &[RoadSeg] {
        self.roads.get(&cell).map_or(&[], Vec::as_slice)
    }
}
