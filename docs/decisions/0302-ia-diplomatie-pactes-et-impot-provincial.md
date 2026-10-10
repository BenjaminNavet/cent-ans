# 0302 — L'IA demande des renforts, propose des pactes, règle l'impôt provincial (lot WR `ai-diplo`)

## Contexte
Les lots WH `diploa`/`diplob`/`econ` ont ajouté « Rejoindre la guerre contre X » (`Article::JoinWar`), le pacte de non-agression
(`Article::NonAggression`) et l'impôt par province, mais l'IA n'en usait pas (restes de `docs/wip/wh/diplob.md` et `econ.md`). Sa
**réponse** à ces articles proposés par le joueur existe déjà (`negotiation/value.rs` : facteurs `join_war_factors` et
`non_aggression_factors`, tests `wh_diplob.rs`) : seule l'initiative manquait.
Le seuil `province_tax_relief_unrest` était laissé à 101 (« à 70 la passe ne se déclenche jamais »). Cause trouvée : l'IA de campagne lit
ses règles dans `AiCampaign::bundled()` (JSON compilé dans le binaire), pas dans le répertoire de données chargé ; l'essai avec
`CENT_ANS_DATA_DIR` ne changeait donc rien. L'échelle (désordre 0-100, révolte à 75) et la condition de la passe étaient bonnes.

## Décision
- **Module neuf** `sim-campaign/src/diplomacy/ai_pacts.rs` (`plan_pacts`), accroché par une ligne dans `plan_diplomacy`. Réglages
  `data/ai/diplomacy.json` § `ai_pacts` (schéma et `AiPactRules` à jour), coupables d'un `enabled`.
- **JoinWar** : une faction en guerre demande à un allié militaire (attitude ≥ 15, pas encore en guerre contre l'ennemi) de la
  rejoindre quand elle perd (puissance de coalition < 0,8 × celle de l'ennemi) ET que l'allié a une rancune ou une frontière contre
  l'ennemi (`join_war_need_both`). Gardes : au plus une demande par tour, une fois tous les 8 tours (décalée par le créneau) ; l'allié
  doit être reposé (`war.rest_turns`) ; la guerre ne doit pas entraîner plus de `join_war_max_pulled` (1) autres factions
  (autres alliés de l'allié, alliés de l'ennemi). Entre IA, la demande n'est envoyée que si le destinataire la signerait
  (`evaluate_treaty_with`) ; au joueur, elle passe par le canal d'offres habituel (`create_offer` et son délai).
  Sans ces gardes, une première version (toutes les 4 saisons, « perd OU rancune », sans plafond d'entraînement) faisait passer
  les guerres déclarées de ~180 à ~9 000 sur 120 saisons (cascade d'appels aux armes) : écartée.
- **Non-agression** : toutes les 6 saisons (créneau), une faction occupée par une autre guerre, ou plus faible que le voisin
  (puissance < 0,5 ×), propose 20 saisons de pacte à son voisin le plus puissant sans grief (ni allié, ni guerre, ni trêve,
  ni rival, ni revendication dans un sens ou l'autre, attitude mutuelle ≥ -10). Au plus 2 pactes actifs par faction.
- **Impôt provincial** : `province_tax_relief_unrest` = 70 (sous le seuil de révolte 75 ; le « Haut » est déjà interdit à 65).
  Test `ai/tests/economy/wr_province_tax_ai.rs` : la passe agit avec la valeur compilée. Note : tout réglage de `ai/campaign.json`
  exige de recompiler pour être mesuré.

## Conséquences
Mesure `campaign_probe` (120 saisons, graines 1-3, avant → après) : guerres déclarées 177/213/156 → 153/229/185 (total identique),
guerres actives 33/37/30 → 29/36/35, guerre FR-EN 67/74/64 % → 72/70/64 %, révoltes 6/1/2 → 1/0/0, pactes ~85 paires en cours,
demandes d'entrée en guerre signées 4/2/3, provinces à impôt propre 1-3 en moyenne. La sonde mesure désormais pactes, JoinWar et
impôts provinciaux. Les demandes de JoinWar restent rares par conception (risque de guerre de blocs) ; les assouplir
(`join_war_max_pulled`, `join_war_period`) se mesure d'abord avec la sonde.
