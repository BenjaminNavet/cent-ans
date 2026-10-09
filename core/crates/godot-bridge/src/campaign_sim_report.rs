//! `CampaignSim` figures of the campaign for the end screen's report (WH turn).

use godot::prelude::*;

use crate::campaign_sim::{CampaignSim, Ctx};

#[godot_api(secondary)]
impl CampaignSim {
    /// `{start_year, year, turns, provinces_start, provinces_peak,
    /// provinces_end, battles_won, battles_lost, objectives_done,
    /// objectives_total, prestige, treasury, score_provinces,
    /// score_objectives, score_prestige, score_treasury, score_other, score}`.
    #[func]
    fn get_campaign_report(&self) -> VarDictionary {
        let Some(Ctx { state, data }) = self.ctx() else {
            return VarDictionary::new();
        };
        let r = state.campaign_report(data);
        vdict! {
            "start_year" => i64::from(r.start_year),
            "year" => i64::from(r.year),
            "turns" => i64::from(r.turns),
            "provinces_start" => i64::from(r.provinces_start),
            "provinces_peak" => i64::from(r.provinces_peak),
            "provinces_end" => i64::from(r.provinces_end),
            "battles_won" => i64::from(r.battles_won),
            "battles_lost" => i64::from(r.battles_lost),
            "objectives_done" => i64::from(r.objectives_done),
            "objectives_total" => i64::from(r.objectives_total),
            "prestige" => i64::from(r.prestige),
            "treasury" => r.treasury,
            "score_provinces" => r.score_provinces,
            "score_objectives" => r.score_objectives,
            "score_prestige" => r.score_prestige,
            "score_treasury" => r.score_treasury,
            "score_other" => r.score_other,
            "score" => r.score,
        }
    }
}
