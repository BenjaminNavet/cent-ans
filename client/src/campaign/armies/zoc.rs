//! Zone-of-control rings (ports `army_movement_controller.gd::{hover, show_zoc, refresh_enemy_zoc}`
//! and `stance_cues.gd::{zoc_max_rings, zoc_color}`). Hovering an enemy army outlines its zone of
//! control; while one of the player's armies is selected and the option `map/show_zoc` is on, every
//! enemy army in view gets its ring (at most `zoc.max_rings`). Fogged armies are not in `ArmyViews`.

use bevy::prelude::*;
use sim_campaign::ArmyId;

use super::projection::px_to_world;
use super::views::{ArmyView, ArmyViews};
use super::{HoveredArmy, SelectedArmy};
use crate::campaign::terrain::surface::GroundHeight;
use crate::settings::Settings;
use crate::sim::Sim;

const SEGMENTS: usize = 64;
/// Lift above the ground, in world units.
const LIFT: f32 = 0.25;

/// `zoc` block of `data/map/stance_cues.json`.
#[derive(Resource, Debug, Clone)]
pub struct ZocTuning {
    pub max_rings: usize,
    pub color: Color,
}

impl Default for ZocTuning {
    fn default() -> Self {
        Self::parse("{}")
    }
}

impl ZocTuning {
    pub fn parse(text: &str) -> Self {
        let value: serde_json::Value = serde_json::from_str(text).unwrap_or_default();
        let zoc = &value["zoc"];
        let max_rings = zoc["max_rings"].as_u64().unwrap_or(16).max(1) as usize;
        let hex = zoc["color"].as_str().unwrap_or("#e0241b");
        let alpha = zoc["alpha"].as_f64().unwrap_or(0.8) as f32;
        let [r, g, b] = super::views::parse_hex_color(hex);
        Self {
            max_rings,
            color: Color::srgba(r, g, b, alpha),
        }
    }

    pub fn load() -> Self {
        crate::paths::find_root()
            .ok()
            .and_then(|root| std::fs::read_to_string(root.join("data/map/stance_cues.json")).ok())
            .map_or_else(Self::default, |text| Self::parse(&text))
    }
}

/// Rings drawn this frame (map pixels), dump and tests.
#[derive(Resource, Default, Debug)]
pub struct ZocRings {
    pub centres: Vec<Vec2>,
    pub radius_px: f32,
}

/// Centres of the rings to draw. `hovered` is outlined whatever the option says (Godot
/// `hover`), the enemies in view only with an own army selected and `show_zoc` on.
pub fn ring_centres(
    views: &[ArmyView],
    selected: Option<&ArmyId>,
    hovered: Option<&ArmyId>,
    show_zoc: bool,
    max_rings: usize,
) -> Vec<Vec2> {
    let at = |v: &ArmyView| Vec2::new(v.point[0], v.point[1]);
    let mut centres = Vec::new();
    let selected_is_own = selected.is_some_and(|id| views.iter().any(|v| &v.id == id && v.player));
    if selected_is_own && show_zoc {
        centres.extend(views.iter().filter(|v| v.hostile).take(max_rings).map(at));
    }
    if let Some(view) = hovered.and_then(|id| views.iter().find(|v| &v.id == id && v.hostile)) {
        let centre = at(view);
        if !centres.contains(&centre) {
            centres.push(centre);
        }
    }
    centres
}

#[allow(clippy::too_many_arguments)]
pub fn draw_zoc(
    sim: Res<Sim>,
    views: Res<ArmyViews>,
    selected: Res<SelectedArmy>,
    hovered: Res<HoveredArmy>,
    settings: Option<Res<Settings>>,
    tuning: Res<ZocTuning>,
    mut rings: ResMut<ZocRings>,
    ground: GroundHeight,
    mut gizmos: Gizmos,
) {
    let radius = sim.data.as_ref().map_or(0.0, |data| {
        data.free_movement_rules().zoc_radius_km as f32 * sim_campaign::march::px_per_km(data)
    });
    let show = settings.is_none_or(|s| s.show_zoc);
    rings.radius_px = radius;
    rings.centres = if radius > 0.0 {
        ring_centres(
            &views.0,
            selected.0.as_ref(),
            hovered.0.as_ref(),
            show,
            tuning.max_rings,
        )
    } else {
        Vec::new()
    };
    for centre in &rings.centres {
        let points = (0..=SEGMENTS).map(|i| {
            let angle = i as f32 / SEGMENTS as f32 * std::f32::consts::TAU;
            let p = *centre + Vec2::from_angle(angle) * radius;
            px_to_world(p, &ground) + Vec3::Y * LIFT
        });
        gizmos.linestrip(points, tuning.color);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id_of(text: &str) -> ArmyId {
        let number = match text {
            "mine" => 1,
            "e1" => 2,
            "e2" => 3,
            _ => 4,
        };
        ArmyId::parse(&format!("army_{number:04}")).expect("id")
    }

    fn view(id: &str, x: f32, player: bool, hostile: bool) -> ArmyView {
        ArmyView {
            id: id_of(id),
            faction: data_model::FactionId::new("fac_x").expect("faction"),
            color: [0.0; 3],
            point: [x, 0.0],
            men: 1,
            units: 1,
            player,
            hostile,
            status: "idle",
            in_settlement: false,
            settlement: None,
            categories: vec![],
            movement_left: 0,
            general: None,
            royal: false,
            roster: vec![],
        }
    }

    #[test]
    fn show_zoc_option_gates_the_enemy_rings_not_the_hover_ring() {
        let views = [
            view("mine", 0.0, true, false),
            view("e1", 10.0, false, true),
            view("e2", 20.0, false, true),
            view("friend", 30.0, false, false),
        ];
        let on = ring_centres(&views, Some(&id_of("mine")), None, true, 16);
        assert_eq!(on, vec![Vec2::new(10.0, 0.0), Vec2::new(20.0, 0.0)]);
        assert!(ring_centres(&views, Some(&id_of("mine")), None, false, 16).is_empty());
        // Hover outlines an enemy even with the option off; not a friend.
        assert_eq!(
            ring_centres(&views, None, Some(&id_of("e2")), false, 16),
            vec![Vec2::new(20.0, 0.0)]
        );
        assert!(ring_centres(&views, None, Some(&id_of("friend")), false, 16).is_empty());
        // No own army selected: no enemy rings.
        assert!(ring_centres(&views, Some(&id_of("e1")), None, true, 16).is_empty());
        // Cap, and no duplicate when the hovered one already has its ring.
        assert_eq!(
            ring_centres(&views, Some(&id_of("mine")), Some(&id_of("e1")), true, 1).len(),
            1
        );
        assert_eq!(
            ring_centres(&views, Some(&id_of("mine")), Some(&id_of("e2")), true, 1).len(),
            2
        );
    }

    #[test]
    fn tuning_reads_the_cues_file() {
        let t = ZocTuning::parse(r##"{"zoc":{"max_rings":3,"color":"#ff0000","alpha":0.5}}"##);
        assert_eq!(t.max_rings, 3);
        assert_eq!(t.color, Color::srgba(1.0, 0.0, 0.0, 0.5));
        assert_eq!(ZocTuning::default().max_rings, 16);
    }
}
