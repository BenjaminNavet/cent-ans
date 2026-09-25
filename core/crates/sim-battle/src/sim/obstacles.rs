//! BR3 (ADR 0047): figures kept out of the buildings and the street
//! furniture. A regiment is a point for the rules; its figures are laid out
//! in a rectangle around it ([`crate::unit::Unit::figure_positions`]) and
//! would otherwise stand in the facades of the house blocks, the village
//! houses and the props. Each figure inside a footprint (grown by the figure
//! margin) is moved to the nearest way out that lands in no other footprint.
//!
//! Cost: the obstacles are first filtered by the regiment's bounding circle
//! (nothing is done when none is near), then each figure is only tested
//! against those few rectangles.

use super::BattleSim;
use crate::town::{Footprint, Prop, TownRules};
use crate::unit::Unit;

impl BattleSim {
    /// BR3: the props of the battle village (derived from its houses,
    /// computed on first use).
    pub fn village_props(&self) -> &[Prop] {
        self.village_props.get_or_init(|| {
            self.field
                .village
                .as_ref()
                .map(|v| crate::props::village_props(v, TownRules::bundled()))
                .unwrap_or_default()
        })
    }

    /// BR3: the solid footprints within `radius` of (x, z): standing house
    /// blocks and their props (siege), village houses and props.
    pub fn obstacles_near(&self, x: f64, z: f64, radius: f64) -> Vec<Footprint> {
        let mut out = Vec::new();
        let mut take = |f: Footprint| {
            if (f.x - x).hypot(f.z - z) < radius + f.bounding_radius() {
                out.push(f);
            }
        };
        if let Some(works) = &self.siege {
            for house in works.houses.iter().filter(|h| h.standing()) {
                take(house.footprint());
            }
            for prop in works.standing_props() {
                take(prop.footprint());
            }
        }
        // EP6: buildings and solid props of the decor, camp furniture.
        for f in self.field.decor_footprints_near(x, z, radius) {
            take(f);
        }
        if let Some(village) = &self.field.village {
            // Houses stand within the zone (a little slack for their size).
            if (village.zone.x - x).hypot(village.zone.z - z) < village.zone.radius + radius + 30.0
            {
                for h in &village.houses {
                    take(Footprint::new(h.x, h.z, h.length, h.width, h.yaw));
                }
                for prop in self.village_props() {
                    take(prop.footprint());
                }
            }
        }
        out
    }

    /// BR3: moves the figures of `unit` out of the footprints near it.
    pub(super) fn push_figures_out(&self, unit: &Unit, positions: &mut [(f64, f64, f64)]) {
        if positions.is_empty()
            || (self.siege.is_none() && self.field.village.is_none() && self.field.decor.is_empty())
        {
            return;
        }
        let reach = positions
            .iter()
            .map(|&(x, z, _)| (x - unit.x).hypot(z - unit.z))
            .fold(0.0, f64::max);
        let near = self.obstacles_near(unit.x, unit.z, reach + 1.0);
        if near.is_empty() {
            return;
        }
        let margin = TownRules::bundled().figures.margin_m;
        push_out(&near, positions, margin);
    }
}

/// Moves every position inside one of `obstacles` (grown by `margin`) to
/// the nearest way out that is clear of all of them (else the nearest).
pub(crate) fn push_out(obstacles: &[Footprint], positions: &mut [(f64, f64, f64)], margin: f64) {
    let inside = |x: f64, z: f64, o: &&Footprint| o.contains(x, z, margin * 0.5);
    for pos in positions.iter_mut() {
        for _ in 0..3 {
            let Some(o) = obstacles.iter().find(|o| inside(pos.0, pos.1, o)) else {
                break;
            };
            let exits = o.exits(pos.0, pos.1, margin);
            let (x, z) = exits
                .iter()
                .find(|(_, (x, z))| !obstacles.iter().any(|o| inside(*x, *z, &o)))
                .map_or(exits[0].1, |e| e.1);
            pos.0 = x;
            pos.1 = z;
        }
    }
}
