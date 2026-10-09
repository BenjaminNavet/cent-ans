//! Player and AI orders: validation and immediate application.
//!
//! Every order is validated before anything is mutated; a rejected order
//! leaves the state untouched. Orders apply immediately (moves, attacks and
//! embarkations too, see `march.rs`). Recruitment, construction, garrisons
//! and moves target settlements; a [`Place`] also accepts a province id,
//! which stands for its city.
//!
//! - `order`: the [`Order`] enum and [`Place`];
//! - `error`: [`OrderError`];
//! - `recruit`: recruitment (one [`RecruitContext`] per settlement);
//! - `army`: raising, merging, splitting, garrisoning armies;
//! - `build`: construction.

mod army;
mod build;
mod error;
mod order;
mod recruit;

pub use army::army_room;
pub(crate) use army::status_allows_command;
pub use error::OrderError;
pub use order::{MoveOrderTarget, Order, Place};
pub use recruit::{RecruitContext, RecruitGroup, RecruitOption, RecruitPrice, BASE_RECRUIT_SLOTS};

use data_model::{CharacterId, FactionId, GameData, SettlementId};
use serde::{Deserialize, Serialize};

use crate::dynasty;
use crate::research;
use crate::skills;
use crate::state::{Army, ArmyId, CampaignState, MoveTarget};

/// What an order did, for the orders whose effect the UI shows (lot M2).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum OrderOutcome {
    #[default]
    Done,
    /// A march: the points walked and why it stopped.
    Moved(crate::march::MoveReport),
}

impl CampaignState {
    /// Validates and applies an order of the player faction.
    pub fn submit_order(&mut self, data: &GameData, order: Order) -> Result<(), OrderError> {
        self.submit_order_outcome(data, order).map(|_| ())
    }

    /// Validates and applies an order of the player faction, and says what
    /// it did (lot M2: the march of a move order).
    pub fn submit_order_outcome(
        &mut self,
        data: &GameData,
        order: Order,
    ) -> Result<OrderOutcome, OrderError> {
        let player = self.player_faction.clone();
        // NT3: recruitments and hires count towards the player's missions.
        let recruits = u32::from(matches!(
            order,
            Order::Recruit { .. } | Order::HireMercenary { .. }
        ));
        let outcome = self.apply_order_outcome(data, &player, order)?;
        if recruits > 0 {
            crate::missions::note_units_recruited(self, &player, recruits);
        }
        Ok(outcome)
    }

    /// Validates and applies an order on behalf of `faction`.
    pub fn apply_order(
        &mut self,
        data: &GameData,
        faction: &FactionId,
        order: Order,
    ) -> Result<(), OrderError> {
        self.apply_order_outcome(data, faction, order).map(|_| ())
    }

    /// The settlement or point a move order heads for.
    pub fn resolve_move_target(&self, target: &MoveOrderTarget) -> Result<MoveTarget, OrderError> {
        match target {
            MoveOrderTarget::Place(place) => Ok(MoveTarget::Settlement(self.resolve_place(place)?)),
            MoveOrderTarget::Point { x, y } => Ok(MoveTarget::Point { x: *x, y: *y }),
            MoveOrderTarget::Path(places) => {
                let last = places.last().ok_or(OrderError::EmptyPath)?;
                Ok(MoveTarget::Settlement(self.resolve_place(last)?))
            }
        }
    }

    /// Validates and applies an order on behalf of `faction`, and says what
    /// it did. Events of immediate actions (battles, sieges, captures) open
    /// the journal of the turn.
    pub fn apply_order_outcome(
        &mut self,
        data: &GameData,
        faction: &FactionId,
        order: Order,
    ) -> Result<OrderOutcome, OrderError> {
        if !self.factions.contains_key(faction) {
            return Err(OrderError::UnknownFaction(faction.clone()));
        }
        let mut events = Vec::new();
        let outcome = match order {
            Order::MoveArmy { army, target } => {
                let target = self.resolve_move_target(&target)?;
                self.order_move_army(data, faction, &army, target, &mut events)
                    .map(OrderOutcome::Moved)
            }
            Order::Attack { army, target_army } => self
                .order_attack(data, faction, &army, &target_army, &mut events)
                .map(OrderOutcome::Moved),
            Order::Embark { army, to_port } => self
                .order_embark(data, faction, &army, &to_port, &mut events)
                .map(|()| OrderOutcome::Done),
            Order::ChooseEncounterOption { army, site, option } => crate::encounter::choose_option(
                self,
                data,
                faction,
                &army,
                site,
                option,
                &mut events,
            )
            .map_err(OrderError::from)
            .map(|()| OrderOutcome::Done),
            other => self
                .apply_simple_order(data, faction, other)
                .map(|()| OrderOutcome::Done),
        };
        self.pending_events.extend(events);
        outcome
    }

    fn apply_simple_order(
        &mut self,
        data: &GameData,
        faction: &FactionId,
        order: Order,
    ) -> Result<(), OrderError> {
        if let Some((target, treaty)) = order.proposal(self, faction) {
            if let Order::ProposeFactionMarriage {
                character, spouse, ..
            } = &order
            {
                self.check_owned_character(faction, character)?;
                if self.characters.get(spouse).map(|c| &c.faction) != Some(&target) {
                    return Err(OrderError::NotYourCharacter(target));
                }
            }
            return Ok(self.propose(data, faction, &target, treaty)?);
        }
        match order {
            Order::MoveArmy { .. }
            | Order::Attack { .. }
            | Order::Embark { .. }
            | Order::ChooseEncounterOption { .. } => {
                unreachable!("handled by apply_order_outcome")
            }
            Order::ProposePeace { .. }
            | Order::ProposeAlliance { .. }
            | Order::ProposeTreaty { .. }
            | Order::DemandVassalage { .. }
            | Order::ProposeFactionMarriage { .. } => {
                unreachable!("handled by Order::proposal")
            }
            Order::Recruit {
                settlement,
                unit_type,
            } => {
                let settlement = self.resolve_place(&settlement)?;
                if crate::capture::is_ruined(self, &settlement) {
                    return Err(OrderError::SettlementRuined);
                }
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
            Order::MergeArmies { source, target } => {
                self.order_merge(data, faction, &source, &target)
            }
            Order::SplitArmy { army, unit_indices } => {
                self.order_split(faction, &army, &unit_indices)
            }
            Order::GarrisonUnits { army, unit_indices } => {
                self.order_garrison(data, faction, &army, &unit_indices)
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
                self.own_army_mut(faction, &army)?;
                crate::posture::set_stance(self, data, &army, stance)
            }
            Order::AssignGeneral { army, character } => {
                self.order_assign_general(data, faction, &army, &character)
            }
            Order::Build {
                settlement,
                building,
            } => {
                let settlement = self.resolve_place(&settlement)?;
                if crate::capture::is_ruined(self, &settlement) {
                    return Err(OrderError::SettlementRuined);
                }
                self.order_build(data, faction, &settlement, &building)
            }
            Order::CancelBuild { settlement } => {
                let settlement = self.resolve_place(&settlement)?;
                self.order_cancel_build(data, faction, &settlement)
            }
            Order::CancelQueuedBuild { settlement, index } => {
                let settlement = self.resolve_place(&settlement)?;
                self.order_cancel_queued_build(data, faction, &settlement, index)
            }
            Order::Demolish {
                settlement,
                building,
            } => {
                let settlement = self.resolve_place(&settlement)?;
                crate::buildings::demolish(self, data, faction, &settlement, &building)
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
            Order::TransferCompanion {
                from,
                to,
                companion,
            } => {
                crate::retinue::transfer_companion(self, data, faction, &from, &to, &companion)?;
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
            Order::DebugGrantCompanion {
                character,
                companion,
            } => {
                crate::retinue::grant_companion(self, data, &character, &companion)?;
                Ok(())
            }
            Order::DeclareWar { target } => Ok(self.declare_war(data, faction, &target)?),
            Order::BreakAlliance { target } => Ok(self.break_alliance(data, faction, &target)?),
            Order::BreakTradeAgreement { target } => {
                Ok(self.break_trade_agreement(data, faction, &target)?)
            }
            Order::SetEmbargo { target, active } => {
                Ok(self.set_embargo(data, faction, &target, active)?)
            }
            Order::ReleaseVassal { target } => Ok(self.release_vassal(data, faction, &target)?),
            Order::SendGift { target, amount } => {
                Ok(self.send_gift(data, faction, &target, amount)?)
            }
            Order::AnswerOffer { offer, accept } => {
                Ok(self.answer_offer(data, faction, offer, accept)?)
            }
            Order::ArbitratePrivateWar { offer, verdict } => {
                Ok(self.arbitrate(data, faction, offer, verdict)?)
            }
            Order::DeclareCommise { vassal } => Ok(crate::feudal::declare_commise(
                self, data, faction, &vassal,
            )?),
            Order::GrantTitle { title, grantee } => {
                crate::feudal::grant_title(
                    self,
                    data,
                    faction,
                    &title,
                    crate::feudal::Grantee::Faction(grantee),
                )?;
                Ok(())
            }
            Order::Revolt => {
                crate::feudal::revolt(self, data, faction)?;
                Ok(())
            }
            Order::SwitchAllegiance { lord } => Ok(crate::feudal::switch_allegiance(
                self, data, faction, &lord,
            )?),
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
            Order::QueueResearch { technology } => {
                research::queue_research(self, data, faction, &technology)?;
                Ok(())
            }
            Order::DequeueResearch { technology } => {
                research::dequeue_research(self, data, faction, &technology);
                Ok(())
            }
            Order::ChooseEventOption { decision, option } => {
                Ok(self.choose_event_option(data, faction, decision, option)?)
            }
            Order::ChooseCaptureOutcome { decision, outcome } => {
                Ok(self.choose_capture_outcome(data, faction, decision, outcome)?)
            }
            Order::HireMercenary { army, unit } => {
                self.order_hire_mercenary(data, faction, &army, &unit)
            }
            Order::ChooseArmyTradition { army, tradition } => {
                self.choose_army_tradition(data, faction, &army, &tradition)
            }
            Order::SetDiet { province, diet } => {
                crate::table::set_diet(self, data, faction, &province, &diet)?;
                Ok(())
            }
            Order::SetEdict { province, edict } => {
                crate::edicts::set_edict(self, data, faction, &province, &edict)?;
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
            Order::PreachPassage => {
                crate::crusade::preach_passage(self, data, faction)?;
                Ok(())
            }
            Order::RecruitAgent { settlement, kind } => {
                let settlement = self.resolve_place(&settlement)?;
                self.recruit_agent(data, faction, &settlement, kind)?;
                Ok(())
            }
            Order::MoveAgent { agent, target } => {
                Ok(self.move_agent(data, faction, &agent, &target)?)
            }
            Order::AgentAction {
                agent,
                action,
                target,
                character,
            } => {
                self.agent_act(
                    data,
                    faction,
                    &agent,
                    action,
                    target.as_ref(),
                    character.as_ref(),
                )?;
                Ok(())
            }
            Order::DismissAgent { agent } => Ok(self.dismiss_agent(faction, &agent)?),
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

    pub(crate) fn own_army(&self, faction: &FactionId, id: &ArmyId) -> Result<&Army, OrderError> {
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
}
