# R1 — Relief et occupation du sol réalistes de la carte de campagne

Branche de worktree `agent-a75fccf335fda0e44` (depuis `integration/night`). ADR :
`docs/decisions/0019-relief-et-occupation-du-sol.md`. Pipeline : `docs/geo.md` (section lot R1).

## Fait
- **Données hors ligne** (`tools/cent_ans_tools/geo/`, tests `tools/tests/test_landcover.py`) :
  - `copernicus.py` : Copernicus DEM GLO-90, 312 tuiles (≈ 1 Go, cache `tools/geo/raw/copernicus/`).
  - `kk10.py` : KK10 1330-1349 lu par requêtes HTTP partielles (cache 190 ko).
  - `relief_shade.py` (`cent-ans geo relief-shade`) : tuiles fines 8192² Copernicus moyennées,
    `heightmap_render.png` (relief de rendu rehaussé, trait de côte inchangé), `relief_shade.png`.
  - `landcover.py` (`cent-ans geo landcover`, appelé aussi par `geo splat`) : `splat.png` 4096²,
    `wetlands.png`, `forest_kind.png`.
  - Fichiers curés + schémas : `data/map/historical_forests.json` (55), `data/map/wetlands.json` (32).
- **Rendu** : `game/shaders/relief_landcover.gdshaderinc` (4 crochets d'une ligne dans
  `terrain.gdshader`), `game/scripts/map/relief_landcover.gd` (1 ligne dans `terrain_builder.gd`),
  `MapData.height_file` (lit `heightmap_render.png` si `map.json.render_heightmap`).
- Crédits (`CREDITS.md`, écran des crédits), ADR 0019, `docs/geo.md`.
- Captures `docs/img/r1/before_*.jpg` / `after_*.jpg` (+ vues rapprochées `after_*_near.jpg`) :
  `godot --path game --script res://tests/r1_relief_shots.gd -- --out=<dossier> --prefix=after --near`.

## Contrat pour V4 (rendu des forêts)
- `splat.png` : contrat inchangé (RGBA, B = forêt), taille désormais 4096² (lire la taille dans le fichier).
  Le canal B est binaire lissé (σ 0,7 px) : forêt ⇔ B > 0,5 ; les lisières sont nettes.
- `forest_kind.png` : L8 2048², même emprise, 0 = feuillus, 255 = résineux (part de résineux dans la forêt).
- `wetlands.png` : RGB8 4096² (R marais, G étangs, B prés humides) si V4 veut éviter d'y planter des arbres.

## Mesures
Banc `vegetation_bench.gd` (M4 Pro, vsync coupée, ~10 agents actifs : bruit de ±50 %), avant =
données de la base `650bb94` via `CENT_ANS_DATA_DIR` (même code, crochets R1 inactifs faute de
rasters). Temps de trame médians sur 3 passes avec végétation (avant → après, ms) : France
13,8 → 9,9 ; zoom moyen 20,3 → 17,4 ; proche 27,3 → 38,8 (moyennes 30,5 → 35,4, +11 % d'arbres
visibles : plus de forêt) ; forêt d'Orléans 38,8 → 24,5 ; très proche 17,9 → 17,5 ; bocage
22,7 → 23,7. Sans végétation (2 passes chacune) : écarts dans le bruit, aucune tendance.
Aucune régression > 10 % démontrable ; la vue « proche » est à surveiller sur machine calme.
Chargement : `relief_shade.png` décodé en tâche de fond (≈ 0,9 s, n'allonge plus le démarrage) ;
splat 4096² : + ≈ 150 ms sur `masks_ms`. Smoke : OK (sortie 0).

Taille ajoutée (fichiers courants) : ≈ + 66 Mo (relief_shade 42,9, heightmap_render 14,9, splat
8,4 au lieu de 3,8, tuiles 53,0 au lieu de 50,5, wetlands 0,3, forest_kind 0,4, captures 4,3).
L'historique wip contient en plus une version intermédiaire de splat.png (8 Mo) et les captures
« avant » en PNG (10 Mo).

## Points ouverts
- `navgrid.py` lit désormais `data/map/navgrid_splat.png` (splat V2 figée) : les coûts de forêt du
  cœur ne changent pas. Régénérer la grille sur les forêts historiques = décision de règle.
- `tools/tests/test_portraits.py::test_dry_run_makes_no_network_call` échoue (hors périmètre R1).
- 16384² sur terre écarté (taille, construction GDScript des maillages) : voir ADR 0019.
- Réglages en uniforms (`rl_*`) : occlusion, étangs, roselières.
- Tracés des massifs et marais : ellipses approchées ; un tracé polygonal plus fin (Sherwood,
  Fens...) améliorerait la fidélité.
