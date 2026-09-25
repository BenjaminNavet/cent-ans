//! Line of sight on the campaign map (lot C1 fog of war, per-cell sight
//! radius since lot M5a).
//!
//! A map point is seen by a faction when it lies within `vision_army_km` of
//! one of its armies, or within `vision_settlement_km` of a settlement it
//! controls (`data/movement/rules.json`, [`FreeMovementRules`]). With
//! `share_allied_vision` (`data/rules/vision.json`, [`VisionRules`]), allies
//! (vassals and overlords included, see [`CampaignState::is_allied`]) lend
//! their armies and settlements. C6 agents still keep whole provinces in
//! sight ([`CampaignState::agent_sight`], spread in land steps), and with
//! `own_provinces_visible` every land cell of a province controlled by the
//! faction or a lending ally is seen.
//!
//! The seen area is rasterised as discs on a reduced grid ([`VisionMask`],
//! 512² texels over the 4096² map, one texel = 4×4 navigation cells, soft
//! edge `edge_feather_km` wide, `data/rules/vision.json`); the
//! point tests (armies, settlements) use the exact distance to the sources.
//! A province is visible when `province_seen_percent` of its land texels
//! are seen, when one of its settlements is seen, when a lending army stands
//! in it or when an agent watches it.
//!
//! The front end veils what is not seen and hides the foreign armies whose
//! point is not seen. Nothing in the simulation depends on it: vision is
//! recomputed on demand and never saved. **The AI does not use it** (it
//! reads the whole state, as before lot M5a: the AI "cheats" by design in
//! v1, see `docs/wip/m5a-vision.md`).

use std::collections::{BTreeMap, BTreeSet};

use data_model::entities::movement::FreeMovementRules;
use data_model::{FactionId, GameData, ProvinceId, VisionRules};

use crate::movement::land_neighbors;
use crate::state::{Army, ArmyId, CampaignState};

/// Side of the exported vision mask, in texels (the map is square).
pub const VISION_MASK_SIZE: u32 = 512;
/// Texel value from which a texel counts as seen.
pub const SEEN_THRESHOLD: u8 = 128;

/// A source of sight: a map-pixel point and its radius in map pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SightSource {
    pub point: [f32; 2],
    pub radius_px: f32,
}

/// What a faction sees this turn: a raster of the seen area plus the exact
/// sources it was drawn from.
#[derive(Debug, Clone, PartialEq)]
pub struct VisionMask {
    pub width: u32,
    pub height: u32,
    /// Map pixels per texel.
    pub texel_px: f32,
    /// Row-major coverage, 0 (unseen) to 255 (seen); seen from
    /// [`SEEN_THRESHOLD`]. Soft over the last few kilometres of each disc.
    pub coverage: Vec<u8>,
    /// Armies and settlements lending sight.
    pub sources: Vec<SightSource>,
    /// Provinces kept in sight whole: held provinces (own and lending
    /// allies', `own_provinces_visible`), C6 agents, intelligence.
    pub watched_provinces: BTreeSet<ProvinceId>,
}

impl VisionMask {
    /// `true` when map pixel `point` is seen: within a source's radius, or
    /// inside a watched province.
    pub fn sees_point(&self, data: &GameData, point: [f32; 2]) -> bool {
        self.sources.iter().any(|s| {
            let (dx, dy) = (s.point[0] - point[0], s.point[1] - point[1]);
            dx * dx + dy * dy <= s.radius_px * s.radius_px
        }) || (!self.watched_provinces.is_empty()
            && data
                .province_at_point(point[0], point[1])
                .is_some_and(|p| self.watched_provinces.contains(p)))
    }

    /// Coverage of the texel holding map pixel `point` (0 off the mask).
    pub fn coverage_at(&self, point: [f32; 2]) -> u8 {
        let x = (point[0] / self.texel_px).floor();
        let y = (point[1] / self.texel_px).floor();
        if x < 0.0 || y < 0.0 || x >= self.width as f32 || y >= self.height as f32 {
            return 0;
        }
        self.coverage[(y as usize) * (self.width as usize) + x as usize]
    }

    /// Share of texels seen (0-1), for statistics and tests.
    pub fn seen_share(&self) -> f32 {
        let seen = self
            .coverage
            .iter()
            .filter(|v| **v >= SEEN_THRESHOLD)
            .count();
        seen as f32 / self.coverage.len().max(1) as f32
    }
}

/// A faction's full vision: the mask and the visible provinces.
#[derive(Debug, Clone, PartialEq)]
pub struct Vision {
    pub faction: FactionId,
    pub mask: VisionMask,
    pub provinces: BTreeSet<ProvinceId>,
    /// Factions lending their sight (the faction itself, its allies).
    pub lenders: BTreeSet<FactionId>,
}

impl Vision {
    /// `true` when `army` is shown to the faction: its own and lenders'
    /// armies always, the others when their point is seen.
    pub fn sees_army(&self, state: &CampaignState, data: &GameData, army: &Army) -> bool {
        self.lenders.contains(&army.faction)
            || self.mask.sees_point(data, state.army_point(data, army))
    }
}

impl CampaignState {
    /// Everything `faction` sees this turn (see the module doc).
    pub fn vision(&self, data: &GameData, faction: &FactionId) -> Vision {
        let default_rules = VisionRules::default();
        let rules = data.vision_rules.as_ref().unwrap_or(&default_rules);
        let free: &FreeMovementRules = data.free_movement_rules();
        let lends_sight = |other: &FactionId| {
            other == faction || (rules.share_allied_vision && self.is_allied(faction, other))
        };
        let grid = data.navgrid();
        let px_per_km = grid.px_per_km() as f32;
        let army_px = free.vision_army_km as f32 * px_per_km;
        let settlement_px = free.vision_settlement_km as f32 * px_per_km;

        let mut lenders: BTreeSet<FactionId> = BTreeSet::new();
        let mut sources = Vec::new();
        let mut source_provinces: BTreeSet<ProvinceId> = BTreeSet::new();
        for army in self.armies.values() {
            if lends_sight(&army.faction) {
                sources.push(SightSource {
                    point: self.army_point(data, army),
                    radius_px: army_px,
                });
                if let Some(province) = self.army_province(data, army) {
                    source_provinces.insert(province);
                }
            }
        }
        for (id, settlement) in &self.settlements {
            if lends_sight(&settlement.controller) {
                if let Some(point) = data.settlement_point(id) {
                    sources.push(SightSource {
                        point,
                        radius_px: settlement_px,
                    });
                }
            }
        }
        lenders.insert(faction.clone());
        for other in self.factions.keys() {
            if lends_sight(other) {
                lenders.insert(other.clone());
            }
        }

        // C6: spies see around them; scouting keeps a province in sight.
        let mut range: BTreeMap<ProvinceId, u32> = BTreeMap::new();
        for (province, steps) in self.agent_sight(data, &lends_sight, faction) {
            let best = range.entry(province).or_insert(steps);
            *best = (*best).max(steps);
        }
        let mut watched_provinces = spread_sight(data, range);
        // Total War: a faction always sees the land of the provinces it (or
        // an ally lending sight) controls.
        if rules.own_provinces_visible {
            watched_provinces.extend(
                self.provinces
                    .keys()
                    .filter(|id| self.province_controller(id).is_some_and(lends_sight))
                    .cloned(),
            );
        }

        let map_w = (grid.width * grid.scale) as f32;
        let map_h = (grid.height * grid.scale) as f32;
        let width = VISION_MASK_SIZE.min(grid.width).max(1);
        let texel_px = map_w / width as f32;
        let height = ((map_h / texel_px).ceil() as u32).max(1);
        let mut mask = VisionMask {
            width,
            height,
            texel_px,
            coverage: vec![0; (width as usize) * (height as usize)],
            sources,
            watched_provinces,
        };
        rasterise_discs(&mut mask, rules.edge_feather_km as f32 * px_per_km);
        let provinces = self.province_sight(data, rules, &mut mask, source_provinces);
        Vision {
            faction: faction.clone(),
            mask,
            provinces,
            lenders,
        }
    }

    /// Provinces `faction` can see this turn (see the module doc).
    pub fn visible_provinces(&self, data: &GameData, faction: &FactionId) -> BTreeSet<ProvinceId> {
        self.vision(data, faction).provinces
    }

    /// Armies shown to `faction`: its own, its allies', and the others
    /// whose point is seen.
    pub fn visible_armies(&self, data: &GameData, faction: &FactionId) -> BTreeSet<ArmyId> {
        let vision = self.vision(data, faction);
        self.armies
            .iter()
            .filter(|(_, army)| vision.sees_army(self, data, army))
            .map(|(id, _)| id.clone())
            .collect()
    }

    /// Visible provinces from the rasterised mask (fills the watched
    /// provinces into it on the way).
    fn province_sight(
        &self,
        data: &GameData,
        rules: &VisionRules,
        mask: &mut VisionMask,
        mut visible: BTreeSet<ProvinceId>,
    ) -> BTreeSet<ProvinceId> {
        visible.extend(mask.watched_provinces.iter().cloned());
        for (id, settlement) in &self.settlements {
            if !visible.contains(&settlement.province)
                && data
                    .settlement_point(id)
                    .is_some_and(|p| mask.sees_point(data, p))
            {
                visible.insert(settlement.province.clone());
            }
        }
        let Some(raster) = data.province_raster() else {
            return visible
                .into_iter()
                .filter(|p| self.provinces.contains_key(p))
                .collect();
        };
        let slots = raster.ids.len();
        let watched: Vec<bool> = raster
            .ids
            .iter()
            .map(|id| {
                id.as_ref()
                    .is_some_and(|p| mask.watched_provinces.contains(p))
            })
            .collect();
        let mut total = vec![0u32; slots];
        let mut seen = vec![0u32; slots];
        let (rw, rh) = (raster.width as usize, raster.height as usize);
        for ty in 0..mask.height as usize {
            let py = ((ty as f32 + 0.5) * mask.texel_px) as usize;
            if py >= rh {
                break;
            }
            let row = py * rw;
            for tx in 0..mask.width as usize {
                let px = ((tx as f32 + 0.5) * mask.texel_px) as usize;
                if px >= rw {
                    break;
                }
                let slot = usize::from(raster.indices[row + px]);
                if slot == 0 || slot >= slots {
                    continue;
                }
                let texel = ty * mask.width as usize + tx;
                if watched[slot] {
                    mask.coverage[texel] = 255;
                }
                total[slot] += 1;
                if mask.coverage[texel] >= SEEN_THRESHOLD {
                    seen[slot] += 1;
                }
            }
        }
        let percent = rules.province_seen_percent.clamp(1, 100);
        for (slot, id) in raster.ids.iter().enumerate() {
            if let Some(id) = id {
                if total[slot] > 0 && seen[slot] * 100 >= percent * total[slot] {
                    visible.insert(id.clone());
                }
            }
        }
        visible
            .into_iter()
            .filter(|p| self.provinces.contains_key(p))
            .collect()
    }
}

/// Draws every source's disc into the mask (max of the coverages), with a
/// soft edge of `feather_px` map pixels centred on the radius.
fn rasterise_discs(mask: &mut VisionMask, feather_px: f32) {
    let feather = feather_px.max(1e-3);
    let texel = mask.texel_px;
    for source in &mask.sources {
        let reach = source.radius_px + feather * 0.5;
        let x0 = ((source.point[0] - reach) / texel).floor().max(0.0) as i64;
        let y0 = ((source.point[1] - reach) / texel).floor().max(0.0) as i64;
        let x1 = (((source.point[0] + reach) / texel).ceil() as i64).min(i64::from(mask.width) - 1);
        let y1 =
            (((source.point[1] + reach) / texel).ceil() as i64).min(i64::from(mask.height) - 1);
        for ty in y0..=y1 {
            let cy = (ty as f32 + 0.5) * texel - source.point[1];
            let row = (ty as usize) * (mask.width as usize);
            for tx in x0..=x1 {
                let cx = (tx as f32 + 0.5) * texel - source.point[0];
                let distance = (cx * cx + cy * cy).sqrt();
                let t = (0.5 + (source.radius_px - distance) / feather).clamp(0.0, 1.0);
                let value = (t * 255.0).round() as u8;
                let slot = &mut mask.coverage[row + tx as usize];
                *slot = (*slot).max(value);
            }
        }
    }
}

/// Spreads each seed's remaining range along land edges (multi-source search
/// that only revisits a province when it is reached with more range left).
fn spread_sight(data: &GameData, mut range: BTreeMap<ProvinceId, u32>) -> BTreeSet<ProvinceId> {
    let mut frontier: Vec<(ProvinceId, u32)> = range
        .iter()
        .map(|(id, steps)| (id.clone(), *steps))
        .collect();
    while let Some((province, steps)) = frontier.pop() {
        if steps == 0 || range.get(&province).is_some_and(|best| *best > steps) {
            continue;
        }
        for neighbour in land_neighbors(data, &province) {
            let left = steps - 1;
            if range.get(neighbour).is_none_or(|best| *best < left) {
                range.insert(neighbour.clone(), left);
                frontier.push((neighbour.clone(), left));
            }
        }
    }
    range.into_keys().collect()
}
