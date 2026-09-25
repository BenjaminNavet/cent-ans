# EQ5 — IA de campagne : banqueroutes chroniques et intrusions

Branche : `worktree-agent-a745acdb29a9168ed` (main + EQ4 `worktree-agent-a2b33defc792e3eab` fusionnés).

## État : correctifs en place, mesures en cours

- [x] Sonde : `ECON_TRACE=fac_swiss` (budget d'une faction par tour, événements, écart
      inexpliqué du trésor), `TRESPASS_TRACE=1` (armées en intrusion en fin de tour),
      `DEBUG_ARMY=army_0012` (ordres d'une armée).
- [x] Mesure de référence = EQ4 à l'identique (normal 10, difficile 10, facile 5, très difficile 5).
- [x] Banqueroutes (`ai/src/campaign.rs`) : engagements (tributs, rançons, agents, Table) dans
      le revenu net ; marge de sécurité 5 % ; impôt Haut avec hystérésis (+10) et quand le
      trésor est mince ; bâtiments plafonnés à 35 % du revenu net ; pas d'affaiblissement de
      la monnaie si les bâtiments dépassent 1/3 du revenu.
      Mesure v1 : 0,01-0,04 / fac. / déc. à tous les niveaux (pire graine : 1,8).
- [x] Intrusions (`ai/src/grid.rs`, `campaign.rs`) : une armée peut sortir des terres où elle
      est prise ; ses propres places ne sont jamais interdites ; retour au pays d'une armée
      sans objectif en terre étrangère (table élargie qui traverse les terres fermées) ; en
      paix, une armée dans sa propre place en province étrangère rejoint la garnison ; plus
      de poursuite d'une armée ennemie en terre fermée.
      Mesure v1 : normal 1312 → 883, difficile 2216 → 1089, très difficile 2472 → 881,
      facile 593 → 900 (bien plus de sièges : IA plus riche).
- [ ] Essai v2 : demande de droit de passage (`diplomacy_eval::plan_passage`).
- [ ] Ablation « armées seules » pour isoler l'effet de l'économie sur l'activité militaire.
- [ ] ADR, tests, mesure finale.

## Diagnostic

- Suisses (facile, graine 2) : bâtiments ≈ 60-75 % du revenu après la peste (payés sur le trésor
  initial), impôt qui oscille Haut / Normal autour de 30 de trouble, événements à -300 / -500,
  monnaie affaiblie qui gonfle l'entretien des bâtiments (157 → 201) pour toujours.
- Grenade : guerres lointaines (Angleterre, Écosse, Holstein) soldées par des paix payantes ;
  les tributs par saison (200-550) n'entraient pas dans le budget de l'IA.
- Intrusions : 38 % des saisons étaient des armées dans leur propre place d'une province
  étrangère ; les autres, des armées de campagne coincées (les terres fermées bloquaient
  aussi la sortie), des poursuites en terre neutre, des marches en guerre (chemins qui
  mordent sur une province voisine, passages délibérés des IA agressives).

## Prochaine étape
Comparer ablation / v2, choisir, ADR 0063, tests unitaires, mesure finale, fusion de main.
