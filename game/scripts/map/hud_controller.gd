class_name HudController
extends Node

## HUD « à la Total War » branché sur la carte (lot F10b) : bandeau d'ost + sceau du chef pour
## l'armée sélectionnée, cloche de fin de saison et ses alertes, lettres scellées. Les
## composants vivent dans `MapUI` (nœuds `ArmyStrip`, `GeneralSeal`, `EndTurnCluster`,
## `NewsLetters`) ; ce contrôleur les alimente depuis la simulation et relie leurs signaux.
## `campaign_map.gd` n'appelle que `setup`, `show_army`, `refresh` et `after_end_turn`.
## Aucune règle de jeu : lecture de l'état et envoi des ordres existants.

var map: Node = null  # CampaignMap
var last_events: Array = []


func setup(campaign_map: Node) -> void:
	map = campaign_map


## Remplit le bandeau et le sceau pour l'armée `army_id` (déjà sélectionnée par la carte).
func show_army(_army_id: String, _army: Dictionary, _is_player: bool) -> void:
	pass


## Après tout changement d'état : date de la cloche et alertes.
func refresh() -> void:
	pass


## Après `end_turn` : événements du tour (alertes « construction achevée »…).
func after_end_turn(events: Array) -> void:
	last_events = events
