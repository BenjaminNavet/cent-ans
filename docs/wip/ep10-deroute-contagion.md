# EP10 — direction de la déroute et contagion de moral

Branche : `worktree-agent-adc8c89fbb2be1ac1` (worktree agent), main fusionné (SG5, 222080a4).
ADR : `docs/decisions/0068-direction-de-la-deroute-et-contagion.md` (mesures avant/après).

## État : terminé, en attente de fusion
- `data/rules/battle_rout.json` + schéma + pytest ; `sim-battle/src/rout.rs` ; branché dans `sim.rs`
  (`flight_direction`, contagion dans `resolve_morale_and_fatigue`) ; sièges inchangés.
- Réglages retenus : contagion 0,5/s par unité de poids, poids 1 jusqu'à 15 m derrière, 0,15 au-delà
  de 50 m ; fuite droit vers l'arrière, écart devant ennemi 2, devant amis 0,25 plafonné.
- Tests : `ep10_rout` (reproduction : 7/7 → 0/7 à moral 30 ; sondes `probe`, `probe_symmetric`,
  `probe_small_battle`), unitaires dans `rout.rs`, empreintes `b6.rs` recalculées (justifiées).
- Crochets de réglage temporaires (`EP10_RULES`, `EP10_OLD_FLIGHT`) retirés.

## Points ouverts
- Azincourt 19/20 au bord haut de la fourchette EP7 (14-19).
- `ep9b_duel` miroir : attaquant 18/30 → 7/30 (test 4/10, passe) ; très sensible aux réglages.
- Une ligne au seuil (moral 24-27) cède encore en entier après la déroute d'un voisin.

## Prochaine étape
Fusion par l'orchestrateur (ne pas toucher à main depuis ce lot).
