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
	button.add_theme_font_size_override("font_size", 15)
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
		map.ui.show_toast("%s en attente." % FrText.count(offers.size(), "proposition diplomatique", "propositions diplomatiques"))


func handle_input(event: InputEvent) -> bool:
	if not (event is InputEventKey) or not event.pressed or event.echo:
		return false
	# U7 : actions de l'InputMap (fiche des raccourcis générée depuis celle-ci).
	if event.is_action_pressed("map_toggle_diplomacy"):
		toggle_panel()
		return true
	if event.is_action_pressed("map_mode_diplomacy"):
		_toggle_mode(MapMode.DIPLOMACY)
		return true
	if event.is_action_pressed("map_mode_religion"):
		_toggle_mode(MapMode.RELIGION)
		return true
	return false


func _toggle_mode(target: MapMode) -> void:
	if not available():
		return
	mode = MapMode.NONE if mode == target else target
	if mode != MapMode.DIPLOMACY:
		_clear_relation_markers()
		_sync_minimap(false)
	# DP1 (audit A3, C11/U14) : teinte franche sur la carte 3D et légende permanente, que le
	# panneau de province ne recouvre plus (il est refermé en entrant dans le mode).
	_set_tint_boost(mode != MapMode.NONE)
	_show_legend(mode)
	if mode == MapMode.NONE:
		map.refresh_all()
		map.ui.show_toast("Carte politique.")
		return
	map.unrest_mode = false
	map.ui.hide_province()
	refresh()


func _province_ids() -> PackedStringArray:
	var ids := PackedStringArray()
	for index in range(1, map.map_data.province_count + 1):
		ids.append(str(map.map_data.get_province(index).get("id", "")))
	return ids


func _color_by_relation() -> void:
	var ids := _province_ids()
	# DP2 : positions diplomatiques (allié, accord, neutre, tension, guerre, vassal).
	if DiplomaticStances.available(map.sim):
		var stances := DiplomaticStances.stances(map.sim, ids)
		var stance_colors := PackedColorArray()
		for key in stances:
			stance_colors.append(DiplomaticStances.color_of(key))
		map.terrain.set_province_colors(stance_colors)
		_place_relation_markers(ids, _symbol_keys(stances))
		_sync_minimap(true)
		return
	var relations: PackedStringArray = map.sim.call("get_province_relations", ids)
	var colors := PackedColorArray()
	for relation in relations:
		var color: Color = RELATION_COLORS.get(relation, Color(0, 0, 0, 0))
		if relation == "":
			color.a = 0.0
		colors.append(color)
	map.terrain.set_province_colors(colors)
	_place_relation_markers(ids, relations)


## DP2 : clés de symbole daltonien (relations U12) des positions diplomatiques.
func _symbol_keys(stances: PackedStringArray) -> PackedStringArray:
	var keys := PackedStringArray()
	for key in stances:
		keys.append(str(DiplomaticStances.SYMBOL_KEYS.get(key, "")))
	return keys


## DP2 : la minicarte suit le mode « Diplomatie » de la carte (et réciproquement, voir
## `MinimapController`).
func _sync_minimap(on: bool) -> void:
	var minimap: CampaignMinimap = map.ui.get("minimap") if map != null and map.ui != null else null
	if minimap == null:
		return
	if on:
		minimap.set_diplomacy_colors(DiplomaticStances.colors_for(map.sim, _province_ids()))
		if minimap.mode != CampaignMinimap.MODE_DIPLOMACY:
			minimap.set_mode(CampaignMinimap.MODE_DIPLOMACY)
	elif minimap.mode == CampaignMinimap.MODE_DIPLOMACY:
		minimap.set_mode(CampaignMinimap.MODE_POLITICAL)


## DP2 : entre dans le mode « Diplomatie » ou en sort (bouton de la minicarte).
func set_diplomacy_mode(on: bool) -> void:
	if (mode == MapMode.DIPLOMACY) == on:
		return
	_toggle_mode(MapMode.DIPLOMACY)


## Lot U12 : en mode daltonien, un symbole par province sur la carte diplomatique (⚔ guerre,
## ⚭ alliance, ⚜ vassal ou suzerain, ⌛ trêve), lisible sans distinguer les couleurs.
const MARKER_FONT := "res://assets/third_party/fonts/noto/NotoSansSymbols2-Subset.ttf"
const MARKER_FONT_MAIN := "res://assets/third_party/fonts/noto/NotoSansSymbols-Subset.ttf"
const MARKED_RELATIONS := ["war", "alliance", "vassal", "suzerain", "truce"]
var _markers: Node3D


func _place_relation_markers(ids: PackedStringArray, relations: PackedStringArray) -> void:
	_clear_relation_markers()
	if not Accessibility.colorblind() or not (map is Node3D):
		return
	_markers = Node3D.new()
	_markers.name = "RelationMarkers"
	map.add_child(_markers)
	var font := FontVariation.new()
	font.base_font = load(MARKER_FONT_MAIN)
	font.fallbacks = [load(MARKER_FONT)]
	for index in mini(ids.size(), relations.size()):
		var relation := relations[index]
		if not MARKED_RELATIONS.has(relation):
			continue
		var centroid: Vector2 = map.map_data.centroid_of_id(ids[index])
		var label := Label3D.new()
		label.text = Accessibility.relation_symbol(relation)
		label.font = font
		label.font_size = 72
		label.outline_size = 14
		label.outline_modulate = Color(0.97, 0.93, 0.82)
		label.modulate = (RELATION_COLORS.get(relation, Color.BLACK) as Color).darkened(0.35)
		label.billboard = BaseMaterial3D.BILLBOARD_ENABLED
		label.no_depth_test = true
		label.fixed_size = true
		label.pixel_size = 0.0009
		label.position = Vector3(centroid.x, map.map_data.surface_world_at(centroid.x, centroid.y) + 0.5, centroid.y)
		_markers.add_child(label)


func _clear_relation_markers() -> void:
	if _markers != null and is_instance_valid(_markers):
		_markers.queue_free()
	_markers = null


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


# ----- DP1 : lisibilité des modes de carte (audit A3, C11 et U14) ---------------------------------

## Teinte des provinces à courte distance dans les modes diplomatie et religion (le rendu
## politique normal la garde discrète pour laisser voir le terrain).
const MODE_TINT_NEAR := 0.62
const MODE_TINT_FAR := 0.8
const MODE_SATURATION := 0.9
var _tint_saved: Dictionary = {}
var _legend: PanelContainer


func _set_tint_boost(on: bool) -> void:
	var terrain: Node = map.get("terrain") if map != null else null
	var material: ShaderMaterial = terrain.get("material") if terrain != null else null
	if material == null:
		return
	var params := {"faction_alpha_near": MODE_TINT_NEAR, "faction_alpha_far": MODE_TINT_FAR, "faction_saturation": MODE_SATURATION}
	if on:
		if _tint_saved.is_empty():
			for key in params:
				_tint_saved[key] = material.get_shader_parameter(key)
		for key in params:
			material.set_shader_parameter(key, params[key])
	elif not _tint_saved.is_empty():
		for key in _tint_saved:
			material.set_shader_parameter(key, _tint_saved[key])
		_tint_saved.clear()


func _show_legend(target: MapMode) -> void:
	if _legend != null and is_instance_valid(_legend):
		_legend.queue_free()
	_legend = null
	if target == MapMode.NONE or map == null or map.ui == null:
		return
	_legend = PanelContainer.new()
	_legend.name = "MapModeLegend"
	_legend.theme = load("res://scenes/ui/parchment_theme.tres")
	_legend.add_theme_stylebox_override("panel", HudStyle.panel_box(8))
	_legend.mouse_filter = Control.MOUSE_FILTER_IGNORE
	var box := VBoxContainer.new()
	box.add_theme_constant_override("separation", 2)
	_legend.add_child(box)
	var diplomacy := target == MapMode.DIPLOMACY
	box.add_child(HudStyle.label("Carte diplomatique (N)" if diplomacy else "Carte religieuse (R)", HudStyle.FONT_TITLE, HudStyle.RUBRIC))
	var row := HBoxContainer.new()
	row.add_theme_constant_override("separation", 12)
	var entries: Array = [["self", "Nous"], ["war", "Guerre"], ["truce", "Trêve"], ["alliance", "Alliés"], ["vassal", "Vassaux"], ["peace", "Neutres"]]
	if diplomacy and DiplomaticStances.available(map.sim):
		entries = []  # DP2 : légende des positions diplomatiques
		box.add_child(DiplomaticStances.legend(HudStyle.FONT_BODY, 16.0, false))
	if not diplomacy:
		entries = [[Color(0.25, 0.35, 0.70), "Obédience d'Avignon"], [Color(0.80, 0.65, 0.20), "Obédience de Rome"], [Color(0.20, 0.65, 0.25), "Hérésie"], [Color(0.45, 0.45, 0.45), "Autre foi"]]
	for entry in entries:
		var chip := HBoxContainer.new()
		chip.add_theme_constant_override("separation", 4)
		var swatch := ColorRect.new()
		swatch.custom_minimum_size = Vector2(16, 16)
		swatch.size_flags_vertical = Control.SIZE_SHRINK_CENTER
		swatch.color = RELATION_COLORS.get(entry[0], Color.GRAY) if entry[0] is String else entry[0]
		chip.add_child(swatch)
		var symbol := Accessibility.relation_symbol(entry[0]) if diplomacy and Accessibility.colorblind() else ""
		chip.add_child(HudStyle.label(("%s %s" % [symbol, entry[1]]).strip_edges(), HudStyle.FONT_BODY, HudStyle.INK))
		row.add_child(chip)
	if row.get_child_count() > 0:
		box.add_child(row)
	box.add_child(HudStyle.label("Les terres voilées sont hors de vue de vos armées, de vos places et de vos agents.", HudStyle.FONT_SMALL, HudStyle.INK_FADED))
	map.ui.add_child(_legend)
	_legend.set_anchors_and_offsets_preset(Control.PRESET_CENTER_BOTTOM, Control.PRESET_MODE_MINSIZE, 18)
	_legend.grow_horizontal = Control.GROW_DIRECTION_BOTH
	_legend.grow_vertical = Control.GROW_DIRECTION_BEGIN


## Tests et captures : légende affichée du mode de carte courant (null sinon).
func legend() -> PanelContainer:
	return _legend if _legend != null and is_instance_valid(_legend) else null
