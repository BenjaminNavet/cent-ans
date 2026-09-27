//! Deployment phase (F5a § 3): before the first tick the player places his
//! regiments inside a rectangular zone of his side, then starts the battle.
//!
//! [`BattleSim::begin_deployment`] freezes time (ticks do nothing, only
//! formation / fire-at-will orders are accepted) and lays the AI side out by
//! roles; [`BattleSim::deploy_unit`] moves one regiment, validated against
//! [`BattleSim::deployment_zone`] (sieges: the besiegers outside the walls and
//! out of the wall bands, the garrison inside the ring);
//! [`BattleSim::start_battle`] ends the phase.

use serde::{Deserialize, Serialize};

use super::BattleSim;
use crate::command::CommandError;
use crate::setup::SideId;

/// Depth of a field-battle deployment zone on the standard field (metres
/// from the own edge); EP1: the zone is [`crate::scale::FieldSize::zone_depth`]
/// deep and its front 50 m beyond the side's battle line.
pub const ZONE_DEPTH: f64 = 300.0;
/// Besiegers deploy at least this far from the front of the walls.
pub const SIEGE_STANDOFF: f64 = 60.0;

/// Axis-aligned rectangle `[x0, x1] × [z0, z1]` in field metres.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct DeploymentZone {
    pub x0: f64,
    pub z0: f64,
    pub x1: f64,
    pub z1: f64,
}

impl DeploymentZone {
    pub fn contains(&self, x: f64, z: f64) -> bool {
        (self.x0..=self.x1).contains(&x) && (self.z0..=self.z1).contains(&z)
    }

    pub fn clamp(&self, x: f64, z: f64) -> (f64, f64) {
        (x.clamp(self.x0, self.x1), z.clamp(self.z0, self.z1))
    }
}

impl BattleSim {
    /// Opens the deployment phase (before the first tick only). Returns
    /// `false` when the battle has already started.
    pub fn begin_deployment(&mut self) -> bool {
        if self.ticks > 0 || self.finished {
            return false;
        }
        // CV3-2: a player caught in column or in forced march has no
        // deployment phase.
        if self.setup.player_side.is_some_and(|s| !self.can_deploy(s)) {
            return false;
        }
        if !self.deploying {
            self.deploying = true;
            for side in SideId::BOTH {
                // CV3-2: the column and the forced march keep their automatic
                // placement; the ambusher already stands on the flanks.
                let ambusher = self.ambush.as_ref().is_some_and(|l| l.victim != side);
                if self.ai_enabled[side.index()] && self.can_deploy(side) && !ambusher {
                    self.ai_deploy(side);
                }
            }
            self.log("Déploiement : placez vos régiments.".to_owned(), None);
        }
        true
    }

    /// Places regiment `id` at (x, z) facing `facing` (radians; kept when
    /// `None`), inside its side's zone. Only the player's regiments when the
    /// setup names a player side.
    pub fn deploy_unit(
        &mut self,
        id: u32,
        x: f64,
        z: f64,
        facing: Option<f64>,
    ) -> Result<(), CommandError> {
        self.deploy_unit_width(id, x, z, facing, None)
    }

    /// CB1: [`Self::deploy_unit`] with the frontage of a right-drag: the
    /// regiment forms a Line `width` metres wide (ranks within
    /// `data/rules/formation_width.json`; unchanged without width).
    pub fn deploy_unit_width(
        &mut self,
        id: u32,
        x: f64,
        z: f64,
        facing: Option<f64>,
        width: Option<f64>,
    ) -> Result<(), CommandError> {
        if !self.deploying {
            return Err(CommandError::NotDeploying);
        }
        let unit = self
            .units
            .get(id as usize)
            .ok_or(CommandError::UnknownUnit(id))?;
        if self.setup.player_side.is_some_and(|s| s != unit.side) {
            return Err(CommandError::NotYours(id));
        }
        if !unit.present() {
            return Err(CommandError::Unavailable(id));
        }
        if !self.deployable(unit.side, x, z) {
            return Err(CommandError::OutsideZone(id));
        }
        let unit = &mut self.units[id as usize];
        unit.x = x;
        unit.z = z;
        if let Some(f) = facing.filter(|f| f.is_finite()) {
            unit.facing = f;
        }
        unit.destination = None;
        unit.target = None;
        unit.on_wall = false;
        unit.set_width(width);
        Ok(())
    }

    /// French text of `error` naming the regiment by its display name
    /// (« Les Chevaliers doivent être placés… ») instead of its id.
    pub fn error_text(&self, error: &CommandError) -> String {
        let name = |id: &u32| self.unit(*id).map(|u| u.name.clone());
        let named = match error {
            CommandError::OutsideZone(id) => name(id)
                .map(|n| format!("Les {n} doivent être placés dans votre zone de déploiement")),
            CommandError::NotYours(id) => {
                name(id).map(|n| format!("Les {n} ne sont pas sous vos ordres"))
            }
            CommandError::Unavailable(id) => {
                name(id).map(|n| format!("Les {n} ne répondent plus (déroute ou hors du champ)"))
            }
            _ => None,
        };
        named.unwrap_or_else(|| error.to_string())
    }

    /// Ends the deployment phase: time runs from the next tick.
    pub fn start_battle(&mut self) -> Result<(), CommandError> {
        if !self.deploying {
            return Err(CommandError::NotDeploying);
        }
        self.deploying = false;
        self.log(
            "Les armées sont en place : la bataille commence !".to_owned(),
            None,
        );
        Ok(())
    }

    /// Valid deployment point for `side`: in the zone and, in a siege, on
    /// the right side of the walls and clear of them.
    pub(crate) fn deployable(&self, side: SideId, x: f64, z: f64) -> bool {
        if !x.is_finite()
            || !z.is_finite()
            || !self
                .deployment_zones(side)
                .iter()
                .any(|zone| zone.contains(x, z))
        {
            return false;
        }
        let Some(works) = &self.siege else {
            return true;
        };
        let clear = works
            .nearest_intact(x, z)
            .is_none_or(|(_, d)| d > works.band() + 2.0);
        match side {
            SideId::Attacker => !works.inside(x, z) && clear,
            SideId::Defender => works.inside(x, z),
        }
    }

    /// AI deployment by roles: the default layout (« Ligne de bataille »,
    /// CB6: foot line in the centre, shooters behind it, horse on the
    /// wings, engines behind) then the general's
    /// regiment behind the centre and, for a clearly weaker side of a field
    /// battle, the whole army shifted onto the best height of its zone.
    fn ai_deploy(&mut self, side: SideId) {
        let zone = self.deployment_zone(side);
        let back = if side == SideId::Attacker { -1.0 } else { 1.0 };
        let own: Vec<usize> = self.side_units(side, |u| u.present());
        let power = |s: SideId| -> f64 {
            self.units
                .iter()
                .filter(|u| u.side == s && u.present() && !u.synthetic)
                .map(crate::ai::unit_power)
                .sum()
        };
        // CV3-2: an entrenched camp keeps its line behind its palisade.
        let weaker = power(side) < 0.85 * power(side.other()) && !self.setup.side(side).entrenched;
        if self.siege.is_none() && weaker {
            let (cx, cz) = self.centroid(&own);
            let mut best = (0.0, self.field.height(cx, cz));
            for k in 1..=8 {
                let dz = back * f64::from(k) * 15.0;
                let (_, z) = zone.clamp(cx, cz + dz);
                let h = self.field.height(cx, z);
                if h > best.1 + 0.5 && !self.field.in_forest(cx, z) {
                    best = (z - cz, h);
                }
            }
            for &i in &own {
                let (x, z) = zone.clamp(self.units[i].x, self.units[i].z + best.0);
                self.units[i].x = x;
                self.units[i].z = z;
            }
        }
        let general = own
            .iter()
            .copied()
            .find(|&i| self.units[i].is_general && !self.units[i].on_wall);
        if let (Some(g), true, None) = (general, own.len() > 2, &self.siege) {
            // CB6: the general's place of « Ligne de bataille » (60 m
            // behind the centre of the army).
            let behind = -crate::group_formation::GroupFormationRules::bundled()
                .battle_line()
                .roles
                .as_ref()
                .map_or(-60.0, |r| r.general.depth_m);
            let (cx, cz) = self.centroid(&own);
            let (x, z) = zone.clamp(cx, cz + back * behind);
            self.units[g].x = x;
            self.units[g].z = z;
        }
        // F5d: never in deep water (fords excepted), whatever the shifts.
        let edge = if back < 0.0 { zone.z0 } else { zone.z1 };
        for &i in &own {
            let (x, z) = (self.units[i].x, self.units[i].z);
            let dry = crate::ai::dry_z(&self.field, x, z, edge, back);
            self.units[i].z = dry.clamp(zone.z0, zone.z1);
        }
    }

    fn centroid(&self, list: &[usize]) -> (f64, f64) {
        let n = list.len().max(1) as f64;
        list.iter().fold((0.0, 0.0), |(x, z), &i| {
            (x + self.units[i].x / n, z + self.units[i].z / n)
        })
    }

    /// `true` during the deployment phase.
    pub fn is_deploying(&self) -> bool {
        self.deploying
    }

    /// The deployment zone of `side` (CV3-2: the first flank of an
    /// ambusher; [`BattleSim::deployment_zones`] lists them all, and none for
    /// a side that cannot deploy).
    pub fn deployment_zone(&self, side: SideId) -> DeploymentZone {
        match &self.ambush {
            Some(layout) if layout.victim != side && !layout.zones.is_empty() => layout.zones[0],
            _ => self.standard_zone(side),
        }
    }

    /// The usual rectangle of `side` (own edge, or the siege bands).
    pub(super) fn standard_zone(&self, side: SideId) -> DeploymentZone {
        let (w, d) = (self.field.width, self.field.depth);
        let margin = 20.0;
        if let Some(works) = &self.siege {
            let xs = works.vertices.iter().map(|v| v.0);
            let zs = works.vertices.iter().map(|v| v.1);
            let (min_x, max_x) = xs.fold((f64::MAX, f64::MIN), |(a, b), x| (a.min(x), b.max(x)));
            let (min_z, max_z) = zs.fold((f64::MAX, f64::MIN), |(a, b), z| (a.min(z), b.max(z)));
            return match side {
                SideId::Attacker => DeploymentZone {
                    x0: margin,
                    z0: margin,
                    x1: w - margin,
                    z1: min_z - SIEGE_STANDOFF,
                },
                SideId::Defender => DeploymentZone {
                    x0: min_x,
                    z0: min_z,
                    x1: max_x,
                    z1: max_z,
                },
            };
        }
        let size = self.field.size;
        match side {
            SideId::Attacker => {
                let front = size.attacker_line_z() + 50.0;
                DeploymentZone {
                    x0: margin,
                    z0: (front - size.zone_depth).max(margin),
                    x1: w - margin,
                    z1: front,
                }
            }
            SideId::Defender => {
                let front = size.defender_line_z() - 50.0;
                DeploymentZone {
                    x0: margin,
                    z0: front,
                    x1: w - margin,
                    z1: (front + size.zone_depth).min(d - margin),
                }
            }
        }
    }
}
