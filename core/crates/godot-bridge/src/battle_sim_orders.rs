//! `BattleSim`: orders, abilities, AI switches, deployment and battle start.

use godot::prelude::*;
use sim_battle::Command;

use crate::battle_replay::{note, REPLAY_REFUSAL};
use crate::battle_sim::{parse_side, BattleSim};
use crate::convert::{from_dict, json_to_variant, resolve_reply, to_dict};

#[godot_api(secondary)]
impl BattleSim {
    /// `{type: "move"|"attack"|"halt"|"formation"|"fire_at_will"|"withdraw"
    /// |"target_wall"|"burn"|"leader_order", units: [ids], ...}` → `{ok, error}`.
    /// Siege fire (S2): `{type: "burn", units: [ids], house: i}` or
    /// `{type: "burn", units: [ids], gate: true}`.
    /// A leader's order: `{type: "leader_order", order: "order_war_cry",
    /// units: [ids] (selected-scope orders; empty = every eligible one)}`.
    #[func]
    fn issue_command(&mut self, command: VarDictionary) -> VarDictionary {
        self.touch_poses();
        if self.player.is_some() {
            return resolve_reply(Err(REPLAY_REFUSAL.to_owned()));
        }
        let Some(sim) = &mut self.sim else {
            return resolve_reply(Err("aucune bataille en cours".to_owned()));
        };
        let result = from_dict::<Command>(&command)
            .map_err(|e| crate::campaign_sim::invalid_order_message(&e))
            .and_then(|command| {
                note(
                    &mut self.recorder,
                    sim,
                    sim_battle::ReplayAction::Command {
                        command: command.clone(),
                    },
                );
                sim.issue_command(command).map_err(|e| e.to_string())
            });
        resolve_reply(result)
    }

    /// The leader's order bar of `side`: `[{id, kind, name, label,
    /// description, icon, available, reason, cooldown, cooldown_remaining,
    /// uses, uses_per_battle}]` by rank (`label` is the faction's wording,
    /// e.g. « Montjoie ! Saint-Denis ! »; `reason` is empty when available).
    #[func]
    fn get_leader_orders(&self, side: GString) -> VarArray {
        let (Some(sim), Some(side)) = (&self.sim, parse_side(&side)) else {
            return VarArray::new();
        };
        sim.leader_orders(side)
            .iter()
            .map(|view| {
                serde_json::to_value(view)
                    .map(|json| json_to_variant(&json))
                    .unwrap_or_default()
            })
            .collect()
    }

    /// RS-F: the « Incendier » order of the battle bar for `units` of `side`
    /// (every regiment of the side when empty): `{siege: false}` outside a
    /// siege; else `{siege: true, available, reason, command}` where
    /// `command` is the `burn` order to give (`{type: "burn", units: [id],
    /// house: i}` or `gate: true`) and `target` names it (« la porte », « une
    /// maison du faubourg », « une maison »). The core decides; nothing drawn.
    #[func]
    fn get_burn_order(&self, side: GString, units: PackedInt32Array) -> VarDictionary {
        let (Some(sim), Some(side)) = (&self.sim, parse_side(&side)) else {
            return vdict! { "siege" => false };
        };
        if sim.fire_rules().is_none() || sim.siege().is_none() {
            return vdict! { "siege" => false };
        }
        let ids: Vec<u32> = units
            .as_slice()
            .iter()
            .filter_map(|&id| u32::try_from(id).ok())
            .collect();
        match sim.burn_choice(side, &ids) {
            Ok(choice) => {
                let ids: VarArray = [i64::from(choice.unit).to_variant()].into_iter().collect();
                let mut command = vdict! { "type" => "burn", "units" => &ids };
                let target = match choice.house {
                    Some(house) => {
                        command.set("house", house as i64);
                        if choice.suburb {
                            "une maison du faubourg"
                        } else {
                            "une maison"
                        }
                    }
                    None => {
                        command.set("gate", true);
                        "la porte"
                    }
                };
                vdict! {
                    "siege" => true,
                    "available" => true,
                    "reason" => "",
                    "command" => &command,
                    "target" => target,
                    "unit" => i64::from(choice.unit),
                    "distance_m" => choice.distance_m,
                }
            }
            Err(error) => vdict! {
                "siege" => true,
                "available" => false,
                "reason" => error.to_string(),
            },
        }
    }

    /// CB4: uses (or lifts, when all of them use it) ability `ability` on
    /// those of `units` that have it: `{ok, error}` like `issue_command`
    /// (recorded for the replay the same way).
    #[func]
    fn use_ability(&mut self, units: PackedInt32Array, ability: GString) -> VarDictionary {
        let ids: VarArray = units
            .as_slice()
            .iter()
            .map(|&id| i64::from(id).to_variant())
            .collect();
        let mut command = VarDictionary::new();
        command.set("type", "use_ability");
        command.set("units", &ids);
        command.set("ability", &ability);
        self.issue_command(command)
    }

    /// CB4: `{id: {id, kind, rank, name, description, icon}}` of the
    /// battle's ability catalogue (texts of the tooltips; the state of each
    /// regiment's abilities is in `get_units`).
    #[func]
    fn get_ability_catalog(&self) -> VarDictionary {
        self.sim
            .as_ref()
            .map(crate::battle_sim_abilities::catalog_dict)
            .unwrap_or_default()
    }

    /// Did `side` give the "no quarter" order?
    #[func]
    fn get_no_quarter(&self, side: GString) -> bool {
        match (&self.sim, parse_side(&side)) {
            (Some(sim), Some(side)) => sim.no_quarter(side),
            _ => false,
        }
    }

    /// Lets the AI command `side` (`"attacker"`/`"defender"`), e.g. for autoplay.
    #[func]
    fn set_ai(&mut self, side: GString, enabled: bool) {
        if let Some(side) = parse_side(&side) {
            self.drive(sim_battle::ReplayAction::SetAi { side, enabled });
        }
    }

    /// NT11: "hold ground" for `side` (`"attacker"`/`"defender"`): its
    /// regiments keep their place (no AI move, no skirmish step back), the
    /// rout under pressure aside (guided battle prologue).
    #[func]
    fn set_hold(&mut self, side: GString, enabled: bool) {
        if let Some(side) = parse_side(&side) {
            self.drive(sim_battle::ReplayAction::SetHold { side, enabled });
        }
    }

    /// NT11: does `side` hold its ground?
    #[func]
    fn get_hold(&self, side: GString) -> bool {
        match (&self.sim, parse_side(&side)) {
            (Some(sim), Some(side)) => sim.holds(side),
            _ => false,
        }
    }

    /// F5a: opens the deployment phase (call right after `setup`, before
    /// any `tick`). `false` once the battle has started.
    #[func]
    fn begin_deployment(&mut self) -> bool {
        self.touch_poses();
        if self.player.is_some() {
            return false;
        }
        let Some(sim) = &mut self.sim else {
            return false;
        };
        note(
            &mut self.recorder,
            sim,
            sim_battle::ReplayAction::BeginDeployment,
        );
        sim.begin_deployment()
    }

    /// `true` during the deployment phase (ticks do nothing).
    #[func]
    fn is_deploying(&self) -> bool {
        self.sim.as_ref().is_some_and(|sim| sim.is_deploying())
    }

    /// `{x0, z0, x1, z1}` rectangle of `side` (field metres); empty if unknown
    /// or when the side has no deployment phase (CV3-2: caught in column,
    /// forced march). An ambusher may have a second zone: see
    /// [`Self::get_deployment_zones`].
    #[func]
    fn get_deployment_zone(&self, side: GString) -> VarDictionary {
        match (&self.sim, parse_side(&side)) {
            (Some(sim), Some(side)) if sim.can_deploy(side) => to_dict(&sim.deployment_zone(side)),
            _ => VarDictionary::new(),
        }
    }

    /// CV3-2: every `{x0, z0, x1, z1}` deployment zone of `side` (the one or
    /// two flanks of an ambusher; none for a side that cannot deploy).
    #[func]
    fn get_deployment_zones(&self, side: GString) -> VarArray {
        match (&self.sim, parse_side(&side)) {
            (Some(sim), Some(side)) => sim
                .deployment_zones(side)
                .iter()
                .map(|z| to_dict(z).to_variant())
                .collect(),
            _ => VarArray::new(),
        }
    }

    /// CV3-2: how the battle opened. `{kind: standard|ambush, victim:
    /// attacker|defender|"", on_road, path: PackedVector2Array (column centre
    /// line, head last), attacker_forced_march, defender_forced_march,
    /// attacker_entrenched, defender_entrenched, attacker_can_deploy,
    /// defender_can_deploy}`; empty before `setup`.
    #[func]
    fn get_opening(&self) -> VarDictionary {
        let Some(sim) = &self.sim else {
            return VarDictionary::new();
        };
        let setup = sim.setup();
        let layout = sim.ambush_layout();
        let path: PackedVector2Array = layout
            .map(|l| {
                l.path
                    .iter()
                    .map(|p| Vector2::new(p.0 as f32, p.1 as f32))
                    .collect()
            })
            .unwrap_or_default();
        let mut dict = vdict! {
            "kind" => if setup.opening.is_standard() { "standard" } else { "ambush" },
            "victim" => setup.opening.ambush_victim().map_or("", |s| s.key()),
            "on_road" => layout.is_some_and(|l| l.on_road),
            "path" => &path,
        };
        for side in sim_battle::SideId::BOTH {
            let key = side.key();
            let s = setup.side(side);
            dict.set(format!("{key}_forced_march").as_str(), s.forced_march);
            dict.set(format!("{key}_entrenched").as_str(), s.entrenched);
            dict.set(format!("{key}_can_deploy").as_str(), sim.can_deploy(side));
        }
        dict
    }

    /// Places regiment `id` at (x, z); `facing` in radians, NaN keeps the
    /// current facing; CB1 `width` (metres, optional, ≤ 0 for none): the
    /// frontage of a right-drag, the regiment forming a Line that wide.
    /// → `{ok, error}` (French error).
    #[func]
    fn deploy_unit(
        &mut self,
        id: i64,
        x: f64,
        z: f64,
        facing: f64,
        #[opt(default = 0.0)] width: f64,
    ) -> VarDictionary {
        self.touch_poses();
        if self.player.is_some() {
            return resolve_reply(Err(REPLAY_REFUSAL.to_owned()));
        }
        let Some(sim) = &mut self.sim else {
            return resolve_reply(Err("aucune bataille en cours".to_owned()));
        };
        let facing = facing.is_finite().then_some(facing);
        let width = (width.is_finite() && width > 0.0).then_some(width);
        let result = u32::try_from(id)
            .map_err(|_| format!("unité inconnue : {id}"))
            .and_then(|id| {
                note(
                    &mut self.recorder,
                    sim,
                    sim_battle::ReplayAction::DeployUnit {
                        unit: id,
                        x,
                        z,
                        facing,
                        width,
                    },
                );
                sim.deploy_unit_width(id, x, z, facing, width)
                    .map_err(|e| sim.error_text(&e))
            });
        resolve_reply(result)
    }

    /// Ends the deployment phase → `{ok, error}`.
    #[func]
    fn start_battle(&mut self) -> VarDictionary {
        self.touch_poses();
        if self.player.is_some() {
            return resolve_reply(Err(REPLAY_REFUSAL.to_owned()));
        }
        let Some(sim) = &mut self.sim else {
            return resolve_reply(Err("aucune bataille en cours".to_owned()));
        };
        note(
            &mut self.recorder,
            sim,
            sim_battle::ReplayAction::StartBattle,
        );
        resolve_reply(sim.start_battle().map_err(|e| e.to_string()))
    }
}
