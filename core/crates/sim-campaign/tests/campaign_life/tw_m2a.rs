//! TW lot m2a: heir designation, mission counters (agents, marriages,
//! ransoms), unpaid-wage desertion.
use data_model::test_support::{fac, game_data};
use data_model::CharacterId;
use sim_campaign::test_support::start;
use sim_campaign::{CampaignState, Order, OrderError};

fn chr(id: &str) -> CharacterId {
    CharacterId::new(id).unwrap()
}

/// Adds an adult cousin of the French king's house (clone of the heir).
fn add_cousin(state: &mut CampaignState, id: &str, birth_year: i32) -> CharacterId {
    let jean = state.characters[&chr("chr_jean_de_normandie")].clone();
    let mut cousin = jean;
    cousin.birth_year = birth_year;
    cousin.father = None;
    cousin.mother = None;
    cousin.spouse = None;
    cousin.children.clear();
    cousin.army = None;
    cousin.governor_of = None;
    let id = chr(id);
    state.characters.insert(id.clone(), cousin);
    id
}

#[test]
fn designate_heir_succeeds_and_the_cousin_inherits() {
    let data = game_data();
    let mut state = start(data, "fac_france", 11);
    let cousin = add_cousin(&mut state, "chr_test_cousin", 1310);
    let france = fac("fac_france");
    state.factions.get_mut(&france).unwrap().treasury = 10_000;
    let king = chr("chr_philippe_vi");
    let prestige_before = state.characters[&king].prestige;
    state
        .submit_order(
            data,
            Order::DesignateHeir {
                heir: cousin.clone(),
            },
        )
        .unwrap();
    assert_eq!(state.factions[&france].heir, Some(cousin.clone()));
    assert_eq!(state.factions[&france].treasury, 10_000 - 100);
    assert_eq!(state.characters[&king].prestige, prestige_before - 10);
    let mut events = Vec::new();
    sim_campaign::characters::kill(&mut state, data, &king, &mut events);
    assert_eq!(state.factions[&france].ruler, Some(cousin));
}

#[test]
fn designate_heir_by_law_costs_no_prestige() {
    let data = game_data();
    let mut state = start(data, "fac_france", 11);
    let france = fac("fac_france");
    state.factions.get_mut(&france).unwrap().treasury = 10_000;
    let king = chr("chr_philippe_vi");
    let prestige_before = state.characters[&king].prestige;
    state
        .submit_order(
            data,
            Order::DesignateHeir {
                heir: chr("chr_jean_de_normandie"),
            },
        )
        .unwrap();
    assert_eq!(state.characters[&king].prestige, prestige_before);
}

#[test]
fn designate_heir_refuses_outsiders_ruler_minors_and_poverty() {
    let data = game_data();
    let mut state = start(data, "fac_france", 11);
    let france = fac("fac_france");
    state.factions.get_mut(&france).unwrap().treasury = 10_000;
    let year = state.year;
    let minor = add_cousin(&mut state, "chr_test_minor", year - 3);
    let cousin = add_cousin(&mut state, "chr_test_cousin", 1310);
    let outsider = state
        .characters
        .iter()
        .find(|(_, c)| c.faction != france && c.alive)
        .map(|(id, _)| id.clone())
        .unwrap();
    for refused in [chr("chr_philippe_vi"), minor, outsider] {
        let result = state.submit_order(data, Order::DesignateHeir { heir: refused });
        assert!(matches!(result, Err(OrderError::Heir(_))), "{result:?}");
    }
    state.factions.get_mut(&france).unwrap().treasury = 5;
    let result = state.submit_order(data, Order::DesignateHeir { heir: cousin });
    assert!(matches!(result, Err(OrderError::Heir(_))), "{result:?}");
}
