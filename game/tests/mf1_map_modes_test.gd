extends SceneTree

## Test headless du lot MF1 (filtres de la carte de campagne) sur la vraie simulation :
##  1. bouton « Filtres » dans la rangée de la minicarte, ancien bouton « Commerce » masqué ;
##  2. chaque filtre calculé par le cœur (`get_map_lens`) teinte la carte, pose sa légende et
##     donne une valeur au survol de l'Île-de-France ;
##  3. un seul mode à la fois (une seule légende), même touche deux fois = carte politique ;
##  4. touches : R = religion sans ouvrir les routes commerciales (ancien conflit), V = routes ;
##  5. menu : F l'ouvre, un clic sur une entrée change de mode et le referme ;
##  6. mécontentement : teinte proportionnelle (l'ancien mode saturait au rouge dès 1 %).
## Usage : godot --headless --path game --script res://tests/mf1_map_modes_test.gd

const MAP_PATHS := preload("res://scripts/map/map_paths.gd")
const PARIS := "prov_ile_de_france"

var _failures := 0


func _init() -> void:
	await process_frame
	await _run()
	print("mf1_map_modes_test: %s" % ("OK" if _failures == 0 else "%d failure(s)" % _failures))
	quit(1 if _failures > 0 else 0)


func _check(condition: bool, message: String) -> bool:
	if not condition:
		_failures += 1
		push_error("mf1_map_modes_test: " + message)
	return condition


func _key(map: Node, keycode: Key) -> void:
	var event := InputEventKey.new()
	event.physical_keycode = keycode
	event.keycode = keycode  # `map_toggle_unrest` est lié par keycode, les autres par touche physique
	event.pressed = true
	map._unhandled_input(event)


func _legends(map: Node) -> int:
	var count := 0
	for child in map.ui.get_children():
		if child.name.begins_with("MapModeLegend") and not child.is_queued_for_deletion():
			count += 1
	return count


func _run() -> void:
	if not _check(ClassDB.class_exists("CampaignSim") and ClassDB.instantiate("CampaignSim").has_method("get_map_lens"),
			"CampaignSim.get_map_lens missing (run core/build.sh)"):
		return
	var settings: Node = root.get_node_or_null("/root/Settings")
	if settings != null:
		settings.call("use_test_file")
		settings.call("set_value", "game/autosave_interval", 0, false)
		settings.call("set_value", "tutorial/enabled", false, false)
	var facade: Node = root.get_node("/root/SimFacade")
	facade.set_data_dir(MAP_PATHS.default_data_dir())
	facade.pending_faction = "fac_france"
	facade.pending_seed = 1337
	facade.pending_load_path = ""
	var map: Node3D = (load("res://scenes/campaign_map.tscn") as PackedScene).instantiate()
	root.add_child(map)
	await process_frame
	await process_frame
	if not _check(map.load_ok and map.sim != null and facade.is_real, "campaign map with the real simulation failed to start"):
		map.queue_free()
		return
	var modes: Node = map.map_modes  # non typé : SimFacade (autoload) hors de portée à la compilation du test

	# 1. Bouton et rangée de la minicarte.
	_check(modes != null and modes.button != null, "MapModeController or its button missing")
	_check(modes.button.get_parent() != null and modes.button.get_parent().get_parent() != null
			and modes.button.get_parent().get_parent().get_parent() == map.ui.minimap,
			"filters button should sit in the minimap row")
	_check(not map.ui.trade_button.visible, "the old trade button should be hidden")

	# 2. Filtres du cœur : légende et valeur au survol.
	for mode in ["unrest", "wealth", "population", "loyalty", "supply", "claims", "diplomacy", "religion"]:
		modes.set_mode(mode)
		await process_frame
		_check(modes.mode == mode, "mode %s not set" % mode)
		_check(modes.legend() != null, "%s: no legend" % mode)
		var hover: String = modes.hover_text(PARIS)
		_check(hover != "", "%s: empty hover text for Paris" % mode)
		print("mf1: %s → %s" % [mode, hover])

	# 3. Exclusivité.
	modes.set_mode("diplomacy")
	modes.set_mode("wealth")
	await process_frame
	_check(_legends(map) == 1, "exactly one legend expected, got %d" % _legends(map))
	modes.toggle_mode("wealth")
	await process_frame
	_check(modes.mode == "political" and modes.legend() == null, "same mode twice should return to political")

	# 4. Touches.
	_key(map, KEY_R)
	_check(modes.mode == "religion", "R should open the religion map")
	_check(not map.trade_mode, "R must not toggle trade routes any more")
	_key(map, KEY_R)
	_check(modes.mode == "political", "R twice should return to political")
	_key(map, KEY_V)
	_check(map.trade_mode, "V should show trade routes")
	_key(map, KEY_M)
	_check(modes.mode == "unrest" and map.trade_mode, "unrest mode combines with the trade layer")
	_key(map, KEY_V)
	_key(map, KEY_M)

	# 5. Menu.
	_key(map, KEY_F)
	_check(modes.menu.visible, "F should open the filters menu")
	(modes._menu_buttons["population"] as Button).emit_signal("pressed")
	_check(modes.mode == "population" and not modes.menu.visible, "menu entry should set the mode and close")
	modes.set_mode("political")

	# 6. Mécontentement proportionnel.
	modes.set_mode("unrest")
	var row: Dictionary = modes._lens.get(PARIS, {})
	var unrest := float(row.get("unrest", 0.0))
	var color: Color = modes._lens_color(row, 0.0)
	_check(unrest >= 0.0 and unrest <= 100.0, "unrest out of range: %s" % unrest)
	if unrest < 90.0:
		_check(not color.is_equal_approx((modes.get_script().BAD as Color)), "Paris unrest %.1f should not be full red" % unrest)
	modes.set_mode("political")
	map.queue_free()
