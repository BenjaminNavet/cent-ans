class_name SettlementController
extends Node

## Lot C5 : interface des colonies sur la carte de campagne. Rendu, UI et entrées seulement ;
## toute règle vient de `CampaignSim` :
## - panneau de colonie (`SettlementPanel`) ouvert par `SettlementLayer.settlement_selected`
##   ou par l'onglet « Colonies » du panneau de province (qui centre aussi la caméra) ;
##   ordres `recruit` / `create_army` / `build` / `cancel_build` / `demolish` (RS-N, bouton
##   « Raser », confirmé par `RazeConfirmationDialog`) adressés à la colonie ;
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
## RS-N : confirmation avant l'ordre `demolish` ; ordre en attente de sa réponse
## ({settlement, building}).
var raze_dialog: RazeConfirmationDialog = null
var _pending_raze: Dictionary = {}


func setup(campaign_map: Node) -> void:
	map = campaign_map
	name = "SettlementController"
	panel = SettlementPanel.new()
	var province_panel: Control = map.ui.province_panel
	province_panel.add_sibling(panel)
	panel.place_like(province_panel)
	map.ui.dock_right_panel(panel)  # C7b : à gauche de la minicarte
	panel.hide()
	panel.recruit_requested.connect(_on_recruit)
	panel.create_army_requested.connect(_on_create_army)
	panel.build_requested.connect(_on_build)
	panel.cancel_build_requested.connect(_on_cancel_build)
	panel.raze_requested.connect(_on_raze_requested)
	panel.province_requested.connect(_on_province_requested)
	raze_dialog = RazeConfirmationDialog.new()
	raze_dialog.confirmed.connect(_on_raze_confirmed)
	raze_dialog.cancelled.connect(func() -> void: _pending_raze = {})
	UiZones.put(UiZones.Zone.MODAL, raze_dialog)
	panel.closed.connect(func() -> void:
		if map.settlement_layer != null:
			map.settlement_layer.select(""))
	province_panel.settlement_rows_provider = rows_for_province
	province_panel.possession_provider = possession_of_province  # RJ-c
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
	# Le panneau de province se ferme (sélection de province remise à zéro).
	map.ui.hide_province()
	map.selected_index = 0
	map.terrain.set_highlight(map.hovered_index, 0)
	_show(detail)
	if focus:
		var world: Vector3 = map.settlement_layer.world_position_of(id)
		var distance := minf(map.camera_rig.distance, FOCUS_DISTANCE)
		map.camera_rig.look_at_point(world + _panel_shift(distance), distance)
	_opening = false


## Lot C7b : décalage du point visé pour que la colonie centrée apparaisse au milieu de la
## partie de l'écran laissée libre à gauche du panneau (même échelle que le glisser de la caméra).
func _panel_shift(distance: float) -> Vector3:
	var view: Vector2 = map.get_viewport().get_visible_rect().size
	var right_edge: float = map.ui.docked_right_x if map.ui.docked_right_x > 0.0 else view.x
	var width := maxf(panel.custom_minimum_size.x, panel.get_combined_minimum_size().x)
	var free_center := maxf(right_edge - width, 0.0) * 0.5
	var shift_px := maxf(view.x * 0.5 - free_center, 0.0)
	var right: Vector3 = map.camera.global_transform.basis.x
	right.y = 0.0
	if right.length_squared() < 0.0001:
		return Vector3.ZERO
	return right.normalized() * shift_px * distance * 1.6 / maxf(view.y, 1.0)


func _show(detail: Dictionary) -> void:
	var id := str(detail.get("id", ""))
	var player: String = map.player_faction
	var player_owner := str(detail.get("owner", "")) == player and str(detail.get("controller", "")) == player
	var recruitable: Array = map.sim.call("get_recruitable", id) if player_owner else []
	var buildable: Array = map.sim.call("settlement_buildable", id) if player_owner and map.sim.has_method("settlement_buildable") else []
	var demolition: Array = map.sim.call("settlement_demolition_preview", id) if player_owner and map.sim.has_method("settlement_demolition_preview") else []
	var shown := detail
	var possession := possession_of_settlement(id)  # RJ-c : statut possédé / occupé
	if not possession.is_empty():
		shown = detail.duplicate()
		shown["possession"] = possession
	panel.show_settlement(shown, recruitable, buildable, player_owner, SimFacade.faction_short_name, demolition)


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
	# RJ-c : statut de chaque place (occupée par…) depuis `province_possession`.
	var statuses: Dictionary = {}
	for place in possession_of_province(province_id).get("settlements", []):
		statuses[str(place.get("id", ""))] = place
	for id in map.sim.call("province_settlements", province_id):
		var detail: Dictionary = map.sim.call("settlement_detail", id)
		if detail.is_empty():
			continue
		var row := detail.duplicate()
		row["garrison_units"] = (detail.get("garrison", []) as Array).size()
		row["possession_status"] = str(statuses.get(str(id), {}).get("status", ""))
		rows.append(row)
	return rows


## RJ-c (ADR 0175) : possession de la province pour le joueur, décrite (`PossessionText.describe`) ;
## {} si la simulation ne l'expose pas.
func possession_of_province(province_id: String) -> Dictionary:
	if map == null or map.sim == null or not map.sim.has_method("province_possession"):
		return {}
	return _described(map.sim.call("province_possession", province_id, ""))


## RJ-c : possession d'une place pour le joueur, décrite ; {} si indisponible.
func possession_of_settlement(settlement_id: String) -> Dictionary:
	if map == null or map.sim == null or not map.sim.has_method("settlement_possession"):
		return {}
	return _described(map.sim.call("settlement_possession", settlement_id, ""))


func _described(possession: Dictionary) -> Dictionary:
	var player: String = map.player_faction
	return PossessionText.describe(possession, player, StanceCues.stances(map.sim, player), SimFacade.faction_short_name)


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


## RS-N : ouvre la confirmation avant l'ordre `demolish`.
func _on_raze_requested(settlement_id: String, building_id: String, preview: Dictionary) -> void:
	if not bool(preview.get("can_demolish", false)):
		return
	_pending_raze = {"settlement": settlement_id, "building": building_id}
	raze_dialog.ask(str(preview.get("name", building_id)), preview)


func _on_raze_confirmed() -> void:
	if _pending_raze.is_empty():
		return
	var order := {"type": "demolish", "settlement": _pending_raze.get("settlement", ""), "building": _pending_raze.get("building", "")}
	_pending_raze = {}
	map._submit(order, "Bâtiment rasé, remboursement versé.")


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


## Lot M4 : le mouvement libre (`ArmyMovementController`) remplace les anneaux et l'aperçu
## sur le graphe des colonies ; le panneau de colonie reste.
func _free_movement() -> bool:
	return map != null and map.get("movement_ctl") != null and map.movement_ctl.available()


## Armée sélectionnée (ou "" / armée étrangère) : anneaux des colonies atteignables.
func on_army_selected(army_id: String, is_player: bool) -> void:
	reachable = {}
	hovered_settlement = ""
	if not available() or army_id == "" or not is_player or _free_movement():
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


## Lot C7b : chaque arête suit son tracé routier (`SettlementData.edge_path`) quand il existe,
## sinon un segment droit entre les deux colonies.
func _draw_path(ids: PackedStringArray) -> void:
	var points := PackedVector2Array()
	var previous := ""
	for id in ids:
		var world: Vector3 = map.settlement_layer.world_position_of(id)
		if world == Vector3.ZERO:
			continue
		var road: PackedVector2Array = map.settlement_data.edge_path(previous, id) if previous != "" else PackedVector2Array()
		if road.size() > 2:
			# Extrémités exclues : les positions des colonies font foi.
			points.append_array(road.slice(1, road.size() - 1))
		points.append(Vector2(world.x, world.z))
		previous = id
	map.path_preview.show_points(points, map.camera_rig.distance, ids)


# --- Captures (`--stage=settlement|settlement_orders`) ---------------------------------


func stage_screenshot(stage: String) -> void:
	if not available():
		return
	if stage == "settlement":
		# Ville du joueur la plus peuplée de bâtiments, onglet Garnison avec le recrutement ouvert.
		var best := ""
		var best_score := -1
		for entry in map.sim.call("settlements"):
			if str(entry["kind"]) != "town" or str(entry["owner"]) != map.player_faction or str(entry["controller"]) != map.player_faction:
				continue
			var detail: Dictionary = map.sim.call("settlement_detail", str(entry["id"]))
			var score := (detail.get("buildings", PackedStringArray()) as PackedStringArray).size() * 10 + (detail.get("garrison", []) as Array).size()
			if score > best_score:
				best_score = score
				best = str(entry["id"])
		if best != "":
			open_settlement(best, true)
			map.camera_rig.snap()
			panel.show_recruit()
		return
	var ids: PackedStringArray = map.player_army_ids()
	if ids.is_empty():
		return
	map.select_army(ids[0])
	# Cible à mi-portée : le chemin reste dans le cadre.
	var costs: Array = reachable.values()
	costs.sort()
	var median: int = int(costs[costs.size() / 2]) if not costs.is_empty() else 0
	var target := ""
	for id in reachable:
		if int(reachable[id]) >= median:
			target = str(id)
			break
	var world: Vector3 = map.armies.world_position_of(ids[0])
	map.camera_rig.look_at_point(world, 260.0)
	map.camera_rig.snap()
	if target != "":
		hovered_settlement = target
		preview_to(target, settlement_name(target))


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
	if not _mouse_dirty or map.selected_army == "" or not available() or _free_movement():
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
