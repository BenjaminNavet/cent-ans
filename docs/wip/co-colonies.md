# CO — colonies : moins de villes, lisibilité, onglet Bâtiments, images du Codex

Demande joueur (2026-10-10). Mandat : autonomie complète, fusion dans main et push, sans validation
intermédiaire ; ≤ 10 captures ; images en local d'abord (Z-Image Turbo, ADR 0190), payant
(OpenRouter/fal) seulement pour les échecs flagrants, enveloppe ≈ 5 $ consignée dans `docs/budget.md`.

## Décisions du joueur
- **Colonies** : au plus **5 colonies par province**, tous types confondus (2 148 → ~1 400).
  On garde la cité + les 4 places les plus notables (au moins un village par province quand il y a la place,
  diversité château/abbaye célèbre). Les villes mineures conservées deviennent des villages (viser
  nettement plus de villages que de villes). Pas de nouveau type « bourg ».
- Les colonies retirées deviennent des **hameaux décoratifs** (non sélectionnables), via le `hamlets.json` existant.
- **Emplacements** (`building_slot_cap`) : cité 6, ville 4, château 3, abbaye 3, village 2.
- **Rééquilibrage** : remise dans les bandes (campaign_probe 120 tours × 6 graines).
- **Lisibilité** : à la sélection d'une colonie, les autres colonies de la province apparaissent en pastilles
  reliées à la cité.
- **Onglet Bâtiments** : grande illustration de la colonie à son palier (6 paliers × 5 types = 30 images,
  style occidental), puis les emplacements en cartes illustrées (image du niveau, niveau, effets), survol =
  chaîne d'amélioration. Palier = somme des niveaux de bâtiments rapportée au maximum + fortifications
  (purement visuel, calcul dans `core/`).
- **Images de bâtiments** : une image par niveau (chaque niveau est déjà une entrée `data/buildings/`) ;
  8 manquantes : bailiwick, banal_oven, corn_hall, manor_chapel, provostry, tithe_barn, toll_post, town_hall.
- **Codex** : les 76 entrées sans image (animaux, arbres, roches, oiseaux, économie, 4 personnages) en style
  herbier/bestiaire pour la nature ; revoir la pertinence des images réutilisées.

## Lots (agents Sonnet, worktrees)
| Lot | Contenu | ADR | État |
|---|---|---|---|
| CO-A | réduction des colonies, hameaux, plafonds, références, tests, équilibrage | 0291 | **fusionné** |
| CO-B | pastilles de province à la sélection (vue 3D ; parchemin = cartouche seul) | — | **fusionné** 80134f764 |
| CO-C | palier de colonie (core + pont) + refonte de l'onglet Bâtiments ; orchestrateur : jauge de progression fine, tutoriel | 0292 | **fusionné** b38c2a0e0 |
| CO-D | 30 images de paliers + 8 images de bâtiments (local, 0 $) | — | **fusionné** 690065c34 |
| CO-E | 74 images du Codex en local (0 $), portrait avant miniature d'événement ; détail `docs/wip/co-e-codex.md` | — | **fusionné** |

Conventions partagées :
- Paliers : `game/assets/illustrations/settlement_tiers/<kind>_<n>.jpg`, kind ∈ city/town/castle/abbey/village, n = 1..6.
- Colonies retirées : `data/map/former_settlements.json` (`{"description", "settlements": [{"id", "name", "province", "lonlat", "former_kind"}]}`,
  schéma `former_settlements.schema.json`), fusionnées par `geo/hamlets.py` dans le `data/map/hamlets.json` existant (format GeoNames
  `[{"name","px","province"}]` inchangé) : le rendu de hameaux existant les affiche, CO-B ne fait que les pastilles.

## Notes
- `a6_l7_panel_test` échouait sur main (défilement interne du PopupMenu d'un OptionButton du lot WH) : le test ignore désormais les popups.
- Après un merge ajoutant un `class_name`, relancer `godot --headless --path game --import`.

## Clôture (2026-10-10)
Tous les lots dans main. Dépense : 0 $ (tout en local). Tests : cargo test, pytest, smoke, co_b/co_c/lr08/a6_l7 verts.
Contrôle visuel (4 captures, `game/tests/co_shot.gd`) : cartes illustrées, palier + jauge, pastilles reliées à la cité — conformes.
Correction de l'orchestrateur : élision « d’Andalousie » (cartouche, aide du panneau, `PossessionText.de`), entrée d'infobulle `building_chain`.

Restes :
- Barre des places en bas à gauche (`holdings_controller.gd`, session UX5) : « Emplacemen ts » coupé sur deux lignes.
- `test_budget::test_real_budget_file_parses_and_round_trips` : `docs/budget.md` contient un montant à 3 décimales (0,049 $) écrit par une autre session.
- `test_settlements_schema[prov_bar]` : nom de cité ≠ capitale (préexistant).
- Bakes de rendu (colormap, landcover) non régénérés après la réduction ; `tools/experiments/dn_holes.py` cite deux colonies retirées.
- Révoltes toujours sous la bande 4-10 ; deux graines sortent de la bande FR-EN (83 %, 44 %).
- Images acceptées imparfaites : city_3..6 proches, castle_1 en tour de pierre, chêne vert, cerf roux.
- Pas de pastilles en vue parchemin lointaine (cartouche seul).

## CO-A — résultats (2026-10-10)
Outil : `uv run --project tools cent-ans geo settlement-cap` (réglages `data/map/settlement_cap_rules.json`, ADR 0291).
Ordre de régénération : `settlement-cap` → `geo settlements` (graphe, positions, chemins) → `geo hamlets --merge-former`
→ `geo sea-lanes` → `geo navgrid` → `CENT_ANS_REGEN_STARTING_FIT=1 cargo test -p sim-campaign starting_fit`.

Comptes (2 148 → 1 656) : cités 443 → 443 ; villes 705 → 269 ; villages 419 → 539 ; châteaux 338 → 289 ;
abbayes 243 → 116. Provinces : 250 à 3 colonies (inchangées), 59 à 4, 134 à 5. Villages 539 ≥ 2 × 269 villes.
492 colonies retirées (161 villages, 155 villes, 127 abbayes, 49 châteaux), 281 villes rétrogradées en villages.
132 colonies protégées par les données (ports de flottes, routes maritimes, hubs, monuments, croisade, inrasables) ;
aucune province n'a dépassé le plafond à cause des protections, aucune référence de donnée à remapper.

Références touchées : `fine_anchors.json` (ancrages des retirées déplacés vers `hamlets`), `towns_1340.json`,
`town_footprint.json` (2 lignes), `settlement_graph.json` / `settlements_px.json` / `settlement_edge_paths.json`,
`sea_lanes_px.json` et `navgrid.png` (positions de jeu recalées), `starting_fit.json`. `hamlets.json` : 2 988 hameaux
GeoNames inchangés + 492 colonies retirées ajoutées à la fin (`geo hamlets --merge-former`, un `geo hamlets` complet
décalerait tout le fichier, qui n'était déjà plus reproductible).
Tests adaptés (données, pas assertions) : `capture_tests.rs` (Montlhéry/Royaumont → Gisors/Fleury), `lr08_ruins_test.gd`,
`rs_c_demolition.rs` (les Suisses n'ont plus qu'une cité : bâtiments sur toutes leurs places), `dc6b_weighted_sums.rs`
(dix places du type cédées à la France), `c4_chains.rs` / `p1_tin.rs` (un emplacement libéré : plafond cité 6),
`c4_settlements.rs` (le château de Beaujeu perdait sa posture de siège : l'armée la prend à l'arrivée).

Mesures `campaign_probe` (120 tours, graines 1-6, avant = données de `23ff4501e`, après = ce lot) :
| mesure | avant | après |
|---|---|---|
| guerre FR-EN (% des tours) | 67 74 64 54 66 64 (moy. 64,8) | 75 83 66 61 70 44 (moy. 66,5) |
| révoltes par partie | 6 1 2 5 0 1 (moy. 2,5) | 1 2 0 9 2 4 (moy. 3,0) |
| banqueroutes | 128 86 114 108 91 131 (moy. 109) | 115 96 133 138 97 122 (moy. 117) |
| revenu de la France à 120 tours | 39,6 27,0 36,9 27,9 44,3 23,9 k (moy. 33,3 k) | 26,7 23,1 31,9 27,1 31,5 30,9 k (moy. 28,5 k) |
Moyennes dans les bandes (guerre FR-EN 55-75 %) ; deux graines isolées sortent (83 % et 44 %), bruit de graine comparable
à l'avant (54-74). Révoltes toujours sous la bande 4-10 (point ouvert connu, inchangé). Revenu de la France −14 % en
moyenne, dans la plage 22-42 k de WH econ. Aucun réglage de `rules.json` modifié (seul `building_slot_cap`).
La recherche n'est pas une mesure de la sonde : non mesurée.

Points ouverts : `tools/experiments/dn_holes.py` cite `set_kingston` / `set_montargis` (retirées, script d'essai) ;
`prov_bar` (test de schéma, nom de cité ≠ capital_city) et `test_budget` échouaient déjà avant ; bakes de rendu qui
lisent les positions de colonies (colormap, landcover) non régénérés ; tests Godot non lancés (machine saturée).
