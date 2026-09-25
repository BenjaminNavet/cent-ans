# SG5 — les tireurs et la cavalerie du défenseur ne rompent plus leur propre ligne sur la crête

Branche `worktree-agent-add3a9bbd7d238220`. Suite de SG4 (`docs/wip/sg4-assaut.md`), ADR 0046
(§ Suite SG4, **§ Suite SG5**), ADR 0056 (§ EP9b).

## État : diagnostic en cours

- [x] `git merge main` (EP9b inclus).
- [x] Mesure de départ `sg4_balance` (10 graines) : identique à ADR 0056 § EP9b.
- [ ] Diagnostic (trace crête avec pieux).
- [ ] Correctif côté placement / ordres (`ai.rs`, données `data/rules/`).
- [ ] Mesures, ADR 0046 § Suite SG5, contrôles complets.

## Mesure de départ (main + EP9b, attaquant / défenseur / nuls)

| Terrain | Pieux | Départ |
|---|---|---|
| plat | oui | 3 / 7 / 0 |
| plat | non | 7 / 3 / 0 |
| crête | oui | 6 / 4 / 0 |
| crête | non | 1 / 9 / 0 |
| plaine générée | oui | 2 / 8 / 0 |
| plaine générée | non | 6 / 4 / 0 |
| comme ep1_scale | oui | 6 / 4 / 0 |

## Contraintes
- Ne pas toucher contagion / engagement (`sim.rs`, `decision.rs`, session épique).
- EP7 (cartes historiques, branche à part) : ne pas redéployer un régiment placé par un plan
  explicite (postes `hold` du scénario).

## Prochaine étape
Tracer `SG4_CASES=crest SG4_SEEDS=1..2 SG4_TRACE=1`.
