//! Map lens view (ADR 0357): the per-province values the campaign map filters tint by, seen from
//! one faction. Replaces `CampaignSim.get_map_lens`, `get_province_religion` (the map part) and
//! `get_feudal_map`. No rule here: everything comes from `sim_campaign::{map_lens, feudal,
//! religion, conversion}`; the colours and legends live in the client.

use std::collections::BTreeMap;

use data_model::{FactionId, GameData, ProvinceId};
use serde::Serialize;
use sim_campaign::feudal::feudal_map;
use sim_campaign::map_lens::map_lens;
use sim_campaign::religion::{faction_religion, religion_display};
use sim_campaign::CampaignState;

/// Which families of values to compute (religion and feudal ones cost a pass over the titles).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct LensWanted {
    pub religion: bool,
    pub feudal: bool,
}

/// Faith of a province as the religion filter shows it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ReligionCell {
    /// Faith id the filter colours by: the controller's faith when it has one (the Church's
    /// obedience follows its lord), else the province's own.
    pub religion: String,
    /// Display name of the province's own faith (`get_province_religion.religion_name`).
    pub religion_name: String,
    /// Share (0-100) of the population following the heresy.
    pub heresy: u8,
    pub heresy_name: String,
    /// Conversion progress (0-100) towards `conversion_to_name`, 0 when nothing converts.
    pub conversion_progress: u8,
    pub conversion_to_name: String,
}

/// Feudal colouring of a province (`FeudalCell`, ids as strings, empty when none).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FeudalLensCell {
    /// Sovereign at the top of the chain (background).
    pub sovereign: String,
    /// Holder of the province's own title (hatching when not the sovereign).
    pub holder: String,
    /// Second lord of a double allegiance (party shield), empty when none.
    pub second_lord: String,
}

/// Values of one province for the map filters.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct LensCell {
    pub income: f64,
    pub population: u64,
    pub unrest: f64,
    /// Loyalty (0-100) of the owner when it is a vassal.
    pub vassal_loyalty: Option<u8>,
    /// Suzerain of the owner when it is a vassal (empty otherwise).
    pub suzerain: String,
    /// Seasonal supply change of a viewer army without general standing here.
    pub supply_change: i8,
    /// `""`, `ours`, `against_us` or `contested`.
    pub claim: String,
    pub religion: Option<ReligionCell>,
    pub feudal: Option<FeudalLensCell>,
}

/// Every province's lens, plus the rule value of the worst (winter) supply loss.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct LensView {
    pub cells: BTreeMap<String, LensCell>,
    /// Supply lost per winter season outside friendly land (`economy_rules.supply_loss_winter`).
    pub supply_loss_winter: u8,
}

/// Lens of every province seen from `viewer`.
pub fn lens_view(
    data: &GameData,
    state: &CampaignState,
    viewer: &FactionId,
    wanted: LensWanted,
) -> LensView {
    let feudal = wanted.feudal.then(|| feudal_map(state, data));
    let name_of = |faction: Option<&FactionId>| faction.map_or_else(String::new, |f| f.to_string());
    let cells = map_lens(state, data, viewer)
        .into_iter()
        .map(|(id, lens)| {
            let religion = wanted
                .religion
                .then(|| religion_cell(data, state, &id))
                .flatten();
            let feudal = feudal
                .as_ref()
                .and_then(|map| map.get(&id))
                .map(|cell| FeudalLensCell {
                    sovereign: name_of(cell.sovereign.as_ref()),
                    holder: name_of(cell.holder.as_ref()),
                    second_lord: name_of(cell.second_lord.as_ref()),
                });
            (
                id.to_string(),
                LensCell {
                    income: lens.income,
                    population: lens.population,
                    unrest: lens.unrest,
                    vassal_loyalty: lens.vassal_loyalty,
                    suzerain: name_of(lens.suzerain.as_ref()),
                    supply_change: lens.supply_change,
                    claim: lens.claim.key().to_owned(),
                    religion,
                    feudal,
                },
            )
        })
        .collect();
    LensView {
        cells,
        supply_loss_winter: data.economy_rules.supply_loss_winter,
    }
}

/// Port of the map part of `get_province_religion` + the controller's faith of `_religion_colors`.
fn religion_cell(data: &GameData, state: &CampaignState, id: &ProvinceId) -> Option<ReligionCell> {
    let (province, province_data) = (state.province_state(id)?, data.provinces.get(id)?);
    let faith = state
        .province_faith(data, id)
        .unwrap_or_else(|| province_data.religion.clone());
    let goal = state.conversion_goal(data, id).map(|(goal, _)| goal);
    let controller_faith = state
        .province_controller(id)
        .and_then(|controller| faction_religion(state, data, controller));
    let heresy_name = province
        .heresy_religion
        .as_ref()
        .and_then(|r| data.religions.get(r))
        .map_or_else(String::new, |r| r.name.display.clone());
    Some(ReligionCell {
        religion: controller_faith
            .unwrap_or_else(|| faith.clone())
            .to_string(),
        religion_name: religion_display(state, data, &faith),
        heresy: province.heresy,
        heresy_name,
        conversion_progress: if goal.is_some() {
            province.conversion_progress
        } else {
            0
        },
        conversion_to_name: goal.map_or_else(String::new, |g| religion_display(state, data, &g)),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup() -> (&'static GameData, CampaignState, FactionId) {
        let data = data_model::test_support::game_data();
        let state = sim_campaign::test_support::start_quiet(data, "fac_france", 1337);
        (data, state, FactionId::new("fac_france").unwrap())
    }

    #[test]
    fn lens_matches_the_core_for_every_province() {
        let (data, state, viewer) = setup();
        let view = lens_view(data, &state, &viewer, LensWanted::default());
        let core = map_lens(&state, data, &viewer);
        assert_eq!(view.cells.len(), core.len());
        assert!(view.cells.len() > 100);
        for (id, lens) in &core {
            let cell = &view.cells[id.as_str()];
            assert_eq!(cell.population, lens.population);
            assert_eq!(cell.claim, lens.claim.key());
            assert_eq!(cell.vassal_loyalty, lens.vassal_loyalty);
            assert!(cell.religion.is_none() && cell.feudal.is_none());
        }
        assert!(view.supply_loss_winter > 0);
    }

    #[test]
    fn religion_and_feudal_cells_are_filled_on_request() {
        let (data, state, viewer) = setup();
        let wanted = LensWanted {
            religion: true,
            feudal: true,
        };
        let view = lens_view(data, &state, &viewer, wanted);
        let paris = &view.cells["prov_ile_de_france"];
        let faith = paris.religion.as_ref().expect("religion cell");
        assert!(faith.religion.starts_with("rel_"), "{}", faith.religion);
        assert!(!faith.religion_name.is_empty());
        let feudal = paris.feudal.as_ref().expect("feudal cell");
        assert_eq!(feudal.sovereign, "fac_france");
        // every religion cell names a faith, and at least one faith family is not Catholic
        let faiths: std::collections::BTreeSet<_> = view
            .cells
            .values()
            .filter_map(|c| c.religion.as_ref().map(|r| r.religion.clone()))
            .collect();
        assert!(faiths.len() >= 2, "{faiths:?}");
        // the feudal map names a sovereign wherever there is a holder
        assert!(view
            .cells
            .values()
            .filter_map(|c| c.feudal.as_ref())
            .all(|f| f.holder.is_empty() || !f.sovereign.is_empty()));
    }

    #[test]
    fn claims_and_loyalty_follow_the_viewer() {
        let (data, state, _) = setup();
        let england = FactionId::new("fac_england").unwrap();
        let from_france = lens_view(
            data,
            &state,
            &FactionId::new("fac_france").unwrap(),
            LensWanted::default(),
        );
        let from_england = lens_view(data, &state, &england, LensWanted::default());
        let claims = |v: &LensView| v.cells.values().filter(|c| !c.claim.is_empty()).count();
        assert!(claims(&from_france) > 0 || claims(&from_england) > 0);
        assert_ne!(from_france, from_england);
        assert!(serde_json::to_string(&from_france)
            .unwrap()
            .contains("supply_loss_winter"));
    }
}
