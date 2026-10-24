# Lot IA : doctrines stratégiques de l'IA de campagne (ADR 0782-0783)

Branche `worktree-agent-abd99ac4866467ce0` (avancée en fast-forward sur main après ECO2/CAUSES). Sources : `out/juge/iter-03-design.md`, ADR 0348/0350 (seed bots), 0754 (causes).

## État : terminé (vérifications finales en cours)

- Catalogue `data/ai/strategic_doctrines.json` (8 doctrines, 23 affectations) + schéma ; types `data-model::ai_strategic_doctrine` ; résolveur `sim-campaign::doctrines` (faction, souverain, années, défaites) ; revanche (`BattleTally::last_province`).
- Câblage : `ai::strategy::Strategy` (armées : repli, assaut, seuil de siège, cibles de siège par objectifs, défense, chevauchée d'abord ; grille : supériorité avant d'attaquer ; mariages ; filtre des ordres de finance), diplomatie (`war_target` : audace et revanche, `plan_alliances` : cadence, agressivité).
- Journal des causes : la guerre déclarée par l'IA consigne la doctrine et la raison (`revenge_at` « Revanche d'une défaite subie en {lieu} »).
- Difficulté (ADR 0783) : `ai_prudence_percent`, `ai_anticipation`, `ai_coordination`, `ai_vision_provinces` (brume), neutres au niveau normal.
- Tests : `core/crates/ai/tests/strategy/doctrines.rs` (13 tests, graine 1337) ; `m3_grid_ai::nearby_friendly_armies_are_counted_once` suit le ratio de la doctrine anglaise.

## Mesures (events_probe, graine 1, IA stratégique partout, 1337-1453)

| Joueur | Avant (main) | Après |
|---|---|---|
| France | 4/24 événements, batailles Angleterre>France 44, France>Angleterre 14 ; pas de Crécy ni Poitiers tirés | 7/24, 29 / 6 ; Crécy 1346, Poitiers 1356, Brétigny 1360 |
| Angleterre | 4/24, 7 / 23 ; Poitiers (victoire française) 1356 | 4/24, 21 / 22 ; Crécy 1347 |

Seed bots rapide : 8 parties, 0 constat. Passe longue : 32 parties (4598 tours), 0 constat, aucune dérive d'équilibre.

## Points ouverts

- `cargo clippy --all-targets` échoue sur `ev_history.rs:669` (`for id in [..]` à un élément, lot EVENTS, hors périmètre).
- La doctrine n'est consignée que pour les guerres déclarées ; le changement de doctrine (nouveau souverain, défaites) n'est pas un événement du journal.
- Le « lieu » de la revanche est la province de la dernière défaite, pas le nom de la bataille.
