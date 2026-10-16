//! Heraldic sails of the cogs and ships at sea (lot armies6). Port of
//! `ArmyFigures._dress_sail` (army_figures.gd) and of `campaign_sail.gdshader`: the "Sail"
//! material of the fleet models is replaced by a cloth in the faction colour, sewn strips, the
//! arms of the faction (`heraldry/<faction>.png`) in the middle and a belly swollen by the wind
//! (vertex stage). One material per (faction, colour), shared by every ship of the faction.
//! Shader: `shaders/sail.wesl` (ExtendedMaterial over StandardMaterial, lit by Bevy's PBR).

use std::collections::HashMap;
use std::path::PathBuf;

use bevy::asset::embedded_asset;
use bevy::gltf::GltfMaterialName;
use bevy::pbr::{ExtendedMaterial, MaterialExtension};
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, ShaderType};
use bevy::shader::ShaderRef;
use serde_json::{Value, json};

use crate::agent::dump::DumpAppExt;
use crate::app::AppState;

/// Name of the sail material in the fleet models (`resource_name == "Sail"`).
pub const SAIL_MATERIAL: &str = "Sail";
/// Hollow of the sail in model units and its breathing (`belly`, `breathe` of the Godot shader).
pub const BELLY: f32 = 0.09;
pub const BREATHE: f32 = 0.2;

/// A fleet figure (the cog) whose sail takes the colours of `faction`.
#[derive(Component, Debug, Clone)]
pub struct SailArms {
    pub faction: String,
    pub color: Color,
}

/// Sail mesh already dressed.
#[derive(Component)]
pub struct SailDressed;

#[derive(Clone, Copy, Debug, PartialEq, ShaderType)]
pub struct SailParams {
    pub faction_color: Vec4,
    /// x belly, y breathe, z has_heraldry.
    pub shape: Vec4,
    /// Local axis of the swelling (the sail lies in the YZ plane, the wind pushes towards +X).
    pub wind_axis: Vec4,
}

#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
pub struct SailExt {
    #[uniform(100)]
    pub params: SailParams,
    #[texture(101)]
    #[sampler(102)]
    pub heraldry: Option<Handle<Image>>,
}

impl MaterialExtension for SailExt {
    fn vertex_shader() -> ShaderRef {
        "embedded://cent_ans_client/campaign/armies/shaders/sail.wesl".into()
    }
    fn fragment_shader() -> ShaderRef {
        "embedded://cent_ans_client/campaign/armies/shaders/sail.wesl".into()
    }
    fn enable_prepass() -> bool {
        false
    }
    fn enable_shadows() -> bool {
        false
    }
}

pub type SailMaterial = ExtendedMaterial<StandardMaterial, SailExt>;

impl SailExt {
    pub fn new(color: Color, heraldry: Option<Handle<Image>>) -> Self {
        let linear = color.to_linear();
        Self {
            params: SailParams {
                faction_color: Vec4::new(linear.red, linear.green, linear.blue, 1.0),
                shape: Vec4::new(
                    BELLY,
                    BREATHE,
                    if heraldry.is_some() { 1.0 } else { 0.0 },
                    0.0,
                ),
                wind_axis: Vec4::new(1.0, 0.0, 0.0, 0.0),
            },
            heraldry,
        }
    }
}

/// Sail materials by faction and colour, plus the counters of the dump.
#[derive(Resource, Default)]
pub struct SailMaterials {
    by_key: HashMap<(String, [u8; 3]), Handle<SailMaterial>>,
    /// Sails dressed since the campaign opened.
    pub dressed: usize,
    /// Materials with the arms of the faction.
    pub with_arms: usize,
    asset_root: Option<PathBuf>,
}

impl SailMaterials {
    fn has_arms(&self, faction: &str) -> bool {
        !faction.is_empty()
            && self
                .asset_root
                .as_ref()
                .is_some_and(|root| root.join(format!("heraldry/{faction}.png")).is_file())
    }
}

fn load_root(mut sails: ResMut<SailMaterials>) {
    sails.asset_root = crate::paths::find_root()
        .ok()
        .map(|root| crate::paths::asset_root(&root));
}

fn reset(mut sails: ResMut<SailMaterials>) {
    sails.by_key.clear();
    sails.dressed = 0;
    sails.with_arms = 0;
}

fn rgb_key(color: Color) -> [u8; 3] {
    let c = color.to_srgba();
    [
        (c.red * 255.0) as u8,
        (c.green * 255.0) as u8,
        (c.blue * 255.0) as u8,
    ]
}

/// Replaces the "Sail" material of every new fleet mesh by the heraldic sail of its army.
#[allow(clippy::type_complexity)]
pub fn dress_sails(
    mut commands: Commands,
    new: Query<
        (Entity, &GltfMaterialName),
        (Without<SailDressed>, With<MeshMaterial3d<StandardMaterial>>),
    >,
    parents: Query<&ChildOf>,
    arms: Query<&SailArms>,
    assets: Res<AssetServer>,
    mut materials: ResMut<Assets<SailMaterial>>,
    mut sails: ResMut<SailMaterials>,
) {
    for (entity, name) in &new {
        if name.0 != SAIL_MATERIAL {
            continue;
        }
        let mut cursor = entity;
        let mut found = None;
        for _ in 0..12 {
            if let Ok(arms) = arms.get(cursor) {
                found = Some(arms.clone());
                break;
            }
            match parents.get(cursor) {
                Ok(parent) => cursor = parent.parent(),
                Err(_) => break,
            }
        }
        let Some(arms) = found else {
            continue;
        };
        let key = (arms.faction.clone(), rgb_key(arms.color));
        let handle = if let Some(handle) = sails.by_key.get(&key) {
            handle.clone()
        } else {
            let heraldry = sails
                .has_arms(&arms.faction)
                .then(|| assets.load::<Image>(format!("heraldry/{}.png", arms.faction)));
            if heraldry.is_some() {
                sails.with_arms += 1;
            }
            let handle = materials.add(SailMaterial {
                base: StandardMaterial {
                    base_color: Color::WHITE,
                    perceptual_roughness: 0.92,
                    reflectance: 0.1,
                    cull_mode: None,
                    double_sided: true,
                    ..default()
                },
                extension: SailExt::new(arms.color, heraldry),
            });
            sails.by_key.insert(key, handle.clone());
            handle
        };
        sails.dressed += 1;
        commands
            .entity(entity)
            .try_remove::<MeshMaterial3d<StandardMaterial>>()
            .try_insert((MeshMaterial3d(handle), SailDressed));
    }
}

fn dump_sails(world: &World) -> (String, Value) {
    let sails = world.resource::<SailMaterials>();
    (
        "campaign_sails".into(),
        json!({
            "materials": sails.by_key.len(),
            "with_arms": sails.with_arms,
            "dressed": sails.dressed,
            "belly": BELLY,
            "breathe": BREATHE,
        }),
    )
}

pub struct SailPlugin;

impl Plugin for SailPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SailMaterials>()
            .register_dump(dump_sails)
            .add_systems(OnEnter(AppState::Campaign), load_root)
            .add_systems(OnExit(AppState::Campaign), reset);
        if app.world().contains_resource::<Assets<Mesh>>() {
            embedded_asset!(app, "shaders/sail.wesl");
            app.add_plugins(MaterialPlugin::<SailMaterial>::default())
                .add_systems(
                    Update,
                    dress_sails
                        .after(super::figures::tint_liveries)
                        .run_if(in_state(AppState::Campaign)),
                );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn params_carry_colour_and_arms_flag() {
        let ext = SailExt::new(Color::srgb(1.0, 0.0, 0.0), None);
        assert_eq!(ext.params.shape.z, 0.0);
        assert_eq!(ext.params.shape.x, BELLY);
        assert!((ext.params.faction_color.x - 1.0).abs() < 1e-5);
        let keyed = rgb_key(Color::srgb(1.0, 0.5, 0.0));
        assert_eq!(keyed[0], 255);
    }

    #[test]
    fn arms_need_a_file() {
        let mut sails = SailMaterials::default();
        assert!(!sails.has_arms("fac_france"));
        let dir = std::env::temp_dir().join("cent_ans_sail_test_assets");
        std::fs::create_dir_all(dir.join("heraldry")).unwrap();
        std::fs::write(dir.join("heraldry/fac_x.png"), b"x").unwrap();
        sails.asset_root = Some(dir);
        assert!(sails.has_arms("fac_x"));
        assert!(!sails.has_arms("fac_y"));
        assert!(!sails.has_arms(""));
    }
}
