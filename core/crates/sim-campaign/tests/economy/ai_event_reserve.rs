//! Faillites (enquête 13/10, proposition 1): an AI faction never picks an
//! event option that would push its treasury below the data-driven reserve.
use data_model::test_support::{fac, game_data};
use data_model::EventId;
use sim_campaign::chronicle::ai_affordable_choice_among;
use sim_campaign::test_support::start;

fn pick(treasury: i64, income: i64) -> usize {
    let data = game_data();
    let mut state = start(data, "fac_france", 1);
    let ai = fac("fac_england");
    let f = state.factions.get_mut(&ai).unwrap();
    f.treasury = treasury;
    f.last_budget.income = income;
    let event = &data.events[&EventId::new("evt_brigands").unwrap()];
    // Option 0 (paying, AI weight 2) is the preferred one when affordable.
    ai_affordable_choice_among(&mut state, data, event, Some(&ai), &[0, 1])
}

#[test]
fn the_ai_keeps_its_reserve_when_choosing_an_event_option() {
    // Poor: cost > treasury - reserve, falls back to the free option.
    assert_eq!(pick(100, 1_000), 1);
    // Rich: the paying option stays affordable after the reserve.
    assert_eq!(pick(1_000_000, 1_000), 0);
}
