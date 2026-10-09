class_name SettlementController
extends Node

## Interface des colonies sur la carte de campagne. Rendu, UI et entrées seulement ;
## toute règle vient de `CampaignSim` :
## - panneau de colonie (`SettlementPanel`) ouvert par `SettlementLayer.settlement_selected`
##   ou par l'onglet « Colonies » du panneau de province (qui centre aussi la caméra) ;
##   ordres `recruit` / `create_army` / `build` / `cancel_build` / `demolish` (RS-N, bouton
##   « Raser », confirmé par `ConfirmPanel`) adressés à la colonie ;
## (Les ordres de mouvement et l'aperçu de chemin vivent dans `ArmyMovementController`.)
## Inactif (repli v1 de `campaign_map.gd`) si la simulation n'expose pas ces getters (mock).

## Distance caméra quand on centre une colonie (palier « près », maquettes visibles).
const FOCUS_DISTANCE := 110.0

var map: Node = null  # CampaignMap
var panel: SettlementPanel = null
var _opening := false
## RS-N : confirmation avant l'ordre `demolish` ; ordre en attente de sa réponse
## ({settlement, building}).
var slot_bar: SettlementSlotBar = null
var raze_dialog: ConfirmPanel = null
var _pending_raze: Dictionary = {}


func setup(campaign_map: Node) -> void:
	map = campaign_map
	name = "SettlementController"
	panel = SettlementPanel.new()
	var province_panel: Control = map.ui.province_panel
	province_panel.add_sibling(panel)
	panel.place_like(province_panel)
	map.ui.dock_right_panel(panel)  # À gauche de la minicarte
	panel.hide()
	# A6-L15 (ADR 0185) : barre des emplacements en bas de l'écran.
	slot_bar = SettlementSlotBar.new()
	map.ui.attach_slot_bar(slot_bar, panel)
	slot_bar.slot_activated.connect(_on_slot_activated)
	panel.visibility_changed.connect(func() -> void: slot_bar.visible = panel.visible and slot_bar.slots.size() > 0)
	panel.recruit_requested.connect(_on_recruit)
	panel.create_army_requested.connect(_on_create_army)
	panel.build_requested.connect(_on_build)
	panel.cancel_build_requested.connect(_on_cancel_build)
	panel.cancel_queued_build_requested.connect(_on_cancel_queued_build)
	panel.raze_requested.connect(_on_raze_requested)
	panel.province_requested.connect(_on_province_requested)
	raze_dialog = ConfirmPanel.new("Raser")
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
	if map.settlement_layer != null:
		map.settlement_layer.settlement_selected.connect(func(id: String) -> void: open_settlement(id, false))


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


## Décalage du point visé pour que la colonie centrée apparaisse au milieu de la
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
	_fill_slot_bar(id, str(detail.get("name", id)), player_owner)


## A6-L15 : la bande des emplacements suit la colonie affichée (données du cœur, lecture seule).
func _fill_slot_bar(id: String, place_name: String, player_owner: bool) -> void:
	if slot_bar == null:
		return
	var rows: Array = map.sim.call("settlement_slots", id) if map.sim.has_method("settlement_slots") else []
	var usage: Dictionary = map.sim.call("settlement_slot_usage", id) if map.sim.has_method("settlement_slot_usage") else {}
	slot_bar.set_slots(id, rows, player_owner, place_name, usage)
	slot_bar.visible = panel.visible and not rows.is_empty()


## Clic sur une case : l'onglet des bâtiments du panneau s'ouvre, la ligne de construction de la
## case (premier palier proposé) prend le focus. L'ordre se donne dans le panneau, pas ici.
func _on_slot_activated(settlement_id: String, slot: Dictionary) -> void:
	if panel == null or not panel.visible or panel.settlement_id != settlement_id:
		return
	panel.show_buildings_tab()
	var next: Array = slot.get("next", [])
	if next.is_empty() or panel.buildable_list == null:
		return
	var wanted := str((next[0] as Dictionary).get("name", ""))
	for line in panel.buildable_list.get_children():
		for child in line.get_children():
			if child is Button and not (child as Button).disabled and (child as Button).text.begins_with(wanted):
				(child as Button).grab_focus()
				return


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
	map._submit({"type": "recruit", "settlement": settlement_id, "unit_type": unit_type}, "Recrutement lancé : l’unité rejoindra la garnison au prochain tour.")


func _on_create_army(settlement_id: String, unit_indices: Array) -> void:
	var result: Dictionary = map._submit({"type": "create_army", "settlement": settlement_id, "units_from_garrison": unit_indices}, "Armée formée.")
	if result.get("ok", false) and result.has("army"):
		map.select_army(str(result["army"]))


func _on_build(settlement_id: String, building_id: String) -> void:
	map._submit({"type": "build", "settlement": settlement_id, "building": building_id}, "Construction lancée.")


func _on_cancel_queued_build(settlement_id: String, index: int) -> void:
	map._submit({"type": "cancel_queued_build", "settlement": settlement_id, "index": index}, "Chantier en file annulé (moitié du coût remboursée).")


func _on_cancel_build(settlement_id: String) -> void:
	map._submit({"type": "cancel_build", "settlement": settlement_id}, "Construction annulée (moitié du coût remboursée).")


## RS-N : ouvre la confirmation avant l'ordre `demolish`.
func _on_raze_requested(settlement_id: String, building_id: String, preview: Dictionary) -> void:
	if not bool(preview.get("can_demolish", false)):
		return
	_pending_raze = {"settlement": settlement_id, "building": building_id}
	var building_name := str(preview.get("name", building_id))
	var refund := Money.amount(int(preview.get("refund", 0)))
	var upkeep_saved := Money.amount(int(preview.get("upkeep_saved", 0)))
	raze_dialog.open("Raser %s ?" % building_name, "Le bâtiment est détruit sans retour possible. Rembourse %s ; économise %s d’entretien par saison." % [refund, upkeep_saved])


func _on_raze_confirmed() -> void:
	if _pending_raze.is_empty():
		return
	var order := {"type": "demolish", "settlement": _pending_raze.get("settlement", ""), "building": _pending_raze.get("building", "")}
	_pending_raze = {}
	map._submit(order, "Bâtiment rasé, remboursement versé.")


# --- Ordres d'armée ---------------------------------------------------------------------


func settlement_name(id: String) -> String:
	var entry: Dictionary = map.settlement_data.get_settlement(id) if map.settlement_data != null else {}
	return str(entry.get("name", id))
