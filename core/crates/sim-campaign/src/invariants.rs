//! Rule invariants: properties every campaign state must keep, whatever the
//! orders. Checked by the seed bots after each step (spec
//! `docs/superpowers/specs/2026-10-13-seed-bots-design.md`) and usable by any
//! test or client.

use std::collections::BTreeMap;

use data_model::{CharacterId, GameData, ProvinceId, SettlementId};

use crate::state::{ArmyId, ArmyPosition, CampaignState, Unit};

/// One broken invariant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Violation {
    /// Stable key of the invariant (`settlement_single_owner`…).
    pub invariant: &'static str,
    /// What is wrong, in French, naming the ids involved.
    pub detail: String,
}

/// Treasury beyond this magnitude is an overflow in the making (i64 is far larger).
const TREASURY_LIMIT: i64 = 1_000_000_000_000;

/// Every broken invariant of `state`; empty when the state is sound.
///
/// Stable keys: `settlement_single_owner`, `settlement_refs`, `army_single_place`,
/// `army_refs`, `unit_strength`, `treasury_finite`, `dead_faction_assets`,
/// `character_refs`, `faction_refs`, `title_refs`.
pub fn check_invariants(state: &CampaignState, data: &GameData) -> Vec<Violation> {
    let mut out = Vec::new();
    check_settlements(state, &mut out);
    check_armies(state, &mut out);
    check_factions(state, data, &mut out);
    check_characters(state, &mut out);
    check_titles(state, data, &mut out);
    out
}

fn push(out: &mut Vec<Violation>, invariant: &'static str, detail: String) {
    out.push(Violation { invariant, detail });
}

/// Each settlement belongs to exactly one province, which lists it; its
/// owner and controller are known factions.
fn check_settlements(state: &CampaignState, out: &mut Vec<Violation>) {
    let mut listed: BTreeMap<&SettlementId, Vec<&ProvinceId>> = BTreeMap::new();
    for (pid, province) in &state.provinces {
        if !province.settlements.contains(&province.city) {
            push(
                out,
                "settlement_refs",
                format!("La province {pid} ne liste pas sa ville {}.", province.city),
            );
        }
        for sid in &province.settlements {
            listed.entry(sid).or_default().push(pid);
            if !state.settlements.contains_key(sid) {
                push(
                    out,
                    "settlement_refs",
                    format!("La province {pid} liste la colonie inconnue {sid}."),
                );
            }
        }
    }
    for (sid, settlement) in &state.settlements {
        match listed.get(sid).map(Vec::as_slice) {
            Some([only]) => {
                if **only != settlement.province {
                    push(
                        out,
                        "settlement_single_owner",
                        format!(
                            "La colonie {sid} déclare la province {} mais {only} la liste.",
                            settlement.province
                        ),
                    );
                }
            }
            Some(many) if many.len() > 1 => push(
                out,
                "settlement_single_owner",
                format!("La colonie {sid} est listée par {} provinces.", many.len()),
            ),
            _ => push(
                out,
                "settlement_single_owner",
                format!("La colonie {sid} n'est listée par aucune province."),
            ),
        }
        for (role, faction) in [
            ("propriétaire", &settlement.owner),
            ("occupant", &settlement.controller),
        ] {
            if !state.factions.contains_key(faction) {
                push(
                    out,
                    "settlement_refs",
                    format!("La colonie {sid} a pour {role} la faction inconnue {faction}."),
                );
            }
        }
        if let Some(siege) = settlement
            .siege
            .as_ref()
            .filter(|siege| !state.factions.contains_key(&siege.attacker))
        {
            push(
                out,
                "settlement_refs",
                format!("Siège de {sid} par la faction inconnue {}.", siege.attacker),
            );
        }
        for unit in &settlement.garrison {
            check_unit(out, &format!("garnison de {sid}"), unit);
        }
    }
}

fn check_unit(out: &mut Vec<Violation>, place: &str, unit: &Unit) {
    if unit.strength > unit.max_strength {
        push(
            out,
            "unit_strength",
            format!(
                "Unité {} ({place}) : effectif {} supérieur au maximum {}.",
                unit.unit_type, unit.strength, unit.max_strength
            ),
        );
    }
}

/// Armies: one place (a known settlement or a finite field point), a known
/// faction, units sound, general consistent with the character's `army`.
fn check_armies(state: &CampaignState, out: &mut Vec<Violation>) {
    let mut generals: BTreeMap<&CharacterId, Vec<&ArmyId>> = BTreeMap::new();
    for (aid, army) in &state.armies {
        match &army.position {
            ArmyPosition::Settlement(sid) if !state.settlements.contains_key(sid) => push(
                out,
                "army_single_place",
                format!("L'armée {aid} stationne dans la colonie inconnue {sid}."),
            ),
            ArmyPosition::Field { x, y } if !x.is_finite() || !y.is_finite() => push(
                out,
                "army_single_place",
                format!("L'armée {aid} a une position de campagne non finie ({x}, {y})."),
            ),
            _ => {}
        }
        if !state.factions.contains_key(&army.faction) {
            push(
                out,
                "army_refs",
                format!(
                    "L'armée {aid} appartient à la faction inconnue {}.",
                    army.faction
                ),
            );
        }
        if let Some(general) = &army.general {
            generals.entry(general).or_default().push(aid);
            match state.characters.get(general) {
                None => push(
                    out,
                    "army_refs",
                    format!("L'armée {aid} a pour général le personnage inconnu {general}."),
                ),
                Some(c) if c.army.as_ref() != Some(aid) => push(
                    out,
                    "army_refs",
                    format!(
                        "Le général {general} de l'armée {aid} pointe vers {:?}.",
                        c.army
                    ),
                ),
                _ => {}
            }
        }
        for unit in &army.units {
            check_unit(out, &format!("armée {aid}"), unit);
        }
    }
    for (general, armies) in generals {
        if armies.len() > 1 {
            push(
                out,
                "army_single_place",
                format!(
                    "Le général {general} commande {} armées à la fois.",
                    armies.len()
                ),
            );
        }
    }
    for battle in &state.pending_battles {
        for aid in [&battle.attacker, &battle.defender] {
            if !state.armies.contains_key(aid) {
                push(
                    out,
                    "army_refs",
                    format!("Bataille en attente avec l'armée inconnue {aid}."),
                );
            }
        }
        if !state.settlements.contains_key(&battle.location) {
            push(
                out,
                "army_refs",
                format!(
                    "Bataille en attente dans la colonie inconnue {}.",
                    battle.location
                ),
            );
        }
    }
}

/// Treasury bounded, capital/ruler/heir known, nothing left to a dead faction.
fn check_factions(state: &CampaignState, data: &GameData, out: &mut Vec<Violation>) {
    for (fid, faction) in &state.factions {
        if faction.treasury.unsigned_abs() > TREASURY_LIMIT as u64 {
            push(
                out,
                "treasury_finite",
                format!("Le trésor de {fid} est démesuré : {}.", faction.treasury),
            );
        }
        if !data.provinces.contains_key(&faction.capital) {
            push(
                out,
                "faction_refs",
                format!(
                    "La capitale {} de {fid} est inconnue des données.",
                    faction.capital
                ),
            );
        }
        for (role, id) in [("souverain", &faction.ruler), ("héritier", &faction.heir)] {
            if let Some(id) = id.as_ref().filter(|id| !state.characters.contains_key(*id)) {
                push(
                    out,
                    "faction_refs",
                    format!("Le {role} de {fid} est le personnage inconnu {id}."),
                );
            }
        }
        for other in faction.at_war_with.iter().chain(&faction.allies) {
            if !state.factions.contains_key(other) {
                push(
                    out,
                    "faction_refs",
                    format!("{fid} est liée à la faction inconnue {other}."),
                );
            }
        }
        if !faction.alive {
            let armies = state.armies.values().filter(|a| &a.faction == fid).count();
            let places = state
                .settlements
                .values()
                .filter(|s| &s.owner == fid || &s.controller == fid)
                .count();
            if armies > 0 || places > 0 {
                push(
                    out,
                    "dead_faction_assets",
                    format!(
                        "La faction disparue {fid} possède encore {armies} armée(s) et {places} colonie(s)."
                    ),
                );
            }
        }
    }
}

fn check_characters(state: &CampaignState, out: &mut Vec<Violation>) {
    for (cid, c) in &state.characters {
        if !state.factions.contains_key(&c.faction) {
            push(
                out,
                "character_refs",
                format!(
                    "Le personnage {cid} sert la faction inconnue {}.",
                    c.faction
                ),
            );
        }
        if let Some(loc) = c
            .location
            .as_ref()
            .filter(|loc| !state.provinces.contains_key(*loc))
        {
            push(
                out,
                "character_refs",
                format!("Le personnage {cid} se trouve dans la province inconnue {loc}."),
            );
        }
        if let Some(aid) = c
            .army
            .as_ref()
            .filter(|aid| !state.armies.contains_key(*aid))
        {
            push(
                out,
                "character_refs",
                format!("Le personnage {cid} est rattaché à l'armée inconnue {aid}."),
            );
        }
        for (role, id) in [
            ("conjoint", &c.spouse),
            ("père", &c.father),
            ("mère", &c.mother),
        ] {
            if let Some(id) = id.as_ref().filter(|id| !state.characters.contains_key(*id)) {
                push(
                    out,
                    "character_refs",
                    format!("Le {role} de {cid} est le personnage inconnu {id}."),
                );
            }
        }
        for child in &c.children {
            if !state.characters.contains_key(child) {
                push(
                    out,
                    "character_refs",
                    format!("L'enfant {child} de {cid} est inconnu."),
                );
            }
        }
        if let Some(journey) = c
            .journey
            .as_ref()
            .filter(|journey| !state.provinces.contains_key(&journey.to))
        {
            push(
                out,
                "character_refs",
                format!(
                    "Le voyage de {cid} mène à la province inconnue {}.",
                    journey.to
                ),
            );
        }
    }
}

/// Every held title exists in the data and is held by a known faction.
fn check_titles(state: &CampaignState, data: &GameData, out: &mut Vec<Violation>) {
    for (title, holder) in &state.feudal.holders {
        if !data.titles.contains_key(title) {
            push(
                out,
                "title_refs",
                format!("Le titre {title} est détenu mais absent des données."),
            );
        }
        if !state.factions.contains_key(holder) {
            push(
                out,
                "title_refs",
                format!("Le titre {title} est détenu par la faction inconnue {holder}."),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::OnceLock;

    use data_model::{FactionId, TitleId};

    use super::*;
    use crate::test_support;

    fn fixture() -> &'static (GameData, CampaignState) {
        static FIXTURE: OnceLock<(GameData, CampaignState)> = OnceLock::new();
        FIXTURE.get_or_init(|| {
            let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../data");
            let (data, _) = GameData::load(&root).expect("game data loads");
            let state = test_support::start(&data, "fac_france", 7);
            (data, state)
        })
    }

    fn keys(state: &CampaignState) -> Vec<&'static str> {
        let (data, _) = fixture();
        let mut keys: Vec<_> = check_invariants(state, data)
            .iter()
            .map(|v| v.invariant)
            .collect();
        keys.dedup();
        keys
    }

    #[test]
    fn sound_start_has_no_violation() {
        let (data, state) = fixture();
        assert_eq!(check_invariants(state, data), Vec::new());
    }

    #[test]
    fn settlement_listed_twice_or_never() {
        let (_, sound) = fixture();
        let mut state = sound.clone();
        let (pid, province) = state.provinces.iter().next().unwrap();
        let city = province.city.clone();
        let other = state.provinces.keys().find(|p| *p != pid).unwrap().clone();
        state
            .provinces
            .get_mut(&other)
            .unwrap()
            .settlements
            .push(city);
        assert!(keys(&state).contains(&"settlement_single_owner"));

        let mut state = sound.clone();
        let sid = state.settlements.keys().next().unwrap().clone();
        for p in state.provinces.values_mut() {
            p.settlements.retain(|s| *s != sid);
        }
        assert!(keys(&state).contains(&"settlement_single_owner"));
    }

    #[test]
    fn settlement_with_unknown_owner() {
        let mut state = fixture().1.clone();
        let s = state.settlements.values_mut().next().unwrap();
        s.owner = FactionId::new("fac_nowhere").unwrap();
        assert!(keys(&state).contains(&"settlement_refs"));
    }

    #[test]
    fn army_in_unknown_settlement_or_nowhere() {
        let mut state = fixture().1.clone();
        let army = state.armies.values_mut().next().unwrap();
        army.position = ArmyPosition::Settlement(SettlementId::new("set_nowhere").unwrap());
        assert!(keys(&state).contains(&"army_single_place"));

        let mut state = fixture().1.clone();
        let army = state.armies.values_mut().next().unwrap();
        army.position = ArmyPosition::Field {
            x: f32::NAN,
            y: 0.0,
        };
        assert!(keys(&state).contains(&"army_single_place"));
    }

    #[test]
    fn general_commanding_two_armies() {
        let mut state = fixture().1.clone();
        let mut with_general = state
            .armies
            .iter()
            .filter_map(|(id, a)| a.general.clone().map(|g| (id.clone(), g)));
        let (_, general) = with_general.next().unwrap();
        let (second, _) = with_general.next().unwrap();
        state.armies.get_mut(&second).unwrap().general = Some(general);
        let found = keys(&state);
        assert!(found.contains(&"army_single_place"), "{found:?}");
    }

    #[test]
    fn army_of_unknown_faction() {
        let mut state = fixture().1.clone();
        let army = state.armies.values_mut().next().unwrap();
        army.faction = FactionId::new("fac_nowhere").unwrap();
        assert!(keys(&state).contains(&"army_refs"));
    }

    #[test]
    fn unit_above_max_strength() {
        let mut state = fixture().1.clone();
        let army = state.armies.values_mut().next().unwrap();
        army.units[0].strength = army.units[0].max_strength + 1;
        assert!(keys(&state).contains(&"unit_strength"));
    }

    #[test]
    fn absurd_treasury() {
        let mut state = fixture().1.clone();
        state.factions.values_mut().next().unwrap().treasury = i64::MAX;
        assert!(keys(&state).contains(&"treasury_finite"));
    }

    #[test]
    fn dead_faction_keeps_an_army() {
        let mut state = fixture().1.clone();
        let faction = state.armies.values().next().unwrap().faction.clone();
        state.factions.get_mut(&faction).unwrap().alive = false;
        assert!(keys(&state).contains(&"dead_faction_assets"));
    }

    #[test]
    fn dangling_character_and_faction_references() {
        let mut state = fixture().1.clone();
        let ghost = data_model::CharacterId::new("chr_ghost").unwrap();
        state.factions.values_mut().next().unwrap().heir = Some(ghost.clone());
        assert!(keys(&state).contains(&"faction_refs"));

        let mut state = fixture().1.clone();
        state.characters.values_mut().next().unwrap().spouse = Some(ghost);
        assert!(keys(&state).contains(&"character_refs"));
    }

    #[test]
    fn title_with_unknown_holder() {
        let mut state = fixture().1.clone();
        state.feudal.holders.insert(
            TitleId::new("tit_nowhere").unwrap(),
            FactionId::new("fac_nowhere").unwrap(),
        );
        assert!(keys(&state).contains(&"title_refs"));
    }
}
