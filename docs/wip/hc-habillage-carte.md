# HC — habillage de la carte de campagne (forêts, lacs, champs)

Worktree `../gp-hc`, branche `feat/hc` = main + `feat/gc` (fusionné le 02/10 à 84b92b94c pour juger
les arbres face aux maquettes ; HC ne peut donc entrer dans main qu'après GC). Pyramide en lien
symbolique vers main, dylib copiée de main (aucun changement Rust). ADR 0161.

## Mandat (joueur, 02/10)
« Je veux plus de forêts, lacs, champs etc. pour habiller la carte de campagne. »

## Constat (captures `~/.cache/cent_ans/tb/ours/avant-sol-90.png` et `-400.png`, Paris)
- Forêts : grandes taches vert sombre **plates**. Cause : arbres 1:1 (`tree_ratio` 0,018) coupés
  au-delà de rig 30 (`map_prop_scale.tres`, `vegetation.gd:267-271,540`) ; canopée = bruit de
  teinte qui s'éteint au dézoom ; carte de couleur dominante dès rig 300. Les données sont déjà
  boisées (39,7 % des terres dans `splat.png` B).
- Bosquets, ripisylves, vergers, arbres de haie : présents dans le semis par essences
  (`vegetation_tile_job.gd:448-505`, `data/art/tree_species.json`) mais même portée de 30.
- Lacs : 762 (`lakes.json`, seuil 30 px ≈ 15 km²), dessinés jusqu'à rig 1200 mais teinte
  `lake_color (0.035, 0.10, 0.115)` proche des forêts. Étangs (`wetlands.png` G, `rl_pond_sd`,
  `relief_landcover.gdshaderinc:78`) réduits à une couverture moyenne au dézoom (`:141`).
- Champs : parcellaire lisible jusqu'à rig ≈ 325 ; échelle en cours de réglage par GC5.
- Règles : `splat.png` B et `wetlands.png` alimentent `navgrid.png` (ADR 0045) et le couvert
  (`core/crates/data-model/src/cover.rs`). Recuisson : `geo landcover` (70 s) → `navgrid` (6 s) →
  `colormap` (2 min 20, pic 11,6 Go) → `horizon` (26 s).

## Coordination
- GC (`../gp-gc`, ADR 0158) : **GC5 garde les champs** (`../gp-gc-fields`, `field_scale` dans
  `terrain.gdshader`), hameaux et moulins. HC prend **arbres, forêts, lacs, étangs** (message
  envoyé aux sessions voisines le 02/10). HC ne touche pas au bloc parcellaire de
  `terrain.gdshader` (`field_at`, lignes ≈ 492-570).
- GC (session game-project-e9, 02/10) : `field_scale` 0,3 (enclos ≈ 1 km, `feat/gc-fields`
  89e181771), `hb_cell_scale` 1,2 ; maquettes : ville 11, bourg 6,5, château/abbaye 3,6, village
  3,2, Paris ≈ 22 unités. Décision HC : en style `generalised` les arbres de haie ne suivent plus
  la trame (enclos ≈ 1,4 px < un arbre) : arbres épars dont la densité suit le bocage. Transmis à HC1.
- SA (`../gp-sa`) : ne pas toucher `army_markers.gd`.
- TB (`../gp-tb`, session game-project-ef) : garde la **teinte de sol des forêts** (taches presque
  noires à rig 400-1100, corrigées par TB6 côté sol), brouillard, météo, côtes, brûlis dans
  `terrain.gdshader` ; TB ne touche ni arbres, ni lacs, ni étangs. Au-delà de la portée des arbres
  HC (≈ 700-900), seul le sol de TB compte.
- Checkout principal : fichiers `data/map/*` modifiés non commités par une autre session ; aucune
  recuisson de données depuis main.

## Lots
- [x] HC0 : constat, worktree, inventaires (rendu Godot, pipeline geo), ADR 0161.
- [ ] HC1 (agent, `../gp-hc1`, `feat/hc1`) : arbres généralisés — taille monde constante grossie,
      portée jusqu'à la vue stratégique, `map.tree_style`, bosquets / ripisylves / vergers / haies
      visibles, exclusions élargies, paliers, test `hc_forest_test.gd`, planche `hc_shots.gd`.
- [ ] HC2 (agent, `../gp-hc2`, `feat/hc2`) : eaux — lacs éclaircis, seuil de surface abaissé
      (`geo lakes`), étangs et mares généralisés lisibles au dézoom, test et planche.
- [ ] HC3 : relecture visuelle par la session principale (planche commune Paris / Orléans /
      Sologne / Dombes / Léman, rig 60 / 150 / 300), réglages, fusion de HC1 + HC2 dans `feat/hc`.
- [ ] HC4 (après fusion de GC dans main) : variété des champs (vignes, vergers, landes lisibles à
      hauteur de jeu), sans toucher l'échelle de GC5.
- [ ] HC5 (agent, `../gp-hc5`, `feat/hc5`, **accepté par le joueur le 02/10**) : nouveaux massifs
      nommés, landes et zones humides historiques sur toute la carte, sourcés ; recuisson complète,
      écart de règles mesuré. Note `docs/wip/hc5-massifs-zones-humides.md`. `tools/geo/raw` en lien
      symbolique vers main (cache KK10 indispensable).
- [ ] HC6 : tests, `docs/godot-map.md`, mémoire, fusion dans main (après GC).

## Budget de captures
6 lectures d'image pour la session principale (travail visuel), en planches 2×2 de 640 px ;
3 lectures au plus par agent. Constat : 2 lues (captures existantes de TB).

## État à la pause (02/10, demandée par le joueur)
- **HC1 livré** sur `feat/hc1` (4be0a2587, 7 commits), pas encore fusionné dans `feat/hc` ni relu
  par la session principale. Réglages `generalised_*` dans `map_prop_scale.tres` : hauteur 0,8
  unité, pas 0,9 px, portée rig 900, ombres < rig 70, imposteurs à toutes distances, haies hors
  trame. Tests verts (`hc_forest_test`, `gc_maquettes_test`, smoke, 6 anciens tests épinglés sur
  `real`). Planche : scratchpad de la session `shots/hc_board_3.jpg` (non lue par la session
  principale).
  Points ouverts HC1 : surcoût mesuré sur machine chargée (+5 ms à rig 300, cible < 2 ms non
  établie) → banc sur machine calme ; taille non strictement constante au-delà de rig 150
  (éclaircie 150/d, restants grossis jusqu'à × 1,69 : `generalised_far_density` = 1 pour la rendre
  constante) ; semis d'une tuile 300-800 ms en fond (remplissage progressif) ; premier plan de
  Paris peu arboré ; validation pytest du schéma `campaign_map_ui` non confirmée.
- **HC5 en pause, état propre** sur `feat/hc5` (79f517bfa) : +91 massifs/landes (55 → 146),
  +45 zones humides (32 → 77), cuissons faites et cohérentes (landcover, navgrid, colormap,
  horizon). Écart de règles : forêt 39,8 → 40,3 % des terres, marais + étangs 0,08 → 0,24 %,
  franchissable 92,3 % inchangé, aucun lieu isolé, liaison max +28 % (Buda–Visegrád).
  Reste : pytest `test_colormap.py` / `test_horizon.py`, import + smoke Godot,
  `cargo test -p sim-campaign -p ai`, supprimer `core/target-hc5`.
  **Défaut à arbitrer** : `landcover.py` laisse vides des massifs nommés tombés dans un creux du
  bruit (Fontainebleau 0 %, Sherwood 0 %, Yveline 5 %, Clèves 5 %, Maures 17 %) : d'où un gain de
  forêt de seulement +0,5 point. Avis : le corriger (un massif nommé doit être boisé à sa
  densité), en remesurant l'écart de règles ; `anchors-fine` non relancé (fichier d'une autre
  session) ; lacs historiques (Fucin, Copaïs, Amouq) pour un lot `lakes.json`.
- **HC2 en pause, tout commité** sur `feat/hc2` (8272840ad) : lacs 762 → 1231 (`min_area_px` 8),
  `lake_color` (0.07, 0.23, 0.38) + reflet de ciel, berge claire ; étangs et mares en cellules
  doublées par paliers d'empreinte (`rl_pond_*` / `rl_pool_*`, Dombes : 25 nappes à rig 150, 6 à
  rig 300). Passés sur l'état final : `hc_water_test.gd` (fenêtre), `test_lakes.py`, ruff.
  **À repasser** : `smoke.gd`, `ss_lakes_test.gd`, `hc_water_test.gd` headless,
  `tb1_seasons_test.gd` (jamais lancé). Planches non lues :
  `~/.cache/cent_ans/hc/hc2_water.jpg` et `hc2_water_winter.jpg` (hiver : part d'eau moitié
  moindre, cause non confirmée). Ouverts : cadrage de `hc_water_view.gd` (balayage `--views`
  incohérent une fois), mares des Fens discrètes, taille des étangs en px d'un écran de 1080
  (`rl_water_ref_height`). Lacs historiques absents pour un lot `lakes.json` : Grand-Lieu, Loch
  Ness, Berre, Windermere, Paladru, Aiguebelette, Joux, Nantua, Saint-Point, Léon/Soustons,
  Haarlemmermeer, Whittlesey Mere (+ Fucin, Copaïs, Amouq de HC5).

## HC3 — reprise du 02/10 au soir (en cours)
- main (GC comprise, 425788334) + `feat/hc1` + `feat/hc2` fusionnés dans `feat/hc` (71aa197f1) ;
  conflit `campaign_map.json` + schéma résolu (clés GC et `tree_style` gardées).
- Tests sur la branche fusionnée, tous verts : import, smoke, `hc_forest_test`, `hc_water_test`,
  `gc_maquettes_test`, `ss_lakes_test`, `tb1_seasons_test`, pytest schémas / lacs (19).
- Planches `~/.cache/cent_ans/hc/hc3/trees_a.jpg` et `water_a.jpg` (lues : 4 lectures sur 10) :
  forêts en volume à rig 25-300 (bien) ; abords de Paris nus à rig 60-150 (massifs nommés vides :
  HC5) ; bosquets et arbres épars trop rares à mi-distance (à régler : `generalised_grove_*`,
  `generalised_isolated_gain`) ; Léman et étangs de Dombes / Sologne lisibles ; Fens = masse gris
  sombre → `rl_reed_color` et `rl_marsh_water` éclaircis (à recontrôler).
- HC5 relancé (joueur : « oui corrige les massifs nommés vides ») : correction de l'allocation dans
  `landcover.py`, recuisson, mesures, tests restants.
- Banc HC1 : machine encore chargée (charge 37) ; à refaire.

## Prochaine étape
Au retour de HC5 : fusion de `feat/hc5`, réglage des bosquets, planche de contrôle (Paris 60 / 150,
bocage, Fens), banc, puis HC6 (docs, fusion dans main).

## (ancienne) Prochaine étape — pause
0. GC est dans main (425788334, non poussée) : fusionner main dans `feat/hc` puis dans les trois
   branches de lot (conflit attendu : `data/ui/campaign_map.json` + schéma, clés `town_scale`,
   `town_style`, `camera_floor_distance` contre `tree_style`). Maquettes finales : ville 14, bourg 8,
   château 4,6, abbaye 4,4, village 4, Paris ≈ 28 unités ; plancher de caméra 20 ; `field_scale` 0,3.
   Reste à GC : GC4 (largeurs des fleuves et routes), fichiers que HC ne touche pas. HC peut
   désormais entrer dans main sans attendre.
À la reprise : faire trancher le défaut des massifs vides (HC5), repasser les tests restants de
HC2 et HC5, banc HC1 sur machine calme, puis HC3
(fusion de `feat/hc1`, `feat/hc2`, `feat/hc5` dans `feat/hc`, planche commune, réglages).
