//! Shared streaming of the living map: the view (focus, zone, thinning), the index of live
//! instances and the generic fade targets (ADR 0354: anchored on the world, thinning by a fixed
//! per-instance random number compared with a continuous threshold, fades at the edge and at
//! the range instead of pops).

use std::collections::HashSet;

use bevy::prelude::*;

use super::data::LifeData;
use super::models::Fade;

/// Category of an instance (counted by the dump, parameters of the fades).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Cat {
    Walker,
    Cart,
    Beast,
    Fauna,
    RiverBoat,
    SeaShip,
    Mill,
    Sail,
    Plume,
    Fire,
    Bird,
}

impl Cat {
    pub const ALL: [Cat; 11] = [
        Cat::Walker,
        Cat::Cart,
        Cat::Beast,
        Cat::Fauna,
        Cat::RiverBoat,
        Cat::SeaShip,
        Cat::Mill,
        Cat::Sail,
        Cat::Plume,
        Cat::Fire,
        Cat::Bird,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Cat::Walker => "walkers",
            Cat::Cart => "carts",
            Cat::Beast => "draught_beasts",
            Cat::Fauna => "fauna",
            Cat::RiverBoat => "river_boats",
            Cat::SeaShip => "sea_ships",
            Cat::Mill => "mills",
            Cat::Sail => "mill_sails",
            Cat::Plume => "plumes",
            Cat::Fire => "fires",
            Cat::Bird => "birds",
        }
    }
}

/// One live instance. `anchor` (map px) follows the instance when it moves.
#[derive(Component, Debug, Clone)]
pub struct LifeItem {
    pub key: u64,
    pub cat: Cat,
    pub anchor: Vec2,
    /// Fixed random number [0, 1) of the instance (thinning), shared by a whole unit.
    pub h: f32,
}

/// Keys of the live instances (no double spawn).
#[derive(Resource, Default)]
pub struct LifeIndex(pub HashSet<u64>);

/// Camera, season and time as the living map sees them.
#[derive(Resource, Debug, Clone)]
pub struct LifeView {
    pub ready: bool,
    pub focus: Vec2,
    /// Camera distance to the aimed point (world units).
    pub distance: f32,
    /// Radius of the visible zone (px).
    pub radius: f32,
    /// Share of the thinned instances shown (`keep_for`).
    pub keep: f32,
    /// 0 spring, 1 summer, 2 autumn, 3 winter.
    pub season: usize,
    pub time: f32,
    /// Bumped when the relief exaggeration changed (statics are re-grounded).
    pub relief_version: u32,
}

impl Default for LifeView {
    fn default() -> Self {
        Self {
            ready: false,
            focus: Vec2::ZERO,
            distance: 1000.0,
            radius: 0.0,
            keep: 0.1,
            season: 1,
            time: 0.0,
            relief_version: 0,
        }
    }
}

impl LifeView {
    /// Instances with `h` below this are placed (threshold x 1.5 + 0.03, ADR 0354).
    pub fn ceiling(&self) -> f32 {
        (self.keep * 1.5 + 0.03).min(1.0)
    }

    /// Fade target of a thinned instance: 1 below `keep`, 0 at the ceiling, continuous between.
    pub fn thin_target(&self, h: f32) -> f32 {
        let c = self.ceiling();
        if h < self.keep {
            1.0
        } else if c > self.keep {
            ((c - h) / (c - self.keep)).clamp(0.0, 1.0)
        } else {
            0.0
        }
    }

    /// Fade at the edge of the visible zone (starts at `inner` x radius, 0 at the radius).
    pub fn edge_target(&self, p: Vec2, inner: f32) -> f32 {
        if self.radius <= 0.0 {
            return 0.0;
        }
        let d = p.distance(self.focus);
        let start = self.radius * inner;
        if d <= start {
            1.0
        } else {
            ((self.radius - d) / (self.radius - start).max(1e-3)).clamp(0.0, 1.0)
        }
    }

    /// Fade of the range (`folk_range`): from `start` x range to the range.
    pub fn range_target(&self, range: f32, start: f32) -> f32 {
        let from = range * start;
        if self.distance <= from {
            1.0
        } else {
            ((range - self.distance) / (range - from).max(1e-3)).clamp(0.0, 1.0)
        }
    }
}

/// Lifts an instance to the ground (statics are re-grounded when the relief changes).
#[derive(Component)]
pub struct Grounded {
    pub lift: f32,
    pub last_version: u32,
    pub last_ready: bool,
}

impl Grounded {
    pub fn new(lift: f32) -> Self {
        Self {
            lift,
            last_version: u32::MAX,
            last_ready: false,
        }
    }
}

/// Updates [`LifeView`] from the campaign camera, the sim season and the clock.
pub fn update_view(
    time: Res<Time>,
    data: Res<LifeData>,
    rig: Option<Res<crate::campaign::camera::rig::CampaignRig>>,
    sim: Res<crate::sim::Sim>,
    relief: Option<Res<crate::campaign::terrain::relief_state::ReliefState>>,
    mut view: ResMut<LifeView>,
) {
    view.time += time.delta_secs();
    if let Some(relief) = relief
        && relief.is_changed()
    {
        view.relief_version = view.relief_version.wrapping_add(1);
    }
    if let Some(state) = sim.state.as_ref() {
        view.season = match state.season {
            sim_campaign::Season::Spring => 0,
            sim_campaign::Season::Summer => 1,
            sim_campaign::Season::Autumn => 2,
            sim_campaign::Season::Winter => 3,
        };
    }
    let Some(rig) = rig else {
        view.ready = false;
        return;
    };
    view.ready = true;
    view.focus = Vec2::new(rig.focus.x, rig.focus.z);
    view.distance = rig.distance.max(1.0);
    view.radius = (view.distance * data.props.folk_radius_factor).clamp(30.0, 900.0);
    view.keep = data.props.keep_for(view.distance);
}

/// Sets the fade target of every instance from its category (edge, range, thinning, season).
pub fn update_targets(
    data: Res<LifeData>,
    view: Res<LifeView>,
    mut items: Query<(&LifeItem, &mut Fade)>,
) {
    if !view.ready {
        return;
    }
    let props = &data.props;
    for (item, mut fade) in &mut items {
        let target = match item.cat {
            Cat::Walker | Cat::Cart | Cat::Beast => {
                view.thin_target(item.h)
                    * view.edge_target(item.anchor, props.folk_edge_inner)
                    * view.range_target(props.folk_range, props.folk_range_fade)
            }
            Cat::Fauna => {
                let f = &data.fauna;
                let range = if f.max_distance > f.fade_from {
                    ((f.max_distance - view.distance) / (f.max_distance - f.fade_from))
                        .clamp(0.0, 1.0)
                } else {
                    1.0
                };
                view.thin_target(item.h)
                    * view.edge_target(item.anchor, props.folk_edge_inner)
                    * range
            }
            Cat::RiverBoat | Cat::SeaShip => {
                // Visible in the 3D views, gone under the parchment (distance > 1200).
                let far = ((1400.0 - view.distance) / 200.0).clamp(0.0, 1.0);
                far * view.edge_target(item.anchor, 0.8)
            }
            Cat::Mill | Cat::Sail => {
                view.range_target(props.windmill_range, 0.7) * view.edge_target(item.anchor, 0.85)
            }
            Cat::Plume | Cat::Fire => {
                view.range_target(props.chimney_range.max(500.0), 0.7)
                    * view.edge_target(item.anchor, 0.85)
            }
            Cat::Bird => 1.0,
        };
        fade.target = target;
    }
}

/// Re-grounds the statics.
pub fn ground_statics(
    ground: crate::campaign::terrain::surface::GroundHeight,
    view: Res<LifeView>,
    mut query: Query<(&mut Grounded, &LifeItem, &mut Transform)>,
) {
    let ready = ground.is_ready();
    for (mut g, item, mut t) in &mut query {
        if ready && g.last_ready && g.last_version == view.relief_version {
            continue;
        }
        if let Some(y) = ground.at(item.anchor.x, item.anchor.y) {
            t.translation.y = y + g.lift;
            g.last_ready = true;
            g.last_version = view.relief_version;
        }
    }
}
