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
- [ ] HC5 (optionnel, change les règles) : nouveaux massifs nommés et zones humides historiques,
      recuisson complète, contrôle d'équilibrage. À décider après HC3.
- [ ] HC6 : tests, `docs/godot-map.md`, mémoire, fusion dans main (après GC).

## Budget de captures
6 lectures d'image pour la session principale (travail visuel), en planches 2×2 de 640 px ;
3 lectures au plus par agent. Constat : 2 lues (captures existantes de TB).

## Prochaine étape
Attendre HC1 et HC2, puis HC3.
