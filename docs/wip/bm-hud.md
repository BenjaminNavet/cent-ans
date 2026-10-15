# bm-hud : HUD de campagne (client Bevy)

Dossier `client/src/campaign/hud/` : `mod.rs` (barre du haut, alertes, bandeau d'armées, modes de carte, boutons de panneaux, dump `hud`), `flow.rs` (fin de tour sur thread, confirmation, rapport), `report.rs` (rapport de saison), `decision.rs` (fenêtre de décision de chronique), `text.rs` (infobulles).

Etat : fait et prouvé par script (`click hud:end_turn` : tour 0 -> 1, trésor 28 000 -> 49 559, 384 événements, rapport en modale).
Restes : minimap (`campaign_minimap.gd`, `minimap_controller.gd`), focus caméra sur alerte (lot caméra), brouillard sur alertes d'armées ennemies, chemin « confirmation » non exercé en script (aucune décision bloquante au tour 0), rejeu des tours IA, pastilles groupées de `end_turn_cluster.gd`, tracker de victoire.
