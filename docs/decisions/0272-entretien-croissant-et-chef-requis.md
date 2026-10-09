# 0272 — Entretien croissant des armées et chef requis

Statut : accepté

## Contexte
Rien ne coûtait de multiplier les armées (rapport WH `armees`, A1/A2) : l'entretien somme les régiments, et une armée sans général ne perdait rien. Dans Total War: Warhammer III, la « doomstack » contre six petites armées est un arbitrage.

## Décision
`data/rules/armies.json` (serde default, schéma `army_rules.schema.json`) :
- `upkeep {free_armies: 5, extra_army_upkeep_percent: 10}` : les `free_armies` armées les plus chères restent gratuites de surcoût, la k-ième en plus paie son entretien × (1 + k × 10 %). `faction_upkeep` l'applique ; `TurnBudget` et `FactionEconomy` exposent `army_surcharge` et `army_count` (info-bulle du panneau de faction).
- `leaderless {movement_percent: -25, no_siege, no_ambush}` : une armée sans général perd 25 % de points de mouvement, ne peut ni prendre la posture de siège (ordre refusé) ni s'arrêter devant une place défendue pour l'assiéger (la marche s'arrête aux murs ; une place vide ou un village se prennent encore), ni tendre d'embuscade.
- `pace_percent_by_category` : allure par famille d'unités, l'armée suit la plus lente ; cavalerie +25 %. Appliquée sur les points (et non sur les pas entiers, qui l'auraient arrondie à néant) ; le malus du train de siège (`siege_train_pace_percent`) reste inchangé.
- L'IA (`ai/src/campaign/economy.rs`, surplus de garnison) nomme un chef libre de la province avant de créer une armée ; sans chef et au quota d'armées, elle n'en crée pas.

## Conséquences
- Mesure `campaign_probe` 120 tours, graines 1-2 (avant → après) : guerre FR-EN 66 %/92 % → 64 %/57 % ; banqueroutes 122/87 → 112/138 ; révoltes 1/8 → 12/4 ; plus grosse faction inchangée (France 29-30 provinces).
- Valeurs à ajuster en données seulement. Les armées anciennes déjà sans chef gardent leur siège en cours.
