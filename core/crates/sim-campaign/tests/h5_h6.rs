//! H5 « Monnaie » and H6 « Rançons et ordres de chevalerie » integration
//! tests: coinage (seigniorage, inflation, priced costs, population, AI),
//! ransoms (amount, full payment, installments and default, parole,
//! cession, captive ruler), chivalric orders (foundation, members, morale,
//! collapse, Garter and Star events), old saves and determinism.

use std::path::PathBuf;

use data_model::{
    CharacterId, ChivalricOrderId, EventEffect, EventId, FactionId, GameData, SocialClass,
};
use sim_campaign::chronicle::{self, EventContext};
use sim_campaign::coinage::{self, CoinageLevel, PRICE_BASE};
use sim_campaign::{
    chivalry, ransom, CampaignState, ChivalryError, CoinageError, EventKind, Order, OrderError,
    RansomError, RansomTerms,
};

fn data() -> GameData {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../data");
    let (data, _warnings) = GameData::load(&root).expect("game data loads");
    data
}

fn fac(id: &str) -> FactionId {
    FactionId::new(id).unwrap()
}

fn chr(id: &str) -> CharacterId {
    CharacterId::new(id).unwrap()
}

fn ord(id: &str) -> ChivalricOrderId {
    ChivalricOrderId::new(id).unwrap()
}

/// France 1337 with the chronicle off (no random noise).
fn france(data: &GameData, seed: u64) -> CampaignState {
    let mut state = CampaignState::new_1337(data, fac("fac_france"), seed).expect("1337 start");
    state.chronicle.disabled = true;
    state
}

fn idle(_: &CampaignState, _: &GameData, _: &FactionId) -> Vec<Order> {
    Vec::new()
}

fn set_coinage(state: &mut CampaignState, data: &GameData, level: CoinageLevel) -> Result<(), OrderError> {
    state.submit_order(data, Order::SetCoinage { level })
}

// ----- H5: coinage -------------------------------------------------------------

#[test]
fn coinage_defaults_and_one_change_per_year() {
    let data = data();
    let mut state = france(&data, 1);
    let f = &state.factions[&fac("fac_france")];
    assert_eq!(f.coinage, CoinageLevel::Sound);
    assert_eq!(f.price_level, PRICE_BASE);
    assert_eq!(
        set_coinage(&mut state, &data, CoinageLevel::Sound),
        Err(OrderError::Coinage(CoinageError::Unchanged))
    );
    set_coinage(&mut state, &data, CoinageLevel::Debased).unwrap();
    assert!(matches!(
        set_coinage(&mut state, &data, CoinageLevel::Sound),
        Err(OrderError::Coinage(CoinageError::AlreadyChangedThisYear(1337)))
    ));
    // The next year allows a new change.
    for _ in 0..4 {
        state.end_turn_with(&data, idle);
    }
    set_coinage(&mut state, &data, CoinageLevel::Sound).unwrap();
}

#[test]
fn debasement_yields_seigniorage_and_inflation() {
    let data = data();
    let mut sound = france(&data, 2);
    let mut debased = sound.clone();
    let france_id = fac("fac_france");
    let tax = debased.faction_income_effective(&data, &france_id);
    let forecast = coinage::seigniorage_for(&debased, &data, &france_id, CoinageLevel::Debased);
    assert_eq!(forecast, (tax as f64 * 0.15).round() as i64);
    assert!(
        coinage::seigniorage_for(&debased, &data, &france_id, CoinageLevel::HeavilyDebased)
            > forecast
    );
    set_coinage(&mut debased, &data, CoinageLevel::Debased).unwrap();
    sound.end_turn_with(&data, idle);
    debased.end_turn_with(&data, idle);
    let d = &debased.factions[&france_id];
    let s = &sound.factions[&france_id];
    assert!(d.seigniorage_last_turn > 0);
    assert_eq!(s.seigniorage_last_turn, 0);
    // Short term: more money.
    assert!(d.treasury > s.treasury, "{} vs {}", d.treasury, s.treasury);
    assert_eq!(d.price_level, PRICE_BASE + 3);
    assert_eq!(s.price_level, PRICE_BASE);
    let events = debased.events();
    assert!(events.iter().any(|e| e.kind == EventKind::Coinage));
}

#[test]
fn heavy_debasement_is_ruinous_in_the_long_run() {
    let data = data();
    let mut sound = france(&data, 3);
    let mut heavy = sound.clone();
    set_coinage(&mut heavy, &data, CoinageLevel::HeavilyDebased).unwrap();
    for _ in 0..24 {
        sound.end_turn_with(&data, idle);
        heavy.end_turn_with(&data, idle);
    }
    let france_id = fac("fac_france");
    let h = &heavy.factions[&france_id];
    assert!(h.price_level >= 250, "price level {}", h.price_level);
    // The upkeep outruns the seigniorage.
    let h_net = h.income_last_turn - h.upkeep_last_turn;
    let s = &sound.factions[&france_id];
    let s_net = s.income_last_turn - s.upkeep_last_turn;
    assert!(h_net < s_net, "{h_net} vs {s_net}");
}

#[test]
fn prices_scale_recruitment_upkeep_and_construction() {
    let data = data();
    let mut state = france(&data, 4);
    let france_id = fac("fac_france");
    let province = state.factions[&france_id].capital.clone();
    let unit_type = state.recruitable(&data, &province)[0].unit_type.clone();
    let recruit = |s: &CampaignState| {
        s.recruitable(&data, &province)
            .into_iter()
            .find(|o| o.unit_type == unit_type)
            .unwrap()
            .cost
    };
    let build = |s: &CampaignState| s.buildable(&data, &province)[0].cost;
    let (recruit_before, build_before) = (recruit(&state), build(&state));
    let upkeep_before = state.faction_army_upkeep(&data, &france_id);
    let buildings_before = state.faction_building_upkeep(&data, &france_id);
    state.factions.get_mut(&france_id).unwrap().price_level = 200;
    assert!((i64::from(recruit(&state)) - 2 * i64::from(recruit_before)).abs() <= 1);
    assert_eq!(build(&state), 2 * build_before);
    assert!((state.faction_army_upkeep(&data, &france_id) - 2 * upkeep_before).abs() <= 1);
    assert!((state.faction_building_upkeep(&data, &france_id) - 2 * buildings_before).abs() <= 1);
}

#[test]
fn inflation_angers_burghers_and_clergy_and_strong_money_deflates() {
    let data = data();
    let france_id = fac("fac_france");
    let mut state = france(&data, 5);
    state.factions.get_mut(&france_id).unwrap().price_level = 160;
    let burghers = coinage::class_effects(&state, &france_id, SocialClass::Burghers);
    let clergy = coinage::class_effects(&state, &france_id, SocialClass::Clergy);
    let peasants = coinage::class_effects(&state, &france_id, SocialClass::Peasants);
    assert_eq!(burghers.unrest.flat, 10.0);
    assert_eq!(clergy.wealth.flat, -7.5);
    assert_eq!(peasants.unrest.flat, 0.0);
    let mut calm = france(&data, 5);
    for _ in 0..6 {
        state.factions.get_mut(&france_id).unwrap().price_level = 160;
        state.end_turn_with(&data, idle);
        calm.end_turn_with(&data, idle);
    }
    let capital = state.factions[&france_id].capital.clone();
    let unrest = |s: &CampaignState| s.provinces[&capital].population.burghers.unrest;
    assert!(unrest(&state) > unrest(&calm));

    // Strong money: prices fall 2 points a season, recoinage is paid,
    // burghers are pleased and the ruler gains prestige.
    let mut strong = france(&data, 6);
    strong.factions.get_mut(&france_id).unwrap().price_level = 110;
    set_coinage(&mut strong, &data, CoinageLevel::Strong).unwrap();
    let ruler = strong.factions[&france_id].ruler.clone().unwrap();
    let prestige = strong.characters[&ruler].prestige;
    strong.end_turn_with(&data, idle);
    let f = &strong.factions[&france_id];
    assert_eq!(f.price_level, 108);
    assert!(f.recoinage_last_turn > 0);
    assert!(strong.characters[&ruler].prestige > prestige);
    assert_eq!(
        coinage::class_effects(&strong, &france_id, SocialClass::Burghers).unrest.flat,
        -6.0 + 8.0 / 6.0
    );
    // Never below 100.
    assert_eq!(coinage::next_price_level(101, CoinageLevel::Strong), PRICE_BASE);
}

#[test]
fn ai_coinage_policy() {
    let data = data();
    let mut state = france(&data, 7);
    let england = fac("fac_england");
    state.factions.get_mut(&england).unwrap().treasury = -100;
    assert_eq!(
        coinage::ai_choose_coinage(&state, &data, &england),
        vec![Order::SetCoinage {
            level: CoinageLevel::Debased
        }]
    );
    let f = state.factions.get_mut(&england).unwrap();
    f.treasury = 10_000_000;
    f.coinage = CoinageLevel::Debased;
    f.price_level = 150;
    assert_eq!(
        coinage::ai_choose_coinage(&state, &data, &england),
        vec![Order::SetCoinage {
            level: CoinageLevel::Strong
        }]
    );
    let f = state.factions.get_mut(&england).unwrap();
    f.treasury = 100;
    f.price_level = 100;
    assert_eq!(
        coinage::ai_choose_coinage(&state, &data, &england),
        vec![Order::SetCoinage {
            level: CoinageLevel::Sound
        }]
    );
    state.factions.get_mut(&england).unwrap().coinage_changed_year = Some(state.year());
    assert!(coinage::ai_choose_coinage(&state, &data, &england).is_empty());
}
