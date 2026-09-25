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

## Diagnostic (traces `SG4_ROUTS=1`, `SG4_SHOTS=1`, `SG4_HORSE=a..b` ajoutées à `sg4_balance`)
- Crête avec pieux, graines 1-10 : la première déroute est toujours la cavalerie du défenseur,
  battue au bas du glacis (x ≈ 600, z ≈ 850) ; elle reflue à travers les tireurs de l'extrémité
  gauche puis la ligne ; déroutes en cascade de régiments intacts (« routing friends 3-4 »,
  ennemi à 150-200 m) qui glissent le long de la ligne. Aucune mêlée d'infanterie.
- Les graines changent à peine le début de la bataille : 10 graines ≈ 1-2 batailles
  indépendantes ; un réglage fait basculer tout un cas (0/10 ↔ 10/0).
- Sans les sorties de la cavalerie du défenseur contre les tireurs « isolés » de l'attaquant, le
  défenseur perd 10/10 : ses archers perdent 77-93 % dans le duel (ils tirent sur les arbalétriers
  derrière leurs pavois, les archers de l'attaquant tirent sur eux).
- Cavaliers d'une aile tous sur le même point (empilés) : une déroute emporte toute l'aile.

## Prochaine étape
Sonde élargie (40/60/80 régiments × 10 graines, `sweep2.sh` du scratchpad) pour choisir entre :
aile de cavalerie hors des tireurs et étalée, laisse des sorties, tir de contre-batterie.
Réglages provisoires lus dans des variables `SG5_*` (à retirer avant la fin).
