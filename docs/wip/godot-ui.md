# WIP — Interface Godot M2 (reprise possible)

État courant et prochaine étape, mis à jour à chaque commit `wip:`.

## Fait
- `game/scripts/sim/campaign_sim_mock.gd` : simulation factice (API § 2 du spec M2).
- `game/scripts/sim/sim_facade.gd` (autoload `SimFacade`) : sim réelle si `CampaignSim.get_army_ids` existe, sinon mock ; `GameDataStore` ; sauvegardes `user://saves/*.json` (enveloppe + `save_to_string`).
- `scenes/start_menu.tscn` (scène principale) : 3 cartes (nom/blason/couleur du store), graine, Commencer / Charger / Quitter.
- HUD `campaign_map.tscn` : barre (faction, trésor, revenu, date, Fin du tour + Entrée, menu), journal repliable, `army_panel.tscn`, `province_panel.tscn` étendu (garnison, recrutement, former une armée), `save_load_dialog.tscn`, thème `parchment_theme.tres`.
- Carte : `army_marker.tscn` + `ArmyMarkers`, `PathPreview`, masque atteignable/chemin dans le shader, clic droit = ordre de déplacement, heightmap via `GameDataStore.load_heightmap_u16` (Png16 en repli).
- `tests/smoke.gd` étendu : vert avec le mock.

## Prochaine étape
- Captures d'écran réelles (`docs/img/godot-campaign-hud.png`, `godot-start-menu.png`), corrections de mise en page.
- Rebuild `core/build.sh` quand la vraie `CampaignSim` arrive, valider smoke + capture, `docs/godot-map.md`.
