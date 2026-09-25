//! Worth of a defensive position (lot R4, ADR 0046): one score combining the
//! relief read by [`crate::relief_ai`] (height, glacis, reverse slope) and
//! the site of B5 (hedges, ditches, village, wood edges; flanks leaning on a
//! wood, a river, a marsh or a scarp).

use crate::field::Battlefield;
use crate::relief_ai::ReliefMap;

/// Parts of the score of a front (metres-equivalent points).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct PositionScore {
    pub height: f64,
    pub glacis: f64,
    pub reverse: f64,
    pub cover: f64,
    pub flanks: f64,
}

impl PositionScore {
    pub fn total(&self) -> f64 {
        self.height + self.glacis + self.reverse + self.cover + self.flanks
    }
}

/// Score of a front `width` metres wide centred on `center`, facing
/// `forward` (+1 towards +z).
pub fn score_position(
    _field: &Battlefield,
    _map: &ReliefMap,
    _center: (f64, f64),
    _forward: f64,
    _width: f64,
) -> PositionScore {
    PositionScore::default()
}
