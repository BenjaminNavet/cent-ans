//! Generated semi-realistic glb of the campaign places (lot DN camp-bati, ADR 0211): port of
//! `game/scripts/map/dn_campaign_models.gd` and of the DN part of `town_maquette_layer.gd`
//! (`_dn_for`, `_dn_scale_of`, `_add_dn_tiles`, `_update_dn_view`).
//!
//! Table `data/art/dn_campaign_models.json`: place kind x architecture family (or sub-family,
//! DN-MAQ) -> list of models `{ "path": "dn/buildings/<name>" }`, stored under the asset root as
//! `models/<path>_lod<n>.glb`, base at y = 0, one textured surface (baked albedo). In Godot these
//! models replace the stylised maquettes below `near_distance` (default 1300, beyond the farthest
//! maquette range): in practice every place shown carries its generated glb, the kit maquette is
//! only the fallback when no entry or file exists. A procedural banner in the controller's colour
//! stands on top (`banner_pole_ratio` of the model height).

use std::collections::HashMap;
use std::path::Path;

use serde_json::Value;

/// Table key of the harbour places (`port: true` settlements), tried before the families.
pub const PORT_KEY: &str = "port";
/// Fallback family key of the table.
pub const WILDCARD: &str = "*";

/// One model entry of the table.
#[derive(Debug, Clone, PartialEq)]
pub struct DnEntry {
    pub path: String,
    pub lod: Option<u32>,
    pub albedo_gain: Option<f32>,
    pub scale: f32,
    pub native_width: Option<f32>,
    pub banner: bool,
    /// Families allowed for a `port` entry (empty = all).
    pub families: Vec<String>,
}

impl DnEntry {
    fn parse(v: &Value) -> Option<Self> {
        let f = |k: &str| v.get(k).and_then(Value::as_f64).map(|x| x as f32);
        Some(Self {
            path: v.get("path")?.as_str()?.to_string(),
            lod: v.get("lod").and_then(Value::as_u64).map(|x| x as u32),
            albedo_gain: f("albedo_gain"),
            scale: f("scale").unwrap_or(1.0),
            native_width: f("native_width"),
            banner: v.get("banner").and_then(Value::as_bool).unwrap_or(true),
            families: strings(v.get("families")),
        })
    }
}

/// Sub-family rule (`subfamilies`, order = priority).
#[derive(Debug, Clone, Default)]
pub struct SubfamilyRule {
    pub id: String,
    pub family: String,
    pub also: Vec<String>,
    pub cultures: Vec<String>,
    pub regions: Vec<String>,
    pub religions: Vec<String>,
}

/// Parsed `dn_campaign_models.json`.
#[derive(Debug, Clone, Default)]
pub struct DnModels {
    /// kind -> family key -> entries.
    pub table: HashMap<String, HashMap<String, Vec<DnEntry>>>,
    pub subfamilies: Vec<SubfamilyRule>,
    pub near_distance: f32,
    pub fade_margin: f32,
    pub banner_pole_ratio: f32,
    pub lod: u32,
    pub albedo_gain: f32,
}

fn strings(v: Option<&Value>) -> Vec<String> {
    v.and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(|s| s.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

impl DnModels {
    /// Reads `<data>/art/dn_campaign_models.json`; empty table when missing (no replacement).
    pub fn load(data_dir: &Path) -> Self {
        std::fs::read(data_dir.join("art/dn_campaign_models.json"))
            .ok()
            .and_then(|b| serde_json::from_slice::<Value>(&b).ok())
            .map(|v| Self::from_value(&v))
            .unwrap_or_else(|| Self::from_value(&Value::Null))
    }

    pub fn from_value(doc: &Value) -> Self {
        let d = &doc["defaults"];
        let num = |k: &str, fallback: f32| d[k].as_f64().map_or(fallback, |x| x as f32);
        let mut table: HashMap<String, HashMap<String, Vec<DnEntry>>> = HashMap::new();
        if let Some(kinds) = doc["table"].as_object() {
            for (kind, families) in kinds {
                let Some(families) = families.as_object() else {
                    continue;
                };
                let by_family = table.entry(kind.clone()).or_default();
                for (family, list) in families {
                    let entries: Vec<DnEntry> = list
                        .as_array()
                        .map(|a| a.iter().filter_map(DnEntry::parse).collect())
                        .unwrap_or_default();
                    by_family.insert(family.clone(), entries);
                }
            }
        }
        let subfamilies = doc["subfamilies"]
            .as_array()
            .map(|rules| {
                rules
                    .iter()
                    .map(|r| SubfamilyRule {
                        id: r["id"].as_str().unwrap_or_default().to_string(),
                        family: r["family"].as_str().unwrap_or_default().to_string(),
                        also: strings(r.get("also")),
                        cultures: strings(r.get("cultures")),
                        regions: strings(r.get("regions")),
                        religions: strings(r.get("religions")),
                    })
                    .collect()
            })
            .unwrap_or_default();
        Self {
            table,
            subfamilies,
            near_distance: num("near_distance", 400.0),
            fade_margin: num("fade_margin", 60.0).max(0.001),
            banner_pole_ratio: num("banner_pole_ratio", 0.12),
            lod: num("lod", 1.0) as u32,
            albedo_gain: num("albedo_gain", 1.0),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.table.is_empty()
    }

    /// Sub-family of a province of `family`: first rule whose culture, region or religion
    /// matches; empty otherwise (`subfamily_for`).
    pub fn subfamily_for(&self, family: &str, culture: &str, region: &str, religion: &str) -> &str {
        let has = |list: &[String], v: &str| list.iter().any(|x| x == v);
        self.subfamilies
            .iter()
            .find(|r| {
                r.family == family
                    && (has(&r.cultures, culture)
                        || has(&r.regions, region)
                        || has(&r.religions, religion))
            })
            .map_or("", |r| r.id.as_str())
    }

    /// Lookup keys by priority: sub-family, its `also`, family, `*` (`lookup_keys`).
    pub fn lookup_keys(&self, family: &str, subfamily: &str) -> Vec<String> {
        let mut keys = Vec::new();
        if !subfamily.is_empty() {
            keys.push(subfamily.to_string());
            if let Some(rule) = self.subfamilies.iter().find(|r| r.id == subfamily) {
                keys.extend(rule.also.iter().cloned());
            }
        }
        keys.push(family.to_string());
        keys.push(WILDCARD.to_string());
        keys
    }

    /// Entries of a kind for a family / sub-family; harbour entries first for a port
    /// (`entries_for`).
    pub fn entries_for(
        &self,
        kind: &str,
        family: &str,
        subfamily: &str,
        port: bool,
    ) -> Vec<&DnEntry> {
        let Some(by_family) = self.table.get(kind) else {
            return Vec::new();
        };
        if port && let Some(harbour) = by_family.get(PORT_KEY) {
            let allowed: Vec<&DnEntry> = harbour
                .iter()
                .filter(|e| e.families.is_empty() || e.families.iter().any(|f| f == family))
                .collect();
            if !allowed.is_empty() {
                return allowed;
            }
        }
        for key in self.lookup_keys(family, subfamily) {
            if let Some(list) = by_family.get(&key)
                && !list.is_empty()
            {
                return list.iter().collect();
            }
        }
        Vec::new()
    }

    /// Entry kept for a place: variant `variant_index` modulo the list size (`entry_for`).
    pub fn entry_for(
        &self,
        kind: &str,
        family: &str,
        variant_index: u64,
        subfamily: &str,
        port: bool,
    ) -> Option<&DnEntry> {
        let list = self.entries_for(kind, family, subfamily, port);
        if list.is_empty() {
            return None;
        }
        Some(list[(variant_index % list.len() as u64) as usize])
    }

    /// Asset path (under the asset root) of an entry: `models/<path>_lod<n>.glb`, or the path as
    /// given when it already names a LOD (`model_name`).
    pub fn model_path(&self, entry: &DnEntry) -> String {
        if entry.path.contains("_lod") {
            format!("models/{}.glb", entry.path)
        } else {
            format!(
                "models/{}_lod{}.glb",
                entry.path,
                entry.lod.unwrap_or(self.lod)
            )
        }
    }

    /// Linear albedo gain of an entry (entry value, else the defaults).
    pub fn gain(&self, entry: &DnEntry) -> f32 {
        entry.albedo_gain.unwrap_or(self.albedo_gain)
    }

    /// Opacity of the generated glb at rig distance `distance` (1 under `near_distance`, fading
    /// out over `fade_margin` above it; `_update_dn_view`).
    pub fn alpha_at(&self, distance: f32) -> f32 {
        ((self.near_distance + self.fade_margin - distance) / self.fade_margin).clamp(0.0, 1.0)
    }
}

/// Bounds of the first mesh of a glb (accessor `min`/`max` of its POSITION, node transforms
/// ignored: the generated models have a single identity node). Returns (min, max).
pub fn glb_position_bounds(bytes: &[u8]) -> Option<([f32; 3], [f32; 3])> {
    if bytes.len() < 20 || &bytes[0..4] != b"glTF" {
        return None;
    }
    let len = u32::from_le_bytes(bytes[12..16].try_into().ok()?) as usize;
    let json: Value = serde_json::from_slice(bytes.get(20..20 + len)?).ok()?;
    let mut lo = [f32::MAX; 3];
    let mut hi = [f32::MIN; 3];
    for prim in json["meshes"][0]["primitives"].as_array()? {
        let acc = &json["accessors"][prim["attributes"]["POSITION"].as_u64()? as usize];
        for k in 0..3 {
            lo[k] = lo[k].min(acc["min"][k].as_f64()? as f32);
            hi[k] = hi[k].max(acc["max"][k].as_f64()? as f32);
        }
    }
    (lo[0] <= hi[0]).then_some((lo, hi))
}

/// Native width (largest horizontal side) and top of a glb.
pub fn native_size(bytes: &[u8]) -> Option<(f32, f32)> {
    let (lo, hi) = glb_position_bounds(bytes)?;
    Some((
        (hi[0] - lo[0]).max(hi[2] - lo[2]).max(0.001),
        hi[1].max(0.0),
    ))
}

/// Variant index of a place id: Godot's `String.hash()` (djb2 over the Unicode code points,
/// 32 bits), so the same place gets the same generated model as in Godot (`_dn_for`).
pub fn variant_index(id: &str) -> u64 {
    u64::from(godot_string_hash(id))
}

/// Godot 4 `String::hash` (see `placement::stable_hash`).
pub fn godot_string_hash(text: &str) -> u32 {
    super::placement::stable_hash(text)
}

/// Gives each place with a kit maquette its generated glb when the table has an entry and the
/// file exists (`_dn_for`, `_dn_scale_of`): same world width as the maquette (weight gain and
/// collision reduction included) times the entry's `scale`. Returns the number of places served.
pub fn assign(
    places: &mut super::places::Places,
    data: &data_model::GameData,
    dn: &DnModels,
    assets_dir: &Path,
) -> usize {
    if dn.is_empty() {
        return 0;
    }
    let mut sizes: HashMap<String, Option<(f32, f32)>> = HashMap::new();
    let maquettes = places.maquettes.clone();
    let mut served = 0;
    for p in &mut places.places {
        p.dn = None;
        if p.model.is_none() || p.landmark.is_some() || p.absorbed {
            continue;
        }
        let kit_scale = maquettes.model_scale(p.shown_kind);
        if kit_scale <= 0.0 {
            continue;
        }
        let Ok(sid) = data_model::SettlementId::new(&p.id) else {
            continue;
        };
        let settlement = data.settlements.get(&sid);
        let port = settlement.is_some_and(|s| s.port);
        let province = settlement.and_then(|s| data.provinces.get(&s.province));
        let subfamily = province.map_or("", |pr| {
            dn.subfamily_for(
                &p.family,
                pr.culture.as_str(),
                &pr.region,
                pr.religion.as_str(),
            )
        });
        let Some(entry) = dn.entry_for(
            p.shown_kind,
            &p.family,
            variant_index(&p.id),
            subfamily,
            port,
        ) else {
            continue;
        };
        let model = dn.model_path(entry);
        let size = *sizes.entry(model.clone()).or_insert_with(|| {
            std::fs::read(assets_dir.join(&model))
                .ok()
                .and_then(|b| native_size(&b))
        });
        let Some((measured, top)) = size else {
            continue;
        };
        let width = entry.native_width.unwrap_or(measured).max(0.001);
        let gain_factor = p.model_scale / kit_scale;
        p.dn = Some(super::places::DnPlace {
            scale: maquettes.width(p.shown_kind) * gain_factor * entry.scale / width,
            top,
            gain: dn.gain(entry),
            banner: entry.banner,
            model,
        });
        served += 1;
    }
    served
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn doc() -> DnModels {
        DnModels::from_value(&json!({
            "defaults": {"near_distance": 1300.0, "lod": 0, "albedo_gain": 2.4},
            "table": {
                "town": {
                    "west": [{"path": "dn/buildings/town_west", "albedo_gain": 1.0}],
                    "west_hansa": [{"path": "dn/buildings/town_hansa"}],
                    "port": [{"path": "dn/buildings/town_port_harbour", "families": ["west", "med"]}]
                },
                "city": {"*": [{"path": "dn/buildings/a"}, {"path": "dn/buildings/b_lod2"}]}
            },
            "subfamilies": [
                {"id": "west_hansa", "family": "west", "regions": ["empire_nord"]},
                {"id": "west_carpathian", "family": "west", "also": ["west_hansa"], "regions": ["transylvanie"]}
            ]
        }))
    }

    #[test]
    fn lookup_follows_subfamily_also_family_wildcard() {
        let d = doc();
        assert_eq!(
            d.subfamily_for("west", "cul_x", "empire_nord", ""),
            "west_hansa"
        );
        assert_eq!(d.subfamily_for("med", "cul_x", "empire_nord", ""), "");
        let e = d
            .entry_for("town", "west", 7, "west_carpathian", false)
            .unwrap();
        assert_eq!(e.path, "dn/buildings/town_hansa");
        let e = d.entry_for("town", "west", 0, "", false).unwrap();
        assert_eq!(d.gain(e), 1.0);
        assert_eq!(d.model_path(e), "models/dn/buildings/town_west_lod0.glb");
        assert!(d.entry_for("town", "rus", 0, "", false).is_none());
        let c = d.entry_for("city", "rus", 1, "", false).unwrap();
        assert_eq!(d.model_path(c), "models/dn/buildings/b_lod2.glb");
        assert_eq!(d.gain(c), 2.4);
    }

    #[test]
    fn harbour_only_for_allowed_families() {
        let d = doc();
        let e = d.entry_for("town", "west", 0, "", true).unwrap();
        assert!(e.path.ends_with("town_port_harbour"));
        assert!(d.entry_for("town", "rus", 0, "", true).is_none());
    }

    #[test]
    fn variant_index_matches_godot_string_hash() {
        // Values printed by Godot 4 (`"<id>".hash()`).
        assert_eq!(godot_string_hash(""), 5381);
        assert_eq!(godot_string_hash("a"), 177_670);
        assert_eq!(godot_string_hash("set_paris"), 1_125_892_655);
        assert_eq!(godot_string_hash("set_saint_denis"), 4_130_660_257);
        assert_eq!(godot_string_hash("set_orléans"), 1_402_724_200);
    }

    #[test]
    fn fade_below_near_distance() {
        let d = doc();
        assert_eq!(d.alpha_at(100.0), 1.0);
        assert_eq!(d.alpha_at(1300.0), 1.0);
        assert_eq!(d.alpha_at(1360.0), 0.0);
    }

    #[test]
    fn real_table_and_glbs() {
        let Ok(root) = crate::paths::find_root() else {
            return;
        };
        let d = DnModels::load(&crate::paths::data_dir(&root));
        assert!(!d.is_empty());
        let e = d.entry_for("city", "west", 0, "", false).unwrap();
        let path = crate::paths::asset_root(&root).join(d.model_path(e));
        let bytes = std::fs::read(&path).expect("city_west glb");
        let (w, top) = native_size(&bytes).unwrap();
        assert!((3.0..4.0).contains(&w) && top > 1.0, "{w} {top}");
    }
}
