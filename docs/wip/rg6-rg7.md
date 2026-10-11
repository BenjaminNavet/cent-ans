# RG6 + RG7 — état

Branche : worktree-agent-a0712ffe766bfcd6d (pas de fusion dans main).

## RG6 (résolution auto et effets du général)
- `Side.general_experience_flat/percent` (battle_auto.rs) : `ArmyExperience` du général, appliqué à
  l'expérience de chaque régiment (plafond 10) dans le facteur qualité ; chef de coalition = ses effets.
- `ArmyRanged` : points de tir ajoutés comme la tradition de tir (flat + percent), pour les unités à
  `ranged > 0` (movement/battle.rs `side_from_army`, battle_request.rs `side_setup` pour la bataille jouée).
- Expérience en bataille jouée : `effective_experience` arrondie.
- Infobulle `battle_balance_estimate` (data/ui/tooltips.json) liste les facteurs comptés.

## RG7 (recruter depuis une armée en territoire propre + fusion)
- Cœur : `order_recruit_into` (orders/recruit.rs) ; `holds_province`, `army_recruit_blocker`,
  `recruit_site_for_army`. Livraison (economy.rs `deliver_to_armies`) si l'armée est encore dans la province.
- Donnée : `economy.field_recruit_surcharge_percent` (schéma + data-model, défaut 0).
- Pont : `army_dict` (recruit_blocker, recruit_site, merge_targets, joueur seulement) ; `recruit_armies`
  du détail de colonie inclut les armées de campagne de la province (`field`).
- UI : ArmyStrip boutons « Recruter » / « Fusionner avec… » ; hud_controller ; settlement_panel.open_recruit_for_army.

## Reste
- Vérifs complètes (clippy, tests, build GDExtension, smoke) ; test Godot de l'état du bouton.
