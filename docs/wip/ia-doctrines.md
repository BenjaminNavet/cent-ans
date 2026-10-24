# Lot IA : doctrines stratégiques de l'IA de campagne (ADR 0782-0783)

Branche `worktree-agent-abd99ac4866467ce0` (rebasée par fast-forward sur main après ECO2/CAUSES). Sources : `out/juge/iter-03-design.md`, ADR 0348/0350 (seed bots), 0754 (causes).

## État

- Fait : catalogue `data/ai/strategic_doctrines.json` (8 doctrines, 23 affectations) + schéma ; types `data-model::ai_strategic_doctrine` ; résolveur `sim-campaign::doctrines` (faction, souverain, années, défaites) ; revanche (`BattleTally::last_province`) ; câblage `ai::strategy::Strategy` (armées, grille, cibles de siège, défense, chevauchée d'abord, mariages, finances), diplomatie (`war_target`, `plan_alliances`, agressivité) ; cause de guerre avec doctrine et revanche ; paliers de difficulté (prudence, anticipation, coordination, brume) ; ADR 0782, 0783 ; tests `core/crates/ai/tests/strategy/doctrines.rs`.
- À faire : mesures (events_probe, campaign_probe, seed bots rapide puis longue), tests dépendant de la dynamique (ev_history, f7_events), fmt/clippy/test workspace, pytest schémas, nettoyage `core/target`.

## Prochaine étape

Lancer les tests, mesurer, régler les valeurs du catalogue si les batailles Angleterre/France disparaissent.
