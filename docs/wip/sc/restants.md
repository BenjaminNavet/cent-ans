# SC restes (état au 2026-10-09, main a3d23a2ff) - vérifs par grep/ls/wc, pas de lecture complète
Format : ID | état | reste | fichiers principaux | Rust? | [MÉCANIQUE]
(GELÉ FL = fichier récemment sale par d'autres sessions : settlement_layer, town_maquette_data, zoom_tiers, battle_vegetation, landmarks_v2, tools/experiments)

## DC docs
DC-3 | À FAIRE | docs/architecture.md absent, status.md non tronqué, lien README/CLAUDE.md | docs/status.md, README.md, CLAUDE.md | non
DC-4 | PARTIEL | docs/audit garde a1..a6 + recettes q1/q3/q5, images research non traitées | docs/audit, docs/research, docs/img | non
DC-5 | PARTIEL | docs/superpowers/plans (5) pas archivé, designs m1-m10 non marqués historique | docs/superpowers, docs/design | non
(DC-1 : wip 508 -> ~170 fichiers, archive chantiers.md 2236 l, jugé FAIT ; DC-2 INDEX.md présent)

## GT tests Godot
GT1 | PARTIEL | ~14 *_shot/probe/bench encore là (as1_shot, as5_shot, as8c_shader_probe, dn_*_shot, me1/me2/me5_shot, fl_weather_field_shot, tree_flicker_probe, codex_load_bench, mapload_bench) | game/tests/*_shot.gd | non
GT2 | PARTIEL | q3_playtest.gd reste (pb1_turns à garder) | game/tests/q3_playtest.gd | non
GT3 | PARTIEL | 151/197 *_test.gd sur TestCase, 46 non migrés | game/tests/*_test.gd, game/tests/lib/test_case.gd | non
GT4 | PARTIEL | tools/run_godot_tests.sh existe mais CI windows.yml ne lance que smoke.gd | .github/workflows/windows.yml, tools/run_godot_tests.sh | non
GT5 | À FAIRE | suppr tests rouges/orphelins (inventaire à faire via run_all) | game/tests | non
GT6 | À FAIRE | fusion des tests par thème | game/tests | non
GT7 | FAIT (sc/smoke, 5 smoke_*.gd) | smoke.gd 2893 l non découpé | game/tests/smoke.gd | non
GT8 | PARTIEL | .uid suivis (fait) ; hooks de capture orphelins : 20 occurrences _stage_/stage_screenshot/--stage restent | game/scripts/battle/battle_scene.gd, ui/*, map/* | non
HOOKS | PARTIEL | retirer stage_screenshot()/stage_example restants + smoke.gd:1150 | idem | non

## RT tests Rust
RT2 | PARTIEL | 10 fichiers avec #[ignore] restent (fk_map_scenes, m3_grid_ai, ep13_replay, perf_step, cb6_group_formation, sb_siege_pace, br3, relief, ep1_scale, ai_relief) | core/crates/*/tests | oui
RT6 | À FAIRE | review_tests/rs_c_tests/rs_n_tests toujours dans src (découpe) | core/crates/sim-campaign/src/review_tests.rs | oui
RT7 | À FAIRE | raccourcir tests lents (mesurer d'abord) | core/crates/*/tests | oui
RT8 | À FAIRE | fusion petits tests/tautologies (data-model 1 binaire, ok) | core/crates/*/tests | oui
RT9 | À FAIRE | core/checks garde data_store_check et png_decode_check (garder campaign_sim_check seul) | core/checks/ | oui

## GB godot-bridge (304 #[func] contre 337)
GB6 | À FAIRE | TerrainMesher.build_patch, scatter() transforms, phases d'anim soldier_buffers (perf rendu) | core/crates/godot-bridge/src, game/scripts/battle | oui
GB7 | PARTIEL | key_enum! fait ; doc-comments restants (en dernier) | core/crates/godot-bridge/src | oui

## CB sim-campaign
CB1 | À FAIRE | settle_side_outcomes/losses_percent/retreat_losers introuvables (battle_flow absent) ; hash playthrough | core/crates/sim-campaign/src/battle_outcome.rs, battle_request.rs | oui
CB7 | PARTIEL | GameData::province_name/faction_name existent ; restes bridge (campaign_sim.rs, treaty, feudal), custom.rs faction_name, From<Season>/dist | core/crates/godot-bridge/src, sim-battle/src/custom.rs | oui
CB8 | PARTIEL | hostile_settlement_cells toujours fonction non cachée, passe-plats/depart() à vérifier | core/crates/sim-campaign/src/march.rs, movement/ | oui
CB10 | À FAIRE | debug_* (encounter.rs, battle_request.rs) pas dans staging.rs ; sg3_assault_probe.rs reste | core/crates/sim-campaign/src/battle_request.rs, tests/siege_battle/sg3_assault_probe.rs | oui
CB12 | PARTIEL | constantes naval en data faites ; mémo win_chance non vérifié | core/crates/sim-battle/src/naval | oui
CB13 | PARTIEL | constantes siège/débarquement en data (movement) ; pub->pub(crate), doc Order non vérifiés | core/crates/sim-campaign/src | oui
PROBES | FAIT (sc/cc) | examples diplomacy_probe, dynasty_probe, income_probe (sim-campaign) ; tests century/settlements/ia_quality probes à revérifier | core/crates/sim-campaign/examples | oui

## BB sim-battle
BB1 | FAIT (sc/bb1, ADR 0205) | village B5 : battle_village.gd 612 l + props/obstacles/ai/field/setup/bridge | game/scripts/battle/battle_village.gd, core/crates/sim-battle/src/{town,props,site}.rs | oui | [MÉCANIQUE]
BB14 | FAIT (sc/bb14) | générateurs borough/castle de siege_layouts.rs (912 l) à supprimer | core/crates/sim-battle/src/siege_layouts.rs | oui | [MÉCANIQUE]
BB5 | FAIT (sc/bb5) | WallPiece::new, SiegeWorks::skeleton, from_layout découpé (après BB14) | core/crates/sim-battle/src/siege_layout.rs, siege.rs | oui
BB6 | PARTIEL | battle_terrain.json fait ; battle_relief/site.json non | core/crates/sim-battle/src/{relief,site}.rs, data/rules | oui
BB8 | PARTIEL | place_plot/place_manor encore pub, decor_effect_at unique à vérifier | core/crates/sim-battle/src/decor_gen | oui
BB9 | À FAIRE | cache crossings/bridge_at (piste water_kind par SegmentGrid) | core/crates/sim-battle/src/{hydro,sim/movement}.rs | oui
BB10 | PARTIEL | rng::hash01 commun fait ; siege_fx::hash01 garde son flux ; vegetation/lib.rs a sa copie | core/crates/sim-battle/src/siege_fx.rs | oui
BB12 | À FAIRE | pub->pub(crate) (dernier) | core/crates/sim-battle/src | oui

## PF perf transverse
PF-02 | FAIT (sc/relief, ADR 0203) | suppr relief_quadtree.gd (1295 l) sélection GD + fine_terrain_job.gd (167) + repli sans pyramide terrain_builder (1493) | game/scripts/map/{relief_quadtree,fine_terrain_job,terrain_builder}.gd | non (Rust optionnel) | [MÉCANIQUE visuelle]
PF-03 | FAIT (ADR 0204, sc/veg) | suppr vegetation_tile_job.gd (649 l) + repli GD vegetation.gd [GELÉ vegetation.gd] | game/scripts/map/vegetation_tile_job.gd, vegetation.gd | non
PF-06 | PARTIEL | CmdArgs partout ; 46 interrupteurs --no-* encore présents ; sc/devflags prêt (1f7987144) mais NON fusionné (attend settlement_layer propre) | game/scripts/**, branche sc/devflags (../gp-sc-devflags) | non
PF-07 | FAIT (sc/bench) | map_bench.gd 534, release_journey.gd 525, perf_probe.gd, _bench_* battle_scene (campaign_map ex-gelé FL) | game/scripts/dev/*.gd | non
PF-04 | PARTIEL | chargeur Rust vectoriel fait (DT4, ~500->200 ms) ; bake .bin geojson non | tools/cent_ans_tools/geo, core/crates/godot-bridge/src/map_geo.rs | oui
PF-01 | À FAIRE | OutbuildingPacker Rust [GELÉ FL] | game/scripts/map/outbuilding_layer.gd | oui
PF-10 | À FAIRE | declutter Rust (marker_declutter.gd) [GELÉ FL] | game/scripts/map/marker_declutter.gd | oui
PF-08 | À FAIRE | StampMap stamp_soft_disc + relief_from_heights pour battle_terrain | core/crates/godot-bridge/src/stamp_map.rs, game/scripts/battle/battle_terrain.gd | oui
PF-09 | À FAIRE | parchment_decor précalc | game/scripts/map/parchment_decor.gd | non
PF-11 | À FAIRE | heights_m batch | game/scripts/map | oui
PF-12 | À FAIRE | TileJobPool commun (15 fichiers) | game/scripts/map/*_job.gd | non
PF-13 | PARTIEL | DataFile/JsonLookup largement posés ; 92 lectures JSON brutes restent (136 au départ) | game/scripts/** | non
PF-14 | FAIT (= DT2) | PNG replis relief (relief_shade_[0-3].png 131 Mo = DT2) | game/scripts/map/relief_landcover.gd, tools/cent_ans_tools/export_data.py | non

## BA sim-battle moteur
BA1 | PARTIEL | sim-battle/ai propres ; seuls restent les examples sim-campaign (voir PROBES) | core/crates/sim-campaign/examples | oui
BA8 | À FAIRE | replay digest simplifié, builder ReplayStart, suppr soldier_positions alias (unit.rs:961) | core/crates/sim-battle/src/replay.rs, unit.rs | oui
BA9 | À FAIRE | cache figure_positions layout local | core/crates/sim-battle/src/unit.rs | oui
BA10 | PARTIEL | sim/commands.rs existe ; apply_command découpé à vérifier | core/crates/sim-battle/src/sim/commands.rs | oui
BA11 | PARTIEL | missile_arc.json existe ; table missiles + fire() découpé (sim/fire.rs 625 l) | core/crates/sim-battle/src/sim/fire.rs | oui
BA12 | À FAIRE | géométrie grille formations unifiée (group_formation.rs 825 l) | core/crates/sim-battle/src/group_formation.rs, formations.rs | oui
BA13 | À FAIRE | MovementRules data (sim/movement.rs 624 l) | core/crates/sim-battle/src/sim/movement.rs | oui
BA14 | À FAIRE | pub(crate) + docs lib.rs | core/crates/sim-battle/src/lib.rs | oui

## MC map
MC1 | PARTIEL | relief_decoder.rs Rust existe ; png16.gd (GD) et chemins 8bit toujours là | game/scripts/map/png16.gd | oui
MC3 | PARTIEL | = PF-06 (sc/devflags) ; 5 get_cmdline restants | game/scripts/util/cmd_args.gd | non
MC6 | À FAIRE | folk_scenes 680 l, 10 archétypes -> 4 | game/scripts/map/life_folk/folk_scenes.gd | non | [MÉCANIQUE visuelle]
MC8 | PARTIEL | army_markers 656 l non découpé | game/scripts/map/army_markers.gd | non
MC10 | FAIT (sc/relief, ADR 0203) | = PF-02 | - | non
MC11 | À FAIRE | ReliefState hors MapData | game/scripts/map/map_data.gd | non
MC13 | PARTIEL | MapInstancing fait ; fusion marqueurs/feedback, AI replay 1 mode | game/scripts/map | non
MC15 | À FAIRE | commentaires Lot/ADR (293) + doc-comments (EN DERNIER) | game/scripts/** | non

## CA sim-campaign diplo/agents
CA2 | PARTIEL | effects.rs créé (EFFECTS) mais chronicle.rs reste 1361 l, apply/describe_effect non unifiés | core/crates/sim-campaign/src/chronicle.rs | oui
CA5 | PARTIEL | names.rs absent ; helpers sur GameData ; copies is_rebels/faction_label/splitmix restantes (bridge) | core/crates/data-model/src/{load,util}.rs, godot-bridge | oui
CA7 | PARTIEL | constantes diplo/nego en data (treaty) ; dynasty/feudal à vérifier | core/crates/sim-campaign/src/{dynasty,feudal}.rs | oui
CA9 | À FAIRE | dynasty marriage_candidates dupli, crusade tests inline, feudal/escalation.rs | core/crates/sim-campaign/src/{dynasty,crusade}.rs | oui
CA10 | À FAIRE | doc Lot tags (dernier) | core/crates/sim-campaign/src | oui

## CC sim-campaign reste
CC3 | PARTIEL | = CA5 | - | oui
CC5 | PARTIEL | TurnBudget fait ; economy legacy (province_income, alias) à vérifier | core/crates/sim-campaign/src/economy.rs | oui
CC7 | ÉCARTÉ (déjà une table, sc/cc) | rule_constants table | core/crates/sim-campaign/src/rule_constants.rs | oui
CC8 | À FAIRE | table.rs+edicts.rs ProvincePolicy, medicine->population | core/crates/sim-campaign/src/{table,edicts,medicine}.rs | oui
CC9 | À FAIRE | ajustement garnisons JR4b -> data pré-calculée | core/crates/sim-campaign/src/setup_1337.rs | oui | [MÉCANIQUE]
CC11 | PARTIEL | ai/examples nettoyés ; sg3_assault_probe + jr4b_budget_probe (voir PROBES) | core/crates/sim-campaign/tests | oui
CC12 | FAIT (12 fns, sc/cc) | code mort 15 fns | core/crates/sim-campaign/src | oui
CC14 | PARTIEL | ai découpé (AITURN) ; fonctions géantes sim-campaign (diplomacy 2276, agents 2019, crusade 1850) | core/crates/sim-campaign/src | oui
CC15 | À FAIRE | doc tags (620) + lib.rs sous-dossiers (dernier) | core/crates/sim-campaign/src | oui

## BT battle (3D)
BT2 | PARTIEL | stages/hooks retirés ; bench + flags A/B restent dans battle_scene (2439 l) | game/scripts/battle/battle_scene.gd | non
BT3 | À FAIRE | figurine rigide : battle_meshes.gd 1093 l, battle_soldier.gdshader (garder engins), 27 glb | game/scripts/battle/battle_meshes.gd, game/shaders/battle_soldier.gdshader | non
BT4 | PARTIEL | = PF-06 (sc/devflags, 34 flags A/B + 64 has_method) | game/scripts/battle | non
BT5 | À FAIRE | buffers fine_near/hide/loosen en Rust (perf) | game/scripts/battle/battle_soldiers.gd, core/crates/godot-bridge | oui
BT6 | À FAIRE | manifeste skinné cuit hors ligne, suppr NT12/NT13 mocap trials | game/scripts/battle/battle_skinned.gd, tools | non
BT7 | À FAIRE | height/river battle_terrain en Rust, split Terrain/Mesh/Decor (2060 l) | game/scripts/battle/battle_terrain.gd | oui
BT8 | À FAIRE | plan_deployment en Rust | game/scripts/battle/deployment_controller.gd | oui
BT9 | À FAIRE | constantes -> data, KINDS dupliqué (soldiers+scene) | game/scripts/battle | non
BT10 | À FAIRE | MultiMeshKit/ParticleKit | game/scripts/battle | non
BT11 | FAIT 5a8c696d2, ADR 0230 (queue_tip gardé) | suppr duels/birds/cloud_shadows/queue_tip/secondary_motion (fichiers présents) -800 | game/scripts/battle/battle_{duels,birds,cloud_shadows,queue_tip,secondary_motion}.gd | non | [MÉCANIQUE cosmétique]
BT12 | À FAIRE | battle_scene structure replay/banners/audio, perf _refresh_view | game/scripts/battle/battle_scene.gd | non

## SH shaders
SH1 | PARTIEL | fx_noise fait pour fx ; ~24 copies hash/vnoise/fbm restent, noise_common absent (terrain GELÉ) | game/shaders/*.gdshader | non | [visuel léger]
SH2 | À FAIRE | campaign_map_data.gdshaderinc + river_common (terrain GELÉ) | game/shaders | non
SH5 | PARTIEL | = BT4/PF-06 (uniform bool da6/ga*/sr2 restent dans battle_ground, battle_soldier_skinned) | game/shaders | non
SH6 | À FAIRE | sea_nearby cache terrain (GELÉ) | game/shaders/terrain.gdshader | non
SH7 | À FAIRE | uniformes/fonctions morts | game/shaders | non
SH8 | À FAIRE | battle_ground detail_height 9 lectures -> dFdx (change le rendu, écarté par battledev) | game/shaders/battle_ground.gdshader | non | [visuel]
SH9 | PARTIEL | flipbook fx fait ; dupliqué dans fire_*/life_* (carte) | game/shaders | non
SH10 | À FAIRE | uniformes jamais posés -> const | game/shaders | non

## AD ai/data-model/relief-lod/vegetation
AD7 | FAIT (ADR 0204, sc/veg) | vegetation V4 legacy (species None) + repli GD vegetation_tile_job | core/crates/vegetation/src/lib.rs, game/scripts/map/vegetation_tile_job.gd | oui
AD8 | FAIT (sc/relief, ADR 0203) | relief_quadtree.gd repli GD (= PF-02), has_page mort | core/crates/relief-lod, game/scripts/map/relief_quadtree.gd | oui
AD9 | PARTIEL | data-model/util.rs créé ; segment_distance et hash01 dupliqués dans vegetation/lib.rs, siege_fx.rs, splitmix bridge | core/crates/data-model/src/util.rs, vegetation/src/lib.rs | oui
AD10 | À FAIRE | Cargo.toml profils redondants (profile.dev.package.* x5) | core/Cargo.toml | oui
AD11 | PARTIEL | fallback_edges/fallback_city_id toujours là | core/crates/data-model/src/{movement_graph,settlement_load}.rs | oui
AD12 | À FAIRE | perf ai grid near spatial, threat_by_province, Arc<str> | core/crates/ai/src/grid.rs | oui
AD13 | PARTIEL | ai/campaign découpé ; doc restante (plan_field.rs 862 l) | core/crates/ai | oui

## MB map terrain/nature
MB1 | FAIT (sc/relief, ADR 0203) | = PF-02 (pyramide obligatoire, suppr tuilé, png16, 672 tuiles far) | game/scripts/map/terrain_builder.gd | non
MB2 | FAIT (sc/relief, ADR 0203) | relief_quadtree sélection GD (= AD8b) | game/scripts/map/relief_quadtree.gd | non
MB3 | FAIT (ADR 0204, sc/veg ; vegetation_tile_job.gd gardé réduit à la requête) | = PF-03 | game/scripts/map/vegetation_tile_job.gd | non
MB4 | À FAIRE | StreamedTileLayer commun (5 couches) [vegetation GELÉ] | game/scripts/map | non
MB5 | PARTIEL | flags A/B végétation GA3/FC (--no-ga3-veg x7, --no-fc2/5) | game/scripts/map | non
MB6 | FAIT (sc/dt2) | relief_cache_notice (117) + relief_cache_status (241) -> push_warning | game/scripts/map/relief_cache_*.gd | non
MB7 | À FAIRE | parchemin décor marin animé suppr, redraw à la demande | game/scripts/map/parchment_decor.gd, shaders/parchment_sea.gdshaderinc | non | [MÉCANIQUE visuelle]
MB9 | À FAIRE | life_effects points typés (life reground GELÉ) | game/scripts/map/life_effects.gd | non
MB10 | FAIT | war_scars voie events suppr | game/scripts/map/war_scars.gd | non
MB11 | FAIT | vegetation_mask repli sans splat | game/scripts/map/vegetation_mask.gd | non
MB13 | FAIT | morts | game/scripts/map | non
MB14 | À FAIRE | étude fusion rendus rivière/route | game/scripts/map/{rivers,road}_renderer.gd | non

## UI
UI4 | À FAIRE | encyclopedia MECHANICS (const L57) -> data/ui/encyclopedia.json + fiche commune | game/scripts/ui/encyclopedia.gd | non
UI9 | PARTIEL | morts UI (thousands 2 occurrences, spacer) | game/scripts/ui | non
UI10 | À FAIRE | doc (dernier) | game/scripts/ui | non
UI11 | À FAIRE | tscn vs code (décision, 25 tscn) | game/ | non
UI12 | À FAIRE | menu_backdrop_3d.gd, living_portrait.gd suppr ? (liés MM1 menu récent, prudence) | game/scripts/ui | non | [MÉCANIQUE visuelle]

## MA map villes/UI
MA1 | À FAIRE | style `real` : town_builder 1107, landmark_city_layer 416, landmark_plan/model/monuments, town_far_* [landmarks_v2/towns sales] | game/scripts/map/town_*.gd, landmark_*.gd | non
MA2 | À FAIRE | landmarks v2 + tools paris_v2_author [réserve] | data/landmarks_v2, game/scripts/map/landmark_v2_library.gd | non
MA4 | À FAIRE | settlement_layer découpe labels/hamlets/picking [GELÉ] | game/scripts/map/settlement_layer.gd | non
MA5 | = PF-10 GELÉ | = PF-10 | - | oui
MA6 | FAIT ef8dc7976 | map_ui (1851 l) JournalView/TopBarFit + JOURNAL_STYLES | game/scripts/map/map_ui.gd | non
MA7 | FAIT (= PF-07) | = PF-07 (PerfProbe + map_bench) | game/scripts/dev | non
MA8 | PARTIEL | PF-05 mock fait ; 246 has_method restent (campaign_map, settlement_*) | game/scripts/map | non
MA9 | À FAIRE | règles visuelles villes + governable/commandable -> core | game/scripts/map/settlement_*.gd, core | oui
MA11 | À FAIRE | outbuilding_layer découpe (1718 l) [GELÉ] | game/scripts/map/outbuilding_layer.gd | non
MA12 | PARTIEL | stage_screenshot/morts du settlement_controller | game/scripts/map/settlement_controller.gd | non

## TL tools
TL1 | PARTIEL | tldel fait ; proto_moteur (5 fichiers) reste (experiments hors périmètre) | tools/proto_moteur | non
TL2 | PARTIEL | tldel a supprimé des one-shot ; modules restants à revérifier | tools/cent_ans_tools | non
TL3 | À FAIRE | map_markers + *_raw (da5_raw, da5b_raw, horizon_raw) -26 Mo | tools/da5_raw, da5b_raw, horizon_raw | non
TL4 | PARTIEL | outils GA3 gardés (session i3d/dn-fig-bake) | tools/blender_scripts | non
TL5 | À FAIRE | mocap/FA3/AN1b/FG blender -4.8k (tools/video_mocap, blender) | tools/blender, video_mocap | non
TL6 | À FAIRE | pipeline payant OpenRouter/fal/TTS -8k | tools/cent_ans_tools/{openrouter,voice_tts,portraits,material_gen,local_art}.py | non
TL7 | À FAIRE | audio/UI art gen -5.9k [ADR] | tools/cent_ans_tools/{ui_ornaments,audio_bank,ui_sounds,era_music}.py | non
TL10 | FAIT 3aab598d7 | cli.py 2051 l à découper | tools/cent_ans_tools/cli.py | non
TL12 | À FAIRE | battle_skinned vs fine doublons | tools/cent_ans_tools | non
TL13 | À FAIRE | kit_geometry | tools/cent_ans_tools | non
TL14 | À FAIRE | paths.py/imaging.py (dernier) | tools/cent_ans_tools | non
TLR | PARTIEL | descriptions schémas / commentaire siege_engines_fx.gd:89 citant outils supprimés | data/schemas, game/scripts/battle/siege_engines_fx.gd | non

## MS scripts divers
MS2 | FAIT (= PF-07) | = PF-07 (MapBench/PerfProbe/--bench-map/gen_synthetic_map) | game/scripts/dev, game/tools/gen_synthetic_map.gd | non
MS3 | PARTIEL | DataFile/JsonLookup posés ; battle/ui restants (voir PF-13) | game/scripts | non
MS4 | À FAIRE | codex_bubbles (1215 l) BubbleLayout + set_process conditionnel | game/scripts/codex/codex_bubbles.gd | non
MS5 | PARTIEL | VoicePool créé ; play_sfx round-robin hors pool, volumes/EVENT_SFX JSON à vérifier | game/scripts/audio | non
MS6 | FAIT? render_quality.json existe -> voir FAIT
MS7 | À FAIRE | assets sans référence (textures/buildings brutes 26 Mo ; quaternius 95 fichiers suivis) | game/assets, tools | non
MS8 | À FAIRE | LUT étalonnage atmosphere_library.grade_lut port Rust/précuisson | game/scripts/visual/atmosphere_library.gd | oui
MS9 | FAIT (release_journey suppr) | release_journey --map-ab/--ab-configs/--uncapped | game/scripts/dev/release_journey.gd | non
MS10 | À FAIRE | passe commentaires visual/ audio/, constantes battle_audio -> sound_bank.json | game/scripts/{visual,audio} | non

## DT / divers
DT2 | FAIT (sc/dt2, PNG dé-suivis gardés pour geo) | relief_shade_[0-3].png (131 Mo) toujours suivis ; supprimer repli + dé-suivre | data/map/relief_shade_*.png, game/scripts/map/relief_landcover.gd | non
DT3 | REPORTÉ | fusion landmarks v1->v2 (= MA1/MA2 sales) | - | non
DT5 | FAIT 7d3292c3b | defs communes color_hex/rgb3/snake_id dans common.schema.json | data/schemas | non
DT8 | PARTIEL | schémas town_footprint, forced_sea_edges, 4 orphelins à vérifier | data/schemas | non
SCH | PARTIEL | ~10 tests tools avec Draft202012Validator nu, 12 schémas sans registre | tools/tests, tools/cent_ans_tools/geo | non
PRE | À FAIRE | test_relief_update bake tier3 5 vs 6 (autre session) | tools/tests/test_relief_update.py | non
SIMSPLIT | PARTIEL | ai/plan_field.rs 862 l | core/crates/sim-battle/src/ai/plan_field.rs | oui
NAVAL-reste | PARTIEL | constantes ship.rs en dur, pending.remove(0) | core/crates/sim-battle/src/naval/ship.rs, sim-campaign/src/naval.rs | oui
BATTLEDEV | PARTIEL | RenderQuality.override_level/upscale_override, da6/site_render/fa_on (battle_vegetation GELÉ) | game/scripts/battle/battle_vegetation.gd | non
UIKIT | PARTIEL | RichTooltip.thousands (encyclopedia), délégués make_panel/attach_plain | game/scripts/ui/rich_tooltip.gd | non
BUGS-ouverts | À FAIRE | 2 tests Godot préexistants (cb0 golden ?), LOOKUPS2 balayage call() has_method morts (close_all_dialogs, open_faction_select) | game/scripts | non

## FAITS
DC-1 DC-2 | GT3(partiel) | RT1 RT3 RT4 RT5 | GB1 GB2 GB3 GB4 GB5 | CB4 CB5 CB6 CB9 CB11 | BB2 BB3 BB4 BB7 BB11 BB13 | PF-05 | BA2 BA3 BA4 BA5 BA6 BA7 | MC2 MC4 MC5 MC7 MC9 MC12 MC14 | CA1 CA3 CA4 CA6 CA8 | CC1 CC2 CC4 CC6 CC10 CC13 CC16 | BT1 | SH3 | AD1 AD2 AD3 AD4 AD5 AD6 AD14 | MB8 MB12 | UI1 UI2 UI3 UI5 UI6 UI7 UI8 | MA3 MA10 | TL8 | MS1 MS6 | DT1 DT4 DT6(refusé) DT7 | NV2 RL1 | MA3 | CB2/CB3 annulés
- BB14b : FAIT (sc/bb5) - House.keep/terrace + _build_keeps (donjon) plus produits par aucun générateur depuis BB14 → à supprimer [mech, Rust+GD].
- PF-07b : résidus --bench-ab/--bench-set/--map-ab dans battle_vegetation.gd, settlement_layer.gd (fichiers DN, après leur session), terrain_builder.gd, campaign_weather_view.gd, render_quality.gd [mech]. Préchauffage GPU post-export (--journey) retiré d export_macos.sh : à remplacer si saccades au 1er lancement.
- Fin de chantier : clippy workspace échoue sur sim-campaign (clippy::manual_checked_ops), signalé par veg.
- GT9 : un script de test qui ne compile pas sort en code 0 (Parse Error invisible pour CI/agents). Ajouter une vérif --check-only de game/tests/*.gd (CI ou outil) [mech]. Balayage du 09/10 : 8 cassés réparés (48172e05a, 2907579fa).

GT9/DT5 (reprise 09/10) : check_gd_scripts.sh vérifié (détecte un script cassé, rc 1) ; DT5 : colormap.py et ground_materials.py passent par codex.schema_validator ($ref résolus). pytest tools : 3226 OK, 3 échecs hors lot (rock outcrops glb absents, budget : aussi sur main ; relief_update tier3 : branche en retard sur main).
