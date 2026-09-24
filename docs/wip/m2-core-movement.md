# M2 — cœur du mouvement libre (état)

Spec : `docs/design/2026-09-24-mouvement-libre.md` § 3 et § 7. ADR : `docs/decisions/0010-free-army-movement.md`.
Branche : `m2-core-movement` (worktree `agent-acbe5c6448250fa88`).

## État

- [x] `data/movement/rules.json` + schéma (copie exacte de ceux de M1, clés `plains`/`mountains`).
- [x] data-model : `FreeMovementRules`, `navgrid.rs` (grille, raster des provinces, repli `land_mask`, cache par processus), `settlements_px.json`.
- [ ] sim-campaign : `ArmyPosition`, `movement_left`, `planned_path`, A*/théta*, Dijkstra borné.
- [ ] Exécution immédiate : `MoveArmy`, `Attack`, `Embark`, siège, stationnement, repli sur la grille.
- [ ] `STATE_VERSION` 6, IA mécanique, pont, tests § 7.

## Prochaine étape

Types d'armée (`state.rs`) et module `navigation.rs`.
