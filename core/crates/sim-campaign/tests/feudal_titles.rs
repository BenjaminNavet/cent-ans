//! FE title transfers, forfeiture, inheritance, conquest, objectives (spec § 4.4-4.8), lot F3.

use std::path::PathBuf;

use data_model::{CharacterId, FactionId, GameData, ProvinceId, SuccessionLaw, TitleId};
use sim_campaign::feudal::{self, FelonyReason, FeudalError, Grantee, TitleDemandOutcome};
use sim_campaign::negotiation::{apply_treaty, Article, Party};
use sim_campaign::victory::OutcomeKind;
use sim_campaign::CampaignState;

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

fn tit(id: &str) -> TitleId {
    TitleId::new(id).unwrap()
}

fn chr(id: &str) -> CharacterId {
    CharacterId::new(id).unwrap()
}

fn start(data: &GameData) -> CampaignState {
    CampaignState::new_1337(data, fac("fac_france"), 7).expect("1337 start")
}

fn holder(s: &CampaignState, title: &str) -> Option<FactionId> {
    feudal::holder_of(s, &tit(title)).cloned()
}

/// Leaves `ruler` as the last of its line: every other character of its
/// faction and every child of the ruler dies, and the ruler's house gets a
/// name nobody else bears.
fn last_of_line(s: &mut CampaignState, ruler: &str, house: &str) {
    let ruler = chr(ruler);
    let faction = s.characters[&ruler].faction.clone();
    for (id, c) in s.characters.iter_mut() {
        let child = c.mother.as_ref() == Some(&ruler) || c.father.as_ref() == Some(&ruler);
        if *id != ruler && (c.faction == faction || child) {
            c.alive = false;
        }
    }
    s.characters.get_mut(&ruler).unwrap().house = house.to_owned();
}

fn kill(s: &mut CampaignState, data: &GameData, id: &str) {
    let mut events = Vec::new();
    sim_campaign::characters::kill(s, data, &chr(id), &mut events);
}

#[test]
fn guyenne_forfeiture() {
    let data = data();
    let mut s = start(&data);
    let (france, england) = (fac("fac_france"), fac("fac_england"));
    // England is no direct vassal of France, but holds Guyenne of it.
    assert_eq!(feudal::liege_of(&s, &data, &england), None);
    assert_eq!(
        feudal::declare_commise(&mut s, &data, &france, &england),
        Err(FeudalError::NoFelonyCase(england.clone()))
    );
    let case = feudal::open_felony_towards(
        &mut s,
        &data,
        &england,
        &france,
        FelonyReason::AlliedWithEnemy,
    )
    .expect("England holds Guyenne of France");
    assert_eq!(
        case.expires_turn,
        s.turn + data.feudal_rules.felony_window_turns
    );
    // Not a vassal of England: no case the other way round.
    assert!(
        feudal::open_felony_towards(&mut s, &data, &france, &england, FelonyReason::Revolt)
            .is_none()
    );
    feudal::declare_commise(&mut s, &data, &france, &england).expect("forfeiture declared");
    assert!(s.is_at_war(&france, &england));
    assert!(feudal::has_forfeiture(&s, &france, &england));
    assert_eq!(
        s.casus_belli(&data, &france, &england).as_deref(),
        Some("commise")
    );
    assert!(s.feudal.felonies.is_empty(), "the case is used up");

    // France wins the war: the peace executes the forfeiture.
    s.factions
        .get_mut(&france)
        .unwrap()
        .war_scores
        .insert(england.clone(), 60);
    apply_treaty(&mut s, &data, &france, &england, &[Article::Peace]).expect("peace");
    for title in ["tit_guyenne", "tit_ponthieu"] {
        assert_eq!(holder(&s, title), Some(france.clone()), "{title}");
    }
    assert_eq!(s.province_owner(&prov("prov_guyenne")), Some(&france));
    assert_eq!(holder(&s, "tit_england"), Some(england.clone()));
    assert!(s.factions[&england].alive);
    assert!(s.feudal.forfeitures.is_empty());
    assert_eq!(s.feudal.primary[&france], tit("tit_france"));
}

#[test]
fn lost_forfeiture_war_keeps_the_fief() {
    let data = data();
    let mut s = start(&data);
    let (france, england) = (fac("fac_france"), fac("fac_england"));
    feudal::open_felony_towards(&mut s, &data, &england, &france, FelonyReason::RefusedHost)
        .unwrap();
    feudal::declare_commise(&mut s, &data, &france, &england).unwrap();
    s.factions
        .get_mut(&france)
        .unwrap()
        .war_scores
        .insert(england.clone(), -30);
    apply_treaty(&mut s, &data, &france, &england, &[Article::Peace]).unwrap();
    assert_eq!(holder(&s, "tit_guyenne"), Some(england));
    assert!(s.feudal.forfeitures.is_empty());
}

#[test]
fn brittany_1341_two_claimants() {
    let data = data();
    let mut s = start(&data);
    let (france, england, brittany) = (fac("fac_france"), fac("fac_england"), fac("fac_brittany"));
    let (jeanne, montfort, blois) = (
        chr("chr_jeanne_de_penthievre"),
        chr("chr_jean_de_montfort"),
        chr("chr_charles_de_blois"),
    );
    assert_eq!(feudal::liege_of(&s, &data, &brittany), Some(france.clone()));
    assert_eq!(s.factions[&brittany].heir.as_ref(), Some(&jeanne));
    // Jeanne de Penthièvre married Charles de Blois, nephew of Philip VI.
    s.characters.get_mut(&jeanne).unwrap().spouse = Some(blois.clone());
    s.characters.get_mut(&blois).unwrap().spouse = Some(jeanne.clone());
    if !s.is_at_war(&england, &france) {
        s.declare_war(&data, &england, &france).unwrap();
    }

    kill(&mut s, &data, "chr_jean_iii_de_bretagne");

    assert_eq!(s.factions[&brittany].ruler.as_ref(), Some(&jeanne));
    let dispute = s.feudal.disputes.last().expect("contested succession");
    assert_eq!(dispute.faction, brittany);
    assert_eq!(dispute.title, tit("tit_brittany"));
    assert_eq!(dispute.arbiter, france);
    assert_eq!(dispute.winner, jeanne);
    assert!(dispute.claimants.contains(&montfort));
    assert_eq!(dispute.sponsor.as_ref(), Some(&england));
    // Montfort flees to England, which presses his claim by war.
    assert_eq!(s.characters[&montfort].faction, england);
    assert!(s.is_at_war(&england, &brittany));
    assert!(s.factions[&england]
        .claims
        .iter()
        .any(|c| c.faction.as_ref() == Some(&brittany)));
    assert_eq!(holder(&s, "tit_brittany"), Some(brittany));
}

#[test]
fn burgundy_1361_duchy_and_county_split() {
    let mut data = data();
    // The duchy follows proximity in the male line (John II), the counties
    // the eldest heir (Marguerite): per-title laws.
    data.titles
        .get_mut(&tit("tit_burgundy"))
        .unwrap()
        .succession_law = Some(SuccessionLaw::Salic);
    for county in ["tit_franche_comte", "tit_artois"] {
        data.titles.get_mut(&tit(county)).unwrap().succession_law =
            Some(SuccessionLaw::CognaticPrimogeniture);
    }
    let mut s = start(&data);
    let (france, flanders, burgundy) =
        (fac("fac_france"), fac("fac_flanders"), fac("fac_burgundy"));
    let house = "Bourgogne (test)";
    last_of_line(&mut s, "chr_eudes_iv", house);
    // A common ancestor of the house, dead, mother of both heirs.
    let mother = chr("chr_jeanne_de_bourgogne");
    {
        let m = s.characters.get_mut(&mother).unwrap();
        m.house = house.to_owned();
        m.alive = false;
    }
    let john = chr("chr_jean_de_normandie");
    let marguerite = chr("chr_bonne_de_luxembourg");
    s.characters.get_mut(&john).unwrap().mother = Some(mother.clone());
    {
        let m = s.characters.get_mut(&marguerite).unwrap();
        m.mother = Some(mother.clone());
        m.faction = flanders.clone();
        m.birth_year = 1310;
        m.spouse = None;
    }

    kill(&mut s, &data, "chr_eudes_iv");

    assert_eq!(holder(&s, "tit_burgundy"), Some(france.clone()));
    assert_eq!(holder(&s, "tit_franche_comte"), Some(flanders.clone()));
    assert_eq!(holder(&s, "tit_artois"), Some(flanders.clone()));
    assert_eq!(s.province_owner(&prov("prov_bourgogne")), Some(&france));
    assert_eq!(
        s.province_owner(&prov("prov_franche_comte")),
        Some(&flanders)
    );
    assert_eq!(s.province_owner(&prov("prov_artois")), Some(&flanders));
    assert!(
        !s.factions[&burgundy].alive,
        "no title left: Burgundy vanishes"
    );
    assert!(!s.feudal.primary.contains_key(&burgundy));
    assert!(s.armies.values().all(|a| a.faction != burgundy));
}

#[test]
fn personal_union() {
    let data = data();
    let mut s = start(&data);
    let (castile, navarre) = (fac("fac_castile"), fac("fac_navarre"));
    let house = "Évreux (test)";
    last_of_line(&mut s, "chr_jeanne_ii_de_navarre", house);
    s.characters.get_mut(&chr("chr_alfonso_xi")).unwrap().mother =
        Some(chr("chr_jeanne_ii_de_navarre"));
    let armies_before = s.armies.values().filter(|a| a.faction == navarre).count();

    kill(&mut s, &data, "chr_jeanne_ii_de_navarre");

    assert_eq!(holder(&s, "tit_navarre"), Some(castile.clone()));
    assert_eq!(holder(&s, "tit_angoumois"), Some(castile.clone()));
    assert_eq!(s.feudal.primary[&castile], tit("tit_castile"));
    assert!(!s.factions[&navarre].alive);
    assert_eq!(s.province_owner(&prov("prov_navarra")), Some(&castile));
    assert_eq!(
        s.armies.values().filter(|a| a.faction == navarre).count(),
        0
    );
    assert!(armies_before == 0 || s.armies.values().any(|a| a.faction == castile));
    // Castile now holds Angoumois of France: a title vassal of the crown.
    assert!(feudal::title_vassals(&s, &data, &fac("fac_france")).contains(&castile));
}

#[test]
fn escheat_without_heir() {
    let data = data();
    let mut s = start(&data);
    let (france, brittany) = (fac("fac_france"), fac("fac_brittany"));
    last_of_line(&mut s, "chr_jean_iii_de_bretagne", "Dreux (test)");
    // Jeanne de Penthièvre, designated heir, rules her own county (F4a):
    // no heir at all here.
    s.factions.get_mut(&brittany).unwrap().heir = None;

    kill(&mut s, &data, "chr_jean_iii_de_bretagne");

    assert_eq!(holder(&s, "tit_brittany"), Some(france.clone()));
    assert_eq!(s.province_owner(&prov("prov_bretagne")), Some(&france));
    assert_eq!(
        s.province_owner(&prov("prov_bretagne_ouest")),
        Some(&france)
    );
    assert!(!s.factions[&brittany].alive);
}

#[test]
fn vacant_title_frees_its_vassals() {
    let data = data();
    let mut s = start(&data);
    let brittany = fac("fac_brittany");
    assert!(feudal::liege_of(&s, &data, &brittany).is_some());
    feudal::vacate_title(&mut s, &data, &tit("tit_france"));
    assert_eq!(feudal::liege_of(&s, &data, &brittany), None);
    assert_eq!(holder(&s, "tit_france"), None);
    // France still holds its Norman duchies, now its primary title.
    let primary = &s.feudal.primary[&fac("fac_france")];
    assert_eq!(data.titles[primary].rank, data_model::TitleRank::Duchy);
}

#[test]
fn victory_by_independence() {
    let mut data = data();
    data.feudal_rules.independence_turns = 2;
    let brittany = fac("fac_brittany");
    let mut s = CampaignState::new_1337(&data, brittany.clone(), 7).expect("start");
    assert_eq!(
        s.feudal.start_crowns.get(&brittany),
        Some(&tit("tit_france"))
    );
    s.end_turn(&data);
    assert!(s.outcome.is_none(), "a vassal wins nothing by waiting");
    feudal::vacate_title(&mut s, &data, &tit("tit_france"));
    s.end_turn(&data);
    assert!(s.outcome.is_none());
    s.end_turn(&data);
    let outcome = s.outcome.as_ref().expect("independence held two turns");
    assert_eq!(outcome.kind, OutcomeKind::Victory);
    assert!(
        outcome.text_fr.contains("indépendance"),
        "{}",
        outcome.text_fr
    );
}

#[test]
fn conquered_title_is_usurped_or_granted() {
    let data = data();
    let mut s = start(&data);
    let (france, england, brittany) = (fac("fac_france"), fac("fac_england"), fac("fac_brittany"));
    if !s.is_at_war(&england, &france) {
        s.declare_war(&data, &england, &france).unwrap();
    }
    // A duchy is below England's crown: granted to England's strongest
    // direct vassal if any, else kept.
    let articles = [
        Article::Peace,
        Article::DemandTitle {
            giver: Party::Recipient,
            title: tit("tit_normandie"),
        },
    ];
    apply_treaty(&mut s, &data, &england, &france, &articles).expect("treaty");
    let normandy = holder(&s, "tit_normandie").unwrap();
    let english_vassals = feudal::direct_vassals(&s, &data, &england);
    assert!(
        normandy == england || english_vassals.contains(&normandy),
        "{normandy:?} not in {english_vassals:?}"
    );
    assert_eq!(s.province_owner(&prov("prov_normandie")), Some(&normandy));
    // A crown is usurped by a duke.
    let outcome = feudal::conquer_title(&mut s, &data, &brittany, &tit("tit_france")).unwrap();
    assert_eq!(outcome, TitleDemandOutcome::Usurped);
    assert_eq!(holder(&s, "tit_france"), Some(brittany.clone()));
    assert_eq!(s.feudal.primary[&brittany], tit("tit_france"));
    // A treaty cannot take a faction's last title (Navarre: Navarre, Évreux,
    // Angoulême).
    let navarre = fac("fac_navarre");
    let all = [
        Article::DemandTitle {
            giver: Party::Recipient,
            title: tit("tit_navarre"),
        },
        Article::DemandTitle {
            giver: Party::Recipient,
            title: tit("tit_evreux"),
        },
        Article::DemandTitle {
            giver: Party::Recipient,
            title: tit("tit_angoumois"),
        },
    ];
    assert!(sim_campaign::negotiation::check_treaty(&s, &data, &england, &navarre, &all).is_err());
    let partial = sim_campaign::negotiation::check_treaty(&s, &data, &england, &navarre, &all[2..]);
    assert!(partial.is_ok(), "{partial:?}");
}

#[test]
fn grant_to_a_courtier_founds_a_vassal_faction() {
    let data = data();
    let mut s = start(&data);
    let france = fac("fac_france");
    let blois = chr("chr_godefroy_d_harcourt");
    assert_eq!(
        feudal::grant_title(
            &mut s,
            &data,
            &france,
            &tit("tit_france"),
            Grantee::Character(blois.clone())
        ),
        Err(FeudalError::PrimaryTitle(tit("tit_france")))
    );
    let new = feudal::grant_title(
        &mut s,
        &data,
        &france,
        &tit("tit_normandie"),
        Grantee::Character(blois.clone()),
    )
    .expect("granted");
    assert!(s.factions[&new].alive);
    assert_eq!(s.factions[&new].ruler.as_ref(), Some(&blois));
    assert_eq!(s.characters[&blois].faction, new);
    assert_eq!(s.feudal.primary[&new], tit("tit_normandie"));
    assert_eq!(feudal::liege_of(&s, &data, &new), Some(france.clone()));
    assert_eq!(s.province_owner(&prov("prov_normandie")), Some(&new));
    assert!(feudal::direct_vassals(&s, &data, &france).contains(&new));
}

#[test]
fn objectives_are_evaluated() {
    let mut data = data();
    let brittany = fac("fac_brittany");
    data.titles
        .get_mut(&tit("tit_brittany"))
        .unwrap()
        .objectives = vec![data_model::TitleObjective {
        id: "obj_free".to_owned(),
        title: "Duché libre".to_owned(),
        description: "Ne relever de personne.".to_owned(),
        condition: data_model::TitleObjectiveCondition::BeIndependent,
    }];
    let mut s = start(&data);
    let progress = feudal::evaluate_objectives(&s, &data);
    let mine: Vec<_> = progress.iter().filter(|p| p.faction == brittany).collect();
    assert_eq!(mine.len(), 1);
    assert!(!mine[0].met);
    feudal::vacate_title(&mut s, &data, &tit("tit_france"));
    assert!(feudal::objective_status(&s, &data, &brittany)[0].met);
    assert_eq!(
        feudal::generic_victory(&s, &data, &brittany),
        Some(feudal::GenericVictory::Objectives)
    );
}

/// No faction may win on its title objectives at the 1337 start: at least
/// one objective of every primary title must still be to achieve.
#[test]
fn no_objective_victory_at_start() {
    let data = data();
    let s = start(&data);
    let mut already_won: Vec<String> = s
        .feudal
        .primary
        .keys()
        .filter(|f| feudal::generic_victory(&s, &data, f).is_some())
        .map(|f| f.to_string())
        .collect();
    already_won.sort();
    assert!(already_won.is_empty(), "won at start: {already_won:?}");
}
