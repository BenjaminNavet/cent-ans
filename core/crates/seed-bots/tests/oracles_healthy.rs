//! No false positive: a sound game played by the strategic AI raises no
//! finding from the invariant and save round-trip oracles.
//!
//! Slow unoptimised; run with `cargo test -p seed-bots --release -- --ignored`.

use data_model::{FactionId, GameData};
use seed_bots::oracles::{invariants, save_roundtrip, OracleContext, Origin};
use sim_campaign::CampaignState;

#[test]
#[ignore = "plays 20 turns, slow without --release"]
fn healthy_game_raises_no_finding() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../data");
    let (data, _) = GameData::load(&root).expect("data");
    for raw in ["fac_france", "fac_england", "fac_burgundy"] {
        let player = FactionId::new(raw).unwrap();
        let mut state = CampaignState::new_1337(&data, player.clone(), 1337).expect("state");
        for turn in 0..20u32 {
            let ctx = OracleContext {
                state: &state,
                data: &data,
                faction: &player,
                turn,
            };
            let origin = Origin::for_context(&ctx, 1337, "mirror", Vec::new());
            let findings = invariants::check(&ctx, &origin);
            assert!(findings.is_empty(), "{raw} tour {turn} : {findings:#?}");
            if turn % 10 == 0 {
                assert_eq!(save_roundtrip::check(&state, &origin), None, "{raw} {turn}");
            }
            for order in ai::plan_turn(&state, &data, &player) {
                let _ = state.submit_order(&data, order);
            }
            state.end_turn_with(&data, ai::plan_turn);
        }
    }
}
