class_name SettlementController
extends Node

## Lot C5 : interface des colonies sur la carte de campagne. Rendu, UI et entrées seulement ;
## toute règle vient de `CampaignSim` :
## - panneau de colonie (`SettlementPanel`) ouvert par `SettlementLayer.settlement_selected`
##   ou par l'onglet « Colonies » du panneau de province (qui centre aussi la caméra) ;
##   ordres `recruit` / `create_army` / `build` / `cancel_build` adressés à la colonie ;
## - clic droit sur une colonie avec une armée sélectionnée : ordre `move_army` le long de
##   `find_path(armée, colonie)` ;
## - colonies atteignables ce tour (`get_reachable_settlements`) : anneaux au sol ;
## - aperçu de chemin sur le graphe des colonies (positions `SettlementLayer.world_position_of`).
## Inactif (repli v1 de `campaign_map.gd`) si la simulation n'expose pas ces getters (mock).

## Distance caméra quand on centre une colonie (palier « près », maquettes visibles).
const FOCUS_DISTANCE := 110.0

var map: Node = null  # CampaignMap
var panel: SettlementPanel = null
var markers: ReachableMarkers = null
## Colonies atteignables par l'armée sélectionnée : id → coût.
var reachable: Dictionary = {}
var hovered_settlement: String = ""
var _opening := false
var _mouse_dirty := false
var _last_distance := -1.0


func setup(campaign_map: Node) -> void:
	map = campaign_map
	name = "SettlementController"
	panel = SettlementPanel.new()
	var province_panel: Control = map.ui.province_panel
	province_panel.add_sibling(panel)
	panel.place_like(province_panel)
	panel.hide()
	panel.recruit_requested.connect(_on_recruit)
	panel.create_army_requested.connect(_on_create_army)
	panel.build_requested.connect(_on_build)
	panel.cancel_build_requested.connect(_on_cancel_build)
	panel.province_requested.connect(_on_province_requested)
	panel.closed.connect(func() -> void:
		if map.settlement_layer != null:
			map.settlement_layer.select(""))
	province_panel.settlement_rows_provider = rows_for_province
	province_panel.settlement_requested.connect(func(id: String) -> void: open_settlement(id, true))
	# Un seul des deux panneaux à la fois.
	province_panel.visibility_changed.connect(func() -> void:
		if province_panel.visible and panel.visible:
			panel.hide())
	markers = ReachableMarkers.new()
	map.add_child(markers)
	if map.settlement_layer != null:
		map.settlement_layer.settlement_selected.connect(func(id: String) -> void: open_settlement(id, false))
	if map.picker != null:
		map.picker.right_click_interceptor = _try_right_click


## Vrai si la simulation expose l'API des colonies (vraie `CampaignSim`).
func available() -> bool:
	var sim: Object = map.sim if map != null else null
	return sim != null and sim.has_method("settlement_detail") and sim.has_method("get_reachable_settlements") \
		and map.settlement_layer != null


# --- Panneau de colonie ----------------------------------------------------------------


## Ouvre le panneau de la colonie `id` ; `focus` centre la caméra et la sélectionne sur la carte.
func open_settlement(id: String, focus: bool = false) -> void:
	if not available() or _opening:
		return
	var detail: Dictionary = map.sim.call("settlement_detail", id)
	if detail.is_empty():
		return
	_opening = true
	if focus:
		map.settlement_layer.select(id)
		var world: Vector3 = map.settlement_layer.world_position_of(id)
		map.camera_rig.look_at_point(world, minf(map.camera_rig.distance, FOCUS_DISTANCE))
	# Le panneau de province se ferme (sélection de province remise à zéro).
	map.ui.hide_province()
	map.selected_index = 0
	map.terrain.set_highlight(map.hovered_index, 0)
	_show(detail)
	_opening = false


func _show(detail: Dictionary) -> void:
	var id := str(detail.get("id", ""))
	var player: String = map.player_faction
	var player_owner := str(detail.get("owner", "")) == player and str(detail.get("controller", "")) == player
	var recruitable: Array = map.sim.call("get_recruitable", id) if player_owner else []
	var buildable: Array = map.sim.call("settlement_buildable", id) if player_owner and map.sim.has_method("settlement_buildable") else []
	panel.show_settlement(detail, recruitable, buildable, player_owner, SimFacade.faction_short_name)


## Rafraîchit le panneau ouvert après un changement d'état (appelé par `refresh_all`).
func refresh() -> void:
	if panel == null or not panel.visible or not available():
		return
	var detail: Dictionary = map.sim.call("settlement_detail", panel.settlement_id)
	if detail.is_empty():
		panel.hide()
		return
	_show(detail)


## Lignes de l'onglet « Colonies » d'une province (cité d'abord).
func rows_for_province(province_id: String) -> Array:
	var rows: Array = []
	if not available() or not map.sim.has_method("province_settlements"):
		return rows
	for id in map.sim.call("province_settlements", province_id):
		var detail: Dictionary = map.sim.call("settlement_detail", id)
		if detail.is_empty():
			continue
		var row := detail.duplicate()
		row["garrison_units"] = (detail.get("garrison", []) as Array).size()
		rows.append(row)
	return rows


func _on_province_requested(province_id: String) -> void:
	var index: int = map.map_data.index_of_id(province_id)
	if index > 0:
		panel.hide()
		map.picker.select_index(index)


func _on_recruit(settlement_id: String, unit_type: String) -> void:
	map._submit({"type": "recruit", "settlement": settlement_id, "unit_type": unit_type}, "Recrutement lancé : l'unité rejoindra la garnison au prochain tour.")


func _on_create_army(settlement_id: String, unit_indices: Array) -> void:
	var result: Dictionary = map._submit({"type": "create_army", "settlement": settlement_id, "units_from_garrison": unit_indices}, "Armée formée.")
	if result.get("ok", false) and result.has("army"):
		map.select_army(str(result["army"]))


func _on_build(settlement_id: String, building_id: String) -> void:
	map._submit({"type": "build", "settlement": settlement_id, "building": building_id}, "Construction lancée.")


func _on_cancel_build(settlement_id: String) -> void:
	map._submit({"type": "cancel_build", "settlement": settlement_id}, "Construction annulée (moitié du coût remboursée).")


# --- Ordres d'armée ---------------------------------------------------------------------


## Intercepteur du clic droit : vrai si une colonie a été prise pour cible.
func _try_right_click(screen_position: Vector2) -> bool:
	if not available() or map.selected_army == "":
		return false
	var id: String = map.settlement_layer.pick_screen(screen_position)
	if id == "":
		return false
	var result := order_move_to_settlement(map.selected_army, id)
	if not result.get("ok", false):
		map.ui.show_toast(str(result.get("error", "Ordre refusé")), true)
	return true


## Ordre `move_army` vers une colonie, par `find_path` (colonies successives).
func order_move_to_settlement(army_id: String, settlement_id: String) -> Dictionary:
	var path: PackedStringArray = map.sim.call("find_path", army_id, settlement_id)
	if path.is_empty():
		return {"ok": false, "error": "Aucun chemin vers %s." % settlement_name(settlement_id)}
	var result: Dictionary = map.sim.call("submit_order", {"type": "move_army", "army": army_id, "path": Array(path)})
	if result.get("ok", false):
		map.refresh_all()
	return result


func settlement_name(id: String) -> String:
	var entry: Dictionary = map.settlement_data.get_settlement(id) if map.settlement_data != null else {}
	return str(entry.get("name", id))


## Armée sélectionnée (ou "" / armée étrangère) : anneaux des colonies atteignables.
func on_army_selected(army_id: String, is_player: bool) -> void:
	reachable = {}
	hovered_settlement = ""
	if not available() or army_id == "" or not is_player:
		markers.clear()
		return
	reachable = map.sim.call("get_reachable_settlements", army_id)
	var army: Dictionary = map.sim.call("get_army", army_id)
	var here := str(army.get("location", ""))
	var ids := PackedStringArray()
	var positions := PackedVector3Array()
	for id in reachable:
		if str(id) == here:
			continue
		var world: Vector3 = map.settlement_layer.world_position_of(str(id))
		if world == Vector3.ZERO:
			continue
		ids.append(str(id))
		positions.append(world)
	markers.show_markers(ids, positions, map.camera_rig.distance)


func on_army_deselected() -> void:
	reachable = {}
	hovered_settlement = ""
	markers.clear()


## Chemin déjà ordonné de l'armée (`army.path`, colonies) ; faux si indisponible.
func show_current_path(army: Dictionary) -> bool:
	if not available():
		return false
	var path: Array = Array(army.get("path", PackedStringArray()))
	if path.is_empty():
		map.path_preview.hide_path()
		return true
	var ids := PackedStringArray([str(army.get("location", ""))])
	for step in path:
		ids.append(str(step))
	_draw_path(ids)
	return true


## Aperçu du chemin de l'armée sélectionnée vers `target_id` (colonie, ou province = sa
## cité) : ruban le long des arêtes du graphe, masque des provinces, ligne de survol.
## Faux si indisponible (la carte garde alors l'aperçu v1 par province).
func preview_to(target_id: String, target_name: String) -> bool:
	if not available() or map.selected_army == "":
		return false
	var army: Dictionary = map.sim.call("get_army", map.selected_army)
	var path: PackedStringArray = map.sim.call("find_path", map.selected_army, target_id)
	var path_indices := PackedInt32Array()
	for step in path:
		var entry: Dictionary = map.settlement_data.get_settlement(step)
		var index: int = map.map_data.index_of_id(str(entry.get("province", "")))
		if index > 0 and not path_indices.has(index):
			path_indices.append(index)
	map._apply_reachable_mask(path_indices)
	if path.is_empty():
		map.path_preview.hide_path()
		markers.set_target("")
		map.ui.set_hover_path(target_name, 0, 0, false)
		return true
	var ids := PackedStringArray([str(army.get("location", ""))])
	ids.append_array(path)
	_draw_path(ids)
	var last := path[path.size() - 1]
	markers.set_target(last)
	map.ui.set_hover_path(target_name, path.size(), int(reachable.get(last, 0)), reachable.has(last))
	return true


## Survol d'une province : la colonie survolée (s'il y en a une) reste prioritaire.
func preview_hover(target_id: String, target_name: String) -> bool:
	if hovered_settlement != "":
		return preview_to(hovered_settlement, settlement_name(hovered_settlement))
	return preview_to(target_id, target_name)


func _draw_path(ids: PackedStringArray) -> void:
	var points := PackedVector2Array()
	for id in ids:
		var world: Vector3 = map.settlement_layer.world_position_of(id)
		if world == Vector3.ZERO:
			continue
		points.append(Vector2(world.x, world.z))
	map.path_preview.show_points(points, map.camera_rig.distance, ids)


# --- Survol des colonies ----------------------------------------------------------------


func _unhandled_input(event: InputEvent) -> void:
	if event is InputEventMouseMotion:
		_mouse_dirty = true


func _process(_delta: float) -> void:
	if map == null or markers == null:
		return
	var distance: float = map.camera_rig.distance
	if markers.visible and not is_equal_approx(distance, _last_distance):
		_last_distance = distance
		markers.update_scale(distance)
	if not _mouse_dirty or map.selected_army == "" or not available():
		return
	_mouse_dirty = false
	var id: String = map.settlement_layer.pick_screen(get_viewport().get_mouse_position())
	if id == hovered_settlement:
		return
	hovered_settlement = id
	if id != "":
		preview_to(id, settlement_name(id))
	else:
		markers.set_target("")
		map._on_province_hovered(map.hovered_index)
