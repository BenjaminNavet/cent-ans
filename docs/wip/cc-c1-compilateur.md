# CC C1 — compilateur de la carte composée

Branche `cc/c1` (worktree `game_project-cc1`). Contrat : `docs/wip/cc-carte-composee.md` § Contrat commun.

## Plan
1. `compose/contract.py` : écriture/lecture de `warp.json` + `warp_forward.f32` + `warp_inverse.f32`, classe `ComposedWarp` (interpolation bilinéaire, NaN hors emprise).
2. `geo/project.py` : `ComposedGrid` (même API que `MapGrid`), `RealGrid` = ancienne grille, `grid_from_metadata` gère `"crs": "composed"`.
3. `compose/compile.py` + `cent-ans geo compose` : warp, land_mask, coastline, rivers, heightmap provisoire, ridges.json, map.json.
4. Schémas : `compose_warp`, `compose_pins`, `compose_generalise`, `compose_ridges`, `map`.
5. Recensement des copies de projection Python.
6. Tests de contrat `tools/tests/test_compose_contract.py`.

## État
- Squelette posé.

## Prochaine étape
- contract.py + project.py.

## Avancement (14/10)
- Fait : `contract.py`, `compile.py`, `ComposedGrid`/`RealGrid`, schémas, `geo compose` (≈ 50 s warp relu, 2 min avec `--rebuild-warp`), tests de contrat, sorties régénérées dans `data/map/`.
- Sources réelles lues dans `data/map/real/` (gitignoré, reconstruit par `git show 8e251a874:` si absent).
- Reste : recensement des copies de projection Python, ruff, rapport.
