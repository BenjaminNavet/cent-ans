//! Visual level of a settlement, port of `SettlementGrowth.level_of` (`settlement_growth.gd`,
//! rendering only): 0 village, 1 bourg (open, hall), 2 ville (walls), 3 cité (cathedral, hall,
//! castle, suburbs). From the kind, the buildings and the fortification of the place and the
//! population of its province:
//! - village → bourg: market built or very populous province;
//! - bourg → ville: walls (`bld_stone_walls`, `bld_palisade`, `bld_walls`) or fortification ≥ 2;
//! - ville → cité: episcopal city (cathedral) or very populous provincial capital;
//! - cité: castle as well when fortification ≥ 6.
//!
//! Castles and abbeys have no level; Paris (dedicated landmark city) is excluded. The CV1
//! maquettes of the Godot file (`build_model`, Kenney castle) are replaced by the maquettes of the
//! markers lot (ADR 0158); only the level is ported, for the hover text, the pilgrims and the dump.

pub const LEVEL_NAMES: [&str; 4] = ["village", "bourg", "ville", "cité"];
const EXCLUDED: [&str; 1] = ["set_paris"];
const WALL_BUILDINGS: [&str; 3] = ["bld_stone_walls", "bld_palisade", "bld_walls"];
pub const BOURG_POPULATION: f32 = 420_000.0;
pub const CITE_POPULATION: f32 = 600_000.0;
pub const CASTLE_FORTIFICATION: u32 = 6;

/// Visual level of a settlement, `None` without one (castle, abbey) or when excluded.
/// `is_city`: the place is the capital city of its province.
pub fn level_of(
    id: &str,
    kind: &str,
    buildings: &[&str],
    fortification: u32,
    is_city: bool,
    province_population: f32,
) -> Option<u8> {
    if EXCLUDED.contains(&id) {
        return None;
    }
    let has = |b: &str| buildings.contains(&b);
    let walled = fortification >= 2 || WALL_BUILDINGS.iter().any(|b| has(b));
    match kind {
        "village" => Some(u8::from(
            has("bld_market") || province_population >= BOURG_POPULATION,
        )),
        "town" => Some(if walled { 2 } else { 1 }),
        "city" => Some(
            if has("bld_cathedral") || (is_city && province_population >= CITE_POPULATION) {
                3
            } else if walled {
                2
            } else {
                1
            },
        ),
        _ => None,
    }
}

/// A castle stands beside the cité.
pub fn has_castle(fortification: u32) -> bool {
    fortification >= CASTLE_FORTIFICATION
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn levels() {
        assert_eq!(level_of("a", "village", &[], 0, false, 1.0), Some(0));
        assert_eq!(
            level_of("a", "village", &["bld_market"], 0, false, 1.0),
            Some(1)
        );
        assert_eq!(level_of("a", "town", &[], 0, false, 1.0), Some(1));
        assert_eq!(
            level_of("a", "town", &["bld_palisade"], 0, false, 1.0),
            Some(2)
        );
        assert_eq!(level_of("a", "city", &[], 2, false, 1.0), Some(2));
        assert_eq!(level_of("a", "city", &[], 0, true, 700_000.0), Some(3));
        assert_eq!(
            level_of("a", "city", &["bld_cathedral"], 0, false, 1.0),
            Some(3)
        );
        assert_eq!(level_of("a", "castle", &[], 9, false, 1.0), None);
        assert_eq!(level_of("set_paris", "city", &[], 9, true, 1e7), None);
        assert!(has_castle(6) && !has_castle(5));
    }
}
