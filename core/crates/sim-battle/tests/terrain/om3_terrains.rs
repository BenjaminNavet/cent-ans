//! Lot OM3 (ADR 0116): steppe and desert battlefields reuse the plains
//! ground (open, treeless), with their own landscape profiles and river
//! widths from the data.

use data_model::Terrain;
use sim_battle::{
    BattleRng, BattleSeason, Battlefield, DecorRules, FieldSite, WaterRules, Weather,
};

fn site(terrain: Terrain) -> FieldSite {
    FieldSite {
        terrain,
        river: false,
        coastal: false,
        season: BattleSeason::Summer,
    }
}

#[test]
fn bundled_rules_know_the_eastern_terrains() {
    let decor = DecorRules::bundled();
    assert_eq!(decor.terrain_profiles.of(Terrain::Steppe), "steppe");
    assert_eq!(decor.terrain_profiles.of(Terrain::Desert), "desert");
    assert!(decor.profiles.contains_key("steppe"));
    assert!(decor.profiles.contains_key("desert"));
    let water = WaterRules::bundled();
    let steppe = water.river.width_m.of(Terrain::Steppe);
    let desert = water.river.width_m.of(Terrain::Desert);
    assert!(steppe[0] > 0.0 && steppe[0] <= steppe[1]);
    assert!(desert[0] > 0.0 && desert[0] <= desert[1]);
}

#[test]
fn steppe_and_desert_fields_are_open_and_deterministic() {
    for terrain in [Terrain::Steppe, Terrain::Desert] {
        for seed in [7_u64, 1234, 99] {
            let a = Battlefield::generate_site(
                &site(terrain),
                Weather::Clear,
                &mut BattleRng::from_seed(seed),
            );
            let b = Battlefield::generate_site(
                &site(terrain),
                Weather::Clear,
                &mut BattleRng::from_seed(seed),
            );
            assert_eq!(a, b, "{terrain:?} seed {seed}");
            assert_eq!(a.terrain, terrain);
            assert!(a.forests.is_empty(), "{terrain:?} seed {seed}: no woods");
            assert!(a.mud.is_empty(), "{terrain:?} seed {seed}: dry ground");
        }
    }
}
