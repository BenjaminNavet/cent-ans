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

## JALON 1 OK bd108080d (smoke.gd passe)
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

## JALON 2 (en cours) b87d3c992
Fait : `geo roads --computed` (528 tronçons calculés, 352 arêtes sur route), `geo splat` (+ border_dist, landcover complet),
`geo rivers-render` (ponts de routes), `geo navgrid` (3540 x 3052, connexité vérifiée, 28 cols), `geo colormap` (routes, villages,
halos), `geo sea-lanes`, `compose authored-px` (polygones `coast_types` / `sea_basins` passés par la déformation, clé `px_frame`;
sites de zones humides ; décor `map_landmarks_extra`), `geo horizon` (213 tuiles, 15,6 Mo ; 233 tuiles d'anciennes provinces
supprimées), fixture croisée Rust/Python (`compose/cross_fixture.py`).
Tests Python : plus aucun ignoré `CC C4`. Recalages justifiés :
- `test_compose_contract::no_tiny_islands` : les lacs fins (C4a) sont des anneaux légitimes, parfois ronds ;
- `test_coast_types` / `test_sea_basins` : projection par `grid_from_metadata` (px composés) ; schémas : clé `px_frame` ;
- `test_landmarks` : `anchor.px` reste en grille réelle (villes 1:1 non dessinées sur le canevas composé), plus de proximité au lieu ;
- `test_navgrid` : taille 3540 x 3052 ; `pass_darial` retiré de `crossings.json` (Géorgie / Alanie hors carte, pas de province) ;
- `test_sea_lanes` : marge d'estuaire 150 -> 220 px (Londres est à 200 px de la mer : échelle ~1,5 de l'Ouest) ; route
  Winchelsea-Dieppe repartie de Douvres (Winchelsea absorbé) ; routes Bordeaux-Bayonne et Nantes-Brest supprimées (les deux
  ports sont dans la même province fusionnée, arête terrestre) ;
- `test_compose_select` : lit l'adjacence d'origine (`data/map/real/provinces.geojson`) ; `test_horizon` : tuiles obsolètes retirées.
Restent liés au relief de C2b (à refaire après fusion) : `test_biomes` Mont-Blanc / Alpes, `test_compose_relief` crêtes.
Rust : `max_muster_km` 200 -> 300 (`data/rules/feudal.json`, même raison d'échelle) ; g5 / om_i1 : adjacence réelle Écosse-Angleterre
et France-Bourgogne rétablie (le graphe réel la redonne) ; rc1 : > 300 passages (routes calculées) ; sl1 : prix de la route
du vin 600-900 ; real_data : navgrid 3540 x 3052 ; starting_fit regénéré.
