extends TestCase

## Test headless du lot DZ (frontières diplomatiques) sur la vraie simulation :
##  1. mode Diplomatie : les frontières prennent les couleurs de position (palette imposée), la
##     France (joueur) en « nous », un ennemi en rouge guerre ;
##  2. clic sur une province anglaise : l'Angleterre devient la faction observée (teinte, halo,
##     légende) ; ses propres terres en « nous » ;
##  3. clic sur nos terres : retour au joueur ; sortie du mode : couleurs héraldiques rendues ;
##  4. panneau de diplomatie : la carte suit la faction choisie, bascule « vos relations ».
## Usage : godot --headless --path game --script res://tests/dz_diplo_borders_test.gd

const PARIS := "prov_ile_de_france"
const GUYENNE := "prov_guyenne"


func _init() -> void:
	await process_frame
	await _run()
	finish()


func _palette_color(borders: FactionBorders, faction: String) -> Color:
	var texture: Texture2D = borders.get_param("fr1_palette")
	if texture == null:
		return Color(0, 0, 0, 0)
	return texture.get_image().get_pixel(borders.faction_index(faction), 0)


func _same(a: Color, b: Color) -> bool:
	return absf(a.r - b.r) < 0.02 and absf(a.g - b.g) < 0.02 and absf(a.b - b.b) < 0.02


func _run() -> void:
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
	if not check(map.load_ok and map.sim != null, "campaign map failed to start"):
		map.queue_free()
		return
	var sim: Object = map.sim
	if not check(sim.has_method("get_faction_stances_for"), "get_faction_stances_for missing (rebuild core)"):
		map.queue_free()
		return
	var borders: FactionBorders = map.faction_borders
	var modes: Object = map.map_modes
	var heraldic_france := _palette_color(borders, "fac_france")

	# 1. Mode Diplomatie : palette de positions vue de la France.
	modes.set_mode("diplomacy")
	check(borders.color_override_active(), "diplomacy mode should override border colours")
	check(_same(_palette_color(borders, "fac_france"), DiplomaticStances.COLORS["self"]), "player borders should use the 'self' colour")
	var of_france: Dictionary = sim.call("get_faction_stances_for", "fac_france")
	var enemy := ""
	for faction in of_france:
		if str(of_france[faction]) == "war" and borders.faction_index(str(faction)) > 0:
			enemy = str(faction)
			break
	if enemy != "":
		check(_same(_palette_color(borders, enemy), DiplomaticStances.COLORS["war"]), "enemy %s borders should be red" % enemy)
	else:
		print("dz_diplo_borders_test: no enemy of France holding land at the start (red check skipped)")

	# 2. Clic sur la Guyenne : vue de l'Angleterre.
	var guyenne_index: int = map.map_data.index_of_id(GUYENNE)
	map._on_province_selected(guyenne_index)
	check(modes.focus_faction == "fac_england", "clicking Guyenne should focus England, got '%s'" % modes.focus_faction)
	check(_same(_palette_color(borders, "fac_england"), DiplomaticStances.COLORS["self"]), "England borders should be 'self' in its view")
	var of_england: Dictionary = sim.call("get_faction_stances_for", "fac_england")
	var france_seen := str(of_england.get("fac_france", ""))
	check(_same(_palette_color(borders, "fac_france"), DiplomaticStances.COLORS[france_seen]), "France borders should show England's stance '%s'" % france_seen)
	check(modes.hover_text(GUYENNE) == "terres de %s" % facade.call("faction_short_name", "fac_england"), "hover: %s" % modes.hover_text(GUYENNE))
	var legend: Node = modes.legend()
	check(legend != null and _legend_text(legend).contains("relations de"), "legend title should name the observed faction")

	# 3. Clic sur Paris : retour au joueur ; sortie du mode : héraldique.
	map._on_province_selected(map.map_data.index_of_id(PARIS))
	check(modes.focus_faction == "", "clicking our lands should return to the player's view")
	modes.set_mode("political")
	check(not borders.color_override_active(), "leaving diplomacy should restore heraldic borders")
	check(_same(_palette_color(borders, "fac_france"), heraldic_france), "France heraldic colour should be back")

	# 4. Panneau de diplomatie : la carte suit la faction choisie.
	var panel_script: Script = load("res://scripts/ui/diplomacy/diplomacy_map_view.gd")
	var panel: Node = panel_script.new()
	panel.set("sim", sim)
	panel.set("player_faction", "fac_france")
	panel.set("faction_id", "fac_england")
	check(str(panel.call("map_viewer")) == "fac_england", "panel map should show the selected faction's view")
	panel.set("their_view", false)
	check(str(panel.call("map_viewer")) == "", "toggle off: panel map shows our relations")
	panel.free()
	map.queue_free()
	await process_frame


func _legend_text(node: Node) -> String:
	var text := ""
	if node is Label:
		text += (node as Label).text + "\n"
	for child in node.get_children():
		text += _legend_text(child)
	return text
