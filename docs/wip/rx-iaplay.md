# RX iaplay — IA de campagne (état)

Branche `rx/iaplay` (worktree `../gp-rx-iaplay`). ADR 0258.

## Fait
- `campaign_probe` : métriques de sièges (commencés, pris, paix, autres, part prise).
- Difficulté lue par l'IA : 4 champs `DifficultyModifiers` (`ai_siege_superiority_percent`,
  `ai_assault_odds_delta`, `ai_decision_noise_percent`, `ai_aggression_delta`), valeurs dans `data/rules/difficulty.json`.
- Mémoire de cible (`siege_target_persistence`, `defence_target_persistence`) dans `pick_siege` / `pick_defence`.

## Prochaine étape
- Fait : voir ADR 0258. Reste : réglage fin des valeurs de difficulté.
