# SZ — suites du zoom géographique (après clôture de ZG, ADR 0036)

Demande du joueur (26/09) : traiter les 7 imperfections laissées par ZG7c, « en toute autonomie ».
Orchestrateur : session `game-project-d8`. Coût cloud attendu : 0 $ (calcul local, données ouvertes).
Référence des défauts : `docs/wip/zg7c-recette.md` (tableau « défauts laissés », captures
`docs/img/zg7c/`).

## Conventions (reprises de ZG)
- Worktrees d'agents depuis `main`. Liens symboliques non versionnés vers le cache du dépôt
  principal : `data/map/pyramid`, `tools/geo/raw` (jamais de second téléchargement).
- Compiler avec `CARGO_TARGET_DIR=/Users/jean_hubert/dev/game_project/core/target`
  (disque à 94 %). Les agents « données » ne compilent pas.
- Commits avec chemins explicites (`git commit -- chemins`), `wip:` au plus tard toutes les 15 min.
- Fusion par l'orchestrateur, ff-only dans `main` (fusion préparée hors du checkout principal).
- `terrain.gdshader` : conserver l'`#include` FR1 (`faction_borders.gdshaderinc`).

## Lots
| Lot | Défaut (ZG7c) | Contenu | Vague | État |
|---|---|---|---|---|
| SZ1 | S1 | Haute montagne au palier vallée : exagération ZG4 modulée par l'amplitude locale / plafond de hauteur affichée selon la distance ; caméra hors des canyons | 1 | lancé |
| SZ2 | S2 | Recuisson E0-E4 avec plancher monotone des fonds de vallée (continuité E0/E1), puis `detail-dem`, `hydro-fine`, `anchors-fine` ; Loire d'Orléans au niveau des berges. Cuisson dans un dossier de préparation, bascule atomique par l'orchestrateur | 1 | **dans main** + pyramide basculée : plancher `valley_floor` à tous les étages, `BAKE_VERSION` pyramide 2 / détail 5, estampille `bake.json` ; Loire d'Orléans −0,1 m sous les berges |
| VH0+VH4 | S3 | Squelette VH (ADR 0078, 0037 étant pris par la difficulté) et moteur des villes emblématiques 1:1 géoréférencées ; levée du plancher caméra ZG4b | 1 | lancé |
| SZ4 | S4, S5 | Moulins, hameaux, fumées, arbres proches à l'échelle aux paliers intermédiaires ; disque d'emprise d'Amiens | 1 | **dans main** (4b6c1057) : `MapPropScale` (`map_prop_scale.tres`), toits vus de loin sur le sol bâti (`roofscape_*`) |
| SZ4b | suite SZ4 | Maquettes de colonies géantes jusqu'à d ≈ 8 puis bascule brusque vers les villes 1:1 ; densité des forêts au palier vallée | 2 | lancé |
| SZ5 | S7 | Pluie au palier site (gouttes et stries à l'échelle de la caméra) | 1 | **dans main** (e73abe8d) : `PrecipitationProfile` (`precipitation.tres`), tailles ancrées en mètres de près, identiques à l'ancien au-delà de d = 30 |
| SZ6 | S6 | Pics d'images côté scripts : profilage et étalement (qt_update, recalages, écouteurs) | 1 | lancé |
| SZ7 | hébergement | Décision (ADR 0077) + outillage : paquet « Cent Ans relief » découpé, sommes de contrôle, commande de téléchargement | 2 | **dans main** : `relief-pack` / `relief-fetch`, `relief_hosting.json`, avis du jeu ; **publication en attente de l'accord du joueur** (commandes dans `docs/geo.md`) |
| VH5/6/7 + Rouen | S3 | Paris, Londres, Orléans, Rouen au format v2 1:1 | 2 | après VH4 |

## Journal
- 26/09 : plan écrit ; vague 1 lancée (6 agents).
- 26/09 : SZ5 fusionné (e73abe8d). Limite : pluie peu visible si un grand bâtiment occupe le point visé. Coordination PB3 (autre session) : ADR 0079-0081 réservés par PB3, SZ6 prendra 0082 ; PB3g (quadtree en Rust) attend SZ6.

## Prochaine étape
Suivre la vague 1 ; fusionner lot par lot ; bascule de la pyramide SZ2 quand aucun autre agent ne
lit la pyramide en écriture ; puis vague 2 (villes VH5-VH7 + Rouen, outillage SZ7).

- 26/09 : SZ4 fusionné (4b6c1057). Limites : maquettes de colonies géantes jusqu'à d ≈ 8 puis bascule, forêts clairsemées au palier vallée → SZ4b. SZ7 (outillage) lancé.
- 26/09 : SZ2 fusionné et pyramide basculée (`relief-all --check` complet, 2,69 Go). Limites : altitudes absolues 2-6 m sous le réel (GLO-30), bourrelets E4 du Val de Loire au palier site, contraste E0 un peu réduit, `horizon.py` non recuit, `detail-check` p95 > 5 m sur 31 zones (écart de source, amélioré partout). Agents en cours prévenus de fusionner `main` (E0 recuit).
- 26/09 : SZ7 fusionné (outillage, 12 tests hors réseau, pytest complet 707 OK). Rien publié.
