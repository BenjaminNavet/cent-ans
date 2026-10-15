//! Fog of war as data: `FogOfWar` holds what the player's faction sees this turn (provinces,
//! armies, per-cell coverage mask), read from `CampaignState::vision` (the rule lives in
//! `core/`, `sim_campaign::vision`). Ports `map/fog_of_war` data side (`refresh_fog`,
//! `is_province_visible`) of `minimap_controller.gd` / `map_readability.gd`; other lots (terrain
//! veil, markers, armies, minimap) read the resource, nothing is drawn here.

use std::collections::BTreeSet;

use bevy::prelude::*;
use sim_campaign::CampaignState;

use crate::menu::saves::GameLoaded;
use crate::settings::Settings;
use crate::sim::Sim;

/// Seconds between two refreshes triggered by plain changes of `Sim` (orders, moves).
const REFRESH_MIN_INTERVAL: f32 = 0.4;

/// Per-cell vision: R8 coverage (255 seen, soft edges, seen from [`SEEN_THRESHOLD`]).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct FogMask {
    pub width: u32,
    pub height: u32,
    /// Map pixels per texel.
    pub texel_px: f32,
    pub coverage: Vec<u8>,
}

/// Texel value from which a cell counts as seen (`sim_campaign::vision::SEEN_THRESHOLD`).
pub const SEEN_THRESHOLD: u8 = sim_campaign::vision::SEEN_THRESHOLD;

impl FogMask {
    /// Coverage of the texel holding map pixel `(x, y)` (0 off the mask).
    pub fn coverage_at(&self, x: f32, y: f32) -> u8 {
        if self.texel_px <= 0.0 || x < 0.0 || y < 0.0 {
            return 0;
        }
        let (cx, cy) = (
            (x / self.texel_px).floor() as u32,
            (y / self.texel_px).floor() as u32,
        );
        if cx >= self.width || cy >= self.height {
            return 0;
        }
        self.coverage[(cy * self.width + cx) as usize]
    }

    /// Share of cells seen (0..1).
    pub fn seen_share(&self) -> f32 {
        if self.coverage.is_empty() {
            return 0.0;
        }
        let seen = self
            .coverage
            .iter()
            .filter(|v| **v >= SEEN_THRESHOLD)
            .count();
        seen as f32 / self.coverage.len() as f32
    }
}

/// What the player's faction sees. `active` is false when the setting `fog_of_war` is off or no
/// campaign runs: then everything counts as visible.
#[derive(Resource, Debug, Clone, Default)]
pub struct FogOfWar {
    pub active: bool,
    pub viewer: Option<String>,
    pub turn: u32,
    /// Province ids in sight.
    pub provinces: BTreeSet<String>,
    /// Army ids in sight (own and lenders' included).
    pub armies: BTreeSet<String>,
    pub mask: Option<FogMask>,
    /// Incremented at every refresh; consumers re-upload their texture when it changes.
    pub revision: u64,
}

impl FogOfWar {
    pub fn is_province_visible(&self, id: &str) -> bool {
        !self.active || self.provinces.contains(id)
    }

    pub fn is_army_visible(&self, id: &str) -> bool {
        !self.active || self.armies.contains(id)
    }

    /// Whether map pixel `(x, y)` is in sight.
    pub fn is_point_visible(&self, x: f32, y: f32) -> bool {
        if !self.active {
            return true;
        }
        self.mask
            .as_ref()
            .is_some_and(|m| m.coverage_at(x, y) >= SEEN_THRESHOLD)
    }
}

/// Measures of the last refresh (dump).
#[derive(Resource, Debug, Default)]
pub struct FogStats {
    pub refreshes: u64,
    pub last_ms: f32,
    last_at: f32,
}

/// Computes the fog of `viewer` from the state.
pub fn compute(
    state: &CampaignState,
    data: &data_model::GameData,
    viewer: &data_model::FactionId,
) -> FogOfWar {
    let vision = state.vision(data, viewer);
    let mask = &vision.mask;
    let armies = state
        .armies
        .iter()
        .filter(|(_, army)| vision.sees_army(state, data, army))
        .map(|(id, _)| id.as_str().to_owned())
        .collect();
    FogOfWar {
        active: true,
        viewer: Some(viewer.as_str().to_owned()),
        turn: state.turn(),
        provinces: vision
            .provinces
            .iter()
            .map(|p| p.as_str().to_owned())
            .collect(),
        armies,
        mask: Some(FogMask {
            width: mask.width,
            height: mask.height,
            texel_px: mask.texel_px,
            coverage: mask.coverage.clone(),
        }),
        revision: 0,
    }
}

/// Refreshes on entering the campaign, loading a save, a turn change and (throttled) any change
/// of the simulation; the setting `fog_of_war` switches the whole thing.
#[allow(clippy::too_many_arguments)]
pub fn refresh_fog(
    time: Res<Time<Real>>,
    sim: Res<Sim>,
    settings: Option<Res<Settings>>,
    mut loaded: MessageReader<GameLoaded>,
    mut fog: ResMut<FogOfWar>,
    mut stats: ResMut<FogStats>,
    mut forced: Local<bool>,
) {
    let wanted = settings.as_ref().is_none_or(|s| s.fog_of_war);
    let load_event = loaded.read().count() > 0;
    let (Some(state), Some(data)) = (sim.state.as_ref(), sim.data.as_ref()) else {
        if fog.active || fog.viewer.is_some() {
            *fog = FogOfWar::default();
        }
        return;
    };
    if !wanted {
        if fog.active {
            let revision = fog.revision + 1;
            *fog = FogOfWar::default();
            fog.revision = revision;
        }
        return;
    }
    let now = time.elapsed_secs();
    let changed_player = fog.viewer.as_deref() != Some(sim.player.as_str());
    let turn_changed = fog.turn != state.turn();
    let due = sim.is_changed() && now - stats.last_at >= REFRESH_MIN_INTERVAL;
    if !(!fog.active || changed_player || turn_changed || load_event || due || *forced) {
        return;
    }
    *forced = false;
    let started = std::time::Instant::now();
    let revision = fog.revision + 1;
    *fog = compute(state, data, &sim.player);
    fog.revision = revision;
    stats.refreshes += 1;
    stats.last_ms = started.elapsed().as_secs_f32() * 1000.0;
    stats.last_at = now;
}

pub fn dump(world: &World) -> serde_json::Value {
    let fog = world.resource::<FogOfWar>();
    let stats = world.resource::<FogStats>();
    serde_json::json!({
        "active": fog.active,
        "viewer": fog.viewer,
        "turn": fog.turn,
        "visible_provinces": fog.provinces.len(),
        "visible_armies": fog.armies.len(),
        "mask": fog.mask.as_ref().map(|m| serde_json::json!({
            "size": [m.width, m.height], "texel_px": m.texel_px, "seen_share": m.seen_share(),
        })),
        "revision": fog.revision,
        "refreshes": stats.refreshes,
        "last_refresh_ms": stats.last_ms,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inactive_fog_sees_everything() {
        let fog = FogOfWar::default();
        assert!(fog.is_province_visible("prov_x") && fog.is_point_visible(1.0, 1.0));
        assert!(fog.is_army_visible("a"));
    }

    #[test]
    fn active_fog_filters() {
        let fog = FogOfWar {
            active: true,
            provinces: ["prov_a".to_owned()].into(),
            mask: Some(FogMask {
                width: 2,
                height: 1,
                texel_px: 10.0,
                coverage: vec![255, 0],
            }),
            ..default()
        };
        assert!(fog.is_province_visible("prov_a") && !fog.is_province_visible("prov_b"));
        assert!(fog.is_point_visible(5.0, 5.0) && !fog.is_point_visible(15.0, 5.0));
        assert!(!fog.is_point_visible(-1.0, 0.0) && !fog.is_point_visible(500.0, 0.0));
        assert_eq!(fog.mask.as_ref().unwrap().seen_share(), 0.5);
    }
}
