//! H6 « Rançons » : prix d'un captif, paiement, échéances, parole, cession.
//!
//! A character taken in battle or by a chronicle event (`captive`,
//! `captor`) carries a ransom computed from his rank, his prestige and the
//! wealth of his faction ([`ransom_amount`]). His captor sets the terms
//! (`set_ransom_terms`: money, a border province, or keep him), may free him
//! on parole (`release_on_parole`); his own faction pays (`pay_ransom`), in
//! full or by yearly installments (the captive comes home with the first
//! one, as Jean II after Brétigny; the rest is a debt, [`RansomDebt`]). A
//! missed installment costs prestige, the creditor's goodwill and a 10 %
//! surcharge.
//!
//! Release always goes through `chronicle::release_character`, shared with
//! the ransom events (`evt_rancon_david_ii`...): a freed character is no
//! longer captive, so a second release (order or event) is a no-op.

use data_model::{CharacterId, FactionId, GameData, ProvinceId};
use serde::{Deserialize, Serialize};

use crate::events::{EventKind, GameEvent};
use crate::orders::Order;
use crate::state::CampaignState;

/// Base ransom of a ruler (livres tournois).
pub const RANSOM_SOVEREIGN: i64 = 10_000;
/// Base ransom of a ruler's heir.
pub const RANSOM_HEIR: i64 = 5_000;
/// Base ransom of a great noble (titled, or prestige ≥ [`GREAT_NOBLE_PRESTIGE`]).
pub const RANSOM_GREAT_NOBLE: i64 = 1_500;
/// Base ransom of a plain knight.
pub const RANSOM_KNIGHT: i64 = 400;
/// Prestige from which an untitled character counts as a great noble.
pub const GREAT_NOBLE_PRESTIGE: i32 = 30;
/// Maximum number of yearly installments.
pub const MAX_INSTALLMENTS: u32 = 6;
/// Surcharge (per cent) of a ransom paid by installments.
pub const INSTALLMENT_SURCHARGE_PERCENT: i64 = 10;
/// Surcharge (per cent of the remaining debt) of a missed installment.
pub const DEFAULT_SURCHARGE_PERCENT: i64 = 10;
/// Ruler prestige lost per missed installment.
pub const DEFAULT_PRESTIGE: i32 = 5;
/// Creditor's opinion lost per missed installment.
pub const DEFAULT_OPINION: i32 = -15;
/// Prestige the captor's ruler gains by a release on parole.
pub const PAROLE_PRESTIGE: i32 = 8;
/// Opinion the freed character's faction gains towards the captor.
pub const PAROLE_OPINION: i32 = 20;
/// Ruler prestige lost every season the ruler himself is a captive.
pub const CAPTIVE_RULER_PRESTIGE: i32 = 1;

/// Rank of a captive, which sets the base of his ransom.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CaptiveRank {
    Sovereign,
    Heir,
    GreatNoble,
    Knight,
}

impl CaptiveRank {
    pub fn base_ransom(self) -> i64 {
        match self {
            CaptiveRank::Sovereign => RANSOM_SOVEREIGN,
            CaptiveRank::Heir => RANSOM_HEIR,
            CaptiveRank::GreatNoble => RANSOM_GREAT_NOBLE,
            CaptiveRank::Knight => RANSOM_KNIGHT,
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            CaptiveRank::Sovereign => "sovereign",
            CaptiveRank::Heir => "heir",
            CaptiveRank::GreatNoble => "great_noble",
            CaptiveRank::Knight => "knight",
        }
    }

    pub fn label_fr(self) -> &'static str {
        match self {
            CaptiveRank::Sovereign => "souverain",
            CaptiveRank::Heir => "héritier",
            CaptiveRank::GreatNoble => "grand seigneur",
            CaptiveRank::Knight => "chevalier",
        }
    }
}

/// Terms the captor sets for a captive (`set_ransom_terms`).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RansomTerms {
    /// Freed against the computed ransom (default).
    #[default]
    Money,
    /// Freed against a province of his faction bordering the captor's lands.
    Province { province: ProvinceId },
    /// Freed at once on his word (prestige and goodwill).
    Parole,
    /// Not for sale: the captor keeps him.
    Hold,
}

impl RansomTerms {
    pub fn key(&self) -> &'static str {
        match self {
            RansomTerms::Money => "money",
            RansomTerms::Province { .. } => "province",
            RansomTerms::Parole => "parole",
            RansomTerms::Hold => "hold",
        }
    }
}

/// The unpaid part of a ransom paid by installments (held by the payer).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RansomDebt {
    /// The freed character.
    pub character: CharacterId,
    /// Faction owed the money.
    pub creditor: FactionId,
    /// Livres still owed.
    pub remaining: i64,
    /// Livres due at each yearly installment.
    pub installment: i64,
    /// Turn of the next installment.
    pub next_due_turn: u32,
    /// Installments missed so far.
    #[serde(default)]
    pub missed: u32,
}

/// Why a ransom order was refused.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RansomError {
    #[error("ce personnage n'est pas captif")]
    NotCaptive,
    #[error("ce captif n'appartient pas à votre faction")]
    NotYourCaptive,
    #[error("personne ne détient ce captif contre rançon")]
    NoCaptor,
    #[error("ce captif n'est pas détenu par votre faction")]
    NotYourPrisoner,
    #[error("son geôlier refuse toute rançon")]
    Held,
    #[error("son geôlier exige une province, pas de l'argent : {0}")]
    ProvinceDemanded(String),
    #[error("nombre d'échéances invalide (1 à {MAX_INSTALLMENTS})")]
    BadInstallments,
    #[error("trésor insuffisant : {needed} livres nécessaires, {available} disponibles")]
    InsufficientFunds { needed: i64, available: i64 },
    #[error("province impossible : {0}")]
    BadProvince(String),
    #[error("sa famille refuse : {offered} livres demandés, sa rançon est de {fair} livres")]
    RansomTooHigh { offered: i64, fair: i64 },
    #[error("sa faction ne peut payer : {needed} livres demandées, {available} dans son trésor")]
    PayerCannotPay { needed: i64, available: i64 },
}

/// Rank of `character` in his own faction.
pub fn captive_rank(state: &CampaignState, character: &CharacterId) -> CaptiveRank {
    let Some(c) = state.characters.get(character) else {
        return CaptiveRank::Knight;
    };
    let faction = state.factions.get(&c.faction);
    if faction.is_some_and(|f| f.ruler.as_ref() == Some(character)) {
        CaptiveRank::Sovereign
    } else if faction.is_some_and(|f| f.heir.as_ref() == Some(character)) {
        CaptiveRank::Heir
    } else if c.title.is_some() || c.prestige >= GREAT_NOBLE_PRESTIGE {
        CaptiveRank::GreatNoble
    } else {
        CaptiveRank::Knight
    }
}

/// Wealth factor of a faction: its seasonal tax income / 10 000, between
/// 0.5 and 2 (a rich kingdom pays more for the same knight).
pub fn wealth_factor(state: &CampaignState, data: &GameData, faction: &FactionId) -> f64 {
    let income = state.faction_income_effective(data, faction).max(0);
    (income as f64 / 10_000.0).clamp(0.5, 2.0)
}

/// Ransom of `character`: `base(rank) × (1 + prestige/100) × wealth`,
/// prestige clamped to 0-200, rounded to 50 livres.
pub fn ransom_amount(state: &CampaignState, data: &GameData, character: &CharacterId) -> i64 {
    let Some(c) = state.characters.get(character) else {
        return 0;
    };
    let base = captive_rank(state, character).base_ransom() as f64;
    let prestige = 1.0 + f64::from(c.prestige.clamp(0, 200)) / 100.0;
    let raw = base * prestige * wealth_factor(state, data, &c.faction);
    // A6-L3 (ADR 0183): never more than a share of the payer's income.
    let income = state.faction_income_effective(data, &c.faction).max(0);
    let cap = income * data.economy_rules.ransom.income_cap_percent / 100;
    ((raw / 50.0).round() as i64 * 50).min(cap).max(50)
}

/// A6-L3 (ADR 0183): whether a beaten general may be taken prisoner. A
/// ruler leading a beaten army is taken only when that army routed (rule
/// `economy_rules.ransom.sovereign_capture_only_if_routed`).
pub fn capture_allowed(
    state: &CampaignState,
    data: &GameData,
    general: &CharacterId,
    army_routed: bool,
) -> bool {
    if army_routed || !data.economy_rules.ransom.sovereign_capture_only_if_routed {
        return true;
    }
    captive_rank(state, general) != CaptiveRank::Sovereign
}

/// `(total, installment)` of a ransom of `amount` paid in `installments`
/// yearly payments (surcharge from two installments on).
pub fn installment_plan(amount: i64, installments: u32) -> (i64, i64) {
    let n = i64::from(installments.max(1));
    let total = if n >= 2 {
        amount * (100 + INSTALLMENT_SURCHARGE_PERCENT) / 100
    } else {
        amount
    };
    (total, (total + n - 1) / n)
}

/// The captive `character` of `faction` (checked), and his captor.
fn own_captive(
    state: &CampaignState,
    faction: &FactionId,
    character: &CharacterId,
) -> Result<FactionId, RansomError> {
    let c = state
        .characters
        .get(character)
        .filter(|c| c.alive && c.captive)
        .ok_or(RansomError::NotCaptive)?;
    if &c.faction != faction {
        return Err(RansomError::NotYourCaptive);
    }
    c.captor.clone().ok_or(RansomError::NoCaptor)
}

/// `pay_ransom`: pays for one of our captives under his captor's terms:
/// money in full (`installments` 0 or 1) or by yearly installments (the
/// first now, he comes home), or the province his captor demands.
pub fn pay_ransom(
    state: &mut CampaignState,
    data: &GameData,
    faction: &FactionId,
    character: &CharacterId,
    installments: u32,
) -> Result<(), RansomError> {
    let captor = own_captive(state, faction, character)?;
    let terms = state.characters[character]
        .ransom_terms
        .clone()
        .unwrap_or_default();
    let name = state.character_name(data, character);
    match terms {
        RansomTerms::Hold => Err(RansomError::Held),
        RansomTerms::Parole => {
            release_on_parole(state, data, &captor, character)?;
            Ok(())
        }
        RansomTerms::Province { province } => {
            check_ceded_province(state, data, faction, &captor, &province)?;
            cede_province(state, faction, &captor, &province);
            let province_name = data
                .provinces
                .get(&province)
                .map_or_else(|| province.to_string(), |p| p.name.display.clone());
            crate::chronicle::release_character(state, data, character, 0, &mut Vec::new());
            let text = format!(
                "{name} est libéré contre la cession de la province {}.",
                crate::events::de(&province_name)
            );
            push_news(state, faction, &captor, text);
            Ok(())
        }
        RansomTerms::Money => {
            if installments > MAX_INSTALLMENTS {
                return Err(RansomError::BadInstallments);
            }
            let amount = ransom_amount(state, data, character);
            let (total, first) = installment_plan(amount, installments);
            let available = state.factions[faction].treasury;
            if available < first {
                return Err(RansomError::InsufficientFunds {
                    needed: first,
                    available,
                });
            }
            crate::chronicle::release_character(state, data, character, first, &mut Vec::new());
            let text = if installments >= 2 {
                let turn = state.turn;
                state
                    .factions
                    .get_mut(faction)
                    .expect("checked by apply_order")
                    .ransom_debts
                    .push(RansomDebt {
                        character: character.clone(),
                        creditor: captor.clone(),
                        remaining: total - first,
                        installment: first,
                        next_due_turn: turn + crate::state::TURNS_PER_YEAR,
                        missed: 0,
                    });
                format!(
                    "{name} est libéré : rançon de {total} livres en {installments} échéances annuelles de {first} livres."
                )
            } else {
                format!("{name} est libéré contre une rançon de {total} livres.")
            };
            push_news(state, faction, &captor, text);
            Ok(())
        }
    }
}

/// A ransom message (one journal entry, tagged with the captive's faction;
/// it opens the next turn's journal).
fn push_news(state: &mut CampaignState, payer: &FactionId, _captor: &FactionId, text: String) {
    state
        .pending_events
        .push(GameEvent::new(EventKind::Ransom, text).faction(payer));
}

/// A province `payer` may cede to `captor`: owned and controlled by the
/// payer, not its capital, bordering a province the captor controls.
fn check_ceded_province(
    state: &CampaignState,
    data: &GameData,
    payer: &FactionId,
    captor: &FactionId,
    province: &ProvinceId,
) -> Result<(), RansomError> {
    if !state.provinces.contains_key(province) {
        return Err(RansomError::BadProvince(format!(
            "province inconnue : {province}"
        )));
    }
    if !state.holds_province(payer, province) {
        return Err(RansomError::BadProvince(
            "la province doit être possédée et tenue par la faction du captif".to_owned(),
        ));
    }
    if state
        .factions
        .get(payer)
        .is_some_and(|f| &f.capital == province)
    {
        return Err(RansomError::BadProvince(
            "on ne cède pas sa capitale".to_owned(),
        ));
    }
    let borders = crate::movement::land_neighbors(data, province)
        .iter()
        .any(|n| state.controls_province(captor, n));
    if !borders {
        return Err(RansomError::BadProvince(
            "la province doit toucher une province du geôlier".to_owned(),
        ));
    }
    Ok(())
}

/// Border provinces `payer` could cede to `captor` (id order).
pub fn cedable_provinces(
    state: &CampaignState,
    data: &GameData,
    payer: &FactionId,
    captor: &FactionId,
) -> Vec<ProvinceId> {
    state
        .provinces
        .keys()
        .filter(|id| check_ceded_province(state, data, payer, captor, id).is_ok())
        .cloned()
        .collect()
}

/// The province passes to the captor (as in a peace treaty, without claim);
/// also used by the `transfer_province` event effect (G1). Lot C4: every
/// settlement the payer owns there changes hands (enclaves of third parties
/// stay).
pub(crate) fn cede_province(
    state: &mut CampaignState,
    payer: &FactionId,
    captor: &FactionId,
    province: &ProvinceId,
) {
    state.cede_province(province, Some(payer), captor);
    // The city always follows (province control is the city's).
    let city_owner = state.province_owner(province).cloned();
    if city_owner.as_ref() != Some(captor) {
        if let Some(owner) = city_owner {
            state.cede_province(province, Some(&owner), captor);
        }
    }
    for character in state.characters.values_mut() {
        if character.governor_of.as_ref() == Some(province) {
            character.governor_of = None;
        }
    }
}

/// `set_ransom_terms`: the captor states what it wants for a prisoner
/// (`parole` frees him at once).
pub fn set_ransom_terms(
    state: &mut CampaignState,
    data: &GameData,
    faction: &FactionId,
    character: &CharacterId,
    terms: RansomTerms,
) -> Result<(), RansomError> {
    let owner = own_prisoner(state, faction, character)?;
    if let RansomTerms::Province { province } = &terms {
        check_ceded_province(state, data, &owner, faction, province)?;
    }
    if terms == RansomTerms::Parole {
        return release_on_parole(state, data, faction, character);
    }
    let name = state.character_name(data, character);
    let text = match &terms {
        RansomTerms::Money => format!(
            "{name} peut être racheté pour {} livres.",
            ransom_amount(state, data, character)
        ),
        RansomTerms::Province { province } => format!(
            "Pour libérer {name}, son geôlier exige la province {}.",
            crate::events::de(
                &data
                    .provinces
                    .get(province)
                    .map_or_else(|| province.to_string(), |p| p.name.display.clone())
            )
        ),
        RansomTerms::Hold => format!("{name} restera captif : son geôlier refuse toute rançon."),
        RansomTerms::Parole => unreachable!("handled above"),
    };
    state
        .characters
        .get_mut(character)
        .expect("checked above")
        .ransom_terms = Some(terms);
    push_news(state, &owner, faction, text);
    Ok(())
}

/// The prisoner `character` held by `faction` (checked), and his faction.
fn own_prisoner(
    state: &CampaignState,
    faction: &FactionId,
    character: &CharacterId,
) -> Result<FactionId, RansomError> {
    let c = state
        .characters
        .get(character)
        .filter(|c| c.alive && c.captive)
        .ok_or(RansomError::NotCaptive)?;
    if c.captor.as_ref() != Some(faction) {
        return Err(RansomError::NotYourPrisoner);
    }
    // ADR 0025 § 6: a treaty hostage stays until the end of his term.
    let pledged = state
        .factions
        .get(faction)
        .is_some_and(|f| f.ledger.hostages.iter().any(|h| &h.character == character));
    if pledged {
        return Err(RansomError::Held);
    }
    Ok(c.faction.clone())
}

/// `release_on_parole`: the captor frees a prisoner on his word; its ruler
/// gains prestige, the freed man's faction goodwill towards the captor.
pub fn release_on_parole(
    state: &mut CampaignState,
    data: &GameData,
    faction: &FactionId,
    character: &CharacterId,
) -> Result<(), RansomError> {
    let owner = own_prisoner(state, faction, character)?;
    crate::chronicle::release_character(state, data, character, 0, &mut Vec::new());
    if let Some(ruler) = state.factions.get(faction).and_then(|f| f.ruler.clone()) {
        if let Some(r) = state.characters.get_mut(&ruler) {
            r.prestige += PAROLE_PRESTIGE;
        }
    }
    state.add_modifier(
        &owner,
        faction,
        PAROLE_OPINION,
        "Captif libéré sur parole",
        20,
    );
    let text = format!(
        "{} est libéré sur parole par {}.",
        state.character_name(data, character),
        data.factions
            .get(faction)
            .map_or_else(|| faction.to_string(), |f| f.name.display.clone())
    );
    push_news(state, &owner, faction, text);
    Ok(())
}

/// G1: prestige the captor's ruler gains by freeing a prisoner for ransom.
pub const RELEASE_RANSOM_PRESTIGE: i32 = 3;

/// `release_for_ransom` (order `ReleaseCaptive`): the captor frees a
/// prisoner against `ransom` livres (default: [`ransom_amount`]; 0 is a
/// release on parole). His faction pays at once, if the sum is no more than
/// the computed ransom and its treasury covers it.
pub fn release_for_ransom(
    state: &mut CampaignState,
    data: &GameData,
    faction: &FactionId,
    character: &CharacterId,
    ransom: Option<i64>,
) -> Result<(), RansomError> {
    let owner = own_prisoner(state, faction, character)?;
    let fair = ransom_amount(state, data, character);
    let offered = ransom.unwrap_or(fair).max(0);
    if offered == 0 {
        return release_on_parole(state, data, faction, character);
    }
    if offered > fair {
        return Err(RansomError::RansomTooHigh { offered, fair });
    }
    let available = state.factions.get(&owner).map_or(0, |f| f.treasury);
    if available < offered {
        return Err(RansomError::PayerCannotPay {
            needed: offered,
            available,
        });
    }
    crate::chronicle::release_character(state, data, character, offered, &mut Vec::new());
    if let Some(ruler) = state.factions.get(faction).and_then(|f| f.ruler.clone()) {
        if let Some(r) = state.characters.get_mut(&ruler) {
            r.prestige += RELEASE_RANSOM_PRESTIGE;
        }
    }
    let text = format!(
        "{} est libéré contre une rançon de {offered} livres.",
        state.character_name(data, character)
    );
    push_news(state, &owner, faction, text);
    Ok(())
}

/// Seasonal phase: installments falling due are paid (or missed, with
/// penalties), and a captive ruler costs his own prestige every season
/// (the regency itself is opened by `dynasty::resolve_regencies`).
pub(crate) fn resolve_ransoms(
    state: &mut CampaignState,
    data: &GameData,
    events: &mut Vec<GameEvent>,
) {
    let turn = state.turn;
    let ids: Vec<FactionId> = state.factions.keys().cloned().collect();
    for id in &ids {
        let debts = std::mem::take(&mut state.factions.get_mut(id).expect("listed").ransom_debts);
        let mut kept = Vec::new();
        for mut debt in debts {
            if debt.next_due_turn > turn || !state.factions[id].alive {
                kept.push(debt);
                continue;
            }
            let due = debt.installment.min(debt.remaining);
            let name = state.character_name(data, &debt.character);
            if state.factions[id].treasury >= due {
                state.factions.get_mut(id).expect("listed").treasury -= due;
                if let Some(creditor) = state.factions.get_mut(&debt.creditor) {
                    creditor.treasury += due;
                }
                debt.remaining -= due;
                let text = if debt.remaining > 0 {
                    format!(
                        "Échéance de la rançon de {name} versée : {due} livres (reste {}).",
                        debt.remaining
                    )
                } else {
                    format!(
                        "Dernière échéance de la rançon de {name} versée : la dette est soldée."
                    )
                };
                events.push(GameEvent::new(EventKind::Ransom, text).faction(id));
            } else {
                debt.missed += 1;
                debt.remaining += debt.remaining * DEFAULT_SURCHARGE_PERCENT / 100;
                if let Some(ruler) = state.factions[id].ruler.clone() {
                    if let Some(r) = state.characters.get_mut(&ruler) {
                        r.prestige -= DEFAULT_PRESTIGE;
                    }
                }
                state.add_modifier(&debt.creditor, id, DEFAULT_OPINION, "Rançon impayée", 20);
                events.push(
                    GameEvent::new(
                        EventKind::Ransom,
                        format!(
                            "Échéance de la rançon de {name} impayée : la dette monte à {} livres, le prestige du souverain en souffre.",
                            debt.remaining
                        ),
                    )
                    .faction(id),
                );
            }
            debt.next_due_turn = turn + crate::state::TURNS_PER_YEAR;
            if debt.remaining > 0 {
                kept.push(debt);
            }
        }
        state.factions.get_mut(id).expect("listed").ransom_debts = kept;
    }
    // A king in chains: his prestige wanes season after season.
    for id in &ids {
        let Some(ruler) = state.factions[id].ruler.clone() else {
            continue;
        };
        if let Some(r) = state
            .characters
            .get_mut(&ruler)
            .filter(|r| r.alive && r.captive)
        {
            r.prestige -= CAPTIVE_RULER_PRESTIGE;
        }
    }
}

/// Livres a faction still owes in ransoms.
pub fn ransom_debt_total(state: &CampaignState, faction: &FactionId) -> i64 {
    state
        .factions
        .get(faction)
        .map_or(0, |f| f.ransom_debts.iter().map(|d| d.remaining).sum())
}

/// Ransom policy of the AI factions (deterministic, character id order).
///
/// Payer: under money terms, pays in full with twice the sum in the
/// treasury, by four installments for a ruler or heir when the first one
/// leaves half a season of income; cedes a province only for its ruler.
/// Captor: frees plain knights on parole once at peace with their faction.
pub fn ai_ransom_orders(state: &CampaignState, data: &GameData, faction: &FactionId) -> Vec<Order> {
    let mut orders = Vec::new();
    let Some(f) = state.factions.get(faction).filter(|f| f.alive) else {
        return orders;
    };
    let mut treasury = f.treasury;
    // DC3: the income walks every place; weighed only for a captive heir or sovereign.
    let mut income = None;
    for (id, c) in &state.characters {
        if !c.alive || !c.captive {
            continue;
        }
        let Some(captor) = &c.captor else {
            continue;
        };
        if &c.faction == faction {
            let rank = captive_rank(state, id);
            match c.ransom_terms.clone().unwrap_or_default() {
                RansomTerms::Money => {
                    let amount = ransom_amount(state, data, id);
                    let (_, first) = installment_plan(amount, 4);
                    if treasury >= 2 * amount {
                        treasury -= amount;
                        orders.push(Order::PayRansom {
                            character: id.clone(),
                            installments: 1,
                        });
                    } else if matches!(rank, CaptiveRank::Sovereign | CaptiveRank::Heir)
                        && treasury - first
                            >= *income.get_or_insert_with(|| {
                                state.faction_income_effective(data, faction).max(0)
                            }) / 2
                    {
                        treasury -= first;
                        orders.push(Order::PayRansom {
                            character: id.clone(),
                            installments: 4,
                        });
                    }
                }
                RansomTerms::Province { .. } if rank == CaptiveRank::Sovereign => {
                    orders.push(Order::PayRansom {
                        character: id.clone(),
                        installments: 0,
                    });
                }
                _ => {}
            }
        } else if captor == faction
            && c.ransom_terms != Some(RansomTerms::Hold)
            && !state.is_at_war(faction, &c.faction)
            && captive_rank(state, id) == CaptiveRank::Knight
        {
            orders.push(Order::ReleaseOnParole {
                character: id.clone(),
            });
        }
    }
    orders
}
