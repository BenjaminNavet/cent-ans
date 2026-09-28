# RS — Restes des wip (orchestration, 28/09) — fichier de reprise

Demande du joueur (28/09) : « tu es un orchestrateur et tu vas attaquer les wip restants ».
Tri fait sur `main` 2c475b06 (agent Explore) : 20 points ouverts, faisables sans le joueur.
Exclus : parties pilotes, test sur vrai PC Windows, écoutes, publication du relief, dépenses cloud,
FXAA, retrait de `--coarse-figures` (point 17, choix du joueur).

## Reprendre

Lire ce fichier, `git worktree list`, `git branch --list 'feat/rs-*'`. Chaque lot a son
`docs/wip/rs-<lot>.md`. Fusion : worktree `../gp-rs-merge` (branche `integration/rs`), puis ff-only
vers `main`. ADR réservés : **0100** (lot B ; 0098 pris par FE), **0099** (lot F). GA réserve 0104-0106.

## TERMINÉ (28/09 soir)

Tous les lots RS sont dans `main` (dernier : f93c2359). Dernière vague : **P2f** (74 tailles en dur → `UiType` ;
bannière de fin 40 px gardée), **P2d** (sort de la ville prise en `MODAL`, corps défilant ; avant-bataille et
naval en `UiType`), **N** (bouton « Raser » + `settlement_demolition_preview` au pont, confirmation modale,
infobulle IB `raze_building`). Conflits : P2d × P2f (`chronicle_window.gd`), P2f × IB2 (3 fichiers :
infobulle IB + taille `UiType`), N × règle IB2 (plus de `tooltip_text` littéral).
Worktree `../gp-rs-merge` et branche `integration/rs` supprimés.

Suites possibles (non lancées) :
- `PreBattleDialog._layout()` : déborde à 1280×640 avec les grosses armées de début (défaut préexistant, domaine CB ;
  colonnes de régiments à rendre défilantes) — voir `docs/wip/p2d-sieges.md`.
- Icône dédiée « Raser » (reprend `act_cancel_build`).
- Écrans de siège en bataille (`battle_siege.gd`, points de capture) après TW2 T4.
- Pics carte restants ~8 ms (`settle/labels`, `settle/declutter`), `docs/wip/rs-k2-perf.md`.
- FE F8 : ost d'Empire sans distance, banqueroutes ×20 ; remonter le plancher c7a (ADR 0113) si l'ost est restreint.

## Reprise (28/09 ~15 h)

- P2c **fusionné** (67171a61) : smoke, `p2c_ui_test`, `po_ui_test` verts.
- B : agent de reprise dans son worktree (century_probe 4 niveaux, 2 × 75), puis intégration.
- **B fusionné** (ADR 0100 acceptée ; century_probe 4 niveaux dans la bande, voir `rs-b-order.md`).
  Conflit sémantique B × TW2 (`capture.rs` appelait `province_income` sans règles) : main ne compilait
  plus, corrigé 38954b4f.
- **P2g fusionné** (48f1a5f8) : Tech/Cour/Fiche/Diplo/SaveLoadDialog en zone `MODAL` ; le voile couvre
  la barre du haut (bible § 12.1). Incident : `q3_playtest.gd` lancé par l'agent a réécrit le
  `settings.cfg` du joueur (`advisor_seen` vidé).
- En cours : **C** (`feat/rs-c-diplo`, plafond par motif, démolition IA), **L** (`feat/rs-l-alerts`,
  champ de siège au pont pour `alerts.gd`). P2d attend TW2 T4 (UI de capture).
- **K fusionné** (perf carte : `town/models` 25 → 0 ms, images > 50 ms 106 → 87), **L fusionné**
  (champs `siege_*` dans `get_provinces_snapshot`, `alerts.gd` en lecture groupée), **C fusionné**
  (5aae540f, ADR **0111** renumérotée : plafond d'opinion par motif, ordre `Demolish` + IA en déficit).
  Conflit sémantique C × FE (`feudal/transfer.rs`, champ `deficit_seasons`) corrigé à l'intégration.
  `sz4*_test` exigent `data/map/pyramid` (ignoré par git) : liens symboliques dans les worktrees.
- En cours : **M** (`feat/rs-m-c7a`, sonde c7a rouge sur main : France 34 286 < 40 000 depuis TW2/FE),
  **K2** (`feat/rs-k2-perf`, `settle/*` et `life/*` au zoom). En attente : P2d (TW2 T4), P2f (après IB).
- **K2 fusionné** (c18158c9 : images > 50 ms 50 → 1, pire 123 → 52 ms), **M fusionné** (2fda948b, ADR
  **0113** : pas de bogue, plancher du trésor France de la sonde c7a 40 000 → 15 000 ; causes : rançons du roi
  plus fréquentes avec TW2, ost d'Empire FE avec vassaux italiens ; banqueroutes century 1,09 vs 0,05 —
  signalé à FE pour F8).
- Reste : P2d (attend TW2 T4), P2f (attend IB). Suites facultatives : bouton « Raser » (UI), pics
  `settle/labels`/`declutter` ~8 ms.
- Vague 3 lancée : **P2g** (`feat/p2g-layout`, Tech/Diplo/Cour/Fiche/SaveLoadDialog dans `UiLayout`),
  **K perf** (`feat/rs-k-perf`, `TownLayer` ~10 ms, `qt/collect`). `alerts.gd` (champ de siège au pont)
  attend TW2 SB (même zone) ; C après B ; P2d après TW2 SB ; P2f en dernier.

## PAUSE (28/09 ~09 h 15, demande du joueur) — comment reprendre

`main` = fec99a04 + ce commit : lots A, D, E, F, G, H, J, P2a, P2b, P2e fusionnés. Deux lots arrêtés, tout commité :

- **B Ordre public** — `feat/rs-b-order` (worktree `.claude/worktrees/agent-a90b4dd66bed1d599`, cible privée
  `core/target-rs-b` à supprimer après fusion), dernier commit ae5d94f1. ADR **0100** écrit (seuil de révolte 75,
  2 saisons). Wip : `docs/wip/rs-b-order.md` (chiffres avant/après). À vérifier avant fusion : sonde release
  `fifty_turns_on_eight_seeds_stay_in_the_c7a_band` (trésor France 38 297 < 40 000 avant B), critères eq6 (guerre
  55-75 % à chaque difficulté), test cv3 passé de la graine 7 à 4 (la 7 n'embusque plus). Puis intégration complète.
- **P2c Codex** — `feat/p2c-codex` (worktree `.claude/worktrees/agent-a90707317064d725a`), dernier commit 6f9a2e32
  (main fusionnée, planche faite). Reste : `smoke.gd` + `p2c_ui_test.gd` avec la dylib de `main`, puis fusion.
  Cause du C2 trouvée : centrage manuel résiduel de `codex_window.gd::open()` retiré.

Fusion : worktree `../gp-rs-merge` (branche `integration/rs`), `git merge main` puis les lots ; conflits déjà vus
(FE0 × data-model). Tests d'écran : appeler `Settings.use_test_file()` + `_apply_ui_scale()` avant le premier écran
(le `settings.cfg` du joueur a `ui_size=1.25`).

## Vague 1 (6 agents, fichiers disjoints)

| Lot | Points | Agent | Branche | État |
|---|---|---|---|---|
| A Codex | H10 lots 7-8 (8 fiches), fiche Difficulté | mech | `feat/rs-a-codex` | **fusionné** (d5913850) |
| B Ordre public | constantes économiques → `economy.json` ; pondération `province_effect_percent` (garnison, peste, revenu IA) ; révoltes vers 4-10/partie | dev | `feat/rs-b-order` | lancé |
| D Commerce + tests | chemins commerciaux précalculés ; tests campagne 4/13/14/15 (revue-code) | dev | `feat/rs-d-trade` | **fusionné** (9b1ee0bb), ×39 sur `trade_routes` |
| E Carte | lectures groupées diplomacy_panel/alerts ; légende frontières ; refresh NextHint sur événement | mech | `feat/rs-e-map` | **fusionné** (d5913850) |
| F Bataille | bouton « incendier » (S2) + règles de feu lues depuis `data/` ; sélecteur de formations repliable | dev | `feat/rs-f-battle` | **fusionné** (9b1ee0bb), ADR 0099 ; + correctif `leader_orders_bar.give()` |
| H Nettoyage | événement `movement` du mock ; bug du grand livre `budget.py` | mech | `feat/rs-h-cleanup` | **fusionné** (d5913850) |

Note E : `alerts.gd` garde une lecture par province (détail du siège absent de `get_provinces_snapshot`) : demande un champ côté pont, à faire avec un lot core.

## Vague 2 (lancée 28/09 matin, plafond relevé à 10 agents par le joueur)

| Lot | Contenu | Branche | État |
|---|---|---|---|
| P2a | cour, fiche perso, arbre familial | `feat/p2a-court` | **fusionné** (9b1ee0bb) ; cour et fiche restent hors `UiLayout` (même cause que P2b) |
| P2b | techniques, diplomatie | `feat/p2b-tech-diplo` | **fusionné** (9b1ee0bb) — fini (9ceabff1) ; Tech/Diplo restent hors `UiLayout` (reparentage casse `map_ui._keep_on_screen`) |
| P2c | codex, encyclopédie, infobulles | `feat/p2c-codex` | **fusionné** (67171a61), codex en `MODAL` |
| P2e | menus secondaires | `feat/p2e-menus` | **fusionné** (9b1ee0bb) — fini (816c775b) ; `SaveLoadDialog` de `start_menu`/`map_ui` pas en zone |
| G | Bordeaux, Avignon, Calais, Bruges ; rives | `feat/rs-g-cities` | **fusionné** (9b1ee0bb) ; recuisson par le joueur |
| J | pavois face à une cible cachée (test ignoré de D) | `feat/rs-j-pavise` | **fusionné** (9b1ee0bb), sondes ep7/ep9b/eq7 identiques |

Suites notées : sonde c7a (trésor France 38 297 < 40 000) confiée à B ; `alerts.gd` (champ de siège au pont) ;
P2g éventuel : Tech/Diplo et `SaveLoadDialog` dans `UiLayout` via `map_ui`/`PanelStack` ; P2d sièges après TW2 SB ; P2f après tous les P2.

Intégration 28/09 ~08 h 40 : conflits `data-model/lib.rs` (D × FE0) et `review_fixes.rs` (J) résolus ;
tableau FE de `docs/budget.md` mis au format du grand livre (6 colonnes, sinon `budget.py` lève) ;
`po_ui_test` et `cb6_group_formation_test` passent aux réglages de test (le `settings.cfg` du joueur a
`ui_size=1.25` et faussait les tests d'écran). dylib de `main` rafraîchie.
G : recuisson à lancer par le joueur (`cent-ans geo relief-all --check`, `geo relief-all`, `geo towns`,
`geo landmarks`), voir `docs/wip/rs-g-cities.md`.

## Restant (vague 3)

- C Diplomatie/IA : plafond du bonus Mariage, démolition par l'IA (banqueroutes) — après B (sondes communes).
- G Villes/fleuves : VH8 (Bordeaux, Avignon, Calais, Bruges), rives et couloirs de fleuve.
- I PO phase 2 (P2a-f) : après E et F (fichiers d'UI communs).
- Perf scripts restants (TownLayer, étendards, audio) ; suites EP (seules, marges minces).
