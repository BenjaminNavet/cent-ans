//! Council of the realm, mirroring `data/schemas/council_offices.schema.json`
//! (`data/council/offices.json`, ADR 0341).
//!
//! The rules that read it (appointments, effects, vacancies) live in
//! `sim-campaign::council`.

use serde::{Deserialize, Serialize};

use crate::common::LocalizedName;
use crate::entities::skill::SkillBranch;
use crate::key_enum;

/// Contents of `data/council/offices.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CouncilOffices {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Loyalty points a dismissed or replaced councillor loses.
    #[serde(default)]
    pub dismissal_loyalty_penalty: u8,
    /// Catalogue order is the order of the automatic assignment.
    pub offices: Vec<CouncilOfficeDef>,
}

impl CouncilOffices {
    /// The office `id`, if defined.
    pub fn office(&self, id: &str) -> Option<&CouncilOfficeDef> {
        self.offices.iter().find(|o| o.id == id)
    }
}

key_enum! {
/// Quantity a councillor's skill changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CouncilEffectKind {
    /// Points added to the acceptance of the faction's treaty proposals.
    DiplomaticAcceptance => "diplomatic_acceptance",
    /// Starting experience of new recruits (`ArmyExperience`, flat).
    RecruitExperience => "recruit_experience",
    /// Morale points of every army general (`ArmyMorale`, flat).
    ArmyMorale => "army_morale",
    /// Percent of the tax income (`TaxIncome`, percent).
    TaxIncomePercent => "tax_income_percent",
    /// Unrest points per province (`Unrest`, flat; negative soothes).
    UnrestPerProvince => "unrest_per_province",
    /// Prestige per season for the ruler (fractions are carried over).
    PrestigePerSeason => "prestige_per_season",
}
}

/// One effect of an office, scaled by the holder's skill.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CouncilEffect {
    pub kind: CouncilEffectKind,
    /// Value per point of the holder's skill (0-10).
    pub per_point: f64,
    /// French wording; `{value}` is replaced by the signed figure.
    pub text: String,
}

/// One office of the council.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CouncilOfficeDef {
    pub id: String,
    pub name: LocalizedName,
    /// Skill of the holder that sets the strength of the effects.
    pub skill: SkillBranch,
    pub effects: Vec<CouncilEffect>,
    /// Alert, hint and journal kinds the office advises on (each kind
    /// belongs to exactly one office).
    pub advice_kinds: Vec<String>,
}
