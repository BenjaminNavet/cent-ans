class_name DiplomacyController
extends Node

## Branche la diplomatie et la religion (M5) sur la carte de campagne : bouton « Diplomatie »
## (touche P), panneau `DiplomacyPanel`, fenêtre des propositions reçues en fin de tour, modes
## de carte « Diplomatie » (touche N) et « Religion » (touche R). Séparé de `campaign_map.gd`
## pour garder chaque jalon dans son fichier ; `campaign_map` n'appelle que `setup`, `refresh`,
## `after_end_turn` et `handle_input`.

const RELATION_COLORS := {
	"self": Color(0.85, 0.75, 0.30), "war": Color(0.72, 0.12, 0.10), "truce": Color(0.85, 0.70, 0.20),
	"peace": Color(0.55, 0.55, 0.52), "alliance": Color(0.20, 0.40, 0.78), "vassal": Color(0.50, 0.25, 0.65),
	"suzerain": Color(0.50, 0.25, 0.65),
}

enum MapMode { NONE, DIPLOMACY, RELIGION }

var map: Node = null  # CampaignMap
var panel: DiplomacyPanel
var button: Button
var mode: MapMode = MapMode.NONE


func setup(campaign_map: Node) -> void:
	map = campaign_map
	panel = DiplomacyPanel.new()
	panel.name = "DiplomacyPanel"
	panel.hide()
	map.ui.add_child(panel)
	panel.order_requested.connect(_on_order_requested)
	panel.offer_answered.connect(_on_offer_answered)
	var court_button: Button = map.ui.court_button
	button = Button.new()
	button.text = "Diplomatie"
	button.add_theme_font_size_override("font_size", 18)
	button.tooltip_text = "Diplomatie et religion (P)"
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
	panel.refresh()
	if faction_id != "":
		panel.select_faction(faction_id)
	panel.show()


## Après tout changement d'état.
func refresh() -> void:
	if not available():
		return
	if panel.visible:
		panel.refresh()
	match mode:
		MapMode.DIPLOMACY:
			_color_by_relation()
		MapMode.RELIGION:
			_color_by_religion()
		_:
			pass


## Après `end_turn` : ouvre le panneau sur les propositions reçues s'il y en a.
func after_end_turn() -> void:
	if not available():
		return
	var offers: Array = map.sim.call("get_offers")
	if not offers.is_empty():
		open_panel()
		map.ui.show_toast("%d proposition(s) diplomatique(s) en attente." % offers.size())


func handle_input(event: InputEvent) -> bool:
	if not (event is InputEventKey) or not event.pressed or event.echo:
		return false
	match (event as InputEventKey).physical_keycode:
		KEY_P:
			toggle_panel()
			return true
		KEY_N:
			_toggle_mode(MapMode.DIPLOMACY)
			return true
		KEY_R:
			_toggle_mode(MapMode.RELIGION)
			return true
	return false


func _toggle_mode(target: MapMode) -> void:
	if not available():
		return
	mode = MapMode.NONE if mode == target else target
	if mode == MapMode.NONE:
		map.refresh_all()
		map.ui.show_toast("Carte politique.")
		return
	map.unrest_mode = false
	refresh()
	map.ui.show_toast("Carte diplomatique : rouge guerre, bleu alliés, violet vassaux, jaune trêve." if mode == MapMode.DIPLOMACY else "Carte religieuse : bleu Avignon, or Rome, vert hérésie.")


func _province_ids() -> PackedStringArray:
	var ids := PackedStringArray()
	for index in range(1, map.map_data.province_count + 1):
		ids.append(str(map.map_data.get_province(index).get("id", "")))
	return ids


func _color_by_relation() -> void:
	var relations: PackedStringArray = map.sim.call("get_province_relations", _province_ids())
	var colors := PackedColorArray()
	for relation in relations:
		var color: Color = RELATION_COLORS.get(relation, Color(0, 0, 0, 0))
		if relation == "":
			color.a = 0.0
		colors.append(color)
	map.terrain.set_province_colors(colors)


func _color_by_religion() -> void:
	var colors := PackedColorArray()
	for id in _province_ids():
		var info: Dictionary = map.sim.call("get_province_religion", id) if id != "" else {}
		if info.is_empty():
			colors.append(Color(0, 0, 0, 0))
			continue
		var state: Dictionary = map.sim.call("get_province_state", id)
		var controller := str(state.get("controller", ""))
		var faith: Dictionary = map.sim.call("get_religion_state", controller) if controller != "" else {}
		var religion := str(faith.get("religion", info.get("religion", "")))
		var base := Color(0.25, 0.35, 0.70)  # Église (Avignon pendant le Schisme)
		if religion == "rel_catholic_rome":
			base = Color(0.80, 0.65, 0.20)
		elif religion != "rel_catholic":
			base = Color(0.45, 0.45, 0.45)
		var heresy := float(info.get("heresy", 0)) / 100.0
		colors.append(base.lerp(Color(0.20, 0.65, 0.25), clampf(heresy * 1.5, 0.0, 1.0)))
	map.terrain.set_province_colors(colors)


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
