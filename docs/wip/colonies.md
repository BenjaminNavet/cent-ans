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
| C4 refonte cœur | fait (fusionné `17aa679`) | armées sur colonies, contrôle dérivé de la cité, revenu par poids, sauvegarde v5, entretien des garnisons par type (`rules.json`) |
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

## Fusion de C4 (2026-09-24, reprise)

- Deux fusions de `main` : la seconde a rattrapé G5 (voisinage par la géométrie, capitale jamais cédée). Conflits dans `ai/src/alignment.rs` (`borders` supprimé par G5 au profit de `are_neighbors`) et `sim-campaign/src/diplomacy.rs`. La logique de G5 est gardée et réécrite avec `controls_province` / `province_owner` ; `tests/g5_neighbors.rs` est porté de la même façon.
- Vérifié sur `17aa679` : clippy, cargo test, smoke Godot, `settlements_render_test`. La dylib de `main` est reconstruite.

Hors colonies, connu : `tools/tests/test_portraits.py::test_dry_run_makes_no_network_call` échoue aussi sur `main` (tous les portraits existent déjà).

## Prochaine étape

Vague 3, deux agents en parallèle, chacun dans son worktree :
- **C5** : pont et UI (panneau de colonie, onglet Colonies, ordres par colonie, aperçu de chemin sur le graphe) ; fichier de suivi `docs/wip/c5-settlements-ui.md`.
- **C7a** : repli du perdant, IA et équilibrage, test de 50 tours ; fichier de suivi `docs/wip/c7a-settlements-balance.md`.

Ensuite C7b : rendu (arbres sur le relief fin, routes au palier moyen) et documentation (`manuel.md`, codex).
