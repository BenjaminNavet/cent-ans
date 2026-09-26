//! Lot DP2 (ADR 0075): right of passage and trespass incidents, the AI's
//! respect of it, the path warning and the diplomatic stance of the map.

use std::path::PathBuf;

use data_model::{FactionId, GameData, ProvinceId};
use sim_campaign::diplomacy::RelationKind;
use sim_campaign::passage::{
    ai_may_trespass, has_grievance, penalty, resolve_trespass, trespass_along, trespassed_owner,
    TRESPASS_REASON,
};
use sim_campaign::stance::{diplomatic_stance, Stance};
use sim_campaign::{ArmyId, ArmyPosition, CampaignState, Order};

fn data() -> GameData {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../data");
    let (data, _warnings) = GameData::load(&root).expect("game data loads");
    data
}

fn fac(id: &str) -> FactionId {
    FactionId::new(id).unwrap()
}

fn start(data: &GameData, seed: u64) -> CampaignState {
    CampaignState::new_1337(data, fac("fac_france"), seed).expect("1337 start")
}

/// A faction at plain peace with France (no truce, no grievance of any
/// kind), and one of its provinces whose city point lies in it.
fn neutral_victim(state: &CampaignState, data: &GameData) -> (FactionId, ProvinceId, [f32; 2]) {
    let fr = fac("fac_france");
    for (id, f) in &state.factions {
        if !f.alive
            || id == &fr
            || state.relation(&fr, id) != RelationKind::Peace
            || state.casus_belli(data, id, &fr).is_some()
        {
            continue;
        }
        let mut provinces: Vec<&ProvinceId> = state
            .provinces
            .keys()
            .filter(|p| state.controls_province(id, p))
            .collect();
        provinces.sort();
        for p in provinces {
            let Some(city) = state.province_city_id(p) else {
                continue;
            };
            let Some(point) = data.settlement_point(city) else {
                continue;
            };
            if data.province_at_point(point[0], point[1]) == Some(p) {
                return (id.clone(), p.clone(), point);
            }
        }
    }
    panic!("no neutral neighbour of France");
}

fn french_army(state: &CampaignState) -> ArmyId {
    state
        .armies
        .iter()
        .find(|(_, a)| a.faction.as_str() == "fac_france")
        .map(|(id, _)| id.clone())
        .expect("France has an army")
}

/// Moves a French army to `point` (in the field).
fn place(state: &mut CampaignState, army: &ArmyId, point: [f32; 2]) {
    let a = state.armies.get_mut(army).unwrap();
    a.position = ArmyPosition::field(point);
    a.clear_plan();
}

fn trespass_malus(state: &CampaignState, victim: &FactionId, intruder: &FactionId) -> i32 {
    state.factions[victim]
        .modifiers
        .iter()
        .filter(|m| &m.with == intruder && m.reason_fr == TRESPASS_REASON)
        .map(|m| m.value)
        .sum()
}

fn season(state: &mut CampaignState, data: &GameData) -> Vec<sim_campaign::GameEvent> {
    let mut events = Vec::new();
    resolve_trespass(state, data, &mut events);
    state.turn += 1;
    events
}

#[test]
fn penalty_grows_with_the_seasons_and_is_capped() {
    let data = data();
    let rules = &data.ai_diplomacy.passage;
    assert!(
        rules.enabled,
        "passage rules are on in data/ai/diplomacy.json"
    );
    assert_eq!(penalty(rules, 0), 0);
    assert_eq!(penalty(rules, 1), rules.base_penalty);
    assert_eq!(
        penalty(rules, 2),
        rules.base_penalty + rules.per_season_penalty
    );
    assert_eq!(penalty(rules, 100), rules.max_penalty);
}

#[test]
fn trespass_creates_a_growing_incident_then_a_casus_belli() {
    let data = data();
    let mut state = start(&data, 1);
    let fr = fac("fac_france");
    let (victim, province, point) = neutral_victim(&state, &data);
    assert_eq!(
        trespassed_owner(&state, &fr, &province),
        Some(victim.clone())
    );
    let army = french_army(&state);
    place(&mut state, &army, point);
    assert_eq!(
        state.army_province(&data, &state.armies[&army]),
        Some(province.clone())
    );
    let attitude_before = state.attitude(&data, &victim, &fr).0;

    let events = season(&mut state, &data);
    let rules = data.ai_diplomacy.passage.clone();
    assert_eq!(trespass_malus(&state, &victim, &fr), -rules.base_penalty);
    assert!(
        events
            .iter()
            .any(|e| e.text_fr.contains("sans droit de passage")),
        "{events:?}"
    );
    assert!(state.attitude(&data, &victim, &fr).0 < attitude_before);
    assert!(!has_grievance(&state, &victim, &fr));

    season(&mut state, &data);
    // One modifier, replaced and grown, not stacked.
    assert_eq!(
        trespass_malus(&state, &victim, &fr),
        -(rules.base_penalty + rules.per_season_penalty)
    );
    assert!(has_grievance(&state, &victim, &fr));
    assert_eq!(
        state.casus_belli(&data, &victim, &fr).as_deref(),
        Some("violation de nos frontières")
    );
    let record = &state.factions[&victim].ledger.trespassers[&fr];
    assert_eq!((record.seasons, record.total), (2, 2));
    assert_eq!(record.provinces, vec![province]);
}

#[test]
fn leaving_resets_the_count_and_the_incident_is_forgotten() {
    let data = data();
    let mut state = start(&data, 2);
    let fr = fac("fac_france");
    let (victim, _, point) = neutral_victim(&state, &data);
    let army = french_army(&state);
    let home = state.armies[&army].position.clone();
    place(&mut state, &army, point);
    season(&mut state, &data);
    state.armies.get_mut(&army).unwrap().position = home;
    season(&mut state, &data);
    assert_eq!(state.factions[&victim].ledger.trespassers[&fr].seasons, 0);
    let memory = data.ai_diplomacy.passage.memory_turns;
    for _ in 0..=memory {
        season(&mut state, &data);
    }
    assert!(!state.factions[&victim].ledger.trespassers.contains_key(&fr));
    assert_eq!(
        trespass_malus(&state, &victim, &fr),
        -data.ai_diplomacy.passage.base_penalty
    );
    // The modifier itself expires with the memory.
    let turn = state.turn;
    assert!(state.factions[&victim]
        .modifiers
        .iter()
        .filter(|m| m.reason_fr == TRESPASS_REASON)
        .all(|m| m.expires_turn <= turn));
}

#[test]
fn military_access_or_alliance_make_the_passage_lawful() {
    let data = data();
    let mut state = start(&data, 3);
    let fr = fac("fac_france");
    let (victim, province, point) = neutral_victim(&state, &data);
    state
        .factions
        .get_mut(&victim)
        .unwrap()
        .ledger
        .military_access
        .insert(fr.clone());
    assert_eq!(trespassed_owner(&state, &fr, &province), None);
    let army = french_army(&state);
    place(&mut state, &army, point);
    season(&mut state, &data);
    assert_eq!(trespass_malus(&state, &victim, &fr), 0);
    assert!(!state.factions[&victim].ledger.trespassers.contains_key(&fr));

    // Without access but allied: lawful too.
    let v = state.factions.get_mut(&victim).unwrap();
    v.ledger.military_access.clear();
    v.allies.insert(fr.clone());
    state
        .factions
        .get_mut(&fr)
        .unwrap()
        .allies
        .insert(victim.clone());
    assert_eq!(trespassed_owner(&state, &fr, &province), None);
    // At war: an invasion, not a trespass.
    let v = state.factions.get_mut(&victim).unwrap();
    v.allies.clear();
    v.at_war_with.insert(fr.clone());
    let f = state.factions.get_mut(&fr).unwrap();
    f.allies.remove(&victim);
    f.at_war_with.insert(victim.clone());
    assert_eq!(trespassed_owner(&state, &fr, &province), None);
}

#[test]
fn a_truce_leaves_time_to_withdraw() {
    let data = data();
    let mut state = start(&data, 4);
    let fr = fac("fac_france");
    let (victim, _, point) = neutral_victim(&state, &data);
    let until = state.turn + 20;
    state
        .factions
        .get_mut(&victim)
        .unwrap()
        .truces
        .insert(fr.clone(), until);
    state
        .factions
        .get_mut(&fr)
        .unwrap()
        .truces
        .insert(victim.clone(), until);
    let army = french_army(&state);
    place(&mut state, &army, point);
    let grace = data.ai_diplomacy.passage.truce_grace_seasons;
    for _ in 0..grace {
        season(&mut state, &data);
        assert_eq!(trespass_malus(&state, &victim, &fr), 0);
    }
    season(&mut state, &data);
    assert_eq!(
        trespass_malus(&state, &victim, &fr),
        -data.ai_diplomacy.passage.base_penalty
    );
}

#[test]
fn disabled_rules_ignore_trespass() {
    let mut data = data();
    data.ai_diplomacy.passage.enabled = false;
    let mut state = start(&data, 5);
    let fr = fac("fac_france");
    let (victim, _, point) = neutral_victim(&state, &data);
    let army = french_army(&state);
    place(&mut state, &army, point);
    season(&mut state, &data);
    assert!(state.factions[&victim].ledger.trespassers.is_empty());
    assert!(trespass_along(&state, &data, &fr, &[point, point], &[1]).is_empty());
    assert!(ai_may_trespass(&state, &data, &fr, &victim));
}

#[test]
fn the_end_of_turn_records_trespass_deterministically() {
    let data = data();
    let run = || {
        let mut state = start(&data, 6);
        let (_, _, point) = neutral_victim(&state, &data);
        let army = french_army(&state);
        place(&mut state, &army, point);
        let idle = |_: &CampaignState, _: &GameData, _: &FactionId| -> Vec<Order> { Vec::new() };
        state.end_turn_with(&data, idle);
        serde_json::to_string(&state).unwrap()
    };
    let first = run();
    assert_eq!(first, run());
    let state: CampaignState = serde_json::from_str(&first).unwrap();
    let fr = fac("fac_france");
    assert!(
        state
            .factions
            .values()
            .any(|f| f.ledger.trespassers.contains_key(&fr)),
        "the end of turn runs the trespass check"
    );
}

#[test]
fn the_path_preview_names_the_lands_crossed_and_the_halts() {
    let data = data();
    let state = start(&data, 7);
    let fr = fac("fac_france");
    let (victim, province, point) = neutral_victim(&state, &data);
    let army = french_army(&state);
    let from = state.army_point(&data, &state.armies[&army]);
    let crossings = trespass_along(&state, &data, &fr, &[from, point], &[1]);
    let found = crossings
        .iter()
        .find(|t| t.province == province)
        .expect("the destination is trespassed");
    assert_eq!(found.owner, victim);
    assert!(found.halt, "the season ends there");
    // Our own lands only: nothing.
    assert!(trespass_along(&state, &data, &fr, &[from, from], &[1]).is_empty());
}

#[test]
fn the_ai_respects_the_passage_at_peace_and_breaks_it_by_temper_at_war() {
    let mut data = data();
    let mut state = start(&data, 8);
    let fr = fac("fac_france");
    let (victim, _, _) = neutral_victim(&state, &data);
    // No war: never.
    for f in state.factions.values_mut() {
        f.at_war_with.clear();
    }
    assert!(!ai_may_trespass(&state, &data, &fr, &victim));
    // At war with England.
    let en = fac("fac_england");
    state
        .factions
        .get_mut(&fr)
        .unwrap()
        .at_war_with
        .insert(en.clone());
    state
        .factions
        .get_mut(&en)
        .unwrap()
        .at_war_with
        .insert(fr.clone());
    let personality = data
        .factions
        .get_mut(&fr)
        .unwrap()
        .ai_personality
        .as_mut()
        .unwrap();
    personality.aggression = Some(95);
    assert!(ai_may_trespass(&state, &data, &fr, &victim));
    let personality = data
        .factions
        .get_mut(&fr)
        .unwrap()
        .ai_personality
        .as_mut()
        .unwrap();
    personality.aggression = Some(10);
    data.ai_diplomacy.passage.ai_violate_power_ratio = 1000.0;
    data.ai_diplomacy.passage.ai_violate_attitude = -1000;
    assert!(!ai_may_trespass(&state, &data, &fr, &victim));
    // Hatred overrides a peaceful temper.
    data.ai_diplomacy.passage.ai_violate_attitude = 1000;
    assert!(ai_may_trespass(&state, &data, &fr, &victim));
}

#[test]
fn the_diplomatic_map_stance() {
    let data = data();
    let mut state = start(&data, 9);
    let fr = fac("fac_france");
    let en = fac("fac_england");
    assert_eq!(diplomatic_stance(&state, &data, &fr, &fr), Stance::Own);
    let (victim, _, point) = neutral_victim(&state, &data);
    let before = diplomatic_stance(&state, &data, &fr, &victim);
    assert!(
        matches!(
            before,
            Stance::Neutral | Stance::Agreement | Stance::Tension
        ),
        "{before:?}"
    );
    // A trade agreement: an accord (unless tension prevails).
    for (a, b) in [(&fr, &victim), (&victim, &fr)] {
        state
            .factions
            .get_mut(a)
            .unwrap()
            .ledger
            .trade_agreements
            .insert(b.clone());
    }
    if before != Stance::Tension {
        assert_eq!(
            diplomatic_stance(&state, &data, &fr, &victim),
            Stance::Agreement
        );
    }
    // Our army camping on their lands: tension.
    let army = french_army(&state);
    place(&mut state, &army, point);
    season(&mut state, &data);
    assert_eq!(
        diplomatic_stance(&state, &data, &fr, &victim),
        Stance::Tension
    );
    // War.
    state
        .factions
        .get_mut(&fr)
        .unwrap()
        .at_war_with
        .insert(en.clone());
    state
        .factions
        .get_mut(&en)
        .unwrap()
        .at_war_with
        .insert(fr.clone());
    assert_eq!(diplomatic_stance(&state, &data, &fr, &en), Stance::War);
    assert_eq!(Stance::War.key(), "war");
    assert_eq!(Stance::Agreement.label_fr(), "Accord");
}
