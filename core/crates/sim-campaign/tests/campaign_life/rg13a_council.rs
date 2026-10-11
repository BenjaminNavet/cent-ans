//! Lot RG13a (ADR 0341): the council of the realm — automatic assignment,
//! appointments and dismissals, vacancies, the five effects, persistence.

use data_model::test_support::game_data;
use data_model::{CharacterId, EffectKind, FactionId, GameData, UnitCategory};
use sim_campaign::council::{self, CouncilError};
use sim_campaign::negotiation::{evaluate_treaty, Article};
use sim_campaign::test_support::{idle, start_quiet};
use sim_campaign::{CampaignState, EventKind, Order, OrderError};

const OFFICES: [&str; 5] = [
    "chancellor",
    "constable",
    "treasurer",
    "chaplain",
    "chamberlain",
];

fn fac(id: &str) -> FactionId {
    FactionId::new(id).unwrap()
}

fn france_state() -> (&'static GameData, CampaignState, FactionId) {
    let data = game_data();
    (data, start_quiet(data, "fac_france", 1), fac("fac_france"))
}

/// A living adult of `faction` who sits on no council seat.
fn spare_adult(state: &CampaignState, faction: &FactionId) -> CharacterId {
    council::candidates(state, game_data(), faction, "chancellor")
        .into_iter()
        .find(|c| c.current_office.is_none())
        .expect("a spare candidate")
        .character
}

fn appoint(
    state: &mut CampaignState,
    data: &GameData,
    office: &str,
    character: &CharacterId,
) -> Result<(), OrderError> {
    state.submit_order(
        data,
        Order::AppointCouncillor {
            office: office.to_owned(),
            character: character.clone(),
        },
    )
}

fn set_seat(
    state: &mut CampaignState,
    faction: &FactionId,
    office: &str,
    who: &CharacterId,
    skill: u8,
) {
    let c = state.characters.get_mut(who).unwrap();
    c.skills.court = skill;
    c.skills.command = skill;
    c.skills.governance = skill;
    state
        .factions
        .get_mut(faction)
        .unwrap()
        .council
        .holders
        .insert(office.to_owned(), who.clone());
}

// ----- automatic assignment ----------------------------------------------------

#[test]
fn a_new_game_seats_the_best_candidate_in_every_office() {
    let (data, state, france) = france_state();
    let ruler = state.factions[&france].ruler.clone().unwrap();
    let mut seen = Vec::new();
    for office in OFFICES {
        let holder = council::holder(&state, &france, office)
            .unwrap_or_else(|| panic!("{office} is seated"))
            .clone();
        assert_ne!(holder, ruler, "the ruler presides, he does not sit");
        assert!(!seen.contains(&holder), "one office per character");
        seen.push(holder);
    }
    // The first office of the file takes the best of its skill.
    let best = council::candidates(&state, data, &france, "chancellor");
    let chancellor = council::holder(&state, &france, "chancellor").unwrap();
    let top = best.iter().map(|c| c.skill).max().unwrap();
    assert_eq!(
        council::skill_of(&state, chancellor, data_model::SkillBranch::Court),
        top
    );
}

#[test]
fn the_assignment_is_deterministic() {
    let data = game_data();
    let a = start_quiet(data, "fac_france", 1);
    let b = start_quiet(data, "fac_france", 99);
    for faction in a.factions.keys() {
        assert_eq!(
            a.factions[faction].council.holders, b.factions[faction].council.holders,
            "{faction}"
        );
    }
}

// ----- orders ------------------------------------------------------------------

#[test]
fn appointment_rules_are_enforced() {
    let (data, mut state, france) = france_state();
    let ruler = state.factions[&france].ruler.clone().unwrap();
    assert!(matches!(
        appoint(&mut state, data, "chancellor", &ruler),
        Err(OrderError::Council(CouncilError::IsRuler))
    ));
    let english = state
        .characters
        .iter()
        .find(|(_, c)| c.faction == fac("fac_england") && c.alive)
        .map(|(id, _)| id.clone())
        .unwrap();
    assert!(matches!(
        appoint(&mut state, data, "chancellor", &english),
        Err(OrderError::Council(CouncilError::NotOfFaction))
    ));
    let spare = spare_adult(&state, &france);
    assert!(matches!(
        appoint(&mut state, data, "no_such_office", &spare),
        Err(OrderError::Council(CouncilError::UnknownOffice(_)))
    ));
    // A child cannot sit.
    state.characters.get_mut(&spare).unwrap().birth_year = state.year - 5;
    assert!(matches!(
        appoint(&mut state, data, "chancellor", &spare),
        Err(OrderError::Council(CouncilError::TooYoung))
    ));
    // A dead character is unknown.
    state.characters.get_mut(&spare).unwrap().alive = false;
    assert!(matches!(
        appoint(&mut state, data, "chancellor", &spare),
        Err(OrderError::Council(CouncilError::UnknownCharacter))
    ));
}

#[test]
fn one_office_per_character_and_replacement_costs_loyalty() {
    let (data, mut state, france) = france_state();
    let constable = council::holder(&state, &france, "constable")
        .unwrap()
        .clone();
    assert!(matches!(
        appoint(&mut state, data, "chancellor", &constable),
        Err(OrderError::Council(CouncilError::AlreadyHoldsOffice(_)))
    ));
    assert!(matches!(
        appoint(&mut state, data, "constable", &constable),
        Err(OrderError::Council(CouncilError::AlreadyHolder))
    ));
    let spare = spare_adult(&state, &france);
    let before = state.characters[&constable].loyalty;
    appoint(&mut state, data, "constable", &spare).expect("replacement");
    assert_eq!(council::holder(&state, &france, "constable"), Some(&spare));
    assert_eq!(state.characters[&constable].loyalty, before - 1);
    assert_eq!(council::office_of(&state, &france, &constable), None);
}

#[test]
fn dismissal_empties_the_seat() {
    let (data, mut state, france) = france_state();
    let holder = council::holder(&state, &france, "treasurer")
        .unwrap()
        .clone();
    let before = state.characters[&holder].loyalty;
    state
        .submit_order(
            data,
            Order::DismissCouncillor {
                office: "treasurer".into(),
            },
        )
        .expect("dismissal");
    assert_eq!(council::holder(&state, &france, "treasurer"), None);
    assert_eq!(state.characters[&holder].loyalty, before - 1);
    let again = state.submit_order(
        data,
        Order::DismissCouncillor {
            office: "treasurer".into(),
        },
    );
    assert!(matches!(
        again,
        Err(OrderError::Council(CouncilError::AlreadyVacant))
    ));
    // A dismissed seat stays empty for the player over a turn.
    state.end_turn_with(data, idle);
    assert_eq!(council::holder(&state, &france, "treasurer"), None);
}

// ----- vacancies ---------------------------------------------------------------

#[test]
fn a_dead_holder_leaves_a_vacancy_and_an_event() {
    let (data, mut state, france) = france_state();
    let holder = council::holder(&state, &france, "chaplain")
        .unwrap()
        .clone();
    state.characters.get_mut(&holder).unwrap().alive = false;
    assert_eq!(council::holder(&state, &france, "chaplain"), None);
    let events = state.end_turn_with(data, idle);
    assert!(events
        .iter()
        .any(|e| e.kind == EventKind::CouncilVacancy && e.faction.as_ref() == Some(&france)));
    assert!(!state.factions[&france]
        .council
        .holders
        .contains_key("chaplain"));
}

#[test]
fn ai_factions_refill_their_vacancies() {
    let (data, mut state, _) = france_state();
    let england = fac("fac_england");
    let holder = council::holder(&state, &england, "constable")
        .expect("England has a constable")
        .clone();
    state.characters.get_mut(&holder).unwrap().alive = false;
    state.end_turn_with(data, idle);
    let successor = council::holder(&state, &england, "constable");
    assert!(successor.is_some_and(|s| *s != holder));
}

// ----- effects -----------------------------------------------------------------

#[test]
fn the_treasurer_raises_the_tax_income() {
    let (data, mut state, france) = france_state();
    let who = spare_adult(&state, &france);
    state
        .factions
        .get_mut(&france)
        .unwrap()
        .council
        .holders
        .remove("treasurer");
    let without = state.faction_income(data, &france);
    set_seat(&mut state, &france, "treasurer", &who, 10);
    let with = state.faction_income(data, &france);
    assert!(with > without, "{with} <= {without}");
}

#[test]
fn the_chaplain_soothes_every_province() {
    let (data, mut state, france) = france_state();
    let who = spare_adult(&state, &france);
    let province = state
        .provinces
        .keys()
        .find(|p| state.holds_province(&france, p))
        .unwrap()
        .clone();
    let total = |state: &CampaignState| -> f64 {
        sim_campaign::population::unrest_breakdown(state, data, &province)
            .unwrap()
            .iter()
            .flat_map(|(_, terms)| terms.iter().map(|(_, v)| *v))
            .sum()
    };
    state
        .factions
        .get_mut(&france)
        .unwrap()
        .council
        .holders
        .remove("chaplain");
    let without = total(&state);
    set_seat(&mut state, &france, "chaplain", &who, 10);
    let with = total(&state);
    assert!(
        (without - with - 3.0 * 4.0).abs() < 1e-6,
        "four classes, -3 each: {without} -> {with}"
    );
}

#[test]
fn the_constable_trains_recruits_and_lifts_generals() {
    let (data, mut state, france) = france_state();
    let who = spare_adult(&state, &france);
    state
        .factions
        .get_mut(&france)
        .unwrap()
        .council
        .holders
        .remove("constable");
    let experience = |state: &CampaignState| {
        let effects = sim_campaign::research::faction_tech_effects(state, data, &france);
        sim_campaign::economy::recruit_experience(&effects, UnitCategory::Infantry)
    };
    let general = state
        .characters
        .iter()
        .find(|(_, c)| c.faction == france && c.alive && c.army.is_some())
        .map(|(id, _)| id.clone())
        .expect("a general");
    let morale = |state: &CampaignState| {
        sim_campaign::skills::character_effects(state, data, &general)[EffectKind::ArmyMorale].flat
    };
    let (xp_without, morale_without) = (experience(&state), morale(&state));
    set_seat(&mut state, &france, "constable", &who, 10);
    assert_eq!(experience(&state), xp_without + 2);
    assert!((morale(&state) - morale_without - 5.0).abs() < 1e-9);
}

#[test]
fn the_chancellor_adds_a_line_to_the_negotiation() {
    let (data, mut state, france) = france_state();
    let who = spare_adult(&state, &france);
    let england = fac("fac_england");
    let score = |state: &CampaignState| {
        let verdict = evaluate_treaty(state, data, &france, &england, &[Article::Peace]);
        let line = verdict
            .context
            .iter()
            .find(|(text, _)| text == "Chancelier")
            .map(|(_, points)| *points);
        (verdict.score, line)
    };
    state
        .factions
        .get_mut(&france)
        .unwrap()
        .council
        .holders
        .remove("chancellor");
    let (without, none) = score(&state);
    assert_eq!(none, None);
    set_seat(&mut state, &france, "chancellor", &who, 6);
    let (with, line) = score(&state);
    assert_eq!(line, Some(6));
    assert_eq!(with, without + 6);
}

#[test]
fn the_chamberlain_earns_prestige_each_season() {
    let (data, mut state, france) = france_state();
    let who = spare_adult(&state, &france);
    let ruler = state.factions[&france].ruler.clone().unwrap();
    set_seat(&mut state, &france, "chamberlain", &who, 10);
    let before = state.characters[&ruler].prestige;
    state.end_turn_with(data, idle);
    // 10 skill points × 0,1 = 1 prestige per season, on top of the ordinary
    // court prestige (none outside winter).
    assert_eq!(state.characters[&ruler].prestige, before + 1);
    // A fractional rate carries over.
    let mut without = state.clone();
    without
        .factions
        .get_mut(&france)
        .unwrap()
        .council
        .holders
        .remove("chamberlain");
    set_seat(&mut state, &france, "chamberlain", &who, 4);
    for _ in 0..3 {
        state.end_turn_with(data, idle);
        without.end_turn_with(data, idle);
    }
    let gained = state.characters[&ruler].prestige - without.characters[&ruler].prestige;
    assert!((1..=2).contains(&gained), "0,4 × 3 seasons: {gained}");
}

#[test]
fn effect_wording_is_french_and_data_driven() {
    let data = game_data();
    let offices = data.council.as_ref().unwrap();
    let treasurer = offices.office("treasurer").unwrap();
    assert_eq!(
        council::effect_text(treasurer, Some(7)),
        "+3,5 % de recettes fiscales"
    );
    assert_eq!(
        council::effect_text(treasurer, None),
        "+0,5 % de recettes fiscales par point de compétence"
    );
    let chaplain = offices.office("chaplain").unwrap();
    assert!(council::effect_text(chaplain, Some(5)).starts_with("−1,5 pt"));
}

// ----- persistence -------------------------------------------------------------

#[test]
fn the_council_survives_a_save_and_load() {
    let (data, state, france) = france_state();
    let loaded = CampaignState::load_json(&state.save_json()).expect("reloads");
    assert_eq!(
        loaded.factions[&france].council,
        state.factions[&france].council
    );
    assert!(council::holder(&loaded, &france, "constable").is_some());
    let _ = data;
}

#[test]
fn a_save_without_a_council_is_seated_on_load() {
    let (data, state, france) = france_state();
    let mut json: serde_json::Value = serde_json::from_str(&state.save_json()).unwrap();
    for faction in json["factions"].as_object_mut().unwrap().values_mut() {
        faction.as_object_mut().unwrap().remove("council");
    }
    let mut loaded = CampaignState::load_json(&json.to_string()).expect("old save loads");
    assert!(loaded.factions[&france].council.is_default());
    assert_eq!(council::holder(&loaded, &france, "constable"), None);
    council::ensure_councils(&mut loaded, data);
    assert_eq!(
        loaded.factions[&france].council.holders, state.factions[&france].council.holders,
        "deterministic: same seats as a fresh game"
    );
}
