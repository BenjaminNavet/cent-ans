//! Player and AI orders: validation and immediate application.
//!
//! Every order is validated before anything is mutated; a rejected order
//! leaves the state untouched. Move orders are only recorded (`Army::path`)
//! and resolved in `end_turn`; all other orders apply immediately.
//!
//! Lot C4: recruitment, construction, garrisons and moves target settlements.
//! A [`Place`] also accepts a province id, which stands for its city (the v1
//! JSON field name `province` is still read as an alias).

use data_model::{
    BuildingId, CharacterId, CharacterStatus, FactionId, GameData, ProvinceId, SettlementId,
    SkillId, TechnologyId, UnitTypeId,
};
use serde::{Deserialize, Serialize};

use crate::buildings::CANCEL_REFUND_PERCENT;
use crate::diplomacy::Proposal;
use crate::dynasty::{self, GovernorError, MarriageError};
use crate::economy::TaxRate;
use crate::movement;
use crate::research::{self, ResearchError};
use crate::skills::{self, LearnSkillError};
use crate::state::{Army, ArmyId, CampaignState, Construction, Stance, Unit};

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

/// An order issued by a faction (player through `submit_order`, AI through the planner).
///
/// Serialised as `{"type": "move_army", "army": "...", "path": [...]}`; the
/// variant and field names are the contract with the Godot bridge.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Order {
    /// Walk along `path` (adjacent settlements, the army's own location
    /// excluded). A path naming provinces (v1) means "go to the city of the
    /// last one" by the cheapest route.
    MoveArmy {
        army: ArmyId,
        path: Vec<Place>,
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
    /// Headless-only debug order: grants `amount` XP to `character`, always
    /// accepted (spec § 3, "smoke test"). Never issued by the Godot bridge
    /// UI in a released build.
    DebugGrantXp {
        character: CharacterId,
        amount: u32,
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
}

impl Order {
    /// `MoveArmy` along a path of settlements.
    pub fn move_along(army: ArmyId, path: Vec<SettlementId>) -> Order {
        Order::MoveArmy {
            army,
            path: path.into_iter().map(Place::from).collect(),
        }
    }
}

/// Why an order was refused (messages in French for the UI).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum OrderError {
    #[error("armée inconnue : {0}")]
    UnknownArmy(ArmyId),
    #[error("province inconnue : {0}")]
    UnknownProvince(ProvinceId),
    #[error("colonie inconnue : {0}")]
    UnknownSettlement(SettlementId),
    #[error("type d'unité inconnu : {0}")]
    UnknownUnitType(UnitTypeId),
    #[error("personnage inconnu : {0}")]
    UnknownCharacter(CharacterId),
    #[error("cette armée n'appartient pas à la faction {0}")]
    NotYourArmy(FactionId),
    #[error("ce personnage n'appartient pas à la faction {0}")]
    NotYourCharacter(FactionId),
    #[error("cette province n'est pas contrôlée par la faction {0}")]
    NotYourProvince(FactionId),
    #[error("cette colonie n'est pas contrôlée par la faction {0}")]
    NotYourSettlement(FactionId),
    #[error("chemin vide")]
    EmptyPath,
    #[error("chemin invalide : {from} et {to} ne sont pas reliées (route ou mer entre ports)")]
    NotAdjacent { from: String, to: String },
    #[error("destination inaccessible")]
    NoPath,
    #[error("recrutement impossible : {0}")]
    RecruitUnavailable(String),
    #[error("file de recrutement pleine : {slots} recrutement(s) par tour dans cette colonie")]
    RecruitQueueFull { slots: usize },
    #[error("trésor insuffisant : {needed} livres nécessaires, {available} disponibles")]
    InsufficientFunds { needed: i64, available: i64 },
    #[error("indice d'unité invalide : {0}")]
    InvalidUnitIndex(usize),
    #[error("aucune unité sélectionnée")]
    NoUnitsSelected,
    #[error("une armée doit garder au moins une unité")]
    WouldEmptyArmy,
    #[error("les deux armées doivent être au même endroit")]
    NotSameProvince,
    #[error("les deux armées doivent appartenir à la même faction")]
    NotSameFaction,
    #[error("ce personnage ne peut pas commander (mort, captif, mineur ou d'une autre faction)")]
    CharacterUnavailable,
    #[error("ce personnage n'est pas dans cette province")]
    CharacterElsewhere,
    #[error("ce personnage gouverne déjà une province")]
    AlreadyGoverning,
    #[error("il faut préciser soit une armée soit une province")]
    AmbiguousTarget,
    #[error("faction inconnue : {0}")]
    UnknownFaction(FactionId),
    #[error("bâtiment inconnu : {0}")]
    UnknownBuilding(BuildingId),
    #[error("construction impossible : {0}")]
    BuildUnavailable(String),
    #[error("aucune construction en cours dans cette colonie")]
    NoConstruction,
    #[error(transparent)]
    LearnSkill(#[from] LearnSkillError),
    #[error(transparent)]
    Governor(#[from] GovernorError),
    #[error(transparent)]
    Marriage(#[from] MarriageError),
    #[error(transparent)]
    Diplomacy(#[from] crate::diplomacy::DiplomacyError),
    #[error(transparent)]
    Research(#[from] ResearchError),
    #[error(transparent)]
    Chronicle(#[from] crate::chronicle::ChronicleError),
    #[error(transparent)]
    Assault(#[from] crate::siege::AssaultError),
    #[error(transparent)]
    Diet(#[from] crate::table::DietError),
    #[error(transparent)]
    Coinage(#[from] crate::coinage::CoinageError),
    #[error(transparent)]
    Ransom(#[from] crate::ransom::RansomError),
    #[error(transparent)]
    Chivalry(#[from] crate::chivalry::ChivalryError),
}

/// G1: recruitments every settlement can queue per turn before buildings.
pub const BASE_RECRUIT_SLOTS: usize = 2;

/// One line of the recruitment panel.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecruitOption {
    pub unit_type: UnitTypeId,
    pub name: String,
    pub cost: u32,
    pub upkeep: u32,
    pub available: bool,
    /// French explanation when `available` is `false`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

impl CampaignState {
    /// Validates and applies (or records, for moves) an order of the player faction.
    pub fn submit_order(&mut self, data: &GameData, order: Order) -> Result<(), OrderError> {
        let player = self.player_faction.clone();
        self.apply_order(data, &player, order)
    }

    /// Validates and applies an order on behalf of `faction`.
    pub fn apply_order(
        &mut self,
        data: &GameData,
        faction: &FactionId,
        order: Order,
    ) -> Result<(), OrderError> {
        if !self.factions.contains_key(faction) {
            return Err(OrderError::UnknownFaction(faction.clone()));
        }
        match order {
            Order::MoveArmy { army, path } => self.order_move(data, faction, &army, path),
            Order::Recruit {
                settlement,
                unit_type,
            } => {
                let settlement = self.resolve_place(&settlement)?;
                self.order_recruit(data, faction, &settlement, &unit_type)
            }
            Order::CreateArmy {
                settlement,
                units_from_garrison,
                general,
            } => {
                let settlement = self.resolve_place(&settlement)?;
                self.order_create_army(data, faction, &settlement, &units_from_garrison, general)
            }
            Order::MergeArmies { source, target } => self.order_merge(faction, &source, &target),
            Order::SplitArmy { army, unit_indices } => {
                self.order_split(faction, &army, &unit_indices)
            }
            Order::DisbandUnit {
                army,
                settlement,
                unit_index,
            } => {
                let settlement = settlement
                    .map(|place| self.resolve_place(&place))
                    .transpose()?;
                self.order_disband(faction, army.as_ref(), settlement.as_ref(), unit_index)
            }
            Order::SetStance { army, stance } => {
                self.own_army_mut(faction, &army)?.stance = stance;
                Ok(())
            }
            Order::AssignGeneral { army, character } => {
                self.order_assign_general(faction, &army, &character)
            }
            Order::Build {
                settlement,
                building,
            } => {
                let settlement = self.resolve_place(&settlement)?;
                self.order_build(data, faction, &settlement, &building)
            }
            Order::CancelBuild { settlement } => {
                let settlement = self.resolve_place(&settlement)?;
                self.order_cancel_build(data, faction, &settlement)
            }
            Order::SetTaxRate { rate } => {
                self.factions
                    .get_mut(faction)
                    .expect("checked above")
                    .tax_rate = rate;
                Ok(())
            }
            Order::LearnSkill { character, skill } => {
                self.check_owned_character(faction, &character)?;
                skills::learn_skill(self, data, &character, &skill)?;
                Ok(())
            }
            Order::AssignGovernor {
                province,
                character,
            } => {
                self.check_owned_character(faction, &character)?;
                dynasty::assign_governor(self, &province, &character)?;
                Ok(())
            }
            Order::ProposeMarriage { character, spouse } => {
                self.check_owned_character(faction, &character)?;
                dynasty::propose_marriage(self, data, &character, &spouse)?;
                Ok(())
            }
            Order::DebugGrantXp { character, amount } => {
                skills::grant_experience(self, &character, amount);
                Ok(())
            }
            Order::DeclareWar { target } => Ok(self.declare_war(data, faction, &target)?),
            Order::ProposePeace {
                target,
                provinces,
                tribute,
            } => Ok(self.propose(
                data,
                faction,
                &target,
                Proposal::Peace { provinces, tribute },
            )?),
            Order::ProposeAlliance { target } => {
                Ok(self.propose(data, faction, &target, Proposal::Alliance)?)
            }
            Order::BreakAlliance { target } => Ok(self.break_alliance(data, faction, &target)?),
            Order::SetEmbargo { target, active } => {
                Ok(self.set_embargo(data, faction, &target, active)?)
            }
            Order::DemandVassalage { target } => {
                Ok(self.propose(data, faction, &target, Proposal::Vassalage)?)
            }
            Order::ReleaseVassal { target } => Ok(self.release_vassal(data, faction, &target)?),
            Order::SendGift { target, amount } => {
                Ok(self.send_gift(data, faction, &target, amount)?)
            }
            Order::ProposeFactionMarriage {
                target,
                character,
                spouse,
            } => {
                self.check_owned_character(faction, &character)?;
                if self.characters.get(&spouse).map(|c| &c.faction) != Some(&target) {
                    return Err(OrderError::NotYourCharacter(target));
                }
                Ok(self.propose(
                    data,
                    faction,
                    &target,
                    Proposal::Marriage { character, spouse },
                )?)
            }
            Order::AnswerOffer { offer, accept } => {
                Ok(self.answer_offer(data, faction, offer, accept)?)
            }
            Order::RequestPapalMediation { target } => {
                Ok(self.request_papal_mediation(data, faction, &target)?)
            }
            Order::DonateToChurch { amount } => Ok(self.donate_to_church(data, faction, amount)?),
            Order::ChooseObedience { religion } => Ok(crate::religion::set_obedience(
                self, data, faction, &religion,
            )?),
            Order::Assault { army } => Ok(self.assault(data, faction, &army)?),
            Order::Research { technology } => {
                research::start_research(self, data, faction, &technology)?;
                Ok(())
            }
            Order::ChooseEventOption { decision, option } => {
                Ok(self.choose_event_option(data, faction, decision, option)?)
            }
            Order::SetDiet { province, diet } => {
                crate::table::set_diet(self, data, faction, &province, &diet)?;
                Ok(())
            }
            Order::SetCoinage { level } => {
                crate::coinage::set_coinage(self, data, faction, level)?;
                Ok(())
            }
            Order::PayRansom {
                character,
                installments,
            } => {
                crate::ransom::pay_ransom(self, data, faction, &character, installments)?;
                Ok(())
            }
            Order::SetRansomTerms { character, terms } => {
                crate::ransom::set_ransom_terms(self, data, faction, &character, terms)?;
                Ok(())
            }
            Order::ReleaseCaptive { character, ransom } => {
                crate::ransom::release_for_ransom(self, data, faction, &character, ransom)?;
                Ok(())
            }
            Order::ReleaseOnParole { character } => {
                crate::ransom::release_on_parole(self, data, faction, &character)?;
                Ok(())
            }
            Order::FoundChivalricOrder { order } => {
                crate::chivalry::found_order(self, data, faction, &order)?;
                Ok(())
            }
        }
    }

    /// `character` exists and belongs to `faction` (used by the M4 orders
    /// that are not army/province scoped).
    fn check_owned_character(
        &self,
        faction: &FactionId,
        character: &CharacterId,
    ) -> Result<(), OrderError> {
        let state = self
            .characters
            .get(character)
            .ok_or_else(|| OrderError::UnknownCharacter(character.clone()))?;
        if &state.faction != faction {
            return Err(OrderError::NotYourCharacter(faction.clone()));
        }
        Ok(())
    }

    fn order_build(
        &mut self,
        data: &GameData,
        faction: &FactionId,
        settlement: &SettlementId,
        building: &BuildingId,
    ) -> Result<(), OrderError> {
        self.own_settlement(faction, settlement)?;
        let Some(definition) = data.buildings.get(building) else {
            return Err(OrderError::UnknownBuilding(building.clone()));
        };
        if !definition.allowed_in(self.settlement_kind(settlement)) {
            return Err(OrderError::BuildUnavailable(
                "impossible dans ce type de colonie".to_owned(),
            ));
        }
        let option = self
            .buildable(data, settlement)
            .into_iter()
            .find(|o| &o.building == building)
            .ok_or_else(|| OrderError::UnknownBuilding(building.clone()))?;
        if !option.available {
            return Err(OrderError::BuildUnavailable(
                option.reason.unwrap_or_default(),
            ));
        }
        self.factions
            .get_mut(faction)
            .expect("checked above")
            .treasury -= i64::from(option.cost);
        self.settlements
            .get_mut(settlement)
            .expect("checked above")
            .construction = Some(Construction {
            building: building.clone(),
            turns_left: option.turns,
        });
        Ok(())
    }

    fn order_cancel_build(
        &mut self,
        data: &GameData,
        faction: &FactionId,
        settlement: &SettlementId,
    ) -> Result<(), OrderError> {
        self.own_settlement(faction, settlement)?;
        let settlement_state = self.settlements.get_mut(settlement).expect("checked above");
        let Some(construction) = settlement_state.construction.take() else {
            return Err(OrderError::NoConstruction);
        };
        let refund = data
            .buildings
            .get(&construction.building)
            .map_or(0, |b| b.cost.money * CANCEL_REFUND_PERCENT / 100);
        self.factions
            .get_mut(faction)
            .expect("checked above")
            .treasury += i64::from(refund);
        Ok(())
    }

    fn own_army(&self, faction: &FactionId, id: &ArmyId) -> Result<&Army, OrderError> {
        let army = self
            .armies
            .get(id)
            .ok_or_else(|| OrderError::UnknownArmy(id.clone()))?;
        if &army.faction != faction {
            return Err(OrderError::NotYourArmy(faction.clone()));
        }
        Ok(army)
    }

    fn own_army_mut(&mut self, faction: &FactionId, id: &ArmyId) -> Result<&mut Army, OrderError> {
        self.own_army(faction, id)?;
        Ok(self.armies.get_mut(id).expect("checked above"))
    }

    fn own_settlement(&self, faction: &FactionId, id: &SettlementId) -> Result<(), OrderError> {
        let settlement = self
            .settlements
            .get(id)
            .ok_or_else(|| OrderError::UnknownSettlement(id.clone()))?;
        if &settlement.controller != faction {
            return Err(OrderError::NotYourSettlement(faction.clone()));
        }
        Ok(())
    }

    fn order_move(
        &mut self,
        data: &GameData,
        faction: &FactionId,
        army_id: &ArmyId,
        places: Vec<Place>,
    ) -> Result<(), OrderError> {
        let army = self.own_army(faction, army_id)?;
        let mut path = if places.iter().any(|p| matches!(p, Place::Province(_))) {
            // v1 path of provinces: head for the city of the last one.
            let Some(last) = places.last() else {
                return Err(OrderError::EmptyPath);
            };
            let target = self.resolve_place(last)?;
            self.find_path(data, army_id, &target)
                .ok_or(OrderError::NoPath)?
        } else {
            places
                .into_iter()
                .map(|p| match p {
                    Place::Settlement(id) => id,
                    Place::Province(_) => unreachable!("handled above"),
                })
                .collect()
        };
        if path.first() == Some(&army.location) {
            path.remove(0);
        }
        if path.is_empty() {
            return Err(OrderError::EmptyPath);
        }
        movement::validate_path(data, &army.location, &path)?;
        self.armies.get_mut(army_id).expect("checked").path = path;
        Ok(())
    }

    fn order_recruit(
        &mut self,
        data: &GameData,
        faction: &FactionId,
        settlement: &SettlementId,
        unit_type: &UnitTypeId,
    ) -> Result<(), OrderError> {
        if let Some(state) = self.settlements.get(settlement) {
            let slots = self.recruit_slots(data, settlement);
            if &state.controller == faction && state.recruit_queue.len() >= slots {
                return Err(OrderError::RecruitQueueFull { slots });
            }
        }
        let option = self
            .recruit_option(data, faction, settlement, unit_type)
            .ok_or_else(|| OrderError::UnknownUnitType(unit_type.clone()))?;
        if !option.available {
            let reason = option.reason.unwrap_or_default();
            let treasury = self.factions.get(faction).map_or(0, |f| f.treasury);
            return if reason.starts_with("trésor") {
                Err(OrderError::InsufficientFunds {
                    needed: i64::from(option.cost),
                    available: treasury,
                })
            } else {
                Err(OrderError::RecruitUnavailable(reason))
            };
        }
        let faction_state = self.factions.get_mut(faction).expect("checked");
        faction_state.treasury -= i64::from(option.cost);
        self.settlements
            .get_mut(settlement)
            .expect("checked")
            .recruit_queue
            .push(unit_type.clone());
        Ok(())
    }

    /// Recruitment options of `settlement` for its controller (the player's view).
    pub fn recruitable(&self, data: &GameData, settlement: &SettlementId) -> Vec<RecruitOption> {
        let Some(controller) = self
            .settlements
            .get(settlement)
            .map(|s| s.controller.clone())
        else {
            return Vec::new();
        };
        data.unit_types
            .keys()
            .filter_map(|unit_type| self.recruit_option(data, &controller, settlement, unit_type))
            .collect()
    }

    /// Recruitment options of the city of `province` (v1 signature).
    pub fn recruitable_in_province(
        &self,
        data: &GameData,
        province: &ProvinceId,
    ) -> Vec<RecruitOption> {
        self.province_city_id(province)
            .map(|city| self.recruitable(data, city))
            .unwrap_or_default()
    }

    /// Availability of one unit type in one settlement for `faction`.
    pub fn recruit_option(
        &self,
        data: &GameData,
        faction: &FactionId,
        settlement: &SettlementId,
        unit_type_id: &UnitTypeId,
    ) -> Option<RecruitOption> {
        let unit_type = data.unit_types.get(unit_type_id)?;
        let mut option = RecruitOption {
            unit_type: unit_type_id.clone(),
            name: unit_type.name.display.clone(),
            cost: self.recruit_cost(data, faction, settlement, unit_type),
            upkeep: unit_type.upkeep,
            available: true,
            reason: None,
        };
        let reason = self.recruit_blocker(data, faction, settlement, unit_type);
        if let Some(reason) = reason {
            option.available = false;
            option.reason = Some(reason);
        }
        Some(option)
    }

    fn recruit_blocker(
        &self,
        data: &GameData,
        faction: &FactionId,
        settlement_id: &SettlementId,
        unit_type: &data_model::UnitType,
    ) -> Option<String> {
        let settlement = self.settlements.get(settlement_id)?;
        let province_state = self.provinces.get(&settlement.province)?;
        let province = data.provinces.get(&settlement.province)?;
        let faction_state = self.factions.get(faction)?;
        if &settlement.owner != faction || &settlement.controller != faction {
            return Some("la colonie doit être possédée et contrôlée".to_owned());
        }
        if settlement.siege.is_some() {
            return Some("la colonie est assiégée".to_owned());
        }
        let slots = self.recruit_slots(data, settlement_id);
        if settlement.recruit_queue.len() >= slots {
            return Some(format!("file de recrutement pleine ({slots} par tour)"));
        }
        if let Some(building) = &unit_type.required_building {
            if !settlement.buildings.contains(building) {
                let name = data
                    .buildings
                    .get(building)
                    .map_or_else(|| building.to_string(), |b| b.name.display.clone());
                return Some(format!("bâtiment requis : {name}"));
            }
        }
        if let Some(tech) = &unit_type.required_technology {
            if !faction_state.technologies.contains(tech) {
                let name = data
                    .technologies
                    .get(tech)
                    .map_or_else(|| tech.to_string(), |t| t.name.display.clone());
                return Some(format!("technologie requise : {name}"));
            }
        }
        if !unit_type.required_faction.is_empty() && !unit_type.required_faction.contains(faction) {
            return Some("réservé à d'autres factions".to_owned());
        }
        if !unit_type.required_culture.is_empty()
            && !unit_type.required_culture.contains(&province.culture)
        {
            return Some("culture locale inadaptée".to_owned());
        }
        // Lot C4: the province provides the men, capped by the settlement's share.
        let class = province_state.population.get(unit_type.source_class);
        let share = crate::settlements::weight_share(data, settlement_id);
        if (class.count as f64 * share) < f64::from(unit_type.soldiers) * 10.0 {
            return Some("classe sociale trop peu nombreuse".to_owned());
        }
        let cost = self.recruit_cost(data, faction, settlement_id, unit_type);
        if faction_state.treasury < i64::from(cost) {
            return Some(format!("trésor insuffisant ({cost} livres nécessaires)"));
        }
        None
    }

    /// G1 `RecruitSlots`: recruitments a settlement can queue per turn —
    /// [`BASE_RECRUIT_SLOTS`], one more in the city of the faction capital,
    /// plus the flat `recruit_slots` of its buildings, governor and the
    /// controller's technologies (muster field, stables, armoury…).
    pub fn recruit_slots(&self, data: &GameData, settlement: &SettlementId) -> usize {
        let Some(state) = self.settlements.get(settlement) else {
            return 0;
        };
        let mut effects = self.settlement_effects(data, settlement);
        effects.merge(&research::faction_tech_effects(
            self,
            data,
            &state.controller,
        ));
        let capital = self.factions.get(&state.controller).is_some_and(|f| {
            f.capital == state.province && self.province_city_id(&f.capital) == Some(settlement)
        });
        BASE_RECRUIT_SLOTS + usize::from(capital) + effects.recruit_slots.flat.max(0.0) as usize
    }

    /// Recruitment slots still free this turn in `settlement`.
    pub fn recruit_slots_free(&self, data: &GameData, settlement: &SettlementId) -> usize {
        let queued = self
            .settlements
            .get(settlement)
            .map_or(0, |s| s.recruit_queue.len());
        self.recruit_slots(data, settlement).saturating_sub(queued)
    }

    /// Money cost of recruiting `unit_type` in `settlement` for `faction`
    /// (F1 `RecruitCost`): the settlement's buildings and the governor
    /// (stables: cavalry −10 %) and the faction's technologies
    /// (francs-archers: ranged −10 %), global or per unit family. Never
    /// below a quarter of the base cost.
    pub fn recruit_cost(
        &self,
        data: &GameData,
        faction: &FactionId,
        settlement: &SettlementId,
        unit_type: &data_model::UnitType,
    ) -> u32 {
        let mut effects = self.settlement_effects(data, settlement);
        effects.merge(&research::faction_tech_effects(self, data, faction));
        let targeted = effects.unit_categories.get(unit_type.category).recruit_cost;
        let flat = effects.recruit_cost.flat + targeted.flat;
        let percent = (effects.recruit_cost.percent + targeted.percent).max(-75.0);
        let base = f64::from(unit_type.cost.money);
        // H5: prices follow the coinage.
        let prices = crate::coinage::price_factor(self, faction);
        let cost = ((base + flat) * (1.0 + percent / 100.0))
            .round()
            .max(base / 4.0)
            .max(0.0);
        (cost * prices) as u32
    }

    fn order_create_army(
        &mut self,
        data: &GameData,
        faction: &FactionId,
        settlement: &SettlementId,
        indices: &[usize],
        general: Option<CharacterId>,
    ) -> Result<(), OrderError> {
        self.own_settlement(faction, settlement)?;
        let garrison_len = self.settlements[settlement].garrison.len();
        let indices = unique_sorted(indices, garrison_len)?;
        let province = self.settlements[settlement].province.clone();
        if let Some(character) = &general {
            self.check_general(faction, character, &province)?;
        }
        let units = take_indices(
            &mut self.settlements.get_mut(settlement).expect("checked").garrison,
            &indices,
        );
        let id = self.allocate_army_id();
        self.armies.insert(
            id.clone(),
            Army {
                faction: faction.clone(),
                general: None,
                location: settlement.clone(),
                units,
                movement_points: 0,
                supply: 100,
                stance: Stance::Normal,
                path: Vec::new(),
            },
        );
        if let Some(character) = general {
            self.attach_general(&id, &character);
        }
        // F1: siege trains and movement effects set the pace from the start.
        let allowance = self.army_movement_allowance(data, &self.armies[&id]);
        self.armies
            .get_mut(&id)
            .expect("just created")
            .movement_points = allowance;
        Ok(())
    }

    fn order_merge(
        &mut self,
        faction: &FactionId,
        source: &ArmyId,
        target: &ArmyId,
    ) -> Result<(), OrderError> {
        let source_army = self.own_army(faction, source)?.clone();
        let target_army = self.own_army(faction, target)?;
        if source == target {
            return Err(OrderError::NotSameProvince);
        }
        if source_army.location != target_army.location {
            return Err(OrderError::NotSameProvince);
        }
        let general = source_army.general.clone();
        self.armies.remove(source);
        let target_mut = self.armies.get_mut(target).expect("checked");
        target_mut.units.extend(source_army.units);
        target_mut.movement_points = target_mut.movement_points.min(source_army.movement_points);
        if target_mut.general.is_none() {
            if let Some(general) = general {
                self.attach_general(target, &general);
            }
        } else if let Some(general) = general {
            self.detach_general(&general);
        }
        Ok(())
    }

    fn order_split(
        &mut self,
        faction: &FactionId,
        army_id: &ArmyId,
        indices: &[usize],
    ) -> Result<(), OrderError> {
        let army = self.own_army(faction, army_id)?;
        let indices = unique_sorted(indices, army.units.len())?;
        if indices.len() == army.units.len() {
            return Err(OrderError::WouldEmptyArmy);
        }
        let template = army.clone();
        let units = take_indices(
            &mut self.armies.get_mut(army_id).expect("checked").units,
            &indices,
        );
        let id = self.allocate_army_id();
        self.armies.insert(
            id,
            Army {
                faction: faction.clone(),
                general: None,
                location: template.location,
                units,
                movement_points: template.movement_points,
                supply: template.supply,
                stance: template.stance,
                path: Vec::new(),
            },
        );
        Ok(())
    }

    fn order_disband(
        &mut self,
        faction: &FactionId,
        army: Option<&ArmyId>,
        settlement: Option<&SettlementId>,
        unit_index: usize,
    ) -> Result<(), OrderError> {
        match (army, settlement) {
            (Some(army_id), None) => {
                let army = self.own_army(faction, army_id)?;
                if unit_index >= army.units.len() {
                    return Err(OrderError::InvalidUnitIndex(unit_index));
                }
                if army.units.len() == 1 {
                    // Dismissing the last unit disbands the army (M10), unless
                    // it is about to fight.
                    let fighting = self
                        .pending_battles
                        .iter()
                        .any(|b| &b.attacker == army_id || &b.defender == army_id);
                    if fighting {
                        return Err(OrderError::WouldEmptyArmy);
                    }
                    if let Some(general) = army.general.clone() {
                        self.detach_general(&general);
                    }
                    self.armies.remove(army_id);
                    return Ok(());
                }
                self.armies
                    .get_mut(army_id)
                    .expect("checked")
                    .units
                    .remove(unit_index);
                Ok(())
            }
            (None, Some(settlement_id)) => {
                self.own_settlement(faction, settlement_id)?;
                let garrison = &mut self
                    .settlements
                    .get_mut(settlement_id)
                    .expect("checked")
                    .garrison;
                if unit_index >= garrison.len() {
                    return Err(OrderError::InvalidUnitIndex(unit_index));
                }
                garrison.remove(unit_index);
                Ok(())
            }
            _ => Err(OrderError::AmbiguousTarget),
        }
    }

    fn order_assign_general(
        &mut self,
        faction: &FactionId,
        army_id: &ArmyId,
        character: &CharacterId,
    ) -> Result<(), OrderError> {
        let settlement = self.own_army(faction, army_id)?.location.clone();
        let location = self
            .settlement_province(&settlement)
            .cloned()
            .ok_or_else(|| OrderError::UnknownSettlement(settlement.clone()))?;
        self.check_general(faction, character, &location)?;
        if let Some(previous) = self.armies[army_id].general.clone() {
            self.detach_general(&previous);
        }
        self.attach_general(army_id, character);
        Ok(())
    }

    fn check_general(
        &self,
        faction: &FactionId,
        character: &CharacterId,
        province: &ProvinceId,
    ) -> Result<(), OrderError> {
        let state = self
            .characters
            .get(character)
            .ok_or_else(|| OrderError::UnknownCharacter(character.clone()))?;
        if !state.alive || state.captive || &state.faction != faction || !state.is_major(self.year)
        {
            return Err(OrderError::CharacterUnavailable);
        }
        if state.governor_of.is_some() {
            return Err(OrderError::AlreadyGoverning);
        }
        let in_province = state.location.as_ref() == Some(province)
            || state
                .army
                .as_ref()
                .and_then(|a| self.armies.get(a))
                .is_some_and(|a| self.settlement_province(&a.location) == Some(province));
        if !in_province {
            return Err(OrderError::CharacterElsewhere);
        }
        Ok(())
    }

    /// Makes `character` the general of `army`, detaching it from its previous army.
    pub(crate) fn attach_general(&mut self, army_id: &ArmyId, character: &CharacterId) {
        if let Some(previous_army) = self.characters.get(character).and_then(|c| c.army.clone()) {
            if let Some(previous) = self.armies.get_mut(&previous_army) {
                if previous.general.as_ref() == Some(character) {
                    previous.general = None;
                }
            }
        }
        if let Some(army) = self.armies.get_mut(army_id) {
            army.general = Some(character.clone());
            let location = self
                .settlements
                .get(&army.location)
                .map(|s| s.province.clone());
            if let Some(state) = self.characters.get_mut(character) {
                state.army = Some(army_id.clone());
                if location.is_some() {
                    state.location = location;
                }
            }
        }
    }

    /// Removes `character` from the army it commands (stays in the province).
    pub(crate) fn detach_general(&mut self, character: &CharacterId) {
        let Some(state) = self.characters.get_mut(character) else {
            return;
        };
        if let Some(army_id) = state.army.take() {
            if let Some(province) = self
                .armies
                .get(&army_id)
                .and_then(|army| self.settlements.get(&army.location))
            {
                state.location = Some(province.province.clone());
            }
            if let Some(army) = self.armies.get_mut(&army_id) {
                if army.general.as_ref() == Some(character) {
                    army.general = None;
                }
            }
        }
    }
}

/// `true` when a character with this static status may command at the start.
pub(crate) fn status_allows_command(status: Option<CharacterStatus>) -> bool {
    !matches!(
        status,
        Some(CharacterStatus::Minor) | Some(CharacterStatus::Captive)
    )
}

fn unique_sorted(indices: &[usize], len: usize) -> Result<Vec<usize>, OrderError> {
    if indices.is_empty() {
        return Err(OrderError::NoUnitsSelected);
    }
    let mut sorted: Vec<usize> = indices.to_vec();
    sorted.sort_unstable();
    sorted.dedup();
    if let Some(&bad) = sorted.iter().find(|&&i| i >= len) {
        return Err(OrderError::InvalidUnitIndex(bad));
    }
    Ok(sorted)
}

/// Removes the units at `sorted_indices` (ascending, unique) and returns them in order.
fn take_indices(units: &mut Vec<Unit>, sorted_indices: &[usize]) -> Vec<Unit> {
    let mut taken = Vec::with_capacity(sorted_indices.len());
    for &index in sorted_indices.iter().rev() {
        taken.push(units.remove(index));
    }
    taken.reverse();
    taken
}
