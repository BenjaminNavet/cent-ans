//! Placement of the portulan sea ornaments (port of `parchment_decor.gd`): wind roses and sea
//! monsters out at sea, ships near the coasts, chosen once from the signed coast-distance raster
//! (`data/map/coast_dist.png`, L8: `(value - 128) / scale` map px, negative at sea). No position is
//! hard-coded. Purely visual.

use bevy::math::Vec2;

/// Sampling grid step (map px).
pub const GRID: i32 = 24;
/// Distance to the coast beyond which the sea counts as "open" (map px).
pub const OPEN_SEA_PX: f32 = 45.0;
/// Radius of a wind rose (map px; the painted rose is drawn `ROSE_EXTENT` times larger).
pub const ROSE_RADIUS: f32 = 62.0;
const PAINTED_SHIP_COAST: f32 = 30.0;
const PAINTED_SHIP_CLEARANCE: f32 = 60.0;
const PAINTED_SHIP_GRID: i32 = GRID * 4;
const PAINTED_SHIP_CENTER_BIAS: f32 = 0.25;
const PAINTED_SHIP_SPACING: f32 = 700.0;
const PAINTED_MONSTER_CLEARANCE: f32 = 120.0;
const PAINTED_MONSTER_SHIP_SPACING: f32 = 480.0;
const COAST_MARGIN_PX: f32 = 8.0;

/// Signed coast-distance raster.
pub struct CoastRaster {
    pub width: u32,
    pub height: u32,
    /// One byte per pixel.
    pub data: Vec<u8>,
    /// Map size (px) the raster covers.
    pub map_size: Vec2,
    /// `coast_dist_scale` (2 for the shipped raster).
    pub scale: f32,
}

impl CoastRaster {
    /// Signed distance (map px) at a map pixel; 0 outside the map (counts as coast).
    pub fn at(&self, p: Vec2) -> f32 {
        if p.x < 0.0 || p.y < 0.0 || p.x >= self.map_size.x || p.y >= self.map_size.y {
            return 0.0;
        }
        let sx = self.width as f32 / self.map_size.x;
        let sy = self.height as f32 / self.map_size.y;
        let x = ((p.x * sx) as u32).min(self.width - 1);
        let y = ((p.y * sy) as u32).min(self.height - 1);
        (f32::from(self.data[(y * self.width + x) as usize]) - 128.0) / self.scale
    }

    /// Decodes an 8-bit grey PNG.
    pub fn from_png(bytes: &[u8], map_size: Vec2, scale: f32) -> Result<Self, String> {
        let mut decoder = png::Decoder::new(std::io::Cursor::new(bytes));
        decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
        let mut reader = decoder.read_info().map_err(|e| e.to_string())?;
        let mut buf = vec![0; reader.output_buffer_size().ok_or("png: size overflow")?];
        let info = reader.next_frame(&mut buf).map_err(|e| e.to_string())?;
        let channels = info.color_type.samples();
        let data: Vec<u8> = if channels == 1 {
            buf.truncate(info.buffer_size());
            buf
        } else {
            buf[..info.buffer_size()]
                .chunks_exact(channels)
                .map(|c| c[0])
                .collect()
        };
        Ok(Self {
            width: info.width,
            height: info.height,
            data,
            map_size,
            scale,
        })
    }
}

/// Kind of a sea ornament.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeaKind {
    Ship,
    Serpent,
    Whale,
}

impl SeaKind {
    pub fn name(self) -> &'static str {
        match self {
            SeaKind::Ship => "ship",
            SeaKind::Serpent => "serpent",
            SeaKind::Whale => "whale",
        }
    }
}

/// A placed ship or monster (`ParchmentDecor.sea_items`).
#[derive(Debug, Clone, PartialEq)]
pub struct SeaItem {
    pub kind: SeaKind,
    /// Map pixel.
    pub px: Vec2,
    /// Index of the painted ornament (`None` = drawn vignette).
    pub painted: Option<usize>,
    /// Drawing unit (`unit`, screen px at scale 1).
    pub unit: f32,
    pub heading_right: bool,
    /// Drawn vignette mirrored horizontally.
    pub mirror: bool,
}

/// Result of the placement.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Decor {
    /// Centre (map px) and radius of each wind rose (at most 4).
    pub roses: Vec<(Vec2, f32)>,
    pub items: Vec<SeaItem>,
}

/// How many painted ornaments of each kind the catalogue provides (`use` = `real`).
#[derive(Debug, Clone, Copy, Default)]
pub struct Painted {
    pub ships: usize,
    pub monsters: usize,
}

/// `ParchmentDecor._hash` (64-bit wrapping integer arithmetic of GDScript).
pub fn hash(x: i32, y: i32) -> f32 {
    let mut h = (i64::from(x).wrapping_mul(73_856_093)) ^ (i64::from(y).wrapping_mul(19_349_663));
    h = (h ^ (h >> 13)).wrapping_mul(1_274_126_177);
    (h.unsigned_abs() % 10007) as f32 / 10007.0
}

fn far_from(p: Vec2, points: &[Vec2], spacing: f32) -> bool {
    points.iter().all(|q| p.distance(*q) >= spacing)
}

/// Open water (more than `COAST_MARGIN_PX` off the coast) on a circle of `radius` and at half radius.
fn clear_water(coast: &CoastRaster, p: Vec2, radius: f32) -> bool {
    (0..8).all(|k| {
        let dir = Vec2::from_angle(std::f32::consts::TAU * k as f32 / 8.0);
        coast.at(p + dir * radius) <= -COAST_MARGIN_PX
            && coast.at(p + dir * radius * 0.5) <= -COAST_MARGIN_PX
    })
}

fn sort_desc(list: &mut [(f32, Vec2)]) {
    list.sort_by(|a, b| b.0.total_cmp(&a.0));
}

/// Places roses, monsters and ships (`ParchmentDecor._place` + `_resolve_sea_items`).
pub fn place(coast: &CoastRaster, painted: Painted) -> Decor {
    let mut decor = Decor::default();
    let painted_ships = painted.ships > 0;
    let painted_monsters = painted.monsters > 0;
    let size = coast.map_size;
    let (mut open_sea, mut near_sea) = (Vec::new(), Vec::new());
    let margin = GRID * 4;
    let mut y = margin;
    while y < size.y as i32 - margin {
        let mut x = margin;
        while x < size.x as i32 - margin {
            let p = Vec2::new(x as f32, y as f32);
            let c = coast.at(p);
            let jitter = hash(x, y);
            if c <= -OPEN_SEA_PX {
                open_sea.push((-c + jitter * 8.0, p));
            }
            let painted_spot = painted_ships
                && c < -PAINTED_SHIP_COAST
                && x % PAINTED_SHIP_GRID == 0
                && y % PAINTED_SHIP_GRID == 0;
            let drawn_spot = !painted_ships && c < -10.0 && c > -30.0;
            if painted_spot || drawn_spot {
                near_sea.push((jitter, p));
            }
            x += GRID;
        }
        y += GRID;
    }
    sort_desc(&mut open_sea);
    sort_desc(&mut near_sea);
    // Roses: the points farthest out at sea, spaced; the heart of the map first.
    let center = size * 0.5;
    let mut rose_candidates = open_sea.clone();
    rose_candidates.sort_by(|a, b| {
        let ka = a.0 - a.1.distance(center) * 0.02;
        let kb = b.0 - b.1.distance(center) * 0.02;
        kb.total_cmp(&ka)
    });
    let mut taken: Vec<Vec2> = Vec::new();
    for (_, p) in &rose_candidates {
        if far_from(*p, &taken, 850.0) {
            taken.push(*p);
            decor.roses.push((*p, ROSE_RADIUS));
            if decor.roses.len() >= 4 {
                break;
            }
        }
    }
    // Monsters: out at sea, far from the roses (first pass, as the original placement).
    let mut monsters: Vec<(Vec2, usize)> = Vec::new();
    let mut monster_taken = taken.clone();
    for (_, p) in &open_sea {
        if painted_monsters && painted_ships && !clear_water(coast, *p, PAINTED_MONSTER_CLEARANCE) {
            continue;
        }
        if far_from(*p, &monster_taken, 520.0) {
            monster_taken.push(*p);
            monsters.push((*p, monsters.len() % 2));
            if monsters.len() >= 4 {
                break;
            }
        }
    }
    let mut ships: Vec<(Vec2, bool)> = Vec::new();
    let mut ship_taken = monster_taken.clone();
    if painted_ships {
        let mut spots: Vec<Vec2> = near_sea
            .iter()
            .filter(|(_, p)| clear_water(coast, *p, PAINTED_SHIP_CLEARANCE))
            .map(|(_, p)| *p)
            .collect();
        while ships.len() < 9 && !spots.is_empty() {
            let mut best = 0;
            let mut best_score = f32::NEG_INFINITY;
            for (i, spot) in spots.iter().enumerate() {
                let nearest = ship_taken
                    .iter()
                    .map(|q| spot.distance_squared(*q))
                    .fold(f32::INFINITY, f32::min);
                let score = nearest.sqrt().min(PAINTED_SHIP_SPACING)
                    - spot.distance(center) * PAINTED_SHIP_CENTER_BIAS;
                if score > best_score {
                    best_score = score;
                    best = i;
                }
            }
            let p = spots.remove(best);
            ship_taken.push(p);
            ships.push((p, hash(p.x as i32, p.y as i32 + 7) > 0.5));
        }
    } else {
        for (_, p) in &near_sea {
            if far_from(*p, &ship_taken, 330.0) {
                ship_taken.push(*p);
                ships.push((*p, hash(p.x as i32, p.y as i32 + 7) > 0.5));
                if ships.len() >= 9 {
                    break;
                }
            }
        }
        if painted_monsters {
            // Painted monsters are larger: open water around them, away from the drawn ships.
            monsters.clear();
            let ship_points: Vec<Vec2> = ships.iter().map(|s| s.0).collect();
            let mut painted_taken = taken.clone();
            for (_, p) in &open_sea {
                if !clear_water(coast, *p, PAINTED_MONSTER_CLEARANCE) {
                    continue;
                }
                if far_from(*p, &painted_taken, 520.0)
                    && far_from(*p, &ship_points, PAINTED_MONSTER_SHIP_SPACING)
                {
                    painted_taken.push(*p);
                    monsters.push((*p, monsters.len() % 2));
                    if monsters.len() >= 4 {
                        break;
                    }
                }
            }
        }
    }
    for (i, (p, east)) in ships.iter().enumerate() {
        decor.items.push(SeaItem {
            kind: SeaKind::Ship,
            px: *p,
            painted: painted_ships.then(|| i % painted.ships),
            unit: 16.0,
            heading_right: *east,
            mirror: !*east,
        });
    }
    for (i, (p, variant)) in monsters.iter().enumerate() {
        decor.items.push(SeaItem {
            kind: if *variant == 0 {
                SeaKind::Serpent
            } else {
                SeaKind::Whale
            },
            px: *p,
            painted: painted_monsters.then(|| variant % painted.monsters),
            unit: 20.0,
            heading_right: i % 2 == 1,
            mirror: false,
        });
    }
    decor
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 2000 × 1200 map: land on the left, sea elsewhere (distance grows away from x = 600).
    fn raster() -> CoastRaster {
        let (w, h) = (500u32, 300u32);
        let mut data = Vec::with_capacity((w * h) as usize);
        for _y in 0..h {
            for x in 0..w {
                let px = x as f32 * 4.0;
                let signed = ((600.0 - px) * 0.25).clamp(-63.0, 63.0) * 2.0;
                data.push((128.0 + signed) as u8);
            }
        }
        CoastRaster {
            width: w,
            height: h,
            data,
            map_size: Vec2::new(2000.0, 1200.0),
            scale: 2.0,
        }
    }

    #[test]
    fn hash_is_a_unit_value() {
        let h = hash(24, 48);
        assert!((0.0..1.0).contains(&h));
        assert_ne!(hash(24, 48), hash(48, 24));
    }

    #[test]
    fn roses_out_at_sea_and_ships_near_the_coast() {
        let coast = raster();
        let decor = place(
            &coast,
            Painted {
                ships: 0,
                monsters: 1,
            },
        );
        assert!(!decor.roses.is_empty());
        for (p, _) in &decor.roses {
            assert!(coast.at(*p) <= -OPEN_SEA_PX, "{p}");
        }
        for (i, a) in decor.roses.iter().enumerate() {
            for b in &decor.roses[i + 1..] {
                assert!(a.0.distance(b.0) >= 850.0);
            }
        }
        let ships: Vec<_> = decor
            .items
            .iter()
            .filter(|i| i.kind == SeaKind::Ship)
            .collect();
        assert!(!ships.is_empty());
        for ship in &ships {
            let c = coast.at(ship.px);
            assert!(c < -10.0 && c > -30.0, "{c}");
            assert!(ship.painted.is_none());
        }
        for monster in decor.items.iter().filter(|i| i.kind != SeaKind::Ship) {
            assert_eq!(monster.painted, Some(0));
            for ship in &ships {
                assert!(monster.px.distance(ship.px) >= PAINTED_MONSTER_SHIP_SPACING);
            }
        }
    }
}
