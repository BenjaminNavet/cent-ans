# 0301 — L'IA de campagne utilise recruter-dans-une-armée, sortie et sommation

Statut : accepté

## Contexte
Le lot WH armyb (ADR 0279) a ajouté `Order::RecruitInto`, `Order::Sortie` et `Order::DemandSurrender`, mais
l'IA n'en émettait aucun : ses recrues allaient toujours en garnison (puis `CreateArmy`/fusion), ses garnisons
assiégées attendaient la sortie automatique (garnison > 1,3 × la coalition assiégeante) ou la famine, et ses
assiégeants allaient directement à l'assaut ou attendaient la capitulation.

## Décision
Module neuf `core/crates/ai/src/campaign/military_orders.rs`, deux points d'accroche dans `plan_turn_in` :
- après l'économie : `recruit_into_armies` réécrit les `Recruit` d'une colonie en `RecruitInto` quand une armée de
  la faction y stationne sous `recruit_into_target_units` (file d'attente comptée), hors siège et si la garnison
  garde `recruit_into_min_garrison` unités ; la plus petite armée d'abord ;
- avant les armées : `plan_siege_orders`. Garnison assiégée : `Sortie` si sa part de puissance (garnison /
  garnison + assiégeants, même mesure que `unit_power`/`army_power` du calcul d'assaut) atteint `sortie_odds`, ou
  si les vivres sont ≤ `sortie_starving_supply` et la part ≥ `sortie_desperate_odds` (la reddition par famine
  perdrait la garnison de toute façon). Assiégeant présent sur la place : `DemandSurrender` quand
  `siege::surrender_chance` ≥ `surrender_min_chance`, tous les `surrender_retry_turns` tours depuis le début du
  siège (le jet est déterministe par tour et par place : sans mémoire de l'échec, une cadence fixe évite de
  répéter la sommation chaque tour tout en redonnant sa chance au tour suivant tiré). La sommation précède l'assaut
  du même tour (l'ordre d'assaut échoue sans effet si la place s'est rendue).
Seuils : `data/ai/campaign.json` clé `military_orders` (optionnelle, valeurs par défaut identiques), schéma
`ai_campaign.schema.json`.

## Conséquences
- Pas d'état supplémentaire dans la sauvegarde ; la cadence de sommation dépend de `started_turn`.
- `campaign_probe` compte sorties, redditions par sommation et refus (événements).
- Désactiver un ordre = seuil inatteignable (`sortie_odds` 101, `surrender_min_chance` 101, `recruit_into_target_units` 1).
- Non fait : sortie coordonnée avec une armée de secours, sommation par l'IA contre le joueur avec ultimatum.
