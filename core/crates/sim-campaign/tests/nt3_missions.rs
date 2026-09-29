//! Lot NT3 (ADR 0127): short-term campaign missions of the player's faction
//! (spec `docs/superpowers/specs/2026-09-29-nt-nuit-tww3-design.md`, NT3).

use std::path::PathBuf;

use data_model::{FactionId, GameData, MissionKind, MissionReward, SettlementId};
use sim_campaign::missions::{resolve_missions, treaty_tokens, Mission, NoticeKind};
use sim_campaign::{CampaignState, Order, Place};

fn real_data() -> GameData {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../data");
    GameData::load(&root).expect("game data loads").0
}

fn fac(id: &str) -> FactionId {
    FactionId::new(id).unwrap()
}

fn france(data: &GameData, seed: u64) -> CampaignState {
    CampaignState::new_1337(data, fac("fac_france"), seed).unwrap()
}

fn capital_city(state: &CampaignState) -> SettlementId {
    let capital = state.factions[&state.player_faction].capital.clone();
    state.province_city_id(&capital).cloned().unwrap()
}

fn prestige(state: &CampaignState) -> i32 {
    let ruler = state.factions[&state.player_faction].ruler.clone().unwrap();
    state.characters[&ruler].prestige
}

/// A hand-made mission of `kind`, due at `deadline_turn`.
fn mission(state: &CampaignState, kind: MissionKind, deadline_turn: u32) -> Mission {
    Mission {
        id: 99,
        template: "test".to_owned(),
        kind,
        title: "Mission d'essai".to_owned(),
        objective: "Essai.".to_owned(),
        province: None,
        settlement: None,
        building: None,
        count: 2,
        progress: 0,
        issued_turn: state.turn,
        deadline_turn,
        reward: MissionReward {
            gold: 700,
            prestige: 9,
            public_order: 5,
            free_unit: true,
        },
        baseline: Vec::new(),
    }
}

/// The state with the player's missions taken in hand and replaced by `m`
/// (and no new offer before the next turn).
fn with_mission(mut state: CampaignState, m: Mission) -> CampaignState {
    state.missions.faction = Some(state.player_faction.clone());
    state.missions.active = vec![m];
    state.missions.last_closed_turn = Some(state.turn);
    state
}

#[test]
fn the_player_gets_missions_and_the_ai_none() {
    let data = real_data();
    let mut state = france(&data, 11);
    assert!(state.missions.active.is_empty(), "no mission before turn 1");
    state.end_turn(&data);
    assert_eq!(state.missions.faction.as_ref(), Some(&fac("fac_france")));
    assert_eq!(state.missions.active.len(), 1, "one offer per turn");
    let notices = state.mission_notices();
    assert_eq!(notices.len(), 1);
    assert_eq!(notices[0].kind, NoticeKind::Offered);
    assert!(notices[0].text.starts_with("Nouvelle mission"));
    assert!(state
        .events()
        .iter()
        .any(|e| e.kind == sim_campaign::EventKind::Mission));
    let m = &state.missions.active[0];
    assert!((3..=12).contains(&(m.deadline_turn - m.issued_turn)));
    assert!(!m.title.contains('{') && !m.objective.contains('{'));
    // A second slot fills on a later turn, never beyond two, never twice
    // the same kind at once.
    for _ in 0..6 {
        state.end_turn(&data);
        assert!(state.missions.active.len() <= 2);
        let mut kinds: Vec<_> = state.missions.active.iter().map(|m| m.kind).collect();
        kinds.dedup();
        assert_eq!(kinds.len(), state.missions.active.len());
    }
    let views = state.missions(&data);
    assert_eq!(views.len(), state.missions.active.len());
    for v in &views {
        assert!(!v.progress.is_empty() && !v.reward.is_empty() && !v.deadline.is_empty());
    }
}

#[test]
fn offers_are_deterministic_by_seed() {
    let data = real_data();
    let run = |seed| {
        let mut state = france(&data, seed);
        for _ in 0..4 {
            state.end_turn(&data);
        }
        state.missions.active.clone()
    };
    assert_eq!(run(5), run(5));
}

#[test]
fn the_generator_follows_the_situation() {
    let data = real_data();
    let mut state = france(&data, 3);
    let player = state.player_faction.clone();
    // At war with a neighbour: a province it holds on the border can be the
    // target.
    let england = state
        .provinces
        .keys()
        .filter(|q| state.controls_province(&player, q))
        .flat_map(|q| data.provinces[q].neighbors.iter())
        .filter_map(|n| state.province_controller(n))
        .find(|c| **c != player && !state.factions[&player].allies.contains(*c))
        .cloned()
        .expect("a foreign neighbour");
    state
        .factions
        .get_mut(&player)
        .unwrap()
        .at_war_with
        .insert(england.clone());
    state
        .factions
        .get_mut(&england)
        .unwrap()
        .at_war_with
        .insert(player.clone());
    let mut seen = std::collections::BTreeSet::new();
    for seed in 0..40 {
        let mut s = state.clone();
        s.seed = seed;
        s.turn = 1;
        let mut events = Vec::new();
        resolve_missions(&mut s, &data, &mut events);
        for m in &s.missions.active {
            seen.insert(m.kind);
            if m.kind == MissionKind::TakeProvince {
                let p = m.province.as_ref().unwrap();
                assert_eq!(s.province_controller(p), Some(&england));
                let neighbours = s
                    .provinces
                    .keys()
                    .filter(|q| s.controls_province(&player, q))
                    .any(|q| data.provinces[q].neighbors.contains(p));
                assert!(neighbours, "{p} borders the player");
            }
            if m.kind == MissionKind::ConstructBuilding {
                assert!(m.building.is_some() && m.settlement.is_some());
            }
        }
    }
    assert!(seen.len() >= 4, "several kinds are offered: {seen:?}");
    assert!(seen.contains(&MissionKind::TakeProvince));
}

#[test]
fn recruiting_counts_and_success_pays_the_reward() {
    let data = real_data();
    let state = france(&data, 7);
    let m = mission(&state, MissionKind::RecruitUnits, state.turn + 4);
    let mut state = with_mission(state, m);
    state
        .factions
        .get_mut(&state.player_faction.clone())
        .unwrap()
        .treasury = 50_000;
    let city = capital_city(&state);
    let unit = state
        .recruitable(&data, &city)
        .into_iter()
        .find(|o| o.available)
        .expect("a unit to raise in the capital")
        .unit_type;
    let order = || Order::Recruit {
        settlement: Place::Settlement(city.clone()),
        unit_type: unit.clone(),
    };
    state.submit_order(&data, order()).unwrap();
    assert_eq!(state.missions.active[0].progress, 1);
    // Not yet: 1/2.
    let mut events = Vec::new();
    resolve_missions(&mut state, &data, &mut events);
    assert_eq!(state.missions.active.len(), 1);
    state.submit_order(&data, order()).unwrap();
    assert_eq!(state.missions.active[0].progress, 2);

    let player = state.player_faction.clone();
    let treasury = state.factions[&player].treasury;
    let glory = prestige(&state);
    let garrison = state.settlements[&city].garrison.len();
    let capital = state.factions[&player].capital.clone();
    state.provinces.get_mut(&capital).unwrap().unrest = 20;
    let mut events = Vec::new();
    resolve_missions(&mut state, &data, &mut events);
    assert!(state.missions.active.iter().all(|m| m.id != 99));
    assert_eq!(state.factions[&player].treasury, treasury + 700);
    assert_eq!(prestige(&state), glory + 9);
    assert_eq!(state.provinces[&capital].unrest, 15);
    assert_eq!(state.settlements[&city].garrison.len(), garrison + 1);
    let notice = &state.mission_notices()[0];
    assert_eq!(notice.kind, NoticeKind::Succeeded);
    assert!(notice.text.contains("Mission accomplie"));
    assert!(notice.text.contains("700 livres"));
}

#[test]
fn a_won_battle_counts_through_the_counter() {
    let data = real_data();
    let state = france(&data, 7);
    let mut m = mission(&state, MissionKind::WinBattle, state.turn + 6);
    m.count = 1;
    m.progress = 1;
    let mut state = with_mission(state, m);
    let mut events = Vec::new();
    resolve_missions(&mut state, &data, &mut events);
    assert_eq!(state.mission_notices()[0].kind, NoticeKind::Succeeded);
}

#[test]
fn the_deadline_fails_the_mission_with_a_small_loss() {
    let data = real_data();
    let state = france(&data, 7);
    let m = mission(&state, MissionKind::WinBattle, state.turn);
    let mut state = with_mission(state, m);
    let player = state.player_faction.clone();
    let treasury = state.factions[&player].treasury;
    let glory = prestige(&state);
    let mut events = Vec::new();
    resolve_missions(&mut state, &data, &mut events);
    assert!(state.missions.active.is_empty(), "cooldown: no new offer");
    assert_eq!(state.factions[&player].treasury, treasury);
    assert_eq!(
        prestige(&state),
        glory + data.mission_rules.failure_prestige
    );
    assert!(data.mission_rules.failure_prestige >= -5);
    let notice = &state.mission_notices()[0];
    assert_eq!(notice.kind, NoticeKind::Failed);
    assert!(notice.text.contains("échéance"));
}

#[test]
fn a_held_place_succeeds_at_the_deadline_and_fails_when_lost() {
    let data = real_data();
    let base = france(&data, 7);
    let capital = base.factions[&base.player_faction].capital.clone();
    let mut m = mission(&base, MissionKind::HoldPlace, base.turn + 1);
    m.province = Some(capital.clone());
    m.reward = MissionReward {
        gold: 100,
        ..MissionReward::default()
    };

    // Still held, deadline not reached: ongoing; then reached: success.
    let mut state = with_mission(base.clone(), m.clone());
    let mut events = Vec::new();
    resolve_missions(&mut state, &data, &mut events);
    assert_eq!(state.missions.active.len(), 1);
    state.turn += 1;
    resolve_missions(&mut state, &data, &mut events);
    assert_eq!(state.mission_notices()[0].kind, NoticeKind::Succeeded);

    // Lost: failure at once.
    let mut state = with_mission(base, m);
    let city = capital_city(&state);
    state.settlements.get_mut(&city).unwrap().controller = fac("fac_england");
    resolve_missions(&mut state, &data, &mut events);
    let notice = &state.mission_notices()[0];
    assert_eq!(notice.kind, NoticeKind::Failed);
    assert!(notice.text.contains("perdue"));
}

#[test]
fn a_new_treaty_fulfils_the_treaty_mission() {
    let data = real_data();
    let state = france(&data, 7);
    let m = mission(&state, MissionKind::ConcludeTreaty, state.turn + 8);
    let mut state = with_mission(state, m);
    let player = state.player_faction.clone();
    state.missions.active[0].baseline = treaty_tokens(&state, &player);
    let mut events = Vec::new();
    resolve_missions(&mut state, &data, &mut events);
    assert_eq!(state.missions.active.len(), 1, "nothing new yet");
    let partner = state
        .factions
        .iter()
        .find(|(id, f)| **id != player && f.alive && !state.factions[&player].allies.contains(*id))
        .map(|(id, _)| id.clone())
        .unwrap();
    state
        .factions
        .get_mut(&player)
        .unwrap()
        .allies
        .insert(partner);
    resolve_missions(&mut state, &data, &mut events);
    assert_eq!(state.mission_notices()[0].kind, NoticeKind::Succeeded);
}

#[test]
fn missions_survive_save_and_load_and_old_saves_load_empty() {
    let data = real_data();
    let mut state = france(&data, 21);
    state.end_turn(&data);
    state.end_turn(&data);
    assert!(!state.missions.active.is_empty());
    let json = state.save_json();
    let mut loaded = CampaignState::load_json(&json).unwrap();
    assert_eq!(loaded.missions.active, state.missions.active);
    assert_eq!(loaded.missions.next_id, state.missions.next_id);
    // Same future after loading.
    state.end_turn(&data);
    loaded.end_turn(&data);
    assert_eq!(loaded.missions.active, state.missions.active);

    // A save from before NT3 has no `missions` key.
    let mut value: serde_json::Value = serde_json::from_str(&json).unwrap();
    value.as_object_mut().unwrap().remove("missions");
    let old = CampaignState::load_json(&value.to_string()).unwrap();
    assert!(old.missions.active.is_empty());
    assert!(old.missions.faction.is_none());
}
