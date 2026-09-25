# EQ5 — IA de campagne : banqueroutes chroniques et intrusions

Branche : `worktree-agent-a745acdb29a9168ed` (main + EQ4 `worktree-agent-a2b33defc792e3eab` fusionnés).

## État : diagnostic en cours

- [x] Sonde : `ECON_TRACE=fac_swiss century_probe …` trace le budget d'une faction à chaque tour
      (trésor, revenu, entretiens, impôt, unités, ordres économiques, événements, écart
      inexpliqué du trésor).
- [ ] Mesure de référence (normal 10, difficile 10, facile 5, très difficile 5 graines).
- [ ] Correctif banqueroutes (`ai/src/campaign.rs`).
- [ ] Correctif intrusions.
- [ ] Mesure après, ADR.

## Diagnostic (Suisses, facile, graine 2)

- Déficit structurel : entretien des bâtiments ≈ 60-75 % du revenu après la peste ; l'armée
  est déjà réduite à une unité (la dernière de la capitale, non licenciable).
- Impôt qui oscille Haut / Normal à chaque tour en dette (seuils sans hystérésis).
- Événements (chronique) à -300 / -500 livres qui replongent le trésor sous zéro.
- Affaiblissement de la monnaie : l'inflation gonfle l'entretien des bâtiments (157 → 201).

## Prochaine étape
Mesure de référence, puis correctifs IA.
