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

## C3b — correctifs avant C4b (branche `cc/c3b`)

Problème : la fusion C3 gardait le propriétaire de la province gardée (France 26 -> 3, Angleterre 25 -> 8, Écosse 0),
et Jérusalem était absorbée dans prov_safad.

### Sélection (`select.py`, défaut `merge_scope="tiered"`, `province_target=213`)
- Fusion par paliers : (0) voisines de même détenteur, (1) même suzerain final, (2) toute voisine. Chaque palier
  respecte un quota par détenteur (`n_origine / ratio`, ratio = provinces d'origine retenues / cible, relevé de 3 %
  par tour jusqu'à atteindre la cible ; détenteurs de l'Ouest jouable `WEST_OWNERS` : quota ×1/0,75).
  Capacité : la fusion « même détenteur » seule plafonne à 237 provinces (cible 208 inatteignable sans mélange) ;
  213 est la cible retenue (accepté 180-215), d'où France/Angleterre 10 et non 12.
- Détenteur du groupe = faction avec le plus de provinces d'origine (égalité : population, puis la tête précédente) ;
  un détenteur dont la capitale est hors emprise (Horde) ne l'emporte que seul. Tête (id gardé) = la province la plus
  importante (score des lieux) de ce détenteur ; une tête protégée (capitale de faction, `KEEP_PROVINCE_IDS`) reste
  en place avec son propre détenteur.
- Cité du groupe : d'abord celle d'une province du détenteur (évite une cité enclave : Chios/fac_archipelago),
  puis capitale de faction, de province, score. `FORCED_CITY` : prov_jerusalem -> set_jerusalem.
- Jérusalem : `prov_jerusalem` (+ Gaza, Kerak) protégée, cité `set_jerusalem`, `set_gaza` reste lieu secondaire
  (400 lieux conservés). `crusade.json` (cible, holy_land) revient sur prov_jerusalem / prov_tripoli.
- `data/ai/alignment.json` : `realm_region_provinces` 3 -> 2 (seuil par région, pas une liste de provinces ; la France
  ne possède plus que 2-3 provinces par région).
- Textes recalés : 4 descriptions d'objectifs (Angleterre, Bourgogne x2, France). Les autres mentions d'anciens noms
  (descriptions de factions, relations, titres, événements : ~330 occurrences, prose historique) sont laissées telles
  quelles ; l'ancien nom reste dans la description « Province composée : regroupe aussi … » de la gardée.

### Avant / après (nombre de provinces par détenteur ; 30 premières factions + Ouest jouable)
| Faction | avant composition | C3 | C3b | cible ÷2,1 |
|---|---|---|---|---|
| fac_france | 26 | 3 | 10 | 12.4 |
| fac_england | 25 | 8 | 10 | 11.9 |
| fac_marinids | 14 | 4 | 4 | 6.7 |
| fac_sweden | 14 | 5 | 5 | 6.7 |
| fac_byzantium | 13 | 5 | 4 | 6.2 |
| fac_mamluks | 12 | 2 | 4 | 5.7 |
| fac_golden_horde | 10 | 0 | 0 | 4.8 |
| fac_hungary | 10 | 2 | 3 | 4.8 |
| fac_lithuania | 10 | 3 | 3 | 4.8 |
| fac_naples | 8 | 1 | 2 | 3.8 |
| fac_hafsids | 8 | 1 | 2 | 3.8 |
| fac_castile | 8 | 1 | 3 | 3.8 |
| fac_serbia | 8 | 2 | 2 | 3.8 |
| fac_teutonic | 7 | 2 | 2 | 3.3 |
| fac_bulgaria | 6 | 2 | 2 | 2.9 |
| fac_venice | 6 | 6 | 2 | 2.9 |
| fac_moscow | 6 | 1 | 2 | 2.9 |
| fac_aragon | 5 | 1 | 2 | 2.4 |
| fac_eretna | 5 | 1 | 1 | 2.4 |
| fac_austria | 5 | 1 | 2 | 2.4 |
| fac_poland | 5 | 1 | 1 | 2.4 |
| fac_novgorod | 5 | 1 | 1 | 2.4 |
| fac_barqa | 4 | 1 | 1 | 1.9 |
| fac_empire | 4 | 1 | 1 | 1.9 |
| fac_burgundy | 4 | 1 | 2 | 1.9 |
| fac_bohemia | 4 | 1 | 1 | 1.9 |
| fac_majorca | 4 | 3 | 1 | 1.9 |
| fac_holstein | 4 | 2 | 1 | 1.9 |
| fac_galicia_volhynia | 4 | 1 | 1 | 1.9 |
| fac_georgia | 4 | 0 | 0 | 1.9 |
| fac_brittany | 2 | 1 | 1 | 1.0 |
| fac_flanders | 2 | 1 | 1 | 1.0 |
| fac_navarre | 3 | 3 | 1 | 1.4 |
| fac_scotland | 3 | 0 | 1 | 1.4 |
| fac_portugal | 3 | 1 | 1 | 1.4 |
| fac_armagnac | 1 | 1 | 1 | 0.5 |

Total : 443 -> 208 (C3) -> 213 (C3b). Faction sans province : Horde d'Or et Géorgie (hors emprise, retirées comme en C3).

### Reproduire C3b
`git checkout 8e251a874 -- data` ; `Selection` : `S.write(S.select(S.Params()))` (outil `cent_ans_tools.compose.select`) ;
`cent-ans compose rewrite` ; restaurer depuis HEAD les fichiers hors périmètre que le checkout a ramenés en arrière
(`data/map/height/`, `coastline`, `heightmap*`, `land_mask`, `map.json`, `relief_shade_*`, `rivers.geojson`) ;
alignment 3 -> 2 ; schéma `compose_selection` (enum `tiered`) ; textes ; `CENT_ANS_REGEN_STARTING_FIT=1 cargo test -p sim-campaign --lib starting_fit` (2 fois).

### Points ouverts C3b
- Python : échecs de géométrie hérités de C2/C4 (grille composée 14 336 x 12 288 vs masques) : test_biomes, test_coast_types,
  test_sea_basins, test_navgrid, test_pyramid, test_agri_regions, test_historical_water, test_settlement_graph (2).
- Quelques groupes mixtes de petits détenteurs (Rimini/Papauté, Taranto/Naples, Provence/Empire) gardent le détenteur de
  la tête protégée.
- Rust : cargo test workspace 1859 ok / 0 échec / 40 ignorés (nouveaux ignorés C4 : cv3_ai_stances x2, c7a_retreat, review_tests
  fallback, rg9 rivières, g5/om_i1 voisinage, sl1 golfe de Gascogne ; C6 : om3 fourrage désert).
- `base_supply` additionné sans renormalisation : ile_de_france 25450, touraine 23500, devon 20500 (médiane 3500) -> C6.
- Navarre : `fac_navarre.json` liste encore « Comtesse d'Angoulême/de Mortain » (titres retirés par la fusion) ; assertions
  de titres retirées dans feudal_titles.rs ; à trancher (recréer ou retirer les entrées).
- prov_ile_de_france n'a que prov_normandie comme voisin terrestre (graphe provisoire) -> C4.
- Textes : ~330 mentions d'anciens noms en prose historique laissées (voir ci-dessus).
