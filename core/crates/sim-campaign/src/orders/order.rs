//! The [`Order`] enum and the places it designates.

use data_model::{
    BuildingId, CharacterId, FactionId, ProvinceId, SettlementId, SkillId, TechnologyId, UnitTypeId,
};
use serde::{Deserialize, Serialize};

use super::OrderError;
use crate::economy::TaxRate;
use crate::negotiation::{Article, Party, Treaty};
use crate::state::{ArmyId, CampaignState, Stance};

/// Where an order applies: a settlement, or a province standing for its
/// city (v1 compatibility).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Place {
    Settlement(SettlementId),
    Province(ProvinceId),
}

impl From<SettlementId> for Place {
    fn from(id: SettlementId) -> Self {
        Place::Settlement(id)
    }
}

impl From<&SettlementId> for Place {
    fn from(id: &SettlementId) -> Self {
        Place::Settlement(id.clone())
    }
}

impl From<ProvinceId> for Place {
    fn from(id: ProvinceId) -> Self {
        Place::Province(id)
    }
}

impl From<&ProvinceId> for Place {
    fn from(id: &ProvinceId) -> Self {
        Place::Province(id.clone())
    }
}

impl CampaignState {
    /// The settlement a [`Place`] designates (a province: its city).
    pub fn resolve_place(&self, place: &Place) -> Result<SettlementId, OrderError> {
        match place {
            Place::Settlement(id) if self.settlements.contains_key(id) => Ok(id.clone()),
            Place::Settlement(id) => Err(OrderError::UnknownSettlement(id.clone())),
            Place::Province(id) => self
                .province_city_id(id)
                .cloned()
                .ok_or_else(|| OrderError::UnknownProvince(id.clone())),
        }
    }
}

/// Destination of a `move_army` order (lot M2): a settlement (or a
/// province, standing for its city), a map point `{x, y}` in pixels of the
/// map, or a v1/C4 path whose last place is the destination.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MoveOrderTarget {
    Place(Place),
    Point { x: f32, y: f32 },
    Path(Vec<Place>),
}

/// An order issued by a faction (player through `submit_order`, AI through the planner).
///
/// Serialised as `{"type": "move_army", "army": "...", "target": ...}`; the
/// variant and field names are the contract with the Godot bridge.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Order {
    /// Walks the army towards `target` at once (lot M2, `march.rs`): it
    /// stops at the target, when its points run out (the rest of the march
    /// resumes next turn), in an enemy zone of control, or by entering a
    /// settlement (siege, capture or stop). `path` is accepted for `target`.
    MoveArmy {
        army: ArmyId,
        #[serde(alias = "path")]
        target: MoveOrderTarget,
    },
    /// Closes in on an enemy army within `engage_radius_km` and fights it
    /// at once (lot M2); the attacker cannot move afterwards.
    Attack {
        army: ArmyId,
        target_army: ArmyId,
    },
    /// Crosses the sea from the port the army stands in to `to_port` (a
    /// `sea` edge of the settlement graph); costs the whole turn (lot M2).
    Embark {
        army: ArmyId,
        to_port: SettlementId,
    },
    /// Pay for a unit that joins the settlement's garrison at the end of the turn.
    Recruit {
        #[serde(alias = "province")]
        settlement: Place,
        unit_type: UnitTypeId,
    },
    /// Form a new army from garrison units (indices into the garrison).
    CreateArmy {
        #[serde(alias = "province")]
        settlement: Place,
        units_from_garrison: Vec<usize>,
        #[serde(default)]
        general: Option<CharacterId>,
    },
    /// Move every unit of `source` into `target` (same settlement); `source` disappears.
    MergeArmies {
        source: ArmyId,
        target: ArmyId,
    },
    /// Detach `unit_indices` of `army` into a new army on the same settlement.
    SplitArmy {
        army: ArmyId,
        unit_indices: Vec<usize>,
    },
    /// Lot C7a: leave `unit_indices` of `army` as the garrison of the
    /// settlement it stands on (held by the army's faction, not besieged).
    /// An army giving all its units disappears (its general stays in the
    /// province).
    GarrisonUnits {
        army: ArmyId,
        unit_indices: Vec<usize>,
    },
    /// Dismiss one unit of an army (`army`) or of a garrison (`settlement`).
    DisbandUnit {
        #[serde(default)]
        army: Option<ArmyId>,
        #[serde(default, alias = "province")]
        settlement: Option<Place>,
        unit_index: usize,
    },
    SetStance {
        army: ArmyId,
        stance: Stance,
    },
    AssignGeneral {
        army: ArmyId,
        character: CharacterId,
    },
    /// Starts constructing `building` in `settlement` (spec § 1.2).
    Build {
        #[serde(alias = "province")]
        settlement: Place,
        building: BuildingId,
    },
    /// Cancels the ongoing construction of `settlement`, refunding half its cost.
    CancelBuild {
        #[serde(alias = "province")]
        settlement: Place,
    },
    /// M8: cancels the queued (not yet started) construction at `index`
    /// (0 = next in line), refunding half its cost.
    CancelQueuedBuild {
        #[serde(alias = "province")]
        settlement: Place,
        index: usize,
    },
    /// RS-C: razes the completed `building` of `settlement`, refunding
    /// `economy.json` `demolition_refund_percent` of its money cost.
    Demolish {
        #[serde(alias = "province")]
        settlement: Place,
        building: BuildingId,
    },
    /// Sets the faction's tax bracket (spec § 1.4).
    SetTaxRate {
        rate: TaxRate,
    },
    /// Spends a skill point of `character` on `skill` (spec § 2).
    LearnSkill {
        character: CharacterId,
        skill: SkillId,
    },
    /// Makes `character` the governor of `province` (spec § 2).
    AssignGovernor {
        province: ProvinceId,
        character: CharacterId,
    },
    /// Marries `character` and `spouse` (spec § 2).
    ProposeMarriage {
        character: CharacterId,
        spouse: CharacterId,
    },
    /// C7: `companion` leaves the retinue of `from` for that of `to`, two
    /// generals whose armies stand in the same settlement.
    TransferCompanion {
        from: CharacterId,
        to: CharacterId,
        companion: data_model::CompanionId,
    },
    /// Headless-only debug order: grants `amount` XP to `character`, always
    /// accepted (spec § 3, "smoke test"). Never issued by the Godot bridge
    /// UI in a released build.
    DebugGrantXp {
        character: CharacterId,
        amount: u32,
    },
    /// Headless-only debug order (smoke test, lot C7): `companion` joins
    /// `character`'s retinue if he may gain it (cap, conditions).
    DebugGrantCompanion {
        character: CharacterId,
        companion: data_model::CompanionId,
    },
    // ----- M5: diplomacy & religion (spec § 2.3) --------------------------
    DeclareWar {
        target: FactionId,
    },
    /// Each listed province passes to the other party; `tribute` is paid by
    /// `target` (negative: paid to it).
    ProposePeace {
        target: FactionId,
        #[serde(default)]
        provinces: Vec<ProvinceId>,
        #[serde(default)]
        tribute: i64,
    },
    ProposeAlliance {
        target: FactionId,
    },
    /// Lot DP1: a treaty of several articles (`negotiation::Article`).
    ProposeTreaty {
        target: FactionId,
        articles: Vec<crate::negotiation::Article>,
    },
    BreakAlliance {
        target: FactionId,
    },
    SetEmbargo {
        target: FactionId,
        active: bool,
    },
    DemandVassalage {
        target: FactionId,
    },
    ReleaseVassal {
        target: FactionId,
    },
    SendGift {
        target: FactionId,
        amount: i64,
    },
    /// Cross-faction marriage proposal (`character` ours, `spouse` theirs).
    ProposeFactionMarriage {
        target: FactionId,
        character: CharacterId,
        spouse: CharacterId,
    },
    AnswerOffer {
        offer: u32,
        accept: bool,
    },
    /// FE (§ 4.3.5): the player's verdict on a private war offer.
    ArbitratePrivateWar {
        offer: u32,
        verdict: crate::feudal::Arbitration,
    },
    /// FE5 (§ 4.4): the liege declares forfeiture against `vassal`, which
    /// must be under an open felony case.
    DeclareCommise {
        vassal: FactionId,
    },
    /// FE5 (§ 4.4): grants `title` (not the primary one) to `grantee`.
    GrantTitle {
        title: data_model::TitleId,
        grantee: FactionId,
    },
    /// FE5 (§ 4.2): the vassal revolts against its direct suzerain.
    Revolt,
    /// FE5 (§ 4.2): pays homage to `lord`, leaving the current suzerain.
    SwitchAllegiance {
        lord: FactionId,
    },
    RequestPapalMediation {
        target: FactionId,
    },
    DonateToChurch {
        amount: i64,
    },
    ChooseObedience {
        religion: data_model::ReligionId,
    },
    /// Storms the town the army besieges (M8).
    Assault {
        army: ArmyId,
    },
    /// Researches `technology` (M6); switching keeps the abandoned progress.
    Research {
        technology: TechnologyId,
    },
    /// A6-L4: queues `technology` behind the current research.
    QueueResearch {
        technology: TechnologyId,
    },
    /// A6-L4: removes `technology` from the research queue.
    DequeueResearch {
        technology: TechnologyId,
    },
    /// Answers a pending chronicle decision (M10).
    ChooseEventOption {
        decision: u32,
        option: usize,
    },
    /// Feeds `province` with `diet` (H3 « La Table »); one change per
    /// province and per turn.
    SetDiet {
        province: ProvinceId,
        diet: data_model::DietId,
    },
    /// Lot C4: adopts `edict` in `province` (regional edict, one active at a
    /// time, delayed effect); one change per province and per turn.
    SetEdict {
        province: ProvinceId,
        edict: data_model::EdictId,
    },
    /// H5: strikes the faction's money at `level`; one change per year.
    SetCoinage {
        level: crate::coinage::CoinageLevel,
    },
    /// H6: pays the ransom of one of our captives, in full (`installments`
    /// 0 or 1) or by yearly installments (2 to 6, +10 %).
    PayRansom {
        character: CharacterId,
        #[serde(default)]
        installments: u32,
    },
    /// H6: the captor sets the terms of one of its prisoners.
    SetRansomTerms {
        character: CharacterId,
        terms: crate::ransom::RansomTerms,
    },
    /// H6: the captor frees one of its prisoners on parole.
    ReleaseOnParole {
        character: CharacterId,
    },
    /// G1: the captor frees one of its prisoners against `ransom` livres
    /// (default: the computed ransom), paid at once by his faction if it
    /// accepts (a fair price it can afford).
    ReleaseCaptive {
        character: CharacterId,
        #[serde(default)]
        ransom: Option<i64>,
    },
    /// H6: founds the chivalric order `order` (one per faction).
    FoundChivalricOrder {
        order: data_model::ChivalricOrderId,
    },
    /// JR1: the crusader faction preaches the passage (a paid call for
    /// volunteers who land in one of its ports, `crusade.rs`).
    PreachPassage,
    // ----- C6: agents (`agents.rs`) -----------------------------------------
    /// Recruits a spy, herald or preacher on a settlement of ours.
    RecruitAgent {
        settlement: Place,
        kind: data_model::AgentKind,
    },
    /// Walks an agent towards `target` (resumes each season until reached).
    MoveAgent {
        agent: crate::agents::AgentId,
        target: SettlementId,
    },
    /// One action per season; `target` defaults to the agent's settlement
    /// (a neighbour is allowed); `character` names the captive of a ransom.
    AgentAction {
        agent: crate::agents::AgentId,
        action: data_model::AgentActionKind,
        #[serde(default)]
        target: Option<SettlementId>,
        #[serde(default)]
        character: Option<CharacterId>,
    },
    DismissAgent {
        agent: crate::agents::AgentId,
    },
    // ----- C5: trade (`trade.rs`) --------------------------------------------
    /// Ends an existing trade agreement with `target` (agreements are
    /// concluded by a treaty article, `ProposeTreaty`, lot DP1; ADR 0012).
    BreakTradeAgreement {
        target: FactionId,
    },
    // ----- CV3-3: map encounters (`encounter.rs`) ----------------------------
    /// Answers the encounter `site` met by `army` with option `option`.
    ChooseEncounterOption {
        army: ArmyId,
        site: u32,
        option: usize,
    },
    // ----- TW2-T1: fate of a captured place (`capture.rs`) -------------------
    /// Decides the fate of the place of pending capture `decision`.
    ChooseCaptureOutcome {
        decision: u32,
        outcome: crate::capture::CaptureOutcome,
    },
    // ----- TW2-T3: mercenary companies (`mercenaries.rs`) --------------------
    /// `army` hires a company of `unit` from the reserve of the region it
    /// stands in; the company joins at once.
    HireMercenary {
        army: ArmyId,
        #[serde(alias = "unit_type")]
        unit: UnitTypeId,
    },
    // ----- TW2-T5: army traditions (`traditions.rs`) -------------------------
    /// `army` takes `tradition` (an id of `data/rules/army_traditions.json`)
    /// for a rank it has reached.
    ChooseArmyTradition {
        army: ArmyId,
        tradition: String,
    },
}

impl Order {
    /// `MoveArmy` along a path of settlements (C4 planners): towards the
    /// last one.
    pub fn move_along(army: ArmyId, path: Vec<SettlementId>) -> Order {
        Order::MoveArmy {
            army,
            target: MoveOrderTarget::Path(path.into_iter().map(Place::from).collect()),
        }
    }

    /// `MoveArmy` towards a settlement (entered on arrival).
    pub fn move_to(army: ArmyId, settlement: SettlementId) -> Order {
        Order::MoveArmy {
            army,
            target: MoveOrderTarget::Place(Place::Settlement(settlement)),
        }
    }

    /// `MoveArmy` towards map pixel `point`.
    pub fn move_to_point(army: ArmyId, point: [f32; 2]) -> Order {
        Order::MoveArmy {
            army,
            target: MoveOrderTarget::Point {
                x: point[0],
                y: point[1],
            },
        }
    }

    /// The treaty a diplomatic order proposes, with its target (`None` for
    /// any other order).
    pub fn proposal(
        &self,
        state: &CampaignState,
        proposer: &FactionId,
    ) -> Option<(FactionId, Treaty)> {
        let (target, treaty) = match self {
            Order::ProposePeace {
                target,
                provinces,
                tribute,
            } => (
                target,
                Treaty::peace_terms(state, proposer, provinces, *tribute),
            ),
            Order::ProposeAlliance { target } => (target, Treaty::single(Article::Alliance)),
            Order::ProposeTreaty { target, articles } => (target, Treaty::new(articles.clone())),
            Order::DemandVassalage { target } => (
                target,
                Treaty::single(Article::Vassalage {
                    giver: Party::Recipient,
                }),
            ),
            Order::ProposeFactionMarriage {
                target,
                character,
                spouse,
            } => (
                target,
                Treaty::single(Article::Marriage {
                    character: character.clone(),
                    spouse: spouse.clone(),
                }),
            ),
            _ => return None,
        };
        Some((target.clone(), treaty))
    }
}
