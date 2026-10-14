# CC C4b — couches dépendant des provinces et des lieux

Branche `cc/c4b` (worktree `game_project-cc4b`).

## Plan
- Jalon 1 : `geo compose --rebuild-warp` -> `compose-relief` -> `compose-layers` -> `geo provinces` (union des provinces réelles
  déformées, `compose/provinces_composed.py`) -> `geo settlements` (positions via `place_positions` + retouches) -> `geo hamlets`
  -> smoke.gd.
- Jalon 2 : routes calculées, navgrid (règle massif), graphe, ports, coast_types, sea_basins, sea_lanes, splat/colormap définitifs,
  crossings_px, horizon, detail_zones ; lever les tests `CC C4: geometry`.

## État
- Provinces composées : fait (213 provinces, rapport dans la sortie de `geo provinces`).
- Settlements, hameaux : faits (400 lieux, 3 494 hameaux).
