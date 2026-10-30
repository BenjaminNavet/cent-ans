//! IA-REFUS: the campaign AI must not propose orders the core refuses
//! (a disbanded army still given a general, a feudal contingent merged, a
//! march leg past the movement left...). A handful of refusals stay
//! tolerated (funds spent by an earlier order of the same plan).
//!
//! Slow unoptimised; run with `cargo test -p seed-bots --release -- --ignored`.

use data_model::{FactionId, GameData};
use sim_campaign::CampaignState;

/// Refusals tolerated over 20 turns of the whole map (the seed-bots band is
/// [0 ; 40] over 150 turns; before the fix this was 150+ in 20 turns).
const MAX_REFUSALS: u32 = 30;

#[test]
#[ignore = "plays 20 turns, slow without --release"]
fn ai_campaign_rarely_proposes_refused_orders() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../data");
    let (data, _) = GameData::load(&root).expect("data");
    let player = FactionId::new("fac_france").unwrap();
    let mut state = CampaignState::new_1337(&data, player.clone(), 1337).expect("state");
    for _ in 0..20u32 {
        for order in ai::plan_turn(&state, &data, &player) {
            let _ = state.submit_order(&data, order);
        }
        state.end_turn_with(&data, ai::plan_turn);
    }
    let refused = state.ai_refused_orders();
    assert!(
        refused <= MAX_REFUSALS,
        "{refused} refused AI orders: {:#?} {:#?}",
        state.ai_refusal_kinds(),
        state.ai_refused_order_samples()
    );
}
