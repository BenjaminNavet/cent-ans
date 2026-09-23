# WIP — Interface Godot M2 (reprise possible)

État courant et prochaine étape, mis à jour à chaque commit `wip:`.

## Fait
- `game/scripts/sim/campaign_sim_mock.gd` : simulation factice (API § 2 du spec M2).
- `game/scripts/sim/sim_facade.gd` (autoload `SimFacade`) : choisit sim réelle/mock, charge `GameDataStore`, sauvegardes `user://saves/`.

## Prochaine étape
- Écran de démarrage `scenes/start_menu.tscn`, HUD de campagne, marqueurs d'armées.
