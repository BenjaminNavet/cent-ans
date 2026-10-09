# SC — simplification de la base de code (2026-10-08)

Demande du joueur : grand chantier de simplification (moins de lignes, structure, commentaires de
fonctions, doc, design patterns, performance ; passage en Rust des portions GDScript coûteuses).
Mandat (réponses du 08/10) : mécaniques **libres** (simplifier/supprimer si le code y gagne, ADR
à chaque fois) ; supprimer outils one-shot et sondes obsolètes, condenser les wip clos, garder
les ADR ; **20 agents Sonnet par vague** ; toucher aussi les fichiers de la session FL et les
modifs non commitées de main ; **push** en fin de chantier. Coût cloud : 0 $.

Worktree `../gp-sc`, branche `feat/sc` (dylib copiée de main, `data/map/pyramid` en lien).
Modifs non commitées de main au départ sauvegardées dans le scratchpad (`main-dirty.patch`).

## Taille de départ (lignes)
Rust 188 773 · GDScript scripts 124 976 + tests 50 382 · shaders 13 269 · Python 107 987 ·
docs 58 940 (508 notes wip, 163 ADR).

## Méthode
- Vague 0 : audit en lecture seule par zone → lots chiffrés (gain de lignes, risque, vérif).
- Vagues suivantes : un agent `cent-ans-mech` par lot, worktree isolé, ≤ 5 lots Rust
  simultanés (disque : une cible cargo par worktree), commits sur branche, fusion ff-only ici.
- Vérification : `cargo test`, `cargo clippy -D warnings`, pytest ; Godot seulement en headless (scripts de test ciblés par les agents, `smoke.gd` complet aux fusions). **Jamais de lancement fenêtré du jeu ni de capture** (demande du joueur 08/10).

## Vague 3 (patterns) lancée

Vague 2 entièrement fusionnée (battledev, hooks, uikit, audio, testsupport, schemadefs, names, mock, simsplit). Vague 3 : uipatterns (PanelSection, ListMenu, ConfirmDialog), bridge (ctx accessor, convert, GB3/GB4), treaty (Proposal → Treaty{articles}, ADR 0188 si mécanique), rulesdata (Default depuis JSON embarqué, bundled rules), battleperf (Arc fork, grille spatiale, geom), effects (EffectKind, effects.rs, setup_1337, TurnBudget unique, ADR 0189 si mécanique).

## Consigne ajoutée (08/10)

Le propriétaire précise : le chantier n'est pas que de la suppression ; il faut optimiser et appliquer des design patterns. À partir de la vague 3, chaque lot introduit une abstraction (classe de base, trait, table de données, builder, stratégie) et y fait passer tous les sites ; les lots de pure suppression passent après.

## État
- [x] Vague 0 audit (18/20 zones ; MS et DT en cours) → `docs/wip/sc/lots.md` (catalogue des lots par zone)
- [ ] Vague 1 (lancée 10-08) : worktrees `../gp-sc-<lot>`, branches `sc/<lot>` : probes, naval (ADR 0201), simsplit, names, deadfuncs, gtshots, docs, mock, jsondata, tldel, tlschemas
- Vague 1 fusionnée dans feat/sc : docs, probes, deadfuncs, naval (ADR 0201), tlschemas, gtshots, jsondata (DataFile). En cours : simsplit, names, mock, tldel.
- [ ] Vague 2 (lancée 10-08, 6 lots car disque ≈ 35 Go) : battledev, hooks, uikit, audio, testsupport, schemadefs.
- Veto joueur 10-08 : garder l'issue bataille des rencontres (CB2) et les 4 issues de prise + ruines (CB3).
- Réserve : main a des modifs non commitées sur landmarks_v2/towns_1340 (sauvegarde scratchpad main-dirty.patch) → MA1/MA2 (suppression style real / landmarks v2) en attente de vérif.
- Disque : builds avec `CARGO_PROFILE_DEV_DEBUG=0 CARGO_INCREMENTAL=0`, `core/target` supprimé en fin de lot.

## Coordination FL (session parallèle, `../gp-fl`)
Gel jusqu'à la fusion de FL : `game/scripts/map/vegetation.gd`, `campaign_map.gd` (_process /
map.misc), déclutter des villes (settlement), `outbuilding_layer.gd`, life reground,
`terrain.gdshader` + includes, `dev/map_bench.gd`. Tout portage Rust d'un point chaud de la carte
est annoncé à FL avant de commencer. Base FL : d30 p50 31 ms, d150 p50 46 ms (map.misc 8-12 ms).
- Vague 3 : effects + rulesdata fusionnés (10-08) ; treaty en cours (ADR 0202 réservé). ADR SC renumérotés 0200 (parapluie) et 0201 (naval 3D) ; bloc 0200-0209 réservé SC (main a pris 0186-0192).
- Vague 4 (patterns/perf) lancée : movperf (sim-battle resolve_movement + geom), battlerules (moral/fatigue, tables terrain → data, decor_gen, rng ; ADR 0205 si besoin), naval (NV2/BB2/BB3/BB13 ; ADR 0203 si besoin), aiturn (ArmyTurn, constantes IA → data, PlanCache ; ADR 0204 si besoin), uikit2 (TooltipHost, diplomacy sous-vues, tutoriel → data).

## Fusion intermédiaire (décision du propriétaire, 08/10)
Dès que le point de contrôle est vert (relance propre cargo test + tests carte/UI) et que les lots aiturn, fxargs, lookups, rustperf, testkit sont fusionnés : fusionner feat/sc dans main SANS push (fichiers sales de main préservés, patch scratchpad/main-dirty.patch), puis les vagues suivantes repartent de main. Push seulement à la fin du chantier.

### Point de contrôle fait (08/10)
- `main` avancé en fast-forward sur 1e0d006b8 (feat/sc + main), **non poussé**. Fichiers sales de main préservés (patch 3 voies, désindexés).
- Vérifs : cargo test workspace 103 résultats / 0 échec, fmt/clippy propres ; pytest 3 162 verts (seul échec `test_manifest_bake_versions_follow_the_code`, antérieur, corrigé dans les fichiers sales d'une autre session) ; import 0 erreur ; smoke, ep13, fe_ui, cv1, q6, ib_layout, mf1 verts.
- Conflits : outils GA3 gardés (utilisés par la session i3d), conftest union, readme_gallery.
- Suite : vagues suivantes depuis `main`, intégrées via feat/sc.

### Vague 6 (lancée 08/10 depuis main 632ce7e0e)
- plancache (CC10 PlanCache explicite + CA8 + sels alignment, ADR 0205), rt5 (tests sim-battle/ai en binaires thématiques), fixtests (8 tests Godot en échec), lookups2 (JsonLookup restants + validateurs nus Python).
- Ensuite : lots doc en dernier (UI10, MC15, CA10, CC15, GB7), puis push.

## Vague 7 (08/10, depuis feat/sc 07b1c0bd8)
Inventaire des restes (agent Explore) : ≈ 55 lots faits, 40 partiels, 120 non faits ; CB2/CB3 vetoés.
Lots lancés : orders (CB4/CB5 module + RecruitContext), movement (CB8/CB9/CB13 + Dijkstra unique CA4/CA8),
battlepower (CB6/CB11, ADR 0208 si l'équilibrage bouge), keyenum (BB11/AD14/GB7 macro key_enum!),
gdtests (GT3/GT4 TestCase + run_all), gdshared (MC7 Hash, MC9 ConfirmDialog, MC8 army_markers).
Suivants candidats : BB14/BB5 siege_layouts, BA6/BA11 constantes bataille en data, CC6 save, CC13 missions,
PF-02 relief_quadtree → Rust, UI4 encyclopédie data, GB5 trait BattleSim ; docs (UI10, MC15, CA10, CC15, GB7 doc) en dernier.

## PAUSE (08/10 soir) — point de reprise
État : vagues 1-6 + gdshared (vague 7) dans main, NON poussé. Push à la fin du chantier seulement.
Lire aussi : docs/wip/sc/lots.md (notes par lot), inventaire des restes ci-dessus (« Vague 7 »).

Lots de la vague 7 interrompus, chacun commité en `wip` sur sa branche, worktree conservé :
| lot | worktree / branche | état | à faire |
|---|---|---|---|
| orders | ../gp-sc-orders, sc/orders | orders.rs → orders/ + RecruitContext, commité, non vérifié | fmt, clippy -p sim-campaign -p ai, cargo test idem, puis fusion |
| movement | ../gp-sc-movement, sc/movement | Dijkstra générique data_model::pathfinding (5 sites), movement/ en 6 fichiers, constantes landing/raid/siege en data/rules ; build OK, 3 erreurs clippy corrigées | fmt, clippy, cargo test -p sim-campaign -p ai -p data-model (vérifier omr_r1_agent_paths = départages Dijkstra), pytest -k schema ; supprimer wrappers terrain_cost/sea_neighbors ; fusionner own_army_checked avec orders::own_army après orders |
| battlepower | ../gp-sc-battlepower, sc/battlepower | puissance d'unité commune battle_auto/forecast, non vérifié | clippy, tests sim-campaign/sim-battle/ai ; ADR 0208 si l'équilibrage bouge |
| keyenum | ../gp-sc-keyenum, sc/keyenum | macro key_enum! + migration (data-model, godot-bridge), clippy propre | cargo test --workspace, puis fusion |
| gdtests | ../gp-sc-gdtests, sc/gdtests | game/tests/lib/test_case.gd + tools/run_godot_tests.sh, AUCUN test migré | migrer les *_test.gd (sortie 0/1 identique), comparer échecs avant/après |
Ordre de fusion conseillé : keyenum, orders, movement (dépend d'orders pour own_army), battlepower, gdtests. Conflits probables orders/movement/keyenum dans sim-campaign.
Règle machine : 3 lots Rust en parallèle au plus (5 → charge 200, agents rendus avant leurs tests).
Préexistant à corriger : at1_attack_order_test (siège commencé au lieu d'assaut, pending 0 → 0).
Constats pour le joueur : bouton « scinder l'armée » toujours caché (hud split_supported lit supports_order absent du pont) ; OutbuildingLayer._finish_warm peut bloquer si une vue est demandée dans la frame du setup ; test_relief_update (bake 5≠6) vient d'une autre session.
Vagues suivantes (≈ 5) : restes dans « Vague 7 » ci-dessus ; docs en dernier ; à la fin : push origin main, mémoire project-hyw-sc.

## Reprise (09/10)
main avancé de 93 commits DN depuis la base de la vague 7 (07b1c0bd8), aucun dans core/. Agents lancés : keyenum, orders, battlepower (vérif Rust + commit), gdtests (migration des tests Godot). movement attend orders (own_army). Intégration ensuite dans feat/sc avancé sur main, puis ff main.
- Décision du joueur (09/10) : à partir de la vague 8, les lots Rust ne font que `cargo check -p <crate>` ; une seule vérification complète (fmt, clippy -D warnings, cargo test --workspace) en fin de chantier sur l'intégration, bisect par vague si échec. Jusqu'à 6 lots par vague, lots choisis sans fichiers communs.

## Vague 8 (09/10, priorité joueur : bugs visibles + chargement)
- battlepower fusionné dans main (1ee017ee6, tests verts, équilibrage inchangé). Main réécrit par DN (ancien main 28e2c0337) : rebaser les sc/* avec `git rebase --onto main 28e2c0337 <branche>`.
- Lancés : bugs (bouton scinder, clé summons/peace_summons, OutbuildingLayer warm), attack (at1 siège au lieu d'assaut), mapload (DT4 chargeur Rust rivières/routes/côte/provinces, ADR 0206 si besoin). Worktrees ../gp-sc-<lot>.
- Fusionnés dans main (09/10) : mapload (DT4, ADR 0206, chargement carte ~500 → ~200 ms), attack (test at1 aligné sur ADR 0128), keyenum, orders, bugs (bouton scinder, clé peace_summons, warm-up OutbuildingLayer). Dylib main reconstruite, import + smoke verts.
- En cours : movement (rebasé, own_army), gdtests.

## Vague 9 (09/10, 8 agents, charge redescendue)
siegedet (tests de siège instables + test assaut avec engin prêt), codex (DT7 bundle), battleai (BA6 + data/rules/battle_ai.json), missions (CC13, ADR 0207), devflags (PF-06 + BT4), saves (CC6). Disque 75 Go : core/target supprimé en fin de lot.
