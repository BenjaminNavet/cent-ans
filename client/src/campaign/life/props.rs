//! Table of the models of the living map (`data/rules/folk_props.json`, port of `FolkModels.PROPS`,
//! `ROLES`, `ACTIVITIES`): glb props with their yaw, draught team and figure slots, the figure
//! candidates and liveries of each role.
//!
//! The Godot code measured the mesh of each glb to compute the slots and the team positions. Here
//! the bounds come from the `POSITION` accessors of the glb (`glb_bounds`, node transforms are
//! identity in the generated files), turned by the yaw and brought to the metres of the table
//! (`length_m`), so nothing is measured on the GPU side.

use std::collections::HashMap;
use std::path::Path;

use bevy::prelude::*;
use serde::Deserialize;
use serde_json::Value;

/// Position of a figure around a prop, in metres: `x` lateral (right of the heading), `y` ahead.
pub type Slot = Vec2;

#[derive(Debug, Clone, Deserialize)]
pub struct TeamSpec {
    pub role: String,
    pub count: usize,
    pub gap_m: f32,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct SlotSpec {
    pub side: i32,
    pub margin: f32,
    #[serde(rename = "abs")]
    pub absolute: bool,
    pub at: f32,
    pub offset: f32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PropSpec {
    #[serde(default)]
    pub fk2: String,
    #[serde(default)]
    pub dn: String,
    #[serde(default)]
    pub yaw_deg: f32,
    pub speed: f32,
    pub length_m: f32,
    #[serde(default)]
    pub team: Option<TeamSpec>,
    #[serde(default)]
    pub slots: HashMap<String, SlotSpec>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RoleSpec {
    pub figures: Vec<(String, u32)>,
    pub livery: [f32; 3],
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct PropTable {
    #[serde(default)]
    pub props: HashMap<String, PropSpec>,
    #[serde(default)]
    pub roles: HashMap<String, RoleSpec>,
    #[serde(default)]
    pub activities: HashMap<String, f32>,
    #[serde(default)]
    pub drab: Vec<[f32; 3]>,
}

/// Source of the model of a prop: the generated DN glb when present, else the FK2 placeholder.
#[derive(Debug, Clone, PartialEq)]
pub struct PropModel {
    pub path: String,
    /// Yaw (deg) that brings the front of the file to +Z.
    pub yaw_deg: f32,
    pub is_dn: bool,
}

impl PropTable {
    pub fn load(data_dir: &Path) -> PropTable {
        let path = data_dir.join("rules/folk_props.json");
        match std::fs::read_to_string(&path)
            .map_err(|e| e.to_string())
            .and_then(|t| serde_json::from_str::<PropTable>(&t).map_err(|e| e.to_string()))
        {
            Ok(t) => t,
            Err(e) => {
                warn!("life: {}: {e}", path.display());
                PropTable::default()
            }
        }
    }

    pub fn is_prop(&self, id: &str) -> bool {
        self.props.contains_key(id)
    }

    /// Speed (m/s) of a prop that rolls, else of an activity.
    pub fn speed_of(&self, role: &str, activity: &str) -> f32 {
        if let Some(p) = self.props.get(role) {
            return p.speed;
        }
        self.activities.get(activity).copied().unwrap_or(0.0)
    }

    /// Model of a prop. `lod` selects the DN file (`<id>_lod<lod>.glb`); `exists` tells whether
    /// an asset path exists (relative to the asset root).
    pub fn model_of(&self, id: &str, lod: u8, exists: &dyn Fn(&str) -> bool) -> Option<PropModel> {
        let spec = self.props.get(id)?;
        if !spec.dn.is_empty() {
            for dir in ["props", "fauna"] {
                let path = format!("models/dn/{dir}/{}_lod{lod}.glb", spec.dn);
                if exists(&path) {
                    return Some(PropModel {
                        path,
                        yaw_deg: spec.yaw_deg,
                        is_dn: true,
                    });
                }
            }
        }
        if !spec.fk2.is_empty() {
            let path = format!("models/folk/{}.glb", spec.fk2);
            if exists(&path) {
                // FK2 files look along +X: a quarter turn brings the front to +Z.
                return Some(PropModel {
                    path,
                    yaw_deg: -90.0,
                    is_dn: false,
                });
            }
        }
        None
    }

    /// Candidate figure `(family, variant)` of a role that `present` accepts.
    pub fn figure_of(
        &self,
        role: &str,
        present: &dyn Fn(&str, u32) -> bool,
    ) -> Option<(String, u32)> {
        let spec = self.roles.get(role).or_else(|| self.roles.get("peasant"))?;
        spec.figures
            .iter()
            .find(|(k, v)| present(k, *v))
            .map(|(k, v)| (k.clone(), *v))
    }

    pub fn livery_of(&self, role: &str) -> [f32; 3] {
        self.roles
            .get(role)
            .or_else(|| self.roles.get("peasant"))
            .map_or([0.42, 0.33, 0.22], |r| r.livery)
    }
}

/// Horizontal bounds of a glb as (min, max), from its `POSITION` accessors.
pub fn glb_bounds(bytes: &[u8]) -> Option<(Vec3, Vec3)> {
    if bytes.len() < 20 || &bytes[..4] != b"glTF" {
        return None;
    }
    let json_len = u32::from_le_bytes(bytes[12..16].try_into().ok()?) as usize;
    if &bytes[16..20] != b"JSON" || bytes.len() < 20 + json_len {
        return None;
    }
    let doc: Value = serde_json::from_slice(&bytes[20..20 + json_len]).ok()?;
    let accessors = doc["accessors"].as_array()?;
    let mut lo = Vec3::splat(f32::MAX);
    let mut hi = Vec3::splat(f32::MIN);
    for mesh in doc["meshes"].as_array()? {
        for primitive in mesh["primitives"].as_array()? {
            let index = primitive["attributes"]["POSITION"].as_u64()? as usize;
            let accessor = accessors.get(index)?;
            let read = |k: &str| -> Option<Vec3> {
                let a = accessor[k].as_array()?;
                Some(Vec3::new(
                    a.first()?.as_f64()? as f32,
                    a.get(1)?.as_f64()? as f32,
                    a.get(2)?.as_f64()? as f32,
                ))
            };
            lo = lo.min(read("min")?);
            hi = hi.max(read("max")?);
        }
    }
    (hi.x >= lo.x).then_some((lo, hi))
}

/// Bounds (min, max) of a prop in metres, in the frame where its front is +Z, scaled so that its
/// longest horizontal side is `length_m`.
pub fn oriented_bounds(bounds: (Vec3, Vec3), yaw_deg: f32, length_m: f32) -> (Vec3, Vec3) {
    let rot = Quat::from_rotation_y(yaw_deg.to_radians());
    let mut lo = Vec3::splat(f32::MAX);
    let mut hi = Vec3::splat(f32::MIN);
    for i in 0..8 {
        let corner = Vec3::new(
            if i & 1 == 0 { bounds.0.x } else { bounds.1.x },
            if i & 2 == 0 { bounds.0.y } else { bounds.1.y },
            if i & 4 == 0 { bounds.0.z } else { bounds.1.z },
        );
        let p = rot * corner;
        lo = lo.min(p);
        hi = hi.max(p);
    }
    let horizontal = (hi.x - lo.x).max(hi.z - lo.z);
    let k = if horizontal > 1e-4 && length_m > 0.0 {
        length_m / horizontal
    } else {
        1.0
    };
    (lo * k, hi * k)
}

/// Slots of a prop from its oriented bounds (`FolkModels._slots_from_bounds`): `x` lateral
/// positive to the right of the heading (-X of the world frame), `y` ahead.
pub fn slots_from_bounds(
    bounds: (Vec3, Vec3),
    specs: &HashMap<String, SlotSpec>,
) -> HashMap<String, Slot> {
    let size = bounds.1 - bounds.0;
    let half_width = size.x * 0.5;
    specs
        .iter()
        .map(|(name, s)| {
            let lateral = s.side as f32
                * if s.absolute {
                    s.margin
                } else {
                    half_width + s.margin
                };
            let ahead = bounds.0.z + s.at * size.z + s.offset;
            (name.clone(), Vec2::new(lateral, ahead))
        })
        .collect()
}

/// A draught animal of a team: position in the frame of the prop (metres).
#[derive(Debug, Clone, PartialEq)]
pub struct TeamBeast {
    pub role: String,
    pub lateral: f32,
    pub ahead: f32,
}

/// Beasts hitched ahead of a prop (`FolkModels.team`): `cart` and `animal` are oriented bounds.
pub fn team_beasts(spec: &TeamSpec, cart: (Vec3, Vec3), animal: (Vec3, Vec3)) -> Vec<TeamBeast> {
    let ahead = cart.1.z + spec.gap_m - animal.0.z;
    let pitch = (animal.1.x - animal.0.x) + 0.15;
    (0..spec.count)
        .map(|i| TeamBeast {
            role: spec.role.clone(),
            lateral: (i as f32 - (spec.count as f32 - 1.0) * 0.5) * pitch,
            ahead,
        })
        .collect()
}

/// Derived geometry of the props (slots, teams), computed once from the glb bounds.
#[derive(Resource, Default, Debug, Clone)]
pub struct PropGeometry {
    pub slots: HashMap<String, HashMap<String, Slot>>,
    pub teams: HashMap<String, Vec<TeamBeast>>,
    /// Oriented bounds (metres, front +Z) of the props with a measurable glb.
    pub bounds: HashMap<String, (Vec3, Vec3)>,
}

impl PropGeometry {
    pub fn slot(&self, prop: &str, name: &str, fallback: Slot) -> Slot {
        self.slots
            .get(prop)
            .and_then(|s| s.get(name))
            .copied()
            .unwrap_or(fallback)
    }

    pub fn build(table: &PropTable, asset_root: &Path) -> PropGeometry {
        let exists = |p: &str| asset_root.join(p).is_file();
        let mut geometry = PropGeometry::default();
        for (id, spec) in &table.props {
            let Some(model) = table.model_of(id, 2, &exists) else {
                continue;
            };
            let Some(raw) = std::fs::read(asset_root.join(&model.path))
                .ok()
                .and_then(|b| glb_bounds(&b))
            else {
                continue;
            };
            let yaw = if model.is_dn {
                spec.yaw_deg
            } else {
                model.yaw_deg
            };
            geometry
                .bounds
                .insert(id.clone(), oriented_bounds(raw, yaw, spec.length_m));
        }
        for (id, spec) in &table.props {
            let Some(cart) = geometry.bounds.get(id).copied() else {
                continue;
            };
            if !spec.slots.is_empty() {
                geometry
                    .slots
                    .insert(id.clone(), slots_from_bounds(cart, &spec.slots));
            }
            if let (Some(team), Some(model)) = (&spec.team, table.model_of(id, 2, &exists))
                && model.is_dn
                && let Some(animal) = geometry.bounds.get(&team.role).copied()
            {
                geometry
                    .teams
                    .insert(id.clone(), team_beasts(team, cart, animal));
            }
        }
        geometry
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table() -> PropTable {
        let root = crate::paths::find_root().expect("repository root");
        PropTable::load(&crate::paths::data_dir(&root))
    }

    #[test]
    fn table_loads_and_covers_carts_and_roles() {
        let t = table();
        assert!(t.props.len() >= 14);
        assert!(t.is_prop("merchant_cart") && t.is_prop("flood_water"));
        assert_eq!(t.props["merchant_cart"].team.as_ref().unwrap().count, 2);
        assert!(t.roles.contains_key("monk") && t.roles.contains_key("rioter"));
        assert_eq!(t.speed_of("dead_cart", ""), 0.6);
        assert_eq!(t.speed_of("peasant", "plough"), 0.8);
        assert_eq!(t.drab.len(), 4);
    }

    #[test]
    fn model_prefers_dn_then_fk2() {
        let t = table();
        let all = |_: &str| true;
        let m = t.model_of("merchant_cart", 2, &all).unwrap();
        assert!(m.is_dn && m.path.ends_with("cart_merchant_lod2.glb"));
        let fk2_only = |p: &str| p.starts_with("models/folk/");
        let m = t.model_of("merchant_cart", 2, &fk2_only).unwrap();
        assert!(!m.is_dn && m.yaw_deg == -90.0);
        assert!(t.model_of("flood_water", 2, &all).is_none());
    }

    #[test]
    fn real_glb_bounds_give_slots_and_a_team() {
        let root = crate::paths::find_root().expect("repository root");
        let assets = crate::paths::asset_root(&root);
        let read = |p: &str| std::fs::read(assets.join(p)).ok();
        let t = table();
        let Some(cart_bytes) = read("models/dn/props/cart_merchant_lod2.glb") else {
            return; // model pack absent
        };
        let spec = &t.props["merchant_cart"];
        let cart = oriented_bounds(
            glb_bounds(&cart_bytes).unwrap(),
            spec.yaw_deg,
            spec.length_m,
        );
        // After the yaw the long side lies along +Z.
        assert!((cart.1.z - cart.0.z) > (cart.1.x - cart.0.x));
        let slots = slots_from_bounds(cart, &spec.slots);
        assert!(slots["carter"].x > 0.0, "carter sits on the right");
        assert!(slots["guard"].y < slots["carter"].y, "guard at the tail");
        let horse = &t.props["horse"];
        let animal = oriented_bounds(
            glb_bounds(&read("models/dn/fauna/animal_horse_draught_lod2.glb").unwrap()).unwrap(),
            horse.yaw_deg,
            horse.length_m,
        );
        let team = team_beasts(spec.team.as_ref().unwrap(), cart, animal);
        assert_eq!(team.len(), 2);
        assert!(team[0].lateral < 0.0 && team[1].lateral > 0.0);
        assert!(team[0].ahead > cart.1.z - 1.0);
    }

    #[test]
    fn figure_candidates_fall_back_in_order() {
        let t = table();
        let only_infantry_2 = |k: &str, v: u32| k == "infantry" && v == 2;
        assert_eq!(
            t.figure_of("peasant", &only_infantry_2),
            Some(("infantry".into(), 2))
        );
        assert_eq!(t.figure_of("guard", &|_, _| false), None);
    }
}
