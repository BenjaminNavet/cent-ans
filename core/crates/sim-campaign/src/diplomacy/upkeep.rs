//! Turn phase: modifiers, offers, vassals, extinct lines.

use super::*;

/// Phase (after the economy): expiry, vassal tribute, loyalty and rebellion.
pub(crate) fn resolve_diplomacy(
    state: &mut CampaignState,
    data: &GameData,
    events: &mut Vec<GameEvent>,
) {
    crate::negotiation::resolve_negotiation(state, data, events);
    super::update_league(state, data, events);
    let turn = state.turn;
    for f in state.factions.values_mut() {
        f.ledger.non_aggression.retain(|_, until| *until > turn);
        f.modifiers.retain(|m| m.expires_turn > turn);
        f.claims.retain(|c| c.expires_turn.is_none_or(|t| t > turn));
        f.truces.retain(|_, until| *until > turn);
    }
    let player = state.player_faction.clone();
    let expired: Vec<Offer> = state
        .factions
        .get(&player)
        .map(|f| {
            f.offers
                .iter()
                .filter(|o| o.expires_turn <= turn + 1)
                .cloned()
                .collect()
        })
        .unwrap_or_default();
    if let Some(f) = state.factions.get_mut(&player) {
        f.offers.retain(|o| o.expires_turn > turn + 1);
    }
    for offer in expired {
        if offer.proposal.is_obedience() {
            continue; // keeping the historical obedience is the default
        }
        if offer.proposal.is_feudal_call() {
            crate::feudal::refuse_feudal_call(state, data, &player, &offer);
            continue;
        }
        if offer.proposal.is_ally_call() {
            state.shirk_ally_call(data, &player, &offer);
            continue;
        }
        if offer.proposal.is_ultimatum() {
            state.ultimatum_declined(data, &player, &offer);
            continue;
        }
        events.push(
            GameEvent::new(
                EventKind::DiplomaticOffer,
                format!("L'offre de {} a expiré.", data.faction_name(&offer.from)),
            )
            .faction(&offer.from),
        );
    }

    // Vassals: the suzerains are a view of the titles.
    crate::feudal::forget_expired(state);
    crate::feudal::sync_suzerains(state, data);
    let vassals: Vec<(FactionId, FactionId)> = state
        .factions
        .iter()
        .filter(|(_, f)| f.alive)
        .filter_map(|(id, f)| f.suzerain.clone().map(|s| (id.clone(), s)))
        .collect();
    for (vassal, suzerain) in vassals {
        if !state.factions.get(&suzerain).is_some_and(|f| f.alive) {
            crate::feudal::release_from_liege(state, &vassal);
            let v = state.factions.get_mut(&vassal).expect("exists");
            v.allies.remove(&suzerain);
            continue;
        }
        // Tribute to the direct suzerain only (the maxim, spec § 4.1).
        let tribute = crate::feudal::tribute_due(state, data, &vassal)
            .filter(|(liege, _)| liege == &suzerain)
            .map_or(0, |(_, amount)| amount);
        state.factions.get_mut(&vassal).expect("exists").treasury -= tribute;
        state.factions.get_mut(&suzerain).expect("exists").treasury += tribute;
        let target = loyalty_target(state, data, &vassal, &suzerain);
        let v = state.factions.get_mut(&vassal).expect("exists");
        v.loyalty = move_towards(v.loyalty, target, data.feudal_rules.loyalty.drift_per_turn);
        let loyalty = v.loyalty;
        // FE5: AI vassals revolt by their own order when the policy plans it.
        let planned = crate::feudal::policy().planned_revolts && vassal != state.player_faction;
        if !planned
            && loyalty < data.feudal_rules.rebellion_loyalty
            && !state.is_at_war(&vassal, &suzerain)
            && state
                .rng
                .chance_permille(data.feudal_rules.rebellion_permille)
        {
            // FE (F3): the felony case first (FE5: once the tie is cut the
            // rebel no longer holds of its suzerain and no case opened).
            crate::feudal::on_revolt(state, data, &vassal, &suzerain);
            state.cut_vassal_tie(&vassal, &suzerain);
            state.start_war(&vassal, &suzerain);
            events.push(
                GameEvent::new(
                    EventKind::VassalRebellion,
                    format!(
                        "{} se révolte contre son suzerain {} et proclame son indépendance.",
                        data.faction_name(&vassal),
                        data.faction_name(&suzerain)
                    ),
                )
                .faction(&vassal),
            );
        }
    }
    // Events queued by orders during the previous planning phase join now.
    let queued = std::mem::take(&mut state.pending_events);
    events.splice(0..0, queued);
}

fn move_towards(current: u8, target: u8, step: u8) -> u8 {
    if current < target {
        current.saturating_add(step).min(target)
    } else {
        current.saturating_sub(step).max(target)
    }
}

/// Loyalty a vassal drifts towards (0-100), towards its direct suzerain,
/// with the terms of `feudal.json:loyalty` (spec § 4.2).
pub fn loyalty_target(
    state: &CampaignState,
    data: &GameData,
    vassal: &FactionId,
    suzerain: &FactionId,
) -> u8 {
    let (attitude, reasons) = state.attitude(data, vassal, suzerain);
    let loyalty_term: i32 = reasons
        .iter()
        .filter(|(t, _)| t == "Loyauté envers le suzerain")
        .map(|(_, v)| v)
        .sum();
    let weights = &data.feudal_rules.loyalty;
    let mut target = weights.base + (attitude - loyalty_term) / 2;
    if state.faction_power(suzerain) > weights.power_ratio * state.faction_power(vassal) {
        target += weights.power_favourable;
    } else {
        target += weights.power_unfavourable;
    }
    if religion::is_excommunicated(state, suzerain) {
        target += weights.excommunicated_liege;
    }
    // Kin: a marriage between the ruling houses, or the same house.
    let kin = state.marriage_tie(vassal, suzerain)
        || state.marriage_tie(suzerain, vassal)
        || state
            .ruler_house(vassal)
            .is_some_and(|h| state.ruler_house(suzerain) == Some(h));
    if kin {
        target += weights.family_tie;
    }
    let culture = |f: &FactionId| data.factions.get(f).map(|f| &f.culture);
    if culture(vassal).is_some() && culture(vassal) == culture(suzerain) {
        target += weights.shared_culture;
    }
    target += crate::feudal::remembered_loyalty(state, data, vassal, suzerain);
    // F1 `Loyalty`: a loyal vassal ruler, a generous or kind overlord.
    let loyalty = |e: &crate::buildings::EffectTotals| e[EffectKind::Loyalty].apply(0.0);
    target += state.ruler_effect_points(data, vassal, loyalty)
        + state.ruler_effect_points(data, suzerain, loyalty);
    // Embargoed by an enemy of the suzerain: the vassal's trade suffers
    // for its overlord's quarrels (Flanders and English wool, 1336).
    let squeezed = state
        .factions
        .iter()
        .any(|(id, f)| f.alive && f.embargoes.contains(vassal) && state.is_at_war(id, suzerain));
    if squeezed {
        target += weights.embargo_squeeze;
    }
    target.clamp(0, 100) as u8
}

/// When a ruling line dies out, relatives in other factions through a
/// foreign mother gain a claim on the throne; a friendly claimant at peace
/// takes the realm in personal union (it becomes a vassal).
pub(crate) fn on_line_extinct(
    state: &mut CampaignState,
    data: &GameData,
    faction: &FactionId,
    house: &str,
    events: &mut Vec<GameEvent>,
) {
    let claimant = state
        .characters
        .iter()
        .filter(|(_, c)| c.alive && &c.faction != faction)
        .filter(|(_, c)| {
            c.mother
                .as_ref()
                .and_then(|m| state.characters.get(m))
                .is_some_and(|m| m.house == house)
        })
        .min_by_key(|(id, c)| (c.birth_year, (*id).clone()))
        .map(|(id, c)| (id.clone(), c.faction.clone()));
    let Some((character, claimant_faction)) = claimant else {
        return;
    };
    if claimant_faction.is_rebels() {
        return;
    }
    let text = format!(
        "{} hérite d'une prétention au trône de {} par sa mère.",
        state.character_name(data, &character),
        data.faction_name(faction)
    );
    state
        .factions
        .get_mut(&claimant_faction)
        .expect("exists")
        .claims
        .push(Claim {
            kind: ClaimKind::Throne,
            faction: Some(faction.clone()),
            province: None,
            text_fr: text.clone(),
            expires_turn: None,
            dynastic: false,
        });
    events.push(GameEvent::new(EventKind::Diplomacy, text).faction(&claimant_faction));
    let friendly = !state.is_at_war(&claimant_faction, faction)
        && state.attitude(data, faction, &claimant_faction).0 > 50;
    if friendly && state.factions[faction].suzerain.is_none() {
        state.make_vassal(data, &claimant_faction, faction);
        events.push(
            GameEvent::new(
                EventKind::Vassalage,
                format!(
                    "Union personnelle : {} passe sous l'autorité de {}.",
                    data.faction_name(faction),
                    data.faction_name(&claimant_faction)
                ),
            )
            .faction(faction),
        );
    }
}
