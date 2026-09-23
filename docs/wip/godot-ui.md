# WIP — Interface Godot M2 (reprise possible)

État courant et prochaine étape, mis à jour à chaque commit `wip:`.

## Fait (tout vert avec le mock : `godot --headless --path game --script res://tests/smoke.gd`)
- `scripts/sim/campaign_sim_mock.gd` (API § 2), `scripts/sim/sim_facade.gd` (autoload `SimFacade`, réel/mock, store, sauvegardes).
- `scenes/start_menu.tscn` (scène principale), HUD complet dans `campaign_map.tscn`, `army_panel.tscn`, `province_panel.tscn` étendu, `save_load_dialog.tscn`, thème `parchment_theme.tres`.
- Carte : `army_marker.tscn` + `ArmyMarkers`, `PathPreview`, masque atteignable/chemin, clic droit = ordre, heightmap via `GameDataStore.load_heightmap_u16` (little-endian, `Png16` en repli).
- Captures : `docs/img/godot-campaign-hud.png`, `godot-campaign-province.png`, `godot-start-menu.png` (mock).
- `docs/godot-map.md` réécrit (M1 + M2).

## Prochaine étape
- Quand `core/crates/godot-bridge` expose l'API M2 (`CampaignSim.get_army_ids`) : `core/build.sh`, relancer le smoke (branche « REAL »), refaire `docs/img/godot-campaign-hud.png`, corriger les écarts d'API éventuels.
- Commit final et suppression de ce fichier WIP si le travail est terminé.
