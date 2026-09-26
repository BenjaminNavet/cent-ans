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
- [x] `sim-battle/src/rout.rs` (RoutRules, poids de contagion, direction de fuite) branché dans `sim.rs`
  (sièges : règle inchangée). Sonde : 0 régiment ne cède à moral 27-40 (7 à moral 24).
- [x] Mesures avant (main 2ecdd019) : EP7 Crécy 16/20, Azincourt 15/20, Poitiers 14/20 ; ep9b 7/10 ;
  SG4 plat pieux 3/7, plat sans 7/3, crête pieux 6/4, crête sans 1/9, générée 2/8 et 6/4, ep1 6/4.
- [ ] Réglage : première version (15/50/0,15) → EP7 6, 10, 9 /20 (hors fourchettes), ep9b 9/10 (hors 3-7).
  Fuite seule (contagion ancienne) : EP7 19, 17, 15 ; ep9b 9/10. Variantes en cours.
- [ ] ADR 0067, epic.md, b6.

**Temporaire, à retirer avant la fin** : `EP10_RULES` (rout.rs, `bundled`) et `EP10_OLD_FLIGHT`
(sim.rs, `flight_direction`) servent au réglage.

## Prochaine étape
Choisir les réglages (scripts `variant.sh` dans le scratchpad), retirer les crochets temporaires,
recalculer b6 si nécessaire, ADR.
