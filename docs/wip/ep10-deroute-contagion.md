# EP10 — direction de la déroute et contagion de moral

Branche : `worktree-agent-adc8c89fbb2be1ac1` (worktree agent). ADR prévue : `docs/decisions/0067-direction-de-la-deroute-et-contagion.md`.

## Diagnostic (SG5, session de nuit)
- Contagion (`sim.rs`, `resolve_morale_and_fatigue`) : tout ami en déroute à moins de 120 m compte,
  où qu'il soit (0,4 moral/s par ami, au plus 3).
- Fuite (`sim.rs`, `resolve_movement`) : bord de son camp + opposé de l'ennemi le plus proche ;
  ennemi de flanc → la déroute court le long de la ligne.

## Reproduction (`sim-battle/tests/ep10_rout.rs`, sonde `probe`)
Ligne de 8 régiments d'hommes d'armes à pied face à une ligne miroir à 90 m, chevaliers ennemis à
70 m sur le flanc de l'aile, aile en déroute (moral 15), reste de la ligne ébranlé (moral = plafond).
Avant EP10 — régiments qui cèdent par contagion en 120 s (sur 7), course latérale de l'aile 113 m :

| Moral de la ligne | 24 | 27 | 30 | 35 | 40 |
|---|---|---|---|---|---|
| avant | 7 | 7 | 7 | 7 | 4 |

## État
- [x] Squelette : `data/rules/battle_rout.json`, schéma, pytest, test de reproduction.
- [ ] `sim-battle/src/rout.rs` (RoutRules, poids de contagion, direction de fuite) branché dans `sim.rs`.
- [ ] Mesures avant (binaires copiés dans le scratchpad, en cours) / après.
- [ ] ADR 0067, epic.md.

## Prochaine étape
Implémenter `rout.rs`, brancher, relancer la sonde puis les mesures après.
