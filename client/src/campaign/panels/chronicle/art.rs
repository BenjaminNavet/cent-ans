//! Decision art of the chronicle (`chronicle_window.gd`: `EVENT_ART_DIR + event + ".jpg"`).
//! Without its own picture a decision shows the vignette of its genre (`DecisionView::kind`,
//! `ArtPlates.vignette_for_kind`): see `hud::cover::decision_picture`.

use std::path::Path;

/// Height of the picture above a decision.
pub const ART_HEIGHT: f32 = 120.0;

/// `events/<event>.jpg` when `exists` says the file is there.
pub fn decision_art_path(event: &str, exists: impl Fn(&str) -> bool) -> Option<String> {
    let path = format!("events/{event}.jpg");
    exists(&path).then_some(path)
}

/// Same lookup against the asset folder on disk (`None` without a root).
pub fn decision_art(assets_root: Option<&Path>, event: &str) -> Option<String> {
    let root = assets_root?;
    decision_art_path(event, |path| root.join(path).is_file())
}

/// `game/assets` of this checkout.
pub fn assets_root() -> Option<std::path::PathBuf> {
    crate::paths::find_root()
        .ok()
        .map(|root| crate::paths::asset_root(&root))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_id_maps_to_its_picture() {
        let present = |p: &str| p == "events/evt_a.jpg";
        assert_eq!(
            decision_art_path("evt_a", present).as_deref(),
            Some("events/evt_a.jpg")
        );
        assert_eq!(decision_art_path("evt_b", present), None);
    }
}
