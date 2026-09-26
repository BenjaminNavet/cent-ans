# ADR 0080 — Mise à l'échelle 3D MetalFX

Date : 2026-09-26. Statut : accepté. Lot PB3b (`docs/wip/pb3b-metalfx.md`, plan
`docs/wip/pb3-performance.md`).

## Contexte

Au zoom proche, la campagne est limitée par le GPU sur le M4 Pro (≈ 22-24 ms : relief ≈ 12,
ombres ≈ 7,5, MSAA ≈ 5,5 ; PB1, PB2, PF1). Godot 4.7.2 propose sur le viewport
`scaling_3d_mode` / `scaling_3d_scale` avec `SCALING_3D_MODE_METALFX_SPATIAL` (3) et
`SCALING_3D_MODE_METALFX_TEMPORAL` (4) sous Metal, FSR 1 (1) et FSR 2 (2) ailleurs. La 3D est
calculée en plus petit puis agrandie ; l'interface 2D reste à la définition native.

## Décision

1. `RenderQuality` gagne deux clés globales par préréglage, `upscale_mode` (`off`,
   `metalfx_spatial`, `metalfx_temporal`) et `upscale_scale`, appliquées au viewport racine
   (carte et bataille) par `apply_upscale`. Hors Metal (`RenderingServer.get_current_rendering_driver_name()`
   ≠ `metal`) : FSR 1 remplace le MetalFX spatial, FSR 2 le temporel. Un mode temporel coupe MSAA
   et FXAA (il fait lui-même l'anticrénelage) ; sinon MSAA du préréglage et FXAA de `project.godot`.
2. Préréglages : Basse spatial 0,67 ; Moyenne et Haute spatial 0,75 ; Ultra et `legacy` natifs.
   « Automatique » (RL1) choisit Haute sur M4 Pro, donc MetalFX spatial 0,75.
3. Réglage joueur `video/upscale`, Réglages > Affichage > « Mise à l'échelle » : Automatique
   (suit le préréglage), Désactivée, MetalFX qualité (spatial 0,75), MetalFX performance
   (spatial 0,5). Libellé « Automatique (MetalFX spatial 75 %) » selon la machine.
4. Le temporel n'est pas proposé : il est gardé dans le code et les bancs.
5. Bancs : `release_journey --ab-configs=off,metalfx_s:0.75,metalfx_t:0.75,…` (le banc de carte
   mesure désormais `base` à la définition native), bataille `--bench-ab=off,metalfx_s:0.75,…`,
   captures `--upscale=<config>`. Test `tests/pb3b_upscale_test.gd`.

## Mesures

Temps d'image (ms), vsync coupée (`--uncapped`), Metal, fenêtre 1920 × 1080, préréglage Haute,
debug. Médiane des rapports par série (3 séries alternées, 4 à Paris 150 ; d = 12 : une série).
La machine est partagée : les séries plafonnées par le compositeur (6,9 / 16,7 ms, fenêtre
masquée) ont été écartées et relancées.

| Vue (Paris) | Natif | MetalFX spatial 0,75 | MetalFX temporel 0,75 | spatial 0,5 | temporel 1,0 | bilinéaire 0,75 |
|---|---|---|---|---|---|---|
| d = 1500 | 15,7 | 13,1 (−16 %) | 13,2 (−16 %) | 11,8 (−25 %) | 17,8 (+13 %) | 14,3 (−8 %) |
| d = 491 | 32,7 | 27,1 (−17 %) | 27,7 (−15 %) | 22,3 (−32 %) | 26,3 (−19 %) | 26,1 (−20 %) |
| d = 150 | 32,1 | 23,4 (−27 %) | 20,9 (−35 %) | 17,9 (−44 %) | 27,5 (−14 %) | 22,7 (−29 %) |
| d = 40 (Paris) | 36,9 | 31,0 (−16 %) | 35,9 (−3 %) | 29,2 (−21 %) | 37,2 (+1 %) | 30,0 (−19 %) |
| d = 12 | 31,5 | 23,2 (−26 %) | 19,2 (−39 %) | 17,0 (−46 %) | 25,9 (−18 %) | 22,4 (−29 %) |

Bataille (20 régiments par camp, 4 782 soldats, `--bench-at=20`, moyenne des temps d'image par
configuration, médiane de 3 séries) : natif 20,3 ; spatial 0,75 18,1 (−11 %) ; temporel 0,75
18,2 ; spatial 0,5 16,8 (−17 %) ; temporel 1,0 18,6. La bataille est surtout limitée par le
processeur (bibliothèque debug), le gain y est plus faible.

Le MetalFX spatial coûte à peine plus que l'agrandissement bilinéaire (écart dans le bruit) ; le
temporel 0,75 gagne un peu plus au zoom proche (MSAA coupé) mais le temporel 1,0 (anticrénelage
seul) ne gagne rien de net.

## Captures (`docs/img/pb3b/`)

- `carte_d40_paris.jpg`, `carte_d12_moulin_pluie.jpg`, `carte_d150_foret.jpg` : natif, spatial
  0,75, temporel 0,75, spatial 0,5, temporel 1,0, bilinéaire 0,75.
- `bataille_melee.jpg` : mêlée en mouvement, poussière et mottes (natif, spatial 0,75, temporel
  0,75, spatial 0,5).

Verdict (captures regardées une à une) :
- Spatial 0,75 : très proche du natif en 1080p ; toits de Paris et arbres un peu plus
  contrastés (affûtage), étiquettes 3D nettes, relief du quadtree et fleuves intacts. Retenu.
- Spatial 0,5 : étiquettes 3D (« Paris ») et toits nettement flous, crénelage visible : réservé
  au choix « performance ».
- Temporel : **la pluie disparaît presque entièrement** (gouttes fines et rapides sans vecteurs
  de mouvement : effacées par l'accumulation) et **les ailes des moulins laissent un fantôme**
  translucide (rotation faite dans le shader de sommets `life_windmill`, sans vecteur de mouvement).
  Même défaut attendu pour tout ce qu'anime `TIME` dans un shader de sommets (végétation au vent,
  drapeaux, eau, voiles). Les soldats en mêlée restent propres sur une image fixe.
  Corriger demanderait de fournir aux shaders personnalisés la position de l'image précédente
  (TIME précédent), ce que Godot n'expose pas : hors de portée, d'où le spatial.

## Conséquences

- Sur M4 Pro, préréglage Haute : ≈ −16 à −27 % de temps d'image sur la carte, ≈ −11 % en
  bataille, sans perte visible en 1080p. L'écart grandit en plein écran Retina (plus de pixels).
- Les bancs GPU existants (`pb1_bench`, `zg7a_gpu_ab`, `--bench-ab=<niveau>`) mesurent désormais
  le préréglage avec sa mise à l'échelle ; `release_journey --map-ab` force `off` sauf config
  `metalfx_*`/`bilinear:*`, ce qui garde la référence native comparable aux mesures passées.
- À reprendre si Godot expose les vecteurs de mouvement des sommets animés : le temporel 0,67
  deviendrait intéressant (anticrénelage et gain au zoom proche).
