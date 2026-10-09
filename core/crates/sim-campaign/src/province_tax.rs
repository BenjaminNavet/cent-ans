//! WH econ: a tax bracket per province (Total War's province taxes).
//!
//! The faction's bracket ([`crate::economy::TaxRate`], `SetTaxRate`) applies
//! to every province unless the controller sets another one for a province
//! alone (`SetProvinceTax`). The choice follows the usual per-province policy
//! rules ([`crate::province_policy`]): it lapses when the controller changes
//! and can be changed once per turn. Everything that reads a tax bracket for
//! a province (tax income, burden of the unrest target) goes through
//! [`CampaignState::province_tax_rate`].

use data_model::{FactionId, ProvinceId};
use serde::{Deserialize, Serialize};

use crate::economy::TaxRate;
use crate::province_policy::{changed_this_turn, controller_choice, ProvincePolicy};
use crate::state::CampaignState;

/// A province's own bracket, chosen by its controller.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaxChoice {
    pub rate: TaxRate,
    /// Faction that chose it; the choice lapses when the controller changes.
    pub faction: FactionId,
    /// Turn of the change (one change per province and per turn).
    pub turn: u32,
}

impl ProvincePolicy for TaxChoice {
    fn faction(&self) -> &FactionId {
        &self.faction
    }
    fn turn(&self) -> u32 {
        self.turn
    }
}

/// Why a `SetProvinceTax` order was refused (French messages for the UI).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ProvinceTaxError {
    #[error("{0} n'est pas contrôlée par votre faction")]
    NotControlled(String),
    #[error("l'impôt de {0} a déjà été changé ce tour-ci")]
    AlreadyChanged(String),
}

impl CampaignState {
    /// Bracket applied in `province`: its controller's own choice for the
    /// province, else `faction_rate`.
    pub fn province_tax_rate(&self, province: &ProvinceId, faction_rate: TaxRate) -> TaxRate {
        self.provinces
            .get(province)
            .and_then(|p| controller_choice(self, province, p.tax_override.as_ref()))
            .map_or(faction_rate, |choice| choice.rate)
    }

    /// Bracket applied in `province` under its controller's faction bracket.
    pub fn effective_province_tax(&self, province: &ProvinceId) -> TaxRate {
        let default = self
            .province_controller(province)
            .and_then(|f| self.factions.get(f))
            .map(|f| f.tax_rate)
            .unwrap_or_default();
        self.province_tax_rate(province, default)
    }

    /// `true` when `province` carries a choice of its own.
    pub fn has_province_tax(&self, province: &ProvinceId) -> bool {
        self.provinces
            .get(province)
            .and_then(|p| controller_choice(self, province, p.tax_override.as_ref()))
            .is_some()
    }
}

/// Validates and applies `SetProvinceTax { province, rate }` for `faction`.
pub fn set_province_tax(
    state: &mut CampaignState,
    faction: &FactionId,
    province: &ProvinceId,
    rate: Option<TaxRate>,
) -> Result<(), ProvinceTaxError> {
    let name = province.to_string();
    if !state.controls_province(faction, province) {
        return Err(ProvinceTaxError::NotControlled(name));
    }
    let current = state.provinces.get(province).expect("checked above");
    if changed_this_turn(state, current.tax_override.as_ref(), faction) {
        return Err(ProvinceTaxError::AlreadyChanged(name));
    }
    let turn = state.turn;
    state
        .provinces
        .get_mut(province)
        .expect("checked")
        .tax_override = rate.map(|rate| TaxChoice {
        rate,
        faction: faction.clone(),
        turn,
    });
    Ok(())
}
