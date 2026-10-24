# Lot GAMEPLAY2 — calibrage et lisibilité du gameplay de campagne

Branche : worktree-agent-a0e0ca25b3459a2c0 (main + ce lot). Source : juge game design it.4
(`out/juge/iter-04-design.md`). ADR 0788 (score de règne, emprunt) et 0789 (conseiller, oracle).

## Constats (vérification)
- C1 CONFIRMÉ en partie : au tour 2 le score d'acte valait 35 sans action (objectifs déjà remplis). Avec l'IA complète
  (celle du vrai jeu, `ai::plan_turn`) une France passive s'effondre (défaite vers 1403-1418, score 28-30), elle ne
  s'approche pas de 72 ; le « 66 » du test d'ACTES vient de l'IA minimale des rivaux (`end_turn`), pas du jeu réel.
- C2 CONFIRMÉ par lecture (`economy.rs::administration_upkeep_for` : l'opulence comptait le trésor brut).
- C3 CONFIRMÉ : alerte `free_slot` = compteur Info.
- C4 CONFIRMÉ par lecture : `acts.started` n'était posé qu'au premier `end_turn`.
- C5 CONFIRMÉ : aucun conseil de saison en données (seulement `next_hints.json`).
- C6 CONFIRMÉ, cause : `--quick` ne collectait pas les métriques (`balance: balance || !quick`).
- C7 CONFIRMÉ : `doctrines::active` pure, aucun événement au changement.

## État
- [x] C2 : opulence sur le trésor net des dettes + infobulles (tooltips, finance_panel, encyclopédie).
- [x] C4 : `acts::start` à la création de la campagne.
- [x] C1 : acquis (objectifs remplis à l'ouverture de l'acte) = garde-fou (`hold_guard_percent`), progrès comptés depuis
  l'état de départ (`start_permille`). Sonde : `seed-bots/examples/reign_probe` (profils passive/active/decisive).
- [x] C3/C5/C7 : `sim-campaign/src/advisor.rs`, `data/rules/advisor.json`, alerte `advice` (game-api), ligne de posture.
- [ ] C6 : métriques (battles, provinces, trésor, score de règne), bandes quick + long, régression volontaire.
- [ ] Mesures finales, ADR 0788/0789, fmt/clippy/tests workspace, pytest, suppression de `core/target`.

## Prochaine étape
Résultats de la sonde (`/private/tmp/claude-501/probe-new-1.log`), calibrage de `hold_guard_percent` et du seuil.
