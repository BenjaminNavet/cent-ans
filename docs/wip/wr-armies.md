# WR armies — état

Branche wr/armies (worktree ../gp-wr-armies). ADR 0305. FAIT, prêt à fusionner.
- Renforts lointains : part engagée selon la distance (`reinforce_full_radius_km` 10, `reinforce_min_percent` 40), appliquée en auto et en 3D (pas d'arrivée datée dans sim-battle), mouvement dépensé dans `apply_battle_result`, aperçu « à N km, ~X % ».
- Personnages : `Order::SendCharacter` + `journey`, IA `char_moves.rs`, réserve de commandant tant qu'une armée est sans chef.
- Sonde campaign_probe étendue (batailles, armées sans chef). Chiffres dans l'ADR / rapport.
Reste : interface joueur pour SendCharacter ; arrivée datée des renforts en bataille 3D ; contrôle visuel de « ~X % ».
