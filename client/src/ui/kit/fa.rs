//! Real ornaments of the interface (`FaUi`, game/scripts/ui/fa_ui.gd): wax seals and leather and
//! brass plates cut from public-domain works (`game/assets/ui/fa/`, sources in `SOURCE.md`).
//!
//! The catalogue `data/ui/fa_ui_assets.json` says which seal serves which solemn moment
//! (`display.seals`: `treaty`, `chronicle`) and the rim width of each plate (`materials[].rim`).
//! - [`FaUi::seal`]: an image node of the seal at its own proportions, for the treaty history, the
//!   chronicle decisions and the « Proposer le traité » button.
//! - [`FaPlate`]: put on a kit button, replaces its parchment look by the plate (9-slice, rim
//!   3 px, tiled centre) and lights it under the pointer (`FrontEndStyle.PLATE_HOVER`), like
//!   `FrontEndStyle.style_action_button`.
//!
//! The tables are read once from the files ([`FaUi::global`]); a missing file gives no seal or
//! plate and the caller keeps its look.

use std::collections::BTreeMap;
use std::sync::OnceLock;

use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::ui::widget::NodeImageMode;
use serde_json::{Value, json};

use super::widgets::KitButton;

const CATALOG: &str = "ui/fa_ui_assets.json";
const ASSET_DIR: &str = "ui/fa";
/// `FrontEndStyle.PLATE_HOVER`: a plate under the pointer, pressed or focused is lit.
const PLATE_HOVER: Color = Color::srgb(1.3, 1.26, 1.2);

/// Seal or plate file: path relative to `game/assets` and pixel size.
#[derive(Debug, Clone, PartialEq)]
pub struct FaImage {
    pub path: String,
    pub size: (u32, u32),
    /// Plates: width of the metal rim (the 9-slice border).
    pub rim: f32,
}

/// Seals by role and plates by id.
#[derive(Debug, Default, Clone)]
pub struct FaUi {
    pub seals: BTreeMap<String, FaImage>,
    pub plates: BTreeMap<String, FaImage>,
}

/// Width and height of a PNG from its header (`IHDR`).
pub fn png_size(bytes: &[u8]) -> Option<(u32, u32)> {
    if bytes.len() < 24 || &bytes[1..4] != b"PNG" {
        return None;
    }
    let read =
        |at: usize| u32::from_be_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]]);
    Some((read(16), read(20)))
}

impl FaUi {
    /// The catalogue and files of the repository, read once (empty when the repository root or
    /// the catalogue is missing).
    pub fn global() -> &'static FaUi {
        static FA: OnceLock<FaUi> = OnceLock::new();
        FA.get_or_init(|| {
            let read = |base: Option<std::path::PathBuf>, relative: &str| -> Option<Vec<u8>> {
                std::fs::read(base?.join(relative)).ok()
            };
            let root = crate::paths::find_root().ok();
            let data = root.as_deref().map(crate::paths::data_dir);
            let assets = root.as_deref().map(crate::paths::asset_root);
            FaUi::from_files(|p| read(data.clone(), p), |p| read(assets.clone(), p))
        })
    }

    /// Builds the tables from the catalogue and the files (`read`: path under `game/assets` or
    /// `data/` for the catalogue).
    pub fn from_files(
        read_data: impl Fn(&str) -> Option<Vec<u8>>,
        read_asset: impl Fn(&str) -> Option<Vec<u8>>,
    ) -> Self {
        let mut fa = Self::default();
        let Some(Value::Object(root)) =
            read_data(CATALOG).and_then(|b| serde_json::from_slice(&b).ok())
        else {
            return fa;
        };
        let image = |kind: &str, id: &str, rim: f32| -> Option<FaImage> {
            let path = format!("{ASSET_DIR}/{kind}/{id}.png");
            let size = png_size(&read_asset(&path)?)?;
            Some(FaImage { path, size, rim })
        };
        if let Some(Value::Object(roles)) = root.get("display").and_then(|d| d.get("seals")) {
            for (role, id) in roles {
                if let Some(image) = id.as_str().and_then(|id| image("seals", id, 0.0)) {
                    fa.seals.insert(role.clone(), image);
                }
            }
        }
        if let Some(Value::Array(materials)) = root.get("materials") {
            for entry in materials {
                let Some(id) = entry.get("id").and_then(Value::as_str) else {
                    continue;
                };
                let rim = entry
                    .pointer("/rim/width_px")
                    .and_then(Value::as_f64)
                    .unwrap_or(1.0) as f32;
                if let Some(image) = image("materials", id, rim) {
                    fa.plates.insert(id.to_owned(), image);
                }
            }
        }
        fa
    }

    /// Seal of `role` as an image node `height` px tall at its own proportions (`FaUi.seal_rect`);
    /// `None` when the catalogue names none.
    pub fn seal(
        &self,
        assets: &AssetServer,
        role: &str,
        height: f32,
    ) -> Option<impl Bundle + use<>> {
        let seal = self.seals.get(role)?;
        let (w, h) = seal.size;
        Some((
            Name::new(format!("FaSeal_{role}")),
            Node {
                width: Val::Px((height * w as f32 / h.max(1) as f32).round()),
                height: Val::Px(height),
                flex_shrink: 0.0,
                ..default()
            },
            ImageNode::new(assets.load(seal.path.clone())),
            Pickable::IGNORE,
        ))
    }
}

/// Kit button wearing the plate `id` (`plate_crimson_brass` for the primary action,
/// `plate_leather_brass` otherwise).
#[derive(Component, Debug, Clone, Copy)]
pub struct FaPlate(pub &'static str);

/// Number of plate buttons dressed, for the dump.
#[derive(Resource, Debug, Default)]
pub struct FaStats {
    pub plate_buttons: usize,
    pub lit: usize,
}

#[allow(clippy::type_complexity)]
fn dress_plates(
    assets: Res<AssetServer>,
    mut stats: ResMut<FaStats>,
    mut buttons: Query<(&FaPlate, &Hovered, &mut ImageNode), With<KitButton>>,
) {
    let fa = FaUi::global();
    let (mut dressed, mut lit) = (0, 0);
    for (plate, hovered, mut image) in &mut buttons {
        let Some(entry) = fa.plates.get(plate.0) else {
            continue;
        };
        dressed += 1;
        let handle = assets.load(entry.path.clone());
        if image.image != handle {
            image.image = handle;
        }
        let slicer = TextureSlicer {
            border: BorderRect::all(entry.rim),
            center_scale_mode: SliceScaleMode::Tile { stretch_value: 1.0 },
            sides_scale_mode: SliceScaleMode::Tile { stretch_value: 1.0 },
            max_corner_scale: 1.0,
        };
        if !matches!(image.image_mode, NodeImageMode::Sliced(_)) {
            image.image_mode = NodeImageMode::Sliced(slicer);
        }
        let color = if hovered.get() {
            lit += 1;
            PLATE_HOVER
        } else {
            Color::WHITE
        };
        if image.color != color {
            image.color = color;
        }
    }
    stats.plate_buttons = dressed;
    stats.lit = lit;
}

fn dump(world: &World) -> (String, Value) {
    let fa = FaUi::global();
    let Some(stats) = world.get_resource::<FaStats>() else {
        return ("fa_ui".into(), json!({ "active": false }));
    };
    (
        "fa_ui".into(),
        json!({
            "seals": fa.seals.keys().collect::<Vec<_>>(),
            "plates": fa.plates.keys().collect::<Vec<_>>(),
            "plate_buttons": stats.plate_buttons,
            "plate_buttons_lit": stats.lit,
        }),
    )
}

/// The slider thumb shows `slider_grabber_hover` under the pointer (`HSlider` `grabber_highlight`).
#[allow(clippy::type_complexity)]
fn slider_hover(
    theme: Res<super::UiTheme>,
    mut thumbs: Query<
        (&Hovered, &mut ImageNode),
        (With<bevy::ui_widgets::SliderThumb>, Changed<Hovered>),
    >,
) {
    for (hovered, mut image) in &mut thumbs {
        let name = if hovered.get() {
            "slider_grabber_hover"
        } else {
            "slider_grabber"
        };
        if let Some(handle) = theme.textures.get(name) {
            image.image = handle.clone();
        }
    }
}

pub fn plugin(app: &mut App) {
    use crate::agent::dump::DumpAppExt;
    app.init_resource::<FaStats>()
        .register_dump(dump)
        .add_systems(Update, slider_hover)
        // After the kit's own button visuals, which reset the parchment texture on a state change.
        .add_systems(
            PostUpdate,
            dress_plates.before(bevy::ui::UiSystems::Prepare),
        );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn disk(base: &str) -> impl Fn(&str) -> Option<Vec<u8>> + use<> {
        let root = crate::paths::find_root().unwrap();
        let base = if base == "data" {
            crate::paths::data_dir(&root)
        } else {
            crate::paths::asset_root(&root)
        };
        move |p: &str| std::fs::read(base.join(p)).ok()
    }

    #[test]
    fn catalogue_names_two_seals_and_two_plates() {
        let fa = FaUi::from_files(disk("data"), disk("assets"));
        assert_eq!(
            fa.seals.keys().map(String::as_str).collect::<Vec<_>>(),
            ["chronicle", "treaty"]
        );
        assert_eq!(fa.plates.len(), 2);
        assert_eq!(fa.plates["plate_crimson_brass"].rim, 3.0);
        assert!(fa.seals["treaty"].size.0 > 16);
    }

    #[test]
    fn png_header_size() {
        let mut bytes = vec![
            0x89, b'P', b'N', b'G', 13, 10, 26, 10, 0, 0, 0, 13, b'I', b'H', b'D', b'R',
        ];
        bytes.extend_from_slice(&7u32.to_be_bytes());
        bytes.extend_from_slice(&9u32.to_be_bytes());
        assert_eq!(png_size(&bytes), Some((7, 9)));
        assert_eq!(png_size(b"nope"), None);
    }
}
