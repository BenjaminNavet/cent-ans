//! Buildings outside the walls (farm, mill, vineyard, mine, saltworks, abbey, market, port),
//! posed around the maquettes of the cities and towns according to the buildings the simulation
//! says are built (`data/map/building_models.json`, models `game/assets/models/outbuildings/`).
//! Port of `outbuilding_layer.gd` in its `maquette` style (ADR 0158): constant world size (what
//! each model would hold on screen at `render.maquette.size_distance`), one model per family at
//! the highest level reached, shown within `view_range_units` of the camera.
//!
//! Not ported: the band layout without overlap (`_layout`), the screen-held sizes of the `real`
//! style, the shore/river/road sites (families are spread by sector, sea is avoided), the
//! suburb signs, the opacity fade and the worksite model (the markers lot draws worksites).

use std::collections::{BTreeMap, HashMap};
use std::path::Path;

use bevy::gltf::GltfAssetLabel;
use bevy::prelude::*;
use bevy::world_serialization::WorldAssetRoot;
use serde::Deserialize;
use serde::de::{Deserializer, MapAccess, Visitor};
use serde_json::{Value, json};

use crate::agent::dump::DumpAppExt;
use crate::app::AppState;
use crate::campaign::markers::GroundAnchor;
use crate::campaign::markers::places::Places;
use crate::campaign::terrain::surface::Surface;
use crate::campaign::weather::fog::FogOfWar;
use crate::paths;
use crate::sim::Sim;

use super::layer::TownsWorld;

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Group {
    #[serde(default)]
    pub of: Vec<String>,
    #[serde(default = "one")]
    pub min: u32,
}

fn one() -> u32 {
    1
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct When {
    #[serde(default)]
    pub require: Vec<Group>,
    #[serde(default)]
    pub resources: Vec<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Level {
    pub level: u8,
    pub model: String,
    #[serde(default)]
    pub when: When,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Family {
    #[serde(default)]
    pub site: String,
    #[serde(default)]
    pub levels: Vec<Level>,
}

/// Families in file order (the slot of a family is its rank).
#[derive(Debug, Clone, Default)]
pub struct Families(pub Vec<(String, Family)>);

impl<'de> Deserialize<'de> for Families {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct V;
        impl<'de> Visitor<'de> for V {
            type Value = Families;
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("a map of families")
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Families, A::Error> {
                let mut out = Vec::new();
                while let Some((k, v)) = map.next_entry::<String, Family>()? {
                    out.push((k, v));
                }
                Ok(Families(out))
            }
        }
        d.deserialize_map(V)
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct MaquetteRender {
    #[serde(default = "d_size_distance")]
    pub size_distance: f32,
    #[serde(default = "d_view_range")]
    pub view_range_units: f32,
    #[serde(default)]
    pub kinds: Vec<String>,
    #[serde(default)]
    pub built_in_game_only: Vec<String>,
}

fn d_size_distance() -> f32 {
    50.0
}
fn d_view_range() -> f32 {
    330.0
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct ScreenRender {
    #[serde(default)]
    pub fractions: Vec<f32>,
    #[serde(default = "d_vertical")]
    pub vertical: f32,
}

fn d_vertical() -> f32 {
    1.3
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Render {
    #[serde(default = "d_cap")]
    pub max_per_settlement: usize,
    #[serde(default)]
    pub screen: ScreenRender,
    #[serde(default)]
    pub maquette: MaquetteRender,
}

fn d_cap() -> usize {
    4
}

/// One entry of `models/outbuildings/manifest.json` (metres).
#[derive(Debug, Clone, Default, Deserialize)]
pub struct ModelInfo {
    #[serde(default)]
    pub length: f32,
    #[serde(default)]
    pub depth: f32,
    #[serde(default)]
    pub radius: f32,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct OutConfig {
    #[serde(default)]
    pub families: Families,
    #[serde(default)]
    pub render: Render,
    #[serde(skip)]
    pub models: BTreeMap<String, ModelInfo>,
}

impl OutConfig {
    pub fn load(data_dir: &Path, assets_dir: &Path) -> OutConfig {
        let mut cfg: OutConfig = std::fs::read_to_string(data_dir.join("map/building_models.json"))
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_default();
        cfg.models = std::fs::read_to_string(assets_dir.join("models/outbuildings/manifest.json"))
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_default();
        cfg
    }

    pub fn is_empty(&self) -> bool {
        self.families.0.is_empty() || self.models.is_empty()
    }

    /// Screen fraction of a level (1 to 3) of the held width.
    fn fraction(&self, level: u8) -> f32 {
        let f = &self.render.screen.fractions;
        f.get(
            (level as usize)
                .saturating_sub(1)
                .min(f.len().saturating_sub(1)),
        )
        .copied()
        .unwrap_or(0.0)
    }

    /// Scale factor (>= 1) of a model `width_m` wide: what it holds at `size_distance` (fixed
    /// world size of the `maquette` style, full screen blend).
    pub fn model_factor(&self, width_m: f32, level: u8, fov_deg: f32, mpu: f32) -> f32 {
        let span = 2.0 * (fov_deg.to_radians() * 0.5).tan() * self.render.maquette.size_distance;
        (self.fraction(level) * span * mpu / width_m.max(0.1)).max(1.0)
    }
}

/// A model to pose around a settlement.
#[derive(Debug, Clone, PartialEq)]
pub struct OutModel {
    pub family: String,
    pub level: u8,
    pub model: String,
    pub site: String,
    pub slot: usize,
}

fn level_reached(when: &When, buildings: &[String], resources: &[String]) -> bool {
    for g in &when.require {
        let built = g.of.iter().filter(|b| buildings.contains(b)).count() as u32;
        if built < g.min {
            return false;
        }
    }
    when.resources.is_empty() || when.resources.iter().any(|r| resources.contains(r))
}

fn family_level(family: &Family, buildings: &[String], resources: &[String]) -> u8 {
    family
        .levels
        .iter()
        .filter(|l| level_reached(&l.when, buildings, resources))
        .map(|l| l.level)
        .max()
        .unwrap_or(0)
}

/// Models around a settlement: one per family at the highest level reached, at most
/// `max_per_settlement` (highest levels first, then family order), in slot order.
pub fn models_for(cfg: &OutConfig, buildings: &[String], resources: &[String]) -> Vec<OutModel> {
    let mut out = Vec::new();
    for (slot, (name, family)) in cfg.families.0.iter().enumerate() {
        let level = family_level(family, buildings, resources);
        if level == 0 {
            continue;
        }
        let Some(model) = family
            .levels
            .iter()
            .find(|l| l.level == level)
            .map(|l| l.model.clone())
        else {
            continue;
        };
        out.push(OutModel {
            family: name.clone(),
            level,
            model,
            site: family.site.clone(),
            slot,
        });
    }
    if out.len() > cfg.render.max_per_settlement {
        out.sort_by(|a, b| b.level.cmp(&a.level).then(a.slot.cmp(&b.slot)));
        out.truncate(cfg.render.max_per_settlement);
        out.sort_by_key(|m| m.slot);
    }
    out
}

/// Maquette style: families the layer keeps for a settlement. `built_in_game_only` families (the
/// generic decoration already carries the 1337 ones) need one of their own buildings (first
/// `require` group of any level) built during the game, i.e. absent from `initial`.
pub fn keep_in_maquette_style(
    cfg: &OutConfig,
    model: &OutModel,
    buildings: &[String],
    initial: &[String],
) -> bool {
    if !cfg
        .render
        .maquette
        .built_in_game_only
        .contains(&model.family)
    {
        return true;
    }
    cfg.families
        .0
        .iter()
        .find(|(n, _)| *n == model.family)
        .is_some_and(|(_, f)| {
            f.levels.iter().any(|l| {
                l.when.require.first().is_some_and(|g| {
                    g.of.iter()
                        .any(|b| buildings.contains(b) && !initial.contains(b))
                })
            })
        })
}

/// Centre (map px) and yaw of a model of slot `slot` around a maquette of radius `radius`
/// (units) at `center`: seed sector, façade towards the settlement, clear of the maquette by
/// the model's own reach. `step` rotates the try by thirds of a sector (sea avoidance).
pub fn anchor_of(
    id_seed: u32,
    slot: usize,
    families: usize,
    radius: f32,
    reach: f32,
    center: Vec2,
    step: i32,
) -> (Vec2, f32) {
    let sector = std::f32::consts::TAU / (families as f32 + 1.0);
    let theta0 = (id_seed % 3600) as f32 / 3600.0 * std::f32::consts::TAU;
    let angle = theta0 + (slot as f32 + step as f32 / 3.0) * sector;
    let dir = Vec2::new(angle.cos(), angle.sin());
    let facing = -dir;
    (
        center + dir * (radius + 0.1 + reach),
        facing.x.atan2(facing.y),
    )
}

/// Order of the sector tries (thirds of a sector), nearest first.
pub const ANGLE_STEPS: [i32; 13] = [0, -1, 1, -2, 2, -4, 4, -6, 6, -9, 9, -12, 12];

// --- Bevy layer -------------------------------------------------------------------------------

#[derive(Component)]
pub struct Outbuilding {
    pub place: String,
    pub family: String,
    pub province: String,
}

#[derive(Resource, Default)]
pub struct OutState {
    config: Option<OutConfig>,
    /// Signature of the models built for each place id.
    signatures: HashMap<String, String>,
    entities: HashMap<String, Vec<Entity>>,
    built_total: usize,
    shown: usize,
    fogged: usize,
    by_family: BTreeMap<String, usize>,
    timer: f32,
    ground_checked: usize,
}

pub fn add(app: &mut App) {
    app.init_resource::<OutState>()
        .register_dump(dump_outbuildings)
        .add_systems(OnExit(AppState::Campaign), reset)
        .add_systems(
            Update,
            (refresh_outbuildings, show_outbuildings)
                .chain()
                .run_if(in_state(AppState::Campaign)),
        );
}

fn reset(mut state: ResMut<OutState>) {
    *state = OutState::default();
}

const FOV_DEG: f32 = 55.0;

#[allow(clippy::too_many_arguments)]
fn refresh_outbuildings(
    mut commands: Commands,
    time: Res<Time>,
    mut state: ResMut<OutState>,
    sim: Res<Sim>,
    places: Res<Places>,
    world: Option<Res<TownsWorld>>,
    surface: Option<Res<Surface>>,
    assets: Res<AssetServer>,
) {
    let (Some(world), Some(surface)) = (world, surface) else {
        return;
    };
    if places.places.is_empty() {
        return;
    }
    state.timer += time.delta_secs();
    if state.config.is_some() && state.timer < 1.0 {
        return;
    }
    state.timer = 0.0;
    let (Some(data), Some(game)) = (sim.data.as_ref(), sim.state.as_ref()) else {
        return;
    };
    if state.config.is_none() {
        let Ok(root) = paths::find_root() else { return };
        state.config = Some(OutConfig::load(
            &paths::data_dir(&root),
            &paths::asset_root(&root),
        ));
    }
    let cfg = state.config.take().unwrap();
    if cfg.is_empty() {
        state.config = Some(cfg);
        return;
    }
    let mpu = world.db.meters_per_unit;
    for place in &places.places {
        if !cfg.render.maquette.kinds.contains(&place.kind) {
            continue;
        }
        let Ok(sid) = data_model::SettlementId::new(&place.id) else {
            continue;
        };
        let (Some(live), Some(settlement)) =
            (game.settlement_state(&sid), data.settlements.get(&sid))
        else {
            continue;
        };
        let buildings: Vec<String> = live
            .buildings
            .iter()
            .map(|b| b.as_str().to_string())
            .collect();
        let initial: Vec<String> = settlement
            .buildings
            .iter()
            .map(|b| b.as_str().to_string())
            .collect();
        let resources: Vec<String> = data
            .provinces
            .get(&settlement.province)
            .map(|p| p.resources.iter().map(|r| r.as_str().to_string()).collect())
            .unwrap_or_default();
        let models: Vec<OutModel> = models_for(&cfg, &buildings, &resources)
            .into_iter()
            .filter(|m| keep_in_maquette_style(&cfg, m, &buildings, &initial))
            .collect();
        let signature = format!(
            "{:.2}|{}",
            place.radius,
            models
                .iter()
                .map(|m| m.model.as_str())
                .collect::<Vec<_>>()
                .join(",")
        );
        if state.signatures.get(&place.id) == Some(&signature) {
            continue;
        }
        for e in state.entities.remove(&place.id).unwrap_or_default() {
            commands.entity(e).despawn();
        }
        let seed = super::super::markers::placement::stable_hash(&place.id);
        let mut spawned = Vec::new();
        for model in &models {
            let Some(info) = cfg.models.get(&model.model) else {
                continue;
            };
            let width_m = info.length.max(info.depth).max(1.0);
            let factor = cfg.model_factor(width_m, model.level, FOV_DEG, mpu);
            let reach = factor * info.radius / mpu;
            // First sector try on land (sea and shore are rejected).
            let mut chosen = None;
            for step in ANGLE_STEPS {
                let (px, yaw) = anchor_of(
                    seed,
                    model.slot,
                    cfg.families.0.len(),
                    place.radius,
                    reach,
                    place.px,
                    step,
                );
                if surface.height_m_at(px.x, px.y) > 0.5 {
                    chosen = Some((px, yaw));
                    break;
                }
            }
            let Some((px, yaw)) = chosen else { continue };
            let k = factor / mpu;
            let entity = commands
                .spawn((
                    Name::new(format!("Out_{}_{}", place.id, model.model)),
                    Outbuilding {
                        place: place.id.clone(),
                        family: model.family.clone(),
                        province: place.province.clone(),
                    },
                    WorldAssetRoot(
                        assets.load(
                            GltfAssetLabel::Scene(0)
                                .from_asset(format!("models/outbuildings/{}.glb", model.model)),
                        ),
                    ),
                    Transform::from_translation(Vec3::new(px.x, 0.0, px.y))
                        .with_rotation(Quat::from_rotation_y(yaw))
                        .with_scale(Vec3::new(k, k * cfg.render.screen.vertical, k)),
                    Visibility::Hidden,
                    GroundAnchor { px, lift: 0.0 },
                    DespawnOnExit(AppState::Campaign),
                ))
                .id();
            spawned.push(entity);
        }
        state.signatures.insert(place.id.clone(), signature);
        state.entities.insert(place.id.clone(), spawned);
    }
    state.built_total = state.entities.values().map(Vec::len).sum();
    state.config = Some(cfg);
}

/// Shows the outbuildings within the view range of the camera, except under the fog of war.
fn show_outbuildings(
    mut state: ResMut<OutState>,
    camera: Query<&GlobalTransform, With<Camera3d>>,
    fog: Option<Res<FogOfWar>>,
    mut out: Query<(&Outbuilding, &Transform, &mut Visibility)>,
) {
    let (Some(cam), Some(cfg)) = (camera.iter().next(), state.config.as_ref()) else {
        return;
    };
    let range = cfg.render.maquette.view_range_units;
    let eye = cam.translation();
    let (mut shown, mut fogged) = (0, 0);
    let mut by_family: BTreeMap<String, usize> = BTreeMap::new();
    for (o, t, mut vis) in &mut out {
        *by_family.entry(o.family.clone()).or_default() += 1;
        let hidden_by_fog = fog
            .as_ref()
            .is_some_and(|f| !f.is_province_visible(&o.province));
        fogged += usize::from(hidden_by_fog);
        let show = !hidden_by_fog && eye.distance(t.translation) < range;
        shown += usize::from(show);
        let want = if show {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if *vis != want {
            *vis = want;
        }
    }
    state.shown = shown;
    state.fogged = fogged;
    state.by_family = by_family;
}

fn dump_outbuildings(world: &World) -> (String, Value) {
    let s = world.resource::<OutState>();
    (
        "campaign_outbuildings".into(),
        json!({
            "config_loaded": s.config.as_ref().is_some_and(|c| !c.is_empty()),
            "places": s.entities.len(),
            "built": s.built_total,
            "shown": s.shown,
            "fogged": s.fogged,
            "by_family": s.by_family,
            "ground_checked": s.ground_checked,
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn real() -> Option<OutConfig> {
        let root = paths::find_root().ok()?;
        Some(OutConfig::load(
            &paths::data_dir(&root),
            &paths::asset_root(&root),
        ))
    }

    #[test]
    fn families_keep_file_order() {
        let Some(cfg) = real() else { return };
        let names: Vec<_> = cfg.families.0.iter().map(|(n, _)| n.as_str()).collect();
        assert_eq!(
            names,
            [
                "farm",
                "mill",
                "vineyard",
                "mine",
                "saltworks",
                "abbey",
                "market",
                "port"
            ]
        );
        assert!(cfg.models.contains_key("farm_1"));
    }

    #[test]
    fn levels_follow_buildings_and_resources() {
        let Some(cfg) = real() else { return };
        let none: Vec<String> = vec![];
        assert!(models_for(&cfg, &none, &none).is_empty());
        let market = vec!["bld_market".to_string()];
        let wheat = vec!["res_wheat".to_string()];
        let m = models_for(&cfg, &market, &wheat);
        assert!(
            m.iter().any(|m| m.family == "farm" && m.level == 1),
            "{m:?}"
        );
        // No matching resource: no farm.
        let wine = vec!["res_none".to_string()];
        assert!(
            !models_for(&cfg, &market, &wine)
                .iter()
                .any(|m| m.family == "farm")
        );
    }

    #[test]
    fn factor_is_at_least_one_and_grows_with_level() {
        let Some(cfg) = real() else { return };
        let a = cfg.model_factor(40.0, 1, 55.0, 719.0);
        let b = cfg.model_factor(40.0, 3, 55.0, 719.0);
        assert!(a >= 1.0 && b > a, "{a} {b}");
    }

    #[test]
    fn anchor_is_outside_the_maquette_facing_it() {
        let (p, yaw) = anchor_of(1234, 2, 8, 3.0, 1.0, Vec2::new(100.0, 100.0), 0);
        assert!((p.distance(Vec2::new(100.0, 100.0)) - 4.1).abs() < 1e-4);
        // The façade (+Z rotated by yaw) points back to the centre.
        let facing = Vec2::new(yaw.sin(), yaw.cos());
        let to_centre = (Vec2::new(100.0, 100.0) - p).normalize();
        assert!(facing.dot(to_centre) > 0.999);
    }

    #[test]
    fn built_in_game_only_needs_a_new_building() {
        let Some(cfg) = real() else { return };
        let mill = OutModel {
            family: "mill".into(),
            level: 1,
            model: "mill_1".into(),
            site: "water".into(),
            slot: 1,
        };
        let first = cfg.families.0[1].1.levels[0].when.require[0].of[0].clone();
        let built = vec![first];
        assert!(!keep_in_maquette_style(&cfg, &mill, &built, &built));
        assert!(keep_in_maquette_style(&cfg, &mill, &built, &[]));
    }
}
