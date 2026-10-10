//! Lot TW `ai-camp` (ADR 0334): the AI names a clearly better heir and buys
//! back the Church's grace when excommunicated or interdicted.

use data_model::test_support::{fac, game_data};
use data_model::CharacterId;
use sim_campaign::diplomacy::plan_diplomacy;
use sim_campaign::dynasty::ai_designate_heir;
use sim_campaign::plan_cache::PlanCache;
use sim_campaign::test_support::start;
use sim_campaign::{CampaignState, Order};

fn chr(id: &str) -> CharacterId {
    CharacterId::new(id).unwrap()
}

fn add_cousin(state: &mut CampaignState, id: &str, skill: u8) -> CharacterId {
    let mut cousin = state.characters[&chr("chr_jean_de_normandie")].clone();
    cousin.birth_year = 1310;
    cousin.father = None;
    cousin.mother = None;
    cousin.spouse = None;
    cousin.children.clear();
    cousin.army = None;
    cousin.governor_of = None;
    cousin.skills.command = skill;
    cousin.skills.governance = skill;
    cousin.skills.court = skill;
    let id = chr(id);
    state.characters.insert(id.clone(), cousin);
    id
}

#[test]
fn the_ai_names_a_much_better_cousin_as_heir_but_not_a_slightly_better_one() {
    let data = game_data();
    let mut state = start(data, "fac_france", 11);
    let france = fac("fac_france");
    state.factions.get_mut(&france).unwrap().treasury = 10_000;
    let heir = state.factions[&france].heir.clone().unwrap();
    {
        let c = state.characters.get_mut(&heir).unwrap();
        c.skills.command = 20;
        c.skills.governance = 20;
        c.skills.court = 20;
    }
    let gap = sim_campaign::dynasty::rules().ai_heir_min_skill_gap;
    // Just under the gap: nothing.
    add_cousin(&mut state, "chr_near", 20 + (gap / 3 - 1) as u8);
    assert_eq!(ai_designate_heir(&state, data, &france), None);
    // Clearly better: designated.
    let star = add_cousin(&mut state, "chr_star", 20 + (gap / 3 + 1) as u8);
    assert_eq!(
        ai_designate_heir(&state, data, &france),
        Some(Order::DesignateHeir { heir: star.clone() })
    );
    // Once named, the order is not repeated.
    state
        .submit_order(data, Order::DesignateHeir { heir: star })
        .unwrap();
    assert_eq!(ai_designate_heir(&state, data, &france), None);
    // A poor treasury does not name anybody.
    state.factions.get_mut(&france).unwrap().heir = Some(heir);
    state.factions.get_mut(&france).unwrap().treasury = 50;
    assert_eq!(ai_designate_heir(&state, data, &france), None);
}

fn donation(state: &CampaignState, faction: &str) -> Option<i64> {
    let data = game_data();
    let cache = PlanCache::new(state);
    plan_diplomacy(&cache, data, &fac(faction))
        .into_iter()
        .find_map(|o| match o {
            Order::DonateToChurch { amount } => Some(amount),
            _ => None,
        })
}

#[test]
fn an_interdicted_or_excommunicated_ai_pays_the_church_when_it_can() {
    let data = game_data();
    let rules = data.religion_rules.as_ref().unwrap();
    let mut state = start(data, "fac_france", 12);
    let aragon = fac("fac_aragon");
    state.turn = 8;
    assert_eq!(donation(&state, "fac_aragon"), None);
    // Interdict alone: the lifting gift, once the reserve is there.
    state.factions.get_mut(&aragon).unwrap().interdict_until = Some(state.turn + 10);
    state.factions.get_mut(&aragon).unwrap().treasury = 100;
    assert_eq!(donation(&state, "fac_aragon"), None);
    let rec = rules.ai_reconcile.as_ref().unwrap();
    state.factions.get_mut(&aragon).unwrap().treasury = rules.interdict.lift_donation + rec.reserve;
    assert_eq!(
        donation(&state, "fac_aragon"),
        Some(rules.interdict.lift_donation)
    );
    // Excommunicated too: the larger of the two gifts.
    state
        .factions
        .get_mut(&aragon)
        .unwrap()
        .excommunicated_until = Some(state.turn + 10);
    state.factions.get_mut(&aragon).unwrap().treasury = 100_000;
    assert_eq!(
        donation(&state, "fac_aragon"),
        Some(
            rec.excommunication_donation
                .max(rules.interdict.lift_donation)
        )
    );
}
