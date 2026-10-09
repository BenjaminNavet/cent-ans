//! Can the article be executed (ownership, funds, war)? One function per
//! article kind; `index` is the article's place in the treaty.

use data_model::{CharacterId, SettlementKind};

use super::{Article, Deal, MAX_TRIBUTE_SEASONS};
use crate::diplomacy::{DiplomacyError, FEUDAL_TIE_ALLIANCE};

type Check = Result<(), DiplomacyError>;

fn refused(why: &str) -> Check {
    Err(DiplomacyError::Refused(why.to_owned()))
}

impl Article {
    pub(super) fn check(&self, deal: &Deal, index: usize) -> Check {
        let at_war = deal.state.is_at_war(deal.proposer, deal.recipient);
        let ends_war = deal.articles.iter().any(Article::ends_war);
        // Pacts and exchanges need peace, unless the treaty ends the war.
        let needs_peace = at_war && !ends_war;
        match self {
            Article::Peace | Article::Truce { .. } | Article::Mediation { .. } => {
                if at_war {
                    Ok(())
                } else {
                    Err(DiplomacyError::NotAtWar)
                }
            }
            Article::Alliance => check_alliance(deal, needs_peace, true),
            Article::DefensiveAlliance => check_alliance(deal, needs_peace, false),
            Article::NonAggression { turns } => check_non_aggression(deal, needs_peace, *turns),
            Article::JoinWar { target, .. } => check_join_war(self, deal, needs_peace, target),
            Article::MilitaryAccess { .. } | Article::TradeAgreement => {
                check_pact(self, deal, needs_peace)
            }
            Article::Marriage { character, spouse } => {
                check_marriage(deal, index, character, spouse, needs_peace)
            }
            Article::Tribute {
                per_season,
                seasons,
                ..
            } => {
                if *per_season <= 0 || *seasons == 0 || *seasons > MAX_TRIBUTE_SEASONS {
                    return Err(DiplomacyError::InvalidAmount);
                }
                Ok(())
            }
            Article::Gold { amount, .. } => {
                if *amount <= 0 {
                    return Err(DiplomacyError::InvalidAmount);
                }
                let giver = deal.giver(self).expect("giver");
                if deal.state.factions[giver].treasury < *amount {
                    return Err(DiplomacyError::InsufficientFunds);
                }
                Ok(())
            }
            Article::CedeProvince { province, .. } => {
                let giver = deal.giver(self);
                if deal.state.province_owner(province) != giver {
                    return Err(DiplomacyError::InvalidProvince(province.clone()));
                }
                let owned = deal.state.owned_provinces(giver.expect("giver")).len();
                if deal.count_from_same_giver(self, |a| matches!(a, Article::CedeProvince { .. }))
                    >= owned
                {
                    return refused("on ne cède pas sa dernière province");
                }
                Ok(())
            }
            Article::DemandTitle { title, .. } => {
                let giver = deal.giver(self).expect("giver");
                if crate::feudal::holder_of(deal.state, title) != Some(giver) {
                    return Err(DiplomacyError::Refused(format!(
                        "{} ne détient pas ce titre",
                        deal.data.faction_name(giver)
                    )));
                }
                let held = crate::feudal::titles_of(deal.state, giver).len();
                if deal.count_from_same_giver(self, |a| matches!(a, Article::DemandTitle { .. }))
                    >= held
                {
                    return refused("on ne cède pas son dernier titre");
                }
                Ok(())
            }
            Article::CedeSettlement { settlement, .. } => {
                let Some(s) = deal.state.settlements.get(settlement) else {
                    return refused("place inconnue");
                };
                if Some(&s.owner) != deal.giver(self) || s.kind == SettlementKind::City {
                    return Err(DiplomacyError::Refused(format!(
                        "{} ne peut pas être cédée seule",
                        deal.data.settlement_name(settlement)
                    )));
                }
                Ok(())
            }
            Article::Vassalage { .. } => {
                let (vassal, lord) = (
                    deal.giver(self).expect("giver"),
                    deal.taker(self).expect("taker"),
                );
                if deal.state.factions[vassal].suzerain.is_some()
                    || deal.state.factions[lord].suzerain.as_ref() == Some(vassal)
                {
                    return refused("déjà lié par la vassalité");
                }
                Ok(())
            }
            Article::ReleaseCaptive { character, .. } => {
                let (giver, taker) = (deal.giver(self), deal.taker(self));
                let valid = deal.state.characters.get(character).is_some_and(|c| {
                    c.alive && c.captive && c.captor.as_ref() == giver && Some(&c.faction) == taker
                });
                if valid {
                    Ok(())
                } else {
                    refused("captif invalide")
                }
            }
            Article::Hostage { character, .. } => {
                let giver = deal.giver(self).expect("giver");
                let valid = deal.state.characters.get(character).is_some_and(|c| {
                    c.alive && !c.captive && &c.faction == giver && c.army.is_none()
                }) && deal.state.factions[giver].ruler.as_ref() != Some(character);
                if valid {
                    Ok(())
                } else {
                    refused("otage invalide")
                }
            }
            // Orders of a lord, created by the core, never proposed.
            Article::Obedience { .. }
            | Article::Protection { .. }
            | Article::Arbitration { .. }
            | Article::PeaceSummons { .. }
            | Article::AllyCall { .. } => refused("un appel féodal ne se propose pas"),
        }
    }
}

fn check_alliance(deal: &Deal, needs_peace: bool, military: bool) -> Check {
    // A defensive alliance can be upgraded to a military one.
    let upgrade = military
        && deal
            .state
            .is_defensive_alliance(deal.proposer, deal.recipient);
    if deal.state.is_allied(deal.proposer, deal.recipient) && !upgrade {
        return Err(DiplomacyError::AlreadyAllied);
    }
    if crate::feudal::direct_tie(deal.state, deal.data, deal.proposer, deal.recipient) {
        return refused(FEUDAL_TIE_ALLIANCE);
    }
    if needs_peace {
        return Err(DiplomacyError::AlreadyAtWar);
    }
    Ok(())
}

/// Military access and trade agreements: not already in force.
fn check_pact(article: &Article, deal: &Deal, needs_peace: bool) -> Check {
    if needs_peace {
        return Err(DiplomacyError::AlreadyAtWar);
    }
    let state = deal.state;
    let already = match article {
        Article::MilitaryAccess { .. } => {
            let (giver, taker) = (deal.giver(article), deal.taker(article));
            state.factions[giver.expect("giver")]
                .ledger
                .military_access
                .contains(taker.expect("taker"))
        }
        _ => state.factions[deal.proposer]
            .ledger
            .trade_agreements
            .contains(deal.recipient),
    };
    if already {
        return refused("accord déjà en vigueur");
    }
    Ok(())
}

fn check_marriage(
    deal: &Deal,
    index: usize,
    character: &CharacterId,
    spouse: &CharacterId,
    needs_peace: bool,
) -> Check {
    if needs_peace {
        return Err(DiplomacyError::AlreadyAtWar);
    }
    let own = |c: &CharacterId, f: &data_model::FactionId| {
        deal.state
            .characters
            .get(c)
            .is_some_and(|c| c.alive && &c.faction == f)
    };
    if !own(character, deal.proposer) || !own(spouse, deal.recipient) {
        return refused("époux invalides");
    }
    // Checked here so that the treaty never applies by halves.
    crate::dynasty::check_marriage(deal.state, character, spouse)
        .map_err(|e| DiplomacyError::Refused(e.to_string()))?;
    let wed_twice = deal.articles[..index].iter().any(|other| {
        matches!(other, Article::Marriage { character: c, spouse: s }
            if [c, s].iter().any(|x| *x == character || *x == spouse))
    });
    if wed_twice {
        return refused("un même époux dans deux mariages");
    }
    Ok(())
}

/// Non-aggression pact (WH `diplob`): between parties at peace and not yet
/// bound, for a duration within `non_aggression.min_turns..=max_turns`.
fn check_non_aggression(deal: &Deal, needs_peace: bool, turns: u32) -> Check {
    let rules = &deal.data.ai_diplomacy.non_aggression;
    if turns < rules.min_turns || turns > rules.max_turns {
        return Err(DiplomacyError::InvalidAmount);
    }
    if needs_peace {
        return Err(DiplomacyError::AlreadyAtWar);
    }
    if deal.state.is_allied(deal.proposer, deal.recipient) {
        return refused("déjà liés par une alliance ou la vassalité");
    }
    if deal.state.has_non_aggression(deal.proposer, deal.recipient) {
        return refused("pacte déjà en vigueur");
    }
    Ok(())
}

/// « Rejoindre la guerre contre X » (WH `diplob`): the giver goes to war
/// against an enemy of the other party, as a declaration would allow.
fn check_join_war(
    article: &Article,
    deal: &Deal,
    needs_peace: bool,
    target: &data_model::FactionId,
) -> Check {
    if needs_peace {
        return Err(DiplomacyError::AlreadyAtWar);
    }
    let (state, data) = (deal.state, deal.data);
    let giver = deal.giver(article).expect("giver");
    let taker = deal.taker(article).expect("taker");
    let name = |f: &data_model::FactionId| data.faction_name(f);
    if !state.factions.get(target).is_some_and(|f| f.alive)
        || target.is_rebels()
        || target.as_str() == crate::diplomacy::PAPACY_FACTION
    {
        return refused("cible inconnue ou disparue");
    }
    if target == giver || target == taker {
        return refused("on ne rejoint pas une guerre contre l'un des signataires");
    }
    if !state.is_at_war(taker, target) {
        return Err(DiplomacyError::Refused(format!(
            "{} n'est pas en guerre contre {}",
            name(taker),
            name(target)
        )));
    }
    if state.is_at_war(giver, target) {
        return Err(DiplomacyError::Refused(format!(
            "{} est déjà en guerre contre {}",
            name(giver),
            name(target)
        )));
    }
    if state.is_allied(giver, target) {
        return Err(DiplomacyError::Refused(format!(
            "{} est lié à {} par une alliance ou la vassalité",
            name(giver),
            name(target)
        )));
    }
    if state.has_truce(giver, target) || state.has_non_aggression(giver, target) {
        return Err(DiplomacyError::Refused(format!(
            "{} a donné sa parole à {} (trêve ou non-agression)",
            name(giver),
            name(target)
        )));
    }
    if state.factions[giver].treasury < 0 {
        return Err(DiplomacyError::Refused(format!(
            "{} n'a plus de quoi partir en guerre (trésor vide)",
            name(giver)
        )));
    }
    Ok(())
}
