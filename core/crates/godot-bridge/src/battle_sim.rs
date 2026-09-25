//! M7 battles on the GDExtension side (spec `docs/design/m7-battles.md` § 3):
//! the `BattleSim` class and the pending-battle methods of `CampaignSim`
//! (a secondary `#[godot_api]` block, so `campaign_sim.rs` stays untouched).
//!
//! Setups, commands and outcomes cross the boundary as `Dictionary`s converted
//! through JSON with the serde types of `sim-battle`; positions go out as
//! packed float arrays.

use data_model::UnitCategory;
use godot::classes::RefCounted;
use godot::prelude::*;
use serde_json::Value;
use sim_battle::{BattleOutcome, BattleSetup, Command, SideId, Unit, UnitState};

use crate::campaign_sim::{events_array, CampaignSim};
use crate::convert::variant_to_json;

/// Converts JSON into plain Godot values (integers stay integers).
pub(crate) fn json_to_variant(value: &Value) -> Variant {
    match value {
        Value::Null => Variant::nil(),
        Value::Bool(b) => b.to_variant(),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                i.to_variant()
            } else if let Some(u) = n.as_u64() {
                (u as i64).to_variant()
            } else {
                n.as_f64().unwrap_or(0.0).to_variant()
            }
        }
        Value::String(s) => GString::from(s.as_str()).to_variant(),
        Value::Array(items) => items
            .iter()
            .map(json_to_variant)
            .collect::<VarArray>()
            .to_variant(),
        Value::Object(map) => {
            let mut dict = VarDictionary::new();
            for (key, item) in map {
                dict.set(key.as_str(), &json_to_variant(item));
            }
            dict.to_variant()
        }
    }
}

fn to_dict<T: serde::Serialize>(value: &T) -> VarDictionary {
    serde_json::to_value(value)
        .ok()
        .map(|json| json_to_variant(&json))
        .and_then(|variant| variant.try_to::<VarDictionary>().ok())
        .unwrap_or_default()
}

fn from_dict<T: serde::de::DeserializeOwned>(dict: &VarDictionary) -> Result<T, String> {
    variant_to_json(&dict.to_variant())
        .and_then(|json| serde_json::from_value::<T>(json).map_err(|e| e.to_string()))
}

fn result_dict(result: Result<(), String>) -> VarDictionary {
    match result {
        Ok(()) => vdict! { "ok" => true, "error" => "" },
        Err(error) => vdict! { "ok" => false, "error" => error.as_str() },
    }
}

fn parse_side(raw: &GString) -> Option<SideId> {
    SideId::parse(&raw.to_string())
}

/// Rendering family of a regiment: `infantry`, `archer`, `cavalry`, `siege`,
/// or `ram` / `tower` (siege battles: drawn as one machine, not soldiers).
fn render_key(unit: &Unit) -> &'static str {
    if unit.ram {
        return "ram";
    }
    if unit.siege_tower() {
        return "tower";
    }
    match unit.category {
        UnitCategory::Siege => "siege",
        _ if unit.mounted => "cavalry",
        UnitCategory::Ranged => "archer",
        UnitCategory::Cavalry => "cavalry",
        UnitCategory::Infantry => "infantry",
    }
}

fn category_key(category: UnitCategory) -> &'static str {
    match category {
        UnitCategory::Infantry => "infantry",
        UnitCategory::Ranged => "ranged",
        UnitCategory::Cavalry => "cavalry",
        UnitCategory::Siege => "siege",
    }
}

fn state_label_fr(state: UnitState) -> &'static str {
    match state {
        UnitState::Idle => "au repos",
        UnitState::Marching => "en marche",
        UnitState::Charging => "charge",
        UnitState::Melee => "mêlée",
        UnitState::Shooting => "tir",
        UnitState::Routing => "déroute",
        UnitState::Rallied => "rallié",
        UnitState::Climbing => "escalade",
    }
}

/// Godot-facing handle on one battle (spec § 3).
#[derive(GodotClass)]
#[class(base = RefCounted)]
pub struct BattleSim {
    sim: Option<sim_battle::BattleSim>,
    base: Base<RefCounted>,
}

#[godot_api]
impl IRefCounted for BattleSim {
    fn init(base: Base<RefCounted>) -> Self {
        BattleSim { sim: None, base }
    }
}

#[godot_api]
impl BattleSim {
    /// Builds the battle from a `CampaignSim.get_battle_setup` dictionary.
    #[func]
    fn setup(&mut self, setup: VarDictionary, seed: i64) -> bool {
        let parsed = from_dict::<BattleSetup>(&setup).and_then(|setup| {
            sim_battle::BattleSim::new(setup, seed as u64).map_err(|e| e.to_string())
        });
        match parsed {
            Ok(sim) => {
                self.sim = Some(sim);
                true
            }
            Err(error) => {
                godot_error!("BattleSim.setup: {error}");
                self.sim = None;
                false
            }
        }
    }

    /// Advances the battle by `dt` seconds (fixed 0.1 s steps inside).
    #[func]
    fn tick(&mut self, dt: f64) {
        if let Some(sim) = &mut self.sim {
            sim.tick(dt);
        }
    }

    /// `{type: "move"|"attack"|"halt"|"formation"|"fire_at_will"|"withdraw"
    /// |"target_wall"|"burn"|"leader_order", units: [ids], ...}` → `{ok, error}`.
    /// Siege fire (S2): `{type: "burn", units: [ids], house: i}` or
    /// `{type: "burn", units: [ids], gate: true}`.
    /// A leader's order: `{type: "leader_order", order: "order_war_cry",
    /// units: [ids] (selected-scope orders; empty = every eligible one)}`.
    #[func]
    fn issue_command(&mut self, command: VarDictionary) -> VarDictionary {
        let Some(sim) = &mut self.sim else {
            return result_dict(Err("aucune bataille en cours".to_owned()));
        };
        let result = from_dict::<Command>(&command)
            .map_err(|e| crate::campaign_sim::invalid_order_message(&e))
            .and_then(|command| sim.issue_command(command).map_err(|e| e.to_string()));
        result_dict(result)
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
        if let (Some(sim), Some(side)) = (&mut self.sim, parse_side(&side)) {
            sim.set_ai(side, enabled);
        }
    }

    /// F5a: opens the deployment phase (call right after `setup`, before
    /// any `tick`). `false` once the battle has started.
    #[func]
    fn begin_deployment(&mut self) -> bool {
        self.sim.as_mut().is_some_and(|sim| sim.begin_deployment())
    }

    /// `true` during the deployment phase (ticks do nothing).
    #[func]
    fn is_deploying(&self) -> bool {
        self.sim.as_ref().is_some_and(|sim| sim.is_deploying())
    }

    /// `{x0, z0, x1, z1}` rectangle of `side` (field metres); empty if unknown.
    #[func]
    fn get_deployment_zone(&self, side: GString) -> VarDictionary {
        match (&self.sim, parse_side(&side)) {
            (Some(sim), Some(side)) => to_dict(&sim.deployment_zone(side)),
            _ => VarDictionary::new(),
        }
    }

    /// Places regiment `id` at (x, z); `facing` in radians, NaN keeps the
    /// current facing. → `{ok, error}` (French error).
    #[func]
    fn deploy_unit(&mut self, id: i64, x: f64, z: f64, facing: f64) -> VarDictionary {
        let Some(sim) = &mut self.sim else {
            return result_dict(Err("aucune bataille en cours".to_owned()));
        };
        let facing = facing.is_finite().then_some(facing);
        let result = u32::try_from(id)
            .map_err(|_| format!("unité inconnue : {id}"))
            .and_then(|id| {
                sim.deploy_unit(id, x, z, facing)
                    .map_err(|e| sim.error_text(&e))
            });
        result_dict(result)
    }

    /// Ends the deployment phase → `{ok, error}`.
    #[func]
    fn start_battle(&mut self) -> VarDictionary {
        let Some(sim) = &mut self.sim else {
            return result_dict(Err("aucune bataille en cours".to_owned()));
        };
        result_dict(sim.start_battle().map_err(|e| e.to_string()))
    }

    /// One dictionary per regiment (spec § 3), plus rendering helpers.
    #[func]
    fn get_units(&self) -> VarArray {
        let Some(sim) = &self.sim else {
            return VarArray::new();
        };
        sim.units()
            .iter()
            .map(|unit| {
                let (width, depth) = unit.extent();
                let mut dict = vdict! {
                    "id" => i64::from(unit.id),
                    "side" => unit.side.key(),
                    "type" => unit.unit_type.as_str(),
                    "name" => unit.name.as_str(),
                    "category" => category_key(unit.category),
                    "render" => render_key(unit),
                    "soldiers" => i64::from(unit.soldiers()),
                    "max_soldiers" => i64::from(unit.max_soldiers),
                    "initial_soldiers" => i64::from(unit.initial_soldiers),
                    "kills" => unit.kills.round() as i64,
                    "morale" => unit.morale,
                    "fatigue" => unit.fatigue,
                    "ammo" => i64::from(unit.ammo),
                    "max_ammo" => i64::from(if unit.can_shoot() { unit.stats.ammo } else { 0 }),
                    "state" => unit.state.key(),
                    "state_label" => state_label_fr(unit.state),
                    "formation" => unit.formation.key(),
                    "x" => unit.x,
                    "z" => unit.z,
                    "y" => sim.standing_height(unit, unit.x, unit.z),
                    "facing" => unit.facing,
                    "width" => width,
                    "depth" => depth,
                    "is_general" => unit.is_general,
                    "present" => unit.present(),
                    "reserve" => unit.reserve,
                    "left_field" => unit.left_field,
                    "fire_at_will" => unit.fire_at_will,
                    "can_shoot" => unit.can_shoot(),
                    "running" => unit.running,
                    "withdrawing" => unit.withdrawing,
                    "stakes" => unit.stakes_planted,
                    "target" => unit.target.map_or(-1, i64::from),
                    "on_wall" => unit.on_wall,
                    "climbing" => unit.climbing.map_or(-1, |p| p as i64),
                    "climb_progress" => unit.climb_progress,
                    "ladders" => sim.on_ladders(unit),
                    "synthetic" => unit.synthetic,
                    "ram" => unit.ram,
                    "siege_tower" => unit.siege_tower(),
                    "wall_breaker" => unit.wall_breaker(),
                    "pavise" => unit.pavise.is_some(),
                    "dismounted" => unit.dismounted,
                    "order_morale" => unit.order_morale,
                };
                if let Some((x, z)) = unit.destination {
                    dict.set("destination", Vector2::new(x as f32, z as f32));
                }
                dict.to_variant()
            })
            .collect()
    }

    /// `(x, y, z, angle)` per living soldier of `side`.
    #[func]
    fn get_soldier_transforms(&self, side: GString) -> PackedFloat32Array {
        let (Some(sim), Some(side)) = (&self.sim, parse_side(&side)) else {
            return PackedFloat32Array::new();
        };
        sim.soldier_transforms(side, None)
            .iter()
            .flat_map(|t| t.map(|v| v as f32))
            .collect()
    }

    /// `MultiMesh.buffer` (12 floats per instance, rotation about Y) of the
    /// living soldiers of `side` whose regiment renders as `render`
    /// (`infantry`, `archer`, `cavalry`, `siege`).
    #[func]
    fn get_soldier_buffer(&self, side: GString, render: GString) -> PackedFloat32Array {
        let (Some(sim), Some(side)) = (&self.sim, parse_side(&side)) else {
            return PackedFloat32Array::new();
        };
        let render = render.to_string();
        let mut buffer: Vec<f32> = Vec::new();
        for unit in sim
            .units()
            .iter()
            .filter(|u| u.side == side && render_key(u) == render)
        {
            for (x, z, angle) in unit.soldier_positions() {
                let y = sim.standing_height(unit, x, z);
                let (s, c) = (angle.sin() as f32, angle.cos() as f32);
                buffer.extend_from_slice(&[
                    c, 0.0, s, x as f32, 0.0, 1.0, 0.0, y as f32, -s, 0.0, c, z as f32,
                ]);
            }
        }
        PackedFloat32Array::from(buffer.as_slice())
    }

    /// `{width, depth, resolution, nx, nz, heights, forests[{x, z, radius}],
    /// mud[..], river?{points: PackedVector2Array, width, fords[{x, z, half_width}]},
    /// siege?{...}}` (siege geometry: see [`Self::get_siege`]). B5 (campaign site):
    /// `terrain` (province terrain key), `season`, `ground` (`dry|muddy|snowy`),
    /// `ground_label`, `site_label` (B6), `woodland` (0-1), `pools[{x, z, radius}]`,
    /// `obstacles[{a: Vector2, b: Vector2, kind: hedge|fence|ditch}]`,
    /// `coast?{flank: west|east, shore_x, beach}`,
    /// `village?{x, z, radius, farm, houses[{x, z, length, width, yaw, kind}]}`.
    #[func]
    fn get_terrain(&self) -> VarDictionary {
        let Some(sim) = &self.sim else {
            return VarDictionary::new();
        };
        let field = sim.field();
        let zones = |zones: &[sim_battle::Zone]| -> VarArray {
            zones
                .iter()
                .map(|z| vdict! { "x" => z.x, "z" => z.z, "radius" => z.radius }.to_variant())
                .collect()
        };
        let heights: PackedFloat32Array = field.heights.iter().map(|h| *h as f32).collect();
        let mut dict = vdict! {
            "width" => field.width,
            "depth" => field.depth,
            "resolution" => field.resolution,
            "nx" => field.nx as i64,
            "nz" => field.nz as i64,
            "heights" => &heights,
            "forests" => &zones(&field.forests),
            "mud" => &zones(&field.mud),
            // B5: campaign site.
            "terrain" => field.terrain.key(),
            "season" => season_key(field.season),
            "ground" => field.ground.key(),
            "ground_label" => field.ground.label_fr(),
            // B6: the site in one compact line (pre-battle dialog, HUD).
            "site_label" => field.site_label_fr(),
            "woodland" => field.woodland,
            "pools" => &zones(&field.pools),
            "obstacles" => &field
                .obstacles
                .iter()
                .map(|o| {
                    vdict! {
                        "a" => Vector2::new(o.a.0 as f32, o.a.1 as f32),
                        "b" => Vector2::new(o.b.0 as f32, o.b.1 as f32),
                        "kind" => o.kind.key(),
                    }
                    .to_variant()
                })
                .collect::<VarArray>(),
        };
        if let Some(coast) = &field.coast {
            let flank = match coast.flank {
                sim_battle::Flank::West => "west",
                sim_battle::Flank::East => "east",
            };
            dict.set(
                "coast",
                &vdict! { "flank" => flank, "shore_x" => coast.shore_x, "beach" => coast.beach },
            );
        }
        if let Some(village) = &field.village {
            let houses: VarArray = village
                .houses
                .iter()
                .map(|h| {
                    vdict! {
                        "x" => h.x, "z" => h.z, "length" => h.length, "width" => h.width,
                        "yaw" => h.yaw, "kind" => h.kind.key(),
                    }
                    .to_variant()
                })
                .collect();
            dict.set(
                "village",
                &vdict! {
                    "x" => village.zone.x, "z" => village.zone.z, "radius" => village.zone.radius,
                    "farm" => village.farm, "houses" => &houses,
                },
            );
        }
        if let Some(river) = &field.river {
            let points: PackedVector2Array = river
                .polyline(10.0)
                .iter()
                .map(|(x, z)| Vector2::new(*x as f32, *z as f32))
                .collect();
            let fords: VarArray = river
                .fords
                .iter()
                .map(|f| {
                    vdict! { "x" => f.x, "z" => river.center_z(f.x), "half_width" => f.half_width }
                        .to_variant()
                })
                .collect();
            dict.set(
                "river",
                &vdict! { "points" => &points, "width" => river.width, "fords" => &fords },
            );
        }
        if sim.siege().is_some() {
            dict.set("siege", &self.get_siege());
        }
        dict
    }

    /// Siege battle walls (empty dictionary in a field battle):
    /// `{fortification, center: Vector2, square_radius, thickness, wall_height,
    /// gate, hold_time, hold_to_win, integrity, pieces[{index, kind: "wall"|"gate",
    /// a: Vector2, b: Vector2, hp, max_hp, intact, docked_tower}],
    /// towers[{x, z, radius, height}], houses[{x, z, radius, suburb, fire: {state:
    /// "intact"|"burning"|"burnt", intensity}}], gate_fire: {state, intensity},
    /// wind: Vector2 (direction × strength 0-1), houses_burning, houses_burnt,
    /// sortie}`. Pieces lose HP and houses burn during the battle (S2): call it
    /// again to show the damage.
    #[func]
    fn get_siege(&self) -> VarDictionary {
        let Some(works) = self.sim.as_ref().and_then(|s| s.siege()) else {
            return VarDictionary::new();
        };
        let v2 = |p: (f64, f64)| Vector2::new(p.0 as f32, p.1 as f32);
        let pieces: VarArray = works
            .pieces
            .iter()
            .enumerate()
            .map(|(index, piece)| {
                vdict! {
                    "index" => index as i64,
                    "kind" => match piece.kind {
                        sim_battle::PieceKind::Wall => "wall",
                        sim_battle::PieceKind::Gate => "gate",
                    },
                    "a" => v2(piece.a),
                    "b" => v2(piece.b),
                    "hp" => piece.hp,
                    "max_hp" => piece.max_hp,
                    "intact" => piece.intact(),
                    "docked_tower" => piece.docked_tower.map_or(-1, i64::from),
                }
                .to_variant()
            })
            .collect();
        let towers: VarArray = works
            .towers
            .iter()
            .map(|t| {
                vdict! { "x" => t.x, "z" => t.z, "radius" => t.radius, "height" => t.height }
                    .to_variant()
            })
            .collect();
        // F5a: house blocks (obstacles of the siege pathing), `{x, z, radius}`;
        // S2: their fire (a burnt house no longer blocks) and the suburbs.
        let blaze = |b: &sim_battle::Blaze| {
            vdict! { "state" => b.state.key(), "intensity" => b.intensity }
        };
        let houses: VarArray = works
            .houses
            .iter()
            .map(|h| {
                vdict! {
                    "x" => h.x,
                    "z" => h.z,
                    "radius" => h.radius,
                    "suburb" => h.suburb,
                    "fire" => &blaze(&h.fire),
                }
                .to_variant()
            })
            .collect();
        vdict! {
            "fortification" => i64::from(works.fortification),
            "center" => v2(works.center),
            "square_radius" => works.square_radius,
            "thickness" => works.thickness,
            "wall_height" => works.wall_height,
            "gate" => works.gate as i64,
            "hold_time" => works.hold_time,
            "hold_to_win" => sim_battle::siege::HOLD_TO_WIN,
            "integrity" => works.integrity(),
            "pieces" => &pieces,
            "towers" => &towers,
            "houses" => &houses,
            "sortie" => works.sortie,
            "gate_fire" => &blaze(&works.gate_fire),
            "wind" => v2(works.wind),
            "houses_burning" => works.burning_houses() as i64,
            "houses_burnt" => works.burnt_houses() as i64,
        }
    }

    /// Debug (tests, captures, S2): sets house `house` on fire, or the gate
    /// when `house` < 0; `false` when it already burns or is not a siege.
    #[func]
    fn debug_ignite(&mut self, house: i64) -> bool {
        let Some(sim) = &mut self.sim else {
            return false;
        };
        if house < 0 {
            sim.ignite_gate()
        } else {
            sim.ignite_house(house as usize)
        }
    }

    /// Ground height at (x, z).
    #[func]
    fn get_height(&self, x: f64, z: f64) -> f64 {
        self.sim.as_ref().map_or(0.0, |s| s.field().height(x, z))
    }

    /// B6: the battle site in one compact French line, e.g. « Terre gelée ·
    /// hiver · village · haies · côte ouest » (empty without a battle).
    #[func]
    fn get_site_label(&self) -> GString {
        self.sim
            .as_ref()
            .map(|sim| GString::from(sim.field().site_label_fr().as_str()))
            .unwrap_or_default()
    }

    /// `{key: "clear"|"rain"|"fog"|"snow", label}`.
    #[func]
    fn get_weather(&self) -> VarDictionary {
        let Some(sim) = &self.sim else {
            return VarDictionary::new();
        };
        let weather = sim.weather();
        vdict! { "key" => weather.key(), "label" => weather.label_fr() }
    }

    /// The setup the battle was built from (names, factions, general).
    #[func]
    fn get_setup(&self) -> VarDictionary {
        self.sim
            .as_ref()
            .map(|s| to_dict(s.setup()))
            .unwrap_or_default()
    }

    /// Head count still able to fight on `side`.
    #[func]
    fn get_strength(&self, side: GString) -> i64 {
        match (&self.sim, parse_side(&side)) {
            (Some(sim), Some(side)) => i64::from(sim.strength(side)),
            _ => 0,
        }
    }

    /// Simulated seconds since the start.
    #[func]
    fn get_elapsed(&self) -> f64 {
        self.sim.as_ref().map_or(0.0, |s| s.elapsed())
    }

    #[func]
    fn get_ticks(&self) -> i64 {
        self.sim.as_ref().map_or(0, |s| s.ticks() as i64)
    }

    #[func]
    fn is_finished(&self) -> bool {
        self.sim.as_ref().is_some_and(|s| s.is_finished())
    }

    /// Result in the format accepted by `CampaignSim.resolve_battle` (empty
    /// while the battle goes on).
    #[func]
    fn get_outcome(&self) -> VarDictionary {
        self.sim
            .as_ref()
            .and_then(|s| s.outcome())
            .map(|o| to_dict(&o))
            .unwrap_or_default()
    }

    /// Journal entries `[{time, text_fr, side}]` added since the last call.
    #[func]
    fn get_events(&mut self) -> VarArray {
        let Some(sim) = &mut self.sim else {
            return VarArray::new();
        };
        sim.take_new_events()
            .iter()
            .map(|event| {
                vdict! {
                    "time" => event.time,
                    "text_fr" => event.text_fr.as_str(),
                    "side" => event.side.map_or("", |s| s.key()),
                }
                .to_variant()
            })
            .collect()
    }
}

#[godot_api(secondary)]
impl CampaignSim {
    /// `[{index, attacker, defender, province, province_name, attacker_name,
    /// defender_name, player_side, attacker_strength, defender_strength, seed}]`.
    #[func]
    fn get_pending_battles(&self) -> VarArray {
        let (Some(state), Some(data)) = (&self.state, &self.data) else {
            return VarArray::new();
        };
        state
            .pending_battle_views(data)
            .iter()
            .map(|view| {
                let strength = |id: &sim_campaign::ArmyId| {
                    state.army(id).map_or(0, |a| i64::from(a.total_strength()))
                };
                let settlement_state = state.settlement_state(&view.location);
                let defender_strength = if view.siege {
                    settlement_state.map_or(0, |s| {
                        s.garrison.iter().map(|u| i64::from(u.strength)).sum()
                    })
                } else {
                    strength(&view.defender)
                };
                let breach = settlement_state
                    .and_then(|s| s.siege.as_ref())
                    .map_or(0, |s| i64::from(s.breach));
                let settlement_name = data
                    .settlements
                    .get(&view.location)
                    .map_or_else(|| view.location.to_string(), |s| s.name.display.clone());
                let province_name = data
                    .provinces
                    .get(&view.province)
                    .map_or_else(|| view.province.to_string(), |p| p.name.display.clone());
                let seed = state.seed
                    ^ (u64::from(state.turn()) << 20)
                    ^ ((view.index as u64) << 8)
                    ^ 0xBA77;
                vdict! {
                    "index" => view.index as i64,
                    "attacker" => view.attacker.as_str(),
                    "defender" => view.defender.as_str(),
                    "province" => view.province.as_str(),
                    "province_name" => province_name.as_str(),
                    "attacker_name" => view.attacker_name.as_str(),
                    "defender_name" => view.defender_name.as_str(),
                    "player_side" => view.player_side.map_or("", |s| s.key()),
                    "attacker_strength" => strength(&view.attacker),
                    "defender_strength" => defender_strength,
                    "siege" => view.siege,
                    "fortification" => i64::from(state.fortification_level(data, &view.location)),
                    "location" => view.location.as_str(),
                    "settlement_name" => settlement_name.as_str(),
                    "settlement_kind" => state.settlement_kind(&view.location).key(),
                    "breach" => breach,
                    "seed" => (seed & 0x7FFF_FFFF_FFFF) as i64,
                }
                .to_variant()
            })
            .collect()
    }

    /// Setup of pending battle `index` for `BattleSim.setup` (empty if unknown).
    #[func]
    fn get_battle_setup(&self, index: i64) -> VarDictionary {
        let (Some(state), Some(data)) = (&self.state, &self.data) else {
            return VarDictionary::new();
        };
        match state.battle_setup(data, index.max(0) as usize) {
            Ok(setup) => to_dict(&setup),
            Err(error) => {
                godot_warn!("CampaignSim.get_battle_setup({index}): {error}");
                VarDictionary::new()
            }
        }
    }

    /// UB1: estimated balance of pending battle `index` for the pre-battle
    /// screen (`battle_forecast.rs`): `{attacker_power, defender_power,
    /// attacker_share, attacker_win_chance, attacker_soldiers,
    /// defender_soldiers, attacker_reinforcements, defender_reinforcements,
    /// modifiers, can_withdraw, siege}`; empty if unknown.
    #[func]
    fn get_battle_forecast(&self, index: i64) -> VarDictionary {
        let (Some(state), Some(data)) = (&self.state, &self.data) else {
            return VarDictionary::new();
        };
        match state.battle_forecast(data, index.max(0) as usize) {
            Ok(forecast) => to_dict(&forecast),
            Err(error) => {
                godot_warn!("CampaignSim.get_battle_forecast({index}): {error}");
                VarDictionary::new()
            }
        }
    }

    /// UB1: the player calls off pending battle `index` (attacker only; an
    /// assault is postponed, the siege goes on) → `{ok, error, events}`.
    #[func]
    fn withdraw_pending_battle(&mut self, index: i64) -> VarDictionary {
        let (Some(state), Some(data)) = (&mut self.state, &self.data) else {
            return result_dict(Err("aucune campagne en cours".to_owned()));
        };
        match state.withdraw_pending_battle(data, index.max(0) as usize) {
            Ok(events) => {
                let mut dict = result_dict(Ok(()));
                dict.set("events", &events_array(&events));
                dict
            }
            Err(error) => result_dict(Err(error.to_string())),
        }
    }

    /// Applies a `BattleSim.get_outcome()` dictionary → `{ok, error, events}`.
    #[func]
    fn resolve_battle(&mut self, index: i64, outcome: VarDictionary) -> VarDictionary {
        let (Some(state), Some(data)) = (&mut self.state, &self.data) else {
            return result_dict(Err("aucune campagne en cours".to_owned()));
        };
        let result = from_dict::<BattleOutcome>(&outcome)
            .map_err(|e| format!("résultat invalide : {e}"))
            .and_then(|outcome| {
                state
                    .resolve_pending_battle(data, index.max(0) as usize, &outcome)
                    .map_err(|e| e.to_string())
            });
        match result {
            Ok(events) => {
                let mut dict = result_dict(Ok(()));
                dict.set("events", &events_array(&events));
                dict
            }
            Err(error) => result_dict(Err(error)),
        }
    }

    /// Auto-resolves pending battle `index` now; returns its events.
    #[func]
    fn auto_resolve_battle(&mut self, index: i64) -> VarArray {
        let (Some(state), Some(data)) = (&mut self.state, &self.data) else {
            return VarArray::new();
        };
        match state.auto_resolve_pending(data, index.max(0) as usize) {
            Ok(events) => events_array(&events),
            Err(error) => {
                godot_warn!("CampaignSim.auto_resolve_battle({index}): {error}");
                VarArray::new()
            }
        }
    }

    /// Player setting: fight own battles in 3D (`true`, default) or always
    /// auto-resolve them.
    #[func]
    fn set_interactive_battles(&mut self, enabled: bool) {
        if let Some(state) = &mut self.state {
            state.interactive_battles = enabled;
        }
    }

    #[func]
    fn get_interactive_battles(&self) -> bool {
        self.state.as_ref().is_some_and(|s| s.interactive_battles)
    }

    /// Debug (smoke test, screenshots): puts `army` in siege of `province`
    /// (garrisoned if empty) and records a pending siege battle; returns its
    /// index or -1.
    #[func]
    fn debug_stage_siege(&mut self, army: GString, province: GString) -> i64 {
        let (Some(state), Some(data)) = (&mut self.state, &self.data) else {
            return -1;
        };
        let Some(army) = sim_campaign::ArmyId::parse(&army.to_string()) else {
            return -1;
        };
        let Ok(province) = data_model::ProvinceId::new(province.to_string().as_str()) else {
            return -1;
        };
        match state.debug_stage_siege(data, &army, &province) {
            Ok(index) => index as i64,
            Err(error) => {
                godot_warn!("CampaignSim.debug_stage_siege: {error}");
                -1
            }
        }
    }

    /// Debug (smoke test, screenshots): brings `defender` to `attacker` and
    /// records a pending battle; returns its index or -1.
    #[func]
    fn debug_stage_battle(&mut self, attacker: GString, defender: GString) -> i64 {
        let Some(state) = &mut self.state else {
            return -1;
        };
        let (Some(a), Some(d)) = (
            sim_campaign::ArmyId::parse(&attacker.to_string()),
            sim_campaign::ArmyId::parse(&defender.to_string()),
        ) else {
            return -1;
        };
        match state.debug_stage_battle(&a, &d) {
            Ok(index) => index as i64,
            Err(error) => {
                godot_warn!("CampaignSim.debug_stage_battle: {error}");
                -1
            }
        }
    }
}

/// `snake_case` key of a battle season (B5, `get_terrain`).
fn season_key(season: sim_battle::BattleSeason) -> &'static str {
    match season {
        sim_battle::BattleSeason::Spring => "spring",
        sim_battle::BattleSeason::Summer => "summer",
        sim_battle::BattleSeason::Autumn => "autumn",
        sim_battle::BattleSeason::Winter => "winter",
    }
}
