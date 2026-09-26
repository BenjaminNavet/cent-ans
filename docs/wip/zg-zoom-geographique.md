# ZG — carte de campagne zoomable jusqu'à 1-5 m (ADR 0036)

Demande du joueur (25/09) : « voir des montagnes, des vallées, des villes » en zoomant ; paliers
1 (90 m), 2 (30 m) et 3 (1-5 m) validés, « fais tout jusqu'à 3 sans me demander ».
Session orchestratrice « zoom ». Fusion via le worktree `../gp-zoom-merge` (branche
`integration/zoom`), puis ff-only dans `main`. Coût cloud attendu : 0 $ (données ouvertes).

## Conventions
- Étages de pyramide E0-E7 (E0 = `data/map/height/`, 360 m ; Ek = 360/2^k m/px) ; paliers
  joueur 1 (E1-E2), 2 (E3-E4), 3 (E5-E7).
- Cache hors git : `data/map/pyramid/` et `tools/geo/raw/` du dépôt principal
  (`/Users/jean_hubert/dev/game_project/...`). Dans un worktree d'agent : liens symboliques
  vers ces deux dossiers, jamais de second téléchargement.
- Plafonds : bruts ≤ 20 Go, pyramide ≤ 4 Go, VRAM de pages ≤ 256 Mo.
- Fichiers partagés avec d'autres chantiers : `terrain.gdshader` (crochets d'une ligne
  seulement), `campaign_camera.gd`, `terrain_builder.gd`. Branche voisine `feat/map-modes`
  (autre session, modes de carte) : prévenir avant de toucher au shader de terrain au-delà
  d'un crochet.

## Lots
| Lot | Contenu | Vague | État |
|---|---|---|---|
| ZG0 | Squelette (ADR, manifeste, schémas, stubs, wip) | 0 | fait |
| ZG1 | Données paliers 1-2 (`geo pyramid`) | 1 | **fusionné** dans `integration/zoom` (3beab01c) ; pyramide E1-E7 complète, 2,6 Go |
| ZG2 | Moteur : quadtree streamé, patchs GPU | 1 | **fusionné** dans `integration/zoom` (650d77a5) |
| ZG3 | Données palier 3 (`geo detail-dem`, zones, anachronismes) | 1 | **fusionné** dans `integration/zoom` (26ba32fa) : 34 zones E5-E7, 0,22 Go ; à relancer `geo detail-dem --force` après E3-E4 |
| ZG4 | Caméra rapprochée, exagération verticale dynamique | 2 | **dans main** (a3389a92) |
| ZG5a | Hydrographie fine, ancrages et routes drapées (données) | 2 | **dans main** (5caa8def) |
| ZG3b | Correctif du rehaussement des zones E5-E7 (Londres −12 m) | 2 | **dans main** (2d36ec2a) |
| ZG5b | Rendu : rubans de fleuves, routes drapées, parcellaire de près | 2 | **dans main** (c7e9a1c5) |
| ZG6 | Villes ordinaires à l'échelle réelle vers 1340, finage ↔ parcellaire | 3 | **dans main** (7f38c532) |
| ZG4b | Correctifs recette Q3 : sol nu au-dessus des villes emblématiques (plancher provisoire jusqu'à VH4), pont géant sur Londres, pic des ponts | 3 | **dans main** |
| ZG8 | Relief local exagéré façon Total War (visuel seulement), roche selon la pente, ombrage (wip `zg8-relief-exagere.md`) | 3 | **dans main** |
| ZG7a | Perf (MultiMesh par ville, parcellaire, p99, ponts), lit de la Seine, ponts-portes, rives de Londres, PathPreview (wip `zg7a-perf-finitions.md`) | 4 | **dans main** (609bbcdf) |
| ZG7b | Cache absent (message + commande unique), embarquement de la pyramide, docs geo/godot-map, crédits (wip `zg7b-export-cache.md`) | 4 | **dans main** (0eaec278) |
| ZG7c | Recette visuelle aux 3 paliers après ZG7a/b | 4 | en cours (wip `zg7c-recette.md`) |

## Journal
- 25/09 : ZG0 commité (ADR 0036, `data/map/relief_pyramid.json`, `detail_zones.json`, schémas,
  `geo/pyramid.py`, `geo/detail_dem.py`, `relief_pyramid.gd`, `relief_quadtree.gd`).
- 25/09 : vague 1 lancée (ZG1, ZG2, ZG3 en worktrees d'agents ; cache partagé par liens symboliques). Suite : fusion dans `integration/zoom`, puis vague 2 (ZG4 caméra + échelle verticale, ZG5 hydro/routes/parcellaire), vague 3 (ZG6 villes), vague 4 (ZG7 perf/recette/export/docs).
- 25/09 : ZG3 fusionné dans `integration/zoom` (445 tests Python OK). Coupure de quota vers
  10 h : ZG1 arrêté à ~97 % de la cuisson E3-E4 (reprise par l'orchestrateur, commande
  idempotente), ZG2 relancé avec son contexte (décodeur Rust asynchrone non commité).
- Coordination VH (session « villes historiques », ADR 0037) : VH démarre après fusion de ZG2
  et ZG4 et reprend les villes emblématiques en 1:1 ; ajouter à l'ADR 0036 l'addendum « loupe
  en vue stratégique, 1:1 au zoom rapproché (lot VH4) ». ZG6 = villes ordinaires seulement.
- 25/09 : cuisson E3-E4 terminée par l'orchestrateur, ZG1 fusionné (conflit de manifeste résolu : E1-E4 de ZG1, E5-E7 de ZG3). `geo detail-dem --force` relancé dans `../gp-zoom-merge` pour fondre les zones sur E4. ZG5 scindé : ZG5a (données, lancé) et ZG5b (rendu, après ZG2).
- 25/09 : ZG2 fusionné dans `integration/zoom` (conflit d'ADR : addenda ZG1 + ZG2 + addendum « villes emblématiques » pour VH). `main` fusionné dans l'intégration (995bf6f4). Le smoke plante comme sur `main` (« Message queue out of memory », cause étrangère, correction par l'orchestrateur de nuit). **`main` pas encore avancé** : le checkout principal contient des modifications non commitées du lot PB1 (autre session) sur `terrain_builder.gd` etc. ; demande de coordination envoyée. Les agents de la vague 2 partent donc de `integration/zoom`. PF1 (nuit) sera adapté par-dessus ZG après l'avance de `main`.
- Disque presque plein (≈ 25 Go) : compiler avec `CARGO_TARGET_DIR=/Users/jean_hubert/dev/game_project/core/target` dans les worktrees.
- 25/09 : **ZG0-ZG3 dans main** (a8e0f3d3, ff-only) après libération du checkout par PB1 (game-project-b3, qui rebase sur ZG). Prévenus : PB1 et l'orchestrateur de nuit (PF1 branchera les préréglages de qualité sur le quadtree ; recalages incrémentaux de landmark_model = ZG4).
- 25/09 : correctif 3a3c8a95 (quadtree jamais « stable » en vue parchemin, signalé par PB1) ; ZG5a fusionné, **main = 5caa8def**. Anomalie Londres (E5-E7 10-15 m trop bas, côte grossière) → ZG3b. ZG5b lancé en parallèle de ZG4 (ne touche ni caméra ni paliers).
- 25/09 : ZG4 fusionné, **main = a3389a92**, smoke complet OK (28 étapes, après SM1 de la nuit). Défauts visibles de près (captures ZG4) transmis à ZG5b : tranchée de `river_bed.png`, damier de la splat 719 m, texture floue. ZG6 lancé. Prévenus : nuit (PF1, VH peut démarrer), PB1.
- 25/09 : PF1 (nuit) dans main par-dessus ZG4. ZG3b fusionné : cause = base du rehaussement calculée sur GLO-90 (surface, bâti) au lieu de la source fine + pas de plancher de terre ; corrigé, 34 zones recuites, `geo detail-check` ajouté. `hydro-fine` et `anchors-fine` relancés sur le relief corrigé. **main = 2d36ec2a**. Limite : rives basses de Londres plaquées à 0,5 m (plancher) au lieu de 2-5 m.
- 25/09 : PB1 dans main. ZG5b fusionné (lit fin creusé dans les pages du quadtree, rubans de fleuves/routes, ponts à l'échelle, parcellaire ; −10 % i/s, p99 115 → 167 ms sous charge) ; smoke 28 OK, tests ZG2/ZG4/ZG5b OK. À reprendre en ZG7 : lit de la Seine trop large (chenal brun), pic de 35 ms au basculement des ponts, ponts-portes encore exagérés, coût GPU du parcellaire non mesuré (pas d'outil Metal).
- 25/09 : recette Q3 (nuit) : sol beige nu sous 3 unités à Londres (trou entre ZG4/ZG5b/VH), ruban rouge-gris géant sur la Tamise vers 20 unités, arrêt à 7 unités sans cache. → ZG4b lancé ; le cache absent relève de ZG7 (embarquement + message clair).
- 25/09 21h37 : limite de quota levée, ZG4b (aucun commit encore) et ZG6 (dernier 3b499cf4) relancés avec leur contexte.
- 25/09 : ZG6 fusionné (villes ordinaires 1:1 vers 1340, finage raccordé au parcellaire ZG5b), **main = 7f38c532**, tests ZG2/4/5b/6 + smoke OK. Coût : 48 vs 57 i/s, appels de dessin ×3 (MultiMesh par cellule → par ville à faire en ZG7). Suites VH3 : populations 1328, Gand vide au sud.
- 25/09 : ajout de ZG8 à la demande du joueur (capture Total War) : exagérer le relief local, pas l'altitude ; purement visuel, une seule fonction de hauteur affichée. Passe avant la recette ZG7.
- 25/09 : ZG4b et ZG8 fusionnés, plus le correctif C5 (le « ruban géant » de la Tamise était la couche des routes commerciales, affichée hors mode Commerce). Tests ZG2/4/5b/6/8 + smoke OK. ZG4b : brume météo trop couvrante de près (vraie cause du sol beige), parcellaire absent des rives à 0,5 m, plancher caméra 2,6 au-dessus des villes emblématiques (à lever par VH4), portes et bascule des ponts étalée. ZG8 : hauteur affichée = s·(h + g·max(h − fond, 0)), près ×2,5, gains 0,3/0,8 réglés à l'œil (à affiner en jeu), `--no-relief-exaggeration`. Reste à signaler à d'autres lots : `PathPreview` à l'échelle de la carte de près. Prochain : ZG7.
- 25/09 : ZG7 découpé en ZG7a (perf + finitions visuelles) et ZG7b (export, cache, docs, crédits), lancés en parallèle ; recette finale ZG7c ensuite.
- 26/09 : ZG7b fusionné (0eaec278) : avis « relief incomplet » + `cent-ans geo relief-all [--check]`, export `CENT_ANS_EXPORT_RELIEF=bundle|external|none` (clones APFS), `MapPaths.relief_root_for`, docs geo/godot-map/tools, crédits (textures Poly Haven ajoutées). pytest 581 OK, smoke OK. Suites : `ReliefPyramid` ignore un étage entier si sa 1re tuile manque (à rendre robuste en ZG7c) ; aucune archive « Cent Ans relief » hébergée (à décider avant diffusion).
- 26/09 : ZG7a fusionné (609bbcdf) : appels de dessin des villes −43 % (MultiMesh par ville / cellule 1 km, HLOD par instance), parcellaire −20 %, pages et ponts hors du fil principal (fine_install 19 → 1,6 ms), Seine 133-200 m interpolée le long des ancrages, tabliers à largeur réelle, rives de Londres 2,6-3,8 m, PathPreview fin de près. Non atteint : p99 ~130 ms (≈ base), qt_update 10-16 ms. Tests zg2-zg8 + smoke OK. ZG7c lancé : pyramide partielle robuste, recuisson des 34 zones (plancher v4), Tamise −7,8 m, banc, recette visuelle aux 3 paliers, clôture.
