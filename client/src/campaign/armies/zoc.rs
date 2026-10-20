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

/// Lift above the ground, in world units.
const LIFT: f32 = 0.25;

/// `zoc` block of `data/map/stance_cues.json`.
#[derive(Resource, Debug, Clone)]
pub struct ZocTuning {
    pub max_rings: usize,
    pub color: Color,
    /// Discreet ring of the allied and own armies.
    pub friend_color: Color,
    pub pulse_hz: f32,
    pub pulse_depth: f32,
    pub segments: usize,
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
        let friend_hex = zoc["friend_color"].as_str().unwrap_or("#d9c88f");
        let friend_alpha = zoc["friend_alpha"].as_f64().unwrap_or(0.3) as f32;
        let [fr, fg, fb] = super::views::parse_hex_color(friend_hex);
        Self {
            max_rings,
            color: Color::srgba(r, g, b, alpha),
            friend_color: Color::srgba(fr, fg, fb, friend_alpha),
            pulse_hz: zoc["pulse_hz"].as_f64().unwrap_or(0.5) as f32,
            pulse_depth: zoc["pulse_depth"].as_f64().unwrap_or(0.4).clamp(0.0, 1.0) as f32,
            segments: zoc["segments"].as_u64().unwrap_or(96).clamp(16, 256) as usize,
        }
    }

    pub fn load() -> Self {
        crate::paths::find_root()
            .ok()
            .and_then(|root| std::fs::read_to_string(root.join("data/map/stance_cues.json")).ok())
            .map_or_else(Self::default, |text| Self::parse(&text))
    }

    /// Opacity factor of the pulse at `time_s`, in `[1 - depth, 1]`.
    pub fn pulse(&self, time_s: f32) -> f32 {
        let wave = 0.5 + 0.5 * (time_s * self.pulse_hz * std::f32::consts::TAU).sin();
        1.0 - self.pulse_depth + self.pulse_depth * wave
    }
}

/// A ring to draw: centre in map pixels, enemy (red, pulsed wave) or own / allied (discreet).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ring {
    pub centre: Vec2,
    pub hostile: bool,
}

/// Rings drawn this frame (map pixels), dump and tests.
#[derive(Resource, Default, Debug)]
pub struct ZocRings {
    pub rings: Vec<Ring>,
    pub radius_px: f32,
}

impl ZocRings {
    pub fn hostile(&self) -> usize {
        self.rings.iter().filter(|r| r.hostile).count()
    }
}

/// The rings to draw. `hovered` gets its ring whatever the option says (Godot `hover`: red for
/// an enemy, discreet for a friend); the enemies in view, and the selected own army's discreet
/// ring, only with an own army selected and `show_zoc` on.
pub fn ring_centres(
    views: &[ArmyView],
    selected: Option<&ArmyId>,
    hovered: Option<&ArmyId>,
    show_zoc: bool,
    max_rings: usize,
) -> Vec<Ring> {
    let ring = |v: &ArmyView| Ring {
        centre: Vec2::new(v.point[0], v.point[1]),
        hostile: v.hostile,
    };
    let mut rings: Vec<Ring> = Vec::new();
    let own = selected.and_then(|id| views.iter().find(|v| &v.id == id && v.player));
    if let Some(own) = own
        && show_zoc
    {
        rings.extend(views.iter().filter(|v| v.hostile).take(max_rings).map(ring));
        rings.push(Ring {
            hostile: false,
            ..ring(own)
        });
    }
    if let Some(view) = hovered.and_then(|id| views.iter().find(|v| &v.id == id)) {
        let hovered_ring = ring(view);
        if !rings.iter().any(|r| r.centre == hovered_ring.centre) {
            rings.push(hovered_ring);
        }
    }
    rings
}

#[allow(clippy::too_many_arguments)]
pub fn draw_zoc(
    sim: Res<Sim>,
    time: Res<Time>,
    views: Res<ArmyViews>,
    selected: Res<SelectedArmy>,
    hovered: Res<HoveredArmy>,
    settings: Option<Res<Settings>>,
    tuning: Res<ZocTuning>,
    mut rings: ResMut<ZocRings>,
    ground: GroundHeight,
    mut gizmos: Gizmos,
) {
    // The radius is the core's `zoc_radius_km` (free-movement rules) in map pixels.
    let radius = sim.data.as_ref().map_or(0.0, |data| {
        data.free_movement_rules().zoc_radius_km as f32 * sim_campaign::march::px_per_km(data)
    });
    let show = settings.is_none_or(|s| s.show_zoc);
    rings.radius_px = radius;
    rings.rings = if radius > 0.0 {
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
    let t = time.elapsed_secs();
    let pulse = tuning.pulse(t);
    let wave = (t * tuning.pulse_hz).fract();
    for ring in &rings.rings {
        let base = if ring.hostile {
            tuning.color
        } else {
            tuning.friend_color
        };
        // Ring on the ground at `scale` of the radius: every vertex follows the relief.
        let mut circle = |scale: f32, alpha_scale: f32| {
            let points = (0..=tuning.segments).map(|i| {
                let angle = i as f32 / tuning.segments as f32 * std::f32::consts::TAU;
                let p = ring.centre + Vec2::from_angle(angle) * radius * scale;
                px_to_world(p, &ground) + Vec3::Y * LIFT
            });
            gizmos.linestrip(points, base.with_alpha(base.alpha() * alpha_scale));
        };
        circle(1.0, pulse);
        if ring.hostile {
            // A wave runs inwards from the edge and fades: the zone is alive.
            circle(1.0 - 0.18 * wave, (1.0 - wave) * 0.7);
        }
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

    fn centres(rings: &[Ring]) -> Vec<(Vec2, bool)> {
        rings.iter().map(|r| (r.centre, r.hostile)).collect()
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
        // Enemies in red, then the selected own army's discreet ring.
        assert_eq!(
            centres(&on),
            vec![
                (Vec2::new(10.0, 0.0), true),
                (Vec2::new(20.0, 0.0), true),
                (Vec2::new(0.0, 0.0), false)
            ]
        );
        assert!(ring_centres(&views, Some(&id_of("mine")), None, false, 16).is_empty());
        // Hover outlines an enemy even with the option off.
        assert_eq!(
            centres(&ring_centres(&views, None, Some(&id_of("e2")), false, 16)),
            vec![(Vec2::new(20.0, 0.0), true)]
        );
        // A hovered friend gets a discreet ring, never a red one.
        assert_eq!(
            centres(&ring_centres(
                &views,
                None,
                Some(&id_of("friend")),
                false,
                16
            )),
            vec![(Vec2::new(30.0, 0.0), false)]
        );
        // No own army selected: no enemy rings.
        assert!(ring_centres(&views, Some(&id_of("e1")), None, true, 16).is_empty());
        // Cap, and no duplicate when the hovered one already has its ring.
        let capped = ring_centres(&views, Some(&id_of("mine")), Some(&id_of("e1")), true, 1);
        assert_eq!(capped.iter().filter(|r| r.hostile).count(), 1);
        assert_eq!(capped.len(), 2);
        assert_eq!(
            ring_centres(&views, Some(&id_of("mine")), Some(&id_of("e2")), true, 1).len(),
            3
        );
    }

    #[test]
    fn pulse_stays_within_its_depth() {
        let t = ZocTuning::default();
        for k in 0..100 {
            let p = t.pulse(k as f32 * 0.07);
            assert!(p >= 1.0 - t.pulse_depth - 1e-5 && p <= 1.0 + 1e-5, "{p}");
        }
    }

    #[test]
    fn tuning_reads_the_cues_file() {
        let t = ZocTuning::parse(r##"{"zoc":{"max_rings":3,"color":"#ff0000","alpha":0.5}}"##);
        assert_eq!(t.max_rings, 3);
        assert_eq!(t.color, Color::srgba(1.0, 0.0, 0.0, 0.5));
        assert_eq!(ZocTuning::default().max_rings, 16);
    }
}
