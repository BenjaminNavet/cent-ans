//! `CampaignSim` army replenishment and recruitment pools (lot TW2-T2,
//! ADR 0102): the season's replenishment of an army with the factors of its
//! rate (army sheet tooltip). The pools themselves ride on
//! `get_recruitable` (`pool_*` keys).

use godot::prelude::*;
use sim_campaign::{ArmyId, FactorKind};

use crate::campaign_sim::{CampaignSim, Ctx};

#[godot_api(secondary)]
impl CampaignSim {
    /// What `army_id` regains at the end of this season: `{percent, men,
    /// missing, cost, territory, blocked, factors: [{kind, label, percent,
    /// text}], tooltip}`. `percent` is the share of the missing men,
    /// `blocked` the French reason when nothing comes back (`""`
    /// otherwise), `kind` one of `base`, `multiplier`, `bonus`, `tooltip`
    /// the French lines ready to show. Empty for an unknown army.
    #[func]
    fn get_army_replenishment(&self, army_id: GString) -> VarDictionary {
        let Some(Ctx { state, data }) = self.ctx() else {
            return VarDictionary::new();
        };
        let Some(army) = ArmyId::parse(&army_id.to_string()) else {
            return VarDictionary::new();
        };
        let Some(preview) = state.army_replenishment(data, &army) else {
            return VarDictionary::new();
        };
        let territory = state.army_territory(data, &army);
        let mut factors = VarArray::new();
        let mut lines = vec![format!(
            "Reconstitution : {} % des hommes manquants par saison",
            preview.percent()
        )];
        for factor in &preview.factors {
            let kind = match factor.kind {
                FactorKind::Base => "base",
                FactorKind::Multiplier => "multiplier",
                FactorKind::Bonus => "bonus",
            };
            let text = factor.text_fr();
            lines.push(format!("· {text}"));
            factors.push(
                &vdict! {
                    "kind" => kind,
                    "label" => factor.label.as_str(),
                    "percent" => i64::from(factor.percent),
                    "text" => text.as_str(),
                }
                .to_variant(),
            );
        }
        match &preview.blocked {
            Some(reason) => lines.push(format!("Aucun renfort : {reason}")),
            None => lines.push(format!(
                "Fin de saison : +{} hommes pour {} livres",
                preview.men, preview.cost
            )),
        }
        vdict! {
            "percent" => i64::from(preview.percent()),
            "men" => i64::from(preview.men),
            "missing" => i64::from(preview.missing),
            "cost" => i64::from(preview.cost),
            "territory" => territory.key(),
            "blocked" => preview.blocked.as_deref().unwrap_or(""),
            "factors" => &factors,
            "tooltip" => lines.join("\n").as_str(),
        }
    }
}
