//! `BattleSim`: terrain, decor, sieges, heights, weather and camps.

use godot::prelude::*;

use crate::battle_replay::note;
use crate::battle_sim::BattleSim;

/// BR3: a piece of street furniture for the renderer, `{kind, x, z, yaw,
/// length, depth, house}` (`house` = -1 on the market square; yaw of the
/// core: length along (cos, sin), front along (-sin, cos)).
fn prop_dict(prop: &sim_battle::Prop) -> VarDictionary {
    vdict! {
        "kind" => prop.kind.key(),
        "x" => prop.x,
        "z" => prop.z,
        "yaw" => prop.yaw,
        "length" => prop.length,
        "depth" => prop.depth,
        "house" => prop.house.map_or(-1, |h| h as i64),
    }
}

/// EP6: a decor prop `{kind, x, z, yaw, length, depth, count}`.
fn decor_prop_dict(prop: &sim_battle::DecorProp) -> VarDictionary {
    vdict! {
        "kind" => prop.kind.key(),
        "x" => prop.x,
        "z" => prop.z,
        "yaw" => prop.yaw,
        "length" => prop.length,
        "depth" => prop.depth,
        "count" => prop.count as i64,
    }
}

/// EP6: a decor area `{kind, x, z, length, width, yaw, state}` (`state`:
/// ploughed|sown|crop|stubble for ploughland, else "").
fn decor_area_dict(area: &sim_battle::Area) -> VarDictionary {
    vdict! {
        "kind" => area.kind.key(),
        "x" => area.x,
        "z" => area.z,
        "length" => area.length,
        "width" => area.width,
        "yaw" => area.yaw,
        "state" => area.state.map_or("", |s| s.key()),
    }
}

/// EP6: the decor of the field (see [`BattleSim::get_terrain`]).
fn decor_dict(decor: &sim_battle::Decor) -> VarDictionary {
    let buildings: VarArray = decor
        .buildings
        .iter()
        .map(|h| {
            vdict! {
                "x" => h.x, "z" => h.z, "length" => h.length, "width" => h.width,
                "yaw" => h.yaw, "kind" => h.kind.key(),
            }
            .to_variant()
        })
        .collect();
    let hamlets: VarArray = decor
        .hamlets
        .iter()
        .map(|h| {
            let buildings: PackedInt32Array = h.buildings.iter().map(|&i| i as i32).collect();
            vdict! {
                "layout" => h.layout.key(), "x" => h.x, "z" => h.z, "yaw" => h.yaw,
                "buildings" => &buildings,
            }
            .to_variant()
        })
        .collect();
    let areas: VarArray = decor
        .areas
        .iter()
        .map(|a| decor_area_dict(a).to_variant())
        .collect();
    let props: VarArray = decor
        .props
        .iter()
        .map(|p| decor_prop_dict(p).to_variant())
        .collect();
    let mounds: VarArray = decor
        .mounds
        .iter()
        .map(|m| {
            vdict! { "x" => m.x, "z" => m.z, "radius" => m.radius, "height" => m.height }
                .to_variant()
        })
        .collect();
    let moats: VarArray = decor
        .moats
        .iter()
        .map(|m| {
            vdict! {
                "x" => m.x, "z" => m.z, "length" => m.length, "width" => m.width,
                "yaw" => m.yaw, "ring" => m.ring,
            }
            .to_variant()
        })
        .collect();
    let camps: VarArray = decor
        .camps
        .iter()
        .map(|c| {
            let items: VarArray = c
                .items
                .iter()
                .map(|p| decor_prop_dict(p).to_variant())
                .collect();
            let convoy: VarArray = c
                .convoy
                .iter()
                .map(|p| decor_prop_dict(p).to_variant())
                .collect();
            vdict! {
                "side" => c.side.key(), "area" => &decor_area_dict(&c.area),
                "items" => &items, "convoy" => &convoy,
            }
            .to_variant()
        })
        .collect();
    vdict! {
        "profile" => decor.profile.as_str(),
        "vines_leafy" => decor.vines_leafy,
        "orchard_blossom" => decor.orchard_blossom,
        "buildings" => &buildings,
        "hamlets" => &hamlets,
        "areas" => &areas,
        "props" => &props,
        "mounds" => &mounds,
        "moats" => &moats,
        "camps" => &camps,
    }
}

#[godot_api(secondary)]
impl BattleSim {
    /// `{width, depth, resolution, nx, nz, heights, forests[{x, z, radius}],
    /// mud[..], river?{points: PackedVector2Array, width, fords[{x, z, half_width}]},
    /// siege?{...}}` (siege geometry: see [`Self::get_siege`]). B5 (campaign site):
    /// `terrain` (province terrain key), `season`, `ground` (`dry|muddy|snowy`),
    /// `ground_label`, `site_label` (B6), `woodland` (0-1), `pools[{x, z, radius}]`,
    /// `obstacles[{a: Vector2, b: Vector2, kind: hedge|fence|ditch|palisade}]`
    /// (CV3-2: `palisade` = entrenched camp),
    /// `coast?{flank: west|east, shore_x, beach}`.
    /// R2: `forests` and `mud` are overlapping discs (anchors first, then
    /// lobes and copses).
    /// EP3: `river` also carries `widths` (water width at each point),
    /// `flow` (+1 when the water runs towards +x), `banks[{x0, x1, north,
    /// kind: steep|marsh}]`; `bridges[{x, z, yaw, length, width, span,
    /// deck, stone, arches, stream}]` (`stream` = -1 on the river),
    /// `streams[{kind: tributary|brook, points, width}]`, `roads[{kind:
    /// main|track, points, width}]`; oxbows are appended to `pools`.
    /// EP6: `decor{profile, vines_leafy, orchard_blossom, buildings[{x, z,
    /// length, width, yaw, kind}], hamlets[{layout, x, z, yaw, buildings}],
    /// areas[{kind, x, z, length, width, yaw, state}], props[{kind, x, z,
    /// yaw, length, depth, count}], mounds[{x, z, radius, height}],
    /// moats[{x, z, length, width, yaw, ring}], camps[{side, area, items,
    /// convoy}]}` (windmill mounds are already in `heights`).
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
            // EP1: battle lines of the field (250 / 550 on the standard one).
            "attacker_line_z" => field.attacker_line_z(),
            "defender_line_z" => field.defender_line_z(),
            "resolution" => field.resolution,
            "nx" => field.nx as i64,
            "nz" => field.nz as i64,
            "heights" => &heights,
            // R2: a wood (a patch of mud) is its anchor disc plus its lobes
            // and copses; the discs overlap.
            "forests" => &zones(&[field.forests.as_slice(), &field.forest_parts].concat()),
            "mud" => &zones(&[field.mud.as_slice(), &field.mud_parts].concat()),
            // B5: campaign site.
            "terrain" => field.terrain.key(),
            "season" => field.season.key(),
            "ground" => field.ground.key(),
            "ground_label" => field.ground.label_fr(),
            // B6: the site in one compact line (pre-battle dialog, HUD).
            "site_label" => field.site_label_fr(),
            "woodland" => field.woodland,
            "pools" => &zones(&[field.pools.as_slice(), &field.oxbows].concat()),
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
            let flank = coast.flank.key();
            dict.set(
                "coast",
                &vdict! { "flank" => flank, "shore_x" => coast.shore_x, "beach" => coast.beach },
            );
        }
        if let Some(river) = &field.river {
            let points: PackedVector2Array = river
                .polyline(10.0, field.width)
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
            let widths: PackedFloat32Array = river
                .polyline(10.0, field.width)
                .iter()
                .map(|(x, _)| river.width_at(*x) as f32)
                .collect();
            let banks: VarArray = river
                .banks
                .iter()
                .map(|b| {
                    vdict! { "x0" => b.x0, "x1" => b.x1, "north" => b.north, "kind" => b.kind.key() }
                        .to_variant()
                })
                .collect();
            // The water runs towards the lower end of the field.
            let flow = if field.height(0.0, river.center_z(0.0))
                >= field.height(field.width, river.center_z(field.width))
            {
                1
            } else {
                -1
            };
            dict.set(
                "river",
                &vdict! {
                    "points" => &points, "width" => river.width, "fords" => &fords,
                    "widths" => &widths, "banks" => &banks, "flow" => flow,
                },
            );
        }
        let polyline = |points: &[(f64, f64)]| -> PackedVector2Array {
            points
                .iter()
                .map(|(x, z)| Vector2::new(*x as f32, *z as f32))
                .collect()
        };
        let bridges: VarArray = field
            .bridges
            .iter()
            .map(|b| {
                vdict! {
                    "x" => b.x, "z" => b.z, "yaw" => b.yaw(), "length" => b.length,
                    "width" => b.width, "span" => b.span, "deck" => b.deck, "stone" => b.stone,
                    "arches" => b.arches as i64,
                    "stream" => b.stream.map_or(-1, |s| s as i64),
                }
                .to_variant()
            })
            .collect();
        dict.set("bridges", &bridges);
        let streams: VarArray = field
            .streams
            .iter()
            .map(|s| {
                vdict! { "kind" => s.kind.key(), "points" => &polyline(&s.points), "width" => s.width }
                    .to_variant()
            })
            .collect();
        dict.set("streams", &streams);
        let roads: VarArray = field
            .roads
            .iter()
            .map(|r| {
                vdict! { "kind" => r.kind.key(), "points" => &polyline(&r.points), "width" => r.width }
                    .to_variant()
            })
            .collect();
        dict.set("roads", &roads);
        dict.set("decor", &decor_dict(&field.decor));
        if sim.siege().is_some() {
            dict.set("siege", &self.get_siege());
        }
        dict
    }

    /// Siege battle walls (empty dictionary in a field battle):
    /// `{fortification, center: Vector2, square_radius, thickness, wall_height,
    /// gate, hold_time, hold_to_win, integrity, pieces[{index, kind: "wall"|"gate",
    /// a: Vector2, b: Vector2, hp, max_hp, intact, docked_tower, under_attack (SB)}],
    /// towers[{x, z, radius, height}], houses[{x, z, radius, suburb, fire: {state:
    /// "intact"|"burning"|"burnt", intensity}, length, depth, yaw, rows, church}],
    /// props[{kind, x, z, yaw, length, depth, house}] (BR3), engines[{unit, kind:
    /// "ram"|"tower", side, x, z, hp, max_hp}] (SB), points[{kind: "square"|"gate",
    /// x, z, radius, progress, hold_s, share, status: "held"|"contested"|"capturing"|
    /// "taken", attackers, defenders}] (T4, ADR 0108), gate_fire: {state, intensity},
    /// wind: Vector2 (direction × strength 0-1), houses_burning, houses_burnt,
    /// sortie, ram_period, oil_period}`. Pieces lose HP and houses burn during the battle (S2): call it
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
                    "kind" => piece.kind.key(),
                    "a" => v2(piece.a),
                    "b" => v2(piece.b),
                    "hp" => piece.hp,
                    "max_hp" => piece.max_hp,
                    "intact" => piece.intact(),
                    "docked_tower" => piece.docked_tower.map_or(-1, i64::from),
                    // SB (ADR 0107): battered, shot at or burning just now.
                    "under_attack" => piece.under_attack(),
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
                    // BR3: the block's rectangle (length along (cos, sin)
                    // of yaw, row 0 facing (-sin, cos)), rows, church.
                    "length" => h.length,
                    "depth" => h.depth,
                    "yaw" => h.yaw,
                    "rows" => i64::from(h.rows),
                    "church" => h.church,
                    // NT1: a castle keep (drawn as a great tower).
                    "keep" => h.keep,
                    // NT8: the keep's height (0 for other buildings).
                    "height" => h.height,
                    // NT11: a keep with a crenellated terrace roof.
                    "terrace" => h.terrace,
                }
                .to_variant()
            })
            .collect();
        let props: VarArray = works
            .props
            .iter()
            .map(|p| prop_dict(p).to_variant())
            .collect();
        // SB (ADR 0107): rams and siege towers with their strength, for the
        // health bars.
        let engines: VarArray = self
            .sim
            .as_ref()
            .map(|s| s.siege_engines())
            .unwrap_or_default()
            .iter()
            .map(|e| {
                vdict! {
                    "unit" => i64::from(e.unit),
                    "kind" => e.kind.key(),
                    "side" => e.side.key(),
                    "x" => e.x,
                    "z" => e.z,
                    "hp" => e.hp,
                    "max_hp" => e.max_hp,
                }
                .to_variant()
            })
            .collect();
        // T4 (ADR 0108): capture points (market square, gate) with their
        // progress, for the flags and the capture bars.
        let points: VarArray = works
            .capture_points()
            .iter()
            .map(|p| {
                vdict! {
                    "kind" => p.kind.key(),
                    "x" => p.x,
                    "z" => p.z,
                    "radius" => p.radius,
                    "progress" => p.progress,
                    "hold_s" => p.hold_s,
                    "share" => p.share(),
                    "status" => p.status.key(),
                    "attackers" => p.attackers,
                    "defenders" => p.defenders,
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
            "hold_to_win" => sim_battle::CaptureRules::bundled().square.hold_s,
            "points" => &points,
            "integrity" => works.integrity(),
            "pieces" => &pieces,
            "towers" => &towers,
            "houses" => &houses,
            "props" => &props,
            "engines" => &engines,
            "sortie" => works.sortie,
            "gate_fire" => &blaze(&works.gate_fire),
            "wind" => v2(works.wind),
            "houses_burning" => works.burning_houses() as i64,
            "houses_burnt" => works.burnt_houses() as i64,
            // SG1: rhythm of the ram and of the oil pots (seconds).
            "ram_period" => sim_battle::siege_fx::RAM_PERIOD,
            "oil_period" => sim_battle::siege_fx::OIL_PERIOD,
        }
    }

    /// L3 (ADR 0026): the besieged town drawn from a landmark plan, `{id,
    /// name, gate_name, gatehouses: [{name, at: Vector2}], streets:
    /// [PackedVector2Array], quay: [piece index], plan_scale}`; empty for the
    /// generic town or a field battle.
    #[func]
    fn get_siege_landmark(&self) -> VarDictionary {
        let Some(landmark) = self
            .sim
            .as_ref()
            .and_then(|s| s.siege())
            .and_then(|w| w.landmark.as_ref())
        else {
            return VarDictionary::new();
        };
        let v2 = |p: (f64, f64)| Vector2::new(p.0 as f32, p.1 as f32);
        let gatehouses: VarArray = landmark
            .gatehouses
            .iter()
            .map(|(name, x, z)| {
                vdict! { "name" => name.as_str(), "at" => v2((*x, *z)) }.to_variant()
            })
            .collect();
        let streets: VarArray = landmark
            .streets
            .iter()
            .map(|s| {
                s.iter()
                    .map(|&p| v2(p))
                    .collect::<PackedVector2Array>()
                    .to_variant()
            })
            .collect();
        let quay: VarArray = landmark
            .quay
            .iter()
            .map(|&i| (i as i64).to_variant())
            .collect();
        vdict! {
            "id" => landmark.id.as_str(),
            "name" => landmark.name.as_str(),
            "gate_name" => landmark.gate_name.as_str(),
            "gatehouses" => &gatehouses,
            "streets" => &streets,
            "quay" => &quay,
            "plan_scale" => landmark.plan_scale,
        }
    }

    /// Debug (tests, captures, S2): sets house `house` on fire, or the gate
    /// when `house` < 0; `false` when it already burns or is not a siege.
    #[func]
    fn debug_ignite(&mut self, house: i64) -> bool {
        self.touch_poses();
        if self.player.is_some() {
            return false;
        }
        let Some(sim) = &mut self.sim else {
            return false;
        };
        let target = usize::try_from(house).ok();
        note(
            &mut self.recorder,
            sim,
            sim_battle::ReplayAction::Ignite { house: target },
        );
        if house < 0 {
            sim.ignite_gate()
        } else {
            sim.ignite_house(house as usize)
        }
    }

    /// Debug (captures, SG1): sets the HP of wall piece `index` (clamped to
    /// its maximum; 0 opens it). `false` outside a siege or for a bad index.
    #[func]
    fn debug_set_piece_hp(&mut self, index: i64, hp: f64) -> bool {
        self.touch_poses();
        if self.player.is_some() {
            return false;
        }
        if let (Some(sim), Ok(piece)) = (&self.sim, usize::try_from(index)) {
            note(
                &mut self.recorder,
                sim,
                sim_battle::ReplayAction::PieceHp { piece, hp },
            );
        }
        let Some(works) = self.sim.as_mut().and_then(|s| s.siege_mut()) else {
            return false;
        };
        let Some(piece) = usize::try_from(index)
            .ok()
            .and_then(|i| works.pieces.get_mut(i))
        else {
            return false;
        };
        piece.hp = hp.clamp(0.0, piece.max_hp);
        true
    }

    /// Ground height at (x, z).
    #[func]
    fn get_height(&self, x: f64, z: f64) -> f64 {
        self.sim.as_ref().map_or(0.0, |s| s.field().height(x, z))
    }

    /// EP3: height one walks at (x, z): a bridge deck, else the ground.
    #[func]
    fn get_walk_height(&self, x: f64, z: f64) -> f64 {
        self.sim
            .as_ref()
            .map_or(0.0, |s| s.field().walk_height(x, z))
    }

    /// B6: the battle site in one compact French line, e.g. « Terre gelée ·
    /// hiver · haies · côte ouest » (empty without a battle).
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

    /// EP6: state of each side's camp, `[{side, progress (0-1), looted,
    /// alarmed, looters, guards}]` (empty without camps).
    #[func]
    fn get_camps(&self) -> VarArray {
        let Some(sim) = &self.sim else {
            return VarArray::new();
        };
        sim_battle::SideId::BOTH
            .iter()
            .filter_map(|&side| {
                let state = sim.camp_state(side)?;
                Some(
                    vdict! {
                        "side" => side.key(),
                        "progress" => state.progress,
                        "looted" => state.looted,
                        "alarmed" => state.alarmed,
                        "looters" => state.looters as i64,
                        "guards" => state.guards as i64,
                    }
                    .to_variant(),
                )
            })
            .collect()
    }
}
