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
| SZ1 | S1 | Haute montagne au palier vallée : exagération ZG4 modulée par l'amplitude locale / plafond de hauteur affichée selon la distance ; caméra hors des canyons | 1 | **dans main** (f8ee130f) : écrasement des montagnes (amplitude > 350 m), relief quasi 1:1 autour des villes v2, garde des crêtes |
| SZ2 | S2 | Recuisson E0-E4 avec plancher monotone des fonds de vallée (continuité E0/E1), puis `detail-dem`, `hydro-fine`, `anchors-fine` ; Loire d'Orléans au niveau des berges. Cuisson dans un dossier de préparation, bascule atomique par l'orchestrateur | 1 | **dans main** + pyramide basculée : plancher `valley_floor` à tous les étages, `BAKE_VERSION` pyramide 2 / détail 5, estampille `bake.json` ; Loire d'Orléans −0,1 m sous les berges |
| VH0+VH4 | S3 | Squelette VH (ADR 0078, 0037 étant pris par la difficulté) et moteur des villes emblématiques 1:1 géoréférencées ; levée du plancher caméra ZG4b | 1 | **dans main** (80fb0004) : ADR 0078, format `data/landmarks_v2/`, `docs/landmarks-v2.md`, Rouen vers 1340 (706 rues, 26 monuments, ≈ 5 800 bâtiments) |
| SZ4 | S4, S5 | Moulins, hameaux, fumées, arbres proches à l'échelle aux paliers intermédiaires ; disque d'emprise d'Amiens | 1 | **dans main** (4b6c1057) : `MapPropScale` (`map_prop_scale.tres`), toits vus de loin sur le sol bâti (`roofscape_*`) |
| SZ4b | suite SZ4 | Maquettes de colonies géantes jusqu'à d ≈ 8 puis bascule brusque vers les villes 1:1 ; densité des forêts au palier vallée | 2 | **dans main** (b91572d6) : maquettes continues vers leur emprise, forêt dense `ForestDetail` (budget 220 000 arbres, primitives ×2 en forêt au palier vallée) |
| SZ5 | S7 | Pluie au palier site (gouttes et stries à l'échelle de la caméra) | 1 | **dans main** (e73abe8d) : `PrecipitationProfile` (`precipitation.tres`), tailles ancrées en mètres de près, identiques à l'ancien au-delà de d = 30 |
| SZ6 | S6 | Pics d'images côté scripts : profilage et étalement (qt_update, recalages, écouteurs) | 1 | **dans main** (26b1d806) : p99 91 → 38 ms, pics > 50 ms 231 → 3 ; `--bench-probe` |
| SZ7 | hébergement | Décision (ADR 0077) + outillage : paquet « Cent Ans relief » découpé, sommes de contrôle, commande de téléchargement | 2 | **dans main** : `relief-pack` / `relief-fetch`, `relief_hosting.json`, avis du jeu ; **publication en attente de l'accord du joueur** (commandes dans `docs/geo.md`) |
| VH5/6/7 | S3 | Paris, Londres, Orléans au format v2 1:1 (Rouen fait par VH4) | 2 | **tous dans main** (Paris : 22a953e6, rues et parcelles ALPAGE, 119 monuments, 4 ponts) |
| SZ2b | suite SZ2 | Fleuves au palier site : lit sableux sans nappe d'eau (Seine à Rouen) | 2 | rendu : eau coupée sous les emprises de colonies + relief décalé d'un demi-pixel (ADR 0086) ; **dans main**, vérifié combiné à SZ1 (captures `docs/img/sz2b/combo_*`) |

## Journal
- 26/09 : plan écrit ; vague 1 lancée (6 agents).
- 26/09 : SZ5 fusionné (e73abe8d). Limite : pluie peu visible si un grand bâtiment occupe le point visé. Coordination PB3 (autre session) : ADR 0079-0081 réservés par PB3, SZ6 prendra 0082 ; PB3g (quadtree en Rust) attend SZ6.

- 26/09 : SZ4 fusionné (4b6c1057). Limites : maquettes de colonies géantes jusqu'à d ≈ 8 puis bascule, forêts clairsemées au palier vallée → SZ4b. SZ7 (outillage) lancé.
- 26/09 : SZ2 fusionné et pyramide basculée (`relief-all --check` complet, 2,69 Go). Limites : altitudes absolues 2-6 m sous le réel (GLO-30), bourrelets E4 du Val de Loire au palier site, contraste E0 un peu réduit, `horizon.py` non recuit, `detail-check` p95 > 5 m sur 31 zones (écart de source, amélioré partout). Agents en cours prévenus de fusionner `main` (E0 recuit).
- 26/09 : SZ7 fusionné (outillage, 12 tests hors réseau, pytest complet 707 OK). Rien publié.
- 26/09 : SZ6 fusionné. Causes : rubans de routes drapés (fils), bascule de maillage d'un MultiMesh rempli par `buffer` (relecture GPU, 86 ms), recuisson des maquettes, changements de niveau des tronçons étalés (4 ms/image), étiquettes, index par tronçon. Reste pour PB3/PB3g : pas du quadtree 9-12 ms, `request_reground` de la végétation (copie des pages vers Rust, 70 ms), `NextHintController.refresh` 15 ms/s, `TownLayer` 10 ms.
- 26/09 : VH0+VH4 fusionnés (80fb0004), tests vh4/sz4/sz6/zg4/zg6 + smoke OK sur l'intégration. Défauts vus sur les captures de Rouen : Seine sans nappe d'eau au palier site (→ SZ2b), côtes en murs (SZ1 en cours). Correctif de sécurité SZ7 (noms de parts du manifeste, 5c1c4dc9). Vague 2 : VH5 Paris, VH6 Londres, VH7 Orléans, SZ2b.
- 26/09 : SZ1, VH6 (Londres), VH7 (Orléans) fusionnés (f8ee130f), conflits ponts/tests résolus (piles par arches VH7 + avant-becs et rampes VH6). Incident : ff refusé (revue de code arrivée dans main) mais dylib installée quand même pendant ~10 min ; main réintégré, dylib reconstruite, tests OK, puis ff. La dylib release de main reste celle d'avant (08:32).
- 26/09 : SZ2b fusionné par-dessus SZ1 (dylib reconstruite, tests sz1/zg5b/zg8/zg2/vh4 + smoke OK, captures combinées Rouen, Orléans, Londres correctes). Suites : bande sèche le long d'une rive (largeurs `river_widths.json`), maisons de faubourg au bord de l'eau à Orléans/Tours (`towns_1340.json` sans couloir de fleuve).
- 26/09 : VH5 Paris fusionné par l'orchestrateur (accord du joueur : l'agent était bloqué par le filtre de permissions pendant la fusion). Conflits résolus : plusieurs ponts par ville (`plan.bridges`) avec les ajouts VH6 (piles, avant-becs, chapelle, pont-levis, rampes via `_bridge_levels`) et VH7 (piles selon `arches`, tablier sur les piles) ; un seul recalage des ponts dans `LandmarkPlan.reground` ; un seul index d'eau (`_water_index` VH5, cases de 40 m, polygones). Tests vh4 (Rouen 5 838, Londres 7 076, Orléans 2 590, Paris), zg4, zg6, zg5b, smoke, pytest OK ; Seine visible autour de la Cité (`docs/img/vh5/`). main = 22a953e6.
- 26/09 : **pause demandée par le joueur.** SZ4b arrêté proprement (dernier commit 36d97138, worktree verrouillé).
- 26/09 : reprise demandée par le joueur ; agent SZ4b relancé dans son worktree (fusion de main, cible cargo privée, captures, mesures, tests, doc).

- 26/09 : SZ4b fusionné (b91572d6, dylib installée). `sz1_mountain_test` adapté : Paris est une ville 1:1, relief à l'échelle vraie. Relecture historienne (accord du joueur) : Paris (13 confirmés / 11 corrigés / 5 incertains, `docs/histoire/relecture-vh-paris.md`) et Rouen (14/10/7, enceinte coupée en 1345/1346, `relecture-vh-rouen.md`) fusionnés ; Londres et Orléans en cours. Publication du relief : accord du joueur, mais empaquetage refusé par le filtre de permissions (« Create Public Surface ») ; commandes données au joueur.

- 26/09 : relectures historiennes de Londres (23 confirmés / 20 corrigés / 6 incertains : London Bridge à l'ouest de St Magnus, mur recalé sur Historic England, Old St Paul's) et d'Orléans (12/12/11 : porte Renart, Sainte-Croix romane encore debout, boulevards 1417, pont recalé) fusionnées (f83b00bd). Les quatre rapports sont dans `docs/histoire/relecture-vh-*.md`. Question laissée au joueur : accrue d'Orléans en 1345 (gardée) ou 1391 (Carron et Guillemard 2012).

## Prochaine étape (reprise)
1. **SZ4b** : worktree `.claude/worktrees/agent-af6b91aaabd8c3484`, branche `feat/sz4b-colonies-forets`
   (36d97138, `main` fusionnée jusqu'à 82100583). Code fait (maquettes continues, masquage par
   colonie, forêt dense `ForestDetail` + `DetailArea` Rust). Reste, d'après
   `docs/wip/sz4b-colonies-forets.md` : fusionner `main` (22a953e6), recompiler la dylib (cible
   privée : la cible partagée `core/target` est polluée par d'autres worktrees), captures avant/après
   (`tests/sz4b_shots.gd`), mesures i/s et instances, tests zg6_towns, cv1_campaign_life,
   settlements_render, sz4_prop_scale, smoke, cargo test, doc `godot-map.md`. Puis fusion via
   `../gp-sz-merge` (branche `integration/sz`), ff-only dans main, installer la dylib dans
   `game/bin` de main (rm puis cp) seulement si le ff a réussi (`set -o pipefail`).
2. **Publication du relief** (ADR 0077, SZ7) : attend l'accord explicite du joueur (commandes dans
   `docs/geo.md`).
3. ~~Relecture historienne~~ faite le 26/09 (rapports `docs/histoire/relecture-vh-*.md`) ; points incertains listés dans chaque rapport. Ancien intitulé : faits marqués `probable`/`hypothetical` de Rouen (Martainville, porte
   Saint-Hilaire), Londres (mur vers Aldersgate, Old St Paul's, tablier du pont, dates Westminster),
   Orléans (accrue 1345, portes, Saint-Aignan, boulevards 1404), Paris (Grand-Pont 1340, flèche de
   Notre-Dame, Charles V, noms ALPAGE des portes) ; listes dans `docs/wip/vh4…vh7*.md`.
4. Suites non lancées : VH8 (Bordeaux, Avignon, Calais, Bruges en v2 ; fusion des monuments par
   cellule pour les i/s de Paris), bande sèche le long d'une rive (`river_widths.json`), couloir de
   fleuve dans `towns_1340.json` (Orléans, Tours), bosses GLO-30 dans la Cité de Londres, pics
   restants pour PB3 (quadtree 9-12 ms, `request_reground` 70 ms, `NextHintController` 15 ms/s),
   dylib release de main à reconstruire (`core/build.sh --release`).
