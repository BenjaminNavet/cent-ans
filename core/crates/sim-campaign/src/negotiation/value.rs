//! How the recipient values each article: one function per article kind,
//! each built from small functions per factor. Weights come from
//! `data/ai/diplomacy.json` (`treaty_weights`, `negotiation`).

use data_model::{CharacterId, ClaimKind, ProvinceId, SettlementId};

use super::{Article, ArticleValue, Deal, Party, ReasonList};
use crate::diplomacy;

impl Article {
    /// Value of the article for the recipient, with the reasons behind it.
    pub(super) fn value(&self, deal: &Deal) -> ArticleValue {
        let mut reasons = ReasonList::new();
        let mut blocked: Option<String> = None;
        let gives = self.giver() == Some(Party::Recipient);
        match self {
            Article::Peace => {
                peace_factors(deal, &mut reasons);
                blocked = crusade_vow(deal);
            }
            Article::Truce { turns } => {
                peace_factors(deal, &mut reasons);
                reasons.push_if(
                    *turns < diplomacy::TRUCE_TURNS,
                    "Trêve courte, sans engagement",
                    deal.weights().peace.short_truce,
                );
                blocked = crusade_vow(deal);
            }
            Article::Mediation { .. } => {
                peace_factors(deal, &mut reasons);
                reasons.push("Médiation pontificale", deal.weights().peace.mediation);
                blocked = crusade_vow(deal);
            }
            Article::Alliance => alliance_factors(deal, &mut reasons),
            Article::Marriage { character, spouse } => {
                marriage_factors(deal, character, spouse, &mut reasons);
            }
            Article::Vassalage { giver } => {
                blocked = vassalage_factors(deal, *giver, &mut reasons);
            }
            Article::MilitaryAccess { giver } => access_factors(deal, *giver, &mut reasons),
            Article::TradeAgreement => trade_factors(deal, &mut reasons),
            Article::Tribute {
                per_season,
                seasons,
                ..
            } => tribute_factors(deal, gives, *per_season, *seasons, &mut reasons),
            Article::Gold { amount, .. } => gold_factors(deal, gives, *amount, &mut reasons),
            Article::CedeProvince { province, .. } => {
                blocked = province_factors(deal, gives, province, &mut reasons);
            }
            Article::CedeSettlement { settlement, .. } => {
                settlement_factors(deal, gives, settlement, &mut reasons);
            }
            Article::DemandTitle { title, .. } => title_factors(deal, gives, title, &mut reasons),
            Article::ReleaseCaptive { character, .. } => {
                captive_factors(deal, gives, character, &mut reasons);
            }
            Article::Hostage { character, .. } => {
                hostage_factors(deal, gives, character, &mut reasons);
            }
            // Orders of a lord, not bargains: nothing to weigh.
            Article::Obedience { .. }
            | Article::Protection { .. }
            | Article::Arbitration { .. }
            | Article::PeaceSummons { .. } => {}
        }
        ArticleValue {
            label: self.label(deal.state, deal.data, deal.proposer, deal.recipient),
            value: reasons.total(),
            reasons,
            blocked,
        }
    }
}

// ----- peace, truce, mediation ------------------------------------------

/// The war as the recipient sees it (the attitude is counted once, in the
/// context; the turn-based lassitude gives way to the war weariness).
fn peace_factors(deal: &Deal, reasons: &mut ReasonList) {
    let (state, data) = (deal.state, deal.data);
    let (proposer, recipient) = (deal.proposer, deal.recipient);
    let weights = &deal.weights().peace;
    let score = state.war_score(data, recipient, proposer);
    reasons.push("Score de guerre", -score);
    if !deal.rules().enabled {
        let started = state
            .factions
            .get(recipient)
            .and_then(|f| f.war_started.get(proposer))
            .copied()
            .unwrap_or(state.turn);
        let lassitude = (state.turn.saturating_sub(started) / weights.lassitude_seasons_per_point)
            .min(weights.lassitude_cap);
        reasons.push("Lassitude de la guerre", lassitude as i32);
    }
    let broke = state
        .factions
        .get(recipient)
        .is_some_and(|f| f.treasury < 0);
    reasons.push_if(broke, "Trésor vide", weights.empty_treasury);
    let ravaged = state
        .provinces
        .iter()
        .filter(|(id, p)| {
            state.province_owner(id) == Some(recipient)
                && p.devastation > weights.devastation_threshold
        })
        .count() as i32;
    reasons.push(
        "Provinces ravagées",
        (ravaged * weights.per_ravaged_province).min(weights.ravaged_cap),
    );
    let aggression = data
        .factions
        .get(recipient)
        .and_then(|f| f.ai_personality.as_ref())
        .and_then(|p| p.aggression)
        .map_or(50, i32::from);
    reasons.push(
        "Tempérament belliqueux",
        -(aggression - 50) / weights.aggression_divisor,
    );
    reasons.push("Attrait de la victoire", -weights.victory_lure);
    // A stronger enemy on another front calls for peace here.
    let other_front = state.factions.get(recipient).is_some_and(|f| {
        f.at_war_with.iter().any(|e| {
            e != proposer
                && !e.is_rebels()
                && deal.cache.faction_power(e) > deal.cache.faction_power(proposer)
        })
    });
    reasons.push_if(
        other_front,
        "Guerre sur un autre front",
        weights.other_front,
    );
    // A pretender does not give up its claim lightly.
    if score > weights.claim_score_floor {
        let stakes = diplomacy::claim_stakes(state, recipient, proposer);
        if stakes.throne {
            reasons.push("Prétention au trône", -weights.pretender_reluctance);
        } else if stakes.provinces > 0 {
            reasons.push("Provinces revendiquées", -weights.claimed_provinces);
        }
    }
    recognised_conquests(deal, score, reasons);
}

/// A beaten crown that keeps its capital (or its last land) while the
/// recipient holds all it lost settles the war score (F4, G5).
fn recognised_conquests(deal: &Deal, score_for_recipient: i32, reasons: &mut ReasonList) {
    let (state, data) = (deal.state, deal.data);
    if !data.ai_diplomacy.peace.keep_capital || score_for_recipient <= 0 {
        return;
    }
    let conquests: Vec<&ProvinceId> = state
        .provinces
        .keys()
        .filter(|id| {
            state.province_owner(id) == Some(deal.proposer)
                && state.controls_province(deal.recipient, id)
        })
        .collect();
    let [only] = conquests.as_slice() else {
        return;
    };
    let capital = state.factions.get(deal.proposer).map(|f| &f.capital);
    let landless = !state
        .provinces
        .keys()
        .any(|id| state.controls_province(deal.proposer, id));
    if capital == Some(*only) || landless {
        reasons.push("Conquêtes reconnues", score_for_recipient);
    }
}

/// JR4: the vow of an AI-led crusade allows no peace with the master of its
/// goal.
fn crusade_vow(deal: &Deal) -> Option<String> {
    crate::crusade::ai_vow_forbids_peace(deal.state, deal.data, deal.recipient, deal.proposer)
        .then(|| "le vœu de croisade interdit la paix".to_owned())
}

// ----- alliance, vassalage, marriage -------------------------------------

fn alliance_factors(deal: &Deal, reasons: &mut ReasonList) {
    let (state, data) = (deal.state, deal.data);
    let (proposer, recipient) = (deal.proposer, deal.recipient);
    let weights = &deal.weights().alliance;
    reasons.push("Engagement militaire", weights.military_commitment);
    let at_war_with_our_allies = state
        .factions
        .get(recipient)
        .is_some_and(|f| f.allies.iter().any(|a| state.is_at_war(proposer, a)));
    reasons.push_if(
        at_war_with_our_allies,
        "En guerre contre nos alliés",
        weights.war_on_our_allies,
    );
    let common_enemy = state.factions.get(proposer).is_some_and(|f| {
        f.at_war_with
            .iter()
            .any(|e| !e.is_rebels() && state.is_at_war(recipient, e))
    });
    reasons.push_if(common_enemy, "Ennemi commun", weights.common_enemy);
    let ratio = deal.cache.faction_power(proposer) / deal.cache.faction_power(recipient).max(1.0);
    if ratio > weights.strong_ratio {
        reasons.push("Allié puissant", weights.strong_ally);
    } else if ratio < weights.weak_ratio {
        reasons.push("Allié faible", weights.weak_ally);
    }
    // Shared rivals, counterweight against a menacing neighbour, and no
    // alliance with a rival's friend.
    let proposer_rivals = deal.cache.rivals(proposer);
    let recipient_rivals = deal.cache.rivals(recipient);
    reasons.push_if(
        !proposer_rivals.is_disjoint(&recipient_rivals),
        "Rival commun",
        weights.common_rival,
    );
    let menace = &data.ai_diplomacy.menacing_neighbour;
    let menaced = proposer_rivals.iter().any(|r| {
        deal.cache.faction_power(r)
            > menace.power_ratio * deal.cache.faction_power(recipient).max(1.0)
            && deal.cache.are_neighbors(data, recipient, r)
    });
    reasons.push_if(
        menaced,
        "Contrepoids à un voisin menaçant",
        menace.counterweight,
    );
    let friend_of_rival = state
        .factions
        .get(proposer)
        .is_some_and(|f| f.allies.iter().any(|a| recipient_rivals.contains(a)));
    reasons.push_if(
        friend_of_rival,
        "Allié de nos rivaux",
        weights.friend_of_rival,
    );
}

/// `giver` becomes the other party's vassal. Returns why it is out of the
/// question, if so.
fn vassalage_factors(deal: &Deal, giver: Party, reasons: &mut ReasonList) -> Option<String> {
    let (state, data) = (deal.state, deal.data);
    let (proposer, recipient) = (deal.proposer, deal.recipient);
    let weights = &deal.weights().vassalage;
    if giver == Party::Proposer {
        reasons.push("Hommage d'un nouveau vassal", weights.homage_gain);
        let income = state.factions[proposer].last_budget.income.max(0);
        reasons.push(
            "Tribut du vassal",
            deal.gold_points(income * data.feudal_rules.vassal_tribute_percent / 100 * 20),
        );
        return None;
    }
    let ratio = deal.cache.faction_power(proposer) / deal.cache.faction_power(recipient).max(1.0);
    let required = data.feudal_rules.vassalage_power_ratio;
    let mut blocked = None;
    if ratio < required {
        blocked = Some("vassalité refusée : pas assez puissant".to_owned());
    } else {
        reasons.push(
            "Rapport de forces",
            (((ratio - required) * weights.power_margin_scale) as i32)
                .min(weights.power_margin_cap),
        );
    }
    reasons.push("Perte d'indépendance", weights.independence_loss);
    let beaten = state.is_at_war(proposer, recipient)
        && state.war_score(data, recipient, proposer) < weights.defeat_score;
    reasons.push_if(beaten, "Défaite militaire", weights.defeat);
    blocked
}

fn marriage_factors(
    deal: &Deal,
    character: &CharacterId,
    spouse: &CharacterId,
    reasons: &mut ReasonList,
) {
    let weights = &deal.weights().marriage;
    reasons.push("Alliance matrimoniale", weights.match_bonus);
    let rank = |id: &CharacterId| deal.state.characters.get(id).map_or(0, |c| c.prestige);
    reasons.push(
        "Prestige du prétendant",
        ((rank(character) - rank(spouse)) / weights.prestige_divisor)
            .clamp(-weights.prestige_cap, weights.prestige_cap),
    );
}

// ----- access, trade, money ----------------------------------------------

fn access_factors(deal: &Deal, giver: Party, reasons: &mut ReasonList) {
    let (state, proposer, recipient) = (deal.state, deal.proposer, deal.recipient);
    let weights = &deal.weights().exchange;
    if giver == Party::Proposer {
        reasons.push("Libre passage de nos armées", weights.access_received);
        return;
    }
    reasons.push("Passage d'armées étrangères", weights.access_given);
    reasons.push_if(
        deal.cache.rivals(recipient).contains(proposer),
        "Armées d'un rival",
        weights.access_rival,
    );
    reasons.push_if(
        state.is_allied(recipient, proposer),
        "Entre alliés",
        weights.access_allies,
    );
}

fn trade_factors(deal: &Deal, reasons: &mut ReasonList) {
    let (state, data) = (deal.state, deal.data);
    let (proposer, recipient) = (deal.proposer, deal.recipient);
    let weights = &deal.weights().exchange;
    reasons.push("Commerce", weights.trade_base);
    // C5: the agreement raises the routes linking our marketplaces.
    let routes = crate::trade::common_routes(state, data, proposer, recipient) as i32;
    reasons.push(
        "Routes commerciales communes",
        (weights.trade_per_route * routes).min(weights.trade_routes_cap),
    );
    let partner = state.factions[proposer].last_budget.income.max(0);
    reasons.push(
        "Richesse du partenaire",
        ((partner / weights.trade_wealth_divisor) as i32).min(weights.trade_wealth_cap),
    );
    reasons.push_if(
        deal.cache.rivals(recipient).contains(proposer),
        "Enrichir un rival",
        weights.trade_rival,
    );
    let embargo = state.factions[proposer].embargoes.contains(recipient)
        || state.factions[recipient].embargoes.contains(proposer);
    reasons.push_if(embargo, "Embargo en cours", weights.trade_embargo);
}

fn tribute_factors(
    deal: &Deal,
    gives: bool,
    per_season: i64,
    seasons: u32,
    reasons: &mut ReasonList,
) {
    let weights = &deal.weights().exchange;
    let total = per_season.saturating_mul(i64::from(seasons));
    let points = deal.gold_points(total * weights.tribute_percent / 100);
    if gives {
        reasons.push("Tribut à verser", -points);
        let income = deal.state.factions[deal.recipient]
            .last_budget
            .income
            .max(1);
        reasons.push_if(
            per_season * 2 > income,
            "Tribut ruineux",
            weights.tribute_ruinous,
        );
    } else {
        reasons.push("Tribut reçu", points);
    }
}

fn gold_factors(deal: &Deal, gives: bool, amount: i64, reasons: &mut ReasonList) {
    let weights = &deal.weights().exchange;
    let mut points = deal.gold_points(amount);
    if gives {
        reasons.push("Or à verser", -points);
        return;
    }
    if deal.state.factions[deal.recipient].treasury < 0 {
        points = points * weights.gold_empty_percent / 100;
        reasons.push("Trésor vide", weights.gold_empty_treasury);
    }
    reasons.push("Or reçu", points);
}

// ----- lands and titles ---------------------------------------------------

/// A province changes hands. Returns why the recipient will never give it.
fn province_factors(
    deal: &Deal,
    gives: bool,
    province: &ProvinceId,
    reasons: &mut ReasonList,
) -> Option<String> {
    let (state, data) = (deal.state, deal.data);
    let (proposer, recipient) = (deal.proposer, deal.recipient);
    let rules = deal.rules();
    let weights = &deal.weights().exchange;
    if !gives {
        reasons.push(
            "Gain d'une province",
            rules.province_cost * weights.gain_percent / 100,
        );
        let wanted = super::is_war_goal(state, recipient, proposer, province)
            || diplomacy::claimed_provinces(state, recipient).contains(province);
        reasons.push_if(wanted, "Terre convoitée", rules.war_goal_bonus);
        reasons.push_if(
            state.controls_province(recipient, province),
            "Déjà tenue par nos troupes",
            weights.held_by_our_troops,
        );
        return None;
    }
    let capital = state.factions[recipient].capital == *province;
    let mut cost = if capital {
        rules.capital_cost
    } else {
        rules.province_cost
    };
    let blocked = (data.ai_diplomacy.peace.keep_capital && capital).then(|| {
        format!(
            "{} ne cédera jamais sa capitale",
            data.faction_name(recipient)
        )
    });
    let occupied = state.controls_province(proposer, province);
    if occupied {
        cost = cost * rules.occupied_cost_percent / 100;
    }
    reasons.push(
        if occupied {
            "Province déjà occupée par l'ennemi"
        } else {
            "Perte d'une province"
        },
        -cost,
    );
    let claimed = state.factions[recipient]
        .claims
        .iter()
        .any(|c| c.province.as_ref() == Some(province) && c.kind == ClaimKind::Province);
    reasons.push_if(
        claimed,
        "Terre revendiquée de longue date",
        weights.claimed_land,
    );
    blocked
}

fn settlement_factors(
    deal: &Deal,
    gives: bool,
    settlement: &SettlementId,
    reasons: &mut ReasonList,
) {
    let rules = deal.rules();
    let taker = if gives { deal.proposer } else { deal.recipient };
    let occupied = deal
        .state
        .settlements
        .get(settlement)
        .is_some_and(|s| &s.controller == taker);
    let mut cost = rules.settlement_cost;
    if occupied {
        cost = cost * rules.occupied_cost_percent / 100;
    }
    let cost = cost.max(1);
    if gives {
        reasons.push("Perte d'une place", -cost);
    } else {
        reasons.push("Gain d'une place", cost);
    }
}

/// The title's own provinces held by the giver, plus its rank.
fn title_factors(deal: &Deal, gives: bool, title: &data_model::TitleId, reasons: &mut ReasonList) {
    let (state, data) = (deal.state, deal.data);
    let rules = deal.rules();
    let giver = if gives { deal.recipient } else { deal.proposer };
    let capital = &state.factions[giver].capital;
    let provinces: i32 = data
        .titles
        .get(title)
        .map(|t| {
            t.de_jure_provinces
                .iter()
                .filter(|p| state.province_owner(p) == Some(giver))
                .map(|p| {
                    if p == capital {
                        rules.capital_cost
                    } else {
                        rules.province_cost
                    }
                })
                .sum()
        })
        .unwrap_or(0);
    let value = provinces + data.feudal_rules.title_loss_penalty;
    if gives {
        reasons.push("Perte d'un titre", -value);
    } else {
        reasons.push(
            "Gain d'un titre",
            value * deal.weights().exchange.gain_percent / 100,
        );
    }
}

// ----- captives and hostages ----------------------------------------------

fn captive_factors(deal: &Deal, gives: bool, character: &CharacterId, reasons: &mut ReasonList) {
    let (state, data) = (deal.state, deal.data);
    let weights = &deal.weights().exchange;
    let ransom = crate::ransom::ransom_amount(state, data, character);
    let high_rank = state
        .factions
        .values()
        .any(|f| f.ruler.as_ref() == Some(character) || f.heir.as_ref() == Some(character));
    let mut points = ((ransom / deal.rules().livres_per_point.max(1)) as i32)
        .clamp(weights.ransom_min, weights.ransom_max);
    if high_rank {
        points += weights.ransom_high_rank;
    }
    if gives {
        reasons.push("Libérer un captif sans rançon", -points);
    } else {
        reasons.push("Retour d'un des nôtres", points);
    }
}

fn hostage_factors(deal: &Deal, gives: bool, character: &CharacterId, reasons: &mut ReasonList) {
    let weights = &deal.weights().exchange;
    let heir = deal
        .state
        .factions
        .values()
        .any(|f| f.heir.as_ref() == Some(character));
    if gives {
        reasons.push(
            "Livrer un otage",
            -if heir {
                weights.hostage_heir
            } else {
                weights.hostage_other
            },
        );
    } else {
        reasons.push(
            "Otage en gage de parole",
            if heir {
                weights.pledge_heir
            } else {
                weights.pledge_other
            },
        );
    }
}
