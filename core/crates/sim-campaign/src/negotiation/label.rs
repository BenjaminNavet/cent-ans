//! French wording of an article: its label in a treaty, and the sentence of
//! an offer made of this article alone.

use data_model::{FactionId, GameData};

use super::{party_id, Article};
use crate::diplomacy;
use crate::state::CampaignState;

impl Article {
    /// French label of the article from the proposer's point of view.
    pub fn label(
        &self,
        state: &CampaignState,
        data: &GameData,
        proposer: &FactionId,
        recipient: &FactionId,
    ) -> String {
        let who = |p: &super::Party| data.faction_name(party_id(*p, proposer, recipient));
        let to = |p: &super::Party| data.faction_name(party_id(p.other(), proposer, recipient));
        let rules = &data.ai_diplomacy.negotiation;
        match self {
            Article::Peace => format!(
                "Paix (trêve de {} ans)",
                (if rules.enabled {
                    rules.peace_truce_turns
                } else {
                    diplomacy::TRUCE_TURNS
                } / 4)
                    .max(1)
            ),
            Article::Truce { turns } => format!("Trêve de {} an(s)", (turns / 4).max(1)),
            Article::Mediation { turns } => {
                format!("Trêve de {} an(s) par médiation", (turns / 4).max(1))
            }
            Article::Alliance => "Alliance défensive et offensive".to_owned(),
            Article::MilitaryAccess { giver } => {
                format!("Accès militaire : {} ouvre ses terres", who(giver))
            }
            Article::TradeAgreement => "Accord commercial".to_owned(),
            Article::Marriage { character, spouse } => format!(
                "Mariage de {} et de {}",
                state.character_name(data, character),
                state.character_name(data, spouse)
            ),
            Article::Tribute {
                giver,
                per_season,
                seasons,
            } => format!(
                "Tribut : {} verse {per_season} livres par saison pendant {seasons} saisons",
                who(giver)
            ),
            Article::Gold { giver, amount } => format!("{} verse {amount} livres", who(giver)),
            Article::CedeProvince { giver, province } => format!(
                "{} cède {} à {}",
                who(giver),
                data.province_name(province),
                to(giver)
            ),
            Article::CedeSettlement { giver, settlement } => format!(
                "{} cède la place de {} à {}",
                who(giver),
                data.settlement_name(settlement),
                to(giver)
            ),
            Article::Vassalage { giver } => {
                format!("{} devient vassal de {}", who(giver), to(giver))
            }
            Article::ReleaseCaptive { giver, character } => format!(
                "{} libère {}",
                who(giver),
                state.character_name(data, character)
            ),
            Article::Hostage { giver, character } => format!(
                "{} livre {} en otage",
                who(giver),
                state.character_name(data, character)
            ),
            Article::DemandTitle { giver, title } => format!(
                "{} remet le titre {} à {}",
                who(giver),
                data.titles
                    .get(title)
                    .map_or_else(|| title.to_string(), |t| t.name.display.clone()),
                to(giver)
            ),
            Article::Obedience { religion } => format!(
                "Grand Schisme : rejoindre {}",
                data.religions
                    .get(religion)
                    .map_or_else(|| religion.to_string(), |r| r.name.display.clone())
            ),
            Article::Protection { aggressor } => format!(
                "Appel à la protection contre {}",
                data.faction_name(aggressor)
            ),
            Article::Arbitration { attacker, target } => format!(
                "Arbitrage : {} attaque {}",
                data.faction_name(attacker),
                data.faction_name(target)
            ),
            Article::PeaceSummons { target } => {
                format!(
                    "Sommation de faire la paix avec {}",
                    data.faction_name(target)
                )
            }
        }
    }

    /// Sentence of an offer from `from` made of this article alone.
    pub(crate) fn offer_line(
        &self,
        state: &CampaignState,
        data: &GameData,
        from: &FactionId,
    ) -> Option<String> {
        let name = data.faction_name(from);
        Some(match self {
            Article::Peace => format!("{name} propose la paix."),
            Article::Truce { turns } | Article::Mediation { turns } => format!(
                "Par la médiation du pape, {name} propose une trêve de {} ans.",
                turns / 4
            ),
            Article::Alliance => format!("{name} propose une alliance."),
            Article::Vassalage {
                giver: super::Party::Recipient,
            } => format!("{name} exige que vous deveniez son vassal."),
            Article::Marriage { character, spouse } => format!(
                "{name} propose le mariage de {} et de {}.",
                state.character_name(data, character),
                state.character_name(data, spouse)
            ),
            Article::Obedience { religion } => format!(
                "Grand Schisme : rejoindre {} ?",
                data.religions
                    .get(religion)
                    .map_or_else(|| religion.to_string(), |r| r.name.display.clone())
            ),
            Article::Protection { aggressor } => format!(
                "{name} est attaqué par {} et réclame votre protection.",
                data.faction_name(aggressor)
            ),
            Article::Arbitration { attacker, target } => format!(
                "Guerre privée : {} attaque {}, tous deux vos vassaux.",
                data.faction_name(attacker),
                data.faction_name(target)
            ),
            Article::PeaceSummons { target } => format!(
                "{name} vous somme de faire la paix avec {}.",
                data.faction_name(target)
            ),
            _ => return None,
        })
    }
}
