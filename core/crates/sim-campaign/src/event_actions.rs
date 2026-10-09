//! World mutations applied by event effects and captures: province and title
//! transfers, sale prices, rulers, marriages, prisoners (split from `chronicle`).

use data_model::{CharacterId, FactionId, GameData, ProvinceId};

use crate::events::{EventKind, GameEvent};
use crate::state::CampaignState;

/// G1 `transfer_province`: `province` passes to `faction`, ownership and
/// control (purchase, treaty), through `ransom::cede_province`. Ignored for
/// unknown ids, a dead recipient, a province it already holds, or the
/// capital of its current owner. Returns whether the province changed hands.
pub fn transfer_province(
    state: &mut CampaignState,
    data: &GameData,
    province: &ProvinceId,
    faction: &FactionId,
    events: &mut Vec<GameEvent>,
) -> bool {
    let Some(owner) = state.province_owner(province).cloned() else {
        return false;
    };
    let alive = state.factions.get(faction).is_some_and(|f| f.alive);
    let held = state.holds_province(faction, province);
    let capital = state
        .factions
        .get(&owner)
        .is_some_and(|f| &f.capital == province);
    if !alive || held || capital {
        return false;
    }
    let previous = owner;
    crate::ransom::cede_province(state, &previous, faction, province);
    events.push(
        GameEvent::new(
            EventKind::ProvinceCaptured,
            format!(
                "{} passe de {} à {}.",
                data.province_name(province),
                data.faction_name(&previous),
                data.faction_name(faction)
            ),
        )
        .province(province)
        .faction(faction),
    );
    true
}

/// LR-17 ` (price N livres)` suffix of a sale, naming a payer other than
/// the recipient.
pub(crate) fn price_label(data: &GameData, price: i64, payer: &Option<FactionId>) -> String {
    if price <= 0 {
        return String::new();
    }
    let amount = crate::economy_balance::signed_livres(price);
    let amount = amount.trim_start_matches('+');
    match payer {
        Some(p) => format!(" contre {amount}, payés par {}", data.faction_name(p)),
        None => format!(" contre {amount}"),
    }
}

/// LR-17 `transfer_title`: `title` passes to `faction` through the feudal
/// transfer (its provinces held by the seller follow, capital included; a
/// seller left without title is absorbed). Ignored for an unknown title, a
/// dead recipient or one that already holds it. Returns whether the title
/// changed hands.
pub fn transfer_title(
    state: &mut CampaignState,
    data: &GameData,
    title: &data_model::TitleId,
    faction: &FactionId,
    events: &mut Vec<GameEvent>,
) -> bool {
    if crate::feudal::holder_of(state, title) == Some(faction) {
        return false;
    }
    crate::feudal::transfer_title_into(state, data, title, faction, events).is_ok()
}

/// LR-17: `payer` pays `price` livres for a sale (even into debt); `seller`
/// receives them if it still exists, otherwise the money leaves the map (a
/// crown that is not playable, a seller absorbed by the sale).
pub(crate) fn pay_sale_price(
    state: &mut CampaignState,
    data: &GameData,
    payer: &FactionId,
    seller: Option<&FactionId>,
    price: i64,
    events: &mut Vec<GameEvent>,
) {
    if price <= 0 || seller == Some(payer) {
        return;
    }
    let Some(paying) = state.factions.get_mut(payer).filter(|f| f.alive) else {
        return;
    };
    paying.treasury -= price;
    let receiver = seller.filter(|s| state.factions.get(*s).is_some_and(|f| f.alive));
    if let Some(receiver) = receiver {
        state.factions.get_mut(receiver).expect("alive").treasury += price;
    }
    let amount = crate::economy_balance::signed_livres(price);
    let amount = amount.trim_start_matches('+');
    let text = match receiver {
        Some(r) => format!(
            "{} verse {amount} à {}.",
            data.faction_name(payer),
            data.faction_name(r)
        ),
        None => format!(
            "{} verse {amount} pour son achat.",
            data.faction_name(payer)
        ),
    };
    events.push(GameEvent::new(EventKind::Chronicle, text).faction(payer));
}

/// LR-17 `set_ruler`: `character`, alive and of `faction`, takes its throne
/// (usurpation, election); the former ruler lives on and the heir is picked
/// again by the succession law. Returns whether the ruler changed.
pub fn set_ruler(
    state: &mut CampaignState,
    data: &GameData,
    faction: &FactionId,
    character: &CharacterId,
    events: &mut Vec<GameEvent>,
) -> bool {
    let eligible = state
        .characters
        .get(character)
        .is_some_and(|c| c.alive && &c.faction == faction);
    let Some(realm) = state.factions.get(faction).filter(|f| f.alive) else {
        return false;
    };
    if !eligible || realm.ruler.as_ref() == Some(character) {
        return false;
    }
    let deposed = realm.ruler.clone();
    let realm = state.factions.get_mut(faction).expect("alive");
    realm.ruler = Some(character.clone());
    realm.heir = None;
    let heir = crate::dynasty::pick_heir_by_law(state, data, faction, character);
    state.factions.get_mut(faction).expect("alive").heir = heir;
    let deposed = deposed
        .filter(|d| state.characters.get(d).is_some_and(|c| c.alive))
        .map(|d| format!(" ; {} est écarté", state.character_name(data, &d)))
        .unwrap_or_default();
    events.push(
        GameEvent::new(
            EventKind::Succession,
            format!(
                "{} prend la tête de {}{deposed}.",
                state.character_name(data, character),
                data.faction_name(faction)
            ),
        )
        .faction(faction),
    );
    true
}

/// F1 `capture_character`: `id` becomes the prisoner of `captor` (it
/// leaves its army and its governorship; a ruler keeps the crown).
pub fn capture_character(
    state: &mut CampaignState,
    data: &GameData,
    id: &CharacterId,
    captor: &FactionId,
    events: &mut Vec<GameEvent>,
) {
    let Some(c) = state.characters.get(id).filter(|c| c.alive && !c.captive) else {
        return;
    };
    if &c.faction == captor || !state.factions.contains_key(captor) {
        return;
    }
    let owner = c.faction.clone();
    state.detach_general(id);
    let c = state.characters.get_mut(id).expect("checked above");
    c.captive = true;
    c.captor = Some(captor.clone());
    c.governor_of = None;
    c.location = state.factions.get(captor).map(|f| f.capital.clone());
    events.push(
        GameEvent::new(
            EventKind::GeneralCaptured,
            format!(
                "{} est retenu prisonnier par {}.",
                state.character_name(data, id),
                data.faction_name(captor)
            ),
        )
        .faction(&owner),
    );
}

/// F1 `release_character`: a captive is freed against `ransom` livres paid
/// by its faction to its captor, and returns to its capital.
pub fn release_character(
    state: &mut CampaignState,
    data: &GameData,
    id: &CharacterId,
    ransom: i64,
    events: &mut Vec<GameEvent>,
) {
    let Some(c) = state.characters.get(id).filter(|c| c.alive && c.captive) else {
        return;
    };
    let owner = c.faction.clone();
    let captor = c.captor.clone();
    let ransom = ransom.max(0);
    if ransom > 0 {
        if let Some(f) = state.factions.get_mut(&owner) {
            f.treasury -= ransom;
        }
        if let Some(f) = captor.as_ref().and_then(|c| state.factions.get_mut(c)) {
            f.treasury += ransom;
        }
        // C7: a Lombard banker may follow the money to the captor's ruler.
        if let Some(captor) = &captor {
            crate::retinue::on_ransom_received(state, data, captor, events);
        }
    }
    let capital = state.factions.get(&owner).map(|f| f.capital.clone());
    let c = state.characters.get_mut(id).expect("checked above");
    c.captive = false;
    c.captor = None;
    c.ransom_terms = None;
    c.location = capital;
    crate::dynasty::on_ransomed(state, data, id);
    let text = if ransom > 0 {
        format!(
            "{} est libéré contre une rançon de {ransom} livres.",
            state.character_name(data, id)
        )
    } else {
        format!("{} est libéré.", state.character_name(data, id))
    };
    events.push(GameEvent::new(EventKind::Chronicle, text).faction(&owner));
}

/// F1 `marry`: a historical marriage between two living, unmarried
/// characters (no-op otherwise: history diverged).
pub(crate) fn marry(
    state: &mut CampaignState,
    data: &GameData,
    a: &CharacterId,
    b: &CharacterId,
    events: &mut Vec<GameEvent>,
) {
    let free = |id: &CharacterId| {
        state
            .characters
            .get(id)
            .is_some_and(|c| c.alive && c.spouse.is_none())
    };
    if a == b || !free(a) || !free(b) {
        return;
    }
    for (x, y) in [(a, b), (b, a)] {
        let c = state.characters.get_mut(x).expect("checked above");
        c.spouse = Some(y.clone());
        c.prestige += crate::dynasty::rules().prestige_marriage;
    }
    let faction = state.characters[a].faction.clone();
    events.push(
        GameEvent::new(
            EventKind::Chronicle,
            format!(
                "Mariage de {} et de {}.",
                state.character_name(data, a),
                state.character_name(data, b)
            ),
        )
        .faction(&faction),
    );
}
