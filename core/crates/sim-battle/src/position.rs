//! Worth of a defensive position (lot R4, ADR 0046): one score combining the
//! relief read by [`crate::relief_ai`] and the site of B5.
//!
//! - **height**: ground above the reference (the deployment line, or the
//!   attacker looking at the position), half the local prominence (a true
//!   crest), minus a penalty on a scarp too steep to stand on in order;
//! - **glacis**: the ground the enemy must climb in front (up to 15 m over
//!   100 m);
//! - **reverse slope**: a spot behind the front masked from an enemy 150 m
//!   in front (the line hides there from crossbows and, since R4, from
//!   longbows, which may only lob at it blindly);
//! - **cover**: a hedge, a ditch or a fence just in front of the front (the
//!   shooters stand behind it), or the houses of a village;
//! - **flanks**: each wing leaning on a wood, a river or a pool, a marsh or
//!   a scarp (Crécy, Agincourt: woods on the wings);
//! - **field of fire**: the share of the ground in front the front sees
//!   (dead ground below a rounded crest hides the enemy from the archers);
//!   on a bare crest the shooters stand on its **military crest**
//!   ([`military_crest`]), down the forward slope where they see the glacis.
//!
//! All parts are in metres-equivalent points, the unit of the R2b height
//! search, so that `total()` compares a bare crest, a hedge in a hollow and a
//! hedge on a crest. Pure and deterministic.

use crate::field::Battlefield;
use crate::relief_ai::ReliefMap;
use crate::site::{Obstacle, ObstacleKind, HEDGE_COVER_REACH};

/// A full front of hedge (or the houses of a village) is worth this many
/// metres of height.
pub const COVER_POINTS: f64 = 14.0;
/// A front that sees all the ground in front of it (field of fire).
pub const FIRE_POINTS: f64 = 6.0;
/// Each wing leaning on impassable or slow ground.
pub const FLANK_POINTS: f64 = 4.0;
/// A reverse slope behind the front.
pub const REVERSE_POINTS: f64 = 3.0;
/// Depth of the glacis read in front of the front (as R2b).
pub const GLACIS_DEPTH: f64 = 100.0;
/// A line stands in order on slopes up to this grade (as R2b).
pub const STAND_SLOPE: f64 = 0.15;
/// A slope this steep covers a wing (horse and foot cannot go round it in
/// order).
pub const SCARP_SLOPE: f64 = 0.35;
/// A wing leans on ground within this distance past its end...
pub const FLANK_NEAR: f64 = 30.0;
/// ... and half as well up to this distance.
pub const FLANK_FAR: f64 = 60.0;
/// The line stands at most this far behind the front on the reverse slope.
pub const REVERSE_REACH: f64 = 30.0;
/// Distance of the enemy watching the reverse slope.
pub const REVERSE_WATCH: f64 = 150.0;

/// Parts of the score of a front (metres-equivalent points).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct PositionScore {
    pub height: f64,
    pub glacis: f64,
    pub reverse: f64,
    pub cover: f64,
    pub flanks: f64,
    /// Share of the ground in front the front sees (shooters' field of fire).
    pub fire: f64,
}

impl PositionScore {
    pub fn total(&self) -> f64 {
        self.height + self.glacis + self.reverse + self.cover + self.flanks + self.fire
    }

    /// The score without the cover (a spot chosen for its ground only).
    pub fn ground(&self) -> f64 {
        self.total() - self.cover
    }
}

/// A front to score: `width` metres wide, centred on `center`, facing
/// `forward` (+1 towards +z).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Front {
    pub center: (f64, f64),
    pub forward: f64,
    pub width: f64,
}

/// Weight of an obstacle as cover (B6: a fence neither covers nor breaks a
/// charge, but slows the enemy a little).
pub fn obstacle_weight(kind: ObstacleKind) -> f64 {
    match kind {
        ObstacleKind::Hedge => 1.0,
        ObstacleKind::Ditch => 0.8,
        ObstacleKind::Fence => 0.45,
        ObstacleKind::Palisade => 1.0,
    }
}

/// Score of `front`, heights counted from `base` (metres).
///
/// `reverse`: count the reverse slope (the enemy has shooters).
pub fn score_position(
    field: &Battlefield,
    map: &ReliefMap,
    front: Front,
    base: f64,
    reverse: bool,
) -> PositionScore {
    let (x, z) = front.center;
    let h = field.height(x, z);
    let scarp = (ReliefMap::slope(field, x, z) - STAND_SLOPE).max(0.0) * 60.0;
    PositionScore {
        height: h - base + 0.5 * map.prominence(x, z).max(0.0) - scarp,
        glacis: 0.3 * (h - field.height(x, z + front.forward * GLACIS_DEPTH)).clamp(0.0, 15.0),
        reverse: if reverse && reverse_slope(field, front).is_some() {
            REVERSE_POINTS
        } else {
            0.0
        },
        cover: cover_points(field, front),
        flanks: flank_points(field, front),
        fire: fire_points(field, front),
    }
}

/// Points of the field of fire: [`FIRE_POINTS`] x the share of the glacis
/// (every [`DEAD_STEP`] m from [`DEAD_NEAR`] to [`FIRE_REACH`] m in front,
/// centre and both thirds of the front) seen from the front. A crest with
/// dead ground below it is a poor post for archers, who must see what they
/// shoot (ADR 0046).
pub fn fire_points(field: &Battlefield, front: Front) -> f64 {
    FIRE_POINTS * (1.0 - dead_ground(field, front))
}

/// A front sees the ground from this far in front of it...
pub const DEAD_NEAR: f64 = 30.0;
/// ... to this far (the useful range of a longbow volley aimed at sight).
pub const FIRE_REACH: f64 = 180.0;
/// Sampling step of the glacis.
pub const DEAD_STEP: f64 = 15.0;
/// A military crest leaves at most this share of the glacis in dead ground.
pub const DEAD_TOLERANCE: f64 = 0.1;
/// The military crest is looked for this far in front of the crest at most.
pub const MILITARY_REACH: f64 = 80.0;
/// Shooters stand in order on slopes up to this grade.
pub const SHOOTER_SLOPE: f64 = 0.3;

/// Share of the glacis in front of `front` masked from it by the ground
/// (dead ground: the enemy climbs there unseen and unshot).
pub fn dead_ground(field: &Battlefield, front: Front) -> f64 {
    let (x, z) = front.center;
    let third = front.width / 3.0;
    let mut dead = 0;
    let mut total = 0;
    let mut d = DEAD_NEAR;
    while d <= FIRE_REACH {
        for lateral in [-third, 0.0, third] {
            let spot = (x + lateral, z + front.forward * d);
            if !field.inside(spot.0, spot.1) {
                continue;
            }
            total += 1;
            if field.blocks_sight((x + lateral, z), spot) {
                dead += 1;
            }
        }
        d += DEAD_STEP;
    }
    if total == 0 {
        0.0
    } else {
        f64::from(dead) / f64::from(total)
    }
}

/// The **military crest** of `front` (R4, ADR 0046): the front itself when
/// it sees its glacis, otherwise the highest standable spot down the
/// forward slope (every 5 m up to [`MILITARY_REACH`]) from which at most
/// [`DEAD_TOLERANCE`] of the glacis is dead ground; failing that, the spot
/// leaving the least dead ground. The topographic crest of a rounded hill
/// hides the foot of its slope; archers stand lower, where they see it.
pub fn military_crest(field: &Battlefield, front: Front) -> (f64, f64) {
    let at = |center: (f64, f64)| dead_ground(field, Front { center, ..front });
    let crest = front.center;
    let mut best = (crest, at(crest));
    if best.1 <= DEAD_TOLERANCE {
        return crest;
    }
    let mut ahead = 5.0;
    while ahead <= MILITARY_REACH {
        let spot = (crest.0, crest.1 + front.forward * ahead);
        ahead += 5.0;
        if !field.inside(spot.0, spot.1)
            || field.in_forest(spot.0, spot.1)
            || field.in_mud(spot.0, spot.1)
            || field.water_at(spot.0, spot.1).is_some()
            || ReliefMap::slope(field, spot.0, spot.1) > SHOOTER_SLOPE
        {
            continue;
        }
        let dead = at(spot);
        if dead <= DEAD_TOLERANCE {
            return spot;
        }
        if dead < best.1 {
            best = (spot, dead);
        }
    }
    best.0
}

/// Spot on the reverse slope behind `front` (standable), if any.
pub fn reverse_slope(field: &Battlefield, front: Front) -> Option<(f64, f64)> {
    ReliefMap::reverse_slope(
        field,
        front.center,
        front.forward,
        REVERSE_WATCH,
        REVERSE_REACH,
    )
    .filter(|&(x, z)| !field.in_forest(x, z) && !field.in_mud(x, z))
}

/// Cover of the front: the houses of a village it stands in, or the best
/// hedge, ditch or fence running just in front of it (within
/// [`HEDGE_COVER_REACH`]), by the share of the front it covers.
pub fn cover_points(field: &Battlefield, front: Front) -> f64 {
    let (x, z) = front.center;
    let ahead = (x, z + front.forward * HEDGE_COVER_REACH);
    let width = front.width.max(60.0);
    field
        .obstacles
        .iter()
        .filter(|o| o.crosses(front.center, ahead))
        .map(|o| obstacle_weight(o.kind) * COVER_POINTS * covered_share(o, front, width))
        .fold(0.0, f64::max)
}

/// Share of the front (`width` wide) the obstacle runs across.
fn covered_share(o: &Obstacle, front: Front, width: f64) -> f64 {
    let (lo, hi) = (o.a.0.min(o.b.0), o.a.0.max(o.b.0));
    let (left, right) = (front.center.0 - width * 0.5, front.center.0 + width * 0.5);
    ((hi.min(right) - lo.max(left)) / width).clamp(0.0, 1.0)
}

/// Ground no formed enemy goes round or through quickly.
fn anchors(field: &Battlefield, x: f64, z: f64) -> bool {
    field.inside(x, z)
        && (field.in_forest(x, z)
            || field.in_mud(x, z)
            || field.water_at(x, z).is_some()
            || ReliefMap::slope(field, x, z) > SCARP_SLOPE)
}

/// Wings leaning on a wood, water, a marsh or a scarp: [`FLANK_POINTS`] each
/// within [`FLANK_NEAR`] of the end of the front, half up to [`FLANK_FAR`].
pub fn flank_points(field: &Battlefield, front: Front) -> f64 {
    let (x, z) = front.center;
    let half = front.width * 0.5;
    [-1.0, 1.0]
        .into_iter()
        .map(|side: f64| {
            let near = [10.0, 20.0, FLANK_NEAR]
                .iter()
                .any(|&d| anchors(field, x + side * (half + d), z));
            let far = [45.0, FLANK_FAR]
                .iter()
                .any(|&d| anchors(field, x + side * (half + d), z));
            if near {
                FLANK_POINTS
            } else if far {
                FLANK_POINTS * 0.5
            } else {
                0.0
            }
        })
        .sum()
}
