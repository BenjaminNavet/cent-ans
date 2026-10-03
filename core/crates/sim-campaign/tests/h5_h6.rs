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

fn set_coinage(
    state: &mut CampaignState,
    data: &GameData,
    level: CoinageLevel,
) -> Result<(), OrderError> {
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
        Err(OrderError::Coinage(CoinageError::AlreadyChangedThisYear(
            1337
        )))
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
    let capital = state.factions[&france_id].capital.clone();
    let province = state.province_city_id(&capital).unwrap().clone();
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
    let sum = |s: &CampaignState, gauge: fn(&data_model::PopulationClass) -> u8| -> u32 {
        s.provinces
            .iter()
            .filter(|(id, _)| s.controls_province(&france_id, id))
            .map(|(_, p)| u32::from(gauge(&p.population.burghers)))
            .sum()
    };
    assert!(sum(&state, |c| c.unrest) > sum(&calm, |c| c.unrest));
    assert!(sum(&state, |c| c.wealth) < sum(&calm, |c| c.wealth));

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
        coinage::class_effects(&strong, &france_id, SocialClass::Burghers)
            .unrest
            .flat,
        -6.0 + 8.0 / 6.0
    );
    // Never below 100.
    assert_eq!(
        coinage::next_price_level(101, CoinageLevel::Strong),
        PRICE_BASE
    );
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
    state
        .factions
        .get_mut(&england)
        .unwrap()
        .coinage_changed_year = Some(state.year());
    assert!(coinage::ai_choose_coinage(&state, &data, &england).is_empty());
}

// ----- H6: ransoms ----------------------------------------------------------------

/// Makes `character` the prisoner of `captor` (chronicle capture).
fn capture(state: &mut CampaignState, data: &GameData, character: &str, captor: &str) {
    chronicle::capture_character(state, data, &chr(character), &fac(captor), &mut Vec::new());
    assert!(state.characters[&chr(character)].captive);
}

fn pay(
    state: &mut CampaignState,
    data: &GameData,
    character: &str,
    installments: u32,
) -> Result<(), OrderError> {
    state.submit_order(
        data,
        Order::PayRansom {
            character: chr(character),
            installments,
        },
    )
}

#[test]
fn ransom_follows_rank_prestige_and_wealth() {
    let data = data();
    let mut state = france(&data, 10);
    let king = chr("chr_philippe_vi");
    let heir = chr("chr_jean_de_normandie");
    assert_eq!(
        ransom::captive_rank(&state, &king),
        ransom::CaptiveRank::Sovereign
    );
    assert_eq!(
        ransom::captive_rank(&state, &heir),
        ransom::CaptiveRank::Heir
    );
    // A6-L3: the income cap is lifted to check the formula itself.
    let mut data = data;
    data.economy_rules.ransom.income_cap_percent = 1_000_000;
    let king_ransom = ransom::ransom_amount(&state, &data, &king);
    let heir_ransom = ransom::ransom_amount(&state, &data, &heir);
    assert!(king_ransom > heir_ransom, "{king_ransom} vs {heir_ransom}");
    state.characters.get_mut(&king).unwrap().prestige = 100;
    let famous = ransom::ransom_amount(&state, &data, &king);
    assert!(
        (famous - 2 * king_ransom).abs() <= 50,
        "{famous} vs {king_ransom}"
    );
    assert_eq!(famous % 50, 0);
    // A poorer realm pays less for its king.
    let scot = ransom::ransom_amount(&state, &data, &chr("chr_david_ii"));
    assert!(scot < king_ransom);
    assert_eq!(ransom::installment_plan(1000, 1), (1000, 1000));
    assert_eq!(ransom::installment_plan(1000, 4), (1100, 275));
}

#[test]
fn full_ransom_frees_the_captive_once() {
    let data = data();
    let mut state = france(&data, 11);
    capture(&mut state, &data, "chr_jean_de_normandie", "fac_england");
    let amount = ransom::ransom_amount(&state, &data, &chr("chr_jean_de_normandie"));
    let france_id = fac("fac_france");
    let england = fac("fac_england");
    state.factions.get_mut(&france_id).unwrap().treasury = 0;
    assert!(matches!(
        pay(&mut state, &data, "chr_jean_de_normandie", 1),
        Err(OrderError::Ransom(RansomError::InsufficientFunds { .. }))
    ));
    state.factions.get_mut(&france_id).unwrap().treasury = amount + 10;
    let english = state.factions[&england].treasury;
    pay(&mut state, &data, "chr_jean_de_normandie", 1).unwrap();
    let heir = &state.characters[&chr("chr_jean_de_normandie")];
    assert!(!heir.captive && heir.captor.is_none() && heir.ransom_terms.is_none());
    assert_eq!(state.factions[&france_id].treasury, 10);
    assert_eq!(state.factions[&england].treasury, english + amount);
    // No double release: neither the order nor a ransom event pays again.
    assert_eq!(
        pay(&mut state, &data, "chr_jean_de_normandie", 1),
        Err(OrderError::Ransom(RansomError::NotCaptive))
    );
    let effect = EventEffect::ReleaseCharacter {
        id: data_model::CharacterRef::Id(chr("chr_jean_de_normandie")),
        faction: None,
        ransom: 5000,
    };
    let ctx = EventContext {
        faction: Some(france_id.clone()),
        province: None,
    };
    chronicle::apply_effect(&mut state, &data, &effect, &ctx, &mut Vec::new());
    assert_eq!(state.factions[&france_id].treasury, 10);
}

#[test]
fn installments_are_paid_yearly_and_defaults_are_punished() {
    let data = data();
    let mut state = france(&data, 12);
    let france_id = fac("fac_france");
    let england = fac("fac_england");
    capture(&mut state, &data, "chr_jean_de_normandie", "fac_england");
    let amount = ransom::ransom_amount(&state, &data, &chr("chr_jean_de_normandie"));
    let (total, installment) = ransom::installment_plan(amount, 3);
    pay(&mut state, &data, "chr_jean_de_normandie", 3).unwrap();
    assert!(!state.characters[&chr("chr_jean_de_normandie")].captive);
    let debt = state.factions[&france_id].ransom_debts[0].clone();
    assert_eq!(debt.remaining, total - installment);
    assert_eq!(debt.creditor, england);
    assert_eq!(
        ransom::ransom_debt_total(&state, &france_id),
        total - installment
    );
    // Year one: the installment is paid (due at turn 4).
    state.factions.get_mut(&france_id).unwrap().treasury = 1_000_000;
    for _ in 0..5 {
        state.end_turn_with(&data, idle);
    }
    let debt = state.factions[&france_id].ransom_debts[0].clone();
    assert_eq!(debt.remaining, total - 2 * installment);
    assert_eq!(debt.missed, 0);
    // Year two: an empty treasury misses it (+10 %, prestige, opinion).
    let ruler = state.factions[&france_id].ruler.clone().unwrap();
    state.factions.get_mut(&france_id).unwrap().treasury = -1_000_000;
    let prestige = state.characters[&ruler].prestige;
    for _ in 0..4 {
        state.end_turn_with(&data, idle);
    }
    let after = state.factions[&france_id].ransom_debts[0].clone();
    assert_eq!(after.missed, 1);
    assert!(after.remaining > debt.remaining);
    // −5 prestige (court prestige may add its yearly gain the same winter).
    let court = sim_campaign::dynasty::yearly_court_prestige(&state, &data, &france_id);
    assert!(state.characters[&ruler].prestige <= prestige + court - ransom::DEFAULT_PRESTIGE);
    assert!(state
        .events()
        .iter()
        .any(|e| e.kind == EventKind::Ransom && e.text_fr.contains("impayée")));
    assert!(state.factions[&england]
        .modifiers
        .iter()
        .any(|m| m.with == france_id && m.value < 0));
    assert!(
        state
            .events()
            .iter()
            .chain(state.pending_events.iter())
            .count()
            > 0
    );
}

#[test]
fn captor_terms_parole_hold_and_cession() {
    let data = data();
    let mut state = CampaignState::new_1337(&data, fac("fac_england"), 13).unwrap();
    state.chronicle.disabled = true;
    let france_id = fac("fac_france");
    let england = fac("fac_england");
    capture(&mut state, &data, "chr_jean_de_normandie", "fac_england");
    // Only the captor sets terms.
    assert_eq!(
        state.apply_order(
            &data,
            &france_id,
            Order::SetRansomTerms {
                character: chr("chr_jean_de_normandie"),
                terms: RansomTerms::Hold,
            }
        ),
        Err(OrderError::Ransom(RansomError::NotYourPrisoner))
    );
    state
        .submit_order(
            &data,
            Order::SetRansomTerms {
                character: chr("chr_jean_de_normandie"),
                terms: RansomTerms::Hold,
            },
        )
        .unwrap();
    assert_eq!(
        state.apply_order(
            &data,
            &france_id,
            Order::PayRansom {
                character: chr("chr_jean_de_normandie"),
                installments: 1,
            }
        ),
        Err(OrderError::Ransom(RansomError::Held))
    );
    // Cession of a border province.
    let cedable = ransom::cedable_provinces(&state, &data, &france_id, &england);
    assert!(!cedable.is_empty(), "France borders English lands");
    let province = cedable[0].clone();
    state
        .submit_order(
            &data,
            Order::SetRansomTerms {
                character: chr("chr_jean_de_normandie"),
                terms: RansomTerms::Province {
                    province: province.clone(),
                },
            },
        )
        .unwrap();
    state
        .apply_order(
            &data,
            &france_id,
            Order::PayRansom {
                character: chr("chr_jean_de_normandie"),
                installments: 0,
            },
        )
        .unwrap();
    assert_eq!(state.province_owner(&province), Some(&england));
    assert_eq!(state.province_controller(&province), Some(&england));
    assert!(!state.characters[&chr("chr_jean_de_normandie")].captive);

    // Parole: prestige for the captor's ruler, goodwill from the freed side.
    capture(&mut state, &data, "chr_david_ii", "fac_england");
    let edward = chr("chr_edward_iii");
    let prestige = state.characters[&edward].prestige;
    state
        .submit_order(
            &data,
            Order::ReleaseOnParole {
                character: chr("chr_david_ii"),
            },
        )
        .unwrap();
    assert!(!state.characters[&chr("chr_david_ii")].captive);
    assert_eq!(
        state.characters[&edward].prestige,
        prestige + ransom::PAROLE_PRESTIGE
    );
    assert!(state.factions[&fac("fac_scotland")]
        .modifiers
        .iter()
        .any(|m| m.with == england && m.value > 0));
}

#[test]
fn a_captive_king_weighs_on_his_realm() {
    let data = data();
    let mut state = france(&data, 14);
    let france_id = fac("fac_france");
    capture(&mut state, &data, "chr_philippe_vi", "fac_england");
    let king = chr("chr_philippe_vi");
    let prestige = state.characters[&king].prestige;
    let events = state.end_turn_with(&data, idle);
    assert!(state.factions[&france_id].regency);
    assert!(events
        .iter()
        .any(|e| e.kind == EventKind::Regency && e.text_fr.contains("captif")));
    assert_eq!(
        state.characters[&king].prestige,
        prestige - ransom::CAPTIVE_RULER_PRESTIGE
    );
    // Freed: the regency ends.
    let amount = ransom::ransom_amount(&state, &data, &king);
    state.factions.get_mut(&france_id).unwrap().treasury = amount;
    pay(&mut state, &data, "chr_philippe_vi", 1).unwrap();
    state.end_turn_with(&data, idle);
    assert!(!state.factions[&france_id].regency);
}

#[test]
fn ai_pays_ransoms_and_frees_knights_at_peace() {
    let data = data();
    let mut state = CampaignState::new_1337(&data, fac("fac_england"), 15).unwrap();
    state.chronicle.disabled = true;
    let france_id = fac("fac_france");
    capture(&mut state, &data, "chr_jean_de_normandie", "fac_england");
    state.factions.get_mut(&france_id).unwrap().treasury = 1_000_000;
    let orders = ransom::ai_ransom_orders(&state, &data, &france_id);
    assert_eq!(
        orders,
        vec![Order::PayRansom {
            character: chr("chr_jean_de_normandie"),
            installments: 1
        }]
    );
    for order in orders {
        state.apply_order(&data, &france_id, order).unwrap();
    }
    assert!(!state.characters[&chr("chr_jean_de_normandie")].captive);
}

// ----- H6: chivalric orders ----------------------------------------------------

fn found(state: &mut CampaignState, data: &GameData, order: &str) -> Result<(), OrderError> {
    state.submit_order(data, Order::FoundChivalricOrder { order: ord(order) })
}

#[test]
fn founding_an_order_checks_faction_year_prestige_and_money() {
    let data = data();
    let mut state = france(&data, 20);
    let france_id = fac("fac_france");
    assert_eq!(
        found(&mut state, &data, "ord_garter"),
        Err(OrderError::Chivalry(ChivalryError::OtherFaction))
    );
    assert_eq!(
        found(&mut state, &data, "ord_star"),
        Err(OrderError::Chivalry(ChivalryError::TooEarly(1351)))
    );
    state.year = 1352;
    let ruler = state.factions[&france_id].ruler.clone().unwrap();
    state.characters.get_mut(&ruler).unwrap().prestige = 0;
    assert!(matches!(
        found(&mut state, &data, "ord_star"),
        Err(OrderError::Chivalry(
            ChivalryError::NotEnoughPrestige { .. }
        ))
    ));
    state.characters.get_mut(&ruler).unwrap().prestige = 20;
    state.factions.get_mut(&france_id).unwrap().treasury = 100;
    assert!(matches!(
        found(&mut state, &data, "ord_star"),
        Err(OrderError::Chivalry(
            ChivalryError::InsufficientFunds { .. }
        ))
    ));
    state.factions.get_mut(&france_id).unwrap().treasury = 10_000;
    found(&mut state, &data, "ord_star").unwrap();
    let star = &data.chivalric_orders[&ord("ord_star")];
    assert_eq!(
        state.factions[&france_id].treasury,
        10_000 - i64::from(star.cost)
    );
    assert_eq!(
        state.characters[&ruler].prestige,
        20 + star.founder_prestige
    );
    assert_eq!(
        found(&mut state, &data, "ord_star"),
        Err(OrderError::Chivalry(ChivalryError::AlreadyFounded))
    );
    // Other factions without a historical order found the generic one.
    let options = chivalry::orders_for(&data, &fac("fac_scotland"));
    assert_eq!(options.len(), 1);
    assert!(options[0].faction.is_none());
}

#[test]
fn members_are_named_and_gain_loyalty_and_morale() {
    let data = data();
    let mut state = CampaignState::new_1337(&data, fac("fac_england"), 21).unwrap();
    state.chronicle.disabled = true;
    let england = fac("fac_england");
    let edward = chr("chr_edward_iii");
    state.characters.get_mut(&edward).unwrap().prestige = 20;
    state.year = 1348;
    found(&mut state, &data, "ord_garter").unwrap();
    let order = state.factions[&england].chivalric_order.clone().unwrap();
    let garter = &data.chivalric_orders[&ord("ord_garter")];
    assert!(!order.members.is_empty());
    assert!(order.members.len() <= garter.members as usize);
    assert!(
        !order.members.contains(&edward),
        "the king is the sovereign"
    );
    let member = order.members[0].clone();
    assert_eq!(
        chivalry::member_morale(&state, &data, &member),
        f64::from(garter.member_morale)
    );
    assert_eq!(chivalry::member_morale(&state, &data, &edward), 0.0);
    let effects = sim_campaign::skills::character_effects(&state, &data, &member);
    assert!(effects.army_morale.flat >= f64::from(garter.member_morale));
    // Best first: nobody outside the order has more merit than a member.
    let worst = order
        .members
        .iter()
        .map(|m| chivalry::merit(&state, m))
        .min()
        .unwrap();
    assert!(
        order.members.len() < garter.members as usize
            || state.characters.iter().all(|(id, c)| {
                order.members.contains(id)
                    || c.faction != england
                    || !c.alive
                    || id == &edward
                    || chivalry::merit(&state, id) <= worst
            })
    );
    // A captured member is replaced next season (if anyone is left).
    let prestige = state.characters[&edward].prestige;
    chronicle::capture_character(
        &mut state,
        &data,
        &member,
        &fac("fac_france"),
        &mut Vec::new(),
    );
    state.end_turn_with(&data, idle);
    let order = state.factions[&england].chivalric_order.clone().unwrap();
    assert!(!order.members.contains(&member));
    assert!(!order.collapsed || order.members.len() < 2);
    let _ = prestige;
}

#[test]
fn an_order_losing_half_its_members_collapses() {
    let data = data();
    let mut state = france(&data, 22);
    let france_id = fac("fac_france");
    state.year = 1352;
    let ruler = state.factions[&france_id].ruler.clone().unwrap();
    state.characters.get_mut(&ruler).unwrap().prestige = 30;
    state.factions.get_mut(&france_id).unwrap().treasury = 100_000;
    found(&mut state, &data, "ord_star").unwrap();
    let members = state.factions[&france_id]
        .chivalric_order
        .as_ref()
        .unwrap()
        .members
        .clone();
    assert!(members.len() >= 2);
    // Mauron: half of the companions fall.
    for m in members.iter().take(members.len().div_ceil(2)) {
        state.characters.get_mut(m).unwrap().alive = false;
    }
    let prestige = state.characters[&ruler].prestige;
    let events = state.end_turn_with(&data, idle);
    let order = state.factions[&france_id].chivalric_order.clone().unwrap();
    assert!(order.collapsed);
    assert!(events.iter().any(|e| e.kind == EventKind::Chivalry));
    assert!(state.characters[&ruler].prestige <= prestige - chivalry::COLLAPSE_PRESTIGE + 4);
    let survivor = members.last().unwrap();
    assert_eq!(chivalry::member_morale(&state, &data, survivor), 0.0);
}

fn choose_found_option(state: &mut CampaignState, data: &GameData, event: &str, faction: &str) {
    let definition = &data.events[&EventId::new(event).unwrap()];
    let ctx = EventContext {
        faction: Some(fac(faction)),
        province: None,
    };
    for effect in &definition.options[0].effects {
        chronicle::apply_effect(state, data, effect, &ctx, &mut Vec::new());
    }
}

#[test]
fn garter_and_star_events_found_their_orders() {
    let data = data();
    let mut state = france(&data, 23);
    choose_found_option(
        &mut state,
        &data,
        "evt_ordre_de_la_jarretiere",
        "fac_england",
    );
    choose_found_option(&mut state, &data, "evt_ordre_de_l_etoile", "fac_france");
    let garter = state.factions[&fac("fac_england")]
        .chivalric_order
        .clone()
        .unwrap();
    let star = state.factions[&fac("fac_france")]
        .chivalric_order
        .clone()
        .unwrap();
    assert_eq!(garter.order, ord("ord_garter"));
    assert_eq!(star.order, ord("ord_star"));
    assert!(!garter.members.is_empty() && !star.members.is_empty());
    // A second foundation is a no-op.
    choose_found_option(
        &mut state,
        &data,
        "evt_ordre_de_la_jarretiere",
        "fac_england",
    );
    assert_eq!(
        state.factions[&fac("fac_england")].chivalric_order,
        Some(garter)
    );
}

// ----- saves and determinism ---------------------------------------------------------

#[test]
fn h5_h6_state_survives_saves_and_old_saves_load() {
    let data = data();
    let mut state = france(&data, 30);
    set_coinage(&mut state, &data, CoinageLevel::Debased).unwrap();
    capture(&mut state, &data, "chr_jean_de_normandie", "fac_england");
    state.factions.get_mut(&fac("fac_france")).unwrap().treasury = 1_000_000;
    pay(&mut state, &data, "chr_jean_de_normandie", 3).unwrap();
    state.end_turn_with(&data, idle);
    let json = state.save_json();
    let loaded = CampaignState::load_json(&json).unwrap();
    assert_eq!(loaded, state);

    // A save written before H5/H6 lacks every new field.
    let mut value: serde_json::Value = serde_json::from_str(&json).unwrap();
    for faction in value["factions"].as_object_mut().unwrap().values_mut() {
        let f = faction.as_object_mut().unwrap();
        for key in [
            "coinage",
            "price_level",
            "coinage_changed_year",
            "seigniorage_last_turn",
            "recoinage_last_turn",
            "ransom_debts",
            "chivalric_order",
        ] {
            f.remove(key);
        }
    }
    for character in value["characters"].as_object_mut().unwrap().values_mut() {
        character.as_object_mut().unwrap().remove("ransom_terms");
    }
    let old = CampaignState::load_json(&value.to_string()).expect("old save loads");
    let f = &old.factions[&fac("fac_france")];
    assert_eq!(f.coinage, CoinageLevel::Sound);
    assert_eq!(f.price_level, PRICE_BASE);
    assert!(f.ransom_debts.is_empty() && f.chivalric_order.is_none());
}

#[test]
fn campaign_with_h5_h6_ai_is_deterministic_and_valid() {
    let data = data();
    let mut a = CampaignState::new_1337(&data, fac("fac_france"), 31).unwrap();
    let mut b = a.clone();
    for _ in 0..12 {
        a.end_turn(&data);
        b.end_turn(&data);
    }
    assert_eq!(a.save_json(), b.save_json());
    for faction in a.factions.keys().cloned().collect::<Vec<_>>() {
        let mut probe = a.clone();
        let orders: Vec<Order> = coinage::ai_choose_coinage(&a, &data, &faction)
            .into_iter()
            .chain(ransom::ai_ransom_orders(&a, &data, &faction))
            .chain(chivalry::ai_found_order(&a, &data, &faction))
            .collect();
        for order in orders {
            probe.apply_order(&data, &faction, order).unwrap();
        }
    }
}
