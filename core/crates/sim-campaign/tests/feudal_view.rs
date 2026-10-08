//! FE6 interface views (spec § 6): faction sheet, obligations, province
//! breadcrumb, feudal map filter and vassal status badges.

use data_model::{GameData, TitleId};
use sim_campaign::feudal::{self, VassalStatus};
use sim_campaign::CampaignState;

use data_model::test_support::{fac, game_data, prov};

fn tit(id: &str) -> TitleId {
    TitleId::new(id).unwrap()
}

fn start(data: &GameData) -> CampaignState {
    let mut state = CampaignState::new_1337(data, fac("fac_france"), 7).expect("1337 start");
    feudal::sync_suzerains(&mut state, data);
    state
}

#[test]
fn feudal_view_breadcrumb_runs_from_kingdom_to_county() {
    let data = game_data();
    let state = start(data);
    let crumbs = feudal::province_breadcrumb(&state, data, &prov("prov_charolais"));
    let titles: Vec<&str> = crumbs.iter().map(|l| l.title.as_str()).collect();
    assert_eq!(titles, ["tit_france", "tit_burgundy", "tit_charolais"]);
    assert_eq!(crumbs[0].holder, Some(fac("fac_france")));
    assert_eq!(crumbs[2].holder, Some(fac("fac_burgundy")));
}

#[test]
fn feudal_view_sheet_of_france_and_a_vassal() {
    let data = game_data();
    let state = start(data);
    let france = feudal::faction_sheet(&state, data, &fac("fac_france"));
    assert_eq!(france.liege, None);
    assert_eq!(france.sovereign, fac("fac_france"));
    assert_eq!(france.status, None);
    assert!(france.direct_vassals.contains(&fac("fac_burgundy")));
    assert!(france.title_vassals.contains(&fac("fac_england")));
    let burgundy = feudal::faction_sheet(&state, data, &fac("fac_burgundy"));
    assert_eq!(burgundy.liege, Some(fac("fac_france")));
    assert_eq!(burgundy.sovereign, fac("fac_france"));
    assert!(burgundy.loyalty.is_some());
    assert!(burgundy.status.is_some());
    assert!(burgundy.titles.contains(&tit("tit_charolais")));
}

#[test]
fn feudal_view_obligations_of_a_vassal() {
    let data = game_data();
    let state = start(data);
    let o = feudal::obligations(&state, data, &fac("fac_burgundy"));
    assert_eq!(o.liege, Some(fac("fac_france")));
    assert_eq!(o.tribute_percent, data.feudal_rules.vassal_tribute_percent);
    assert!(o.grantable_titles.contains(&tit("tit_charolais")));
    assert!(!o.grantable_titles.contains(&tit("tit_burgundy")));
    let france = feudal::obligations(&state, data, &fac("fac_france"));
    assert_eq!(france.liege, None);
    assert_eq!(france.tribute, 0);
}

#[test]
fn feudal_view_map_marks_double_allegiance() {
    let data = game_data();
    let state = start(data);
    let map = feudal::feudal_map(&state, data);
    let guyenne = &map[&prov("prov_guyenne")];
    assert_eq!(guyenne.holder, Some(fac("fac_england")));
    assert_eq!(guyenne.sovereign, Some(fac("fac_france")));
    assert_eq!(guyenne.second_lord, Some(fac("fac_france")));
    let charolais = &map[&prov("prov_charolais")];
    assert_eq!(charolais.holder, Some(fac("fac_burgundy")));
    assert_eq!(charolais.second_lord, None);
}

#[test]
fn feudal_view_status_follows_loyalty_and_felony() {
    let data = game_data();
    let mut state = start(data);
    let burgundy = fac("fac_burgundy");
    state.factions.get_mut(&burgundy).unwrap().loyalty = 100;
    assert_eq!(
        feudal::vassal_status(&state, data, &burgundy),
        Some(VassalStatus::Loyal)
    );
    state.factions.get_mut(&burgundy).unwrap().loyalty = 0;
    assert_eq!(
        feudal::vassal_status(&state, data, &burgundy),
        Some(VassalStatus::Discontent)
    );
    feudal::open_felony(
        &mut state,
        data,
        &burgundy,
        feudal::FelonyReason::RefusedHost,
    );
    assert_eq!(
        feudal::vassal_status(&state, data, &burgundy),
        Some(VassalStatus::Felon)
    );
    let o = feudal::obligations(&state, data, &fac("fac_france"));
    assert!(o.felons.iter().any(|c| c.vassal == burgundy));
}

#[test]
fn feudal_view_homage_candidates_outrank_the_vassal() {
    let data = game_data();
    let state = start(data);
    let lords = feudal::homage_candidates(&state, data, &fac("fac_burgundy"));
    assert!(
        !lords.contains(&fac("fac_france")),
        "current liege excluded"
    );
    assert!(!lords.is_empty(), "{lords:?}");
    for lord in &lords {
        assert!(
            feudal::primary_rank(&state, data, lord)
                > feudal::primary_rank(&state, data, &fac("fac_burgundy"))
        );
    }
}
