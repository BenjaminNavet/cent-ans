//! "Cover" cropping of illustrations (Godot `STRETCH_KEEP_ASPECT_COVERED` + `clip_contents`):
//! the picture fills the whole box, keeps its proportions and the overflow is cut, centred.
//! Bevy UI 0.20 has no object-fit: reuses the menu's [`CoverFit`] source-rect crop.
//! Used by the decision art (HUD window and chronicle tab) and the journal vignette.
//! (The report vignette is a rich-text `[img]`, which cannot be cropped.)

use std::path::Path;

use bevy::prelude::*;

use crate::menu::CoverFit;
use crate::ui::art_plates::ArtPlates;

/// Asset path (relative to `game/assets`) of an `res://assets/...` vignette image.
pub fn plate_path(image: &str) -> String {
    image
        .strip_prefix("res://assets/")
        .unwrap_or(image)
        .to_owned()
}

/// A `w` x `h` node showing `path` in cover mode: [`CoverFit`] crops the source rectangle
/// (centred) to the node's aspect once the image is loaded, like Godot's
/// `STRETCH_KEEP_ASPECT_COVERED`.
pub fn cover_image(assets: &AssetServer, path: &str, name: String, w: f32, h: f32) -> impl Bundle {
    (
        Name::new(name),
        Node {
            width: Val::Px(w),
            height: Val::Px(h),
            align_self: AlignSelf::Center,
            flex_shrink: 0.0,
            ..default()
        },
        ImageNode::new(assets.load(path.to_owned())),
        CoverFit::new(Vec2::splat(0.5)),
    )
}

/// The picture of a decision: `events/<event>.jpg`, else the vignette of its genre.
pub fn decision_picture(
    plates: Option<&ArtPlates>,
    assets_root: Option<&Path>,
    event: &str,
    kind: &str,
) -> Option<String> {
    crate::campaign::panels::chronicle::art::decision_art(assets_root, event).or_else(|| {
        plates?
            .vignette_for_kind(kind)
            .filter(|v| !v.image.is_empty())
            .map(|v| plate_path(&v.image))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vignette_falls_back_to_the_genre_of_the_decision() {
        let plates = ArtPlates::parse(&serde_json::json!({"vignettes": [
            {"id": "vg_plague", "kinds": ["plague"], "image": "res://assets/art/vignettes/vg_plague.jpg"}]}));
        assert_eq!(
            decision_picture(Some(&plates), None, "evt_x", "plague").as_deref(),
            Some("art/vignettes/vg_plague.jpg")
        );
        assert_eq!(decision_picture(Some(&plates), None, "evt_x", ""), None);
    }
}
