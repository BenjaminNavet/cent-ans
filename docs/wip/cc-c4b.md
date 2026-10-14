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

## JALON 1 OK (smoke.gd passe)
- Chaîne : `geo compose --rebuild-warp` (Jérusalem au premier rang) -> `geo compose-relief` -> `geo compose-layers` -> `geo provinces`
  -> `geo settlements` -> `geo hamlets` -> `geo navgrid`.
- `geo provinces` : sur carte composée, étiquette de chaque pixel = ancienne étiquette réelle (`data/map/real/province_ids.png`,
  grille réelle) lue par la déformation inverse, convertie par `id_map.json` ; côte = `land_mask`, trous < 4 000 px comblés,
  filtre majoritaire 5 x 5, composantes < 60 px absorbées. 213 provinces, aucune manquante. `_join_components` relie par la
  côte la plus proche tout groupe de provinces coupé du reste (Irlande, Crimée : le détroit composé dépasse 60 px).
- Lieux : positions via `place_positions` (retouches de l'éditeur) ; arête terrestre à > 50 % sur l'eau sans ports aux deux bouts
  : supprimée (au lieu de gardée terrestre). `set_mdina` devient port (Malte). Arêtes maritimes forcées : Caffa-Trébizonde,
  Caffa-Constantinople. Graphe : 1 composante.
- Hameaux : `former_settlements.json` relu via `id_map` ; positions NaN hors déformation ignorées (3 494 hameaux).
- `data/settlements/rules.json` `points_per_step` 70 -> 250 (médiane des arêtes 213 contre 58 avant) : sinon Paris n'atteint
  aucune autre province en une saison. A recaler en C6 avec `meters_per_px`.
- Smoke recalé : édits sur prov_touraine (France ne possède plus prov_bourbonnais) ; régime « refusé » pris dans une province
  voisine si la France n'en a pas.
