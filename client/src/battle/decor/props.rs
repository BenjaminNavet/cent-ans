//! Generated GA3 props of the battle site (port of `game/scripts/visual/ga3_kit.gd`, which
//! `building_kit.gd` and `battle_site_features.gd` call): the player's TRELLIS models of
//! `game/assets/models/props_ga/` (manifest + `ga3_<name>_lod{0,1,2}.glb`) replace part of the kit
//! models (house, church, windmill, tent, well, cart: `share` of the manifest, drawn on the
//! position) and make the palisade segments. Absent under snow (no snowy variant) and when a
//! kit model is a ruin.
//!
//! - [`Ga3Manifest`], [`variant_for`], [`fitted_transform`], [`palisade_plan`]: pure, tested.
//! - [`swap_kit_models`]: every `Kit_<model>` scene spawned by the site or the siege town gets its
//!   GA3 variant in place (same entity: the camp loot keeps working), three LOD scenes with
//!   visibility ranges (90 / 280 m, then the range of its kind).
//! - [`spawn_palisades`]: the `palisade` obstacles become whole generated segments.
//!
//! Deviation: Godot draws a tile-wise `MultiMesh` per LOD; here every instance is a glTF scene
//! (instancing happens in the renderer). The share draw uses its own hash of the position, so
//! the chosen houses are not the same as in Godot, in the same proportion.

use std::collections::BTreeMap;

use bevy::camera::visibility::VisibilityRange;
use bevy::gltf::GltfAssetLabel;
use bevy::prelude::*;
use bevy::world_serialization::{WorldAsset, WorldAssetRoot};
use sim_battle::{BattleSeason, ObstacleKind};

use super::hash01;
use crate::agent::dump::DumpAppExt;
use crate::app::AppState;
use crate::battle::scene::BattleSession;
use crate::campaign::towns::kit::{Kit, KitEntry};

/// Directory of the generated props, relative to the asset root.
pub const DIR: &str = "models/props_ga/";
/// End of LOD 0 and 1 (m) and the fade margin between levels.
pub const LOD_ENDS: [f32; 2] = [90.0, 280.0];
pub const LOD_MARGIN: f32 = 8.0;
/// Sinking of the generated buildings (m): they have no foundations, the kit goes down to -2 m.
pub const BUILDING_SINK: f32 = 0.35;
/// Largest anisotropy of the footprint fit (the baked texture bears a strong stretch badly).
pub const MAX_STRETCH: f32 = 1.15;

#[derive(Debug, Clone, PartialEq)]
pub struct Ga3Entry {
    pub name: String,
    pub kit_kind: String,
    /// `footprint`, `length` or `real`.
    pub fit: String,
    pub share: f32,
    pub wired: bool,
    pub length: f32,
    pub depth: f32,
}

#[derive(Debug, Clone, Default)]
pub struct Ga3Manifest {
    pub entries: BTreeMap<String, Ga3Entry>,
}

impl Ga3Manifest {
    pub fn from_json(doc: &serde_json::Value) -> Self {
        let mut entries = BTreeMap::new();
        for (name, e) in doc.as_object().into_iter().flatten() {
            let (Some(kit_kind), Some(length), Some(depth)) = (
                e["kit_kind"].as_str(),
                e["length"].as_f64(),
                e["depth"].as_f64(),
            ) else {
                continue;
            };
            entries.insert(
                name.clone(),
                Ga3Entry {
                    name: name.clone(),
                    kit_kind: kit_kind.to_owned(),
                    fit: e["fit"].as_str().unwrap_or("footprint").to_owned(),
                    share: e["share"].as_f64().unwrap_or(0.0) as f32,
                    wired: e["wired"].as_bool().unwrap_or(false),
                    length: length as f32,
                    depth: depth as f32,
                },
            );
        }
        Self { entries }
    }

    pub fn load(asset_root: &std::path::Path) -> Self {
        std::fs::read_to_string(asset_root.join(DIR).join("manifest.json"))
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok())
            .map(|v| Self::from_json(&v))
            .unwrap_or_default()
    }

    /// Wired variants of a kit kind (`cottage`, `church`...), sorted.
    pub fn variants_of(&self, kit_kind: &str) -> Vec<&Ga3Entry> {
        self.entries
            .values()
            .filter(|e| e.wired && e.kit_kind == kit_kind)
            .collect()
    }

    /// Variant replacing the kit model `kit` placed at `origin` (x, z), `None` = the kit stays
    /// (`Ga3Kit.variant_for`).
    pub fn variant_for(&self, kit: &KitEntry, origin: Vec3) -> Option<&Ga3Entry> {
        if kit.ruined {
            return None;
        }
        let candidates = self.variants_of(&kit.kind);
        if candidates.is_empty() {
            return None;
        }
        let x = (origin.x * 7.0) as i32 as f32;
        let z = (origin.z * 7.0) as i32 as f32;
        let pick =
            candidates[(hash01(x, z, 41.0) * candidates.len() as f32) as usize % candidates.len()];
        (hash01(x, z, 83.0) < pick.share).then_some(pick)
    }
}

/// Transform of the variant `ga3` in place of the kit model `kit` laid with `placed`: same
/// orientation; scale by `fit` (`footprint`: the kit footprint with a bounded stretch; `length`:
/// uniform on the length, +-20 %; `real`: generated size) (`Ga3Kit.fitted_transform`).
pub fn fitted_transform(ga3: &Ga3Entry, kit: &KitEntry, placed: &Transform) -> Transform {
    let scale = placed.scale;
    let sunk = placed.translation - Vec3::Y * BUILDING_SINK;
    if ga3.fit == "real" || ((scale.x - 1.0).abs() < 0.01 && (scale.z - 1.0).abs() < 0.01) {
        return Transform::from_translation(placed.translation).with_rotation(placed.rotation);
    }
    if ga3.fit == "length" {
        let s = (scale.x * kit.length / ga3.length).clamp(0.8, 1.2);
        return Transform::from_translation(sunk)
            .with_rotation(placed.rotation)
            .with_scale(Vec3::splat(s));
    }
    let mut sx = scale.x * kit.length / ga3.length;
    let mut sz = scale.z * kit.depth / ga3.depth;
    let mean = (sx * sz).sqrt();
    sx = sx.clamp(mean / MAX_STRETCH, mean * MAX_STRETCH);
    sz = sz.clamp(mean / MAX_STRETCH, mean * MAX_STRETCH);
    let sy = mean.clamp(0.75, 1.3);
    Transform::from_translation(sunk)
        .with_rotation(placed.rotation)
        .with_scale(Vec3::new(sx, sy, sz))
}

/// Farthest distance (m) at which a variant of this kit kind is drawn: the ranges of the batches
/// of `battle_decor.gd` (landmarks, houses, camps, props).
pub fn visibility_end(kit_kind: &str) -> f32 {
    match kit_kind {
        "church" | "windmill" => 2800.0,
        "cottage" => 1900.0,
        "tent" => 1300.0,
        "palisade" => 1200.0,
        _ => 750.0,
    }
}

/// A span `(from, to)` of the field in x, z.
pub type Span = ((f64, f64), (f64, f64));

/// One palisade segment: where it stands and how it is stretched.
#[derive(Debug, Clone, PartialEq)]
pub struct PalisadeSegment {
    pub transform: Transform,
}

/// Whole segments along each obstacle (`_ga3_palisades`): `round(length / segment)` of them
/// (at least one), stretched by `length / (n * segment)` in `0.75..1.25`, centred on their span,
/// standing 8 cm into the ground.
pub fn palisade_plan(
    obstacles: &[Span],
    segment: f32,
    ground: &impl Fn(f64, f64) -> f32,
) -> Vec<PalisadeSegment> {
    let mut out = Vec::new();
    for &(a, b) in obstacles {
        let (dx, dz) = (b.0 - a.0, b.1 - a.1);
        let length = dx.hypot(dz) as f32;
        if length < 0.1 {
            continue;
        }
        let n = ((length / segment).round() as usize).max(1);
        let stretch = (length / (n as f32 * segment)).clamp(0.75, 1.25);
        let yaw = -(dz.atan2(dx) as f32);
        for k in 0..n {
            let t = (k as f64 + 0.5) / n as f64;
            let (px, pz) = (a.0 + dx * t, a.1 + dz * t);
            out.push(PalisadeSegment {
                transform: Transform::from_xyz(px as f32, ground(px, pz) - 0.08, pz as f32)
                    .with_rotation(Quat::from_rotation_y(yaw))
                    .with_scale(Vec3::new(stretch, 1.0, 1.0)),
            });
        }
    }
    out
}

/// Marker of a scene that is one LOD of a variant (its meshes get their visibility range).
#[derive(Component, Clone, Copy)]
pub struct Ga3Lod {
    pub lod: u8,
    pub end: f32,
}

/// The entity already carries its variant.
#[derive(Component)]
pub struct Ga3Swapped;

#[derive(Resource, Default)]
pub struct Ga3Props {
    pub manifest: Ga3Manifest,
    kit: Kit,
    loaded: bool,
}

#[derive(Resource, Default, Debug)]
pub struct PropsStats {
    /// Variants laid, by model (`ga3_house`...).
    pub by_model: BTreeMap<String, usize>,
    /// Kit models looked at / left to the kit.
    pub kit_seen: usize,
    pub palisade_segments: usize,
    pub winter: bool,
    pub lod_meshes: usize,
}

fn visibility_range(lod: u8, end: f32) -> VisibilityRange {
    let (start, stop) = match lod {
        0 => (0.0..0.0, LOD_ENDS[0]..LOD_ENDS[0] + LOD_MARGIN),
        1 => (
            LOD_ENDS[0]..LOD_ENDS[0] + LOD_MARGIN,
            LOD_ENDS[1]..LOD_ENDS[1] + LOD_MARGIN,
        ),
        _ => (LOD_ENDS[1]..LOD_ENDS[1] + LOD_MARGIN, end..end),
    };
    VisibilityRange {
        start_margin: start,
        end_margin: stop,
        use_aabb: false,
    }
}

fn lod_path(model: &str, lod: u8) -> String {
    format!("{DIR}{model}_lod{lod}.glb")
}

fn lod_scene(assets: &AssetServer, model: &str, lod: u8) -> Handle<WorldAsset> {
    assets.load(GltfAssetLabel::Scene(0).from_asset(lod_path(model, lod)))
}

fn ensure_loaded(props: &mut Ga3Props) {
    if props.loaded {
        return;
    }
    props.loaded = true;
    if let Ok(root) = crate::paths::find_root() {
        let assets = crate::paths::asset_root(&root);
        props.manifest = Ga3Manifest::load(&assets);
        props.kit = Kit::load(&assets);
    }
}

fn is_winter(session: &mut BattleSession) -> bool {
    matches!(session.sim_mut().field().season, BattleSeason::Winter)
}

type FreshKit = (Added<WorldAssetRoot>, Without<Ga3Swapped>);

/// Swaps the freshly spawned `Kit_<model>` scenes for their GA3 variant.
fn swap_kit_models(
    mut commands: Commands,
    assets: Res<AssetServer>,
    mut props: ResMut<Ga3Props>,
    mut stats: ResMut<PropsStats>,
    session: Option<ResMut<BattleSession>>,
    mut fresh: Query<(Entity, &Name, &mut Transform, &mut WorldAssetRoot), FreshKit>,
) {
    if fresh.is_empty() || std::env::var_os("CENT_ANS_NO_GA3").is_some() {
        return;
    }
    let Some(mut session) = session else { return };
    ensure_loaded(&mut props);
    stats.winter = is_winter(&mut session);
    if stats.winter || props.manifest.entries.is_empty() {
        return;
    }
    for (entity, name, mut transform, mut root) in &mut fresh {
        let Some(model) = name.as_str().strip_prefix("Kit_") else {
            continue;
        };
        let Some(kit) = props.kit.get(model) else {
            continue;
        };
        stats.kit_seen += 1;
        let Some(ga3) = props.manifest.variant_for(kit, transform.translation) else {
            continue;
        };
        *transform = fitted_transform(ga3, kit, &transform);
        let end = visibility_end(&ga3.kit_kind);
        root.0 = lod_scene(&assets, &ga3.name, 0);
        commands.entity(entity).insert((
            Ga3Swapped,
            Ga3Lod { lod: 0, end },
            Name::new(format!("Ga3_{}", ga3.name)),
        ));
        for lod in 1..=2u8 {
            commands.entity(entity).with_child((
                Name::new(format!("{}_lod{lod}", ga3.name)),
                WorldAssetRoot(lod_scene(&assets, &ga3.name, lod)),
                Transform::default(),
                Visibility::default(),
                Ga3Lod { lod, end },
            ));
        }
        *stats.by_model.entry(ga3.name.clone()).or_default() += 1;
    }
}

/// Gives every mesh of a LOD scene its visibility range (the component does not propagate).
fn range_meshes(
    mut commands: Commands,
    mut stats: ResMut<PropsStats>,
    meshes: Query<Entity, (Added<Mesh3d>, Without<VisibilityRange>)>,
    parents: Query<&ChildOf>,
    lods: Query<&Ga3Lod>,
) {
    for mesh in &meshes {
        let found = std::iter::once(mesh)
            .chain(parents.iter_ancestors(mesh))
            .find_map(|e| lods.get(e).ok());
        if let Some(lod) = found {
            commands
                .entity(mesh)
                .try_insert(visibility_range(lod.lod, lod.end));
            stats.lod_meshes += 1;
        }
    }
}

/// The `palisade` obstacles of the field as generated segments (the volumes of `site.rs` skip
/// them when [`replaces_palisades`]).
fn spawn_palisades(
    mut commands: Commands,
    assets: Res<AssetServer>,
    mut props: ResMut<Ga3Props>,
    mut stats: ResMut<PropsStats>,
    mut session: ResMut<BattleSession>,
) {
    ensure_loaded(&mut props);
    stats.winter = is_winter(&mut session);
    let Some(entry) = props
        .manifest
        .variants_of("palisade")
        .first()
        .copied()
        .cloned()
    else {
        return;
    };
    if stats.winter || std::env::var_os("CENT_ANS_NO_GA3").is_some() {
        return;
    }
    let field = session.sim_mut().field();
    let ground = |x: f64, z: f64| field.height(x, z) as f32;
    let spans: Vec<_> = field
        .obstacles
        .iter()
        .filter(|o| matches!(o.kind, ObstacleKind::Palisade))
        .map(|o| (o.a, o.b))
        .collect();
    let segments = palisade_plan(&spans, entry.length, &ground);
    let end = visibility_end("palisade");
    for segment in &segments {
        let parent = commands
            .spawn((
                Name::new("Ga3Palisade"),
                WorldAssetRoot(lod_scene(&assets, &entry.name, 0)),
                segment.transform,
                Visibility::default(),
                Ga3Lod { lod: 0, end },
                DespawnOnExit(AppState::Battle),
            ))
            .id();
        for lod in 1..=2u8 {
            commands.entity(parent).with_child((
                WorldAssetRoot(lod_scene(&assets, &entry.name, lod)),
                Transform::default(),
                Visibility::default(),
                Ga3Lod { lod, end },
            ));
        }
    }
    stats.palisade_segments = segments.len();
    if !segments.is_empty() {
        *stats.by_model.entry(entry.name).or_default() += segments.len();
    }
}

/// True when the generated palisade replaces the volumes (a variant exists, no snow, not
/// disabled): `site.rs` then leaves the `palisade` obstacles to [`spawn_palisades`].
pub fn replaces_palisades(winter: bool) -> bool {
    if winter || std::env::var_os("CENT_ANS_NO_GA3").is_some() {
        return false;
    }
    let Ok(root) = crate::paths::find_root() else {
        return false;
    };
    !Ga3Manifest::load(&crate::paths::asset_root(&root))
        .variants_of("palisade")
        .is_empty()
}

fn dump_props(world: &World) -> (String, serde_json::Value) {
    let stats = world.get_resource::<PropsStats>();
    let manifest = world.get_resource::<Ga3Props>();
    (
        "battle_props_ga".to_owned(),
        serde_json::json!({
            "manifest": manifest.map(|m| m.manifest.entries.keys().cloned().collect::<Vec<_>>()),
            "wired": manifest.map(|m| m.manifest.entries.values().filter(|e| e.wired).map(|e| e.name.clone()).collect::<Vec<_>>()),
            "placed_by_model": stats.map(|s| &s.by_model),
            "kit_models_seen": stats.map(|s| s.kit_seen),
            "palisade_segments": stats.map(|s| s.palisade_segments),
            "winter": stats.map(|s| s.winter),
            "lod_meshes": stats.map(|s| s.lod_meshes),
        }),
    )
}

pub struct PropsPlugin;

impl Plugin for PropsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Ga3Props>()
            .init_resource::<PropsStats>()
            .register_dump(dump_props)
            .add_systems(
                Update,
                (
                    spawn_palisades.run_if(resource_added::<BattleSession>),
                    (swap_kit_models, range_meshes)
                        .chain()
                        .run_if(in_state(AppState::Battle)),
                ),
            )
            .add_systems(OnExit(AppState::Battle), |mut stats: ResMut<PropsStats>| {
                *stats = PropsStats::default();
            });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest() -> Ga3Manifest {
        Ga3Manifest::from_json(&serde_json::json!({
            "ga3_house": {"kit_kind": "cottage", "fit": "footprint", "share": 0.5, "wired": true, "length": 8.0, "depth": 5.6},
            "ga3_church": {"kit_kind": "church", "fit": "length", "share": 1.0, "wired": true, "length": 22.0, "depth": 15.0},
            "ga3_ram": {"kit_kind": "ram", "fit": "real", "share": 0.0, "wired": false, "length": 7.0, "depth": 3.6},
            "ga3_palisade": {"kit_kind": "palisade", "fit": "real", "share": 1.0, "wired": true, "length": 3.0, "depth": 0.7},
        }))
    }

    fn kit_entry(kind: &str, ruined: bool) -> KitEntry {
        KitEntry {
            name: "x".into(),
            kind: kind.into(),
            ruined,
            framed: false,
            southern: false,
            length: 10.0,
            depth: 6.0,
            height: 6.0,
        }
    }

    #[test]
    fn only_wired_variants_of_the_kind_are_candidates() {
        let m = manifest();
        assert_eq!(m.variants_of("cottage").len(), 1);
        assert!(m.variants_of("ram").is_empty());
    }

    #[test]
    fn a_ruin_or_an_unknown_kind_keeps_the_kit() {
        let m = manifest();
        assert!(
            m.variant_for(&kit_entry("church", true), Vec3::ZERO)
                .is_none()
        );
        assert!(
            m.variant_for(&kit_entry("barn", false), Vec3::ZERO)
                .is_none()
        );
    }

    #[test]
    fn the_share_is_respected_over_many_positions() {
        let m = manifest();
        let kit = kit_entry("cottage", false);
        let n = 4000;
        let swapped = (0..n)
            .filter(|i| {
                m.variant_for(
                    &kit,
                    Vec3::new(*i as f32 * 3.7, 0.0, (*i as f32 * 1.3) % 900.0),
                )
                .is_some()
            })
            .count() as f32
            / n as f32;
        assert!((swapped - 0.5).abs() < 0.08, "{swapped}");
        let church = kit_entry("church", false);
        assert!(m.variant_for(&church, Vec3::new(5.0, 0.0, 9.0)).is_some());
    }

    #[test]
    fn footprint_fit_is_bounded_and_length_fit_uniform() {
        let m = manifest();
        let kit = kit_entry("cottage", false);
        let placed = Transform::from_xyz(1.0, 2.0, 3.0).with_scale(Vec3::new(1.5, 1.0, 0.8));
        let t = fitted_transform(&m.entries["ga3_house"], &kit, &placed);
        let mean = (t.scale.x * t.scale.z).sqrt();
        assert!(t.scale.x / mean <= MAX_STRETCH + 1e-4);
        assert!((t.translation.y - (2.0 - BUILDING_SINK)).abs() < 1e-6);
        let church = fitted_transform(&m.entries["ga3_church"], &kit, &placed);
        assert_eq!(church.scale.x, church.scale.z);
        assert!((0.8..=1.2).contains(&church.scale.x));
        // Real size, or unscaled by the kit: the position stays, no sinking.
        let real = fitted_transform(&m.entries["ga3_palisade"], &kit, &placed);
        assert_eq!(real.scale, Vec3::ONE);
        assert_eq!(real.translation, placed.translation);
    }

    #[test]
    fn palisade_runs_get_whole_stretched_segments() {
        let plan = palisade_plan(&[((0.0, 0.0), (10.0, 0.0))], 3.0, &|_, _| 5.0);
        assert_eq!(plan.len(), 3);
        let stretch = plan[0].transform.scale.x;
        assert!((stretch - 10.0 / 9.0).abs() < 1e-5);
        assert!((plan[0].transform.translation.x - 10.0 / 6.0).abs() < 1e-4);
        assert!((plan[0].transform.translation.y - 4.92).abs() < 1e-5);
        // A tiny run still gets one segment; a degenerate one none.
        assert_eq!(
            palisade_plan(&[((0.0, 0.0), (1.0, 0.0))], 3.0, &|_, _| 0.0).len(),
            1
        );
        assert!(palisade_plan(&[((0.0, 0.0), (0.0, 0.0))], 3.0, &|_, _| 0.0).is_empty());
    }

    #[test]
    fn lod_ranges_chain_without_gaps() {
        let a = visibility_range(0, 1900.0);
        let b = visibility_range(1, 1900.0);
        let c = visibility_range(2, 1900.0);
        assert_eq!(a.end_margin, b.start_margin);
        assert_eq!(b.end_margin, c.start_margin);
        assert_eq!(c.end_margin, 1900.0..1900.0);
    }

    #[test]
    fn the_real_manifest_wires_nine_models_and_two_engines_stay_off() {
        let Ok(root) = crate::paths::find_root() else {
            return;
        };
        let m = Ga3Manifest::load(&crate::paths::asset_root(&root));
        if m.entries.is_empty() {
            return;
        }
        assert_eq!(m.entries.len(), 9);
        assert_eq!(m.entries.values().filter(|e| e.wired).count(), 7);
        assert!(!m.entries["ga3_ram"].wired && !m.entries["ga3_trebuchet"].wired);
    }
}
