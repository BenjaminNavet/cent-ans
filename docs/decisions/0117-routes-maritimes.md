# ADR 0117 — Routes maritimes

Date : 2026-09-30. Statut : accepté. Lot SL1 (`docs/wip/sl1-routes-maritimes.md`).

## Contexte

Le graphe des colonies (`settlement_graph.json`, lot C3) ne relie par mer que des ports de provinces
voisines (`sea_neighbors` : Douvres–Wissant, Sandwich–Yarmouth, détroits de la Baltique…). Il manquait
les grandes lignes de navigation de la guerre de Cent Ans : aucune route Angleterre–Gascogne (l'armée
anglaise devait faire le tour par Calais), ni Londres–Flandre directe, ni galées italiennes. Rien
n'était dessiné sur la mer ; les routes commerciales « maritimes » étaient des traits droits à travers
les terres ; la maîtrise des mers (NV1, ADR 0028) n'agissait pas sur le commerce.

## Décision

- **Catalogue en données** : `data/naval/sea_lanes.json` (schéma `sea_lanes.schema.json`), 27 routes
  historiques entre ports (`from`, `to`, `sea`, `kind` : `coastal` ou `open_sea`, sources), plus les
  règles (coût au kilomètre, gros temps, facteur de saison du commerce, facteur d'interception).
- **Géométrie générée** : `cent-ans geo sea-lanes` route chaque ligne sur l'eau de `land_mask.png`
  (chemin de moindre coût à 2,9 km, éloigné des côtes, simplifié puis lissé sans toucher terre) et écrit
  `data/map/sea_lanes_px.json` (pixels carte + `length_km`). Le cœur n'en lit que la longueur ; Godot
  lit le tracé. Test `tools/tests/test_sea_lanes.py` (ports connus, tracé sur l'eau).
- **Graphe** : chaque route devient une arête maritime de `movement_graph` au chargement
  (`GameData::build_movement_graph`), coût = longueur × `cost_per_km` (0,5), au moins `min_cost_steps`
  pas. `settlement_graph.json` n'est pas régénéré : les routes s'ajoutent à ses courts passages.
  L'IA, les agents et le commerce (Dijkstra de `trade_paths`) les empruntent sans code dédié.
- **Règles** (`sim-campaign::sea_lanes`) :
  - la mer d'une traversée est celle de sa route (`naval::crossing_sea`) ;
  - chance d'interception × `intercept_factor` (haute mer 0,75) ;
  - gros temps : chaque régiment d'une armée qui traverse perd `storm_loss_percent` de ses hommes selon
    la saison et le type (haute mer : 1/0/3/8 %, cabotage : 0/0/1/3 %), doublé sous un orage de la météo
    de campagne (ADR 0027) au port de départ ; arrondi inférieur, jamais le dernier homme ; **aucun
    tirage** dans le générateur de la campagne (le déterminisme et les graines des tests existants sont
    préservés) ;
  - commerce : un tronçon maritime est coupé quand un ennemi de l'un des deux bouts tient sa mer au seuil
    de blocus (`blockade_control`) ou bloque l'un de ses ports ; en dessous, sécurité × (1 −
    `hostile_control_security` × maîtrise) ; valeur × `trade_season_factor` du tronçon le plus exposé.
    Cela remplace, pour la mer, le « pas de blocus naval dédié » de l'ADR 0012 écrit avant NV1.
- **Rendu** : `SeaLaneLayer` (tirets à l'encre, `sea_lane.gdshader`, largeur et tirets en pixels
  écran) : bleu nuit, bleu profond pour une mer tenue par le joueur, rouge pour une mer ennemie ; tirets
  longs en haute mer, courts en cabotage ; infobulle au survol ; masquée aux paliers vallée / site. Les
  routes commerciales suivent le tracé maritime. Un clic droit sur un port relié par la mer embarque
  d'abord (`is_sea_link`) au lieu de chercher un détour terrestre. Légende « Route maritime ».

## Conséquences

- Les traversées longues (Southampton–Bordeaux, Plymouth–La Corogne) tiennent en une saison comme les
  autres (`embark_cost = all`) : la mer est plus rapide que la terre, fidèle à l'époque. Le prix est le
  risque (interception, gros temps d'hiver), pas le délai.
- L'Angleterre peut renforcer la Guyenne par mer, et l'IA le fait si le chemin est plus court : à
  surveiller dans les sondes d'équilibre (EQ6).
- Pas de flotte-entité ni de patrouille sur les routes (limite de l'ADR 0028 inchangée) ; les courts
  passages du graphe ne sont pas dessinés (ils gardent le trait droit du commerce).
- Aucun changement de `STATE_VERSION` : aucun état nouveau, tout est dérivé des données et de
  `NavalState`.
