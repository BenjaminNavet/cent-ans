# PB1 — Benchmark et optimisation des performances (FPS, chargements, zooms)

Demande : benchmarker le jeu et optimiser FPS + temps d'exécution (chargements, zooms) sans
dégrader le contenu. Branche `pb1-perf`, worktree `../gp-pb1` (lien symbolique
`data/map/pyramid` → cache de l'arbre principal). Coordination : ZG4 (branche `zg4-camera`,
session game-project-86) s'occupe des recalages incrémentaux de `landmark_model` /
`settlement_layer` — ne pas y toucher en double.

## Outils de mesure
- `godot --path game --script res://tests/pb1_bench.gd [-- --views=1500,491,150,40 --trace --veg-jobs=N]`
  → `PB1_JSON` : chargement carte, stabilisation après chaque saut de zoom (relief fin +
  végétation), temps d'image par vue (médiane de 3 échantillons), zoom animé France↔Paris
  (médiane/p95/max). `--trace` : pics > 60 ms attribués. GPU seulement avec
  `--rendering-driver vulkan` (Metal ne mesure pas).
- `godot --path game --rendering-driver vulkan -- --journey --uncapped --map-ab=<d> --ab-configs=...`
  A/B GPU (configs combinables `angular:0+soft:3+no_blend`, `nocast:<nœud>`, `hide:<nœud>`…).
  Bug corrigé : `no_fog`/`no_shadows` n'étaient jamais restaurés (base faussée).
- Parcours complet : `godot --path game -- --journey --uncapped` (`JOURNEY_JSON`).

## Référence (avant PB1 et avant ZG2, M4 Pro, 1440×900, qualité Haute)
- Carte (Vulkan, GPU ms) : d=1500 5,7 · d=1250 17 · d=491 17,5-23 · d=150 24,6-28 · d=40 27,4.
  Limitée par le GPU près du sol ; ombres ≈ 10-13 ms, dont le PCSS du soleil.
- Zoom : pire image 1,25 s à l'arrivée à d=150 ; zoom animé max 530 ms, p95 133 ms.
- Chargement carte 4,4-5,9 s ; fin de tour 424 ms (cœur Rust 80 ms) ; bataille 18,5 ms/image.

## Fait
1. (main, f2eacfb1) Soleil de campagne sans PCSS (`light_angular_distance` 0) : −4 à −6,5 ms GPU
   aux zooms proches, aucune différence visible (captures comparées, `shadow_blur` garde le flou).
2. (pb1-perf) Pics du fil principal au zoom (mesurés avant ZG2) :
   - `FrameBudget` (6 ms/image) pour tuiles proches, rubans de route, hameaux (≥ 1 par image).
   - Étiquettes de colonies recalées une fois par image (plus à chaque tuile).
   - `CampaignLife._reground` : seulement les points des tuiles changées ; tampon MultiMesh
     écrit en bloc.
   - Tuiles proches : indices en cache, hauteurs sans `surface_get_arrays`.
   - `TerrainBuilder.surface_heights_at` (lot, identique à `surface_height_at`, chemin
     quadtree délégué) utilisé par les rubans de route.
   - (abandonné au profit de ZG4) regroupement de `LandmarkModel._bake_heights` : ~4 s cumulées
     sur le banc avant ZG2.
   Résultat intermédiaire : pire image arrivée d=150 456 ms (était 640-985), zoom animé max
   225 ms (était 390-530), p95 63 ms (était 80-133).

## En cours / prochaines étapes
- Rebasé sur ZG0-ZG3 (quadtree CDLOD) : **re-mesurer la référence** (tout le relief change).
- **Instrumentation temporaire à retirer avant fusion** : `_PT` dans `campaign_map.gd`
  (`_process`), `TerrainBuilder.PT` ; `game/tests/pb1_shots.gd` à supprimer.
- Pics restants (avant ZG2) : `settlement_layer.update_view`, `terrain.update_lod`, `Crossings`.
- Végétation à d=491 : ~15 s avant la forêt complète.
- Ensuite : chargement de la carte, fin de tour (340 ms de GDScript), bataille.
- Smoke + tests avant fusion.
