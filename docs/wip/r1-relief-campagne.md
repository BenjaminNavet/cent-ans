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
- Captures `docs/img/r1/before_*.png` / `after_*.png` (+ vues rapprochées `after_*_near.png`) :
  `godot --path game --script res://tests/r1_relief_shots.gd -- --out=<dossier> --prefix=after --near`.

## Contrat pour V4 (rendu des forêts)
- `splat.png` : contrat inchangé (RGBA, B = forêt), taille désormais 4096² (lire la taille dans le fichier).
  Le canal B est binaire lissé (σ 0,7 px) : forêt ⇔ B > 0,5 ; les lisières sont nettes.
- `forest_kind.png` : L8 2048², même emprise, 0 = feuillus, 255 = résineux (part de résineux dans la forêt).
- `wetlands.png` : RGB8 4096² (R marais, G étangs, B prés humides) si V4 veut éviter d'y planter des arbres.

## Performance
Voir la section « Mesures » ci-dessous (banc `vegetation_bench.gd`, avant = données de la base
`650bb94` via `CENT_ANS_DATA_DIR`, même code : shader R1 inactif faute de rasters).

## Prochaine étape
- Captures finales, banc, smoke ; rapport.
