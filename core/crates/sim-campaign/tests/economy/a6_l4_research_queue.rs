//! A6-L4: idle research reserve (capped) and research queue.

use data_model::{FactionId, GameData, TechnologyId};
use sim_campaign::test_support::idle;
use sim_campaign::{CampaignState, Order, OrderError, ResearchError};

use data_model::test_support::game_data;

fn fac() -> FactionId {
    FactionId::new("fac_france").unwrap()
}

fn tech(id: &str) -> TechnologyId {
    TechnologyId::new(id).unwrap()
}

fn state(data: &GameData) -> CampaignState {
    CampaignState::new_1337(data, fac(), 4).expect("1337 start")
}

fn queue(s: &mut CampaignState, d: &GameData, id: &str) -> Result<(), OrderError> {
    s.submit_order(
        d,
        Order::QueueResearch {
            technology: tech(id),
        },
    )
}

#[test]
fn idle_points_pile_up_in_a_capped_reserve() {
    let data = game_data();
    let mut s = state(data);
    let points = s.research_points_per_turn(data, &fac());
    assert!(points > 0);
    let cap_turns = data.economy_rules.research_reserve_turns;
    assert!(cap_turns >= 1);
    s.end_turn_with(data, idle);
    assert_eq!(s.factions[&fac()].research_progress, points);
    for _ in 0..(cap_turns + 3) {
        s.end_turn_with(data, idle);
    }
    let p = s.research_points_per_turn(data, &fac());
    let (reserve, cap) = s.research_reserve(data, &fac());
    assert_eq!(cap, p * cap_turns);
    assert_eq!(reserve, cap);
    // The reserve is poured into the next research.
    s.submit_order(
        data,
        Order::Research {
            technology: tech("tech_pavise"),
        },
    )
    .unwrap();
    assert_eq!(s.factions[&fac()].research_progress, cap);
}

#[test]
fn queue_chains_researches() {
    let data = game_data();
    let mut s = state(data);
    queue(&mut s, data, "tech_pavise").unwrap(); // idle: starts
    assert_eq!(s.factions[&fac()].research, Some(tech("tech_pavise")));
    assert!(s.factions[&fac()].research_queue.is_empty());
    queue(&mut s, data, "tech_gunpowder").unwrap();
    assert_eq!(
        s.factions[&fac()].research_queue,
        vec![tech("tech_gunpowder")]
    );
    // Same technology twice: no-op.
    queue(&mut s, data, "tech_gunpowder").unwrap();
    assert_eq!(s.factions[&fac()].research_queue.len(), 1);
    // Almost finish the current research.
    let cost =
        sim_campaign::research::effective_cost(&data.technologies[&tech("tech_pavise")], s.year());
    s.factions.get_mut(&fac()).unwrap().research_progress = cost - 1;
    s.end_turn_with(data, idle);
    let f = &s.factions[&fac()];
    assert!(f.technologies.contains(&tech("tech_pavise")));
    assert_eq!(f.research, Some(tech("tech_gunpowder")));
    assert!(f.research_queue.is_empty());
    // Surplus carried into the next research.
    assert!(f.research_progress > 0);
}

#[test]
fn queue_is_bounded_and_dequeue_works() {
    let data = game_data();
    let mut s = state(data);
    queue(&mut s, data, "tech_pavise").unwrap();
    let max = data.economy_rules.research_queue_max as usize;
    let candidates: Vec<String> = data
        .technologies
        .values()
        .filter(|t| {
            t.prerequisites.is_empty()
                && t.id != tech("tech_pavise")
                && !s.factions[&fac()].technologies.contains(&t.id)
        })
        .map(|t| t.id.to_string())
        .collect();
    assert!(candidates.len() > max);
    for id in candidates.iter().take(max) {
        queue(&mut s, data, id).unwrap();
    }
    let err = queue(&mut s, data, &candidates[max]).unwrap_err();
    assert!(matches!(
        err,
        OrderError::Research(ResearchError::QueueFull)
    ));
    s.submit_order(
        data,
        Order::DequeueResearch {
            technology: tech(&candidates[0]),
        },
    )
    .unwrap();
    assert_eq!(s.factions[&fac()].research_queue.len(), max - 1);
}

#[test]
fn old_saves_without_the_queue_load() {
    let data = game_data();
    let mut s = state(data);
    let json = serde_json::to_value(&s).unwrap();
    // An empty queue is not serialised at all: an old save looks the same.
    assert!(!json.to_string().contains("research_queue"));
    let back: CampaignState = serde_json::from_value(json).unwrap();
    assert!(back.factions[&fac()].research_queue.is_empty());
    queue(&mut s, data, "tech_pavise").unwrap();
    queue(&mut s, data, "tech_gunpowder").unwrap();
    let json = serde_json::to_string(&s).unwrap();
    let back: CampaignState = serde_json::from_str(&json).unwrap();
    assert_eq!(
        back.factions[&fac()].research_queue,
        vec![tech("tech_gunpowder")]
    );
}
