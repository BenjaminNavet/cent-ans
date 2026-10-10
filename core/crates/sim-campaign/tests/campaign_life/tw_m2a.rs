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

// ----- mission counters ---------------------------------------------------------

use data_model::MissionCounter;
use sim_campaign::missions::Mission;

/// A `count` mission of the bundled template `template`, given to the player.
fn give_mission(state: &mut CampaignState, template: &str) {
    let data = game_data();
    let t = data
        .mission_rules
        .templates
        .iter()
        .find(|t| t.id == template)
        .unwrap();
    state.missions.faction = Some(state.player_faction.clone());
    state.missions.last_closed_turn = Some(state.turn);
    state.missions.active = vec![Mission {
        id: 77,
        template: template.to_owned(),
        goal: t.goal,
        counter: t.counter,
        title: "Essai".to_owned(),
        objective: "Essai.".to_owned(),
        province: None,
        settlement: None,
        building: None,
        count: 3,
        progress: 0,
        issued_turn: state.turn,
        deadline_turn: state.turn + 10,
        reward: t.reward.clone(),
        baseline: Vec::new(),
    }];
}

#[test]
fn the_new_templates_use_the_new_counters() {
    let data = game_data();
    for (id, counter) in [
        ("agent_work", MissionCounter::AgentActions),
        ("royal_match", MissionCounter::Marriages),
        ("ransom_trade", MissionCounter::Ransoms),
    ] {
        let t = data
            .mission_rules
            .templates
            .iter()
            .find(|t| t.id == id)
            .unwrap();
        assert_eq!(t.counter, Some(counter));
    }
}

#[test]
fn a_marriage_of_the_player_counts_for_its_mission() {
    let data = game_data();
    let mut state = start(data, "fac_france", 11);
    give_mission(&mut state, "royal_match");
    let jean = chr("chr_jean_de_normandie");
    let mut bride = state.characters[&jean].clone();
    bride.sex = data_model::Sex::Female;
    bride.father = None;
    bride.mother = None;
    bride.spouse = None;
    bride.children.clear();
    bride.army = None;
    bride.governor_of = None;
    state.characters.insert(chr("chr_test_bride"), bride);
    state.characters.get_mut(&jean).unwrap().spouse = None;
    state
        .submit_order(
            data,
            Order::ProposeMarriage {
                character: jean,
                spouse: chr("chr_test_bride"),
            },
        )
        .unwrap();
    assert_eq!(state.missions.active[0].progress, 1);
}

#[test]
fn a_ransom_paid_by_the_player_counts_for_its_mission() {
    let data = game_data();
    let mut state = start(data, "fac_france", 11);
    give_mission(&mut state, "ransom_trade");
    let jean = chr("chr_jean_de_normandie");
    {
        let c = state.characters.get_mut(&jean).unwrap();
        c.captive = true;
        c.captor = Some(fac("fac_england"));
        c.army = None;
    }
    state.factions.get_mut(&fac("fac_france")).unwrap().treasury = 10_000;
    let mut events = Vec::new();
    sim_campaign::chronicle::release_character(&mut state, data, &jean, 500, &mut events);
    assert_eq!(state.missions.active[0].progress, 1);
    // A release without ransom does not count.
    let c = state.characters.get_mut(&jean).unwrap();
    c.captive = true;
    c.captor = Some(fac("fac_england"));
    sim_campaign::chronicle::release_character(&mut state, data, &jean, 0, &mut events);
    assert_eq!(state.missions.active[0].progress, 1);
}
