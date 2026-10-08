# RS — Restes des wip (orchestration, 28/09) — fichier de reprise

Demande du joueur (28/09) : « tu es un orchestrateur et tu vas attaquer les wip restants ».
Tri fait sur `main` 2c475b06 (agent Explore) : 20 points ouverts, faisables sans le joueur.
Exclus : parties pilotes, test sur vrai PC Windows, écoutes, publication du relief, dépenses cloud,
FXAA, retrait de `--coarse-figures` (point 17, choix du joueur).

## Décisions du joueur en attente (30/09, chantier NT)

Mocap payante, clé fal.ai (GA3), chantier guerre civile / prétendants : détail et pistes d'achat dans
`docs/archive/chantiers.md` § « Pour le joueur ».

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

Suites du 29/09 (toutes dans `main`) : O (test C2 de l'avant-bataille durci ; correctif déjà fait par 65dd4452),
P (icône « Raser » déjà faite, 982dec2b, 0,05 $), K3 (`settle/labels` p99 8,9 → 1,8 ms, `declutter` 16,7 → 5,8,
`map.settlements` 19,4 → 8,0), correctif IB2 de `diplomacy_panel.gd`. Écrans de siège en bataille : T4 dans main,
aucune taille en dur restante ; la session NT (sièges) les reprend, rien à faire côté RS. **RS clos (30/09).**
Reste hors RS : FE F8. `da7d_overlap_test` (seuil 4 ms) échoue sous charge, base comprise.

Suites possibles (état au 28/09 soir) :
- ~~`PreBattleDialog._layout()` déborde à 1280×640~~ — FAIT 28/09 (65dd4452) : colonnes et conditions défilantes,
  bannière réduite sous 760 px, re-layout au changement de minimum ; C2 de `p2d_ui_test` bloquant.
- ~~Icône dédiée « Raser »~~ — FAIT 28/09 : `act_raze` (`raze.png`, 0,04 $).
- Écrans de siège en bataille (`battle_siege.gd`, points de capture) après TW2 T4.
- Pics carte restants ~8 ms (`settle/labels`, `settle/declutter`), `docs/archive/chantiers.md`.
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
  2 saisons). Wip : `docs/archive/chantiers.md` (chiffres avant/après). À vérifier avant fusion : sonde release
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

## Restes issus des chantiers archivés (ménage SC, 2026-10-08)

Détail dans `docs/archive/chantiers.md` ; texte complet via `git log -- docs/wip/<x>.md`.

### f5a-battle-sim
- 5. [ ] Renforts échelonnés au-delà de 20 régiments : **non fait**.

### c1-settlements-skeleton
- Smoke Godot : échec **préexistant** (reproduit sur 64dec0b sans ce lot) :
- La garnison de la cité est vide dans `SettlementState` (la garnison de province reste la vraie
- Règle « au moins un port par province côtière » (§ 3.1) non vérifiée par le test Python (non

### c4-edits-chaines
- Données de colonies d'avant C4 : plusieurs paliers d'une même chaîne restent
- Pas de condition de taille de colonie au-delà de `settlement_kinds` et des
- L'arbre de construction reste une liste triée (rang affiché, prérequis en

### p2a-court
- **Choix pris pour ce lot (mécanique, sans trancher la question) :** `CourtPanel` et
- **À trancher plus tard** (orchestrateur / PO6b) : soit modifier `map_ui.gd` pour réclamer

### dp1-diplomatie
- Graines 4 (48 %) et 3 (52 %) du siècle sous la cible : cause non analysée (`dp1_probe` affiche
- L'accès militaire ne joue que sur le ravitaillement (pas de règle d'intrusion en paix).
- Accord commercial autonome : à relier à C5 (`integration/tw`) quand il sera sur main.

### tw2-t3
- La surprime n'apparaît pas dans le budget de l'interface (`economy.rs`, lot RS) : panneau et journal.
- Pas d'illustration propre pour les deux nouvelles unités (icônes recadrées).
- Pistes : compagnies allemandes en Italie, gallowglass, licenciement volontaire d'une compagnie.

### ub1-interface-bataille
- Prévision d'équilibre = estimation simple (mêmes pièces que l'auto-résolution actuelle, sans
- Retraite avant bataille : seul l'assaillant peut refuser (−10 moral) ; un assaut remis garde le
- Butin : aucune règle de butin en bataille rangée ; l'encart montre les rançons à percevoir.

### zg3-palier3
- Le plancher `MIN_LAND_M` reste une valeur plate (0,5 m) là où le rehaussement creuse fort
- Le seuil `LAND_GAP_ALERT_M` (5 m) déclenche sur la quasi-totalité des zones : attendu vu leur

### zg7a-perf-finitions
- Le relief E1-E4 (ZG1) plaque les fonds de vallée proches de plateaux à 0,5 m (rehaussement de
- Tamise fine : niveau d'eau -7,8 m à Londres (PAVA mêlé à la bathymétrie de l'estuaire).

### epic
- Équilibrage : avec R4, le défenseur gagne à forces égales à l'échelle épique (graines 3, 5, 11) ;
- `--standard-shot=foot` cadre parfois dans une pile de pont sur un site EP3 : décaler la caméra.
- `battle_skinned_poses.py` : défauts ruff (docstrings) antérieurs, venus d'EP5.

### pb3c-bataille-cpu
- `get_units` et les tampons renvoyés entre deux pas sont partagés : lecture seule côté GDScript.
- `_find_braced` laissé tel quel (déjà O(n) hors charge de cavalerie).
- Pistes suivantes : `soldiers.update` (~3,3 ms, boucles GDScript), étendards 1,7 ms, audio

### vh6-londres
- Tracé du mur entre Newgate et Aldersgate et position d'Aldersgate ; point de départ à la Tour.
- Position et axe d'Old St Paul's (−4° grille, centre 15 m à l'est de la cathédrale de Wren),
- London Bridge : extrémités (à l'est de St Magnus), hauteur du tablier (5,5 m au-dessus de

### vh7-orleans
- **Butte** : le relief réel monte de 20-25 m entre la Loire (87 m) et la cathédrale (115 m, RGE
- **Loire sans nappe d'eau** au palier site (SZ2b en cours) : le pont franchit un pré ; les quais et
- Entre le mur de Loire (47,8983 N) et l'eau fine, une bande de 50-80 m (quais du XVIIIe s. gagnés

### fe6-ui
- Bretagne, Flandre, Navarre (départs recommandés de la spec) ne sont pas jouables dans les données

### fk3-folk
- Trajets rectilignes en boucle (pas de suivi de polyligne en shader) : une charrette parcourt un
- Clés lues dans `map_scenes.json` (premier niveau, `folk` ou `densities`) : `pool_cap`,

### fk5-incidents
- Pictogramme unique (`hud_chronicle_decision`) pour tous les incidents : un pictogramme par
- L'avis d'expiration repère l'entrée de chronique par la marque « (délai écoulé) » du cœur
- `--no-folk` coupe aussi les sceaux (repli sur la fenêtre de début de tour).

### nt1-sieges
- Bourg moins dense que la cité (≈ 40 îlots) : à juger visuellement, réglable dans `places.borough`.
- Donjon rendu en tour ronde agrandie (pas de maquette de donjon carré dans le kit).
- Équilibre d'un assaut de château (petite enceinte, garnison serrée) à surveiller en partie pilote.

### nt11
- Escalier et palier hors de l'emprise de la simulation (rendu seul, ~3,4 m devant la façade).
- Pas de capture faite (consigne) : escalier et terrasse à juger à l'œil (`nt1_siege_shot.gd`).
- Dans le prologue, l'ennemi peut toujours se débander sous la charge et les flèches.

### omr-r4
- Kholmogory (1355), Kotelnitch, Vychni Volotchek (cité au XVe s.) : gardés comme localités
- Sozopolis : bulgare ou byzantine en 1337 (laissée avec Anchialos, bulgare).
- Mourom rattachée à Souzdal (principauté de Mourom autonome), Tchernigov à Briansk.

### tf
- Atlas `Building` plein : une nouvelle matière demandera 32 tranches.
- Maquettes de colonies CV1 et monuments non réexportés (panneaux `Plaster`).
- Toits du Midi : seules les variantes `southern` ont des tuiles canal ; longères, granges et

### vn
- 147 factions sans miniature d'encyclopédie (~12 $ en fal.ai) + bld_collegiate_church : non fait
- Tours de siège énormes (rayon 5+fortif m, cœur `siege_layouts.rs`) : règle du cœur, session
- `q6_ui_test` (SimFacade introuvable avec --script) et `fe_ui_test` échouent aussi sur main.

