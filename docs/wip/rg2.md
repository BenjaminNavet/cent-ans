# RG2 — pastilles « Avis » et fenêtre de croisade

État : FAIT. Vérifié : clippy -D warnings, `cargo test -p godot-bridge`, `rg2_test.gd`, `smoke.gd`.

- Libellés : `trade`, `mission`, `royal_act`, `level_up`, `loyalty`, `captain` dans `end_turn_cluster.gd`.
- Itinéraires : `game/scripts/ui/alert_routes.gd` (`AlertRoutes.route_of`, jamais vide) ; actions dans `HudController._on_alert_activated` ; repli journal (`open_journal`).
- Croisade : `CrusadeWindow` + `HudController.show_crusade` ; getter `CampaignSim.get_papal_crusade`.
- Reste : œil du joueur sur la fenêtre ; pas de test de bout en bout sur une vraie campagne avec appel actif.
