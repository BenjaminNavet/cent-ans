//! FE war escalation and private war (spec § 4.3), lot F2.

use std::path::PathBuf;

use data_model::{FactionId, GameData, TitleId};
use sim_campaign::diplomacy::Proposal;
use sim_campaign::feudal::{
    self, Arbitration, Likelihood, PROTECTION_GRANTED_REASON, PROTECTION_REFUSED_REASON,
};
use sim_campaign::CampaignState;

fn data() -> GameData {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../data");
    let (data, _warnings) = GameData::load(&root).expect("game data loads");
    data
}

fn fac(id: &str) -> FactionId {
    FactionId::new(id).unwrap()
}

fn title(id: &str) -> TitleId {
    TitleId::new(id).unwrap()
}

/// Rules forcing every AI suzerain to intervene (`true`) or to shirk.
fn data_with_duty(intervene: bool) -> GameData {
    let mut data = data();
    data.feudal_rules.escalation.score.base = if intervene { 1000 } else { -1000 };
    data
}

fn start(data: &GameData, player: &str) -> CampaignState {
    CampaignState::new_1337(data, fac(player), 7).expect("1337 start")
}

/// 1337 has no count under a duke under a king: Flanders is given the
/// county of Artois (below the duchy of Burgundy, below France) as its
/// primary title, and its faction tie follows (what F1 derives).
fn count_duke_king(state: &mut CampaignState, data: &GameData) {
    let (flanders, burgundy, france) =
        (fac("fac_flanders"), fac("fac_burgundy"), fac("fac_france"));
    state
        .feudal
        .holders
        .insert(title("tit_artois"), flanders.clone());
    state
        .feudal
        .primary
        .insert(flanders.clone(), title("tit_artois"));
    feudal::sync_suzerains(state, data);
    assert_eq!(state.factions[&flanders].suzerain, Some(burgundy.clone()));
    let f = state.factions.get_mut(&flanders).unwrap();
    f.allies.remove(&france);
    f.allies.insert(burgundy.clone());
    state
        .factions
        .get_mut(&france)
        .unwrap()
        .allies
        .remove(&flanders);
    state
        .factions
        .get_mut(&burgundy)
        .unwrap()
        .allies
        .insert(flanders);
}

fn prestige(state: &CampaignState, faction: &str) -> i32 {
    let ruler = state.factions[&fac(faction)].ruler.clone().expect("ruler");
    state.characters[&ruler].prestige
}

fn has_modifier(state: &CampaignState, holder: &str, with: &str, reason: &str) -> bool {
    state.factions[&fac(holder)]
        .modifiers
        .iter()
        .any(|m| m.with == fac(with) && m.reason_fr == reason)
}

#[test]
fn count_duke_king_chain() {
    let data = data_with_duty(true);
    let mut state = start(&data, "fac_castile");
    count_duke_king(&mut state, &data);
    let (flanders, burgundy, france, brabant) = (
        fac("fac_flanders"),
        fac("fac_burgundy"),
        fac("fac_france"),
        fac("fac_brabant"),
    );
    assert_eq!(
        feudal::liege_of(&state, &data, &flanders),
        Some(burgundy.clone())
    );
    assert_eq!(
        feudal::liege_of(&state, &data, &burgundy),
        Some(france.clone())
    );

    let preview = feudal::war_escalation_preview(&state, &data, &brabant, &flanders);
    let chain: Vec<&FactionId> = preview.iter().map(|s| &s.faction).collect();
    assert_eq!(chain, vec![&burgundy, &france]);
    assert!(preview.iter().all(|s| s.likelihood == Likelihood::Likely));
    assert!(preview.iter().all(|s| !s.reason.is_empty()));

    state.declare_war(&data, &brabant, &flanders).unwrap();
    assert!(state.is_at_war(&burgundy, &brabant), "the duke protects");
    assert!(state.is_at_war(&france, &brabant), "the king follows");
    assert!(has_modifier(
        &state,
        "fac_flanders",
        "fac_burgundy",
        PROTECTION_GRANTED_REASON
    ));
    // The king summons the host of his direct vassals, loyal ones march.
    let host = feudal::direct_vassals(&state, &data, &france);
    assert!(host.contains(&burgundy) && !host.contains(&flanders));
    for vassal in host {
        // ADR 0114: only the vassals within reach are summoned.
        if state.factions[&vassal].loyalty >= data.feudal_rules.call_to_arms_loyalty
            && !state.is_allied(&vassal, &brabant)
            && feudal::can_serve(&state, &data, &vassal, &france, &brabant)
        {
            assert!(
                state.is_at_war(&vassal, &brabant),
                "{vassal} answers the host"
            );
        }
    }
    // Sovereign France has no suzerain: the chain ends there.
    assert_eq!(feudal::liege_of(&state, &data, &france), None);
}

#[test]
fn liege_shirks_protection() {
    let data = data_with_duty(false);
    let mut state = start(&data, "fac_castile");
    count_duke_king(&mut state, &data);
    let (flanders, burgundy, brabant) =
        (fac("fac_flanders"), fac("fac_burgundy"), fac("fac_brabant"));
    let preview = feudal::war_escalation_preview(&state, &data, &brabant, &flanders);
    assert_eq!(preview[0].faction, burgundy);
    assert_eq!(preview[0].likelihood, Likelihood::Unlikely);

    let prestige_before = prestige(&state, "fac_burgundy");
    let loyalty_before = state.factions[&flanders].loyalty;
    state.declare_war(&data, &brabant, &flanders).unwrap();
    assert!(!state.is_at_war(&burgundy, &brabant), "the duke shirks");
    let rules = &data.feudal_rules.escalation;
    assert_eq!(
        prestige(&state, "fac_burgundy"),
        prestige_before + rules.shirk_prestige
    );
    assert_eq!(
        state.factions[&flanders].loyalty,
        loyalty_before.saturating_sub(rules.shirk_loyalty_drop)
    );
    assert!(has_modifier(
        &state,
        "fac_flanders",
        "fac_burgundy",
        PROTECTION_REFUSED_REASON
    ));
}

#[test]
fn cascade_stops_at_a_shirking_liege() {
    let (flanders, burgundy, france, brabant) = (
        fac("fac_flanders"),
        fac("fac_burgundy"),
        fac("fac_france"),
        fac("fac_brabant"),
    );

    // The duke shirks: the king is never called.
    let data = data_with_duty(false);
    let mut state = start(&data, "fac_france");
    count_duke_king(&mut state, &data);
    let prestige_before = prestige(&state, "fac_france");
    state.declare_war(&data, &brabant, &flanders).unwrap();
    assert!(!state.is_at_war(&france, &brabant));
    assert!(state.factions[&france].offers.is_empty());
    assert_eq!(prestige(&state, "fac_france"), prestige_before);

    // The duke intervenes: the king (the player) is called by an offer.
    let data = data_with_duty(true);
    let mut state = start(&data, "fac_france");
    count_duke_king(&mut state, &data);
    state.declare_war(&data, &brabant, &flanders).unwrap();
    assert!(state.is_at_war(&burgundy, &brabant));
    assert!(!state.is_at_war(&france, &brabant), "waits for the player");
    let offer = state.factions[&france]
        .offers
        .iter()
        .find(
            |o| matches!(&o.proposal, Proposal::Protection { aggressor } if aggressor == &brabant),
        )
        .expect("call for protection")
        .clone();
    assert_eq!(offer.from, burgundy);

    // The player shirks: prestige and the loyalty of every direct vassal.
    let loyalties: Vec<(FactionId, u8)> = feudal::direct_vassals(&state, &data, &france)
        .into_iter()
        .map(|v| {
            let l = state.factions[&v].loyalty;
            (v, l)
        })
        .collect();
    let prestige_before = prestige(&state, "fac_france");
    let mut refused = state.clone();
    refused
        .answer_offer(&data, &france, offer.id, false)
        .unwrap();
    assert!(!refused.is_at_war(&france, &brabant));
    let rules = &data.feudal_rules.escalation;
    assert_eq!(
        prestige(&refused, "fac_france"),
        prestige_before + rules.shirk_prestige
    );
    for (vassal, before) in loyalties {
        assert_eq!(
            refused.factions[&vassal].loyalty,
            before.saturating_sub(rules.shirk_loyalty_drop),
            "{vassal}"
        );
    }

    // Accepting instead: the king enters the war.
    state.answer_offer(&data, &france, offer.id, true).unwrap();
    assert!(state.is_at_war(&france, &brabant));
}

#[test]
fn private_war_is_arbitrated_by_the_common_liege() {
    let (brittany, flanders, france) =
        (fac("fac_brittany"), fac("fac_flanders"), fac("fac_france"));

    // AI lord strong enough and even-handed: it imposes peace.
    let mut data = data();
    data.feudal_rules
        .escalation
        .arbitration
        .take_side_attitude_gap = 1000;
    data.feudal_rules
        .escalation
        .arbitration
        .impose_peace_power_ratio = 0.0;
    let mut state = start(&data, "fac_castile");
    assert_eq!(
        feudal::common_liege(&state, &data, &brittany, &flanders),
        Some(france.clone())
    );
    let preview = feudal::war_escalation_preview(&state, &data, &brittany, &flanders);
    assert_eq!(preview.len(), 1);
    assert_eq!(preview[0].faction, france);
    assert_eq!(preview[0].likelihood, Likelihood::Likely);
    assert!(preview[0].reason.contains("paix"), "{}", preview[0].reason);
    state.declare_war(&data, &brittany, &flanders).unwrap();
    assert!(!state.is_at_war(&brittany, &flanders), "peace imposed");
    assert!(state.has_truce(&brittany, &flanders));
    assert!(!state.is_at_war(&france, &brittany) && !state.is_at_war(&france, &flanders));

    // Too weak and even-handed: it lets them fight.
    data.feudal_rules
        .escalation
        .arbitration
        .impose_peace_power_ratio = 1.0e9;
    let mut state = start(&data, "fac_castile");
    state.declare_war(&data, &brittany, &flanders).unwrap();
    assert!(state.is_at_war(&brittany, &flanders));
    assert!(!state.is_at_war(&france, &brittany));

    // The player as lord gets an arbitration offer and takes a side.
    let mut state = start(&data, "fac_france");
    state.declare_war(&data, &brittany, &flanders).unwrap();
    assert!(state.is_at_war(&brittany, &flanders));
    let offer = state.factions[&france]
        .offers
        .iter()
        .find(|o| matches!(o.proposal, Proposal::Arbitration { .. }))
        .expect("arbitration offer")
        .clone();
    let loyalty_before = state.factions[&brittany].loyalty;
    state
        .arbitrate(
            &data,
            &france,
            offer.id,
            Arbitration::TakeSide {
                side: flanders.clone(),
            },
        )
        .unwrap();
    assert!(state.is_at_war(&france, &brittany));
    assert!(state.factions[&brittany].loyalty < loyalty_before || loyalty_before == 0);
    assert!(state.factions[&france]
        .offers
        .iter()
        .all(|o| o.id != offer.id));
}
