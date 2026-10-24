# Lot GAMEPLAY2 — calibrage et lisibilité du gameplay de campagne

Branche : worktree-agent-a0e0ca25b3459a2c0 (main c10f396e7 + ce lot). Source : juge game design it.4
(`out/juge/iter-04-design.md`). ADR 0788 (score de règne, emprunt) et 0789 (conseiller, oracle).

## Constats (vérification)
- C1 score récompense l'inaction : à vérifier par `reign_probe` (seed-bots) et test acts.
- C2 emprunt/administration : CONFIRMÉ par lecture (`economy.rs::administration_upkeep_for` : l'opulence compte
  le trésor brut, emprunt inclus). Correction : opulence sur trésor net des dettes (+ infobulles).
- C3 colonies sans chantier : à faire (conseiller + indicateur).
- C4 objectifs t0 : CONFIRMÉ par lecture (`acts.started` n'est posé qu'au premier `end_turn`, `act_view` est vide avant).
- C5/C7 conseiller / doctrine : à faire.
- C6 oracle : cause = `--quick` ne collecte pas les métriques (`balance: balance || !quick`) + 9 bandes `to_initialise`.

## État
- [x] C2 code + test `borrowing_does_not_raise_the_administration_charge` + textes (non encore exécuté).
- [ ] C4, C1, C3, C5, C7, C6.

## Prochaine étape
C4 (démarrer l'arc à la création), puis sonde `reign_probe` (seed-bots/examples) passif / modéré / actif.
