extends SceneTree

## Chantier IB (ADR 0109) : planche des infobulles, une image JPEG par infobulle, recadrée sur
## le panneau, dans `<out>/NN-<vue>.jpg`. Le processus parent relance Godot fenêtré (le rendu
## headless ne produit pas d'image) avec `--view=board` ; l'enfant charge la carte de campagne
## (France, graine 1337) pour lire les vraies données `live` (recrutement, armée, constructions,
## arbre des techniques) puis affiche les infobulles une à une.
## `--mode=avant` : panneau BBCode historique (`RichTooltip.make_panel`) ;
## `--mode=apres` : rendu en sections (`TooltipView.build`), versions courte et complète.
## Usage :
##   godot --path game --script res://tests/ib_shot.gd -- [--mode=avant|apres] [--out=<dossier>]
## `--out` relatif : relatif à la racine du dépôt ; défaut `docs/img/ib/<mode>`.

const MAP_PATHS := preload("res://scripts/map/map_paths.gd")
const RESOLUTION := "1600x900"
const MAP := "res://scenes/campaign_map.tscn"
const TIMEOUT_S := 240.0
const MARGIN := 10


func _init() -> void:
	var mode := "avant"
	var out := ""
	var view := ""
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--out="):
			out = arg.trim_prefix("--out=")
		elif arg.begins_with("--mode="):
			mode = arg.trim_prefix("--mode=")
		elif arg.begins_with("--view="):
			view = arg.trim_prefix("--view=")
	if out == "":
		out = "docs/img/ib/" + mode
	var repo := ProjectSettings.globalize_path("res://").path_join("..").simplify_path()
	var out_dir := out if out.is_absolute_path() else repo.path_join(out)
	if view == "board":
		_board.call_deferred(mode, out_dir)
		return
	DirAccess.make_dir_recursive_absolute(out_dir)
	var args := PackedStringArray(["--path", ProjectSettings.globalize_path("res://"), "--resolution", RESOLUTION,
		"--script", "res://tests/ib_shot.gd", "--", "--view=board", "--mode=" + mode, "--out=" + out_dir])
	var output := []
	var code := OS.execute(OS.get_executable_path(), args, output, true)
	var text := "".join(output)
	for line in text.split("\n"):
		if line.begins_with("IB_SHOT"):
			print(line)
	if code != 0:
		print("IB_SHOT FAIL (code %d)\n%s" % [code, text.right(2000)])
	quit(code)


## Enfant fenêtré : carte chargée, infobulles affichées une à une et capturées.
func _board(mode: String, out_dir: String) -> void:
	create_timer(TIMEOUT_S).timeout.connect(func() -> void:
		print("IB_SHOT timeout")
		quit(1))
	await process_frame
	var settings: Node = root.get_node_or_null("/root/Settings")
	if settings != null:
		settings.call("use_test_file")
		settings.call("set_value", "game/autosave_interval", 0, false)
		settings.call("set_value", "tutorial/enabled", false, false)
		settings.call("set_value", "interface/season_report", false, false)
		settings.call("_apply_ui_scale")
	var facade: Node = root.get_node("/root/SimFacade")
	facade.set_data_dir(MAP_PATHS.default_data_dir())
	facade.pending_faction = "fac_france"
	facade.pending_seed = 1337
	facade.pending_load_path = ""
	var map: Node3D = (load(MAP) as PackedScene).instantiate()
	root.add_child(map)
	current_scene = map
	for i in 20:
		await process_frame
	var views := _views(map)
	var layer := CanvasLayer.new()
	layer.layer = 120
	root.add_child(layer)
	var backdrop := ColorRect.new()
	backdrop.color = Color(0.16, 0.18, 0.14)
	backdrop.set_anchors_preset(Control.PRESET_FULL_RECT)
	layer.add_child(backdrop)
	var failed := 0
	var index := 0
	for entry in views:
		var variants: Array = [[str(entry[0]), false]]
		if mode == "apres" and str(entry[1]) != "":
			variants = [[str(entry[0]) + "-courte", false], [str(entry[0]) + "-complete", true]]
		for variant in variants:
			index += 1
			var panel := _panel(mode, entry, bool(variant[1]))
			panel.position = Vector2(MARGIN * 2, MARGIN * 2)
			layer.add_child(panel)
			for i in 6:
				await process_frame
			await RenderingServer.frame_post_draw
			var image := root.get_viewport().get_texture().get_image()
			var scale := float(image.get_width()) / root.get_visible_rect().size.x
			var rect := Rect2i(Vector2i(panel.get_global_rect().grow(MARGIN).position * scale), Vector2i(panel.get_global_rect().grow(MARGIN).size * scale))
			rect = rect.intersection(Rect2i(Vector2i.ZERO, image.get_size()))
			var path := out_dir.path_join("%02d-%s.jpg" % [index, variant[0]])
			var err := image.get_region(rect).save_jpg(path, 0.85)
			print("IB_SHOT %s %s (%dx%d)" % [path, "OK" if err == OK else "FAIL", rect.size.x, rect.size.y])
			if err != OK:
				failed += 1
			panel.queue_free()
			await process_frame
	quit(0 if failed == 0 else 1)


## [nom, clé `ib:` (vide : pas de spec, repli BBCode), live, BBCode historique].
func _views(map: Node) -> Array:
	var sim: Object = map.sim
	var faction: String = map.player_faction
	var capital := str(root.get_node("/root/SimFacade").call("faction_info", faction).get("capital", ""))
	var views: Array = []
	var recruitable: Array = sim.call("get_recruitable", capital)
	var recruit: Dictionary = recruitable[0] if not recruitable.is_empty() else {"unit_type": "unit_longbowmen"}
	for row in recruitable:
		if str(row.get("unit_type", "")).contains("crossbow"):
			recruit = row
			break
	var recruit_type := str(recruit.get("unit_type", ""))
	views.append(["unite-recrutement", "ib:unit:" + recruit_type, recruit, RichTooltip.unit(recruit_type, recruit)])
	var army_unit: Dictionary = {}
	for army_id in map.player_army_ids():
		var units: Array = sim.call("get_army", army_id).get("units", [])
		if not units.is_empty():
			army_unit = units[0]
			break
	var army_type := str(army_unit.get("unit_type", "unit_men_at_arms"))
	views.append(["unite-armee", "ib:unit:" + army_type, army_unit, RichTooltip.unit(army_type, army_unit)])
	var city: Dictionary = sim.call("get_province_city", capital) if sim.has_method("get_province_city") else {}
	var build_row: Dictionary = {"building": "bld_castle"}
	for row in city.get("buildable", []):
		if not (GameCatalog.building(str(row.get("building", ""))).get("effects", []) as Array).is_empty():
			build_row = row
			break
	var building_id := str(build_row.get("building", "bld_castle"))
	views.append(["batiment", "ib:building:" + building_id, build_row, RichTooltip.building(building_id, build_row)])
	var tech: Dictionary = {}
	for node in sim.call("get_tech_tree", faction):
		if str(node.get("state", "")) == "available" and not (node.get("effects", []) as Array).is_empty():
			tech = node
			break
	views.append(["technique", "ib:technology:" + str(tech.get("id", "")), tech, RichTooltip.technology(tech)])
	views.append(["jauge-province", "", {}, RichTooltip.gauge("unrest", 23)])
	return views


func _panel(mode: String, entry: Array, detailed: bool) -> Control:
	if mode != "apres" or str(entry[1]) == "":
		return RichTooltip.make_panel(str(entry[3]))
	var tooltip_script: GDScript = load("res://scripts/ui/rich_tooltip.gd")
	var view_script: GDScript = load("res://scripts/ui/tooltip_view.gd")
	var spec: Dictionary = tooltip_script.call("spec_for", str(entry[1]), entry[2])
	return view_script.call("build", spec, detailed)
