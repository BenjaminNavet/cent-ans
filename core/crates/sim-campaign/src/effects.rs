//! Event effects (M10): what an [`EventEffect`] does ([`apply_effect`]) and
//! how a tooltip words it ([`CampaignState::describe_effect`]).
//!
//! Effects that nudge one number share a table: [`ProvinceStat`] (a delta on
//! every class of the targeted provinces) and [`CharacterStat`] (a delta on
//! one character), each with its label and its way of applying a delta, so
//! the description and the application cannot drift apart. The other effects
//! have one arm in each of the two `match`es below.

use data_model::{
    CharacterId, CharacterRef, ClaimKind, EventEffect, FactionId, GameData, ProvinceId, ProvinceRef,
};

use crate::chronicle::{
    capture_character, marry, pay_sale_price, price_label, release_character, set_ruler,
    transfer_province, transfer_title, EventContext, PlagueWave, ScheduledEvent, OPINION_TURNS,
    PEACE_TRUCE_TURNS,
};
use crate::diplomacy::Claim;
use crate::events::{EventKind, GameEvent};
use crate::state::{Army, CampaignState, Unit, TURNS_PER_YEAR};
use crate::{characters, religion, skills};

impl CampaignState {
    /// French one-line summary of an effect (tooltips).
    pub fn describe_effect(
        &self,
        data: &GameData,
        effect: &EventEffect,
        ctx: &EventContext,
    ) -> String {
        let signed = |value: i64| {
            if value >= 0 {
                format!("+{value}")
            } else {
                value.to_string()
            }
        };
        let faction_label = |explicit: &Option<FactionId>| {
            explicit
                .as_ref()
                .filter(|f| Some(*f) != ctx.faction.as_ref())
                .map(|f| format!(" ({})", data.faction_name(f)))
                .unwrap_or_default()
        };
        let where_ = |target: &Option<ProvinceRef>| match (target, &ctx.province) {
            (Some(ProvinceRef::All), _) | (None, None) => " (toutes vos provinces)".to_owned(),
            (Some(ProvinceRef::Id(p)), _) | (None, Some(p)) => {
                format!(" ({})", data.province_name(p))
            }
        };
        let who = |character: &CharacterRef, faction: &Option<FactionId>| {
            let faction = faction.clone().or(ctx.faction.clone());
            match self.resolve_character(character, faction.as_ref()) {
                Some(id) => format!(" ({})", self.character_name(data, &id)),
                None => String::new(),
            }
        };
        if let Some((stat, province, amount)) = ProvinceStat::of(effect) {
            return format!(
                "{} {}{}{}",
                stat.label(),
                signed(i64::from(amount)),
                stat.unit(),
                where_(province)
            );
        }
        if let Some((stat, character, faction, amount)) = CharacterStat::of(effect) {
            return format!(
                "{} {}{}",
                stat.label(),
                signed(i64::from(amount)),
                who(character, faction)
            );
        }
        match effect {
            // Handled by the stat tables above.
            EventEffect::Unrest { .. }
            | EventEffect::Health { .. }
            | EventEffect::Wealth { .. }
            | EventEffect::Devastation { .. }
            | EventEffect::Population { .. }
            | EventEffect::Prestige { .. }
            | EventEffect::Piety { .. } => unreachable!("handled by ProvinceStat / CharacterStat"),
            EventEffect::Treasury { faction, amount } => {
                // UI audit A3 E3: livres tournois with the ₶ sign and
                // grouped digits, as everywhere else in the interface.
                // EQ1: the amount the faction will really pay or receive.
                let target = faction.clone().or(ctx.faction.clone());
                let amount = event_treasury_amount(self, data, target.as_ref(), *amount);
                format!(
                    "Trésor {}{}",
                    crate::economy_balance::signed_livres(amount),
                    faction_label(faction)
                )
            }
            EventEffect::PapalFavor { faction, amount } => format!(
                "Faveur pontificale {}{}",
                signed(i64::from(*amount)),
                faction_label(faction)
            ),
            EventEffect::Opinion {
                faction,
                towards,
                amount,
                ..
            } => {
                let towards = towards
                    .as_ref()
                    .or(ctx.faction.as_ref())
                    .map(|f| format!(" envers {}", data.faction_name(f)))
                    .unwrap_or_default();
                format!(
                    "Attitude de {}{} {}",
                    data.faction_name(faction),
                    towards,
                    signed(i64::from(*amount))
                )
            }
            EventEffect::DeclareWar { a, b } => format!(
                "Guerre : {} contre {}",
                data.faction_name(a),
                data.faction_name(b)
            ),
            EventEffect::Peace { a, b } => format!(
                "Paix entre {} et {}",
                data.faction_name(a),
                data.faction_name(b)
            ),
            EventEffect::AddTrait {
                character,
                faction,
                trait_id,
            } => {
                let name = data
                    .traits
                    .get(trait_id)
                    .map_or_else(|| trait_id.to_string(), |t| t.name.display.clone());
                format!("Trait « {name} »{}", who(character, faction))
            }
            EventEffect::KillCharacter { id, faction } => {
                format!("Mort{}", who(id, faction))
            }
            EventEffect::SpawnArmy {
                faction,
                province,
                units,
            } => {
                let place = province
                    .as_ref()
                    .or(ctx.province.as_ref())
                    .map(|p| format!(" en {}", data.province_name(p)))
                    .unwrap_or_default();
                format!(
                    "Nouvelle armée de {} unités{}{}",
                    units.len(),
                    place,
                    faction_label(faction)
                )
            }
            EventEffect::Claim {
                faction,
                kind,
                target,
            } => {
                let target_name = match kind {
                    ClaimKind::Throne => FactionId::new(target.clone())
                        .map(|f| format!("le trône de {}", data.faction_name(&f)))
                        .unwrap_or_else(|_| target.clone()),
                    ClaimKind::Province => ProvinceId::new(target.clone())
                        .map(|p| data.province_name(&p))
                        .unwrap_or_else(|_| target.clone()),
                };
                format!("Prétention sur {target_name}{}", faction_label(faction))
            }
            EventEffect::Loyalty { vassal, amount } => {
                let who = vassal
                    .as_ref()
                    .map(|v| format!(" de {}", data.faction_name(v)))
                    .unwrap_or_else(|| " des vassaux".to_owned());
                format!("Loyauté{who} {}", signed(i64::from(*amount)))
            }
            EventEffect::PlagueWave { .. } => {
                "La peste gagne toutes les provinces, du sud vers le nord (santé, population, \
                 mécontentement)"
                    .to_owned()
            }
            EventEffect::CaptureCharacter {
                id,
                faction,
                captor,
            } => format!(
                "Captivité{} aux mains de {}",
                who(id, faction),
                data.faction_name(captor)
            ),
            EventEffect::ReleaseCharacter {
                id,
                faction,
                ransom,
            } => {
                if *ransom > 0 {
                    format!(
                        "Libération{} contre {ransom} livres de rançon",
                        who(id, faction)
                    )
                } else {
                    format!("Libération{}", who(id, faction))
                }
            }
            EventEffect::ScheduleEvent { event, delay } => {
                let title = data
                    .events
                    .get(event)
                    .map_or_else(|| event.to_string(), |e| e.title.clone());
                format!("Suite : « {title} » dans {delay} saison(s)")
            }
            EventEffect::FoundChivalricOrder { order, .. } => {
                let name = data
                    .chivalric_orders
                    .get(order)
                    .map_or_else(|| order.to_string(), |o| o.name.display.clone());
                format!("Fondation de l'ordre « {name} »")
            }
            EventEffect::TransferProvince {
                province,
                faction,
                price,
                payer,
                ..
            } => {
                let text = match faction {
                    Some(f) => format!(
                        "{} passe à {}",
                        data.province_name(province),
                        data.faction_name(f)
                    ),
                    None => format!("{} rejoint le domaine", data.province_name(province)),
                };
                format!("{text}{}", price_label(data, *price, payer))
            }
            EventEffect::TransferTitle {
                title,
                faction,
                price,
                payer,
                ..
            } => {
                let name = data
                    .titles
                    .get(title)
                    .map_or_else(|| title.to_string(), |t| t.name.display.clone());
                let text = match faction {
                    Some(f) => format!("{name} passe à {}", data.faction_name(f)),
                    None => format!("{name} rejoint le domaine"),
                };
                format!("{text}{}", price_label(data, *price, payer))
            }
            EventEffect::SetRuler { character, faction } => {
                let realm = faction
                    .as_ref()
                    .or(ctx.faction.as_ref())
                    .map(|f| format!(" de {}", data.faction_name(f)))
                    .unwrap_or_default();
                format!(
                    "{} prend la tête{realm}",
                    self.character_name(data, character)
                )
            }
            EventEffect::Marry { a, b } => format!(
                "Mariage de {} et {}",
                self.character_name(data, a),
                self.character_name(data, b)
            ),
        }
    }

    fn resolve_character(
        &self,
        character: &CharacterRef,
        faction: Option<&FactionId>,
    ) -> Option<CharacterId> {
        let id = match character {
            CharacterRef::Ruler => self.factions.get(faction?)?.ruler.clone()?,
            CharacterRef::Heir => self.factions.get(faction?)?.heir.clone()?,
            CharacterRef::Id(id) => id.clone(),
        };
        self.characters.get(&id).filter(|c| c.alive).map(|_| id)
    }

    /// Provinces targeted by a province effect.
    fn effect_provinces(
        &self,
        target: &Option<ProvinceRef>,
        ctx: &EventContext,
    ) -> Vec<ProvinceId> {
        let all = || match &ctx.faction {
            Some(faction) => self.controlled_provinces(faction),
            None => Vec::new(),
        };
        match (target, &ctx.province) {
            (Some(ProvinceRef::All), _) | (None, None) => all(),
            (Some(ProvinceRef::Id(p)), _) | (None, Some(p)) => {
                if self.provinces.contains_key(p) {
                    vec![p.clone()]
                } else {
                    Vec::new()
                }
            }
        }
    }
}

pub(crate) fn add_clamped(value: u8, delta: i32) -> u8 {
    (i32::from(value) + delta).clamp(0, 100) as u8
}

/// A number of the targeted provinces an effect nudges by a delta.
#[derive(Clone, Copy)]
enum ProvinceStat {
    Unrest,
    Health,
    Wealth,
    Devastation,
    /// The delta is a percentage of every class's headcount.
    Population,
}

impl ProvinceStat {
    /// The stat, province target and delta of `effect`, if it is one of them.
    fn of(effect: &EventEffect) -> Option<(Self, &Option<ProvinceRef>, i32)> {
        match effect {
            EventEffect::Unrest { province, amount } => Some((Self::Unrest, province, *amount)),
            EventEffect::Health { province, amount } => Some((Self::Health, province, *amount)),
            EventEffect::Wealth { province, amount } => Some((Self::Wealth, province, *amount)),
            EventEffect::Devastation { province, amount } => {
                Some((Self::Devastation, province, *amount))
            }
            EventEffect::Population { province, percent } => {
                Some((Self::Population, province, *percent))
            }
            _ => None,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Unrest => "Mécontentement",
            Self::Health => "Santé",
            Self::Wealth => "Richesse",
            Self::Devastation => "Dévastation",
            Self::Population => "Population",
        }
    }

    fn unit(self) -> &'static str {
        match self {
            Self::Population => " %",
            _ => "",
        }
    }

    fn apply(self, state: &mut CampaignState, province: &ProvinceId, delta: i32) {
        let Some(p) = state.provinces.get_mut(province) else {
            return;
        };
        if let Self::Devastation = self {
            p.devastation = add_clamped(p.devastation, delta);
            return;
        }
        let factor = f64::from((100 + delta).max(0)) / 100.0;
        for entry in p.population.iter_mut() {
            match self {
                Self::Unrest => entry.unrest = add_clamped(entry.unrest, delta),
                Self::Health => entry.health = add_clamped(entry.health, delta),
                Self::Wealth => entry.wealth = add_clamped(entry.wealth, delta),
                Self::Population => entry.count = (entry.count as f64 * factor).round() as u64,
                Self::Devastation => {}
            }
        }
    }
}

/// A number of one character an effect nudges by a delta.
#[derive(Clone, Copy)]
enum CharacterStat {
    Prestige,
    Piety,
}

impl CharacterStat {
    /// The stat, character, faction and delta of `effect`, if it is one of them.
    fn of(effect: &EventEffect) -> Option<(Self, &CharacterRef, &Option<FactionId>, i32)> {
        match effect {
            EventEffect::Prestige {
                character,
                faction,
                amount,
            } => Some((Self::Prestige, character, faction, *amount)),
            EventEffect::Piety {
                character,
                faction,
                amount,
            } => Some((Self::Piety, character, faction, *amount)),
            _ => None,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Prestige => "Prestige",
            Self::Piety => "Piété",
        }
    }

    fn apply(self, character: &mut crate::state::CharacterState, delta: i32) {
        match self {
            Self::Prestige => character.prestige += delta,
            Self::Piety => character.piety = add_clamped(character.piety, delta),
        }
    }
}

/// EQ1 (`data/rules/economy.json`): the treasury effect `amount` of an
/// event for `faction`, scaled down for a faction whose seasonal income is
/// below the reference (never below the minimum share): event costs are
/// written for a middling realm, and a small county must not be ruined by a
/// fire it could not have prevented.
pub fn event_treasury_amount(
    state: &CampaignState,
    data: &GameData,
    faction: Option<&FactionId>,
    amount: i64,
) -> i64 {
    let rules = &data.economy_rules;
    let Some(f) = faction.and_then(|id| state.factions.get(id).map(|f| (id, f))) else {
        return amount;
    };
    let income = if f.1.income_last_turn > 0 {
        f.1.income_last_turn
    } else {
        state.faction_income_effective(data, f.0)
    };
    let reference = rules.event_treasury_reference_income.max(1);
    if income >= reference {
        return amount;
    }
    let scale = (income.max(0) as f64 / reference as f64)
        .max(rules.event_treasury_min_scale)
        .min(1.0);
    (amount as f64 * scale).round() as i64
}

/// Applies one effect in `ctx` (the deciding faction and the event's
/// province). Unknown ids and impossible actions are ignored.
pub fn apply_effect(
    state: &mut CampaignState,
    data: &GameData,
    effect: &EventEffect,
    ctx: &EventContext,
    events: &mut Vec<GameEvent>,
) {
    let target_faction = |explicit: &Option<FactionId>| explicit.clone().or(ctx.faction.clone());
    if let Some((stat, province, amount)) = ProvinceStat::of(effect) {
        for id in state.effect_provinces(province, ctx) {
            stat.apply(state, &id, amount);
        }
        return;
    }
    if let Some((stat, character, faction, amount)) = CharacterStat::of(effect) {
        let faction = target_faction(faction);
        if let Some(c) = state
            .resolve_character(character, faction.as_ref())
            .and_then(|id| state.characters.get_mut(&id))
        {
            stat.apply(c, amount);
        }
        return;
    }
    match effect {
        // Handled by the stat tables above.
        EventEffect::Unrest { .. }
        | EventEffect::Health { .. }
        | EventEffect::Wealth { .. }
        | EventEffect::Devastation { .. }
        | EventEffect::Population { .. }
        | EventEffect::Prestige { .. }
        | EventEffect::Piety { .. } => unreachable!("handled by ProvinceStat / CharacterStat"),
        EventEffect::Treasury { faction, amount } => {
            let target = target_faction(faction);
            let amount = event_treasury_amount(state, data, target.as_ref(), *amount);
            if let Some(f) = target.and_then(|f| state.factions.get_mut(&f)) {
                f.treasury += amount;
            }
        }
        EventEffect::PapalFavor { faction, amount } => {
            if let Some(f) = target_faction(faction) {
                religion::change_favor(state, &f, *amount);
            }
        }
        EventEffect::Opinion {
            faction,
            towards,
            amount,
            reason,
            duration,
        } => {
            let Some(towards) = towards.clone().or(ctx.faction.clone()) else {
                return;
            };
            if faction != &towards
                && state.faction_alive(faction)
                && state.factions.contains_key(&towards)
            {
                // LR-07: an event may give a capped motive too.
                state.add_capped_modifier(
                    data,
                    faction,
                    &towards,
                    *amount,
                    reason,
                    duration.unwrap_or(OPINION_TURNS),
                );
            }
        }
        EventEffect::DeclareWar { a, b } => {
            if state.faction_alive(a) && state.faction_alive(b) && !state.is_at_war(a, b) {
                let _ = state.declare_war(data, a, b);
            }
        }
        EventEffect::Peace { a, b } => {
            if state.is_at_war(a, b) {
                state.make_peace(data, a, b, &[], 0, PEACE_TRUCE_TURNS);
            }
        }
        EventEffect::AddTrait {
            character,
            faction,
            trait_id,
        } => {
            let faction = target_faction(faction);
            if !data.traits.contains_key(trait_id) {
                return;
            }
            if let Some(id) = state.resolve_character(character, faction.as_ref()) {
                if skills::grant_trait(state, data, &id, trait_id) {
                    let name = &data.traits[trait_id].name.display;
                    let owner = state.characters[&id].faction.clone();
                    events.push(
                        GameEvent::new(
                            EventKind::TraitAcquired,
                            format!("{} devient « {name} ».", state.character_name(data, &id)),
                        )
                        .faction(&owner),
                    );
                }
            }
        }
        EventEffect::KillCharacter { id, faction } => {
            let faction = target_faction(faction);
            if let Some(id) = state.resolve_character(id, faction.as_ref()) {
                characters::kill(state, data, &id, events);
            }
        }
        EventEffect::SpawnArmy {
            faction,
            province,
            units,
        } => {
            let Some(faction) = target_faction(faction).filter(|f| state.faction_alive(f)) else {
                return;
            };
            let location = province
                .clone()
                .or(ctx.province.clone())
                .or_else(|| state.factions.get(&faction).map(|f| f.capital.clone()))
                .filter(|p| state.provinces.contains_key(p));
            let Some(location) = location else {
                return;
            };
            let units: Vec<Unit> = units
                .iter()
                .filter_map(|u| data.unit_types.get(u))
                .map(Unit::fresh)
                .collect();
            if units.is_empty() {
                return;
            }
            let Some(city) = state.province_city_id(&location).cloned() else {
                return;
            };
            let id = state.allocate_army_id();
            let mut army = Army::new(
                faction.clone(),
                crate::state::ArmyPosition::Settlement(city),
                units,
            );
            army.movement_left = crate::march::km_to_grid_points(
                data,
                f64::from(state.season_movement_points(data)),
            );
            state.armies.insert(id.clone(), army);
            events.push(
                GameEvent::new(
                    EventKind::Chronicle,
                    format!(
                        "Une nouvelle armée de {} se lève en {}.",
                        data.faction_name(&faction),
                        data.province_name(&location)
                    ),
                )
                .faction(&faction)
                .province(&location)
                .army(&id),
            );
        }
        EventEffect::Claim {
            faction,
            kind,
            target,
        } => {
            let Some(holder) = target_faction(faction) else {
                return;
            };
            let (claim_faction, claim_province) = match kind {
                ClaimKind::Throne => (FactionId::new(target.clone()).ok(), None),
                ClaimKind::Province => (None, ProvinceId::new(target.clone()).ok()),
            };
            let known = claim_faction
                .as_ref()
                .is_some_and(|f| state.factions.contains_key(f))
                || claim_province
                    .as_ref()
                    .is_some_and(|p| state.provinces.contains_key(p));
            let Some(f) = state.factions.get_mut(&holder).filter(|_| known) else {
                return;
            };
            let duplicate = f.claims.iter().any(|c| {
                c.kind == *kind && c.faction == claim_faction && c.province == claim_province
            });
            if !duplicate {
                f.claims.push(Claim {
                    kind: *kind,
                    faction: claim_faction,
                    province: claim_province,
                    text_fr: "prétention née de la chronique".to_owned(),
                    expires_turn: None,
                });
            }
        }
        EventEffect::Loyalty { vassal, amount } => {
            let vassals: Vec<FactionId> = match vassal {
                Some(v) => vec![v.clone()],
                None => match &ctx.faction {
                    Some(suzerain) => state
                        .factions
                        .iter()
                        .filter(|(_, f)| f.suzerain.as_ref() == Some(suzerain))
                        .map(|(id, _)| id.clone())
                        .collect(),
                    None => Vec::new(),
                },
            };
            for v in vassals {
                if let Some(f) = state.factions.get_mut(&v) {
                    f.loyalty = add_clamped(f.loyalty, *amount);
                }
            }
        }
        EventEffect::CaptureCharacter {
            id,
            faction,
            captor,
        } => {
            let faction = target_faction(faction);
            if let Some(id) = state.resolve_character(id, faction.as_ref()) {
                capture_character(state, data, &id, captor, events);
            }
        }
        EventEffect::ReleaseCharacter {
            id,
            faction,
            ransom,
        } => {
            let faction = target_faction(faction);
            if let Some(id) = state.resolve_character(id, faction.as_ref()) {
                release_character(state, data, &id, *ransom, events);
            }
        }
        EventEffect::ScheduleEvent { event, delay } => {
            if data.events.contains_key(event) {
                state.chronicle.scheduled.push(ScheduledEvent {
                    event: event.clone(),
                    turn: state.turn + (*delay).max(1),
                    faction: ctx.faction.clone(),
                    province: ctx.province.clone(),
                });
            }
        }
        EventEffect::Marry { a, b } => {
            marry(state, data, a, b, events);
        }
        EventEffect::TransferProvince {
            province,
            faction,
            from,
            price,
            payer,
        } => {
            let holder = state
                .province_owner(province)
                .zip(state.province_controller(province));
            let from_ok = from
                .as_ref()
                .is_none_or(|f| holder.is_some_and(|(o, c)| o == f || c == f));
            let seller = state.province_owner(province).cloned();
            if let Some(faction) = target_faction(faction).filter(|_| from_ok) {
                if transfer_province(state, data, province, &faction, events) {
                    let payer = payer.clone().unwrap_or_else(|| faction.clone());
                    pay_sale_price(state, data, &payer, seller.as_ref(), *price, events);
                }
            }
        }
        EventEffect::TransferTitle {
            title,
            faction,
            from,
            price,
            payer,
        } => {
            let holder = crate::feudal::holder_of(state, title).cloned();
            let from_ok = from.as_ref().is_none_or(|f| holder.as_ref() == Some(f));
            if let Some(faction) = target_faction(faction).filter(|_| from_ok) {
                if transfer_title(state, data, title, &faction, events) {
                    let payer = payer.clone().unwrap_or_else(|| faction.clone());
                    pay_sale_price(state, data, &payer, holder.as_ref(), *price, events);
                }
            }
        }
        EventEffect::SetRuler { character, faction } => {
            if let Some(faction) = target_faction(faction) {
                set_ruler(state, data, &faction, character, events);
            }
        }
        EventEffect::FoundChivalricOrder { order, faction } => {
            if let Some(faction) = target_faction(faction) {
                crate::chivalry::found_order_by_event(state, data, &faction, order, events);
            }
        }
        EventEffect::PlagueWave { from_year, to_year } => {
            if state.chronicle.plague_wave.is_none() {
                let years = (to_year - from_year).max(0) as u32;
                state.chronicle.plague_wave = Some(PlagueWave {
                    start_turn: state.turn,
                    duration: (years * TURNS_PER_YEAR).max(1),
                });
            }
        }
    }
}
