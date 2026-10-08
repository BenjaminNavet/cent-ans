# Lots issus de l'audit (résumé orchestrateur)
## DC docs
DC-1 archive 368 wip clos -> docs/archive/chantiers.md (6 grappes préfixes), git rm ; listes dans scratchpad/cls.json (op/cl/cs). -50k lignes
DC-2 docs/decisions/INDEX.md (doublons 0117/0146/0149 bis, prochain=0186)
DC-3 docs/architecture.md + status.md tronqué + lien README/CLAUDE.md
DC-4 suppr docs/audit/a2-sonde, images research non référencées, condense recettes/histoire audits (après DC-1)
DC-5 designs m1-m10 marqués historique, superpowers -> archive (après DC-1)
## GT tests Godot (CI = smoke.gd seul ; ~180 *_test jamais lancés)
GT1 suppr ~110 shots + sondes (garder vh4/vh7_shots, readme_gallery/shots, ui_resolutions.sh, po_shot, pb1_turns) -12k
GT2 suppr bancs/playtests (garder pb1_turns) -3.5k
GT3 lib/test_case.gd + migration 157 *_test -3.5k
GT4 run_all runner + CI ; corriger dv_two_views_test
GT5 suppr tests rouges/orphelins ; GT6 fusion par thème ; GT7 découper smoke.gd ; GT8 .uid
## RT tests Rust (199 fichiers 54.5k ; 1269 tests rechargent GameData)
RT1 suppr 11 sondes ignore-only (br3_assault_probe, ia_survey, sg4_balance, r4_survey, l13_duration, l13_trace, l13b_behaviour, ep3_probe, jr4b_budget_probe, lr15_budget_breakdown, ai jr_crusade_probe) -3k
RT2 suppr #[ignore] probes dans ep11/ep9/ep9b/ep10/b6/eq7/f5d/ep7/cb2/cb4/sg3/dp2/f7/dp1/c6/review_tests/preview ; garder m3 fifty_turns, fk player_incident_rate, cb6 write_deploy_golden, ep13 write_sample -1.7k
RT3 test_support (data-model feature) data() OnceLock, fac(), prov() ; remplacer 121 data() + 99 fac() -1.2k + perf
RT4 common helpers campagne/ai (idle,start,france,main_army...) + src/test_util.rs (après RT3)
RT5 fusion 199 fichiers -> ~25 binaires thématiques (après RT1-4), 1 sous-lot par crate, compter tests avant/après
RT6 review_tests découpe ; RT7 raccourcir tests lents (mesurer d'abord) ; RT8 fusion petits tests/tautologies ; RT9 core/checks garder campaign_sim_check seul
## GB godot-bridge (337 #[func])
GB3 suppr #[func] mortes (get_field_size, get_waterside_spot, make_image, get_agent_path/upkeep/types, get_price_levels, get_map_scene_rules, get_plague_resistance, get_wound_recovery, get_visible_army_ids, submit_order_report?, clear_pages/slots non-func, get_warnings/faction_ids/character_ids/trait/skill, load_mask_u8/rgb8 + checks dev) -300 ; convention: une #[func] doit avoir un appelant dans game/
GB1 ctx()/ctx_mut() remplaçant 102 gardes + resolve_reply helper + warn_empty -400 (après GB3)
GB2 convert.rs: json_to_variant unique, to_dict/from_dict/to_array, reasons_array -200 (après GB1)
GB4 battle_sim.rs 2085l éclaté en battle_sim_terrain/campaign/units (#[godot_api(secondary)]), sous-fonctions
GB5 trait/macro commun BattleSim/NavalBattleSim ; Columns builder provinces_snapshot (après GB1+GB4)
GB6 nouvelle API Rust: TerrainMesher.build_patch, scatter() transforms, anim phases dans soldier_buffers (après GB2)
GB7 doc-comments, enum key() au lieu de format!("{:?}").to_lowercase() (en dernier)
## CB sim-campaign orders/movement/siege...
CB6 battle_auto: suppr side_power/average_armor/effective_armor + examples/forecast_probe ; struct Engagement/Combatant, une fonction resolve ; découpe resolve_with_crossings ; RNG même ordre
CB13 code mort (SEA_EDGE_COST, CAPTURE_UNREST, Retreat wrapper...), pub->pub(crate), constantes siège -> data/rules, doc-comments Order
CB7 GameData::{province_name,faction_name,settlement_name,building_name,tech_name} + From<Season> + dist() ; remplace copies dans TOUS fichiers sim-campaign (transverse, en dernier)
CB10 debug_* -> staging.rs ; suppr debug_stage_landmark/place_siege + sg3_assault_probe si démos
CB8 own_army_checked dup, depart() helper, MoveReport::from_walk, passe-plats ; cache hostile_settlement_cells
CB5 RecruitContext + enum RecruitBlock (perf x10-50)
CB4 orders.rs découpé orders/{mod,recruit,army}.rs, apply par variante
CB1 battle_flow.rs: settle_side_outcomes, losses_percent, retreat_losers, winner_rewards -250 ; hash playthrough ai/examples/turn_digest
CB9 movement.rs -> movement/coalition(field_battle)/retreat (après CB1)
CB11 unit_bonus partagé auto vs 3D
CB2 ANNULÉ (joueur 10-08 : garder issue bataille des rencontres)
CB3 ANNULÉ (joueur 10-08 : garder 4 issues de prise + ruines)
CB12 naval constantes -> data ; mémo win_chance
Transverse: Dijkstra dupliqué agents.rs/ai grid.rs/review_tests ; examples/*_probe.rs à supprimer (ai + sim-campaign)
## BB sim-battle décor/hydro/siège/naval
BB1 suppr village B5 (doublon décor EP6) site/props/obstacles/ai/field/setup/bridge/battle_terrain.gd -600 [MÉCANIQUE]
BB14 suppr générateurs borough/castle (siege_layouts.rs 925l), garder réel+octogone -800 [MÉCANIQUE] avant BB5/BB7
BB3 naval perf: Arc<NavalRules>, VecDeque (avant BB2) ; BB2 auto_resolve découpé + constantes -> NavalRules data
BB4 geom.rs commun (8 copies segment_distance) ; BB5 WallPiece::new, SiegeWorks::skeleton, from_layout découpé (après BB4,BB14)
BB6 tables terrain -> data/rules battle_relief/site.json ; BB7 decor_gen découpé en module ; BB8 place_* pub(crate), decor_effect_at unique
BB9 cache crossings/bridge_at ; BB10 rng count/hash01/mix unique ; BB11 macro key_enum! (transverse) ; BB12 pub->pub(crate) dernier ; BB13 naval scenario types fusion
## PF perf transverse (motif: voie rapide + repli GDScript conservé)
PF-05 suppr campaign_sim_mock.gd (1822l) + repli SimFacade, saves "mock" -1.9k
PF-02 suppr sélection GDScript relief_quadtree + FineTerrainJob + repli sans pyramide (terrain_builder) -700 [FL touche? terrain_builder pas listé]
PF-03 suppr semis GDScript vegetation_tile_job.gd + repli vegetation.gd -400 [vegetation.gd GELÉ FL]
PF-06 suppr 63 interrupteurs --no-* A/B + autoload DevFlags -700 (bataille: battle_soldiers per_kind, battle_terrain stamp GDS)
PF-07 bancs: garder map_bench+PerfProbe réduit ; suppr release_journey, fps_probe, laps, _bench_* battle_scene -900 [map_bench/campaign_map GELÉ FL]
PF-04 bake .bin des gros geojson/json carte (tools geo bake-lines) -1s chargement
PF-01 OutbuildingPacker Rust [GELÉ FL -> annoncer] ; PF-10 declutter Rust [GELÉ FL]
PF-08 StampMap stamp_soft_disc + relief_from_heights pour battle_terrain
PF-09 parchment_decor précalc ; PF-11 heights_m batch ; PF-12 TileJobPool commun (15 fichiers) ; PF-13 JsonFile.read (136 occurrences) ; PF-14 PNG replis relief
## BA sim-battle moteur
BA1 = RT1 sondes (+ examples/probe.rs, sg4_balance, ia_survey...) -2.4k
BA2 sim.rs 3405l -> sim/{melee,shooting,morale,movement,commands,setup}.rs déplacement pur (digest ep13 inchangé) EN PREMIER
BA3 morale/fatigue factorisé, constantes -> data, log_unit helper (après BA2)
BA4 perf voisinage par pas (contacts x2, n² sqrt -> dist²), ordre stable digest (après BA2)
BA5 perf fork_for_step: Arc<BattleSetup>/Arc<Battlefield> make_mut, Arc<str> (gros gain mémoire)
BA6 ai.rs 3041 -> ai/{view,roles,plan_field,shooter,horse,siege}.rs ; geom commun ; constantes -> data/rules/battle_ai.json
BA7 macro bundled_rules! (28 modules OnceLock/include_str) -200 (transverse, aussi naval/decor)
BA8 replay digest simplifié (rompt anciens replays OK), builder ReplayStart, suppr soldier_positions alias
BA9 cache figure_positions layout local (perf rendu)
BA10 apply_command découpé (après BA2) ; BA11 missiles table data + fire() découpé ; BA12 géométrie grille formations unifiée ; BA13 MovementRules data ; BA14 pub(crate) + docs lib.rs
geom commun: BB4 + BA6 + BA13 -> un seul lot sim-battle/src/geom.rs
## MC map (armées, folk, contrôleurs...)
MC2=PF-13 JsonData.load helper (28 _data_dir, 82 parse, 67 preload map_paths) -500 TRANSVERSE FONDATION (vague 1)
MC3=PF-06 CliFlags/DevFlags autoload + suppr drapeaux morts FONDATION
MC7 Hash.h01 commun (6 copies _h) + fonctions mortes folk FONDATION
MC1 décodeur Rust heightmap obligatoire, suppr png16/8bit -300
MC4 tutoriel féodal -> TutorialSteps ; stage_screenshot hors prod -250 ; MC5 tutorial data-driven -200
MC6 folk_scenes 10 -> 4 archétypes, suppr flood/celebration -300 [MÉCANIQUE visuelle]
MC8 army_markers refresh découpé, dirty flag plates, fonctions mortes, legacy markers -120
MC9 panneaux UI construits en code -> constructeur commun, ConfirmDialog (war_declaration+raze) -400
MC10 FineTerrainJob suppr (=PF-02) ; chaîne fine_* garder (pyramide)
MC11 ReliefState hors MapData ; MC12 map_mode table -200 ; MC13 fusion petits marqueurs/feedback, AI replay 1 mode -300
MC14 JsonLookup base + exports morts map_prop_scale -300 (après MC2)
MC15 commentaires Lot/ADR (293) + doc-comments -1000 (EN DERNIER)
## CA sim-campaign diplo/agents/chronicle/...
CA1 Proposal Peace/Truce/Alliance/Vassalage/Marriage -> Treaty{articles} -500..-800 [haut, MÉCANIQUE-structure] avant CA6
CA2 chronicle.rs -> effects.rs, apply/describe_effect unifiés -250
CA3 agents: 10 actions -> fusion Truce+Parley, suppr Incite/Counter/Denounce?; ActionSpec table -300..-450 [MÉCANIQUE]
CA4 agent_dijkstra_by_ids+test omr_r1_agent_paths suppr; AgentTable vs movement::dijkstra -120..-200
CA5=CC3 names.rs (province_name x10, faction_name x7, is_rebels, faction_label, settlement_name, capitalized) + rng::splitmix FONDATION
CA6 diplomacy evaluate / negotiation article_value split + constantes -> data + ReasonList -150 (après CA1)
CA7 constantes diplo/nego/dynasty/feudal -> data rules structs ~0
CA8 _walk/_by_ids privés/cfg(test) -40 (lié CC10)
CA9 dynasty marriage_candidates dupli, crusade tests inline -> tests file, feudal/escalation.rs (structure)
CA10 doc Lot tags (dernier)
probes à suppr: tests century_probe.rs, settlements_probe.rs, ia_quality_probe.rs
## CC sim-campaign reste
CC1 tests/common/mod.rs: data() OnceLock + fac/prov... (104 copies) -700..-900 + tests plus rapides FONDATION
CC2 setup_1337 new_1337 découpé + link helper + Default derive FactionState/CharacterState/CampaignState::empty -250
CC4 EffectTotals 36 champs -> tableau indexé EffectKind -200 (après CC3; ai/bridge lisent)
CC5 economy legacy (province_income, faction_income, effective, alias) + TurnBudget::compute + last_budget unique -150
CC6 saves: STATE_VERSION 9, suppr Pre*Save, QueuedRecruitRepr, serde defaults compat, tests old_save -250..-350 [saves cassées: ok]
CC7 rule_constants table -80
CC8 table.rs+edicts.rs ProvincePolicy, medicine->population -120
CC9 setup JR4b ajustement garnisons -> data pré-calculée -200 [MÉCANIQUE]
CC10 planning_scope registre global -> PlanCache explicite -150 [haut]
CC11 probes sg3_assault_probe, jr4b_budget_probe, ai/examples econ_probe/r3_small_scan/start_economy_probe -380
CC12 code mort 15 fns -130
CC13 missions 3 kinds data-driven, ransom acomptes+ordres chevalerie suppr, heresy->population -800..-1200 [MÉCANIQUE haut]
CC14 fonctions géantes découpées; ai_minimal::plan_turn réduit -200
CC15 doc tags (620) + lib.rs regroupé en sous-dossiers -300 (dernier)
CC16 perf weight_share précalc, resolve_decay BTreeSet, controlled_provinces itérateur
## BT battle + naval
BT1 suppr bataille navale 3D (naval_scene etc, shaders naval, assets, nv tests, bridge naval_sim get_ships...) -2800..-3000 [MÉCANIQUE; feedback no-naval-3d!] PREMIER
BT2 battle_scene harnais dev (bench, 45 flags, _stage_*) -> BattleDevHarness -700..-800
BT3 figurine rigide supprimée (battle_meshes, battle_soldier.gdshader réduit aux engins, 27 glb) -1100
BT4 34 flags A/B --no-* + replis GD + 64 has_method -400
BT5 buffers fine_near/hide/loosen en Rust (perf) -250 (après BT4)
BT6 manifeste skinné cuit hors ligne, suppr NT12/NT13 mocap trials, modes fa_anim -450 + Mo d'assets (après BT3)
BT7 battle_terrain height/river en Rust via BattleSim, split Terrain/Mesh/Decor -300
BT8 plan_deployment en Rust -200
BT9 constantes -> data, KINDS/TRIM centralisés, 12 morts -100
BT10 MultiMeshKit/ParticleKit -250
BT11 suppr duels/birds/cloud_shadows/queue_tip/secondary_motion -800 [MÉCANIQUE cosmétique]
BT12 battle_scene structure replay/banners/audio, perf _refresh_view
## SH shaders
SH1 noise_common.gdshaderinc (25 copies hash/vnoise/fbm, garder variantes A et C) -350..-450 [visuel léger] — terrain.gdshader GELÉ FL
SH2 campaign_map_data.gdshaderinc (province_at/faction_of/height lod x3) + river_common -120..-160 — terrain GELÉ FL
SH3 battle_bone_anim.gdshaderinc + palette include -80
SH4=BT3 battle_soldier.gdshader rigide (vérifier engins siège)
SH5 interrupteurs uniform bool ga1/ga2/da6/ga4/sr2 -> voie par défaut -200..-300 (avec BT4)
SH6 perf terrain sea_nearby cache (GELÉ FL)
SH7 uniformes/fonctions morts -40
SH8 perf battle_ground detail_height 9 lectures -> dFdx
SH9 fx_flipbook + nz_luma -50 (après SH1)
SH10 uniformes jamais posés -> const (faux positifs) 
## AD ai/data-model/relief-lod/vegetation
AD1 defaults Rust dupliquant data/rules JSON -> Default via include_str JSON embarqué -1200..-1500 FONDATION (2 vagues)
AD2 load.rs opt_json helper/table -170 (avec AD1)
AD3 ai plan_armies/plan_economy découpés ArmyTurn -250..-400 (turn_digest identique)
AD4 constantes IA -> data/ai/campaign.json, sels -> module salts (après AD1)
AD5 ai experiment.rs + ai_duel_probe suppr -176
AD6 ai/examples 26 sondes -> garder turn_perf/turn_digest/turn_hotspot/playthrough/pb1_core -6500 (century/balance/ia_quality probes inclus)
AD7 vegetation V4 legacy (species None) + repli GD vegetation_tile_job -90 Rust -400 GD (vegetation.gd GELÉ FL -> tile_job seulement?)
AD8 vegetation dépend de relief-lod page_bilinear; relief_quadtree.gd repli GD -500; has_page mort
AD9=geom/util: distance x6, is_rebels x5, splitmix x3, hash01 x3, lerp/smoothstep, segment_distance, dice::roll -> data-model/src/util.rs FONDATION (bit à bit)
AD10 Cargo.toml profils redondants -25
AD11 mort count_by_kind, has_page, fallback_city/slugify, fallback_edges -300
AD12 perf ai grid near spatial, threat_by_province, ids Arc<str>/interner (piste)
AD13 ai doc/découpe -150
AD14 string_enum! macro, CharacterRef deserialize générique -200
## MB map terrain/nature
MB1=PF-02 pyramide obligatoire: suppr pipeline tuilé terrain_builder, FineTerrainJob, png16, 672 tuiles far -650..-800 (terrain_builder non gelé? vérifier FL)
MB2 relief_quadtree sélection GD suppr -300 (=AD8b)
MB3=PF-03 semis GD vegetation_tile_job suppr -450 (vegetation.gd GELÉ)
MB4 StreamedTileLayer commun (5 couches) -400 (après MB3/MB5) [vegetation.gd GELÉ]
MB5 flags A/B végétation GA3/FC -250
MB6 relief_cache_notice/status -> push_warning -300
MB7 parchemin décor marin animé suppr + redraw 100ms -> à la demande -220 [MÉCANIQUE visuelle]
MB8=MC2 DataFile.load_cached
MB9 life_effects points typés -170 (life reground GELÉ)
MB10 war_scars voie events suppr -90 ; MB11 vegetation_mask repli sans splat -90 ; MB13 morts -60
MB12 Rust: mask sample / ReliefFloor / clutter+rocks seeding (perf)
MB14 étude fusion rendus rivière/route
## UI
UI1 thousands x8 -> Money -80 ; UI9 morts -60 (premier)
UI2 UiBuild kit (vbox/hbox/margin/button/label) a/b/c -400..-700
UI3 ListMenu base 4 menus -200 ; UI4 encyclopedia MECHANICS -> data/ui/encyclopedia.json + fiche commune -300
UI5 tutorial_steps textes -> data/tutorial -150 ; UI6 TooltipHost unique + rich_tooltip découpé -200
UI7 diplomacy_panel sous-vues ; UI8 ProvinceSection base 7 sections -250 ; UI10 doc (dernier) ; UI11 tscn vs code (décision) ; UI12 menu_backdrop_3d/living_portrait suppr? [MÉCANIQUE visuelle, demander? non: mandat libre mais prudence]
## MA map villes/UI
MA1 suppr style villes `real` (9 fichiers town_*/landmark_city*, town_builder -> building_kit) -4700 GD -2100 tests [ATTENTION: main dirty touche landmarks_v2 + towns_1340 -> quelqu'un y travaille? vérifier avant]
MA2 suppr landmarks v2 (+ tools paris_v2_author etc) -3100 (après MA1, même réserve)
MA3 campaign_map harnais stages -> MapStages -880 [GELÉ FL]
MA4 settlement_layer découpe labels/hamlets/picking -250 [declutter GELÉ FL]
MA5 declutter Rust [GELÉ FL, annoncer]
MA6 map_ui découpe JournalView/TopBarFit + JOURNAL_STYLES -120
MA7 suppr PerfProbe + map_bench [GELÉ FL]
MA8 shims has_method ancien cœur campaign_map/settlement_* -130 (après PF-05 mock) [campaign_map GELÉ]
MA9 règles visuelles villes -> core settlement_visual_state, governable/commandable -> core
MA10=MC2 DataFiles
MA11 outbuilding_layer découpe [GELÉ FL]
MA12 settlement_controller stage_screenshot/morts -90
## TL tools
TL1 proto_moteur, experiments, paris_v2_author, disk_cleanup -19.4k ; TL2 9 modules one-shot -1.3k ; TL3 map_markers + *_raw (après TL6) -26 Mo ; TL4 GA3/HB blender -6.2k ; TL9 world_frame reframe -300  => vague 1 (tl-del)
TL8 test_schemas paramétré -2k => vague 1 (tl-schemas)
TL5 mocap/FA3/AN1b/FG blender -4.8k ; TL6 pipeline payant OpenRouter/fal/TTS -8k (+pydantic) ; TL7 audio/UI art gen -5.9k [ADR] ; TL10 cli split (après) ; TL11=BT3 ; TL12 battle_skinned vs fine doublons ; TL13 kit_geometry ; TL14 paths.py/imaging.py (dernier)

## MS — scripts divers (audio, codex, debug, dev, sim, visual, assets)
- MS1 suppression CampaignSimMock + replis SimFacade (`is_real`, `engine`) + gardes `has_method` de campaign_map (gelé FL) → = lot vague 1 `mock` ; reste campaign_map après FL.
- MS2 suppression MapBench/PerfProbe/`--bench-map`/gen_synthetic_map (-1100 l) → touche map_bench.gd et campaign_map.gd (gelés FL) : après FL.
- MS3 helper unique `data_dir()` + `load_json` → = lot vague 1 `jsondata` ; étendre ensuite aux fichiers battle/ui restants.
- MS4 codex_bubbles : (a) extraire BubbleLayout / fils d'Ariane, (b) `set_process` seulement si bulles. (c) suppression repliement/verrou Alt REFUSÉE (IB approuvé par le joueur).
- MS5 VoicePool commun (UiSounds/BattleAudio/AudioDirector), volumes dans Settings seul, accesseurs test-only, EVENT_SFX en JSON (-200 l).
- MS6 render_quality PRESETS → data/fx/render_quality.json + schéma (-130 l).
- MS7 assets sans référence : third_party/{vegetation,characters,animals}, quaternius_medieval_village (54 Mo), textures/buildings brutes (26 Mo, entrées build_textures.py) : vérifier SOURCE.md et tools/blender* avant.
- MS8 LUT d'étalonnage (atmosphere_library.grade_lut) : port Rust ou précuisson.
- MS9 release_journey.gd : retirer options A/B (`--map-ab`, `--ab-configs`, `--uncapped`) (-250 l).
- MS10 passe commentaires visual/ + audio/, constantes battle_audio → sound_bank.json (-150 l).
- Morts : advisor.gd BUBBLE_WIDTH/BUBBLE_BOTTOM ; ~20 accesseurs test-only (liste dans rapport MS).

## DT — data/ et schémas
- DT1 test de conformité paramétré `data → schema` (-2500 l) → = lot vague 1 `tlschemas` ; y ajouter les 4 schémas orphelins (DT8).
- DT2 relief_shade_[0-3].png (131 Mo) : repli quand le BC5 manque → supprimer repli (relief_landcover.gd, export_data.py) et dé-suivre.
- DT3 fusion landmarks v1→v2 (-2500 l GDScript) : change l'aspect des maquettes (GC approuvé) et main a des modifs non commitées sur landmarks_v2 → REPORTÉ (pas dans ce chantier sans vérif).
- DT4 chargeur Rust GDExtension pour rivières/routes/côte/provinces (22 Mo JSON parsés en GDScript, provinces.geojson parsé 3×) : gros gain de chargement ; touche map/ (après FL pour la partie campaign_map).
- DT5 defs communes (`color_hex`, `rgb3`, `snake_id`, refs ids) dans common.schema.json (-500 l JSON) ; après DT1.
- DT6 sortir data/map/height de git → REFUSÉ (le clone doit rester jouable hors ligne).
- DT7 codex : bundle généré au lieu de 477 fichiers lus au démarrage.
- DT8 schémas town_footprint, forced_sea_edges ; brancher les 4 schémas orphelins.
- RL1 relief-lod ReliefSelector::clear_pages/clear_slots sans appelant ; wound_recovery / visible_armies seulement testés (deadfuncs)
- NV2 moteur naval temps réel sim-battle (`naval/{sim,ship,scenario,setup,outcome}.rs`), `data/naval/scenarios` + schéma : vérifier d'abord si `auto.rs` / `auto_resolve_naval_battle` en dépend (la résolution auto doit rester) ; mettre à jour ADR 0028 (statut remplacé par 0187).
- GT8 hooks de capture orphelins (après gtshots) : battle_scene `_stage_*`, flow_controller `--flow-stage`, `stage_screenshot()` des contrôleurs, tutorial capture_mode, start_menu `--menu-stage` (vérifier po_shot/readme_gallery), faction_select.stage_map, diplomacy_panel.stage_example, war_scars engins imposés ; campaign_map `--stage=` et life_effects A/B après FL. Commentaires citant des tests supprimés.
- PRE échec préexistant (avant SC) : `tools/tests/test_relief_update.py::test_manifest_bake_versions_follow_the_code` (tier3 attendu 5, trouvé 6) — à corriger (manifeste ou code).
- TLR descriptions de schémas art_ga3_decor / art_rock_outcrops et commentaire siege_engines_fx.gd:89 citent des outils supprimés (tldel).
- SCH lecteurs Python sans registre $ref (geo/colormap.py, geo/biomes.py, ground_materials.py…) → passer par codex.schema_validator puis factoriser 12 schémas restants

- SIMSPLIT (fait) : `ai/plan_field.rs` reste à 897 lignes, découpage plus fin possible.
- AUDIO (fait) : `AudioDirector.play_sfx` reste en round-robin, hors VoicePool (MS5a partiel).
- HOOKS (fait) : après levée du gel FL, retirer le tronc `--screenshot=`/`--stage=` de campaign_map.gd et tous les `stage_screenshot()`/`stage_example` avec vn_ui_720_b/c_test et smoke.gd:1150.
- BATTLEDEV (fait, −1330) : suite après levée de l'interdit sur battle_vegetation.gd : `--no-fa-grass`, `--bench-ab=fa-grass`, `set_fa_view`, param `da6_on`, puis `da6`/`site_render` de battle_terrain.gd, `fa_on` du shader herbe, `RenderQuality.override_level`/`upscale_override`. Hors battle : `--no-ga3`, `--no-sr5` (+ tests ga3_l1, ga3_l5, sr5). cb0_input_equivalence_test : golden à vérifier.
- PRE2 : cb0_input_equivalence_test échoue aussi sur main (12 commandes vs 14) : golden périmé, antérieur à SC.
- UIKIT (fait, −480) : `RichTooltip.thousands` reste pour encyclopedia.gd (interdit) ; `UiBuild.spacer` non utilisé encore ; UI1 a basculé sur Money.NBSP et « − » U+2212.
