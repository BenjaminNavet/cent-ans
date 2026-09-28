//! Regression tests for the fixes of the 2026-09-26 code review
//! (`docs/wip/revue-code.md`, section « Corrections sim-campaign »).

use std::path::PathBuf;

use data_model::{FactionId, GameData, ProvinceId, SettlementId};

use crate::battle_auto::{BattleResult, SideOutcome, Winner};
use crate::state::{ArmyId, ArmyPosition, CampaignState, Stance};

fn data() -> GameData {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../data");
    GameData::load(&root).expect("game data loads").0
}

fn fac(id: &str) -> FactionId {
    FactionId::new(id).unwrap()
}

fn prov(id: &str) -> ProvinceId {
    ProvinceId::new(id).unwrap()
}

fn city(state: &CampaignState, province: &str) -> SettlementId {
    state.province_city_id(&prov(province)).unwrap().clone()
}

/// The first army of `faction` led by a general.
fn led_army(state: &CampaignState, faction: &str) -> ArmyId {
    state
        .armies
        .iter()
        .find(|(_, a)| a.faction == fac(faction) && a.general.is_some())
        .map(|(id, _)| id.clone())
        .expect("a led army")
}

/// `army` besieges `place` (siege stance, siege state begun).
fn besiege(state: &mut CampaignState, data: &GameData, army: &ArmyId, place: &SettlementId) {
    let a = state.armies.get_mut(army).unwrap();
    a.position = ArmyPosition::Settlement(place.clone());
    a.stance = Stance::Siege;
    a.clear_plan();
    let faction = a.faction.clone();
    let mut events = Vec::new();
    crate::siege::begin_siege(state, data, place, &faction, army, &mut events);
}

fn outcome(units: usize, general_captured: bool) -> SideOutcome {
    SideOutcome {
        power: 1.0,
        losses: vec![0; units],
        total_losses: 0,
        morale_delta: 0,
        routed: false,
        general_captured,
        general_killed: false,
    }
}

/// Fix 1: a general taken in a failed assault is held by the defender.
#[test]
fn general_captured_in_assault_has_a_captor() {
    let data = data();
    let mut state = CampaignState::new_1337(&data, fac("fac_france"), 1).unwrap();
    let army = led_army(&state, "fac_france");
    let general = state.armies[&army].general.clone().unwrap();
    let guyenne = city(&state, "prov_guyenne");
    besiege(&mut state, &data, &army, &guyenne);
    let controller = state.settlements[&guyenne].controller.clone();
    let units = state.armies[&army].units.len();
    let garrison = state.settlements[&guyenne].garrison.len();
    let result = BattleResult {
        winner: Winner::Defender,
        attacker: outcome(units, true),
        defender: outcome(garrison, false),
    };
    let mut events = Vec::new();
    crate::siege::apply_assault_result(
        &mut state,
        &data,
        std::slice::from_ref(&army),
        &guyenne,
        &result,
        true,
        &mut events,
    );
    let c = &state.characters[&general];
    assert!(c.captive);
    assert_eq!(c.captor.as_ref(), Some(&controller));
}

/// Fix 2: a besieged garrison cannot form an army (it would lift the siege).
#[test]
fn besieged_garrison_cannot_create_an_army() {
    let data = data();
    let mut state = CampaignState::new_1337(&data, fac("fac_france"), 1).unwrap();
    let army = led_army(&state, "fac_france");
    let guyenne = city(&state, "prov_guyenne");
    besiege(&mut state, &data, &army, &guyenne);
    let controller = state.settlements[&guyenne].controller.clone();
    assert!(!state.settlements[&guyenne].garrison.is_empty());
    let order = crate::orders::Order::CreateArmy {
        settlement: crate::orders::Place::Settlement(guyenne.clone()),
        units_from_garrison: vec![0],
        general: None,
    };
    assert_eq!(
        state.apply_order(&data, &controller, order),
        Err(crate::orders::OrderError::SettlementBesieged)
    );
}

/// Makes `a` and `b` allies at war with `enemy`.
fn ally_against(state: &mut CampaignState, a: &str, b: &str, enemy: &str) {
    for (x, y) in [(a, b), (b, a)] {
        let f = state.factions.get_mut(&fac(x)).unwrap();
        f.allies.insert(fac(y));
        f.at_war_with.remove(&fac(y));
    }
    for x in [a, b] {
        state
            .factions
            .get_mut(&fac(x))
            .unwrap()
            .at_war_with
            .insert(fac(enemy));
        state
            .factions
            .get_mut(&fac(enemy))
            .unwrap()
            .at_war_with
            .insert(fac(x));
    }
}

/// Fix 3: an ally joining (or taking over) a siege keeps its progress.
#[test]
fn allied_besieger_keeps_the_siege_progress() {
    let data = data();
    let mut state = CampaignState::new_1337(&data, fac("fac_france"), 1).unwrap();
    ally_against(&mut state, "fac_france", "fac_scotland", "fac_england");
    let french = led_army(&state, "fac_france");
    let scots = state
        .armies
        .iter()
        .find(|(_, a)| a.faction == fac("fac_scotland"))
        .map(|(id, _)| id.clone())
        .unwrap();
    let guyenne = city(&state, "prov_guyenne");
    besiege(&mut state, &data, &french, &guyenne);
    {
        let siege = state.settlements.get_mut(&guyenne).unwrap().siege.as_mut();
        let siege = siege.unwrap();
        siege.breach = 40;
        siege.supplies = 50;
        siege.started_turn = 0;
    }
    besiege(&mut state, &data, &scots, &guyenne);
    let siege = state.settlements[&guyenne].siege.clone().unwrap();
    assert_eq!(siege.attacker, fac("fac_france"));
    assert_eq!(siege.breach, 40);
    // The French leave: the Scots carry on the same siege.
    let kent = city(&state, "prov_kent");
    state.armies.get_mut(&french).unwrap().position = ArmyPosition::Settlement(kent);
    let mut events = Vec::new();
    crate::siege::resolve_sieges(&mut state, &data, &mut events);
    if let Some(siege) = state.settlements[&guyenne].siege.clone() {
        assert_eq!(siege.attacker, fac("fac_scotland"));
        assert!(siege.breach >= 40 && siege.supplies <= 50);
    } else {
        assert_eq!(state.settlements[&guyenne].controller, fac("fac_scotland"));
    }
}

/// Fix 5: a captured settlement loses its building site (the occupant
/// cannot cancel it for the refund).
#[test]
fn capture_drops_the_construction() {
    let data = data();
    let mut state = CampaignState::new_1337(&data, fac("fac_france"), 1).unwrap();
    let guyenne = city(&state, "prov_guyenne");
    let building = data.buildings.keys().next().unwrap().clone();
    state.settlements.get_mut(&guyenne).unwrap().construction = Some(crate::state::Construction {
        building,
        turns_left: 3,
        paid: 100,
        drawn: Default::default(),
    });
    let mut events = Vec::new();
    crate::siege::capture(&mut state, &data, &guyenne, &fac("fac_france"), &mut events);
    let s = &state.settlements[&guyenne];
    assert_eq!(s.controller, fac("fac_france"));
    assert!(s.construction.is_none());
    assert!(s.recruit_queue.is_empty());
}

/// Fix 7: a player who still holds a castle or a town is not defeated.
#[test]
fn holding_a_lesser_settlement_is_no_defeat() {
    let data = data();
    let mut state = CampaignState::new_1337(&data, fac("fac_france"), 4).unwrap();
    let cities: Vec<SettlementId> = state
        .provinces
        .keys()
        .filter_map(|p| state.province_city_id(p).cloned())
        .collect();
    let kept = state
        .settlements
        .iter()
        .find(|(id, s)| s.controller == fac("fac_france") && !cities.contains(id))
        .map(|(id, _)| id.clone())
        .expect("France holds a lesser settlement");
    for (id, s) in state.settlements.iter_mut() {
        if s.controller == fac("fac_france") && *id != kept {
            s.controller = fac("fac_england");
        }
    }
    let mut events = Vec::new();
    crate::victory::resolve_victory(&mut state, &data, &mut events);
    assert!(state.outcome.is_none());
    state.settlements.get_mut(&kept).unwrap().controller = fac("fac_england");
    crate::victory::resolve_victory(&mut state, &data, &mut events);
    assert!(state.outcome.is_some());
}

/// Aragon hands a courtier as a hostage to Castile (treaty article).
fn aragonese_hostage(state: &mut CampaignState, data: &GameData) -> data_model::CharacterId {
    let (ar, ca) = (fac("fac_aragon"), fac("fac_castile"));
    let ruler = state.factions[&ar].ruler.clone();
    let hostage = state
        .characters
        .iter()
        .find(|(id, c)| {
            c.alive
                && c.faction == ar
                && !c.captive
                && c.army.is_none()
                && Some(*id) != ruler.as_ref()
        })
        .map(|(id, _)| id.clone())
        .expect("an Aragonese courtier");
    let article = crate::negotiation::Article::Hostage {
        giver: crate::negotiation::Party::Proposer,
        character: hostage.clone(),
    };
    crate::negotiation::apply_treaty(state, data, &ar, &ca, &[article]).unwrap();
    hostage
}

/// Fix 8: a treaty hostage can be neither bought back nor freed on parole
/// before his term (ADR 0025 § 6).
#[test]
fn treaty_hostage_is_held_for_his_term() {
    use crate::ransom::{self, RansomError, RansomTerms};
    let data = data();
    let mut state = CampaignState::new_1337(&data, fac("fac_aragon"), 10).unwrap();
    let hostage = aragonese_hostage(&mut state, &data);
    let (ar, ca) = (fac("fac_aragon"), fac("fac_castile"));
    assert_eq!(
        state.characters[&hostage].ransom_terms,
        Some(RansomTerms::Hold)
    );
    state.factions.get_mut(&ar).unwrap().treasury = 1_000_000;
    assert_eq!(
        ransom::pay_ransom(&mut state, &data, &ar, &hostage, 1),
        Err(RansomError::Held)
    );
    assert_eq!(
        ransom::release_on_parole(&mut state, &data, &ca, &hostage),
        Err(RansomError::Held)
    );
    assert_eq!(
        ransom::set_ransom_terms(&mut state, &data, &ca, &hostage, RansomTerms::Money),
        Err(RansomError::Held)
    );
    assert!(state.characters[&hostage].captive);
}

/// Fix 11: the holder attacking the giver of its hostages does not make the
/// giver a perjurer; the giver attacking the holder does.
#[test]
fn hostage_betrayal_falls_on_the_aggressor_only() {
    let data = data();
    let (ar, ca) = (fac("fac_aragon"), fac("fac_castile"));
    let betrayed = |state: &CampaignState| {
        state.factions[&ca]
            .modifiers
            .iter()
            .any(|m| m.with == ar && m.reason_fr == crate::negotiation::HOSTAGE_BETRAYAL_REASON)
    };
    let idle = |_: &CampaignState, _: &GameData, _: &FactionId| Vec::new();
    // The holder attacks.
    let mut state = CampaignState::new_1337(&data, fac("fac_aragon"), 10).unwrap();
    aragonese_hostage(&mut state, &data);
    state.declare_war(&data, &ca, &ar).unwrap();
    state.end_turn_with(&data, idle);
    assert!(!betrayed(&state));
    // The giver attacks.
    let mut state = CampaignState::new_1337(&data, fac("fac_aragon"), 10).unwrap();
    aragonese_hostage(&mut state, &data);
    state.declare_war(&data, &ar, &ca).unwrap();
    assert!(betrayed(&state));
}

/// Fix 12: a vanished faction pays no more tribute.
#[test]
fn dead_faction_pays_no_tribute() {
    let data = data();
    let mut state = CampaignState::new_1337(&data, fac("fac_france"), 1).unwrap();
    let (payer, payee) = (fac("fac_navarre"), fac("fac_castile"));
    let turn = state.turn;
    {
        let f = state.factions.get_mut(&payer).unwrap();
        f.alive = false;
        f.ledger.tributes.push(crate::negotiation::TributeDue {
            to: payee.clone(),
            per_season: 100,
            until_turn: turn + 10,
        });
    }
    let before = (
        state.factions[&payer].treasury,
        state.factions[&payee].treasury,
    );
    let mut events = Vec::new();
    crate::negotiation::resolve_negotiation(&mut state, &data, &mut events);
    assert_eq!(state.factions[&payer].treasury, before.0);
    assert_eq!(state.factions[&payee].treasury, before.1);
    assert!(state.factions[&payer].ledger.tributes.is_empty());
}

/// Fix 9: the child of a cross-faction marriage belongs to its father's
/// faction when the mother holds no crown there.
#[test]
fn cross_faction_child_follows_the_father() {
    use data_model::Sex;
    let data = data();
    let mut state = CampaignState::new_1337(&data, fac("fac_france"), 3).unwrap();
    let crowned = |state: &CampaignState, id: &data_model::CharacterId| {
        state
            .factions
            .values()
            .any(|f| f.ruler.as_ref() == Some(id) || f.heir.as_ref() == Some(id))
    };
    let pick = |state: &CampaignState, faction: &str, sex: Sex| {
        state
            .characters
            .iter()
            .find(|(id, c)| {
                c.alive && c.faction == fac(faction) && c.sex == sex && !crowned(state, id)
            })
            .map(|(id, _)| id.clone())
            .expect("a courtier")
    };
    let father = pick(&state, "fac_england", Sex::Male);
    let mother = pick(&state, "fac_france", Sex::Female);
    // Only this couple is married and fertile.
    for c in state.characters.values_mut() {
        c.spouse = None;
    }
    let year = state.year;
    let m = state.characters.get_mut(&mother).unwrap();
    m.spouse = Some(father.clone());
    m.birth_year = year - 25;
    let f = state.characters.get_mut(&father).unwrap();
    f.spouse = Some(mother.clone());
    f.birth_year = year - 30;
    state.season = crate::state::Season::Winter;
    let before: Vec<data_model::CharacterId> = state.characters.keys().cloned().collect();
    let mut events = Vec::new();
    for _ in 0..60 {
        crate::dynasty::resolve_births(&mut state, &data, &mut events);
        if let Some(child) = state
            .characters
            .iter()
            .find(|(id, c)| c.mother.as_ref() == Some(&mother) && !before.contains(id))
            .map(|(_, c)| c)
        {
            assert_eq!(child.faction, fac("fac_england"));
            return;
        }
    }
    panic!("no child born");
}

/// Fix 10: a treaty whose marriage is impossible is refused as a whole
/// (the peace is not applied by halves).
#[test]
fn impossible_marriage_refuses_the_whole_treaty() {
    use crate::negotiation::Article;
    use data_model::Sex;
    let data = data();
    let mut state = CampaignState::new_1337(&data, fac("fac_france"), 1).unwrap();
    let (fr, en) = (fac("fac_france"), fac("fac_england"));
    assert!(state.is_at_war(&fr, &en));
    let man = |state: &CampaignState, f: &FactionId| {
        state
            .characters
            .iter()
            .find(|(_, c)| c.alive && &c.faction == f && c.sex == Sex::Male)
            .map(|(id, _)| id.clone())
            .unwrap()
    };
    let (a, b) = (man(&state, &fr), man(&state, &en));
    let articles = [
        Article::Peace,
        Article::Marriage {
            character: a,
            spouse: b,
        },
    ];
    assert!(crate::negotiation::apply_treaty(&mut state, &data, &fr, &en, &articles).is_err());
    assert!(state.is_at_war(&fr, &en));
}

/// Fix 16: a port bordering two enemy-held seas is blockaded once.
#[test]
fn a_port_on_two_enemy_seas_is_blockaded_once() {
    use crate::naval::SeaControl;
    let data = data();
    let mut state = CampaignState::new_1337(&data, fac("fac_france"), 1).unwrap();
    state.naval.ensure(&data);
    let (province, seas) = data
        .provinces
        .iter()
        .find(|(id, p)| {
            p.has_port() && p.sea_zones.len() >= 2 && state.province_owner(id).is_some()
        })
        .map(|(id, p)| (id.clone(), p.sea_zones.clone()))
        .expect("a port on two seas");
    let owner = state.province_owner(&province).unwrap().clone();
    let enemy = state
        .factions
        .keys()
        .find(|f| **f != owner && !state.is_allied(f, &owner))
        .unwrap()
        .clone();
    state
        .factions
        .get_mut(&owner)
        .unwrap()
        .at_war_with
        .insert(enemy.clone());
    state
        .factions
        .get_mut(&enemy)
        .unwrap()
        .at_war_with
        .insert(owner.clone());
    state.naval.control.clear();
    for sea in &seas[..2] {
        state.naval.control.insert(
            sea.clone(),
            SeaControl {
                faction: enemy.clone(),
                level: 100,
            },
        );
    }
    let expected = data
        .provinces
        .iter()
        .filter(|(id, p)| {
            p.has_port()
                && p.sea_zones.iter().any(|s| seas[..2].contains(s))
                && state.province_owner(id) == Some(&owner)
        })
        .count();
    let mut events = Vec::new();
    crate::naval::resolve_season(&mut state, &data, &mut events);
    let text = events
        .iter()
        .find(|e| e.text_fr.starts_with("Blocus") && e.faction.as_ref() == Some(&owner))
        .map(|e| e.text_fr.clone())
        .expect("a blockade");
    assert!(
        text.starts_with(&format!("Blocus : {expected} port(s)")),
        "{text}"
    );
}

/// Fix 17: research counts the libraries of the places a faction holds
/// (controller), not those it merely owns de jure.
#[test]
fn research_follows_the_controller() {
    let data = data();
    let mut state = CampaignState::new_1337(&data, fac("fac_france"), 1).unwrap();
    let (fr, en) = (fac("fac_france"), fac("fac_england"));
    let library = data
        .buildings
        .values()
        .find(|b| {
            b.effects.iter().any(|e| {
                e.effect == data_model::EffectKind::ResearchPoints
                    && e.mode == data_model::EffectMode::Add
                    && e.value >= 1.0
            })
        })
        .map(|b| b.id.clone())
        .expect("a research building");
    let place = city(&state, "prov_guyenne");
    let base_fr = state.research_points_per_turn(&data, &fr);
    let base_en = state.research_points_per_turn(&data, &en);
    let s = state.settlements.get_mut(&place).unwrap();
    s.owner = en.clone();
    s.controller = fr.clone();
    s.buildings.push(library);
    assert!(state.research_points_per_turn(&data, &fr) > base_fr);
    assert!(state.research_points_per_turn(&data, &en) <= base_en);
}

/// Reference for fix 18c: the per-resolution search `trade.rs` ran before
/// the paths were precomputed (Dijkstra over `movement::edges`), verbatim.
fn reference_shortest_path(
    data: &GameData,
    from: &SettlementId,
    to: &SettlementId,
) -> Option<(u32, Vec<SettlementId>)> {
    use std::cmp::Reverse;
    use std::collections::{BTreeMap, BinaryHeap};
    if from == to {
        return Some((0, vec![from.clone()]));
    }
    let mut dist: BTreeMap<SettlementId, u32> = BTreeMap::new();
    let mut prev: BTreeMap<SettlementId, SettlementId> = BTreeMap::new();
    let mut heap = BinaryHeap::new();
    dist.insert(from.clone(), 0);
    heap.push(Reverse((0u32, from.clone())));
    while let Some(Reverse((cost, current))) = heap.pop() {
        if &current == to {
            break;
        }
        if cost > *dist.get(&current).unwrap_or(&u32::MAX) {
            continue;
        }
        for (next, edge_cost) in crate::movement::edges(data, &current) {
            let next_cost = cost + edge_cost;
            if next_cost < dist.get(&next).copied().unwrap_or(u32::MAX) {
                dist.insert(next.clone(), next_cost);
                prev.insert(next.clone(), current.clone());
                heap.push(Reverse((next_cost, next)));
            }
        }
    }
    let cost = *dist.get(to)?;
    let mut path = vec![to.clone()];
    let mut current = to.clone();
    while &current != from {
        let previous = prev.get(&current)?;
        path.push(previous.clone());
        current = previous.clone();
    }
    path.reverse();
    Some((cost, path))
}

/// Fix 18c: the trade paths precomputed at load are exactly those the old
/// per-season search found, for every route of the catalogue.
#[test]
fn precomputed_trade_paths_match_the_old_search() {
    let data = data();
    let catalog = data.trade.as_ref().expect("trade catalogue");
    assert!(!catalog.routes.is_empty());
    let mut found = 0;
    for route in &catalog.routes {
        let from = &catalog.hub(&route.from_hub).unwrap().settlement;
        let to = &catalog.hub(&route.to_hub).unwrap().settlement;
        let key = (from.clone(), to.clone());
        let cached = data
            .trade_paths
            .paths
            .get(&key)
            .unwrap_or_else(|| panic!("{} is precomputed", route.id));
        let reference = reference_shortest_path(&data, from, to);
        assert_eq!(cached, &reference, "{}", route.id);
        assert_eq!(data.trade_path(from, to), reference, "{}", route.id);
        found += usize::from(reference.is_some());
    }
    assert!(found > 0, "some route has a path");

    // The whole view is unchanged, cache or not.
    let state = CampaignState::new_1337(&data, fac("fac_france"), 1).unwrap();
    let mut uncached = data.clone();
    uncached.trade_paths = Default::default();
    assert_eq!(
        crate::trade::trade_routes(&state, &data),
        crate::trade::trade_routes(&state, &uncached)
    );
}

/// Fix 18c, timing: `cargo test -p sim-campaign --release --lib --
/// --ignored trade_paths_timing --nocapture`.
#[test]
#[ignore = "timing probe"]
fn trade_paths_timing() {
    const RUNS: u32 = 200;
    let data = data();
    let state = CampaignState::new_1337(&data, fac("fac_france"), 1).unwrap();
    let mut uncached = data.clone();
    uncached.trade_paths = Default::default();
    let time = |d: &GameData| {
        let start = std::time::Instant::now();
        for _ in 0..RUNS {
            std::hint::black_box(crate::trade::trade_routes(&state, d));
        }
        start.elapsed() / RUNS
    };
    let before = time(&uncached);
    let after = time(&data);
    let mut rebuilt = data.clone();
    let start = std::time::Instant::now();
    rebuilt.build_trade_paths();
    let build = start.elapsed();
    println!(
        "trade_routes: {before:?} per call before, {after:?} after; \
         build_trade_paths once: {build:?}"
    );
}
