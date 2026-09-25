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
| ZG1 | Données paliers 1-2 (`geo pyramid`) | 1 | en cours (wip `zg1-pyramide.md`) |
| ZG2 | Moteur : quadtree streamé, patchs GPU | 1 | en cours (wip `zg2-quadtree.md`) |
| ZG3 | Données palier 3 (`geo detail-dem`, zones, anachronismes) | 1 | **fusionné** dans `integration/zoom` (26ba32fa) : 34 zones E5-E7, 0,22 Go ; à relancer `geo detail-dem --force` après E3-E4 |
| ZG4 | Caméra rapprochée, exagération verticale dynamique | 2 | — |
| ZG5 | Hydrographie, côtes, routes, colonies recollées, parcellaire | 2 | — |
| ZG6 | Villes à l'échelle réelle vers 1340 | 3 | — |
| ZG7 | Perf, recette aux 3 paliers, export, docs, crédits | 4 | — |

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
