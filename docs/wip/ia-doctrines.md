# Lot IA : doctrines stratégiques de l'IA de campagne (ADR 0782-0783)

Branche `worktree-agent-abd99ac4866467ce0`. Sources : `out/juge/iter-03-design.md`, ADR 0348/0350 (seed bots), 0754 (causes).

## État

- Fait : catalogue `data/ai/strategic_doctrines.json` + schéma, types `data-model::ai_strategic_doctrine`, résolveur `sim-campaign::doctrines`, mémoire du lieu de la dernière défaite (`BattleTally::last_province`), paliers de difficulté (prudence, anticipation, coordination, brume) dans `DifficultyModifiers`.
- À faire : câblage dans `ai` (armées, grille, mariages, finances), diplomatie (`war_target`, alliances), cause de guerre (doctrine + revanche), textes, tests, mesures (events_probe, seed bots), ADR.

## Prochaine étape

Câbler `Context`/`Fleet`/`GridPlanner` sur `doctrines::active`.
