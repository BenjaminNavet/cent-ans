# FE — intégration de la vague 1 (F1-F3, F4a) sur `feat/fe`

Worktree `../game_project-fe`, branche `feat/fe`. Mandat : orchestrateur FE, 2026-09-28.

## État
- [x] Schéma d'objectifs de titre resserré (commit dédié).
- [x] `docs/budget.md` : tableau FE à six colonnes, `test_budget.py` vert.
- [x] 8 tests rouges (F3 + succession) après F4a : verts. Règles corrigées : héritier venu d'un
  royaume moindre (`dynasty::pick_heir_by_law` repli, `feudal::heir_comes_home`), loi salique
  agnatique dans l'extinction, concession d'un titre conquis au seul vassal de rang suffisant.
  Données : `chr_marguerite_de_valois` (mère de Charles de Blois, sœur de Philippe VI).
- [ ] Fusion `feat/fe1-deductions` et réconciliation F1/F2/F3.
- [ ] Vérifications finales (fmt, clippy, tests, pytest, build, smoke).

## Prochaine étape
- pytest : 16 échecs dus aux données F4a (colonies, horizon, navgrid, héraldique, front-end,
  portraits) — confiés à un agent dans un worktree séparé.
- Fusion F1.
