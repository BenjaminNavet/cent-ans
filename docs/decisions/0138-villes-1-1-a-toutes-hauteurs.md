# ADR 0138 — Villes à l'échelle 1:1 à toutes les hauteurs de la vue 3D

Date : 2026-09-30. Statut : acceptée (demande du joueur du 30/09, bancs du lot VT-I). Chantier VT
(« vrai territoire »), suivi `docs/wip/vt.md`.

## Contexte

Le joueur trouve dommage que la carte n'exploite pas le vrai territoire. Aujourd'hui, la ville 1:1
posée sur le relief (ZG6, VH4) n'apparaît qu'au palier vallée (distance du rig < 8). Au-dessus, la
vue 3D (< 1200, ADR 0124) montre des maquettes agrandies :
- maquettes L1/L2 des villes emblématiques sous loupe ×3,5 (ADR 0015, 0078) ;
- maquettes de colonies grossies par `MapPropScale` (SZ4, SZ4b, jusqu'à ×125).

Ces maquettes recouvrent la vraie vallée. Le joueur demande la ville « en très rapproché » à toutes
les hauteurs, pour toutes les villes : les 2 134 colonies de `towns_1340.json` et les 8 villes v2.

## Décision

- Plus aucune maquette agrandie sur la carte de campagne. `LandmarkModel` et `data/landmarks/`
  restent réservés au décor de bataille (`landmark_backdrop.gd`).
- Toutes les villes sont rendues à l'échelle 1:1 sur tout l'intervalle 0-1250, en trois niveaux :
  - **ZG6/VH4** (rig < `max_rig_distance` = 16) : plan complet, HLOD bloc/maison inchangé ;
  - **F1** (rig < ~300) : maillage lointain par ville (nappe de toits polaire sur les 32
    relèvements de `radii`, faubourgs, enceinte, monuments simplifiés), fusionné par tuile de
    128 unités ;
  - **F2** (au-delà) : polygone à 16 côtés, jupe, une flèche au plus, par tuile de 512 unités, sans
    ombre.
- Le lointain est généré à l'exécution dans des fils de travail depuis `towns_1340.json` et
  `landmarks_v2`, sans `TownPlan`. Ses hauteurs viennent d'une grille `ground_m` cuite hors ligne
  (`cent-ans geo towns`) et le shader les pose (`campaign_display_height`, ZG8). Une jupe de 30 m
  masque l'écart avec le relief.
- Passage au plan complet : un masque par ville enfonce le lointain d'une ville dès que sa ville
  1:1 est construite et proche (en miroir du fondu des blocs). La teinte des toits est partagée
  (`roofscape.gdshaderinc`).
- Repérage de loin : nom et écu (DV2). Le clic, l'anneau de sélection et les étiquettes se règlent
  sur l'emprise réelle, avec un rayon de clic minimal de 8 px.
- Hameaux et cheminées à taille réelle ; les incendies restent exagérés parce qu'ils signalent un
  événement. La végétation est exclue du finage autour des villes. La croissance visuelle CV1
  (`replace_models`) est abandonnée pour l'instant.

## Conséquences

- Remplace, pour la carte de campagne, la loupe de l'ADR 0015, les maquettes L1/L2 de l'ADR 0078
  en vue lointaine et la mise à l'échelle des colonies de SZ4/SZ4b.
- De haut, une ville est petite (Paris ≈ 3-4 px à d = 1200). C'est voulu.
- Coût estimé : ≈ 120 k triangles dessinés à d = 300, ≤ 35 appels F2, ≈ 30 Mo de mémoire vidéo,
  ≈ 0,3 s de génération au chargement. En échange, on ne crée plus les ~2 100 maquettes.
- Les arbres restent grossis (hors demande) ; l'exclusion par le finage évite qu'ils ensevelissent
  les villes. À juger sur capture.

## Mesures (lot VT-I, 30/09)

Banc `--bench-map --bench-pan-only` (M4 Pro, 1280 × 720, qualité Haute, 3 passes alternées avec
`main` 94cfa8103, machine partagée à une charge de 6-25), base → VT :
- appels de dessin p50 : d 1100 475 → 201 ; d 150 3 284 → 751 ; d 30 1 088 → 517 ;
- primitives p50 : 1,40 → 1,30 M ; 5,24 → 3,55 M ; 9,19 → 8,60 M ;
- i/s : égaux (plafond 60 de l'affichage aux passes 1-2 ; passe 3 : 145/144, 82/83, 55/54) ;
- chargement de la carte 6,4-7,4 s → 5,3-6,0 s ; lointain généré ensuite en tâche de fond en
  1,4-1,9 s réelles ; fil principal ≤ 2,2 ms par image (un pic isolé de 17 ms sur 9 passes).
- Mémoire : 2 141 villes, 757 tuiles F1 (638 k tri) + 122 tuiles F2 (118 k tri), 1,23 M sommets,
  ≈ 41 Mo (estimation initiale : 30 Mo). Génération 1,4-1,9 s au lieu de 0,3 s estimées, hors du
  chargement bloquant.
Détail : `docs/godot-map.md`, section « Villes 1:1 à toutes les hauteurs ».

## Addendum VT2 (30/09) : moulins, fumées et figurants à l'échelle 1:1

Contexte : après VT, moulins (jusqu'à ×125 : ~2 km de loin) et panaches de cheminée suivaient
encore l'exagération commune, et les figurants FK (ADR 0122) gardaient un plancher de lisibilité
qui les rendait plus gros que les îlots de Paris à d = 15. Le joueur a validé leur passage à
l'échelle réelle.

Décision :
- Moulins, panaches de cheminée et figurants FK (gens, bêtes, charrettes) sont à l'échelle 1:1 à
  toute distance de la vue 3D : moulin ~11 m (ailes ~18 m), panache ~9 × 28 m, homme 1,8 m,
  charrette ~4 m.
- Quand ils deviennent sous-pixel, on les **coupe** au-delà d'une portée au lieu de les grossir :
  moulins 30, panaches 25 (fondu), figurants 3 (distances du rig). Réglages dans
  `resources/map_prop_scale.tres` et `data/rules/map_scenes.json` (`figure_max_distance`).
- Arbres et incendies (signal d'un événement de jeu) restent exagérés.

Conséquences :
- La vie de la carte (figurants) n'apparaît plus qu'en vue très rapprochée (rig ≤ 3, soit une vue
  de ~2 km) ; au-delà, la campagne est vide de figurants. Les densités FK (réglées pour un rayon de
  60 u) donnent peu de figurants dans le rayon de 6 u : à rééquilibrer si la partie pilote le
  demande.
- Moins de travail pendant un zoom (plus de réécriture d'échelle des moulins et panaches) et
  moins de rendu (moulins, panaches et figurants masqués au-delà de leur portée).
- Détail : `docs/godot-map.md`, section « Moulins, fumées et figurants 1:1 (VT2) ».

## Addendum VT3 (30/09) : arbres à l'échelle 1:1

Contexte : après VT2, les arbres de la carte restaient grossis (`tree_ratio` 0,035, exagération
jusqu'à ×125 entre d = 28 et 8) : à d = 15 un houppier valait un quartier de Paris, seule
incohérence d'échelle avec les villes, hameaux et moulins 1:1. Le joueur a validé leur passage à
l'échelle réelle à toute distance de la vue 3D.

Décision :
- Arbres (tuiles de végétation, forêt dense, haies, vergers, arbres isolés) à l'échelle 1:1
  constante : `tree_ratio` 0,018 (chêne 14-22 m, hêtre 17-25 m, conifère 17-27 m).
- Pas de grossissement quand ils deviennent sous-pixel : plus aucun arbre individuel au-delà de
  la portée où un arbre de 20 m fait ≈ 1 px (1080p, fov 55°) : rig 30, distance caméra 30 par
  arbre (fondu par graine). Ombres des arbres jusqu'à d = 12.
- La forêt lointaine est portée par le terrain : canopée procédurale (relief des houppiers,
  contraste, quatre octaves filtrées par la taille du pixel) sur la part de forêt du splat.
- Couvert : forêt dense au pas réel (~21 m), part pleine jusqu'à d = 8, 0,35 à la portée ;
  plancher natif du pas de semis abaissé (0,05 → 0,01 u).
- Seuls les incendies (événement de jeu) restent exagérés ; les touffes d'herbe FC3 gardent,
  hors demande, l'ancienne courbe (`clutter_scale`), découplée des arbres.

Conséquences :
- Au-delà de d = 30, aucun coût d'arbre (appels de dessin, instances, ombres) ; la lecture des
  massifs dépend de la canopée du terrain.
- Près du sol, ~3 × plus d'instances par unité² de forêt : le budget de la forêt dense
  (300 k instances) resserre son rayon dans les grands massifs ; au-delà, arbres plus clairsemés
  sur la canopée.
- Les niveaux lointains de la végétation (imposteurs au-delà de `detail_distance`) ne servent
  plus en jeu.
- Détail et mesures : `docs/godot-map.md`, section « Arbres 1:1 (VT3) ».
