# R1 — Relief et occupation du sol réalistes de la carte de campagne

Branche de worktree `agent-a75fccf335fda0e44` (depuis `integration/night`). ADR : `docs/decisions/0019-relief-et-occupation-du-sol.md`.

## Plan
1. Données hors ligne (`tools/cent_ans_tools/geo/`) :
   - `copernicus.py` : Copernicus DEM GLO-90 (bucket AWS public, cache `tools/geo/raw/copernicus/`, 312 tuiles 1°).
   - `kk10.py` : KK10 (PANGAEA, CC-BY) lu par requêtes HTTP partielles (h5py + fsspec en `--with`), moyenne 1330-1349,
     cache `tools/geo/raw/kk10/*.npz`.
   - `relief_shade.py` : tuiles fines 8192² refaites depuis Copernicus (moyenne de zone), `heightmap_render.png`
     (relief de rendu rehaussé, lu par `MapData`), `relief_shade.png` (détail + occlusion/courbure).
   - `landcover.py` : forêts vers 1340 (KK10 × potentiel × massifs nommés `data/map/historical_forests.json`),
     `splat.png` régénéré, `forest_kind.png` (essences), `wetlands.png` (`data/map/wetlands.json`).
2. Rendu : `game/shaders/relief_landcover.gdshaderinc` + crochets d'une ligne dans `terrain.gdshader`.
3. Captures avant/après `docs/img/r1/`, banc `vegetation_bench.gd`, smoke.

## État
- Squelette posé ; téléchargements Copernicus et KK10 en cours.

## Prochaine étape
- Mesurer la taille des tuiles fines refaites (8192² vs 16384²), écrire `relief_shade.py`.

## Notes pour la reprise
- `tools/geo/raw` du worktree est un lien symbolique vers `tools/geo/raw` du dépôt principal (caches partagés).
- KK10 : `uv run --project tools --with h5py --with fsspec --with aiohttp --with requests cent-ans geo kk10`.
