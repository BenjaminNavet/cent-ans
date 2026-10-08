//! Lot DF1 (ADR 0037): campaign difficulty levels. Each lever moves the
//! right way in easy / hard, the level survives a save, an old save loads
//! as normal, and the level is frozen once the first turn is played.

use data_model::{FactionId, GameData};
use sim_campaign::difficulty::{effect_summary, Difficulty};
use sim_campaign::{ArmyId, CampaignState};

use data_model::test_support::{fac, game_data};

fn campaign(data: &GameData, level: Difficulty) -> CampaignState {
    let mut state = CampaignState::new_1337(data, fac("fac_france"), 11).expect("1337 start");
    state.chronicle.disabled = true;
    assert!(state.set_difficulty(level));
    state
}

fn first_army(state: &CampaignState, faction: &str) -> ArmyId {
    state
        .armies
        .iter()
        .find(|(_, a)| a.faction == fac(faction))
        .map(|(id, _)| id.clone())
        .unwrap()
}

fn at_war(state: &mut CampaignState) {
    for (a, b) in [("fac_france", "fac_england"), ("fac_england", "fac_france")] {
        state
            .factions
            .get_mut(&fac(a))
            .unwrap()
            .at_war_with
            .insert(fac(b));
    }
}

/// The three levels compared by most tests.
fn levels(data: &GameData) -> [CampaignState; 3] {
    [
        campaign(data, Difficulty::Easy),
        campaign(data, Difficulty::Normal),
        campaign(data, Difficulty::Hard),
    ]
}

#[test]
fn data_defines_four_levels_and_normal_is_neutral() {
    let data = game_data();
    let ids: Vec<&str> = data
        .difficulty
        .levels
        .iter()
        .map(|l| l.id.as_str())
        .collect();
    assert_eq!(ids, ["easy", "normal", "hard", "very_hard"]);
    for id in ids {
        assert!(Difficulty::from_id(id).is_some(), "{id}");
    }
    assert!(data.difficulty.modifiers("normal").is_neutral());
    assert_eq!(data.difficulty.default, "normal");
    assert_eq!(
        effect_summary(&data.difficulty.modifiers("normal")),
        ["Aucun modificateur."]
    );
    assert!(effect_summary(&data.difficulty.modifiers("very_hard")).len() >= 6);
}

#[test]
fn income_follows_the_level() {
    let data = game_data();
    let [easy, normal, hard] = levels(data);
    let ai = |s: &CampaignState| s.faction_income_effective(data, &fac("fac_england"));
    let player = |s: &CampaignState| s.faction_income_effective(data, &fac("fac_france"));
    assert!(ai(&easy) < ai(&normal) && ai(&normal) < ai(&hard));
    assert!(player(&easy) > player(&normal));
    assert_eq!(player(&hard), player(&normal));
    let very_hard = campaign(data, Difficulty::VeryHard);
    assert!(player(&very_hard) < player(&normal));
}

#[test]
fn ai_upkeep_and_recruitment_are_cheaper_when_hard() {
    let data = game_data();
    let [easy, normal, hard] = levels(data);
    let england = fac("fac_england");
    let france = fac("fac_france");
    let upkeep = |s: &CampaignState, f: &FactionId| s.faction_army_upkeep(data, f);
    assert!(upkeep(&hard, &england) < upkeep(&normal, &england));
    assert_eq!(upkeep(&easy, &england), upkeep(&normal, &england));
    assert_eq!(upkeep(&hard, &france), upkeep(&normal, &france));

    let very_hard = campaign(data, Difficulty::VeryHard);
    let settlement = normal
        .settlements
        .iter()
        .find(|(_, s)| s.controller == england)
        .map(|(id, _)| id.clone())
        .expect("an English settlement");
    let unit_type = data.unit_types.values().next().expect("a unit type");
    let cost = |s: &CampaignState, f: &FactionId| s.recruit_cost(data, f, &settlement, unit_type);
    assert!(cost(&very_hard, &england) < cost(&normal, &england));
    assert_eq!(cost(&very_hard, &france), cost(&normal, &france));
}

#[test]
fn player_unrest_rises_with_the_level() {
    let data = game_data();
    let average_unrest = |level: Difficulty| {
        let mut state = campaign(data, level);
        state.end_turn(data);
        let player = state.player_faction().clone();
        let (sum, count) = state
            .provinces
            .iter()
            .filter(|(id, _)| state.controls_province(&player, id))
            .fold((0.0, 0usize), |(s, n), (_, p)| {
                (
                    s + sim_campaign::population::weighted_unrest(&p.population),
                    n + 1,
                )
            });
        sum / count.max(1) as f64
    };
    let easy = average_unrest(Difficulty::Easy);
    let normal = average_unrest(Difficulty::Normal);
    let hard = average_unrest(Difficulty::VeryHard);
    assert!(easy < normal, "easy {easy} normal {normal}");
    assert!(hard > normal, "very hard {hard} normal {normal}");
}

#[test]
fn ai_attitude_and_war_appetite_towards_the_player() {
    let data = game_data();
    let [easy, normal, hard] = levels(data);
    let england = fac("fac_england");
    let france = fac("fac_france");
    let towards_player = |s: &CampaignState| s.attitude(data, &england, &france).0;
    assert!(towards_player(&easy) > towards_player(&normal));
    assert!(towards_player(&hard) < towards_player(&normal));
    // Other directions are untouched.
    let from_player = |s: &CampaignState| s.attitude(data, &france, &england).0;
    assert_eq!(from_player(&hard), from_player(&normal));
    let scotland = fac("fac_scotland");
    assert_eq!(
        hard.attitude(data, &england, &scotland).0,
        normal.attitude(data, &england, &scotland).0
    );
    // War appetite: a lower ratio demanded against the player only.
    assert!(hard.difficulty_war_ratio_factor(data, &france) < 1.0);
    assert!(easy.difficulty_war_ratio_factor(data, &france) > 1.0);
    assert_eq!(normal.difficulty_war_ratio_factor(data, &france), 1.0);
    assert_eq!(hard.difficulty_war_ratio_factor(data, &england), 1.0);
}

#[test]
fn ai_morale_against_the_player_in_3d_and_auto_resolve() {
    let data = game_data();
    let staged = |level: Difficulty| {
        let mut state = campaign(data, level);
        at_war(&mut state);
        let attacker = first_army(&state, "fac_england");
        let defender = first_army(&state, "fac_france");
        state.debug_stage_battle(&attacker, &defender).unwrap();
        state
    };
    let [easy, normal, hard] = [
        staged(Difficulty::Easy),
        staged(Difficulty::Normal),
        staged(Difficulty::Hard),
    ];
    // 3D: the English (AI) regiments start with more (or less) morale; the
    // player's are untouched.
    let setup = |s: &CampaignState| s.battle_setup(data, 0).expect("setup");
    let ai_morale = |s: &CampaignState| -> Vec<u8> {
        setup(s).attacker.units.iter().map(|u| u.morale).collect()
    };
    let player_morale = |s: &CampaignState| -> Vec<u8> {
        setup(s).defender.units.iter().map(|u| u.morale).collect()
    };
    for ((e, n), h) in ai_morale(&easy)
        .iter()
        .zip(ai_morale(&normal))
        .zip(ai_morale(&hard))
    {
        assert!(*e <= n && n <= h, "{e} {n} {h}");
        assert!(n == 100 || h == (n + 5).min(100));
    }
    assert_eq!(player_morale(&hard), player_morale(&normal));
    // Auto-resolve (forecast): the AI side weighs more when hard.
    let power = |s: &CampaignState| s.battle_forecast(data, 0).unwrap().attacker_power;
    assert!(power(&easy) < power(&normal) && power(&normal) < power(&hard));
    let defender = |s: &CampaignState| s.battle_forecast(data, 0).unwrap().defender_power;
    assert!((defender(&hard) - defender(&normal)).abs() < 1e-9);
    assert!(hard
        .battle_forecast(data, 0)
        .unwrap()
        .modifiers
        .iter()
        .any(|m| m.contains("difficulté")));
}

#[test]
fn level_survives_a_save_and_old_saves_load_as_normal() {
    let data = game_data();
    let hard = campaign(data, Difficulty::Hard);
    let loaded = CampaignState::load_json(&hard.save_json()).expect("loads");
    assert_eq!(loaded.difficulty(), Difficulty::Hard);

    let mut json: serde_json::Value = serde_json::from_str(&hard.save_json()).expect("json");
    json.as_object_mut().unwrap().remove("difficulty");
    let old = CampaignState::load_json(&json.to_string()).expect("old save loads");
    assert_eq!(old.difficulty(), Difficulty::Normal);
}

#[test]
fn level_is_frozen_after_the_first_turn() {
    let data = game_data();
    let mut state = campaign(data, Difficulty::Easy);
    assert!(state.set_difficulty(Difficulty::VeryHard));
    assert_eq!(state.difficulty(), Difficulty::VeryHard);
    state.end_turn(data);
    assert!(!state.set_difficulty(Difficulty::Easy));
    assert_eq!(state.difficulty(), Difficulty::VeryHard);
}

#[test]
fn a_normal_campaign_plays_as_before() {
    // Setting normal explicitly changes nothing to a whole turn.
    let data = game_data();
    let mut default = CampaignState::new_1337(data, fac("fac_france"), 5).unwrap();
    let mut explicit = CampaignState::new_1337(data, fac("fac_france"), 5).unwrap();
    assert!(explicit.set_difficulty(Difficulty::Normal));
    default.end_turn(data);
    explicit.end_turn(data);
    assert_eq!(default.save_json(), explicit.save_json());
}
