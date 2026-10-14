# CC C3 — provinces, lieux, factions (données)

Branche `cc/c3` (worktree `game_project-cc3`). Outil : `cent-ans compose rewrite`
(`tools/cent_ans_tools/compose/rewrite.py`), piloté par `data/map/compose/id_map.json` + `selection.json`.

## Reproduire
Partir des données d'avant composition : `git checkout 8e251a874 -- data && uv run --project tools cent-ans compose rewrite`
(idempotent sur des données déjà réécrites, sauf calculs qui exigent les provinces absorbées : régions).
Puis `CENT_ANS_REGEN_STARTING_FIT=1 cargo test -p sim-campaign --lib starting_fit` (fit des garnisons dérivé).

## Règles
- Province gardée = id de la table ; propriétaire/culture/religion/terrain/titre de la gardée ; population
  additionnée (taux moyennés au prorata), base_supply additionné, ressources/fleuves/ports/mers en union ;
  capitale = cité de la sélection. Buildings/fortification province = ceux de la gardée (ils vont à sa cité).
- Lieu gardé : copié, kind de la sélection ; venu d'une province de propriétaire différent => enclave (`owner`) ;
  l'ancienne cité gardée comme lieu secondaire reçoit bâtiments + fortification de son ancienne province ;
  bâtiments interdits au nouveau kind (ex. cathédrale en `town`) ou sans prérequis retirés.
- Factions sans territoire ni enclave retirées (10 de la liste + fac_golden_horde), avec personnages, titres, événements liés.
- Titres : domaine = provinces survivantes ; titre vidé retiré sauf titre primaire d'une faction qui a des terres ;
  enfants re-parentés (grand-père, sinon titre primaire du détenteur).
- Références : walker générique (liste -> élément retiré, clé facultative retirée, sinon l'élément porteur) ; cas
  particuliers : trade (comptoir -> cité de la province), sea_lanes (port -> port retenu le plus proche), rivers,
  battle_decor, building_regions, régions disparues, graphe des lieux (contracté, provisoire jusqu'à C4).
- Fichiers `data/map/` réécrits (références d'auteur, pas de géométrie) : settlement_graph (contraction provisoire),
  forced_sea_edges, detail_zones, map_birds, map_landmarks_extra, settlement_markers.

## Comptes
provinces 443 -> 208 ; lieux 1656 -> 400 ; factions 177 -> 166 ; personnages 253 -> 247 ; titres 221 -> 189 ; événements 159 -> 156.

## État
Fait. cargo test workspace vert (31 ignorés dont 12 marqués CC), clippy/fmt verts, pytest 2708 ok / 8 skipped.
Tests ignorés pour C4 (geometry) : h5_h6 captor_terms_parole_hold_and_cession, dp1 weariness_grows..., wh_diplob only_military_allies...,
campaign sea_crossings_go_port_to_port..., c6_agents spies_see_around_them, ai m3_grid_ai england_embarks_at_dover...;
Python skip C4 : settlement_graph (edge_paths, placed_inside), sea_lanes geometry, navgrid, horizon, detail_dem, forced_sea_edges.
Pour C6 (balance) : jr4b_starting_budget, france_income..., cm2_weather, a6_l2_siege_walls, a6_forecast_calibration, rg10_vocations_ai.

## Points ouverts
- Jérusalem (set_jerusalem) absente de la sélection C0a (cible de croisade -> prov_safad) ; Chypre n'a qu'un port ;
  la France ne possède que 3 provinces (Île-de-France, Normandie, Carcassonne) car les fusions suivent le suzerain
  mais gardent le propriétaire de la province gardée => `realm_region_provinces`, objectifs et textes à recaler (C6).
- Textes (description d'objectifs, titres) citent encore d'anciens noms de provinces fusionnées.
- Hameaux, `settlements_px`, `provinces.geojson`, `game/assets/horizon/relief/index.json` : régénérés en C4.
- Vignes de combat de la Champagne perdues (fusion dans Île-de-France).
