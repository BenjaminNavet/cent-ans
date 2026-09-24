# Orchestration : refonte « colonies » (échelle Total War)

Spec : `docs/design/2026-09-24-echelle-colonies.md`. ADR : `docs/decisions/0005-settlements-within-provinces.md`.
Le joueur a validé la spec et autorise toutes les décisions sans demander (2026-09-24).

## Lots et état

| Lot | État | Notes |
|---|---|---|
| Schéma `settlement` | fait | `data/schemas/settlement.schema.json`, `settlement_id` dans `common.schema.json` |
| C1 squelette cœur | fait (fusionné `a4de853`) | types data-model, `SettlementState`, cité auto, `rules.json`, getter `settlements()` |
| C2a colonies France | fait | 41 prov., 196 colonies, 8 enclaves |
| C2b colonies Nord | fait | 56 prov., 227 colonies ; Stirling anglais, Dunbar écossais |
| C2c colonies Sud | fait | 35 prov., 145 colonies ; enclaves scaligères (Lucques, Trévise), Mantoue impériale, Alghero génoise ; total 568 colonies |
| C3 pipeline géo | fait (`b79fbeb`) | graphe 1 345 arêtes (634 route, 28 mer), routes Itiner-e + 70 calculées hors limes, 2 999 hameaux, relief 8192² en 256 tuiles (50 Mo) |
| C4 refonte cœur | **fini, fusion en cours** (branche `merge-c4`, voir Reprise) | armées sur colonies, contrôle dérivé de la cité, revenu par poids, sauvegarde v5, entretien des garnisons par type (`rules.json`) |
| C5 pont + UI | à faire | panneau de colonie sur `SettlementLayer.settlement_selected(id)`, clic droit sur une colonie comme cible, aperçu de chemin sur le graphe |
| C6 rendu paliers | fait | paliers loin > 620 / moyen 150-620 / près < 150 ; 13 maquettes Blender ; relief fin LRU ; captures `docs/img/colonies/` ; signal `settlement_selected(id)` à brancher en C5 |
| C7 IA, équilibrage, docs | à faire | + perdant sans colonie amie voisine reste sur place ; arbres sur relief fin ; routes principales peu visibles au palier moyen |

## Coordination

La session 6 (`docs/wip/tw.md`, plan `docs/design/2026-09-24-rapprochement-total-war.md`) rapproche aussi le jeu de Total War, colonies exclues de son périmètre. Ses lots s'appellent aussi « C1 »… (minicarte, brouillard) : sans rapport avec les lots colonies. Fichier de friction probable : `game/scripts/map/campaign_map.gd`.

## Décisions prises en cours de route

- Découpage de la recherche par grande région (champ `region` des provinces) plutôt que par pays.
- Avignon (cité du Comtat) laissée à la Papauté (`owner: null`) : juridiquement angevine jusqu'en 1348, mais siège de fait du pape ; sinon la règle « contrôleur de province = contrôleur de la cité » retirait sa capitale à la Papauté.
- Test des colonies : bornes vérifiées en pixels de la carte carrée (Vienne à 16,37° E est sur la carte), pas en degrés.
- `set_marmoutier` (Alsace) / `set_marmoutier_tours` (Touraine) : homonymes.
- Le smoke Godot échoue déjà avant la refonte (`projected_income should increase after construction`, `smoke.gd:358`, fichier modifié par une autre session) : non imputable aux colonies.
- Colonies hors province (71) : position de jeu ramenée dans la province (Voronoï approximatif, ports en mer). Seules les attributions défendables ont été changées : Lund et Skanör → Götaland (Scanie suédoise depuis 1332), Mont-Cassin → Naples (Terre de Labour), Auch → Toulousain ; Vordingborg ajouté pour garder 3 colonies au Sjælland. Bois-le-Duc, Elvas, etc. restent dans leur province historique.
- C6 lancé sans attendre C5 : le rendu lit les positions statiques de `data/map/` et le contrôleur par le getter `settlements()` (déjà dans C1).
- C4 lancé en parallèle de C3 : si `settlement_graph.json` manque, le cœur construit un graphe de repli (colonies d'une province reliées entre elles, cités reliées aux cités voisines, ports aux ports `sea_neighbors`).

## Reprise (arrêt de session, 2026-09-24 soir)

État exact :
- C1, C2a-c, C3, C6 sont dans `main`. C4 est terminé mais **pas encore dans `main`**.
- La branche **`merge-c4`** (commit `8208996`) contient C4 fusionné avec `main` jusqu'à `f52fd92`, conflits résolus :
  - `load.rs` : `vision_rules` (session 6) et `movement_graph` (C4) gardés tous les deux ;
  - `army_markers.gd` : masquage par brouillard et position sur la colonie combinés ;
  - `vision.rs` + `tests/c1_vision.rs` : la vision part de la province de la colonie et du contrôleur de la cité ;
  - `minimap_controller.gd` et `smoke.gd` : lisent `location_province` ;
  - `ai/src/alignment.rs`, `ai/tests/g4.rs`, `ai/examples/century_probe.rs` (lot G4 d'une autre session) : contrôle/propriété de province via `controls_province` / `province_owner` / `city_state_mut`.
- Sur `8208996` : clippy `-D warnings` et `cargo test` **passent**. **Smoke Godot non relancé** depuis la fusion de G4 (le précédent était vert).

À faire pour finir C4 :
1. `git worktree add <scratch>/merge-c4 merge-c4` (le worktree temporaire a pu disparaître ; la branche reste).
2. `git merge main` (attention : git est en français, un conflit s'affiche « CONFLIT », pas « conflict »).
3. `cd core && cargo fmt && cargo clippy --all-targets -- -D warnings && cargo test` ; corriger les lectures de `province.controller` / `.owner` / `.garrison` / `army.location` que d'autres sessions auraient ajoutées (le compilateur les trouve).
4. `core/build.sh`, `godot --headless --path game --import`, `godot --headless --path game --script res://tests/smoke.gd`, `res://tests/settlements_render_test.gd`.
5. `git merge --ff-only merge-c4` dans `main` immédiatement après (d'autres sessions commitent souvent) ; supprimer `merge-c4` et le worktree/branche `worktree-agent-ae94e419e9c0f37b2`.

Hors colonies, connu : `tools/tests/test_portraits.py::test_dry_run_makes_no_network_call` échoue aussi sur `main` (tous les portraits existent, le dry-run n'en trouve plus : test à adapter par la session portraits).

## Prochaine étape

Finir la fusion de C4 (ci-dessus), puis vague 3 : C5 (pont + UI des colonies), puis C7 (IA, équilibrage, test 50 tours, docs `manuel.md` et codex).
