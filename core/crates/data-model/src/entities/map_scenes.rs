//! Living map tuning (lot FK1), mirroring
//! `data/schemas/map_scenes_rules.schema.json` (`data/rules/map_scenes.json`).
//! Spec `docs/design/2026-09-29-carte-vivante-folk.md` §§ 2.1.1, 2.1.4.
//!
//! Purely visual: the scene durations feed `sim-campaign::map_scenes`, the
//! densities and caps are read by the renderer (`game/scripts/map/life_folk`).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::entities::event::SceneKind;

/// Turns a scene lasts when a kind is missing from `durations`.
pub const DEFAULT_SCENE_TURNS: u32 = 2;

/// Contents of `data/rules/map_scenes.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MapSceneRules {
    /// Turns a scene born of a chronicle event stays on the map, by kind.
    pub durations: BTreeMap<SceneKind, u32>,
    /// Devastation (0-100) from which a province shows a `devastation` scene.
    pub devastation_threshold: u8,
    /// Most figurines alive at once on the close view (halved when the
    /// frame budget is exceeded).
    pub pool_cap: u32,
    /// Radius (world units) around the camera centre where figurines live.
    pub activity_radius: f64,
    /// Camera distance (world units) beyond which no figurine is placed:
    /// figurines are drawn at real (1:1) scale and are sub-pixel further
    /// away (VT2, ADR 0138).
    pub figure_max_distance: f64,
    /// Road travellers per world unit at the reference population.
    pub road_folk_per_unit: f64,
    /// Chance a field point is worked.
    pub field_work_probability: f64,
    /// Chance a pasture point holds a herd.
    pub herd_probability: f64,
    /// Chance a forest point holds woodcutters in winter.
    pub woodcutter_probability: f64,
    /// Chance a road piece near a city holds pilgrims.
    pub pilgrim_probability: f64,
    /// Merchant carts per 10 world units of road and per unit of trade
    /// route value.
    pub carts_per_trade_value: f64,
    /// Trade route value from which a cart has a guard (two at double).
    pub guard_value: f64,
    /// Extras of a province scene at intensity 0.
    pub scene_figures_min: u32,
    /// Extras of a province scene at intensity 1.
    pub scene_figures_max: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

impl MapSceneRules {
    /// Turns a scene of `kind` born of an event lasts (at least one).
    pub fn duration(&self, kind: SceneKind) -> u32 {
        self.durations
            .get(&kind)
            .copied()
            .unwrap_or(DEFAULT_SCENE_TURNS)
            .max(1)
    }

    /// Longest duration of any kind (history the chronicle keeps).
    pub fn max_duration(&self) -> u32 {
        SceneKind::ALL
            .iter()
            .map(|kind| self.duration(*kind))
            .max()
            .unwrap_or(DEFAULT_SCENE_TURNS)
    }
}

crate::bundled_rules!(MapSceneRules, "rules/map_scenes.json", default);
