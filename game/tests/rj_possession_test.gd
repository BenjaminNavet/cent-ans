extends SceneTree

## Test headless du lot RJ-c (ADR 0175, possession et occupation expliquées, bannières) sur la
## vraie simulation, France jouée :
##  1. textes des statuts (`PossessionText`) pour les cinq cas ;
##  2. `province_possession` / `settlement_possession` du cœur : étrangère, puis prise de la cité
##     (occupée par nous ; vue du propriétaire : à lui, occupée ; vue d'un tiers : occupée) ;
##  3. panneau de province (statut, places tenues), liste des colonies, panneau de colonie,
##     survol de la carte, rappel « occuper n'est pas posséder » de la fenêtre de capture ;
##  4. bannières : écu du propriétaire + écu de l'occupant après la prise, liserés de position ;
##     temps d'une réécriture complète des ~2 000 bannières.
## Usage : godot --headless --path game --script res://tests/rj_possession_test.gd

const MAP_PATHS := preload("res://scripts/map/map_paths.gd")
const PLAYER := "fac_france"
const OWNER := "fac_england"

var _failures := 0


func _init() -> void:
	await process_frame
	_check_texts()
	await _run()
	print("rj_possession_test: %s" % ("OK" if _failures == 0 else "%d failure(s)" % _failures))
	quit(1 if _failures > 0 else 0)


func _check(condition: bool, message: String) -> bool:
	if not condition:
		_failures += 1
		push_error("rj_possession_test: " + message)
	return condition


func _check_texts() -> void:
	_check(PossessionText.status_line("own", "France", "France") == "À vous", "own")
	_check(PossessionText.status_line("own_occupied", "France", "Angleterre") == "À vous — occupée par Angleterre", "own_occupied")
	_check(PossessionText.status_line("occupied_by_viewer", "Angleterre", "France") == "Occupée par vous (propriétaire de droit : Angleterre) — rendue à la paix si non cédée", "occupied_by_viewer")
	_check(PossessionText.status_line("foreign", "Angleterre", "Angleterre") == "À Angleterre", "foreign")
	_check(PossessionText.status_line("foreign_occupied", "Angleterre", "Bourgogne") == "À Angleterre — occupée par Bourgogne", "foreign_occupied")
	_check(PossessionText.held_line(3, 3, PLAYER, PLAYER).contains("bonus acquis"), "whole province bonus")
	_check(PossessionText.held_line(2, 5, "", PLAYER).contains("encore 3 places"), "places missing for the bonus")
	_check(PossessionText.occupied_mention("foreign", "A", "A") == "", "no mention when not occupied")
	_check(PossessionText.occupied_mention("own_occupied", "France", "Angleterre") == "occupée par Angleterre", "occupied mention")


## Province dont la cité est à `faction` et tenue par elle (premier id dans l'ordre).
func _province_of(sim: Object, map: Node, faction: String) -> String:
	var ids := ProvinceSnapshot.province_ids(map.map_data)
	var snapshot := ProvinceSnapshot.read(sim, ids)
	for i in ids.size():
		if snapshot.owner[i] == faction and snapshot.controller[i] == faction:
			return ids[i]
	return ""


func _run() -> void:
	var settings: Node = root.get_node_or_null("/root/Settings")
	if settings != null:
		settings.call("use_test_file")
		settings.call("set_value", "game/autosave_interval", 0, false)
		settings.call("set_value", "tutorial/enabled", false, false)
	var facade: Node = root.get_node("/root/SimFacade")
	facade.set_data_dir(MAP_PATHS.default_data_dir())
	facade.pending_faction = PLAYER
	facade.pending_seed = 1337
	facade.pending_load_path = ""
	var map: Node3D = (load("res://scenes/campaign_map.tscn") as PackedScene).instantiate()
	root.add_child(map)
	await process_frame
	await process_frame
	if not _check(map.load_ok and map.sim != null, "campaign map failed to start"):
		map.queue_free()
		return
	var sim: Object = map.sim
	if not _check(sim.has_method("province_possession"), "province_possession missing (rebuild core)"):
		map.queue_free()
		return
	var province := _province_of(sim, map, OWNER)
	if not _check(province != "", "an English province should exist in 1337"):
		map.queue_free()
		return

	# 2. Cœur : étrangère, puis cité prise.
	var before: Dictionary = sim.call("province_possession", province, "")
	_check(str(before.get("status", "")) == "foreign", "%s should be foreign before the capture" % province)
	_check(int(before.get("total", 0)) >= 1 and int(before.get("held", -1)) == 0, "no place of %s is held by the player" % province)
	var city := str(before.get("city", ""))
	_check(bool(sim.call("debug_capture_place", city)), "debug capture of %s" % city)
	var after: Dictionary = sim.call("province_possession", province, "")
	_check(str(after.get("status", "")) == "occupied_by_viewer", "taking the city occupies the province (got %s)" % after.get("status"))
	_check(str(after.get("owner", "")) == OWNER, "possession stays English")
	_check(int(after.get("held", 0)) >= 1, "the city is now held")
	_check(str((sim.call("province_possession", province, OWNER) as Dictionary).get("status", "")) == "own_occupied", "seen by the owner: own, occupied")
	_check(str((sim.call("province_possession", province, "fac_burgundy") as Dictionary).get("status", "")) == "foreign_occupied", "seen by a third party: foreign, occupied")
	var place: Dictionary = sim.call("settlement_possession", city, "")
	_check(bool(place.get("is_city", false)) and bool(place.get("occupied", false)), "settlement_possession of the city")

	# Fenêtre de capture : rappel « occuper n'est pas posséder ».
	var captures: Array = sim.call("get_pending_captures")
	_check(not captures.is_empty() and str(captures[0].get("text", "")).contains("Occuper n'est pas posséder"), "capture decision should remind that occupying is not possessing")
	_check(not captures.is_empty() and str(captures[0].get("text", "")).contains("C'est la cité de"), "capture of a city should say it gives the province's control")

	# 3. UI.
	map.refresh_all()
	await process_frame
	var described: Dictionary = map.settlements_ctl.possession_of_province(province)
	var line := str(described.get("line", ""))
	_check(line.begins_with("Occupée par vous (propriétaire de droit : "), "described line: %s" % line)
	_check(str(described.get("cue", "")) == StanceCues.SELF, "a place occupied by the player carries the self cue")
	var hover: Dictionary = map._with_mode_value(map.province_info(map.map_data.index_of_id(province)))
	_check(str(hover.get("possession_line", "")) == line, "map hover carries the status")
	map.ui.set_hovered(hover)
	_check(str(map.ui.hover_label.text).contains(line), "hover label shows the status")
	map.picker.select_index(map.map_data.index_of_id(province))
	await process_frame
	var panel: Control = map.ui.province_panel
	_check(panel.visible and panel.owner_value.text == line, "province panel status (got '%s')" % panel.owner_value.text)
	_check(panel.possession_held != null and panel.possession_held.visible and panel.possession_held.text.begins_with("Places tenues : "), "province panel held line")
	_check(panel.possession_help().contains("donne le contrôle de la province"), "province panel help sentence")
	var rows: Array = map.settlements_ctl.rows_for_province(province)
	var city_row := {}
	for row in rows:
		if str(row.get("id", "")) == city:
			city_row = row
	_check(str(city_row.get("possession_status", "")) == "occupied_by_viewer", "colony list row carries the status")
	map.settlements_ctl.open_settlement(city)
	await process_frame
	var settlement_panel: Control = map.settlements_ctl.panel
	_check(settlement_panel.owner_value.text == line, "settlement panel status (got '%s')" % settlement_panel.owner_value.text)
	_check(settlement_panel.possession_help.visible and settlement_panel.possession_help.text.contains("qui la tient contrôle la province"), "settlement panel help")

	# 4. Bannières.
	var layer: Node3D = map.settlement_layer
	var index: int = layer.data.index_by_id.get(city, -1)
	if _check(index >= 0, "city %s on the settlement layer" % city):
		# Le serveur de rendu factice (headless) ne relit pas les couleurs du MultiMesh : on lit
		# la clé « case|canal g » écrite avec elles.
		var holder := str(layer._marker_holder[index]).split("|")
		var color := Color(float(holder[0]), float(holder[1]), 0.0)
		var packed := int(round(color.g))
		_check(int(round(color.r)) == layer.heraldry.shield_of(OWNER), "main shield = the owner's arms")
		_check(packed / 16 - 1 == layer.heraldry.shield_of(PLAYER), "second shield = the occupant's arms")
		_check(packed % 4 == layer.BANNER_CUES["self"], "occupant edge = self")
		var owner_cue := StanceCues.category_of(OWNER, PLAYER, StanceCues.stances(sim, PLAYER))
		_check((packed / 4) % 4 == int(layer.BANNER_CUES.get(owner_cue, 0)), "owner edge follows the stance")
		_check(layer._occupied[index] == 1 and layer.banner_width(index) > 1.0, "occupied banner is wider")
	var own_city := str((sim.call("province_possession", _province_of(sim, map, PLAYER), "") as Dictionary).get("city", ""))
	var own_index: int = layer.data.index_by_id.get(own_city, -1)
	if own_index >= 0:
		var own_packed := int(str(layer._marker_holder[own_index]).split("|")[1])
		_check(own_packed == layer.BANNER_CUES["self"] * 4, "own place: no occupant, self edge")
	# Perf : réécriture complète de toutes les bannières.
	layer._marker_holder.fill("?")
	var started := Time.get_ticks_usec()
	layer._refresh_shields(sim)
	var elapsed_ms := (Time.get_ticks_usec() - started) / 1000.0
	print("rj_possession_test: full banner rewrite of %d places in %.1f ms" % [layer.data.settlements.size(), elapsed_ms])
	# `refresh` lit les positions une fois pour les bannières et l'encre des noms.
	var stances := StanceCues.stances(sim, PLAYER)
	started = Time.get_ticks_usec()
	layer._refresh_shields(sim, false, PLAYER, stances)
	print("rj_possession_test: unchanged banner refresh in %.2f ms" % ((Time.get_ticks_usec() - started) / 1000.0))
	_check(layer.data.settlements.size() >= 1000, "at least 1000 places carry a banner")
	map.queue_free()
	await process_frame
