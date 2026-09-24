class_name SiegeController
extends Node

## Sièges (M8) côté carte : ligne d'état du siège et bouton « Donner l'assaut » ajoutés à la
## boîte d'actions au-dessus du bandeau d'ost (`MapUI.army_actions_box`, F10b) quand l'armée
## sélectionnée du joueur assiège une place. Séparé de `campaign_map.gd` ;
## celui-ci n'appelle que `setup` et `on_army_shown`.

var map: Node = null  # CampaignMap
var box: VBoxContainer
var status_label: Label
var assault_button: Button
var _army_id: String = ""


func setup(campaign_map: Node) -> void:
	map = campaign_map
	box = VBoxContainer.new()
	box.name = "SiegeBox"
	status_label = Label.new()
	status_label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	box.add_child(status_label)
	assault_button = Button.new()
	assault_button.text = "Donner l'assaut"
	assault_button.pressed.connect(_on_assault)
	box.add_child(assault_button)
	map.ui.army_actions_box.add_child(box)
	box.hide()


func available() -> bool:
	return map != null and map.sim != null and map.sim.has_method("get_assault_odds")


## Appelé après chaque affichage du bandeau d'ost.
func on_army_shown(army_id: String, is_player: bool) -> void:
	_army_id = army_id
	_update(is_player)
	map.ui.queue_layout()


func _update(is_player: bool) -> void:
	if not available() or not is_player:
		box.hide()
		return
	var odds: Dictionary = map.sim.call("get_assault_odds", _army_id)
	if not bool(odds.get("available", false)):
		box.hide()
		return
	var walls_text := "murailles intactes" if bool(odds["walls"]) else "brèche ouverte"
	status_label.text = "Siège : vivres %d %% (reddition dans ~%d tour(s)), brèche %d %% — %s." % [
		int(odds["supplies"]), int(odds["turns_left"]), int(odds["breach"]), walls_text]
	assault_button.text = "Donner l'assaut (chances ≈ %d %%)" % int(odds["odds"])
	box.show()


func _on_assault() -> void:
	if _army_id == "":
		return
	var result: Dictionary = map.sim.call("submit_order", {"type": "assault", "army": _army_id})
	if result.get("ok", false):
		var pending: Array = map.sim.call("get_pending_events")
		map.ui.show_toast(str(pending[-1].get("text_fr", "L'assaut est donné.")) if not pending.is_empty() else "L'assaut est donné.")
		# M8 § 2 : avec les batailles interactives, l'assaut attend en bataille de siège.
		if map.has_method("_offer_pending_battles"):
			map._offer_pending_battles()
	else:
		map.ui.show_toast(str(result.get("error", "Assaut impossible")), true)
	map.refresh_all()
