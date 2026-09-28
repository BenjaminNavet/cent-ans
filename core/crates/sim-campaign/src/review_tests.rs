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

/// Fix 4: an army emptied by the stragglers of an orderly fallback is
/// dispersed (as in a rout), not left on the map without a regiment.
#[test]
fn an_army_emptied_by_its_fallback_is_dispersed() {
    let mut data = data();
    // As in c7a_retreat: no English place in reach of Saint-Denis, but a
    // refuge (a place no enemy holds) within the neutral radius.
    let retreat = &mut data.settlement_rules.as_mut().unwrap().retreat;
    retreat.friendly_radius_steps = 0.5;
    retreat.neutral_radius_steps = 2.0;
    assert!(data.retreat_rules().neutral_loss_percent > 0);
    let mut state = CampaignState::new_1337(&data, fac("fac_france"), 3).unwrap();
    state.interactive_battles = false;
    let english = led_army(&state, "fac_england");
    let french = led_army(&state, "fac_france");
    let saint_denis = SettlementId::new("set_saint_denis").unwrap();
    let paris = SettlementId::new("set_paris").unwrap();
    for (army, place) in [(&english, &saint_denis), (&french, &paris)] {
        let a = state.armies.get_mut(army).unwrap();
        a.position = ArmyPosition::Settlement(place.clone());
        a.clear_plan();
    }
    let paris_point = data.settlement_point(&paris).unwrap();
    assert!(matches!(
        crate::movement::retreat_target_after(&state, &data, &english, paris_point, 0),
        Some(crate::movement::Retreat::Fallback(_))
    ));
    // A single regiment at exactly 5 % of its men: it survives the battle
    // (no loss), but the fallback's stragglers disband it.
    let general = state.armies[&english].general.clone();
    {
        let a = state.armies.get_mut(&english).unwrap();
        a.units.truncate(1);
        a.units[0].max_strength = 100;
        a.units[0].strength = 5;
    }
    let units = state.armies[&french].units.len();
    let result = BattleResult {
        winner: Winner::Defender,
        attacker: outcome(1, false),
        defender: outcome(units, false),
    };
    let mut events = Vec::new();
    crate::movement::apply_battle_result(
        &mut state,
        &data,
        std::slice::from_ref(&english),
        std::slice::from_ref(&french),
        &result,
        &mut events,
    );
    assert!(!state.armies.contains_key(&english), "dispersed");
    assert!(events
        .iter()
        .any(|e| e.kind == crate::events::EventKind::ArmyDestroyed));
    if let Some(general) = general {
        let c = &state.characters[&general];
        assert!(c.alive && !c.captive, "the general escapes");
    }
}

/// England and France at war only with each other, `score` for France.
fn anglo_french_war(data: &GameData, player: &str, score: i32) -> CampaignState {
    let mut state = CampaignState::new_1337(data, fac(player), 5).unwrap();
    state.chronicle.disabled = true;
    let (fr, en) = (fac("fac_france"), fac("fac_england"));
    state.turn = 40;
    let ids: Vec<FactionId> = state.factions.keys().cloned().collect();
    for id in &ids {
        state.factions.get_mut(id).unwrap().at_war_with.clear();
    }
    for (a, b) in [(&fr, &en), (&en, &fr)] {
        let f = state.factions.get_mut(a).unwrap();
        f.at_war_with.insert(b.clone());
        f.truces.remove(b);
        f.war_started.insert(b.clone(), 20);
        f.war_scores.insert(b.clone(), 0);
    }
    set_war_score(&mut state, data, score);
    state
}

/// Sets France's war score against England to `score`.
fn set_war_score(state: &mut CampaignState, data: &GameData, score: i32) {
    let (fr, en) = (fac("fac_france"), fac("fac_england"));
    state
        .factions
        .get_mut(&fr)
        .unwrap()
        .war_scores
        .insert(en.clone(), 0);
    let base = state.war_score(data, &fr, &en);
    state
        .factions
        .get_mut(&fr)
        .unwrap()
        .war_scores
        .insert(en, score - base);
}

fn ceded(order: Option<crate::Order>) -> Vec<ProvinceId> {
    let Some(crate::Order::ProposeTreaty { articles, .. }) = order else {
        panic!("a treaty, got {order:?}");
    };
    articles
        .into_iter()
        .filter_map(|a| match a {
            crate::negotiation::Article::CedeProvince { province, .. } => Some(province),
            _ => None,
        })
        .collect()
}

/// Fix 13: a winner demands its war goals, even unheld, before the other
/// provinces it holds (ADR 0025 § 5).
#[test]
fn the_winner_demands_its_unheld_war_goals_first() {
    let data = data();
    let (fr, en) = (fac("fac_france"), fac("fac_england"));
    // England is the player: its acceptance is not weighed, only the order.
    let mut state = anglo_french_war(&data, "fac_england", 0);
    let capital = state.factions[&en].capital.clone();
    let english: Vec<ProvinceId> = state
        .provinces
        .keys()
        .filter(|p| state.province_owner(p) == Some(&en) && *p != &capital)
        .cloned()
        .collect();
    assert!(english.len() >= 2, "{english:?}");
    // France holds the first English province (by id); its war goal is the
    // last one, which it does not hold.
    let (held, goal) = (english[0].clone(), english[english.len() - 1].clone());
    for s in state.settlements.values_mut() {
        if s.province == held {
            s.controller = fr.clone();
        }
    }
    assert!(state.controls_province(&fr, &held));
    assert!(!state.controls_province(&fr, &goal));
    state
        .factions
        .get_mut(&fr)
        .unwrap()
        .ledger
        .war_goals
        .insert(en.clone(), vec![goal.clone()]);
    set_war_score(&mut state, &data, 100);
    let provinces = ceded(crate::negotiation::plan_peace(&state, &data, &fr));
    assert_eq!(provinces.first(), Some(&goal), "{provinces:?}");
    assert!(provinces.contains(&held), "{provinces:?}");
}

/// Fix 13: a crown sues for peace from a war score of -50 included
/// (`score <= 2 * SURRENDER_WAR_SCORE`, ADR 0025 § 5 « score ≤ -50 »).
#[test]
fn a_crown_beaten_at_minus_fifty_sues_for_peace() {
    let mut data = data();
    data.ai_diplomacy.peace.cornered_provinces = 0;
    let fr = fac("fac_france");
    let threshold = 2 * crate::diplomacy::SURRENDER_WAR_SCORE;
    assert_eq!(threshold, -50);
    let at = |score: i32| {
        let state = anglo_french_war(&data, "fac_england", score);
        assert_eq!(state.war_score(&data, &fr, &fac("fac_england")), score);
        assert!(!crate::diplomacy::is_cornered(&state, &data, &fr));
        crate::negotiation::plan_peace(&state, &data, &fr)
    };
    assert_eq!(at(threshold + 1), None);
    assert!(at(threshold).is_some());
}

/// Fix 14: an AI herald does not wait at the captor's for a ransom his
/// treasury cannot pay; he moves on to his other errands (here a truce).
#[test]
fn a_poor_herald_does_not_wait_for_a_ransom() {
    use data_model::{AgentActionKind, AgentKind};
    let data = data();
    let mut state = anglo_french_war(&data, "fac_scotland", -60);
    let (fr, en) = (fac("fac_france"), fac("fac_england"));
    let captive = state
        .characters
        .iter()
        .find(|(id, c)| {
            c.faction == fr && c.alive && state.factions[&fr].ruler.as_ref() != Some(*id)
        })
        .map(|(id, _)| id.clone())
        .unwrap();
    crate::chronicle::capture_character(&mut state, &data, &captive, &en, &mut Vec::new());
    assert!(state.characters[&captive].captive);
    // A herald recruited at home, then standing in an English city.
    state.factions.get_mut(&fr).unwrap().treasury = 100_000;
    let herald = state
        .settlements
        .iter()
        .filter(|(_, s)| s.controller == fr)
        .map(|(id, _)| id.clone())
        .collect::<Vec<_>>()
        .into_iter()
        .find_map(|id| {
            state
                .recruit_agent(&data, &fr, &id, AgentKind::Emissary)
                .ok()
        })
        .expect("a place to recruit a herald");
    let english_city = state
        .settlements
        .iter()
        .find(|(_, s)| s.controller == en && s.kind == data_model::SettlementKind::City)
        .map(|(id, _)| id.clone())
        .unwrap();
    {
        let agent = state.agents.agents.get_mut(&herald).unwrap();
        agent.location = english_city;
        agent.movement_points = 500;
    }
    let action_of = |state: &CampaignState| {
        crate::agents::plan_agents(state, &data, &fr)
            .into_iter()
            .find_map(|o| match o {
                crate::Order::AgentAction { agent, action, .. } if agent == herald => Some(action),
                _ => None,
            })
    };
    // Rich: he buys the captive back.
    assert_eq!(action_of(&state), Some(AgentActionKind::Ransom));
    // Penniless: no ransom, a truce with the winning enemy instead.
    state.factions.get_mut(&fr).unwrap().treasury = 0;
    assert_eq!(action_of(&state), Some(AgentActionKind::Truce));
}

/// Fix 15: hired cogs lost in a crossing battle are not taken from the
/// faction's fleet; only its own ships are.
#[test]
fn hired_cogs_lost_at_sea_are_not_the_fleets() {
    use data_model::ShipClassId;
    use sim_battle::naval::{NavalSideResult, NavalSideSetup, ShipFate, ShipResult, ShipSetup};
    let data = data();
    let mut state = CampaignState::new_1337(&data, fac("fac_scotland"), 1).unwrap();
    let en = fac("fac_england");
    let cog_id = ShipClassId::new("ship_cog").unwrap();
    let cog = data.naval.ship("ship_cog").expect("cog class").clone();
    // England owns two cogs; the crossing hires two more.
    state
        .naval
        .fleets
        .insert(en.clone(), [(cog_id.clone(), 2)].into_iter().collect());
    let ship = |n: usize| ShipSetup {
        name: format!("cogue {n}"),
        class: cog.clone(),
        crew: Vec::new(),
        fireship: false,
        chain: None,
        fire_arrows: false,
        position: None,
        heading_deg: None,
        flagship: n == 0,
    };
    let side = NavalSideSetup {
        faction: en.to_string(),
        faction_name: String::new(),
        army: String::new(),
        admiral: String::new(),
        units: Vec::new(),
        ships: (0..4).map(ship).collect(),
        hold: false,
    };
    let fates = [
        ShipFate::Kept,
        ShipFate::Sunk,
        ShipFate::Captured,
        ShipFate::Sunk,
    ];
    let result = NavalSideResult {
        ships: fates
            .iter()
            .enumerate()
            .map(|(index, fate)| ShipResult {
                index,
                name: format!("cogue {index}"),
                class: "ship_cog".to_owned(),
                fate: *fate,
            })
            .collect(),
        ..Default::default()
    };
    // Ships 0-1 are England's own, 2-3 hired: one own cog lost (ship 1).
    assert_eq!(
        crate::naval::own_ships_lost(&state, &en, &side, &result),
        vec![cog_id]
    );
}
