# SC restes (état au 2026-10-09, main a3d23a2ff) - vérifs par grep/ls/wc, pas de lecture complète
Format : ID | état | reste | fichiers principaux | Rust? | [MÉCANIQUE]
(GELÉ FL = fichier récemment sale par d'autres sessions : settlement_layer, town_maquette_data, zoom_tiers, battle_vegetation, landmarks_v2, tools/experiments)

## DC docs
DC-3 | À FAIRE | docs/architecture.md absent, status.md non tronqué, lien README/CLAUDE.md | docs/status.md, README.md, CLAUDE.md | non
DC-4 | PARTIEL | docs/audit garde a1..a6 + recettes q1/q3/q5, images research non traitées | docs/audit, docs/research, docs/img | non
DC-5 | PARTIEL | docs/superpowers/plans (5) pas archivé, designs m1-m10 non marqués historique | docs/superpowers, docs/design | non
(DC-1 : wip 508 -> ~170 fichiers, archive chantiers.md 2236 l, jugé FAIT ; DC-2 INDEX.md présent)

## GT tests Godot
GT1 | FAIT (sc/hooks) | ~14 *_shot/probe/bench encore là (as1_shot, as5_shot, as8c_shader_probe, dn_*_shot, me1/me2/me5_shot, fl_weather_field_shot, tree_flicker_probe, codex_load_bench, mapload_bench) | game/tests/*_shot.gd | non
GT2 | FAIT (sc/hooks) | q3_playtest.gd reste (pb1_turns à garder) | game/tests/q3_playtest.gd | non
GT3 | PARTIEL | 151/197 *_test.gd sur TestCase, 46 non migrés | game/tests/*_test.gd, game/tests/lib/test_case.gd | non
GT4 | FAIT | windows.yml : étape « Godot tests » (shell bash, GODOT console) lance tools/run_godot_tests.sh (qui honore maintenant $GODOT) ; non exécuté sur Windows réel | .github/workflows/windows.yml, tools/run_godot_tests.sh | non
GT5 | FAIT (sc/gt5) | inventaire docs/wip/sc/gt5-inventaire.md : 200 tests, 25 rouges -> 6 restants (po_ui = bug layout, hb5/dn_* = paquet DN absent, 2 blocages à l'arrêt) ; bug SC fine_geo_layer is_enabled corrigé | game/tests | non
GT6 | À FAIRE | fusion des tests par thème | game/tests | non
GT7 | FAIT (sc/smoke, 5 smoke_*.gd) | smoke.gd 2893 l non découpé | game/tests/smoke.gd | non
GT8 | FAIT (sc/hooks) | .uid suivis (fait) ; hooks de capture orphelins : 20 occurrences _stage_/stage_screenshot/--stage restent | game/scripts/battle/battle_scene.gd, ui/*, map/* | non
HOOKS | FAIT (sc/hooks) | retirer stage_screenshot()/stage_example restants + smoke.gd:1150 | idem | non

## RT tests Rust
RT2 | FAIT | sondes/benchs supprimés (perf_step, parties de ep13/ep1_scale/relief/sb_siege_pace/ai_relief) ; 5 #[ignore] restants = régénérateurs ou test long documentés (m3_grid_ai, ep13_replay, br3, cb6_group_formation, fk_map_scenes) | core/crates/*/tests | oui
RT6 | FAIT f663a1c46 | review_tests/rs_c_tests/rs_n_tests toujours dans src (découpe) | core/crates/sim-campaign/src/review_tests.rs | oui
RT7 | À FAIRE (non mesuré, machine chargée) | raccourcir tests lents (mesurer d'abord) | core/crates/*/tests | oui
RT8 | À FAIRE (non fait) | fusion petits tests/tautologies (data-model 1 binaire, ok) | core/crates/*/tests | oui
RT9 | FAIT f663a1c46 | core/checks garde data_store_check et png_decode_check (garder campaign_sim_check seul) | core/checks/ | oui

## GB godot-bridge (304 #[func] contre 337)
GB6 | PARTIEL (partie soldats FAITE 6f0943c15 : poses écrites en place dans le paquet, sans Vec intermédiaire ni copie, -18 % en micro-bench ; reste TerrainMesher/scatter, autre agent) | TerrainMesher.build_patch, scatter() transforms, phases d'anim soldier_buffers (perf rendu) | core/crates/godot-bridge/src, game/scripts/battle | oui
GB7 | PARTIEL | key_enum! fait ; doc-comments restants (en dernier) | core/crates/godot-bridge/src | oui

## CB sim-campaign
CB1 | RIEN À FAIRE | settle_side_outcomes/losses_percent/retreat_losers n'existent plus nulle part dans core/ (grep `fn`), battle_flow absent : déjà refondu | - | -
CB7 | FAIT sc/ca2 | restes: custom.rs faction_name (sim-battle), From<Season>/dist, title_name, campaign_sim_diplomacy allies (filtre inconnus) laissés | core/crates/godot-bridge/src, sim-battle/src/custom.rs | oui
CB8 | FAIT | hostile_settlement_cells toujours fonction non cachée, passe-plats/depart() à vérifier | core/crates/sim-campaign/src/march.rs, movement/ | oui ; RIEN À FAIRE : depart() n'existe plus ; hostile_blocker garde 4 appelants ; cache dans CampaignState = risque de périmé sans gain mesuré (PlanCache existe pour l'IA)
CB10 | FAIT 7822c9ba6 | debug_stage_siege/landmark_siege/battle + debug_put_encounter_site regroupés dans src/staging.rs ; sg3_assault_probe.rs conservé (2 tests avec assertions, sonde en --ignored) | core/crates/sim-campaign/src/staging.rs | oui
CB12 | FAIT | constantes naval en data faites ; mémo win_chance non vérifié | core/crates/sim-battle/src/naval | oui ; RIEN À FAIRE : win_chance (vue UI) simule 5 combats par requête distincte, jamais deux fois les mêmes entrées
CB13 | FAIT | constantes siège/débarquement en data (movement) ; pub->pub(crate), doc Order non vérifiés | core/crates/sim-campaign/src | oui ; FAIT : breach_per_turn en pub(crate) (les autres pub sont utilisés par les tests d'intégration) ; Order déjà documenté
PROBES | FAIT (sc/cc) | examples diplomacy_probe, dynasty_probe, income_probe (sim-campaign) ; tests century/settlements/ia_quality probes à revérifier | core/crates/sim-campaign/examples | oui

## BB sim-battle
BB1 | FAIT (sc/bb1, ADR 0205) | village B5 : battle_village.gd 612 l + props/obstacles/ai/field/setup/bridge | game/scripts/battle/battle_village.gd, core/crates/sim-battle/src/{town,props,site}.rs | oui | [MÉCANIQUE]
BB14 | FAIT (sc/bb14) | générateurs borough/castle de siege_layouts.rs (912 l) à supprimer | core/crates/sim-battle/src/siege_layouts.rs | oui | [MÉCANIQUE]
BB5 | FAIT (sc/bb5) | WallPiece::new, SiegeWorks::skeleton, from_layout découpé (après BB14) | core/crates/sim-battle/src/siege_layout.rs, siege.rs | oui
BB6 | FAIT | battle_relief.json (shelf) et battle_site.json (portées obstacle/haie, couvert) + schémas + test_schemas | core/crates/sim-battle/src/{relief,site}.rs, data/rules | oui
BB8 | FAIT | place_plot/place_manor restent pub (tests d'intégration ep6_decor les appellent) ; decor_effect_at déjà unique | core/crates/sim-battle/src/decor.rs | oui
BB9 | NON RETENU | crossings déjà en cache (OnceLock, sim/water.rs) ; bridge_at = scan de ≤ quelques ponts, un index SegmentGrid n'apporterait rien et exigerait une invalidation sur Battlefield (champs pub mutables) | core/crates/sim-battle/src/hydro.rs | non
BB10 | FAIT | siege_fx::hash01 renommé jitter01 (flux conservé) ; copie vegetation de segment_distance supprimée ; hash01 de vegetation = miroir du shader, conservé | core/crates/sim-battle/src/siege_fx.rs | oui
BB12 | À FAIRE | pub->pub(crate) (dernier) | core/crates/sim-battle/src | oui

## PF perf transverse
PF-02 | FAIT (sc/relief, ADR 0203) | suppr relief_quadtree.gd (1295 l) sélection GD + fine_terrain_job.gd (167) + repli sans pyramide terrain_builder (1493) | game/scripts/map/{relief_quadtree,fine_terrain_job,terrain_builder}.gd | non (Rust optionnel) | [MÉCANIQUE visuelle]
PF-03 | FAIT (ADR 0204, sc/veg) | suppr vegetation_tile_job.gd (649 l) + repli GD vegetation.gd [GELÉ vegetation.gd] | game/scripts/map/vegetation_tile_job.gd, vegetation.gd | non
PF-06 | PARTIEL | CmdArgs partout ; 46 interrupteurs --no-* encore présents ; sc/devflags prêt (1f7987144) mais NON fusionné (attend settlement_layer propre) ; sc/devflags 10-09 (refait à la main, main déjà largement nettoyé) : retirés --no-countryside/--no-fields/--no-dn-trees, has_method(duck_music) morts, mentions --no-* des shaders. Reste : --no-fa-grass (battle_vegetation gelé), uniform ga4_on/sea_life_on (data-pilotés, terrain/water), --no-hud/--no-fps/--no-fog (outils de capture, gardés) | game/scripts/**, branche sc/devflags (../gp-sc-devflags) | non
PF-07 | FAIT (sc/bench) | map_bench.gd 534, release_journey.gd 525, perf_probe.gd, _bench_* battle_scene (campaign_map ex-gelé FL) | game/scripts/dev/*.gd | non
PF-04 | RIEN À FAIRE (mesure 09/10 : parse serde release des 5 fichiers, 17,5 Mo = 57 ms cumulés (provinces 5,5 ; rivers 7,7 ; coastline 4,1 ; roads 17,5 ; rivers_render 22,6), soit ~28 % des ~200 ms de chargement DT4 et < 60 ms ; le reste est la conversion Godot ; un bake .bin gagnerait ~40 ms au mieux) | tools/cent_ans_tools/geo, core/crates/godot-bridge/src/map_geo.rs | non
PF-01 | À FAIRE | OutbuildingPacker Rust [GELÉ FL] | game/scripts/map/outbuilding_layer.gd | oui
PF-10 | À FAIRE | declutter Rust (marker_declutter.gd) [GELÉ FL] | game/scripts/map/marker_declutter.gd | oui
PF-08 | FAIT 24bbff975 | StampMap stamp_soft_disc + relief_from_heights pour battle_terrain | core/crates/godot-bridge/src/stamp_map.rs, game/scripts/battle/battle_terrain.gd | oui
PF-09 | FAIT | parchment_decor précalc (sea_items résolus au build) | game/scripts/map/parchment_decor.gd | non
PF-11 | PARTIEL (GDScript heights_m_at en lot ; pas d appel Rust batch dans le pont) | heights_m batch | game/scripts/map | oui
PF-12 | FAIT (TileJobPool : rock_outcrops, ground_clutter, landmark_*, terroir_mask, relief_quadtree, fine_geo_store, road_renderer ; les *_job.gd sont des charges de travail, pas des files) | TileJobPool commun (15 fichiers) | game/scripts/map/*_job.gd | non
PF-13 | PARTIEL | DataFile/JsonLookup largement posés ; 92 lectures JSON brutes restent (136 au départ) | game/scripts/** | non
PF-14 | FAIT (= DT2) | PNG replis relief (relief_shade_[0-3].png 131 Mo = DT2) | game/scripts/map/relief_landcover.gd, tools/cent_ans_tools/export_data.py | non

## BA sim-battle moteur
BA1 | FAIT | plus aucun example sim-campaign (dossier absent) ; sim-battle/ai propres | core/crates/sim-campaign/examples | oui
BA8 | FAIT b19d13f32 | replay digest simplifié, builder ReplayStart, suppr soldier_positions alias (unit.rs:961) | core/crates/sim-battle/src/replay.rs, unit.rs | oui
BA9 | FAIT b19d13f32 | cache figure_positions layout local | core/crates/sim-battle/src/unit.rs | oui
BA10 | FAIT | apply_command déjà découpé en handlers par commande (command_move, command_attack, …) avec dispatch par match ; audit 10-09, rien à changer | core/crates/sim-battle/src/sim/commands.rs | oui
BA11 | FAIT | audit 10-09 : sim/fire.rs = incendies de siège, déjà pilotés par data/rules/siege_fire.json ; tables de projectiles déjà en data (missile_arc/missile_morale) ; aucune fonction > 80 l ; rien à extraire | core/crates/sim-battle/src/sim/fire.rs | oui
BA12 | FAIT | primitive unique `Strip` (bande de grille : curseur + sens + écart) pour rangées, ailes et colonne de group_formation.rs | core/crates/sim-battle/src/group_formation.rs | oui
BA13 | FAIT c55198a89 | MovementRules data (sim/movement.rs 624 l) | core/crates/sim-battle/src/sim/movement.rs | oui
BA14 | À FAIRE | pub(crate) + docs lib.rs | core/crates/sim-battle/src/lib.rs | oui

## MC map
MC1 | FAIT (sc/mc1) | png16.gd + replis 8 bits/big-endian/PageJob.run supprimés ; décodeur Rust obligatoire (shaders gardent la branche height_bpp==2, inerte) | - | non
MC3 | PARTIEL | = PF-06 (sc/devflags) ; 5 get_cmdline restants ; voir PF-06 (sc/devflags 10-09) | game/scripts/util/cmd_args.gd | non
MC6 | FAIT (sc/army) | folk_scenes 680->436 l + folk_scene_layers 346 l ; 10 scènes décrites par data/rules/folk_scene_layouts.json (schéma) sur 4 primitives de couche (site, scatter, procession, convoy) | game/scripts/map/life_folk | non
MC8 | FAIT (sc/army) | army_markers 656->397 l ; ArmyPlate, ArmyPlateLayout, ArmyScale, ArmyPicker extraits | game/scripts/map | non
MC10 | FAIT (sc/relief, ADR 0203) | = PF-02 | - | non
MC11 | FAIT (relief_state.gd, MapData délègue) | ReliefState hors MapData | game/scripts/map/map_data.gd | non
MC13 | FAIT (sc/army) | MapInstancing + ScreenSigns (base marqueurs/feedback) ; AI replay en un seul mode (réglage map/ai_moves retiré, reste la vitesse) | game/scripts/map | non
MC15 | À FAIRE | commentaires Lot/ADR (293) + doc-comments (EN DERNIER) | game/scripts/** | non

## CA sim-campaign diplo/agents
CA2 | FAIT sc/ca2 | chronicle 1361 -> ~870 l (event_actions.rs, plague.rs) ; apply/describe déjà unifiés dans effects.rs | core/crates/sim-campaign/src/chronicle.rs | oui
CA5 | FAIT sc/ca2 | copies splitmix (retinue, ai/alignment) et alias label (ai/feudal, march) supprimés ; siege_fx hash01 gardé (stream de bataille) | core/crates/data-model/src/{load,util}.rs, godot-bridge | oui
CA7 | FAIT sc/ca2 | dynasty -> data/rules/dynasty.json + schéma (DynastyRules) ; feudal MAX_DEPTH = garde-fou structurel, laissé | core/crates/sim-campaign/src/{dynasty,feudal}.rs | oui
CA9 | FAIT fdf3c8465 | dynasty marriage_blocker partagé, crusade/tests.rs (feudal/escalation.rs absent) | core/crates/sim-campaign/src/{dynasty,crusade}.rs | oui
CA10 | À FAIRE | doc Lot tags (dernier) | core/crates/sim-campaign/src | oui

## CC sim-campaign reste
CC3 | FAIT sc/ca2 (= CA5) | - | oui
CC5 | FAIT | TurnBudget fait ; economy.rs vérifié : province_income, faction_economy, receipts encore appelés (ai, bridge, starting_fit), aucun alias ni reste mort | core/crates/sim-campaign/src/economy.rs | oui
CC7 | ÉCARTÉ (déjà une table, sc/cc) | rule_constants table | core/crates/sim-campaign/src/rule_constants.rs | oui
CC8 | FAIT c9eb470d1 | table.rs+edicts.rs ProvincePolicy, medicine->population | core/crates/sim-campaign/src/{table,edicts,medicine}.rs | oui
CC9 | FAIT aa98fbad0 | ajustement garnisons JR4b -> data pré-calculée | core/crates/sim-campaign/src/setup_1337.rs | oui | [MÉCANIQUE]
CC11 | FAIT | sg3_assault_probe.rs réduit à ses 2 tests (réglages ENV ENGINES/ATTACKER_SHARE/DUMP, dump_units et champs inutilisés supprimés ; nom gardé, cité par des ADR) ; jr4b_starting_budget est un vrai test ; aucune autre sonde (century/settlements/ia_quality : rien de tel) | core/crates/sim-campaign/tests | oui
CC12 | FAIT (12 fns, sc/cc) | code mort 15 fns | core/crates/sim-campaign/src | oui
CC14 | FAIT | ai découpé (AITURN) ; diplomacy/, agents/, crusade/ en sous-modules, apply_effects (195 l) découpée | core/crates/sim-campaign/src | oui
CC15 | À FAIRE | doc tags (620) + lib.rs sous-dossiers (dernier) | core/crates/sim-campaign/src | oui

## BT battle (3D)
BT2 | PARTIEL | stages/hooks retirés ; bench + flags A/B restent dans battle_scene (2439 l) | game/scripts/battle/battle_scene.gd | non
BT3 | À FAIRE | figurine rigide : battle_meshes.gd 1093 l, battle_soldier.gdshader (garder engins), 27 glb | game/scripts/battle/battle_meshes.gd, game/shaders/battle_soldier.gdshader | non
BT4 | PARTIEL | = PF-06 (sc/devflags, 34 flags A/B + 64 has_method) ; voir PF-06 (sc/devflags 10-09) | game/scripts/battle | non
BT5 | FAIT 6f0943c15 (fine_near + hide en Rust via fine_near_buffer/fold_figure_slots ; loosen était déjà dans le cœur depuis RJ-b) | buffers fine_near/hide/loosen en Rust (perf) | game/scripts/battle/battle_soldiers.gd, core/crates/godot-bridge | oui
BT6 | À FAIRE | manifeste skinné cuit hors ligne, suppr NT12/NT13 mocap trials | game/scripts/battle/battle_skinned.gd, tools | non
BT7 | FAIT 24bbff975 (Rust: hauteurs, rivière, relief, maillages ; split Splat/Mesh/Scatter) | height/river battle_terrain en Rust, split Terrain/Mesh/Decor (2060 l) | game/scripts/battle/battle_terrain.gd | oui
BT8 | FAIT | plan_deployment en Rust | game/scripts/battle/deployment_controller.gd | oui
BT9 | FAIT c6ad59bfc (KINDS seul ; constantes visuelles -> data non faites) | constantes -> data, KINDS dupliqué (soldiers+scene) | game/scripts/battle | non
BT10 | À FAIRE | MultiMeshKit/ParticleKit | game/scripts/battle | non
BT11 | FAIT 5a8c696d2, ADR 0230 (queue_tip gardé) | suppr duels/birds/cloud_shadows/queue_tip/secondary_motion (fichiers présents) -800 | game/scripts/battle/battle_{duels,birds,cloud_shadows,queue_tip,secondary_motion}.gd | non | [MÉCANIQUE cosmétique]
BT12 | À FAIRE | battle_scene structure replay/banners/audio, perf _refresh_view | game/scripts/battle/battle_scene.gd | non

## SH shaders
SH1 | PARTIEL | fx_noise fait pour fx ; ~24 copies hash/vnoise/fbm restent, noise_common absent (terrain GELÉ) | game/shaders/*.gdshader | non | [visuel léger]
SH2 | À FAIRE | campaign_map_data.gdshaderinc + river_common (terrain GELÉ) | game/shaders | non
SH5 | PARTIEL | = BT4/PF-06 (uniform bool da6/ga*/sr2 restent dans battle_ground, battle_soldier_skinned) ; voir PF-06 (sc/devflags 10-09) | game/shaders | non
SH6 | À FAIRE | sea_nearby cache terrain (GELÉ) | game/shaders/terrain.gdshader | non
SH7 | FAIT 31cca7fa1 (2 uniformes morts ; fonctions mortes non traitées) | uniformes/fonctions morts | game/shaders | non
SH8 | À FAIRE | battle_ground detail_height 9 lectures -> dFdx (change le rendu, écarté par battledev) | game/shaders/battle_ground.gdshader | non | [visuel]
SH9 | FAIT | flipbook (fx_flipbook) + particule (fx_particle) partagés par fire_* et life_* | game/shaders | non
SH10 | ÉCARTÉ : beaucoup d'uniformes posés par nom construit (prefix + clé, clés des données) ; const = réglage ignoré en silence, invérifiable sans rendu GPU | uniformes jamais posés -> const | game/shaders | non

## AD ai/data-model/relief-lod/vegetation
AD7 | FAIT (ADR 0204, sc/veg) | vegetation V4 legacy (species None) + repli GD vegetation_tile_job | core/crates/vegetation/src/lib.rs, game/scripts/map/vegetation_tile_job.gd | oui
AD8 | FAIT (sc/relief, ADR 0203) | relief_quadtree.gd repli GD (= PF-02), has_page mort | core/crates/relief-lod, game/scripts/map/relief_quadtree.gd | oui
AD9 | FAIT | data_model::util::segment_distance_xz (f64) partagé par sim-battle::geom et vegetation | core/crates/data-model/src/util.rs | oui
AD10 | FAIT f663a1c46 | Cargo.toml profils redondants (profile.dev.package.* x5) | core/Cargo.toml | oui
AD11 | FAIT | fallback_edges/fallback_city_id toujours là | core/crates/data-model/src/{movement_graph,settlement_load}.rs | oui ; RIEN À FAIRE : fallback_edges/fallback_city_id sont testés (c4_settlements) et servent aux données réduites
AD12 | FAIT 99a2b89fa | perf ai grid near spatial, threat_by_province, Arc<str> | core/crates/ai/src/grid.rs | oui
AD13 | FAIT | plan_field découpé en sous-modules documentés (cf. SIMSPLIT) | core/crates/ai | oui

## MB map terrain/nature
MB1 | FAIT (sc/relief, ADR 0203) | = PF-02 (pyramide obligatoire, suppr tuilé, png16, 672 tuiles far) | game/scripts/map/terrain_builder.gd | non
MB2 | FAIT (sc/relief, ADR 0203) | relief_quadtree sélection GD (= AD8b) | game/scripts/map/relief_quadtree.gd | non
MB3 | FAIT (ADR 0204, sc/veg ; vegetation_tile_job.gd gardé réduit à la requête) | = PF-03 | game/scripts/map/vegetation_tile_job.gd | non
MB4 | À FAIRE | StreamedTileLayer commun (5 couches) [vegetation GELÉ] | game/scripts/map | non
MB5 | PARTIEL | flags A/B végétation GA3/FC (--no-ga3-veg x7, --no-fc2/5) ; voir PF-06 (sc/devflags 10-09) | game/scripts/map | non
MB6 | FAIT (sc/dt2) | relief_cache_notice (117) + relief_cache_status (241) -> push_warning | game/scripts/map/relief_cache_*.gd | non
MB7 | FAIT | parchemin décor marin statique, RedrawOnDemand (plus de redessin périodique ; shader sans TIME) | game/scripts/map/parchment_decor.gd, shaders/parchment_sea.gdshaderinc | non | [MÉCANIQUE visuelle]
MB9 | À FAIRE | life_effects points typés (life reground GELÉ) | game/scripts/map/life_effects.gd | non
MB10 | FAIT | war_scars voie events suppr | game/scripts/map/war_scars.gd | non
MB11 | FAIT | vegetation_mask repli sans splat | game/scripts/map/vegetation_mask.gd | non
MB13 | FAIT | morts | game/scripts/map | non
MB14 | À FAIRE | étude fusion rendus rivière/route | game/scripts/map/{rivers,road}_renderer.gd | non

## UI
UI4 | FAIT sc/ui (MechanicSheet + data/ui/encyclopedia.json) | encyclopedia MECHANICS (const L57) -> data/ui/encyclopedia.json + fiche commune | game/scripts/ui/encyclopedia.gd | non
UI9 | FAIT sc/ui (thousands retiré, UiBuild.spacer partout ; spacer vertical army_strip laissé) | morts UI (thousands 2 occurrences, spacer) | game/scripts/ui | non
UI10 | À FAIRE | doc (dernier) | game/scripts/ui | non
UI11 | FAIT sc/ui ADR 0237 ; 3 tscn triviaux retirés, 11 coquilles restantes listées dans l ADR | tscn vs code (décision, 25 tscn) | game/ | non
UI12 | RIEN À FAIRE : MenuBackdrop3D instancié par start_menu.gd, LivingPortrait utilisé par portrait_frame/loader/family_tree | menu_backdrop_3d.gd, living_portrait.gd suppr ? (liés MM1 menu récent, prudence) | game/scripts/ui | non | [MÉCANIQUE visuelle]

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
MA12 | FAIT (sc/hooks) | stage_screenshot/morts du settlement_controller | game/scripts/map/settlement_controller.gd | non

## TL tools
TL1 | PARTIEL | tldel fait ; proto_moteur (5 fichiers) reste (experiments hors périmètre) | tools/proto_moteur | non
TL2 | FAIT (sc/tools) | modules revérifiés, 5 fonctions mortes retirées (vulture) | tools/cent_ans_tools | non
TL3 | RIEN À FAIRE (map_markers déjà absent ; *_raw = sources brutes de ink_icons/entity_icons/horizon_panoramas, gardées, ADR 0235) | map_markers + *_raw (da5_raw, da5b_raw, horizon_raw) -26 Mo | tools/da5_raw, da5b_raw, horizon_raw | non
TL4 | FAIT (sc/tools) | revérifié ; outils GA3 gardés | tools/blender_scripts | non
TL5 | FAIT (sc/tools, 9 fichiers, -971 l) | mocap/FA3/AN1b/FG blender -4.8k (tools/video_mocap, blender) | tools/blender, video_mocap | non
TL6 | FAIT (rien de mort : pipelines câblés+testés gardés, ADR 0235) | pipeline payant OpenRouter/fal/TTS -8k | tools/cent_ans_tools/{openrouter,voice_tts,portraits,material_gen,local_art}.py | non
TL7 | FAIT (idem TL6, ADR 0235) | audio/UI art gen -5.9k [ADR] | tools/cent_ans_tools/{ui_ornaments,audio_bank,ui_sounds,era_music}.py | non
TL10 | FAIT 3aab598d7 | cli.py 2051 l à découper | tools/cent_ans_tools/cli.py | non
TL12 | FAIT 3db03ba04 (rien à factoriser : aucune fonction identique, implémentations distinctes) | battle_skinned vs fine doublons | tools/cent_ans_tools | non
TL13 | RIEN À FAIRE (pas de doublon) | kit_geometry | tools/cent_ans_tools | non
TL14 | PARTIEL 3db03ba04 (paths.py fait ; imaging.py non : pas de helper dupliqué) | paths.py/imaging.py | tools/cent_ans_tools | non
TLR | FAIT (sc/tools) | descriptions schémas / commentaire siege_engines_fx.gd:89 citant outils supprimés | data/schemas, game/scripts/battle/siege_engines_fx.gd | non

## MS scripts divers
MS2 | FAIT (= PF-07) | = PF-07 (MapBench/PerfProbe/--bench-map/gen_synthetic_map) | game/scripts/dev, game/tools/gen_synthetic_map.gd | non
MS3 | PARTIEL | DataFile/JsonLookup posés ; battle/ui restants (voir PF-13) | game/scripts | non
MS4 | FAIT | codex_bubbles 1215→1099 l, BubbleLayout (156 l), set_process conditionnel | game/scripts/codex/codex_bubbles.gd | non
MS5 | FAIT | play_sfx via VoicePool (plus de round-robin) ; aucun volume/EVENT_SFX codé en dur dans audio_director | game/scripts/audio | non
MS6 | FAIT? render_quality.json existe -> voir FAIT
MS7 | À FAIRE | assets sans référence (textures/buildings brutes 26 Mo ; quaternius 95 fichiers suivis) | game/assets, tools | non
MS8 | FAIT | LUT étalonnage atmosphere_library.grade_lut port Rust/précuisson | game/scripts/visual/atmosphere_library.gd | oui — port Rust `GradeLut` (core/crates/godot-bridge/src/grade_lut.rs), identique octet à octet, ~20x plus rapide (debug) ; test game/tests/ms8_grade_lut_test.gd
MS9 | FAIT (release_journey suppr) | release_journey --map-ab/--ab-configs/--uncapped | game/scripts/dev/release_journey.gd | non
MS10 | FAIT d3d86f0c0 | passe commentaires visual/ audio/, constantes battle_audio -> sound_bank.json | game/scripts/{visual,audio} | non

## DT / divers
DT2 | FAIT (sc/dt2, PNG dé-suivis gardés pour geo) | relief_shade_[0-3].png (131 Mo) toujours suivis ; supprimer repli + dé-suivre | data/map/relief_shade_*.png, game/scripts/map/relief_landcover.gd | non
DT3 | REPORTÉ | fusion landmarks v1->v2 (= MA1/MA2 sales) | - | non
DT5 | FAIT 7d3292c3b | defs communes color_hex/rgb3/snake_id dans common.schema.json | data/schemas | non
DT8 | FAIT (déjà rattachés : town_footprint_rules, forced_sea_edges ; aucun schéma orphelin) | schémas town_footprint, forced_sea_edges, 4 orphelins à vérifier | data/schemas | non
SCH | FAIT (sc/tools : biomes.py + 2 tests via schema_validator) | ~10 tests tools avec Draft202012Validator nu, 12 schémas sans registre | tools/tests, tools/cent_ans_tools/geo | non
PRE | À FAIRE | test_relief_update bake tier3 5 vs 6 (autre session) | tools/tests/test_relief_update.py | non
SIMSPLIT | FAIT | ai/plan_field.rs → ai/plan_field/{mod,charge,measures,reserve,orders}.rs, plan_engines extrait | core/crates/sim-battle/src/ai/plan_field/ | oui
NAVAL-reste | FAIT | constantes ship.rs en dur, pending.remove(0) | core/crates/sim-battle/src/naval/ship.rs, sim-campaign/src/naval.rs | oui ; FAIT : crew_ammo_cap en data/naval/rules.json + schéma, pending en VecDeque
BATTLEDEV | PARTIEL | RenderQuality.override_level/upscale_override, da6/site_render/fa_on (battle_vegetation GELÉ) | game/scripts/battle/battle_vegetation.gd | non
UIKIT | FAIT sc/ui (thousands, make_panel, attach_plain retirés de RichTooltip) | RichTooltip.thousands (encyclopedia), délégués make_panel/attach_plain | game/scripts/ui/rich_tooltip.gd | non
BUGS-ouverts | À FAIRE | 2 tests Godot préexistants (cb0 golden ?), LOOKUPS2 FAIT (sc/lk2 : game/scripts propre, 2 lookups morts retirés dans tests pb1_turns/q8_start_faction) | game/scripts | non

## FAITS
DC-1 DC-2 | GT3(partiel) | RT1 RT3 RT4 RT5 | GB1 GB2 GB3 GB4 GB5 | CB4 CB5 CB6 CB9 CB11 | BB2 BB3 BB4 BB7 BB11 BB13 | PF-05 | BA2 BA3 BA4 BA5 BA6 BA7 | MC2 MC4 MC5 MC7 MC9 MC12 MC14 | CA1 CA3 CA4 CA6 CA8 | CC1 CC2 CC4 CC6 CC10 CC13 CC16 | BT1 | SH3 | AD1 AD2 AD3 AD4 AD5 AD6 AD14 | MB8 MB12 | UI1 UI2 UI3 UI5 UI6 UI7 UI8 | MA3 MA10 | TL8 | MS1 MS6 | DT1 DT4 DT6(refusé) DT7 | NV2 RL1 | MA3 | CB2/CB3 annulés
- BB14b : FAIT (sc/bb5) - House.keep/terrace + _build_keeps (donjon) plus produits par aucun générateur depuis BB14 → à supprimer [mech, Rust+GD].
- PF-07b : résidus --bench-ab/--bench-set/--map-ab dans battle_vegetation.gd, settlement_layer.gd (fichiers DN, après leur session), terrain_builder.gd, campaign_weather_view.gd, render_quality.gd [mech]. Préchauffage GPU post-export (--journey) retiré d export_macos.sh : à remplacer si saccades au 1er lancement.
- Fin de chantier : clippy workspace échoue sur sim-campaign (clippy::manual_checked_ops), signalé par veg.
- GT9 : un script de test qui ne compile pas sort en code 0 (Parse Error invisible pour CI/agents). Ajouter une vérif --check-only de game/tests/*.gd (CI ou outil) [mech]. Balayage du 09/10 : 8 cassés réparés (48172e05a, 2907579fa).

GT9/DT5 (reprise 09/10) : check_gd_scripts.sh vérifié (détecte un script cassé, rc 1) ; DT5 : colormap.py et ground_materials.py passent par codex.schema_validator ($ref résolus). pytest tools : 3226 OK, 3 échecs hors lot (rock outcrops glb absents, budget : aussi sur main ; relief_update tier3 : branche en retard sur main).
