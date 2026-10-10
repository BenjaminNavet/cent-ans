# 0306 — Sortie jouée en bataille (WR sortie)

Statut : accepté.

## Contexte
La sortie de la garnison assiégée (WH armyb, ADR 0279 ; sortie automatique de M8 quand la garnison écrase les assiégeants) était résolue sur-le-champ, même quand le joueur y prenait part. Un assaut ou une bataille de campagne, eux, passent par `pending_battles` et le dialogue d'avant-bataille (« Livrer bataille » en 3D ou « Résolution automatique »).

## Décision
- La sortie est une bataille de campagne en attente : `BattleRequest.sortie` (`#[serde(default)]`, absent des sauvegardes anciennes). `attacker` (= `defender`) est l'armée assiégeante de tête, `location` la place ; **la garnison est le camp attaquant**, la coalition assiégeante le camp défenseur.
- Déclencheurs : l'ordre `Sortie` du joueur, la sortie automatique de `resolve_sieges` (garnison > 1,3 × assiégeants) et celle de l'IA (`Order::Sortie`) dès que le joueur y est partie (sa garnison, ou son armée parmi les assiégeants) et que `interactive_battles` est actif. Sans le joueur, ou batailles non interactives : résolution immédiate comme avant. Pendant la sortie en attente, le siège ne progresse pas ce tour-là.
- Réutilisation : `siege::sortie` est scindée en `sortie_setup` (camps, profils, cohérents avec la prévision), `apply_sortie_result` (effets communs : pertes, captifs, journal, historique, missions, ferveur, retraite des assiégeants battus, levée du siège) et `auto_sortie`. Le résultat d'une bataille 3D passe par `resolve_pending_battle` → `apply_sortie_result`, d'où les mêmes effets que l'auto-résolution. « Résolution automatique » et la résolution en début de tour suivant appellent `auto_sortie` (sortie forcée, sans seuil, puisqu'elle est déjà décidée).
- Carte : le **terrain de la province de la place**, sans murs ni `SiegeSetup` (bataille de campagne ordinaire, aux abords de la ville). Le générateur de siège n'est pas utilisé : la sortie n'a aucune muraille à prendre et son plan de ville n'apporte rien à un combat en rase campagne ; c'est le plus simple qui fonctionne.
- Prévision et retraite : `forecast_request` bâtit les mêmes camps que l'auto-résolution ; le joueur dont la garnison sort peut renoncer (« Rester dans la place », sortie annulée, aucune perte de moral) ; attaqué par l'IA il ne le peut pas. `is_live` : l'armée de tête est toujours aux abords d'une place garnie, sans exiger l'objet de siège (il peut naître le même tour).
- Pont : `get_pending_battles` ajoute `sortie`, `attacker_strength` = effectif de la garnison ; `debug_stage_sortie` (tests). UI : `PreBattleDialog` (titre « Sortie de X », « Livrer bataille », « Rester dans la place », rôles garnison / assiégeants), `settlement_controller` ouvre le dialogue après l'ordre.

## Conséquences
- Aucune valeur de règle nouvelle ; aucune donnée ajoutée.
- Les tests qui s'appuyaient sur la sortie immédiate d'un joueur fixent `interactive_battles = false`.
- Le lot ai-mil (IA qui émet des `Sortie`) en profite sans changement : pendant son tour, une sortie contre une armée du joueur devient une bataille en attente.
- Limite : un joueur qui n'ouvre pas le dialogue voit la sortie résolue automatiquement au début de la fin de tour suivante (comme tout `pending_battles`).
