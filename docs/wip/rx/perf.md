# RX perf — performance et robustesse (2026-10-09)

Mesures faites sous charge machine très forte (load 48-110, autres sessions) : durées indicatives.
Journaux : `/private/tmp/claude-501/rx-shots/perf/` (smoke.log, runall.log, tests/*.log, bench.log).

## Verdict
- Forces : `smoke.gd` passe ; 194 tests Godot sur 204 passent ; carte à 47 i/s (p50 19,7 ms, p99 34,9 ms, 0 pic > 50 ms) à d = 150 malgré la charge ; aucun `SHADER ERROR` dans les 204 journaux ; sauvegarde versionnée avec refus explicite et message.
- Faiblesses : 10 tests en échec (dont 1 blocage de 616 s et 1 erreur d'analyse GDScript), fuites systématiques à la sortie (66 journaux sur 204), bruit d'erreurs `push_error` dans des tests verts, sauvegardes d'anciennes versions rejetées sans migration, suite complète très longue (≈ 55 min sous charge).

## Constats

### [majeur] bug — `dn_campaign_models_test` se bloque (code 137, 616 s)
**Constat** : le test reste à 0 % CPU après le log `TownMaquetteLayer`, tué par le garde-fou 600 s. Même symptôme que `settlements_render_test` (FL). **Preuve** : `runall.log` `FAIL dn_campaign_models_test (code 137, 616 s)` ; dernier log 20:18, process à 0,0 % CPU. **Correction** : chercher l'attente (await signal jamais émis / variable de condition) dans le test et l'ajouter une échéance (`create_timer`) ; idem settlements_render_test. **Coût** : S-M.

### [majeur] bug — `tb1_seasons_test` ne compile plus
**Constat** : `AtmosphereLibrary._prepare()` / `_grade()` introuvables (fonctions statiques supprimées ou déplacées lors de SC) ; le fichier `game/scripts/map/atmosphere_library.gd` n'existe plus à ce chemin. Code de sortie non nul seulement grâce à la détection « Failed to load script ». **Preuve** : `tests/tb1_seasons_test.log` (`Static function "_prepare()" not found in base "AtmosphereLibrary"`), `game/tests/tb1_seasons_test.gd:95`. **Correction** : repointer le test vers la nouvelle API (ou le supprimer si couvert ailleurs). **Coût** : S.

### [majeur] bug — tests UI en échec sur seuils de mise en page
**Preuve** : `po_ui_test` C2 (bandeau d'armées chevauche SIDE_PANEL en 1280×720 et 1920×1080) et C3 (taille de police < 15 px) ; `p2b_ui_test` / `p2g_ui_test` C3 « au plus 4 tailles distinctes, 5 vues » (tailles 14/15/17/20/26) ; `rs_f_battle_test` « collapsed picker shrinks to its header » ; `da7d_overlap_test` (chevauchements d'étiquettes à Flandre 1100, Île-de-France 600/1100). Soit régression visuelle réelle (bandeau d'armées sur le panneau latéral, visible au joueur), soit seuils périmés. **Correction** : trancher test par test ; po_ui C2 d'abord (chevauchement d'UI). **Coût** : M.

### [majeur] bug — `zg5b_fine_geo_test` : pont non ancré
**Preuve** : « bridge not on its fine anchor: (0,0,0) », « not at water level », « not across the current » ; le test voit aussi `ReliefQuadtree: unreadable tile .../zg5b_test/pyra...` et `ReliefPyramid: ... tiles missing on disk` (cache `~/Library/Application Support/Godot/app_userdata/Cent Ans/` partiel, ou test dépendant de données absentes). **Correction** : distinguer cache corrompu/absent (le test doit alors se sauter explicitement, pas échouer) d'une vraie régression de placement de pont. **Coût** : S-M.

### [majeur] robustesse — sauvegardes anciennes rejetées sans migration
**Constat** : `STATE_VERSION = 9` ; tout `state_version` différent est refusé (`VersionMismatch` / version antérieure), message clair mais la partie est perdue à chaque changement de format. **Preuve** : `core/crates/sim-campaign/src/save.rs:43-57`. **Correction** : chaîne de migrations JSON v(n)→v(n+1) dans `save.rs` + un test avec une sauvegarde v8 figée dans `tests/fixtures`, avant la première version publique (ou ADR assumant la rupture). **Coût** : M.

### [mineur] robustesse — fuites systématiques à la sortie
**Constat** : 66 journaux sur 204 finissent par « N ObjectDB instances were leaked at exit » ; 59 par « 4 resources still in use ». Smoke : 462 instances. Plus 7 « RID allocations ... DummyTexture leaked ». Surtout des nœuds jamais `free()` en fin de script de test (`quit()` sans nettoyage), mais cela masque une vraie fuite éventuelle. **Preuve** : `smoke.log` fin ; `tests/*.log`. **Correction** : `free()` de la racine de scène dans `smoke_base.gd` avant `quit`, puis mesurer l'écart avec `--verbose` pour isoler les fuites réelles du jeu. **Coût** : S-M.

### [mineur] bug — erreurs moteur répétées dans des tests verts
**Constat** : `Parameter "t" is null` (27 journaux, origine `ga3_vegetation.gd:83` `_load_mesh`, texture_2d_initialize sous rendu factice : texture chargée à partir d'un `Image` null) ; `Parameter "m"/"mem"/"material" is null` (23/6/4) ; `Attempting to use an uninitialized RID` (6) ; `Trying to assign Nil to PackedVector3Array` à `outbuilding_layer.gd:1530` (`with_pieces`, vt3_trees_test) ; `Invalid access to property 'markers' on SettlementController` à `c5_settlements_ui_test.gd:119` et `m4_free_movement_ui_test.gd:64` (propriété renommée ; le test affiche pourtant OK : assertion non exécutée). **Correction** : garde `if image == null` dans `_load_mesh` ; corriger `with_pieces` ; mettre à jour les deux tests (`markers`). **Coût** : S.

### [mineur] finition — smoke : erreurs `push_error` attendues mais bruyantes
**Constat** : `smoke.gd` (OK, 7 min 20 s sous charge) imprime `ERROR: TerrainBuilder: Relief indisponible : pyramide ... fixtures`, `CampaignSim: cannot load data from game/tests/fixtures`, `CampaignMap: campaign could not start`, `CampaignSim: method called before new_campaign` : chemins de repli testés volontairement mais indiscernables d'une vraie panne. **Preuve** : `smoke.log` lignes 5-30. **Correction** : message préfixé `[attendu]` ou `push_warning` quand `CENT_ANS_SMOKE` est défini, pour qu'un grep `^ERROR` détecte les régressions. **Coût** : S.

### [mineur] robustesse — dépendances de données absentes seulement signalées en WARNING
**Preuve** (smoke) : `ReliefLandcover: relief_shade BC5 copy missing (run cent-ans geo gpu-textures)`, `MapBirdFlocks: map_birds.json unreadable`, `HbGround: biomes.png ou ground_biome_mix.json absent, habillage désactivé`, `ReliefPyramid: level N, N of N tiles missing on disk` (6 tests). Dégradation douce correcte, mais pas de vérification globale au lancement. **Correction** : un contrôle `tools/check_assets.sh`/écran de diagnostic au premier lancement listant ce qui manque. **Coût** : S.

### [mineur] perf — tests UI très lents sous charge
**Preuve** : `runall.log` : `q7_end_turn_test` 176 s, `q8_pause_menu_test` 106 s, `rj_possession_test` 101 s, `po_ui_test` 100 s, `q6_ui_test` 92 s ; suite entière ≈ 55 min (204 tests, 8-10 s de chargement carte chacun). **Correction** : regrouper les tests carte dans un processus (comme smoke) ou marquer un sous-ensemble « rapide » dans `run_godot_tests.sh`. Le temps de fin de tour n'est pas isolé : `q7_end_turn_test` mesure surtout la charge. **Coût** : M.

### [mineur] perf — chargement carte ≈ 7,1-7,8 s
**Preuve** : `m4_free_movement_ui_test.log` `total_ms 7097` (decor_ms 3485, terrain 1194, masks 509) ; `bench.log` `startup_total_ms 7773`. Sous charge ; `decor_ms` est la moitié. **Correction** : profiler `decor` (sonde PerfProbe) pour déplacer des étapes sur des threads ou différer hors de l'écran de chargement. **Coût** : M.

### [mineur] perf — cuisson des maquettes de repères : 7 s cumulées
**Preuve** : `bench.log` `landmark_bakes 22`, `landmark_bake_ms_total 6986,9` (≈ 318 ms/cuisson) ; heureusement `landmark_bake_frame_ms_max 0,43` (hors fil principal). Aucun effet visible tant que c'est asynchrone ; à surveiller si le fil se met en concurrence avec le décodage (`decode_ms_max 76 ms`). **Coût** : S (surveillance).

### [mineur] perf — appels de dessin et primitives à d = 150
**Preuve** : bench d = 150 : `draw_calls_p50 1347`, `primitives_p50 8,26 M`, contre 865 / 3,06 M à FL0 (d 150). Bien que FL2 ait établi que les appels ne limitent pas l'image, la hausse (+55 % / ×2,7) après VT/HC/GC/RV/FA mérite une ablation. Mode `scale_3d 0,52`. **Correction** : relancer `a6_drawcalls_probe` sur machine calme et comparer à FL2. **Coût** : S.

### [mineur] test — `sz1_mountain_test` : « GDScript twin changed »
**Preuve** : `sz1_mountain_test.log`. Le jumeau GDScript d'une règle Rust (test de dérive) a divergé : vérifier dans quel sens (règle de jeu hors `core/` = contraire à CLAUDE.md si la logique vit réellement en GDScript). **Coût** : S.

### [mineur] test — `tf_far_layer_test` : seuil de temps (10,85 ms > 8,0)
Seuil dépendant de la charge machine (déjà noté en FL1 : 124 ms sous charge 231) ; test non déterministe. **Correction** : seuil relatif à une mesure de référence prise dans le même processus, ou compte d'opérations. **Coût** : S.

### [mineur] test — tests Rust `#[ignore]` : 6
`fk_map_scenes.rs:288` (165 s), `m3_grid_ai.rs:331`, 3 régénérateurs de fixtures, 1 sans motif `battle_pose_lerp.rs:304` (`#[ignore]` nu). **Correction** : ajouter la raison au dernier ; lancer les lents en CI nocturne. **Coût** : S.

## À ne pas changer
- Garde-fous headless (watchdog erreurs/temps) : ont fonctionné (blocage tué à 616 s, pas de journal géant).
- Détection « Failed to load script » dans `run_godot_tests.sh` (Godot sort en 0 sinon).
- Refus explicite des sauvegardes de mauvaise version (message en français) : bon comportement par défaut.
- Banc `map_bench.gd` : rapport riche et reproductible ; pas de pic > 50 ms ; chargement des pages de relief et cuisson hors fil principal.
- Dégradation douce (WARNING) quand une donnée de relief/biome manque : le jeu démarre.
