# ZG7c — recette visuelle aux 3 paliers et clôture du chantier ZG (ADR 0036)

Worktree d'agent `worktree-agent-a980562fd6a6cb636` (depuis `main` 553a345d). Liens symboliques non
versionnés : `data/map/pyramid`, `tools/geo/raw` → dépôt principal. Dylib :
`CARGO_TARGET_DIR=/Users/jean_hubert/dev/game_project/core/target cargo build -p godot-bridge` puis copie
dans `game/bin/libcent_ans.debug.dylib`. Visuel et données seulement, rien dans le jeu de `core/`.

## Plan
1. `ReliefPyramid` robuste à une pyramide trouée (repli par tuile), test avec pyramide trouée.
2. Recuisson complète `geo detail-dem` (plancher v4 sur les 34 zones), `detail-check`,
   `relief-all --check` ; écriture atomique ; fonds de vallée E1-E4 (plancher 0,5 m) : à évaluer.
3. Tamise fine à −7,8 m : corriger `hydro_fine` (Londres ~0-2 m).
4. Banc : attribution de `process_ms` dans `map_bench`.
5. Recette visuelle aux 3 paliers (12 lieux), captures `docs/img/zg7c/`, défauts.
6. Clôture : `docs/godot-map.md` (vue d'ensemble ZG), addendum final ADR 0036.

## État
- [x] 1. `ReliefPyramid._drop_missing_tiles` : un listage de dossier par étage, seules les tuiles
  absentes sont retirées (avant : tout l'étage si sa 1re tuile manquait) ; repli par tuile sur
  l'ancêtre le plus fin présent (`finest_ancestor`) ; tuile corrompue écartée à l'exécution
  (`mark_broken`, déjà là). Test `tests/zg7c_partial_cache_test.gd` (trous E1/E2, E3 sous un trou
  E2, tuile corrompue, reliquat `.part.png`) : quadtree stable à d = 40, 10, 4.
- [x] 2. `geo detail-dem` complet : 30/31 grappes E5, 31/32 E6, 32/33 E7 recuites (Londres déjà à
  jour), 190 s ; 281 tuiles E4 et 300 E5 parentes recalées ; manifeste inchangé (mêmes tuiles).
  Écriture déjà atomique (`.part.png` puis `replace`, idem tuiles parentes et tuiles CAFV).
  `detail-check` : p95 quasi identiques à ZG3b (22 → 4 m, signal indicatif, pas un critère) ;
  médianes relevées dans les vallées (Castillon 0 → 4,5 m, Château-Gaillard 1,1 → 4,5, Harfleur
  1,1 → 4,4) : écart entre le plancher v4 (≤ 5 m) et E4 plaqué à 0,5 m, attendu.
  `relief-all --check` : complet, 2,77 Go.
  Fonds de vallée E1-E4 à 0,5 m : **non corrigés** (recuisson E1-E4 = pyramide entière, plusieurs
  heures, et E0 `heightmap_render.png` porte le même rehaussement : continuité E0/E1 à reprendre
  ensemble) → suite S2.
- [x] 3. Tamise : `hydro_fine.water_level` borne le fond à 0 m (`MIN_WATER_LEVEL_M`) avant
  l'ajustement monotone ; `hydro-fine` puis `anchors-fine` relancés. Londres −7,76 → 0,07 m ;
  Bordeaux (bac) −15,8 → 0 m ; Avignon/Arles −0,97 → 0,45 m ; 28 passages modifiés.
  Recette : la Loire à Orléans passait **10 m sous le relief E7** (eau 71 m, berges 82-86 m) : les
  recalages des fleuves en cache dataient d'avant le correctif ZG3b. `SNAP_VERSION` 4 et clé de
  cache incluant `detail_dem.BAKE_VERSION` (nouveau recalage après toute recuisson) ;
  `hydro-fine` complet puis `anchors-fine` relancés.
- [x] 4. `map_bench` : `process_ms` = début de l'itération (nœud `FrameStart`, priorité minimale,
  physique comprise) → banc (priorité maximale). Descente : process p50 5,2 ms, p99 75 ms ; 131 pics
  > 50 ms sur 131 dominés par les scripts (médiane 59 ms) : contrairement à ce que concluait ZG7a
  avec `TIME_PROCESS`, les pics sont côté scripts (hydro-fine tournait en parallèle : relatif).
- [x] 5. Recette (script `tests/zg7c_recette_shots.gd`, `--map-weather=clear`, brouillard de guerre
  coupé) : 12 lieux × 3 paliers + parchemin, filtres, avis de cache. Défauts ci-dessous.
- [ ] 6. Docs de clôture.

## Recette : défauts corrigés dans le lot
| # | Défaut | Correctif | Capture |
|---|---|---|---|
| C1 | Montagnes en aiguilles et murs aux paliers vallée / site (Alpes, puys, Pyrénées, Snowdonia) : le gain ZG8 s'applique à `h − fond` jusqu'à 2 000 m | `ReliefFloor` relève le fond à `sommets voisins − local_relief_cap_m` (350 m, `relief_exaggeration.tres`) : collines et falaises inchangées, montagnes +≤ 350 × g m ; fond en 0,27 s au lieu de 0,22 | `alpes_vallee_avant/alpes_vallee`, `massif_central_vallee_avant/massif_central_vallee` |
| C2 | Tamise à −7,8 m, Garonne à −15,8 m (bathymétrie dans le niveau d'eau) | `hydro_fine.water_level` (≥ 0 m) | `londres_vallee`, `londres_site` |
| C3 | Loire sous le relief à Orléans (recalages en cache périmés) | `SNAP_VERSION` 4, clé liée à `BAKE_VERSION` | `orleans_vallee` (avant) |
| C4 | Avis « relief incomplet » : commande grisée illisible (champ non modifiable) | encre normale du champ | `avis_cache_absent` |
| C5 | Cache partiel : étage entier ignoré si sa 1re tuile manque | repli par tuile | test `zg7c_partial_cache_test` |

## Recette : défauts laissés (suites)
| # | Défaut | Piste | Capture |
|---|---|---|---|
| S1 | Haute montagne au palier vallée : l'exagération ZG4 seule (×3,4 à d = 6) fait des murs qui remplissent la vue (Pyrénées, Galles) ; caméra au fond de canyons | exagération fonction aussi de l'amplitude locale du relief (ou plafond de hauteur affichée par distance) ; touche caméra ZG4 et bornes du quadtree | `pyrenees_vallee`, `galles_vallee` |
| S2 | Fonds de vallée E1-E4 plaqués à 0,5 m (Seine 0,5-3 m de Paris à Rouen, Loire à 16 m à Amboise au lieu de ~55 m) ; marches sombres le long des coteaux de la Loire | recuisson E0-E4 avec le plancher monotone de ZG7a (plusieurs heures, continuité E0/E1) | `val_de_loire_site` |
| S3 | Villes emblématiques au palier site : maquette à la loupe sur relief 1:1 (Rouen : falaise au milieu de la ville, plan d'eau vertical) ; au palier vallée, Orléans n'est qu'un disque d'emprise | lot VH4 (1:1 géoréférencé) | `rouen_seine_site`, `orleans_vallee` |
| S4 | Objets à l'échelle de la carte au palier vallée : moulins, hameaux, fumées de colonies (colonnes blanches), arbres géants près de la caméra | étendre `campaign_prop_scale` / le masquage du palier site aux paliers intermédiaires | `crecy_vallee`, `paris_vallee`, `val_de_loire_vallee` |
| S5 | Ville ordinaire (Amiens) au palier vallée : disque d'emprise brun avant les maisons | seuil d'activation de la couche ZG6 (poids ≥ 0,5) à abaisser, ou emprise moins visible | `amiens_vallee` |
| S6 | Pics > 50 ms dominés par les scripts (banc corrigé) : sélection / application du quadtree, recalages | profilage script par script (`--bench-listeners`, minuteries `qt_step_ms_max`) | — |
| S7 | Pluie en bâtonnets blancs au palier site (échelle de particules) | taille minimale des gouttes à revoir | — |

Notes : `--pyramid-dir=` coupe l'avis par conception (essais) ; avec `CENT_ANS_RELIEF_DIR` vers un
dossier vide, l'avis s'affiche (MISSING, 9 couches) et la caméra s'arrête vers 7 unités ; avec
`--pyramid-dir=<vide>`, repli E0 sans quadtree, sans erreur. Parchemin et filtres MF1 (richesse,
ravitaillement, politique) : corrects aux trois paliers. Le « sol beige » de Londres et du pays de
Galles dans les premières captures était le brouillard de guerre (camp France), pas le relief.

## Prochaine étape
Vérifier la Loire à Orléans après le nouveau recalage (journal scratchpad `zg7c_hydro2.log`),
`anchors-fine`, docs de clôture (godot-map « Vue d'ensemble ZG », addendum ADR 0036), fusion de
`main`, tests.
