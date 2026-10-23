//! Lot HIST: the 1337 spouses of the main courts are set, so the marriage rule
//! no longer pairs a married woman or man with someone else at turn 1.
use data_model::test_support::{fac, game_data};
use sim_campaign::CampaignState;

fn id(s: &str) -> data_model::CharacterId {
    data_model::CharacterId::new(s).unwrap()
}

#[test]
fn historical_couples_are_linked_in_1337() {
    let data = game_data();
    let state = CampaignState::new_1337(data, fac("fac_france"), 1).unwrap();
    for (a, b) in [
        ("chr_louis_de_nevers", "chr_marguerite_de_france_flandre"),
        ("chr_agnes_randolph", "chr_patrick_de_dunbar"),
        ("chr_ludwig_iv", "chr_marguerite_de_hainaut"),
        ("chr_jean_de_luxembourg", "chr_beatrice_de_bourbon"),
        ("chr_charles_de_luxembourg", "chr_blanche_de_valois"),
        ("chr_afonso_iv", "chr_beatriz_de_castilla"),
        ("chr_david_ii", "chr_joan_of_the_tower"),
    ] {
        assert_eq!(state.characters[&id(a)].spouse, Some(id(b)), "{a}");
        assert_eq!(state.characters[&id(b)].spouse, Some(id(a)), "{b}");
    }
    let candidates = state.marriage_candidates(data, &id("chr_raoul_de_lorraine"));
    assert!(!candidates.contains(&id("chr_agnes_randolph")));
}
