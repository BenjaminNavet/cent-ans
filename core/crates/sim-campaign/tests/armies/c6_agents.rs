//! Lot C6: campaign agents (spies, heralds, preachers) — `agents.rs`.

use data_model::{
    AgentActionKind, AgentKind, AgentRules, BuildingCategory, CharacterId, FactionId, GameData,
    SettlementId, SettlementKind,
};
use sim_campaign::agents::{self, AgentId};
use sim_campaign::test_support::start;
use sim_campaign::{CampaignState, Order, OrderError, SiegeState};

use data_model::test_support::{fac, game_data};

/// Rules where every action succeeds and nobody dies.
fn sure(data: &mut GameData) {
    let mut rules = agents::rules(data).clone();
    for action in rules.actions.values_mut() {
        action.base_chance = 100;
        action.death_risk = 0;
    }
    rules.max_chance = 100;
    rules.min_chance = 100;
    data.agent_rules = Some(rules);
}

/// Rules where every action fails and the agent is lost.
fn doomed(data: &mut GameData) {
    let mut rules = agents::rules(data).clone();
    for action in rules.actions.values_mut() {
        action.base_chance = 0;
        action.per_level = 0;
        action.death_risk = 100;
    }
    rules.max_chance = 0;
    rules.min_chance = 0;
    data.agent_rules = Some(rules);
}

fn war(state: &mut CampaignState, a: &FactionId, b: &FactionId) {
    state
        .factions
        .get_mut(a)
        .unwrap()
        .at_war_with
        .insert(b.clone());
    state
        .factions
        .get_mut(b)
        .unwrap()
        .at_war_with
        .insert(a.clone());
    state.factions.get_mut(a).unwrap().allies.remove(b);
    state.factions.get_mut(b).unwrap().allies.remove(a);
}

fn peace(state: &mut CampaignState, a: &FactionId, b: &FactionId) {
    state.factions.get_mut(a).unwrap().at_war_with.remove(b);
    state.factions.get_mut(b).unwrap().at_war_with.remove(a);
}

/// Some city controlled by `faction`.
fn city_of(state: &CampaignState, faction: &FactionId) -> SettlementId {
    state
        .settlements
        .iter()
        .find(|(_, s)| &s.controller == faction && s.kind == SettlementKind::City)
        .map(|(id, _)| id.clone())
        .expect("a city")
}

fn paris(state: &CampaignState) -> SettlementId {
    state
        .province_city_id(&data_model::ProvinceId::new("prov_ile_de_france").unwrap())
        .unwrap()
        .clone()
}

/// A settlement of `enemy` adjacent (one edge) to a settlement of `friend`:
/// `(friendly, enemy)`, preferring enemy cities.
fn border(
    state: &CampaignState,
    data: &GameData,
    friend: &FactionId,
    enemy: &FactionId,
) -> (SettlementId, SettlementId) {
    let mut found = None;
    for (id, s) in &state.settlements {
        if &s.controller != friend {
            continue;
        }
        for (next, _) in sim_campaign::movement::edges(data, id) {
            let Some(other) = state.settlements.get(&next) else {
                continue;
            };
            if &other.controller == enemy {
                let city = other.kind == SettlementKind::City;
                if city {
                    return (id.clone(), next);
                }
                found.get_or_insert((id.clone(), next));
            }
        }
    }
    found.expect("a border between the two factions")
}

/// Places a fresh agent of `faction` directly (bypassing recruitment).
fn place(
    state: &mut CampaignState,
    data: &GameData,
    faction: &FactionId,
    kind: AgentKind,
    at: &SettlementId,
) -> AgentId {
    state.factions.get_mut(faction).unwrap().treasury += 10_000;
    // DC3: a city with a religious building first (every kind enlists there; the
    // densified map puts abbeys before those cities, and a herald only enlists in a city),
    // else an abbey.
    let religious_city = |s: &sim_campaign::SettlementState| {
        s.kind == SettlementKind::City
            && s.buildings.iter().any(|b| {
                data.buildings
                    .get(b)
                    .is_some_and(|d| d.category == BuildingCategory::Religious)
            })
    };
    let own = state
        .settlements
        .iter()
        .filter(|(_, s)| &s.controller == faction)
        .find(|(_, s)| religious_city(s))
        .or_else(|| {
            state
                .settlements
                .iter()
                .find(|(_, s)| &s.controller == faction && s.kind == SettlementKind::Abbey)
        })
        .map(|(id, _)| id.clone())
        .expect("a place to recruit");
    let id = state.recruit_agent(data, faction, &own, kind).unwrap();
    let agent = state.agents.agents.get_mut(&id).unwrap();
    agent.location = at.clone();
    agent.movement_points = 500;
    id
}

fn act(
    state: &mut CampaignState,
    data: &GameData,
    id: &AgentId,
    action: AgentActionKind,
    target: Option<&SettlementId>,
) -> sim_campaign::AgentReport {
    let faction = state.agent(id).unwrap().faction.clone();
    state
        .agent_act(data, &faction, id, action, target, None)
        .unwrap_or_else(|e| panic!("{action:?} refused: {e}"))
}

#[test]
fn rules_file_matches_the_design_defaults() {
    let data = game_data();
    let loaded = data.agent_rules.as_ref().expect("data/rules/agents.json");
    let defaults = AgentRules::default();
    for kind in AgentKind::ALL {
        assert_eq!(loaded.types[&kind].cost, defaults.types[&kind].cost);
        assert_eq!(
            loaded.types[&kind].max_per_faction,
            defaults.types[&kind].max_per_faction
        );
        assert!(!loaded.types[&kind].names.is_empty());
    }
    for action in AgentActionKind::ALL {
        assert_eq!(
            loaded.actions[&action].base_chance,
            defaults.actions[&action].base_chance
        );
    }
    assert_eq!(loaded.ransom_price_percent, defaults.ransom_price_percent);
}

#[test]
fn recruitment_pays_checks_places_and_caps() {
    let data = game_data();
    let mut state = start(data, "fac_france", 7);
    let france = fac("fac_france");
    let paris = paris(&state);
    state.factions.get_mut(&france).unwrap().treasury = 5_000;
    let before = state.factions[&france].treasury;
    state
        .submit_order(
            data,
            Order::RecruitAgent {
                settlement: paris.clone().into(),
                kind: AgentKind::Spy,
            },
        )
        .expect("a spy in Paris");
    let spent = before - state.factions[&france].treasury;
    assert!(spent >= 200, "spent {spent}");
    let (id, agent) = state.agents_of(&france)[0];
    assert_eq!(agent.location, paris);
    assert_eq!(agent.level, 1);
    assert_eq!(agent.movement_points, 0, "no march on the recruitment turn");
    assert!(!agent.name.is_empty());
    let id = id.clone();
    // Cap: three spies.
    for _ in 0..2 {
        state
            .recruit_agent(data, &france, &paris, AgentKind::Spy)
            .unwrap();
    }
    let err = state
        .recruit_agent(data, &france, &paris, AgentKind::Spy)
        .unwrap_err();
    assert!(err.to_string().contains("plafond"), "{err}");
    // A herald needs a city; a village is refused.
    let village = state
        .settlements
        .iter()
        .find(|(_, s)| s.controller == france && s.kind == SettlementKind::Village)
        .map(|(id, _)| id.clone())
        .expect("a French village");
    assert!(state
        .recruit_agent(data, &france, &village, AgentKind::Emissary)
        .is_err());
    // Not in someone else's settlement.
    let english = state
        .settlements
        .iter()
        .find(|(_, s)| s.controller == fac("fac_england"))
        .map(|(id, _)| id.clone())
        .unwrap();
    assert!(state
        .recruit_agent(data, &france, &english, AgentKind::Spy)
        .is_err());
    // Options of the panel.
    let options = state.agent_recruit_options(data, &france, &paris);
    assert_eq!(options.len(), 3);
    let spy = options.iter().find(|o| o.kind == AgentKind::Spy).unwrap();
    assert!(!spy.available);
    assert_eq!(spy.count, 3);
    // Dismissal.
    state
        .submit_order(data, Order::DismissAgent { agent: id.clone() })
        .unwrap();
    assert!(state.agent(&id).is_none());
}

#[test]
fn preacher_needs_a_religious_building_or_an_abbey() {
    let data = game_data();
    let mut state = start(data, "fac_france", 7);
    let france = fac("fac_france");
    state.factions.get_mut(&france).unwrap().treasury = 5_000;
    let religious = |s: &sim_campaign::SettlementState| {
        s.kind == SettlementKind::Abbey
            || s.buildings.iter().any(|b| {
                data.buildings
                    .get(b)
                    .is_some_and(|d| d.category == BuildingCategory::Religious)
            })
    };
    let without = state
        .settlements
        .iter()
        .find(|(_, s)| s.controller == france && s.kind == SettlementKind::Town && !religious(s))
        .map(|(id, _)| id.clone());
    if let Some(without) = without {
        let err = state
            .recruit_agent(data, &france, &without, AgentKind::Preacher)
            .unwrap_err();
        assert!(err.to_string().contains("religieux"), "{err}");
    }
    let with = state
        .settlements
        .iter()
        .find(|(_, s)| s.controller == france && s.kind != SettlementKind::Village && religious(s))
        .map(|(id, _)| id.clone())
        .expect("a French abbey or church");
    state
        .recruit_agent(data, &france, &with, AgentKind::Preacher)
        .expect("preacher recruited");
}

#[test]
fn agents_walk_the_settlement_graph_through_enemy_land() {
    let mut data = game_data().clone();
    sure(&mut data);
    let mut state = start(&data, "fac_france", 7);
    let (france, england) = (fac("fac_france"), fac("fac_england"));
    war(&mut state, &france, &england);
    let (home, enemy) = border(&state, &data, &france, &england);
    let id = place(&mut state, &data, &france, AgentKind::Spy, &home);
    let allowance = state.agent_movement_allowance(&data, AgentKind::Spy);
    state.agents.agents.get_mut(&id).unwrap().movement_points = allowance;
    let reach = state.agent_reachable(&data, &id);
    assert!(
        reach.contains_key(&enemy),
        "an enemy settlement is reachable"
    );
    assert!(reach.values().all(|c| *c <= allowance));
    // A far target: several seasons, the destination is kept.
    let far = state
        .settlements
        .keys()
        .filter(|s| !reach.contains_key(*s) && state.agent_find_path(&data, &id, s).is_some())
        .max_by_key(|s| state.agent_find_path(&data, &id, s).map_or(0, |p| p.len()))
        .unwrap()
        .clone();
    state
        .submit_order(
            &data,
            Order::MoveAgent {
                agent: id.clone(),
                target: far.clone(),
            },
        )
        .unwrap();
    let agent = state.agent(&id).unwrap();
    assert_ne!(agent.location, home, "the agent left at once");
    assert_eq!(agent.destination.as_ref(), Some(&far));
    let after_first = agent.location.clone();
    state.end_turn(&data);
    let agent = state.agent(&id).expect("still alive");
    assert_ne!(agent.location, after_first, "the march resumed next season");
    // Unknown destination.
    let err = state
        .submit_order(
            &data,
            Order::MoveAgent {
                agent: id.clone(),
                target: SettlementId::new("set_nowhere").unwrap(),
            },
        )
        .unwrap_err();
    assert!(matches!(err, OrderError::Agent(_)));
}

#[test]
fn scouting_reports_and_keeps_the_province_in_sight() {
    let mut data = game_data().clone();
    sure(&mut data);
    let mut state = start(&data, "fac_france", 7);
    let (france, england) = (fac("fac_france"), fac("fac_england"));
    let (home, enemy) = border(&state, &data, &france, &england);
    let id = place(&mut state, &data, &france, AgentKind::Spy, &home);
    let province = state.settlement_province(&enemy).unwrap().clone();
    let report = act(&mut state, &data, &id, AgentActionKind::Scout, Some(&enemy));
    assert!(report.success);
    assert!(report.text_fr.contains("garnison"), "{}", report.text_fr);
    assert!(state
        .agents
        .intel
        .iter()
        .any(|i| i.faction == france && i.province == province));
    // Move the spy far away: the intelligence alone keeps the sight.
    let paris = paris(&state);
    state.agents.agents.get_mut(&id).unwrap().location = paris;
    assert!(state.visible_provinces(&data, &france).contains(&province));
    // One action per season.
    let err = state
        .agent_act(&data, &france, &id, AgentActionKind::Scout, None, None)
        .unwrap_err();
    assert!(err.to_string().contains("déjà"), "{err}");
    assert_eq!(state.agent(&id).unwrap().experience, 2);
    // The report opens the next journal.
    assert!(state
        .pending_events
        .iter()
        .any(|e| e.kind == sim_campaign::EventKind::Agent));
}

#[test]
fn spies_see_around_them() {
    let data = game_data();
    let mut state = start(data, "fac_france", 7);
    let france = fac("fac_france");
    // The farthest settlement from France: somewhere nobody French sees.
    let before = state.visible_provinces(data, &france);
    let hidden = state
        .settlements
        .iter()
        .find(|(_, s)| !before.contains(&s.province))
        .map(|(id, _)| id.clone())
        .expect("a hidden settlement");
    let province = state.settlement_province(&hidden).unwrap().clone();
    place(&mut state, data, &france, AgentKind::Spy, &hidden);
    let after = state.visible_provinces(data, &france);
    assert!(after.contains(&province));
    for neighbour in sim_campaign::movement::land_neighbors(data, &province) {
        assert!(after.contains(neighbour), "spy range 1 reaches {neighbour}");
    }
}

#[test]
fn sabotage_opens_a_breach_or_delays_works() {
    let mut data = game_data().clone();
    sure(&mut data);
    let mut state = start(&data, "fac_france", 7);
    let (france, england) = (fac("fac_france"), fac("fac_england"));
    let (home, enemy) = border(&state, &data, &france, &england);
    // At peace: refused.
    peace(&mut state, &france, &england);
    let id = place(&mut state, &data, &france, AgentKind::Spy, &home);
    assert!(state
        .agent_act(
            &data,
            &france,
            &id,
            AgentActionKind::Sabotage,
            Some(&enemy),
            None
        )
        .is_err());
    war(&mut state, &france, &england);
    state.settlements.get_mut(&enemy).unwrap().siege = Some(SiegeState {
        attacker: france.clone(),
        turns_left: 4,
        turns_elapsed: 1,
        supplies: 80,
        breach: 10,
        started_turn: 0,
        engine_work: 0,
    });
    act(
        &mut state,
        &data,
        &id,
        AgentActionKind::Sabotage,
        Some(&enemy),
    );
    let siege = state.settlements[&enemy].siege.clone().unwrap();
    assert_eq!(siege.breach, 35);
    assert_eq!(siege.supplies, 65);
    // Without siege: garrison morale.
    let id2 = place(&mut state, &data, &france, AgentKind::Spy, &home);
    let s = state.settlements.get_mut(&enemy).unwrap();
    s.siege = None;
    let morale_before: Vec<u8> = s.garrison.iter().map(|u| u.morale).collect();
    act(
        &mut state,
        &data,
        &id2,
        AgentActionKind::Sabotage,
        Some(&enemy),
    );
    let morale_after: Vec<u8> = state.settlements[&enemy]
        .garrison
        .iter()
        .map(|u| u.morale)
        .collect();
    for (b, a) in morale_before.iter().zip(&morale_after) {
        assert_eq!(*a, b.saturating_sub(15));
    }
}

#[test]
fn inciting_raises_unrest() {
    let mut data = game_data().clone();
    sure(&mut data);
    let mut state = start(&data, "fac_france", 7);
    let (france, england) = (fac("fac_france"), fac("fac_england"));
    war(&mut state, &france, &england);
    let (_, enemy) = border(&state, &data, &france, &england);
    let id = place(&mut state, &data, &france, AgentKind::Spy, &enemy);
    let province = state.settlement_province(&enemy).unwrap().clone();
    let before = state.provinces[&province].population.peasants.unrest;
    act(&mut state, &data, &id, AgentActionKind::Incite, None);
    let after = state.provinces[&province].population.peasants.unrest;
    let incite =
        match &sim_campaign::agents::rules(&data).actions[&AgentActionKind::Incite].success[0] {
            data_model::ActionEffect::Unrest { delta, .. } => *delta,
            other => panic!("incite should raise unrest, got {other:?}"),
        };
    assert_eq!(i32::from(after), (i32::from(before) + incite).min(100));
}

#[test]
fn counter_espionage_unmasks_foreign_spies_actively_and_passively() {
    let mut data = game_data().clone();
    sure(&mut data);
    let mut state = start(&data, "fac_france", 7);
    let (france, england) = (fac("fac_france"), fac("fac_england"));
    war(&mut state, &france, &england);
    let paris = paris(&state);
    let intruder = place(&mut state, &data, &england, AgentKind::Spy, &paris);
    let hunter = place(&mut state, &data, &france, AgentKind::Spy, &paris);
    // The intruder's odds suffer from the French spy.
    let (chance, _) = state
        .agent_action_odds(&data, &intruder, AgentActionKind::Scout, None, None)
        .unwrap_or((0, 0));
    assert_eq!(chance, 100, "sure rules clamp to 100");
    let report = act(&mut state, &data, &hunter, AgentActionKind::Counter, None);
    assert!(report.text_fr.contains("démasque"), "{}", report.text_fr);
    assert!(state.agent(&intruder).is_none());
    // Passive: an English spy next to a French one is caught at the end of
    // the season when the base chance is certain.
    let intruder = place(&mut state, &data, &england, AgentKind::Spy, &paris);
    let mut rules = agents::rules(&data).clone();
    rules.passive_counter_base = 100;
    rules.passive_counter_cap = 100;
    data.agent_rules = Some(rules);
    let events = state.end_turn(&data);
    assert!(state.agent(&intruder).is_none(), "caught passively");
    assert!(events
        .iter()
        .any(|e| e.kind == sim_campaign::EventKind::Agent && e.text_fr.contains("démasquent")));
}

#[test]
fn counter_spies_lower_the_odds() {
    let data = game_data();
    let mut state = start(data, "fac_france", 7);
    let (france, england) = (fac("fac_france"), fac("fac_england"));
    let paris = paris(&state);
    let intruder = place(&mut state, data, &england, AgentKind::Spy, &paris);
    let (alone, _) = state
        .agent_action_odds(data, &intruder, AgentActionKind::Scout, None, None)
        .unwrap();
    place(&mut state, data, &france, AgentKind::Spy, &paris);
    let (watched, _) = state
        .agent_action_odds(data, &intruder, AgentActionKind::Scout, None, None)
        .unwrap();
    assert_eq!(alone - watched, 8, "5 + 3 per seal of the French spy");
}

#[test]
fn herald_parley_truce_bribe() {
    let mut data = game_data().clone();
    sure(&mut data);
    let mut state = start(&data, "fac_france", 7);
    let (france, england) = (fac("fac_france"), fac("fac_england"));
    let (home, enemy) = border(&state, &data, &france, &england);
    peace(&mut state, &france, &england);
    let herald = place(&mut state, &data, &france, AgentKind::Emissary, &enemy);
    // Parley at peace.
    act(&mut state, &data, &herald, AgentActionKind::Parley, None);
    assert!(state.factions[&england]
        .modifiers
        .iter()
        .any(|m| m.with == france && m.reason_fr == agents::PARLEY_REASON && m.value == 10));
    // Truce and bribe need a war.
    war(&mut state, &france, &england);
    let herald2 = place(&mut state, &data, &france, AgentKind::Emissary, &enemy);
    assert!(state
        .agent_act(
            &data,
            &france,
            &herald2,
            AgentActionKind::Parley,
            None,
            None
        )
        .is_err());
    let report = act(&mut state, &data, &herald2, AgentActionKind::Truce, None);
    assert!(report.success);
    // Bribe: the enemy settlement changes hands (no English army on it).
    for army in state.armies_at(&enemy) {
        state.armies.remove(&army);
    }
    state.agents.agents.remove(&herald);
    state.agents.agents.remove(&herald2);
    // A fresh war (the truce may have ended it).
    war(&mut state, &france, &england);
    let herald3 = place(&mut state, &data, &france, AgentKind::Emissary, &home);
    let treasury = state.factions[&france].treasury;
    let report = act(
        &mut state,
        &data,
        &herald3,
        AgentActionKind::Bribe,
        Some(&enemy),
    );
    assert!(report.success);
    assert_eq!(state.settlements[&enemy].controller, france);
    assert!(
        state.factions[&france].treasury < treasury,
        "the bribe is paid"
    );
}

#[test]
fn herald_buys_back_a_captive_at_a_discount() {
    let mut data = game_data().clone();
    sure(&mut data);
    let mut state = start(&data, "fac_france", 7);
    let (france, england) = (fac("fac_france"), fac("fac_england"));
    let captive: CharacterId = state
        .characters
        .iter()
        .find(|(id, c)| {
            c.faction == france && c.alive && state.factions[&france].ruler.as_ref() != Some(*id)
        })
        .map(|(id, _)| id.clone())
        .unwrap();
    sim_campaign::chronicle::capture_character(
        &mut state,
        &data,
        &captive,
        &england,
        &mut Vec::new(),
    );
    assert!(state.characters[&captive].captive);
    let (_, enemy) = border(&state, &data, &france, &england);
    let herald = place(&mut state, &data, &france, AgentKind::Emissary, &enemy);
    let fair = sim_campaign::ransom::ransom_amount(&state, &data, &captive);
    let (_, cost) = state
        .agent_action_odds(&data, &herald, AgentActionKind::Ransom, None, None)
        .unwrap();
    assert!(cost < fair, "{cost} < {fair}");
    let english_before = state.factions[&england].treasury;
    act(&mut state, &data, &herald, AgentActionKind::Ransom, None);
    assert!(!state.characters[&captive].captive);
    assert_eq!(state.factions[&england].treasury, english_before + cost);
}

#[test]
fn preacher_preach_denounce_curia() {
    let mut data = game_data().clone();
    sure(&mut data);
    let mut state = start(&data, "fac_france", 7);
    let (france, england) = (fac("fac_france"), fac("fac_england"));
    let paris = paris(&state);
    let province = state.settlement_province(&paris).unwrap().clone();
    state.provinces.get_mut(&province).unwrap().heresy = 20;
    state.provinces.get_mut(&province).unwrap().heresy_religion =
        Some(data_model::ReligionId::new("rel_lollard").unwrap());
    let preacher = place(&mut state, &data, &france, AgentKind::Preacher, &paris);
    act(&mut state, &data, &preacher, AgentActionKind::Preach, None);
    assert_eq!(state.provinces[&province].heresy, 12);
    // Denounce: only an enemy (or excommunicated/schismatic) prince.
    peace(&mut state, &france, &england);
    let enemy = city_of(&state, &england);
    let preacher2 = place(&mut state, &data, &france, AgentKind::Preacher, &enemy);
    assert!(state
        .agent_act(
            &data,
            &france,
            &preacher2,
            AgentActionKind::Denounce,
            None,
            None
        )
        .is_err());
    war(&mut state, &france, &england);
    let favor = state.factions[&england].papal_favor;
    act(
        &mut state,
        &data,
        &preacher2,
        AgentActionKind::Denounce,
        None,
    );
    assert_eq!(
        state.factions[&england].papal_favor,
        favor.saturating_sub(3)
    );
    // Curia: Avignon.
    let avignon = SettlementId::new("set_avignon").unwrap();
    state.agents.agents.remove(&preacher);
    let preacher3 = place(&mut state, &data, &france, AgentKind::Preacher, &avignon);
    let favor = state.factions[&france].papal_favor;
    act(&mut state, &data, &preacher3, AgentActionKind::Curia, None);
    assert_eq!(state.factions[&france].papal_favor, (favor + 3).min(100));
    // Out of range: a settlement two edges away.
    let far = paris.clone();
    let err = state
        .agent_act(
            &data,
            &france,
            &preacher3,
            AgentActionKind::Curia,
            Some(&far),
            None,
        )
        .unwrap_err();
    assert!(err.to_string().contains("déjà") || err.to_string().contains("portée"));
}

#[test]
fn failure_can_cost_the_agent_and_experience_raises_the_seal() {
    let mut data = game_data().clone();
    doomed(&mut data);
    let mut state = start(&data, "fac_france", 7);
    let (france, england) = (fac("fac_france"), fac("fac_england"));
    let (_, enemy) = border(&state, &data, &france, &england);
    let spy = place(&mut state, &data, &france, AgentKind::Spy, &enemy);
    let report = act(&mut state, &data, &spy, AgentActionKind::Scout, None);
    assert!(!report.success && report.lost);
    assert!(state.agent(&spy).is_none());
    assert!(report.text_fr.contains("pendu"));
    // Seals: thresholds 2, 5, 9, 14.
    assert_eq!(agents::level_for(&data, 0), 1);
    assert_eq!(agents::level_for(&data, 2), 2);
    assert_eq!(agents::level_for(&data, 13), 4);
    assert_eq!(agents::level_for(&data, 99), 5);
}

#[test]
fn rolls_are_deterministic_and_leave_the_main_stream_alone() {
    let data = game_data();
    let (france, england) = (fac("fac_france"), fac("fac_england"));
    let run = || {
        let mut state = start(data, "fac_france", 7);
        let (_, enemy) = border(&state, data, &france, &england);
        let rng_before = state.rng.clone();
        let mut outcomes = Vec::new();
        for _ in 0..6 {
            let spy = place(&mut state, data, &france, AgentKind::Spy, &enemy);
            let report = act(&mut state, data, &spy, AgentActionKind::Scout, None);
            outcomes.push((report.success, report.lost, report.chance));
            state.agents.agents.remove(&spy);
        }
        assert_eq!(state.rng, rng_before, "the main generator is untouched");
        outcomes
    };
    let first = run();
    assert_eq!(first, run());
    assert!(first.iter().any(|o| o.0), "some scouting succeeds at 75 %");
}

#[test]
fn agents_survive_a_save() {
    let data = game_data();
    let mut state = start(data, "fac_france", 7);
    let france = fac("fac_france");
    let paris = paris(&state);
    state.factions.get_mut(&france).unwrap().treasury = 5_000;
    state
        .recruit_agent(data, &france, &paris, AgentKind::Spy)
        .unwrap();
    // Round trip with agents.
    let json = state.save_json();
    let loaded = CampaignState::load_json(&json).unwrap();
    assert_eq!(loaded.agents, state.agents);
}

#[test]
fn upkeep_is_paid_each_season_and_points_come_back() {
    let data = game_data();
    let mut state = start(data, "fac_france", 7);
    let france = fac("fac_france");
    let paris = paris(&state);
    state.factions.get_mut(&france).unwrap().treasury = 5_000;
    let id = state
        .recruit_agent(data, &france, &paris, AgentKind::Spy)
        .unwrap();
    state.end_turn(data);
    assert!(
        state
            .agents
            .upkeep_last_turn
            .get(&france)
            .copied()
            .unwrap_or(0)
            >= 15
    );
    let agent = state.agent(&id).unwrap();
    assert_eq!(
        agent.movement_points,
        state.agent_movement_allowance(data, AgentKind::Spy)
    );
    assert!(!agent.acted);
    // The action bar lists the four spy actions.
    let bar = state.agent_actions(data, &id);
    assert_eq!(bar.len(), 4);
    assert!(bar.iter().all(|o| o.available || o.reason.is_some()));
}
