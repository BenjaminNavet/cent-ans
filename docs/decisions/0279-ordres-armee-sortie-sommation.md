# 0279 — Recrues dans l'armée, sortie, sommation, chevauchée nourricière, repos de l'IA

Statut : accepté

## Contexte
Le rapport WH `armees` (A5, A11, A9, A13) et `ui` (#10) demandent cinq compléments de la guerre de siège et de campagne, sans refonte du flux de bataille.

## Décision
- **Recruter dans l'armée (A5)** : nouvel ordre `RecruitInto { settlement, unit_type, army }` (variante distincte de `Recruit`, pour ne pas toucher la trentaine d'appels existants) ; `QueuedRecruit.into_army` (`serde(default)`). À la livraison (`economy.rs::deliver_to_armies`), l'unité rejoint l'armée si elle est toujours dans la place, de la même faction et sous le plafond `max_units` ; sinon, elle va à la garnison. Ordre refusé si l'armée est pleine, étrangère ou ailleurs.
- **Sortie (A11)** : ordre `Sortie { settlement }` ; `siege::sortie` prend un paramètre `forced` qui supprime le seuil 1,3 de l'IA. Toujours auto-résolue (pas de `BattleRequest` interactif : écart à la spec, point ouvert). Victoire : siège levé ; défaite : pertes de la garnison, siège maintenu.
- **Sommation (ui #10)** : ordre `DemandSurrender { settlement }` du seul assiégeant. `siege.surrender` de `data/rules/capture.json` : refus d'office tant que les vivres ≥ `refuse_from_supplies` (50) ; chance = `supplies_weight` × (50 − vivres) + `breach_weight` × brèche ; certitude à vivres 0 et brèche ≥ `breach_open`. Le tirage est un hachage stable (tour, place, graine) : redemander le même tour ne change rien, sans nouveau champ dans `SiegeState`. Acceptée, la garnison est dissoute et la place prise (comme à la famine) ; l'interface affiche la chance exacte.
- **Chevauchée nourricière (A9)** : `capture.json:raid.supply_gain_percent` (20) rend des vivres à l'armée qui ravage une terre hostile ; rendement décroissant : butin × `diminishing_loot_percent` (50 %) quand la dévastation de la province dépasse `diminishing_devastation` (70). Écart : la spec disait « après N saisons » ; le seuil de dévastation en est l'image exacte (30 par chevauchée) et ne demande aucun compteur.
- **Repos de l'IA (A13)** : le repos en place existait déjà (NT6c, `rest_plan`, 60 % / 85 %). Manquait le trajet : `grid.json:postures.rest.seek_place` ; une armée affaiblie en terres amies, hors de toute place et sans ennemi à portée, rejoint la place amie la plus proche dans le mouvement du tour (`Objective::Retreat`, `pick_rest_place`), puis `rest_plan` la garde.

## Conséquences
- Nouveaux champs de données avec valeurs par défaut sûres ; schémas à jour.
- L'IA ne se sert ni de `RecruitInto`, ni de `Sortie`, ni de `DemandSurrender`.
