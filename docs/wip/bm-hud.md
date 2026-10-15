# bm-hud : HUD de campagne (client Bevy)

Dossier `client/src/campaign/hud/` : `mod.rs` (barre du haut, alertes, bandeau d'armées, modes de carte, boutons de panneaux, dump `hud`), `flow.rs` (fin de tour sur thread, confirmation, rapport), `report.rs` (rapport de saison), `decision.rs` (fenêtre de décision de chronique), `text.rs` (infobulles).

Etat : squelette + tout compile. Prochaine étape : preuve par script (`--script`) puis restes.
Restes : minimap (`campaign_minimap.gd`), focus caméra sur alerte (lot caméra), brouillard sur alertes d'armées ennemies.
