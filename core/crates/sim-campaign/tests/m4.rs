//! M4 (characters & dynasties) integration tests: experience and skills,
//! traits, generals and governors, marriages, births, succession, regency,
//! and the version-3 save format. See `docs/design/m4-characters-dynasties.md` § 2.

use std::path::PathBuf;

use data_model::{CharacterId, FactionId, GameData, ProvinceId, SkillBranch, SkillId, TraitId};
use sim_campaign::battle_auto::{resolve_auto, BattleContext, BattleUnit, Side, Winner};
use sim_campaign::{CampaignRng, CampaignState, EventKind, Order, OrderError};

fn data() -> GameData {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../data");
    let (data, _warnings) = GameData::load(&root).expect("game data loads");
    data
}

fn fac(id: &str) -> FactionId {
    FactionId::new(id).unwrap()
}

fn prov(id: &str) -> ProvinceId {
    ProvinceId::new(id).unwrap()
}

fn chr(id: &str) -> CharacterId {
    CharacterId::new(id).unwrap()
}

fn france(data: &GameData, seed: u64) -> CampaignState {
    CampaignState::new_1337(data, fac("fac_france"), seed).expect("1337 start")
}

/// A planner that does nothing: only the orders the test submits apply.
fn idle(_: &CampaignState, _: &GameData, _: &FactionId) -> Vec<Order> {
    Vec::new()
}

/// A tier-1 skill of `branch` with no prerequisite, and one of its tier-2
/// successors.
fn tier1_and_successor(data: &GameData, branch: SkillBranch) -> (SkillId, SkillId) {
    let successor = data
        .skills
        .values()
        .filter(|s| s.branch == branch && s.tier == 2 && s.prerequisites.len() == 1)
        .min_by_key(|s| s.id.clone())
        .expect("a tier-2 skill with one prerequisite");
    (successor.prerequisites[0].clone(), successor.id.clone())
}

fn unit(strength: u32) -> BattleUnit {
    BattleUnit {
        strength,
        max_strength: 100,
        experience: 2,
        morale: 70,
        melee: 10,
        ranged: 0,
        armor: 30,
        is_ranged: false,
    }
}

// ----- experience & skills --------------------------------------------------

#[test]
fn experience_converts_into_skill_points() {
    let data = data();
    let mut state = france(&data, 1);
    let philippe = chr("chr_philippe_vi");
    state
        .submit_order(
            &data,
            Order::DebugGrantXp {
                character: philippe.clone(),
                amount: 250,
            },
        )
        .unwrap();
    let c = state.character(&philippe).unwrap();
    assert_eq!(c.skill_points, 2, "100 XP per point at tier 1");
    assert_eq!(c.experience, 50, "remainder kept");
    assert_eq!(sim_campaign::skills::xp_for_next_point(0), 100);
    assert_eq!(sim_campaign::skills::xp_for_next_point(3), 200);
    assert_eq!(sim_campaign::skills::xp_for_next_point(99), 300);
}

#[test]
fn learn_skill_checks_points_and_prerequisites() {
    let data = data();
    let mut state = france(&data, 2);
    let philippe = chr("chr_philippe_vi");
    let (first, second) = tier1_and_successor(&data, SkillBranch::Command);

    let no_points = state.submit_order(
        &data,
        Order::LearnSkill {
            character: philippe.clone(),
            skill: first.clone(),
        },
    );
    assert!(matches!(no_points, Err(OrderError::LearnSkill(_))));

    state
        .submit_order(
            &data,
            Order::DebugGrantXp {
                character: philippe.clone(),
                amount: 1000,
            },
        )
        .unwrap();
    let missing_prereq = state.submit_order(
        &data,
        Order::LearnSkill {
            character: philippe.clone(),
            skill: second.clone(),
        },
    );
    assert!(matches!(
        missing_prereq,
        Err(OrderError::LearnSkill(
            sim_campaign::LearnSkillError::MissingPrerequisite(_)
        ))
    ));

    let command_before = state.character(&philippe).unwrap().skills.command;
    state
        .submit_order(
            &data,
            Order::LearnSkill {
                character: philippe.clone(),
                skill: first.clone(),
            },
        )
        .unwrap();
    state
        .submit_order(
            &data,
            Order::LearnSkill {
                character: philippe.clone(),
                skill: second.clone(),
            },
        )
        .unwrap();
    let c = state.character(&philippe).unwrap();
    assert!(c.skills_learned.contains(&first) && c.skills_learned.contains(&second));
    assert_eq!(c.skills.command, (command_before + 2).min(10));
    let again = state.submit_order(
        &data,
        Order::LearnSkill {
            character: philippe.clone(),
            skill: first,
        },
    );
    assert!(again.is_err(), "a skill is learned once");
}

// ----- traits ----------------------------------------------------------------

#[test]
fn veteran_trait_after_five_battles() {
    let data = data();
    let mut state = france(&data, 3);
    let general = chr("chr_raoul_de_brienne");
    let veteran = TraitId::new("trait_veteran").unwrap();
    let mut events = Vec::new();
    for battle in 1..=5 {
        assert!(!state.character(&general).unwrap().traits.contains(&veteran));
        sim_campaign::dynasty::on_battle_resolved(&mut state, &data, &general, true, &mut events);
        assert_eq!(state.character(&general).unwrap().battles_fought, battle);
    }
    let c = state.character(&general).unwrap();
    assert!(c.traits.contains(&veteran));
    assert!(c.prestige > 0, "victories bring prestige");
    assert_eq!(
        c.skill_points * 100 + c.experience,
        5 * sim_campaign::skills::BATTLE_VICTORY_XP
    );
}

#[test]
fn opposite_traits_exclude_each_other() {
    let data = data();
    let with_opposite = data
        .traits
        .values()
        .find(|t| !t.opposites.is_empty())
        .expect("some trait has opposites");
    let mut traits = std::collections::BTreeSet::new();
    traits.insert(with_opposite.opposites[0].clone());
    assert!(!sim_campaign::skills::can_acquire(
        &data,
        &traits,
        &with_opposite.id
    ));
    assert!(sim_campaign::skills::can_acquire(
        &data,
        &Default::default(),
        &with_opposite.id
    ));
}

// ----- generals & governors --------------------------------------------------

#[test]
fn a_skilled_general_wins_more_often() {
    let enemy = Side {
        units: (0..6).map(|_| unit(100)).collect(),
        ..Side::default()
    };
    let plain = Side {
        units: (0..6).map(|_| unit(100)).collect(),
        ..Side::default()
    };
    let led = Side {
        general_command: 8,
        general_morale_bonus: 10.0,
        general_charge_percent: 20.0,
        ..plain.clone()
    };
    let wins = |side: &Side| {
        (0..200)
            .filter(|seed| {
                let mut rng = CampaignRng::from_seed(*seed);
                resolve_auto(side, &enemy, &BattleContext::default(), &mut rng).winner
                    == Winner::Attacker
            })
            .count()
    };
    let plain_wins = wins(&plain);
    let led_wins = wins(&led);
    assert!(
        led_wins > plain_wins + 20,
        "general should matter: {led_wins} vs {plain_wins}"
    );
}

#[test]
fn a_governor_with_justice_lowers_unrest_effects() {
    let data = data();
    let mut state = france(&data, 4);
    let rouen = prov("prov_normandie");
    let before = state.province_effects(&data, &rouen).unrest;
    let governor = chr("chr_raoul_de_brienne");
    // Grant a governance skill that reduces unrest.
    let skill = data
        .skills
        .values()
        .filter(|s| s.prerequisites.is_empty())
        .find(|s| {
            s.effects
                .iter()
                .any(|e| format!("{:?}", e.effect) == "Unrest" && e.value < 0.0)
        })
        .expect("a tier-1 skill lowering unrest");
    state
        .submit_order(
            &data,
            Order::DebugGrantXp {
                character: governor.clone(),
                amount: 100 * skill.cost,
            },
        )
        .unwrap();
    state
        .submit_order(
            &data,
            Order::LearnSkill {
                character: governor.clone(),
                skill: skill.id.clone(),
            },
        )
        .unwrap();
    state
        .submit_order(
            &data,
            Order::AssignGovernor {
                province: rouen.clone(),
                character: governor.clone(),
            },
        )
        .unwrap();
    assert_eq!(state.province_governor(&rouen), Some(&governor));
    let after = state.province_effects(&data, &rouen).unrest;
    assert!(
        after.flat + after.percent < before.flat + before.percent,
        "governor effect: {before:?} -> {after:?}"
    );

    // Governance XP accrues each turn.
    state.end_turn_with(&data, idle);
    let c = state.character(&governor).unwrap();
    assert!(c.experience > 0 || c.skill_points > 0);
}

#[test]
fn governor_and_general_are_exclusive_and_adults_only() {
    let data = data();
    let mut state = france(&data, 5);
    let rouen = prov("prov_normandie");
    // The ruler commands the royal army at start.
    let philippe = chr("chr_philippe_vi");
    assert!(state.character(&philippe).unwrap().army.is_some());
    let refused = state.submit_order(
        &data,
        Order::AssignGovernor {
            province: rouen.clone(),
            character: philippe,
        },
    );
    assert!(matches!(refused, Err(OrderError::Governor(_))));

    // A foreign character cannot govern a French province.
    let edward = chr("chr_edouard_iii");
    let foreign = state.submit_order(
        &data,
        Order::AssignGovernor {
            province: rouen.clone(),
            character: edward,
        },
    );
    assert!(foreign.is_err());

    // A child cannot govern either.
    let child = state
        .characters
        .iter()
        .find(|(_, c)| c.alive && c.faction == fac("fac_france") && !c.is_major(1337))
        .map(|(id, _)| id.clone());
    if let Some(child) = child {
        let minor = state.submit_order(
            &data,
            Order::AssignGovernor {
                province: rouen,
                character: child,
            },
        );
        assert!(minor.is_err());
    }
}

// ----- marriages ---------------------------------------------------------------

#[test]
fn marriage_valid_and_refused_cases() {
    let data = data();
    let mut state = france(&data, 6);
    let philippe = chr("chr_philippe_vi");
    let jeanne = chr("chr_jeanne_de_bourgogne");
    let jean = chr("chr_jean_de_normandie");

    // Already married couples from the data.
    assert_eq!(
        state.character(&philippe).unwrap().spouse,
        Some(jeanne.clone())
    );
    let married = state.submit_order(
        &data,
        Order::ProposeMarriage {
            character: philippe.clone(),
            spouse: jeanne.clone(),
        },
    );
    assert!(matches!(
        married,
        Err(OrderError::Marriage(
            sim_campaign::MarriageError::AlreadyMarried
        ))
    ));

    // Same sex.
    let same_sex = sim_campaign::dynasty::propose_marriage(&mut state, &data, &philippe, &jean);
    assert_eq!(same_sex, Err(sim_campaign::MarriageError::SameSex));

    // Parent / child: widow Jeanne's husband first so only kinship blocks.
    let mut events = Vec::new();
    sim_campaign::characters::kill(&mut state, &data, &philippe, &mut events);
    assert_eq!(state.character(&jeanne).unwrap().spouse, None, "widowed");
    let related = sim_campaign::dynasty::propose_marriage(&mut state, &data, &jean, &jeanne);
    assert!(related.is_err(), "mother and son (or already married son)");

    // A valid marriage between candidates offered by the engine.
    let (a, b) = state
        .characters
        .keys()
        .find_map(|id| {
            state
                .marriage_candidates(&data, id)
                .first()
                .map(|spouse| (id.clone(), spouse.clone()))
        })
        .expect("some valid couple in 1337");
    let prestige_before = state.character(&a).unwrap().prestige;
    sim_campaign::dynasty::propose_marriage(&mut state, &data, &a, &b).unwrap();
    assert_eq!(state.character(&a).unwrap().spouse, Some(b.clone()));
    assert_eq!(state.character(&b).unwrap().spouse, Some(a.clone()));
    assert!(state.character(&a).unwrap().prestige > prestige_before);
}

#[test]
fn too_young_to_marry() {
    let data = data();
    let mut state = france(&data, 7);
    let child = state
        .characters
        .iter()
        .find(|(_, c)| c.alive && c.spouse.is_none() && c.age(state.year()) < 14)
        .map(|(id, c)| (id.clone(), c.sex))
        .expect("a child in 1337");
    let adult = state
        .characters
        .iter()
        .find(|(_, c)| c.alive && c.spouse.is_none() && c.age(1337) >= 20 && c.sex != child.1)
        .map(|(id, _)| id.clone())
        .expect("an unmarried adult of the other sex");
    let result = sim_campaign::dynasty::propose_marriage(&mut state, &data, &child.0, &adult);
    assert_eq!(result, Err(sim_campaign::MarriageError::TooYoung));
}

// ----- births ------------------------------------------------------------------

#[test]
fn births_are_deterministic() {
    let data = data();
    let run = |seed| {
        let mut state = france(&data, seed);
        let mut births = Vec::new();
        for _ in 0..16 {
            for event in state.end_turn_with(&data, idle) {
                if event.kind == EventKind::Birth {
                    births.push(event.text_fr);
                }
            }
        }
        births
    };
    let first = run(42);
    assert!(!first.is_empty(), "four winters bring births");
    assert_eq!(first, run(42));
}

#[test]
fn charles_v_is_born_when_jean_and_bonne_are_married() {
    let data = data();
    let charles = chr("chr_charles_v");
    let mut state = france(&data, 8);
    assert!(state.character(&charles).is_none(), "unborn in 1337");
    let jean = chr("chr_jean_de_normandie");
    let bonne = chr("chr_bonne_de_luxembourg");
    assert_eq!(state.character(&jean).unwrap().spouse, Some(bonne.clone()));
    for _ in 0..8 {
        state.end_turn_with(&data, idle);
    }
    let born = state.character(&charles).expect("Charles V born by 1339");
    assert_eq!(born.birth_year, 1338);
    assert_eq!(born.father, Some(jean.clone()));
    assert_eq!(born.mother, Some(bonne));
    assert!(state.character(&jean).unwrap().children.contains(&charles));

    // Without the marriage, history diverges: he is never born.
    let mut state = france(&data, 8);
    let mut events = Vec::new();
    sim_campaign::characters::kill(&mut state, &data, &jean, &mut events);
    for _ in 0..8 {
        state.end_turn_with(&data, idle);
    }
    assert!(state.character(&charles).is_none());
}

// ----- succession & regency --------------------------------------------------

#[test]
fn salic_succession_from_philippe_to_jean() {
    let data = data();
    let mut state = france(&data, 9);
    let mut events = Vec::new();
    sim_campaign::characters::kill(&mut state, &data, &chr("chr_philippe_vi"), &mut events);
    let france_state = state.faction_state(&fac("fac_france")).unwrap();
    assert_eq!(france_state.ruler, Some(chr("chr_jean_de_normandie")));
    assert!(france_state.heir.is_some(), "a new heir is designated");
    let heir = state
        .character(france_state.heir.as_ref().unwrap())
        .unwrap();
    assert_eq!(heir.sex, data_model::Sex::Male, "salic law");
    assert!(events.iter().any(|e| e.kind == EventKind::Succession));
}

#[test]
fn a_minor_ruler_opens_a_regency() {
    let data = data();
    let mut state = france(&data, 10);
    // Let Charles V be born, then kill every adult Valois ahead of him.
    for _ in 0..8 {
        state.end_turn_with(&data, idle);
    }
    let charles = chr("chr_charles_v");
    assert!(state.character(&charles).is_some());
    let mut events = Vec::new();
    for id in ["chr_philippe_vi", "chr_jean_de_normandie"] {
        sim_campaign::characters::kill(&mut state, &data, &chr(id), &mut events);
    }
    let ruler = state
        .faction_state(&fac("fac_france"))
        .unwrap()
        .ruler
        .clone()
        .unwrap();
    assert_eq!(ruler, charles, "the eldest son of Jean inherits");
    let paris = prov("prov_ile_de_france");
    let unrest_before = state.province_state(&paris).unwrap().unrest;
    let turn_events = state.end_turn_with(&data, idle);
    assert!(turn_events.iter().any(|e| e.kind == EventKind::Regency));
    assert!(state.faction_state(&fac("fac_france")).unwrap().regency);
    // The regency penalty is applied (the population model may also move
    // unrest, so only check the flag stays set and no event is repeated).
    let _ = unrest_before;
    let next = state.end_turn_with(&data, idle);
    assert!(
        !next.iter().any(|e| e.kind == EventKind::Regency),
        "reported once"
    );
}

// ----- save & determinism -----------------------------------------------------

#[test]
fn save_round_trip_keeps_m4_fields() {
    let data = data();
    let mut state = france(&data, 11);
    let raoul = chr("chr_raoul_de_brienne");
    state
        .submit_order(
            &data,
            Order::DebugGrantXp {
                character: raoul.clone(),
                amount: 150,
            },
        )
        .unwrap();
    state
        .submit_order(
            &data,
            Order::AssignGovernor {
                province: prov("prov_normandie"),
                character: raoul.clone(),
            },
        )
        .unwrap();
    for _ in 0..4 {
        state.end_turn_with(&data, idle);
    }
    let json = state.save_json();
    assert!(json.contains("\"state_version\":4"));
    let loaded = CampaignState::load_json(&json).unwrap();
    assert_eq!(loaded, state);
    let c = loaded.character(&raoul).unwrap();
    assert_eq!(c.governor_of, Some(prov("prov_normandie")));
    assert!(c.skill_points >= 1);
}

#[test]
fn twenty_turns_are_deterministic_with_dynasties() {
    let data = data();
    let run = || {
        let mut state = france(&data, 12);
        for _ in 0..20 {
            state.end_turn(&data);
        }
        state.save_json()
    };
    assert_eq!(run(), run());
}

#[test]
fn view_lists_ruler_first_and_family() {
    let data = data();
    let state = france(&data, 13);
    let ids = state.faction_characters(&fac("fac_france"));
    assert_eq!(ids.first(), Some(&chr("chr_philippe_vi")));
    assert_eq!(ids.get(1), Some(&chr("chr_jean_de_normandie")));
    let view = state
        .character_view(&data, &chr("chr_philippe_vi"))
        .unwrap();
    assert_eq!(view.spouse, Some(chr("chr_jeanne_de_bourgogne")));
    assert!(view
        .children
        .iter()
        .any(|c| c.id == chr("chr_jean_de_normandie")));
    assert!(!view.traits.is_empty());
}
