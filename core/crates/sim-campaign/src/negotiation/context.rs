//! General considerations of the recipient about a treaty, whatever its
//! articles: attitude, trust, threat, honour, war weariness, pretensions.

use super::{bastion_stalemate, Article, Deal, Party, ReasonList, HOSTAGE_BETRAYAL_REASON};
use crate::diplomacy;

/// The recipient's general considerations about the treaty.
pub(super) fn context_reasons(deal: &Deal) -> ReasonList {
    let mut reasons = ReasonList::new();
    let ends_war = deal.articles.iter().any(Article::ends_war);
    attitude(deal, ends_war, &mut reasons);
    trust(deal, &mut reasons);
    threat(deal, &mut reasons);
    if ends_war {
        honour(deal, &mut reasons);
        if deal.rules().enabled {
            war_weariness(deal, &mut reasons);
            interminable_war(deal, &mut reasons);
            bastions(deal, &mut reasons);
            pretender(deal, &mut reasons);
            war_goals(deal, &mut reasons);
        }
    }
    reasons.push("Prudence", deal.weights().context.prudence);
    reasons
}

/// A peace is judged on the war, not on the hatred the war feeds.
fn attitude(deal: &Deal, ends_war: bool, reasons: &mut ReasonList) {
    let weights = &deal.weights().context;
    let (attitude, _) = deal
        .state
        .attitude(deal.data, deal.recipient, deal.proposer);
    let divisor = if ends_war {
        weights.attitude_divisor_peace
    } else {
        weights.attitude_divisor
    };
    reasons.push("Attitude", attitude / divisor);
}

/// The proposer's word: perjuries, hostages, marriages, treaties.
fn trust(deal: &Deal, reasons: &mut ReasonList) {
    let (state, proposer, recipient) = (deal.state, deal.proposer, deal.recipient);
    let weights = &deal.weights().context;
    let rec = &state.factions[recipient];
    let perjuries = rec
        .modifiers
        .iter()
        .filter(|m| {
            &m.with == proposer
                && m.expires_turn > state.turn
                && (m.reason_fr == diplomacy::PERJURY_REASON
                    || m.reason_fr == HOSTAGE_BETRAYAL_REASON)
        })
        .count() as i32;
    let hostages = rec
        .ledger
        .hostages
        .iter()
        .filter(|h| &h.from == proposer)
        .count() as i32;
    let pacts = i32::from(rec.ledger.trade_agreements.contains(proposer))
        + i32::from(state.is_allied(recipient, proposer));
    let trust = (weights.perjury * perjuries
        + weights.hostage_held * hostages
        + weights.marriage_tie * i32::from(state.marriage_tie(recipient, proposer))
        + weights.standing_pact * pacts)
        .clamp(weights.trust_min, weights.trust_max);
    reasons.push("Confiance", trust);
}

/// A much stronger neighbour obtains concessions more easily.
fn threat(deal: &Deal, reasons: &mut ReasonList) {
    let (state, data, proposer, recipient) = (deal.state, deal.data, deal.proposer, deal.recipient);
    let weights = &deal.weights().context;
    let gives = deal.articles.iter().any(|a| {
        a.giver() == Some(Party::Recipient)
            || (a.ends_war() && state.war_score(data, recipient, proposer) < 0)
    });
    if !gives {
        return;
    }
    let ratio = state.faction_power(proposer) / state.faction_power(recipient).max(1.0);
    if ratio > weights.threat_ratio && state.are_neighbors(data, recipient, proposer) {
        reasons.push(
            "Menace de sa puissance",
            ((ratio - 1.0) * weights.threat_scale)
                .round()
                .min(f64::from(weights.threat_cap)) as i32,
        );
    } else if ratio < weights.weak_demander_ratio {
        reasons.push("Faiblesse du demandeur", weights.weak_demander);
    }
}

/// A separate peace abandons the allies still fighting.
fn honour(deal: &Deal, reasons: &mut ReasonList) {
    let weights = &deal.weights().context;
    let abandoned = deal.state.factions[deal.recipient]
        .allies
        .iter()
        .filter(|a| deal.state.is_at_war(a, deal.proposer))
        .count() as i32;
    reasons.push_if(
        abandoned > 0,
        "Honneur : ne pas abandonner nos alliés",
        weights.abandoned_ally * abandoned.min(weights.abandoned_allies_max),
    );
}

fn war_weariness(deal: &Deal, reasons: &mut ReasonList) {
    let weariness = deal.state.factions[deal.recipient].ledger.weariness;
    reasons.push(
        "Fatigue de guerre",
        (weariness / deal.rules().weariness_peace_divisor.max(1)) as i32,
    );
}

/// EQ6: a war nobody wins outright ends in a truce, the winner weary of it
/// too.
fn interminable_war(deal: &Deal, reasons: &mut ReasonList) {
    let rules = deal.rules();
    if rules.long_war_years == 0 {
        return;
    }
    let years = deal.state.factions[deal.recipient]
        .war_started
        .get(deal.proposer)
        .map_or(0, |started| deal.state.turn.saturating_sub(*started) / 4);
    let beyond = years.saturating_sub(rules.long_war_years) as i32;
    reasons.push(
        "Guerre interminable",
        (beyond * rules.long_war_points_per_year).min(rules.long_war_max_points),
    );
}

/// LR-11: a war against a realm down to its last bastions, which the other
/// side never besieges without a claim (F4), has nothing left to fight for
/// once the campaign seasons are over.
fn bastions(deal: &Deal, reasons: &mut ReasonList) {
    let rules = deal.rules();
    let lasted = deal.state.factions[deal.recipient]
        .war_started
        .get(deal.proposer)
        .is_some_and(|started| deal.state.turn >= started + rules.min_war_turns);
    if rules.bastion_war_points > 0
        && lasted
        && bastion_stalemate(deal.state, deal.data, deal.recipient, deal.proposer)
    {
        reasons.push(
            "Guerre sans enjeu : derniers bastions",
            rules.bastion_war_points,
        );
    }
}

/// A pretender does not give up a crown for nothing.
fn pretender(deal: &Deal, reasons: &mut ReasonList) {
    let (state, data) = (deal.state, deal.data);
    let gains_land = deal.articles.iter().any(|a| {
        matches!(
            a,
            Article::CedeProvince {
                giver: Party::Proposer,
                ..
            }
        )
    });
    if diplomacy::claim_stakes(state, deal.recipient, deal.proposer).throne
        && !gains_land
        && state.war_score(data, deal.recipient, deal.proposer) > -40
    {
        reasons.push(
            "Prétention à la couronne",
            -deal.rules().pretender_reluctance,
        );
    }
}

/// A belligerent not beaten keeps fighting for its war goals.
fn war_goals(deal: &Deal, reasons: &mut ReasonList) {
    let (state, data) = (deal.state, deal.data);
    let rules = deal.rules();
    let Some(goals) = state.factions[deal.recipient]
        .ledger
        .war_goals
        .get(deal.proposer)
    else {
        return;
    };
    let obtained = deal.articles.iter().any(|a| match a {
        Article::CedeProvince {
            giver: Party::Proposer,
            province,
        } => goals.contains(province),
        _ => false,
    });
    let held = goals
        .iter()
        .any(|p| state.controls_province(deal.recipient, p));
    if !goals.is_empty() && !obtained && state.war_score(data, deal.recipient, deal.proposer) > -30
    {
        let value = if held {
            rules.unmet_goals_reluctance
        } else {
            rules.unmet_goals_reluctance / 2
        };
        reasons.push("Buts de guerre non atteints", -value);
    }
}
