class_name DiplomacyController
extends Node

## Branche la diplomatie et la religion (M5) sur la carte de campagne : bouton « Diplomatie »
## (touche P), panneau `DiplomacyPanel`, fenêtre des propositions reçues en fin de tour. Les modes
## de carte « Diplomatie » et « Religion » vivent dans `MapModeController` (lot MF1). Séparé de
## `campaign_map.gd` pour garder chaque jalon dans son fichier ; `campaign_map` n'appelle que
## `setup`, `refresh`, `after_end_turn` et `handle_input`.

var map: Node = null  # CampaignMap
var panel: DiplomacyPanel
var button: Button
## Q5 : propositions déjà présentées au joueur (id → true). Le panneau ne s'ouvre seul que pour
## une offre nouvelle ; les suivantes restent signalées par la pastille « Diplomatie ».
var _seen_offers: Dictionary = {}


func setup(campaign_map: Node) -> void:
	map = campaign_map
	panel = DiplomacyPanel.new()
	panel.name = "DiplomacyPanel"
	panel.hide()
	map.ui.add_child(panel)
	panel.order_requested.connect(_on_order_requested)
	panel.offer_answered.connect(_on_offer_answered)
	panel.arbitration_requested.connect(_on_arbitration_requested)
	var court_button: Button = map.ui.court_button
	button = Button.new()
	button.text = "Diplomatie"
	button.add_theme_font_size_override("font_size", UiType.size(UiType.CAPTION))
	RichTooltip.attach_plain(button, "diplomacy_open")
	court_button.get_parent().add_child(button)
	court_button.get_parent().move_child(button, court_button.get_index())
	button.pressed.connect(toggle_panel)


func available() -> bool:
	return map != null and map.sim != null and map.sim.has_method("get_diplomacy")


func toggle_panel() -> void:
	if panel.visible:
		panel.hide()
		return
	open_panel()


func open_panel(faction_id: String = "") -> void:
	if not available():
		map.ui.show_toast("Diplomatie indisponible avec cette simulation.", true)
		return
	panel.sim = map.sim
	panel.player_faction = map.player_faction
	panel.province_name_of = map.province_name_of
	panel.map_data = map.map_data  # DP1 : carte des relations de l'écran
	panel.refresh()
	# Panneau central : ferme les panneaux latéraux qu'il recouvrirait.
	map.ui.hide_province()
	if faction_id != "":
		panel.select_faction(faction_id)
	panel.show()


## Après tout changement d'état.
func refresh() -> void:
	if not available():
		return
	if panel.visible:
		panel.refresh()


## Après `end_turn` : ouvre le panneau sur les propositions reçues s'il y en a.
func after_end_turn() -> void:
	if not available():
		return
	var offers: Array = map.sim.call("get_offers")
	var fresh := 0
	for offer in offers:
		var offer_id := int(offer.get("id", -1))
		if not _seen_offers.has(offer_id):
			_seen_offers[offer_id] = true
			fresh += 1
	if fresh > 0:
		open_panel()
		map.ui.show_toast("%s en attente." % FrText.count(offers.size(), "proposition diplomatique", "propositions diplomatiques"))


func handle_input(event: InputEvent) -> bool:
	if not (event is InputEventKey) or not event.pressed or event.echo:
		return false
	# U7 : actions de l'InputMap (fiche des raccourcis générée depuis celle-ci).
	if event.is_action_pressed("map_toggle_diplomacy"):
		toggle_panel()
		return true
	return false


func _on_order_requested(order: Dictionary, success_text: String) -> void:
	var result: Dictionary = map.sim.call("submit_order", order)
	if result.get("ok", false):
		map.ui.show_toast(success_text)
	else:
		map.ui.show_toast(str(result.get("error", "Ordre refusé")), true)
	map.refresh_all()


func _on_offer_answered(offer_id: int, accept: bool) -> void:
	var result: Dictionary = map.sim.call("answer_offer", offer_id, accept)
	if result.get("ok", false):
		map.ui.show_toast("Proposition acceptée." if accept else "Proposition repoussée.")
	else:
		map.ui.show_toast(str(result.get("error", "Réponse impossible")), true)
	map.refresh_all()


## FE6 : verdict du joueur sur une guerre privée entre deux de ses vassaux.
func _on_arbitration_requested(offer_id: int, verdict: String, side: String) -> void:
	var result: Dictionary = map.sim.call("feudal_arbitrate", offer_id, verdict, side)
	if result.get("ok", false):
		map.ui.show_toast("Arbitrage rendu.")
	else:
		map.ui.show_toast(str(result.get("error", "Arbitrage impossible")), true)
	map.refresh_all()
