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

## État : terminé (hors fusion dans main)
- [x] C2 : opulence sur le trésor net des dettes + infobulles (tooltips, finance_panel, encyclopédie) ; test dédié.
- [x] C4 : `acts::start` à la création de la campagne (test `the_act_is_open_before_the_first_end_of_turn`).
- [x] C1 : acquis (objectifs remplis à l'ouverture de l'acte) = garde-fou (`hold_guard_percent` 35), progrès comptés depuis
  l'état de départ (`start_permille`). Sonde `seed-bots/examples/reign_probe` (passif / IA / décisif), mesures dans l'ADR 0788.
- [x] C3/C5/C7 : `sim-campaign/src/advisor.rs`, `data/rules/advisor.json`, alerte `advice` (game-api), ligne de posture.
- [x] C6 : 4 métriques, bandes rapides (`seed_bots_bands_quick.json`) et longues initialisées (passe longue du 24/10 :
  32 parties, partielle au plafond de 15 min, 0 dérive, 1 « tour lent » dû à la charge de la machine), régressions volontaires
  attrapées (ADR 0789).
- [x] fmt, clippy -D warnings, `cargo test --workspace` (2065 tests, 0 échec), pytest 1295, tests client `hint`.

## Points ouverts
- Avec l'IA minimale des rivaux (`end_turn`, test `a_full_campaign_crosses_the_three_acts`), une France passive finit à 49
  (avant 66) : prestige, Dauphiné (événement historique) et trésor progressent seuls. Avec l'IA complète (jeu réel) : 5-15.
- Profil « décisif » simulé par changement de contrôleur, pas par la guerre : à compléter par un vrai test de jeu.
- Bandes rapides calées sur une graine à 20 tours : à réviser si une règle déplace légitimement une métrique.
- Client : seule la liaison `advice` -> conseil (`hint.rs`) ; aucune capture prise.
