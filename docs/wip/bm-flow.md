# bm-flow — flux campagne ↔ bataille (client Bevy)

État : terminé (voir `client/README.md` § Flux). `game-api::flow` (vues + tests), `client/src/flow/` (dialogue d'avant-bataille, rencontres, carte de chargement, avis de résultat, retour de bataille, accroches `CENT_ANS_*`), édition minimale de `battle/scene/mod.rs` (lit `flow::CampaignBattle`, `session_from_sim`).
Preuves : `--script` campagne → rencontre → auto-résolution ; campagne → bataille → `LeaveBattle` → campagne ; abandon = retraite (dumps `out/flow*.json`).
Restes : bannière illustrée / minicarte du site / météo prévue du dialogue (`pre_battle_dialog.gd`) ; autosave avant bataille (`flow_controller.gd`) ; batailles navales (`NavalCampaign.offer`) ; écran de fin 3D (lot battle/hud : doit envoyer `LeaveBattle`) ; historique de site (`historical_site`, EP7) non transmis à la bataille.
