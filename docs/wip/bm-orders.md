# bm-orders : sélection et ordres de bataille (Bevy)

État : lots orders + bdeploy faits et prouvés. Dumps `battle_orders` (avec `CENT_ANS_ORDERS_DEMO=1`) et, pour bdeploy, `CENT_ANS_BDEPLOY_DEMO=1` (sections `deployment`, `formations`, `marks`, `cinematic`, `bdeploy_demo`).

bdeploy (porté) : phase de déploiement (`deploy.rs`, `deployment_controller.gd` : zone, bandeau, bouton Commencer, caméra cadrée, fantômes du glisser, places du cœur `plan_deployment`), préréglages de formation de groupe (`formations.rs`, `battle_formation_picker.gd` : panneau, Alt+Maj+1..6, fantômes, ordres `move` + verrou de groupe, proposition en déploiement), capacités Alt+1..4 (`hotkeys::ability_command`), points de passage numérotés et fantômes d'arrivée (`marks.rs`), signets de caméra Ctrl+F2..F4 / F2..F4 (`marks.rs`), plan cinématique au premier choc (`cinematic.rs`, `battle_cinematic.gd`).

Restes :
- Menu des formations de régiment (`battle_formation_menu.gd`) : côté HUD (message `IssueBattleCommand` prêt).
- Infobulles du sélecteur (`tooltip_for`) et des capacités (`battle_ability_icons.gd` : icônes, cadran de recharge) : lot hud.
- Plan de victoire (`start_victory`, `victory_shot`) : besoin de l'étendard du général vainqueur ; HUD non masqué pendant le plan.
- Trajets des ordres en file : segments droits (Godot : `preview_path_from` par segment).
- Rejeu de bataille (`battle_replay_controller.gd`) : demande de reconstruire la `BattleSession` depuis `sim_battle::replay` (lot scene), non commencé.
- Déploiement : désactivé avec `CENT_ANS_NO_DEPLOY=1` (ou pendant `CENT_ANS_ORDERS_DEMO`) ; par défaut toute bataille avec un camp joueur s'ouvre figée en déploiement.
- Points ouverts hérités : touches physiques (AZERTY), détection de l'interface sous le curseur par heuristique, `ground_point` sans test unitaire.
