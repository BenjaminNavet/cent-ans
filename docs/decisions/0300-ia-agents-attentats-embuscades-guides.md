# 0300 — Règles de décision de l'IA pour les agents (attentat, poison, embuscade, guide)

Statut : accepté

## Contexte
Le lot WH `charsb` (ADR 0284) a ajouté quatre actions d'agent (`assassinate`, `poison`, `ambush`, `guide_army`) que seul le
joueur utilisait. L'IA d'agents de campagne vit dans `sim-campaign` (`agents/ai.rs`, appelée par `ai::campaign`), pas dans
la crate `ai` : le nouveau module s'y range.

## Décision
- Module `agents/ai_strikes.rs`, un seul point d'accroche (`plan_special`, appelé avant le comportement habituel de chaque
  agent) ; si aucune action spéciale n'est choisie, l'agent se comporte comme avant.
- Un tirage dérivé (graine, tour, agent) par agent et par saison, sans toucher au flux aléatoire principal ; chaque action
  prend une tranche de [0, 100) : attentat, puis poison, puis embuscade (espion) ; guide pour un héraut.
- Attentat/poison : l'espion doit déjà se trouver dans une place d'un seigneur en guerre avec sa faction, avec un général
  à portée dont l'armée atteint `min_general_strength` ; la cible est le général de l'armée la plus forte (le souverain
  seulement si le sceau de l'espion le permet ; gouverneur seulement si `strike_governors`). Il faut une chance de réussite
  ≥ `min_strike_odds`, et une trésorerie restant ≥ `reserve` après le coût. Plafond glissant par faction :
  `strike_cap` tentatives par `window_turns` (journal `AgentsState.strike_log`, alimenté par l'effet `strike`).
- Embuscade : armée ennemie en marche (destination) et assez forte à portée ; guide : armée amie en marche, assez forte,
  sur un territoire qui n'est pas ami de la faction. Chance ≥ `min_support_odds`.
- Toutes les valeurs sont dans `data/rules/agents.json`, bloc `ai_agents` (schéma `agent_rules.schema.json`). La
  difficulté multiplie les chances de `1 + ai_aggression_delta × aggression_percent_per_point %`.
- Pas de paix : les conditions de l'action (`at_war`) refusent l'ordre ; pas d'agent : aucun ordre.

## Conséquences
- L'IA peut tuer ou blesser des généraux (y compris ceux du joueur) : les valeurs par défaut visent quelques tentatives par
  grande faction et par partie, mesurées par `campaign_probe` (ligne « agent strikes »).
- Les agents ne se déplacent pas exprès vers les cibles : l'IA profite de leur position (ils vont déjà en pays ennemi pour
  renseigner). Un déplacement dédié reste possible (point ouvert).
- Le poison n'a pas de risque diplomatique propre côté IA (le moteur ne le révèle pas), d'où sa chance plus faible.

## Mesure (campaign_probe, 120 tours, graines 1-3)
Chances livrées : attentat 20 %, poison 8 %, par agent et saison, seulement avec une cible à portée. Sonde : 1, 1 et 0
attentats sur la carte entière (0 avant), soit de l'ordre de 3 par partie complète, jamais plus d'un par faction. Guerre
FR-EN 62/72/47 % contre 67/74/64 %, révoltes 0/2/3 contre 6/1/2 : écarts dans le bruit d'une simulation chaotique.
