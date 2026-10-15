//! Emblematic 1:1 cities in the town layer (ADR 0138/0356): streaming, the plan of
//! `landmark_plan`, houses (kit or blocks), real monuments (meshes of `landmark_monuments`),
//! wall rings with explicit normals, regular towers and gates, bridges (deck, piers, starlings,
//! access ramps, houses and chapel on the deck) standing on the water level. Port of the landmark
//! part of `landmark_city_layer.gd` and of `town_builder.gd` (wall rings, bridges).
//!
//! A landmark city is keyed by its settlement id (`set_paris`) in [`TownsState`]: markers read
//! [`TownsState::is_shown`] like for the ordinary towns, and the same fade (root scaled on Y)
//! applies.

use super::*;
use crate::campaign::towns::landmark_monuments::MeshData;
use crate::campaign::towns::landmark_plan::{self, Bridge, LmGate, WallRing};

/// What a built landmark city adds to [`Shown`].
pub(super) struct LmShown {
    pub plan: Arc<LandmarkPlan>,
    pub walls: Vec<Handle<Mesh>>,
}

/// Anchors of the landmark cities: their settlement in `settlements_px.json`.
pub(super) fn landmark_anchors(
    lib: &LandmarkLibrary,
    data_dir: &std::path::Path,
) -> HashMap<String, Vec2> {
    let px: HashMap<String, [f32; 2]> =
        std::fs::read_to_string(data_dir.join("map/settlements_px.json"))
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_default();
    lib.cities
        .keys()
        .filter_map(|id| px.get(id).map(|p| (id.clone(), Vec2::new(p[0], p[1]))))
        .collect()
}

fn push_quad(out: &mut Quads, corners: [Vec3; 4], normal: Vec3, uv: [[f32; 2]; 4]) {
    let base = out.positions.len() as u32;
    for (c, t) in corners.iter().zip(uv) {
        out.positions.push(c.to_array());
        out.normals.push(normal.to_array());
        out.uvs.push(t);
    }
    out.indices
        .extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
}

#[derive(Default)]
struct Quads {
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    uvs: Vec<[f32; 2]>,
    indices: Vec<u32>,
}

impl Quads {
    fn into_mesh(self) -> Mesh {
        let mut mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        );
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, self.positions);
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, self.normals);
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, self.uvs);
        mesh.insert_indices(Indices::U32(self.indices));
        mesh
    }
}

/// Wall of any outline: outer and inner faces, wall walk, outer parapet; the foot sinks 2.5 m
/// under the ground. `y_of`: displayed ground height (local metres) at a plan point.
pub(super) fn wall_ring_mesh(w: &WallRing, y_of: &dyn Fn(Vec2) -> f32) -> Mesh {
    let mut q = Quads::default();
    let half = w.thickness * 0.5;
    let mut along = 0.0;
    let v3 = |p: Vec2, y: f32| Vec3::new(p.x, y, p.y);
    for i in 1..w.ring.len() {
        let (a, b) = (w.ring[i - 1], w.ring[i]);
        let seg = a.distance(b);
        if w.gaps[i] || w.gaps[i - 1] {
            along += seg;
            continue;
        }
        let (na, nb) = (w.normals[i - 1] * half, w.normals[i] * half);
        let (ha, hb) = (y_of(a), y_of(b));
        let n_out = Vec3::new(w.normals[i].x, 0.0, w.normals[i].y);
        let (u0, u1) = (along / 8.0, (along + seg) / 8.0);
        let top = w.height;
        let wall =
            |q: &mut Quads, p0: Vec2, p1: Vec2, h0: f32, h1: f32, lo: f32, hi: f32, n: Vec3| {
                push_quad(
                    q,
                    [
                        v3(p0, h0 + lo),
                        v3(p1, h1 + lo),
                        v3(p1, h1 + hi),
                        v3(p0, h0 + hi),
                    ],
                    n,
                    [
                        [u0, lo / 8.0],
                        [u1, lo / 8.0],
                        [u1, hi / 8.0],
                        [u0, hi / 8.0],
                    ],
                );
            };
        wall(&mut q, a + na, b + nb, ha, hb, -2.5, top, n_out);
        wall(&mut q, b - nb, a - na, hb, ha, -2.5, top, -n_out);
        push_quad(
            &mut q,
            [
                v3(a + na, ha + top),
                v3(b + nb, hb + top),
                v3(b - nb, hb + top),
                v3(a - na, ha + top),
            ],
            Vec3::Y,
            [[u0, 0.0], [u1, 0.0], [u1, 0.3], [u0, 0.3]],
        );
        // Outer parapet: merlons simplified into a single rail.
        let (pa, pb) = (a + na * 0.6, b + nb * 0.6);
        wall(
            &mut q,
            pa + na * 0.4,
            pb + nb * 0.4,
            ha,
            hb,
            top,
            top + 1.6,
            n_out,
        );
        wall(&mut q, pb, pa, hb, ha, top, top + 1.6, -n_out);
        along += seg;
    }
    q.into_mesh()
}

/// Triangle soup of a monument (positions, normals, vertex colours).
pub(super) fn monument_mesh(m: &MeshData) -> Mesh {
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, m.positions.clone());
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, m.normals.clone());
    mesh.insert_attribute(
        Mesh::ATTRIBUTE_UV_0,
        vec![[0.0_f32, 0.0]; m.positions.len()],
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, m.colors.clone());
    mesh
}

// --- Streaming --------------------------------------------------------------------------------

/// Builds the nearest missing landmark city within `radius` of the focus (one per frame).
#[allow(clippy::too_many_arguments)]
pub(super) fn stream_landmarks(
    commands: &mut Commands,
    assets: &AssetServer,
    meshes: &mut Assets<Mesh>,
    shared: &TownMeshes,
    world: &TownsWorld,
    state: &mut TownsState,
    surface: &Surface,
    ground: &GroundHeight,
    sim: &Sim,
    focus: Vec2,
    radius: f32,
    lod: Lod,
) {
    let mut wanted: Vec<(&String, Vec2, f32)> = world
        .landmark_px
        .iter()
        .filter(|(id, _)| !state.shown.contains_key(*id))
        .map(|(id, p)| (id, *p, p.distance(focus)))
        .filter(|(_, _, d)| *d < radius)
        .collect();
    wanted.sort_by(|a, b| a.2.total_cmp(&b.2).then_with(|| a.0.cmp(b.0)));
    let Some((id, anchor, _)) = wanted.first().copied() else {
        return;
    };
    let Some(city) = world.landmarks.get(id) else {
        return;
    };
    let started = std::time::Instant::now();
    let mpu = world.db.meters_per_unit;
    let scale = 1.0 / mpu;
    let lp = match state.landmark_plans.get(id) {
        Some(p) => p.clone(),
        None => {
            let h = |p: Vec2| surface.height_m_at(anchor.x + p.x / mpu, anchor.y + p.y / mpu);
            let p = Arc::new(landmark_plan::generate(city, &h));
            state.landmark_plans.insert(id.clone(), p.clone());
            state.plans_built += 1;
            state.plan_ms_max = state
                .plan_ms_max
                .max(started.elapsed().as_secs_f32() * 1000.0);
            p
        }
    };
    let spawn_started = std::time::Instant::now();
    let y_of = |l: Vec2| local_height(ground, anchor, scale, l);
    let mut scenes = std::mem::take(&mut state.scenes);
    let seed = city["seed"].as_u64().unwrap_or(1) as u32;
    let (root, ribbons, walls, counts) = spawn_landmark(
        commands,
        assets,
        meshes,
        shared,
        world,
        &mut scenes,
        id,
        anchor,
        seed,
        &lp,
        lod,
        state.weight,
        &y_of,
        style_of_town(sim, world, id),
    );
    state.scenes = scenes;
    let trees = spawn_trees(
        commands,
        meshes,
        shared,
        root,
        &lp.plan,
        lod == Lod::Detail,
        &y_of,
    );
    state.towns_spawned += 1;
    state.spawn_ms_max = state
        .spawn_ms_max
        .max(spawn_started.elapsed().as_secs_f32() * 1000.0);
    state.shown.insert(
        id.clone(),
        Shown {
            root,
            lod,
            growth: Growth::default(),
            plan: Arc::new(lp.plan.clone()),
            ribbons,
            counts,
            anchor,
            landmark: Some(LmShown { plan: lp, walls }),
            trees,
        },
    );
}

// --- Spawning ---------------------------------------------------------------------------------

fn spawn_bridge(sp: &mut Spawner, br: &Bridge) {
    let dir = Vec2::new(br.yaw.cos(), br.yaw.sin());
    let (deck, water) = (br.deck, br.water);
    let mat = sp.masonry();
    sp.fixed = Some((br.pos, 0.0));
    // Deck, laid on the pier base (water level) and raised in metres.
    sp.block(
        false,
        mat.clone(),
        br.pos,
        br.yaw,
        Vec3::new(br.length, 1.6, br.width),
        deck - water - 1.5,
    );
    let (piers, spread) = if br.arches > 1 {
        (br.arches as usize - 1, 0.92)
    } else {
        (((br.length / 15.0) as usize).max(1), 0.8)
    };
    for k in 0..piers {
        let t = (k as f32 + 0.5) / piers as f32 - 0.5;
        let pc = br.pos + dir * (t * br.length * spread);
        sp.block(
            false,
            mat.clone(),
            pc,
            br.yaw,
            Vec3::new(br.pier_m, (deck - water + 1.0).max(2.0), br.width + 1.0),
            -1.5,
        );
        if br.starling_m > 0.0 {
            sp.block(
                false,
                mat.clone(),
                pc,
                br.yaw,
                Vec3::new(br.pier_m * 1.7, 2.6, br.starling_m),
                -1.5,
            );
        }
    }
    // Access ramps of the deck down to low banks.
    for end in [-1.0_f32, 1.0] {
        let bank = if end > 0.0 { br.bank_to } else { br.bank_from };
        let rise = deck - bank;
        if rise <= 0.8 {
            continue;
        }
        let run = (rise * 7.0).clamp(8.0, 45.0);
        let out_d = dir * end;
        let start = br.pos + out_d * (br.length * 0.5 - 1.0);
        let center = start + out_d * (run * 0.5);
        let out_yaw = out_d.y.atan2(out_d.x);
        let tilt = Quat::from_axis_angle(Vec3::new(-out_d.y, 0.0, out_d.x), -rise.atan2(run));
        let lift = (deck + bank) * 0.5 - 1.5 - water;
        let g = TownGround {
            anchor: sp.anchor,
            local: center,
            yaw: out_yaw,
            half: Vec2::ZERO,
            lift_m: lift,
            sink_m: 0.0,
            sample: Some(br.pos),
        };
        let y = footing_y(sp.y_of, &g);
        sp.commands.spawn((
            Mesh3d(sp.shared.cube.clone()),
            MeshMaterial3d(mat.clone()),
            Transform::from_translation(Vec3::new(center.x, y, center.y))
                .with_rotation(tilt * Quat::from_rotation_y(-out_yaw))
                .with_scale(Vec3::new((run * run + rise * rise).sqrt(), 1.6, br.width)),
            NotShadowCaster,
            g,
            ChildOf(sp.root),
        ));
        sp.counts.blocks += 1;
        sp.counts.entities += 1;
    }
    sp.fixed = None;
}

#[allow(clippy::too_many_arguments)]
fn spawn_landmark(
    commands: &mut Commands,
    assets: &AssetServer,
    meshes: &mut Assets<Mesh>,
    shared: &TownMeshes,
    world: &TownsWorld,
    scenes: &mut HashMap<String, Handle<WorldAsset>>,
    id: &str,
    anchor: Vec2,
    seed: u32,
    lp: &LandmarkPlan,
    lod: Lod,
    weight: f32,
    y_of: &dyn Fn(Vec2) -> f32,
    style: Option<RegionStyle>,
) -> (Entity, [Handle<Mesh>; 2], Vec<Handle<Mesh>>, Counts) {
    let scale = 1.0 / world.db.meters_per_unit;
    let look = &world.look;
    let detail = lod == Lod::Detail;
    let plan = &lp.plan;
    let root = commands
        .spawn((
            Name::new(format!("Landmark_{id}")),
            TownRoot { id: id.to_string() },
            Transform::from_translation(Vec3::new(anchor.x, 0.0, anchor.y))
                .with_scale(root_scale(scale, weight)),
            Visibility::Inherited,
            DespawnOnExit(AppState::Campaign),
        ))
        .id();
    let street = meshes.add(street_mesh(plan, y_of, look.ribbon_lift_world / scale));
    let ground_handle = meshes.add(ground_mesh(plan, y_of, look.ground_lift_world / scale));
    commands.spawn((
        Name::new("Streets"),
        Mesh3d(street.clone()),
        MeshMaterial3d(shared.street.clone()),
        NotShadowCaster,
        Transform::IDENTITY,
        ChildOf(root),
    ));
    commands.spawn((
        Name::new("Ground"),
        Mesh3d(ground_handle.clone()),
        MeshMaterial3d(shared.ground.clone()),
        NotShadowCaster,
        Transform::IDENTITY,
        ChildOf(root),
    ));
    let mut wall_handles = Vec::new();
    for (k, ring) in lp.wall_rings.iter().enumerate() {
        let handle = meshes.add(wall_ring_mesh(ring, y_of));
        commands.spawn((
            Name::new(format!("WallRing_{k}")),
            Mesh3d(handle.clone()),
            MeshMaterial3d(shared.masonry.clone()),
            Transform::IDENTITY,
            ChildOf(root),
        ));
        wall_handles.push(handle);
    }
    let mut sp = Spawner {
        commands,
        assets,
        shared,
        world,
        scenes,
        root,
        anchor,
        y_of,
        style,
        fixed: None,
        counts: Counts {
            entities: 3 + wall_handles.len(),
            wall_pieces: wall_handles.len(),
            ..Default::default()
        },
    };
    // Houses (the ones on a bridge deck stand on the water level).
    for (i, h) in plan.houses.iter().enumerate() {
        let mut rng = Rng::new(seed ^ (i as u32).wrapping_mul(0x9E37_79B1));
        let bridge = lp.fixed_bases.get(&i).and_then(|b| lp.bridges.get(*b));
        sp.fixed = bridge.map(|b| (b.pos, h.base - b.water));
        spawn_building(
            &mut sp,
            detail,
            HOUSE_KINDS[h.kind],
            h.pos,
            h.yaw,
            h.front,
            h.depth,
            h.tint,
            &mut rng,
            false,
        );
        sp.counts.houses += 1;
    }
    sp.fixed = None;
    // Monuments: prepared meshes (monument frame), foot on the lowest point of the footprint;
    // the chapel of a bridge stands on the water level.
    for m in &lp.monuments {
        let bridge = m.bridge.and_then(|b| lp.bridges.get(b));
        let g = TownGround {
            anchor,
            local: m.pos,
            yaw: m.yaw,
            half: if bridge.is_some() {
                Vec2::ZERO
            } else {
                Vec2::new(m.length, m.depth) * 0.5
            },
            lift_m: bridge.map_or(0.0, |b| m.base - b.water),
            sink_m: if bridge.is_some() {
                0.0
            } else {
                look.foundation_sink_m
            },
            sample: bridge.map(|b| b.pos),
        };
        let y = footing_y(y_of, &g);
        sp.commands.spawn((
            Name::new(format!("Monument_{}", m.id)),
            Mesh3d(meshes.add(monument_mesh(&m.built.mesh))),
            MeshMaterial3d(shared.vertex.clone()),
            Transform::from_translation(Vec3::new(m.pos.x, y, m.pos.y))
                .with_rotation(Quat::from_rotation_y(-m.yaw)),
            g,
            ChildOf(root),
        ));
        sp.counts.monuments += 1;
        sp.counts.entities += 1;
    }
    // Regular towers and gates of the wall rings.
    let mat = sp.masonry();
    for t in &plan.towers {
        sp.block(
            true,
            mat.clone(),
            t.pos,
            0.0,
            Vec3::new(t.radius, t.height + 2.0, t.radius),
            -2.0,
        );
        sp.counts.towers += 1;
    }
    for g in &lp.gates {
        spawn_gate(&mut sp, g, lp, mat.clone());
        sp.counts.gates += 1;
    }
    for br in &lp.bridges {
        spawn_bridge(&mut sp, br);
    }
    let counts = sp.counts;
    (root, [street, ground_handle], wall_handles, counts)
}

fn spawn_gate(sp: &mut Spawner, g: &LmGate, lp: &LandmarkPlan, mat: Handle<StandardMaterial>) {
    match g.bridge.and_then(|b| lp.bridges.get(b)) {
        Some(b) => {
            sp.fixed = Some((b.pos, 0.0));
            sp.block(
                false,
                mat,
                g.pos,
                g.yaw,
                Vec3::new(sp.world.db.gate_depth_m, g.height, 11.0),
                b.deck - 0.5 - b.water,
            );
            sp.fixed = None;
        }
        None => sp.block(
            false,
            mat,
            g.pos,
            g.yaw,
            Vec3::new(sp.world.db.gate_depth_m, g.height + 2.0, 11.0),
            -2.0,
        ),
    }
}
