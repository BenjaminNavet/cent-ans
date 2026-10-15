//! Diplomacy screen views (ADR 0357): faction list, relation detail with the opinion
//! breakdown, treaty options and clause menu, acceptance estimate of a draft treaty, the
//! unilateral actions with their consequences, offers received, and the submission of an order.
//!
//! Port of `godot-bridge` `campaign_sim_diplomacy.rs`, `campaign_sim_treaty.rs`,
//! `campaign_sim_dp2.rs`. No rule here: everything comes from `sim-campaign`.

use std::collections::BTreeMap;

use data_model::{FactionId, GameData, SettlementKind};
use serde::Serialize;
use sim_campaign::diplomacy::{call_to_arms_forecast, CallForecast, RelationKind};
use sim_campaign::negotiation::{
    self, counter_proposal, evaluate_treaty, is_war_goal, ruler_name, Article, Party,
    COUNTER_TARGET_CHANCE,
};
use sim_campaign::plan_cache::PlanCache;
use sim_campaign::religion::{faction_religion, religion_display};
use sim_campaign::stance::{diplomatic_stance, diplomatic_stance_cached};
use sim_campaign::treaty_explain::explain_treaty;
use sim_campaign::{CampaignState, Order, Season};

use crate::common::{faction_refs, FactionRef};

/// Gift sent by the "Présents" action (livres).
pub const GIFT_AMOUNT: i64 = 1000;
/// Length (seasons) of the truce clause of the menu.
pub const TRUCE_TURNS: u32 = 8;

/// `{text, value, turns_left}`: one weighted reason (`turns_left` 0: permanent).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ReasonView {
    pub text: String,
    pub value: i32,
    pub turns_left: u32,
}

/// One other faction seen from the player: a row of the list and the head of the sheet.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FactionRow {
    pub id: String,
    pub name: String,
    pub color: String,
    pub ruler: String,
    /// Relation key: `war`, `truce`, `peace`, `alliance`, `vassal`, `suzerain`.
    pub status: String,
    /// Diplomatic stance key (`ally`, `agreement`, `neutral`, `tension`, `war`, `vassal`).
    pub stance: String,
    pub stance_label: String,
    /// Their attitude towards the player.
    pub attitude: i32,
    pub attitude_band: String,
    /// Reasons of the attitude, heaviest first.
    pub reasons: Vec<ReasonView>,
    pub allies: Vec<FactionRef>,
    pub enemies: Vec<FactionRef>,
    pub vassals: Vec<FactionRef>,
    pub truce_turns_left: u32,
    pub embargo_by_us: bool,
    pub embargo_on_us: bool,
    pub war_score: i32,
    pub casus_belli: String,
    pub claims: Vec<String>,
    pub religion_name: String,
    pub loyalty: Option<u8>,
    pub power: i64,
    pub weariness: u32,
    pub trade_agreement: bool,
    pub access_given: bool,
    pub access_received: bool,
    /// `""`, `defensive` or `military`.
    pub alliance_kind: String,
    pub non_aggression_turns_left: u32,
    /// Shares a land border with the player.
    pub neighbour: bool,
    /// Target of the league of princes.
    pub hegemon: bool,
}

/// Every other living faction seen from `player` (sort and filter are the client's).
pub fn faction_list(data: &GameData, state: &CampaignState, player: &FactionId) -> Vec<FactionRow> {
    if !state.factions.contains_key(player) {
        return Vec::new();
    }
    let plan = PlanCache::new(state);
    let neighbours = state.neighbour_factions(data, player);
    state
        .diplomacy_view(data, player)
        .iter()
        .map(|entry| {
            let definition = data.factions.get(&entry.faction);
            let religion = faction_religion(state, data, &entry.faction);
            let stance = diplomatic_stance_cached(&plan, data, player, &entry.faction);
            let mut reasons: Vec<ReasonView> = entry
                .attitude_reasons
                .iter()
                .map(|(text, value)| ReasonView {
                    text: text.clone(),
                    value: *value,
                    turns_left: entry.reason_turns.get(text).copied().unwrap_or(0),
                })
                .collect();
            reasons.sort_by_key(|r| std::cmp::Reverse(r.value.abs()));
            let me = &state.factions[player];
            let them = &state.factions[&entry.faction];
            FactionRow {
                id: entry.faction.as_str().to_owned(),
                name: definition.map_or_else(
                    || entry.faction.as_str().to_owned(),
                    |f| f.short_or_display_name().to_owned(),
                ),
                color: definition.map_or_else(
                    || "#888888".to_owned(),
                    |f| f.heraldry.primary_color.clone(),
                ),
                ruler: ruler_name(state, data, &entry.faction),
                status: entry.relation.key().to_owned(),
                stance: stance.key().to_owned(),
                stance_label: stance.label_fr().to_owned(),
                attitude: entry.attitude,
                attitude_band: entry.attitude_band.clone(),
                reasons,
                allies: faction_refs(data, &entry.allies),
                enemies: faction_refs(data, &entry.enemies),
                vassals: faction_refs(data, &entry.vassals),
                truce_turns_left: entry.truce_turns_left,
                embargo_by_us: entry.embargo_by_us,
                embargo_on_us: entry.embargo_on_us,
                war_score: entry.war_score,
                casus_belli: entry.casus_belli.clone().unwrap_or_default(),
                claims: entry.claims.clone(),
                religion_name: religion
                    .as_ref()
                    .map_or_else(String::new, |r| religion_display(state, data, r)),
                loyalty: entry.loyalty,
                power: state.faction_power(&entry.faction).round() as i64,
                weariness: negotiation::weariness(state, &entry.faction),
                trade_agreement: me.ledger.trade_agreements.contains(&entry.faction),
                access_given: me.ledger.military_access.contains(&entry.faction),
                access_received: them.ledger.military_access.contains(player),
                alliance_kind: if entry.relation != RelationKind::Alliance {
                    ""
                } else if state.is_defensive_alliance(player, &entry.faction) {
                    "defensive"
                } else {
                    "military"
                }
                .to_owned(),
                non_aggression_turns_left: me
                    .ledger
                    .non_aggression
                    .get(&entry.faction)
                    .map_or(0, |until| until.saturating_sub(state.turn())),
                neighbour: neighbours.contains(&entry.faction),
                hegemon: state
                    .league
                    .as_ref()
                    .is_some_and(|l| l.target == entry.faction),
            }
        })
        .collect()
}

/// Trespass of one side on the other's lands.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TrespassSide {
    pub seasons: u32,
    pub total: u32,
    pub grievance: bool,
    pub provinces: Vec<String>,
}

/// Trespass between the player and a faction, both ways.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TrespassView {
    pub theirs: TrespassSide,
    pub ours: TrespassSide,
}

/// A province offered in the treaty menus.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProvinceOption {
    pub id: String,
    pub name: String,
    pub capital: bool,
    /// Held by the other side.
    pub occupied: bool,
    pub war_goal: bool,
}

/// A secondary settlement offered in the treaty menus.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SettlementOption {
    pub id: String,
    pub name: String,
    pub province: String,
    pub occupied: bool,
}

/// A character offered (captive, hostage, spouse).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CharacterOption {
    pub id: String,
    pub name: String,
    pub female: bool,
}

/// What one side can put on the table.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TreatySide {
    pub id: String,
    pub name: String,
    pub ruler: String,
    pub treasury: i64,
    pub income: i64,
    pub weariness: u32,
    pub provinces: Vec<ProvinceOption>,
    pub settlements: Vec<SettlementOption>,
    pub captives: Vec<CharacterOption>,
    pub hostages: Vec<CharacterOption>,
    pub marriageable: Vec<CharacterOption>,
    pub enemies: Vec<FactionRef>,
}

/// Where a clause of the menu goes: common clauses, what the player offers, what they ask.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ClauseSlot {
    Common,
    Offer,
    Demand,
}

/// One entry of the clause menu (« + Clause », « + Ajouter »).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ClauseChoice {
    pub slot: ClauseSlot,
    /// Submenu (`""` for a top-level entry).
    pub group: String,
    pub label: String,
    pub article: Article,
}

/// Standing state and what each side can bring to a treaty with a faction.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TreatyOptions {
    pub at_war: bool,
    pub allied: bool,
    pub defensive_alliance: bool,
    pub non_aggression: bool,
    pub trade: bool,
    pub access_given: bool,
    pub access_received: bool,
    pub war_score: i32,
    pub ours: TreatySide,
    pub theirs: TreatySide,
    pub clauses: Vec<ClauseChoice>,
}

/// An unilateral action of the sheet, with its consequences when the sim can foresee them.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ActionView {
    /// Stable key for widget names (`declare_war`, `set_embargo`, ...).
    pub key: String,
    pub label: String,
    pub success_text: String,
    pub order: Order,
    pub preview: Option<Preview>,
}

/// Consequences of a declaration of war or a mediation request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Preview {
    pub accept: bool,
    pub score: i32,
    pub reasons: Vec<ReasonView>,
    /// How each ally of the target will answer a war declaration.
    pub allies: Vec<AllyForecast>,
}

/// Answer of an ally of the target to a call to arms.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AllyForecast {
    pub id: String,
    pub name: String,
    /// `joins`, `hesitates`, `refuses`.
    pub answer: String,
    pub text: String,
}

/// War between the player and a faction.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct WarView {
    pub war_score: i32,
    pub weariness_ours: u32,
    pub weariness_theirs: u32,
    pub goals_ours: Vec<GoalView>,
    pub goals_theirs: Vec<GoalView>,
}

/// A war goal and whether its holder controls it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GoalView {
    pub id: String,
    pub name: String,
    pub held: bool,
}

/// A treaty signed or refused.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TreatyRecordView {
    pub turn: u32,
    pub date: String,
    pub accepted: bool,
    pub proposed: bool,
    /// `broken`, `perjury`, `refused_call` or empty.
    pub rupture: String,
    pub text: String,
}

/// Everything the right column of the screen shows for one faction.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RelationDetail {
    pub row: FactionRow,
    pub trespass: TrespassView,
    pub war: Option<WarView>,
    pub history: Vec<TreatyRecordView>,
    pub options: TreatyOptions,
    pub actions: Vec<ActionView>,
}

/// Detail of the relation of `player` with `target`, `None` for an unknown or dead faction.
pub fn relation_detail(
    data: &GameData,
    state: &CampaignState,
    player: &FactionId,
    target: &FactionId,
) -> Option<RelationDetail> {
    let row = faction_list(data, state, player)
        .into_iter()
        .find(|r| r.id == target.as_str())?;
    let options = treaty_options(data, state, player, target)?;
    let actions = actions_for(data, state, player, target, &row);
    let war = (row.status == "war").then(|| war_view(data, state, player, target));
    let history = state.factions[player]
        .ledger
        .history
        .iter()
        .rev()
        .filter(|r| &r.with == target)
        .map(|r| TreatyRecordView {
            turn: r.turn,
            date: date_of(state, r.turn),
            accepted: r.accepted,
            proposed: r.proposed,
            rupture: r.rupture.map_or("", |k| k.key()).to_owned(),
            text: r.text_fr.clone(),
        })
        .collect();
    Some(RelationDetail {
        trespass: trespass_view(data, state, player, target),
        row,
        war,
        history,
        options,
        actions,
    })
}

fn trespass_view(
    data: &GameData,
    state: &CampaignState,
    player: &FactionId,
    other: &FactionId,
) -> TrespassView {
    let side = |victim: &FactionId, intruder: &FactionId| {
        let record = state.factions[victim].ledger.trespassers.get(intruder);
        TrespassSide {
            seasons: record.map_or(0, |t| t.seasons),
            total: record.map_or(0, |t| t.total),
            grievance: sim_campaign::passage::has_grievance(state, victim, intruder),
            provinces: record
                .map(|t| {
                    t.provinces
                        .iter()
                        .map(|p| data.province_name(p).to_owned())
                        .collect()
                })
                .unwrap_or_default(),
        }
    };
    TrespassView {
        theirs: side(player, other),
        ours: side(other, player),
    }
}

fn war_view(
    data: &GameData,
    state: &CampaignState,
    player: &FactionId,
    enemy: &FactionId,
) -> WarView {
    let goals = |a: &FactionId, b: &FactionId| -> Vec<GoalView> {
        state.factions[a]
            .ledger
            .war_goals
            .get(b)
            .map(|list| {
                list.iter()
                    .map(|p| GoalView {
                        id: p.as_str().to_owned(),
                        name: data.province_name(p).to_owned(),
                        held: state.controls_province(a, p),
                    })
                    .collect()
            })
            .unwrap_or_default()
    };
    WarView {
        war_score: state.war_score(data, player, enemy),
        weariness_ours: negotiation::weariness(state, player),
        weariness_theirs: negotiation::weariness(state, enemy),
        goals_ours: goals(player, enemy),
        goals_theirs: goals(enemy, player),
    }
}

/// Season label of a past `turn`, as `get_treaty_history` did.
fn date_of(state: &CampaignState, turn: u32) -> String {
    let now = Season::ALL
        .iter()
        .position(|s| *s == state.season())
        .unwrap_or(0) as i64;
    let absolute = i64::from(state.year()) * 4 + now - i64::from(state.turn()) + i64::from(turn);
    let season = Season::ALL[absolute.rem_euclid(4) as usize];
    format!("{} {}", season.label_fr(), absolute.div_euclid(4))
}

/// Treaty options between the player and `target`, `None` when either is unknown.
pub fn treaty_options(
    data: &GameData,
    state: &CampaignState,
    player: &FactionId,
    target: &FactionId,
) -> Option<TreatyOptions> {
    if !state.factions.contains_key(target) || !state.factions.contains_key(player) {
        return None;
    }
    let me = &state.factions[player];
    let at_war = state.is_at_war(player, target);
    let mut options = TreatyOptions {
        at_war,
        allied: state.is_allied(player, target),
        defensive_alliance: state.is_defensive_alliance(player, target),
        non_aggression: state.has_non_aggression(player, target),
        trade: me.ledger.trade_agreements.contains(target),
        access_given: me.ledger.military_access.contains(target),
        access_received: state.factions[target]
            .ledger
            .military_access
            .contains(player),
        war_score: if at_war {
            state.war_score(data, player, target)
        } else {
            0
        },
        ours: side_options(state, data, player, target),
        theirs: side_options(state, data, target, player),
        clauses: Vec::new(),
    };
    options.clauses = clause_menu(data, &options);
    Some(options)
}

fn character_option(
    state: &CampaignState,
    data: &GameData,
    id: &data_model::CharacterId,
    female: bool,
) -> CharacterOption {
    CharacterOption {
        id: id.as_str().to_owned(),
        name: state.character_name(data, id),
        female,
    }
}

fn side_options(
    state: &CampaignState,
    data: &GameData,
    side: &FactionId,
    other: &FactionId,
) -> TreatySide {
    let f = &state.factions[side];
    let mut owned = state.owned_provinces(side);
    owned.sort();
    let provinces = owned
        .iter()
        .map(|p| ProvinceOption {
            id: p.as_str().to_owned(),
            name: data.province_name(p).to_owned(),
            capital: &f.capital == p,
            occupied: state.controls_province(other, p),
            war_goal: is_war_goal(state, other, side, p),
        })
        .collect();
    let settlements = state
        .settlements
        .iter()
        .filter(|(_, s)| &s.owner == side && s.kind != SettlementKind::City)
        .map(|(id, s)| SettlementOption {
            id: id.as_str().to_owned(),
            name: data.settlement_name(id),
            province: data.province_name(&s.province).to_owned(),
            occupied: &s.controller == other,
        })
        .collect();
    let captives = state
        .characters
        .iter()
        .filter(|(_, c)| {
            c.alive && c.captive && c.captor.as_ref() == Some(side) && &c.faction == other
        })
        .map(|(id, _)| character_option(state, data, id, false))
        .collect();
    let hostages = state
        .characters
        .iter()
        .filter(|(id, c)| {
            c.alive
                && !c.captive
                && &c.faction == side
                && c.army.is_none()
                && f.ruler.as_ref() != Some(*id)
                && state.year() - c.birth_year >= 12
        })
        .take(12)
        .map(|(id, _)| character_option(state, data, id, false))
        .collect();
    let marriageable = state
        .characters
        .iter()
        .filter(|(_, c)| {
            c.alive
                && !c.captive
                && &c.faction == side
                && c.spouse.is_none()
                && (14..=45).contains(&(state.year() - c.birth_year))
        })
        .take(12)
        .map(|(id, c)| character_option(state, data, id, c.sex == data_model::Sex::Female))
        .collect();
    let enemies: Vec<FactionId> = f
        .at_war_with
        .iter()
        .filter(|e| *e != other && !e.is_rebels())
        .filter(|e| state.factions.get(*e).is_some_and(|x| x.alive))
        .cloned()
        .collect();
    TreatySide {
        id: side.as_str().to_owned(),
        name: data.faction_name(side),
        ruler: ruler_name(state, data, side),
        treasury: f.treasury,
        income: f.last_budget.income,
        weariness: f.ledger.weariness,
        provinces,
        settlements,
        captives,
        hostages,
        marriageable,
        enemies: faction_refs(data, &enemies),
    }
}

/// The clause menu, as `DiplomacyNegotiationTab.fill_menu` built it.
fn clause_menu(data: &GameData, o: &TreatyOptions) -> Vec<ClauseChoice> {
    let mut out = Vec::new();
    let mut push = |slot, group: &str, label: String, article| {
        out.push(ClauseChoice {
            slot,
            group: group.to_owned(),
            label,
            article,
        });
    };
    use ClauseSlot::{Common, Demand, Offer};
    if o.at_war {
        push(Common, "", "Paix".into(), Article::Peace);
        push(
            Common,
            "",
            "Trêve de deux ans".into(),
            Article::Truce { turns: TRUCE_TURNS },
        );
    }
    if !o.allied || o.defensive_alliance {
        push(Common, "", "Alliance militaire".into(), Article::Alliance);
    }
    if !o.allied {
        push(
            Common,
            "",
            "Alliance défensive".into(),
            Article::DefensiveAlliance,
        );
        if !o.at_war && !o.non_aggression {
            for turns in &data.ai_diplomacy.non_aggression.durations {
                push(
                    Common,
                    "Pacte de non-agression",
                    format!("{} ans", turns / 4),
                    Article::NonAggression { turns: *turns },
                );
            }
        }
    }
    if !o.trade {
        push(
            Common,
            "",
            "Accord commercial".into(),
            Article::TradeAgreement,
        );
    }
    let mut pairs = 0;
    for a in &o.ours.marriageable {
        for b in &o.theirs.marriageable {
            if a.female != b.female && pairs < 10 {
                pairs += 1;
                push(
                    Common,
                    "Mariage",
                    format!("{} et {}", a.name, b.name),
                    Article::Marriage {
                        character: parse_id(&a.id),
                        spouse: parse_id(&b.id),
                    },
                );
            }
        }
    }
    for (slot, giver, side, other_side) in [
        (Offer, Party::Proposer, &o.ours, &o.theirs),
        (Demand, Party::Recipient, &o.theirs, &o.ours),
    ] {
        let access_in_force = if giver == Party::Proposer {
            o.access_given
        } else {
            o.access_received
        };
        if !access_in_force {
            push(
                slot,
                "",
                "Accès militaire".into(),
                Article::MilitaryAccess { giver },
            );
        }
        let occupied_by = if giver == Party::Proposer {
            "occupée par eux"
        } else {
            "occupée par nous"
        };
        for p in &side.provinces {
            let mut label = p.name.clone();
            if p.capital {
                label.push_str(" (capitale)");
            }
            if p.occupied {
                label.push_str(&format!(" — {occupied_by}"));
            }
            if p.war_goal {
                label = format!("★ {label}");
            }
            push(
                slot,
                "Province",
                label,
                Article::CedeProvince {
                    giver,
                    province: parse_id(&p.id),
                },
            );
        }
        for s in side.settlements.iter().take(24) {
            push(
                slot,
                "Place forte ou colonie",
                format!("{} ({})", s.name, s.province),
                Article::CedeSettlement {
                    giver,
                    settlement: parse_id(&s.id),
                },
            );
        }
        for c in &side.captives {
            push(
                slot,
                "Libérer un captif",
                c.name.clone(),
                Article::ReleaseCaptive {
                    giver,
                    character: parse_id(&c.id),
                },
            );
        }
        for c in &side.hostages {
            push(
                slot,
                "Otage",
                c.name.clone(),
                Article::Hostage {
                    giver,
                    character: parse_id(&c.id),
                },
            );
        }
        // « Rejoindre la guerre contre X »: the giver goes to war against an enemy of the other.
        let enemies = &other_side.enemies;
        let group = if giver == Party::Recipient {
            "Rejoindre la guerre contre…"
        } else {
            "Rejoindre leur guerre contre…"
        };
        for enemy in enemies {
            push(
                slot,
                group,
                enemy.name.clone(),
                Article::JoinWar {
                    giver,
                    target: parse_id(&enemy.id),
                },
            );
        }
        push(
            slot,
            "",
            if giver == Party::Proposer {
                "Devenir vassal"
            } else {
                "Vassalité"
            }
            .into(),
            Article::Vassalage { giver },
        );
    }
    out
}

/// Identifier types built from the ids the sim itself produced.
trait FromSimId: Sized {
    fn from_sim_id(id: &str) -> Self;
}

macro_rules! from_sim_id {
    ($($t:ty),*) => {$(
        impl FromSimId for $t {
            fn from_sim_id(id: &str) -> Self {
                <$t>::new(id.to_owned()).expect("identifier produced by the sim")
            }
        }
    )*};
}
from_sim_id!(
    data_model::CharacterId,
    data_model::ProvinceId,
    data_model::SettlementId,
    FactionId
);

fn parse_id<T: FromSimId>(id: &str) -> T {
    T::from_sim_id(id)
}

/// Unilateral actions of the sheet (`DiplomacyActionsSection._render`).
fn actions_for(
    data: &GameData,
    state: &CampaignState,
    player: &FactionId,
    target: &FactionId,
    row: &FactionRow,
) -> Vec<ActionView> {
    let mut out = Vec::new();
    let mut add = |key: &str, label: String, success: &str, order: Order, preview| {
        out.push(ActionView {
            key: key.to_owned(),
            label,
            success_text: success.to_owned(),
            order,
            preview,
        });
    };
    let status = row.status.as_str();
    let target = target.clone();
    if status == "war" {
        let order = Order::RequestPapalMediation {
            target: target.clone(),
        };
        let preview = Some(preview_order(data, state, player, &order));
        add(
            "papal_mediation",
            "Médiation pontificale".into(),
            "Le pape obtient une trêve.",
            order,
            preview,
        );
    } else if !matches!(status, "alliance" | "vassal" | "suzerain") {
        let order = Order::DeclareWar {
            target: target.clone(),
        };
        let preview = Some(preview_order(data, state, player, &order));
        add(
            "declare_war",
            "Déclarer la guerre".into(),
            "La guerre est déclarée.",
            order,
            preview,
        );
    }
    if status == "alliance" {
        add(
            "break_alliance",
            "Rompre l’alliance".into(),
            "Alliance rompue.",
            Order::BreakAlliance {
                target: target.clone(),
            },
            None,
        );
    }
    if row.access_given {
        add(
            "revoke_access",
            "Retirer l’accès militaire".into(),
            "Accès militaire retiré.",
            Order::RevokeMilitaryAccess {
                target: target.clone(),
            },
            None,
        );
    }
    if status == "vassal" {
        add(
            "release_vassal",
            "Libérer le vassal".into(),
            "Vassal libéré.",
            Order::ReleaseVassal {
                target: target.clone(),
            },
            None,
        );
    }
    let embargo = row.embargo_by_us;
    add(
        "embargo",
        if embargo {
            "Lever l’embargo"
        } else {
            "Imposer un embargo"
        }
        .into(),
        if embargo {
            "Embargo levé."
        } else {
            "Embargo imposé."
        },
        Order::SetEmbargo {
            target: target.clone(),
            active: !embargo,
        },
        None,
    );
    add(
        "gift",
        format!("Présents ({GIFT_AMOUNT} ₶)"),
        "Présents envoyés.",
        Order::SendGift {
            target: target.clone(),
            amount: GIFT_AMOUNT,
        },
        None,
    );
    if row.trade_agreement {
        add(
            "break_trade",
            "Rompre l’accord commercial".into(),
            "Accord commercial rompu.",
            Order::BreakTradeAgreement { target },
            None,
        );
    }
    out
}

/// Acceptance estimate of a diplomatic order before sending it (`evaluate_proposal`); for a
/// declaration of war the reasons list the motive, the reputation cost and the allies' answers.
pub fn preview_order(
    data: &GameData,
    state: &CampaignState,
    player: &FactionId,
    order: &Order,
) -> Preview {
    let plain = |accept, score, reasons: Vec<(String, i32)>| Preview {
        accept,
        score,
        reasons: reasons
            .into_iter()
            .map(|(text, value)| ReasonView {
                text,
                value,
                turns_left: 0,
            })
            .collect(),
        allies: Vec::new(),
    };
    let (target, articles) = match order {
        Order::DeclareWar { target } => return war_preview(data, state, player, target),
        Order::RequestPapalMediation { target } => (
            target.clone(),
            vec![Article::Mediation {
                turns: sim_campaign::diplomacy::MEDIATION_TRUCE_TURNS,
            }],
        ),
        other => match other.proposal(state, player) {
            Some((target, treaty)) => (target, treaty.articles),
            None => return plain(true, 0, Vec::new()),
        },
    };
    let evaluation = evaluate_treaty(state, data, player, &target, &articles);
    plain(
        evaluation.accept,
        evaluation.score,
        evaluation.reasons().to_vec(),
    )
}

fn war_preview(
    data: &GameData,
    state: &CampaignState,
    player: &FactionId,
    target: &FactionId,
) -> Preview {
    let mut reasons: Vec<(String, i32)> = Vec::new();
    if state.is_at_war(player, target) {
        return Preview {
            accept: false,
            score: 0,
            reasons: vec![ReasonView {
                text: "Déjà en guerre".into(),
                value: 0,
                turns_left: 0,
            }],
            allies: Vec::new(),
        };
    }
    if state.has_truce(player, target) {
        reasons.push((
            "Trêve rompue : parjure (-40 auprès de tous)".to_owned(),
            -40,
        ));
    } else if state.has_non_aggression(player, target) {
        reasons.push((
            "Pacte de non-agression rompu : parjure (-40 auprès de tous)".to_owned(),
            -40,
        ));
    }
    match state.casus_belli(data, player, target) {
        Some(motive) => reasons.push((format!("Motif : {motive}"), 0)),
        None => reasons.push(("Sans motif : réputation -20 auprès de tous".to_owned(), -20)),
    }
    let defender = &state.factions[target];
    let mut called: Vec<FactionId> = defender.allies.iter().cloned().collect();
    called.extend(defender.suzerain.iter().cloned());
    called.sort();
    called.dedup();
    let mut allies = Vec::new();
    for ally in called
        .iter()
        .filter(|a| *a != player && state.factions.get(*a).is_some_and(|f| f.alive))
        .filter(|a| !state.is_at_war(a, player) && !state.is_allied(a, player))
    {
        let name = data.faction_name(ally);
        let (answer, line) = match call_to_arms_forecast(state, data, ally, target, player) {
            CallForecast::Joins => ("joins", format!("{name} viendra à son secours")),
            CallForecast::Hesitates => ("hesitates", format!("{name} hésite")),
            CallForecast::Refuses(why) => ("refuses", format!("{name} refusera : {why}")),
        };
        reasons.push((line.clone(), 0));
        allies.push(AllyForecast {
            id: ally.as_str().to_owned(),
            name,
            answer: answer.to_owned(),
            text: line,
        });
    }
    let prestige = state.declaration_prestige_cost(data, player, target);
    reasons.push((
        if prestige == 0 {
            "Prestige du souverain : inchangé".to_owned()
        } else {
            format!("Prestige du souverain : {prestige}")
        },
        0,
    ));
    let score = reasons.iter().map(|(_, v)| v).sum();
    Preview {
        accept: true,
        score,
        reasons: reasons
            .into_iter()
            .map(|(text, value)| ReasonView {
                text,
                value,
                turns_left: 0,
            })
            .collect(),
        allies,
    }
}

/// A weighted line of the recipient's reasoning; `article` is the index of the clause it
/// belongs to (`None`: a general consideration).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct VerdictLine {
    pub text: String,
    pub value: i32,
    pub article: Option<usize>,
}

/// Their verdict on a draft treaty: score, readable reasoning, and the counter-offer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TreatyVerdict {
    pub chance: u8,
    /// Would accept (score >= 0 and nothing blocked).
    pub accept: bool,
    pub score: i32,
    pub summary: String,
    /// Why a clause is impossible, `""` otherwise.
    pub blocked: String,
    /// Value for them of each clause, in draft order.
    pub clause_values: Vec<i32>,
    /// French label of each clause, in draft order.
    pub clause_labels: Vec<String>,
    pub lines: Vec<VerdictLine>,
    /// Single blocking point, if any.
    pub blocker: Option<VerdictLine>,
    /// Their counter-offer (empty if none), its chance and text.
    pub counter: Vec<Article>,
    pub counter_chance: u8,
    pub counter_text: String,
}

/// Verdict of `target` on `articles` proposed by the player; `None` for an unknown faction.
pub fn evaluate_draft(
    data: &GameData,
    state: &CampaignState,
    player: &FactionId,
    target: &FactionId,
    articles: &[Article],
) -> Option<TreatyVerdict> {
    if !state.factions.contains_key(target) || articles.is_empty() {
        return None;
    }
    let evaluation = evaluate_treaty(state, data, player, target, articles);
    let explanation = explain_treaty(state, data, player, target, articles);
    let line = |l: &sim_campaign::treaty_explain::ExplanationLine| VerdictLine {
        text: l.text.clone(),
        value: l.value,
        article: l.article,
    };
    Some(TreatyVerdict {
        chance: evaluation.chance,
        accept: evaluation.accept,
        score: evaluation.score,
        summary: explanation.summary.clone(),
        blocked: evaluation.blocked.clone().unwrap_or_default(),
        clause_values: evaluation.articles.iter().map(|a| a.value).collect(),
        clause_labels: evaluation
            .articles
            .iter()
            .map(|a| a.label.clone())
            .collect(),
        lines: explanation.lines.iter().map(line).collect(),
        blocker: explanation.blocker.as_ref().map(|b| VerdictLine {
            text: b.text.clone(),
            value: b.value,
            article: b.article,
        }),
        counter: explanation.counter.clone().unwrap_or_default(),
        counter_chance: explanation.counter_chance,
        counter_text: explanation.counter_text.clone(),
    })
}

/// « Que faudrait-il ? »: what `target` would need to accept, or a French reason why not.
pub fn ask_counter(
    data: &GameData,
    state: &CampaignState,
    player: &FactionId,
    target: &FactionId,
    articles: &[Article],
) -> Result<Vec<Article>, String> {
    if !state.factions.contains_key(target) {
        return Err(format!("faction inconnue : {target}"));
    }
    let current = evaluate_treaty(state, data, player, target, articles);
    if current.blocked.is_none() && current.chance >= COUNTER_TARGET_CHANCE {
        return Err(format!(
            "Rien à ajouter : ils accepteraient déjà en l'état (score {:+}).",
            current.score
        ));
    }
    counter_proposal(state, data, player, target, articles)
        .ok_or_else(|| "Rien de ce que vous pouvez offrir ne suffirait.".to_owned())
}

/// An offer waiting for the player's answer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct OfferView {
    pub id: u32,
    pub from: String,
    pub from_name: String,
    pub kind: String,
    pub ultimatum: bool,
    pub text: String,
    pub expires_in: u32,
}

/// Offers waiting for the player's answer.
pub fn offers(data: &GameData, state: &CampaignState, player: &FactionId) -> Vec<OfferView> {
    state
        .valid_offers(data, player)
        .into_iter()
        .map(|offer| OfferView {
            id: offer.id,
            from: offer.from.as_str().to_owned(),
            from_name: data.factions.get(&offer.from).map_or_else(
                || offer.from.as_str().to_owned(),
                |f| f.short_or_display_name().to_owned(),
            ),
            kind: offer.proposal.kind().to_owned(),
            ultimatum: offer.proposal.is_ultimatum(),
            text: offer.text_fr.clone(),
            expires_in: offer
                .expires_turn
                .saturating_sub(state.turn())
                .saturating_sub(1),
        })
        .collect()
}

/// Result of an order sent through the sim.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct OrderResult {
    pub ok: bool,
    /// French error when `ok` is false, empty otherwise.
    pub message: String,
}

/// Submits a diplomatic order of the player exactly like `CampaignSim.submit_order`
/// (`propose_treaty`, `declare_war`, `set_embargo`, ...). A refused treaty comes back as an
/// error carrying the sim's French answer.
pub fn submit_order(data: &GameData, state: &mut CampaignState, order: Order) -> OrderResult {
    match state.submit_order(data, order) {
        Ok(()) => OrderResult {
            ok: true,
            message: String::new(),
        },
        Err(error) => OrderResult {
            ok: false,
            message: error.to_string(),
        },
    }
}

/// Answers an offer of the player (`answer_offer`).
pub fn answer_offer(
    data: &GameData,
    state: &mut CampaignState,
    offer: u32,
    accept: bool,
) -> OrderResult {
    submit_order(data, state, Order::AnswerOffer { offer, accept })
}

/// Stance of the player towards `faction` (`key`, `label`).
pub fn faction_stance(
    data: &GameData,
    state: &CampaignState,
    player: &FactionId,
    faction: &FactionId,
) -> Option<(String, String)> {
    state.factions.contains_key(faction).then(|| {
        let stance = diplomatic_stance(state, data, player, faction);
        (stance.key().to_owned(), stance.label_fr().to_owned())
    })
}

/// Truces and non-aggression pacts ending within this many turns are listed on the court banner.
pub const COURT_EXPIRY_HORIZON: u32 = 2;

/// Lines of the « Affaires de la Cour » banner (offers waiting, treaties about to expire, active
/// league of princes). Empty when there is nothing to show. Port of `diplomacy_court_banner.gd`.
pub fn court_banner(
    data: &GameData,
    state: &CampaignState,
    rows: &[FactionRow],
    offers: &[OfferView],
) -> Vec<String> {
    let plural = |n: u32, one: &str, many: &str| format!("{n} {}", if n > 1 { many } else { one });
    let mut lines = Vec::new();
    if !offers.is_empty() {
        lines.push(format!(
            "\u{2709} {} \u{e0} trancher",
            plural(offers.len() as u32, "proposition", "propositions")
        ));
    }
    for row in rows {
        for (turns, label) in [
            (row.truce_turns_left, "Tr\u{ea}ve"),
            (row.non_aggression_turns_left, "Pacte de non-agression"),
        ] {
            if turns > 0 && turns <= COURT_EXPIRY_HORIZON {
                lines.push(format!(
                    "\u{231b} {label} avec {} : encore {}",
                    row.name,
                    plural(turns, "tour", "tours")
                ));
            }
        }
    }
    if let Some(league) = state.league.as_ref() {
        let bound = state.is_allied(state.player_faction(), &league.target);
        lines.push(format!(
            "\u{2694} Ligue des princes contre {}{}",
            data.faction_name(&league.target),
            if bound {
                " (nous sommes li\u{e9}s \u{e0} lui)"
            } else {
                ""
            }
        ));
    }
    lines
}

/// Counts used by tests and dumps: rows by relation status.
pub fn status_counts(rows: &[FactionRow]) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for row in rows {
        *counts.entry(row.status.clone()).or_insert(0) += 1;
    }
    counts
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup() -> (&'static GameData, CampaignState, FactionId) {
        let data = data_model::test_support::game_data();
        let state = sim_campaign::test_support::start_quiet(data, "fac_france", 1337);
        (data, state, FactionId::new("fac_france").unwrap())
    }

    #[test]
    fn court_banner_lists_offers_and_expiring_truces() {
        let (data, state, france) = setup();
        let mut rows = faction_list(data, &state, &france);
        assert!(court_banner(data, &state, &rows, &[])
            .iter()
            .all(|l| !l.contains("proposition")));
        rows[0].truce_turns_left = 1;
        rows[1].non_aggression_turns_left = 5;
        let offer = OfferView {
            id: 1,
            from: "x".into(),
            from_name: "X".into(),
            kind: "peace".into(),
            ultimatum: false,
            text: String::new(),
            expires_in: 2,
        };
        let lines = court_banner(data, &state, &rows, &[offer]);
        assert!(lines[0].contains("1 proposition"));
        assert!(lines.iter().any(|l| l.contains("encore 1 tour")));
        assert!(!lines.iter().any(|l| l.contains("non-agression")));
    }

    #[test]
    fn france_sees_england_at_war() {
        let (data, state, france) = setup();
        let rows = faction_list(data, &state, &france);
        assert!(rows.len() > 10);
        let england = rows
            .iter()
            .find(|r| r.id == "fac_england")
            .expect("england");
        assert_eq!(england.status, "war");
        assert!(!england.reasons.is_empty());
        assert!(england
            .reasons
            .windows(2)
            .all(|w| w[0].value.abs() >= w[1].value.abs()));
        assert!(!england.stance_label.is_empty());
        assert!(!england.name.is_empty());
        assert!(serde_json::to_string(&rows).is_ok());
        let unknown = FactionId::new("fac_nowhere").unwrap();
        assert!(faction_list(data, &state, &unknown).is_empty());
    }

    #[test]
    fn england_detail_has_war_options_and_actions() {
        let (data, state, france) = setup();
        let england = FactionId::new("fac_england").unwrap();
        let detail = relation_detail(data, &state, &france, &england).expect("detail");
        assert!(detail.war.is_some());
        assert!(detail.options.at_war);
        assert!(detail
            .options
            .clauses
            .iter()
            .any(|c| matches!(c.article, Article::Peace)));
        assert!(detail.actions.iter().any(|a| a.key == "papal_mediation"));
        assert!(!detail.actions.iter().any(|a| a.key == "declare_war"));
        let nowhere = FactionId::new("fac_nowhere").unwrap();
        assert!(relation_detail(data, &state, &france, &nowhere).is_none());
    }

    #[test]
    fn peace_draft_is_evaluated_and_submitted() {
        let (data, mut state, france) = setup();
        let england = FactionId::new("fac_england").unwrap();
        let draft = vec![Article::Peace];
        let verdict = evaluate_draft(data, &state, &france, &england, &draft).expect("verdict");
        assert_eq!(verdict.clause_values.len(), 1);
        assert_eq!(verdict.clause_labels.len(), 1);
        assert!(!verdict.summary.is_empty());
        // Sent through the sim: the answer is either a signed peace or a French refusal.
        let result = submit_order(
            data,
            &mut state,
            Order::ProposeTreaty {
                target: england.clone(),
                articles: draft,
            },
        );
        if result.ok {
            assert!(!state.is_at_war(&france, &england));
        } else {
            assert!(!result.message.is_empty());
            assert!(state.is_at_war(&france, &england));
        }
    }

    #[test]
    fn war_preview_and_peaceful_neighbour_actions() {
        let (data, state, france) = setup();
        let rows = faction_list(data, &state, &france);
        let peaceful = rows
            .iter()
            .find(|r| r.status == "peace" || r.status == "truce")
            .expect("a faction at peace");
        let target = FactionId::new(&peaceful.id).unwrap();
        let detail = relation_detail(data, &state, &france, &target).unwrap();
        let war = detail
            .actions
            .iter()
            .find(|a| a.key == "declare_war")
            .expect("declare war");
        let preview = war.preview.as_ref().unwrap();
        assert!(preview.accept);
        assert!(!preview.reasons.is_empty());
        assert!(detail.actions.iter().any(|a| a.key == "gift"));
    }

    #[test]
    fn counter_for_an_unacceptable_draft() {
        let (data, state, france) = setup();
        let england = FactionId::new("fac_england").unwrap();
        let draft = vec![Article::Vassalage {
            giver: Party::Recipient,
        }];
        match ask_counter(data, &state, &france, &england, &draft) {
            Ok(counter) => assert!(!counter.is_empty()),
            Err(message) => assert!(!message.is_empty()),
        }
        assert!(offers(data, &state, &france).len() < 100);
    }
}
