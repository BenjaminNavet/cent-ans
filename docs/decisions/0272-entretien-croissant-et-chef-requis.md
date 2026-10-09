# 0272 — Entretien croissant des armées et chef requis

Statut : accepté

## Contexte
Rien ne coûtait de multiplier les armées (rapport WH `armees`, A1/A2) : l'entretien somme les régiments, et une armée sans général ne perdait rien. Dans Total War: Warhammer III, la « doomstack » contre six petites armées est un arbitrage.

## Décision
`data/rules/armies.json` (serde default, schéma `army_rules.schema.json`) :
- `upkeep {free_armies: 5, extra_army_upkeep_percent: 10}` : les `free_armies` armées les plus chères restent gratuites de surcoût, la k-ième en plus paie son entretien × (1 + k × 10 %). `faction_upkeep` l'applique ; `TurnBudget` et `FactionEconomy` exposent `army_surcharge` et `army_count` (info-bulle du panneau de faction).
- `leaderless {movement_percent: -25, no_siege, no_ambush}` : une armée sans général perd 25 % de points de mouvement, ne peut ni prendre la posture de siège (ordre refusé) ni s'arrêter devant une place défendue pour l'assiéger (la marche s'arrête aux murs ; une place vide ou un village se prennent encore), ni tendre d'embuscade.
- `pace_percent_by_category` : allure par famille d'unités, l'armée suit la plus lente ; cavalerie +25 %. Appliquée sur les points (et non sur les pas entiers, qui l'auraient arrondie à néant) ; le malus du train de siège (`siege_train_pace_percent`) reste inchangé.
- L'IA (`ai/src/campaign/economy.rs`, surplus de garnison) nomme un chef libre de la province avant de créer une armée ; sans chef libre dans la province, elle ne forme que `free_armies` armées sans chef au plus (convois de renfort qui rejoignent une armée menée) et garde le surplus en garnison au-delà. Elle réserve aussi son meilleur commandant libre (hors gouvernorats) tant qu'elle a de la place pour une armée.

## Conséquences
- Mesure `campaign_probe` 120 tours, graines 1-2 (avant → après) : guerre FR-EN 66 %/92 % → 73 %/61 % ; banqueroutes 122/87 → 118/89 ; révoltes 1/8 → 2/4 ; plus grosse faction France (29 provinces) inchangée. Deux variantes intermédiaires écartées : IA sans aucune armée sans chef (garnisons de 49 régiments inertes, aucun siège en 40 tours) et IA libre de créer des armées sans chef sans réserve de commandant (sièges impossibles). Conséquence sur les tests : le premier siège français de `a_stronger_faction_at_war_besieges` vient dans les 20 tours (12 avant) ; les graines de `the_ai_never_gives_a_stance_order_the_core_refuses` passent de 2, 4, 5 à 1, 3, 4, 6.
- Valeurs à ajuster en données seulement. Les armées anciennes déjà sans chef gardent leur siège en cours.
