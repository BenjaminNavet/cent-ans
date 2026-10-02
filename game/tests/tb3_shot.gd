extends SceneTree

## Lot TB3 : captures de contrôle des bâtiments hors les murs et de la croissance des villes, et
## mesure de leur coût. États imposés (`OutbuildingLayer.forced_states`) : la partie n'est pas
## modifiée.
## Usage : godot --path game --resolution 1600x900 --script res://tests/tb3_shot.gd --
##   [--out=<dossier>] [--town=<id>] (défaut set_agen : ferme, vignoble et marché de niveau 3,
##   moulin de niveau 1, abbaye de niveau 1, chantier) [--growth=<id>] (ville ouverte : deux
##   quartiers de faubourg, enceinte de pierre, suie ; défaut : la plus proche de `--town`)
##   [--distances=20,45,90] [--no-before] [--bench] (appels de dessin et ms par image à 20, 45,
##   90 et 400, couche affichée puis masquée ; sans vsync avec `--disable-vsync`)
##   [--coast=<id>] (ville côtière : port et saline de niveau 3 ; défaut set_la_rochelle)
## Écrit, par ville et par distance, `tb3-<id>-<distance>-avant.png` (même cadrage sans le lot :
## couche masquée, suie retirée, comme `--no-tb3`) puis `tb3-<id>-<distance>-apres.png`, à la
## résolution de la fenêtre, HUD masqué ; puis une planche par ville `tb3-planche-<id>.png`
## (800 px de large : une ligne par distance, avant à gauche, après à droite, recadrage central
## de 400 × 300 px sans réduction).

const MAP_PATHS := preload("res://scripts/map/map_paths.gd")
const TILE := Vector2i(400, 300)
const COAST_BUILDINGS := ["bld_port", "bld_fair", "bld_counting_house", "bld_weaving_workshop"]
const SETTLE_FRAMES := 150
const BENCH_FRAMES := 240
const LEVEL_3 := ["bld_fair", "bld_herb_garden", "bld_stables", "bld_weaving_workshop", "bld_windmill", "bld_vineyard_press", "bld_counting_house", "bld_scriptorium"]


func _init() -> void:
	var out_dir := "user://tb3"
	var town_id := "set_agen"
	var growth_id := ""
	var coast_id := "set_la_rochelle"
	var distances: Array[float] = [20.0, 45.0, 90.0]
	var before := true
	var bench := false
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--out="):
			out_dir = arg.trim_prefix("--out=")
		elif arg.begins_with("--town="):
			town_id = arg.trim_prefix("--town=")
		elif arg.begins_with("--growth="):
			growth_id = arg.trim_prefix("--growth=")
		elif arg.begins_with("--coast="):
			coast_id = arg.trim_prefix("--coast=")
		elif arg.begins_with("--distances="):
			distances.clear()
			for d in arg.trim_prefix("--distances=").split(","):
				distances.append(float(d))
		elif arg == "--bench":
			bench = true
		elif arg == "--no-before":
			before = false
	DirAccess.make_dir_recursive_absolute(out_dir)
	await process_frame
	var settings: Node = root.get_node_or_null("/root/Settings")
	if settings != null:
		settings.call("use_test_file")
		settings.call("set_value", "tutorial/enabled", false, false)
	var facade: Node = root.get_node("/root/SimFacade")
	facade.set_data_dir(MAP_PATHS.default_data_dir())
	facade.pending_faction = "fac_france"
	facade.pending_seed = 1337
	facade.pending_load_path = ""
	var map: Node3D = (load("res://scenes/campaign_map.tscn") as PackedScene).instantiate()
	root.add_child(map)
	for i in 5:
		await process_frame
	if not map.get("load_ok"):
		push_error("TB3 shot: campaign map failed to load")
		quit(1)
		return
	for layer in root.find_children("*", "CanvasLayer", true, false):
		(layer as CanvasLayer).visible = false
	var rig: CampaignCamera = map.camera_rig
	var data: MapData = map.get("map_data")
	var settlements: SettlementLayer = map.get("settlement_layer")
	var out := settlements.outbuildings
	var town := settlements.data.get_settlement(town_id)
	if town.is_empty() or out == null:
		push_error("TB3 shot: unknown town %s or no outbuilding layer" % town_id)
		quit(1)
		return
	var town_px: Vector2 = town["px"]
	if growth_id == "":
		growth_id = _open_town_near(settlements, town_px)
	# États imposés : niveaux 1 et 3 autour de la ville, croissance d'une ville ouverte voisine.
	out.forced_states[town_id] = {"buildings": LEVEL_3, "building": true}
	if growth_id != "":
		var growth := settlements.data.get_settlement(growth_id)
		out.forced_states[growth_id] = {"buildings": ["bld_market", "bld_windmill", "bld_stone_walls"], "building": false}
		out.forced_population_ratio[str(growth["province"])] = 1.2
		settlements.set_town_soot(growth_id, 0.8)
	var coast := settlements.data.get_settlement(coast_id)
	if not coast.is_empty():
		out.forced_states[coast_id] = {"buildings": COAST_BUILDINGS, "building": false}
	out.invalidate()
	if bench:
		await _bench(rig, data, out, town_px)
		quit(0)
		return
	var shots := [[town_id, town_px]]
	if growth_id != "":
		shots.append([growth_id, settlements.data.get_settlement(growth_id)["px"]])
	if not coast.is_empty():
		shots.append([coast_id, coast["px"]])
	for shot: Array in shots:
		var sheet := Image.create(TILE.x * 2, TILE.y * distances.size(), false, Image.FORMAT_RGB8)
		var row := 0
		var px: Vector2 = shot[1]
		var ground := Vector3(px.x, data.surface_world_at(px.x, px.y), px.y)
		for distance in distances:
			rig.look_at_point(ground, maxf(distance, rig.min_distance_at(ground)))
			rig.snap()
			if before:  # même cadrage sans le lot
				out.enabled = false
				if growth_id != "":
					settlements.set_town_soot(growth_id, 0.0)
				for i in SETTLE_FRAMES:
					await process_frame
				_tile(sheet, _shot(out_dir, "tb3-%s-%d-avant" % [shot[0], int(distance)]), Vector2i(0, row))
				out.enabled = true
				if growth_id != "":
					settlements.set_town_soot(growth_id, 0.8)
			for i in SETTLE_FRAMES:
				await process_frame
			print("TB3 %s d=%.0f : %s" % [shot[0], distance, _summary(out, str(shot[0]))])
			_tile(sheet, _shot(out_dir, "tb3-%s-%d-apres" % [shot[0], int(distance)]), Vector2i(1, row))
			row += 1
		sheet.save_png(out_dir.path_join("tb3-planche-%s.png" % shot[0]))
		print("TB3 planche %s" % out_dir.path_join("tb3-planche-%s.png" % shot[0]))
	quit(0)


## Ville ouverte du plan de 1340 la plus proche de `px` (croissance : faubourgs, enceinte).
func _open_town_near(settlements: SettlementLayer, px: Vector2) -> String:
	var best := ""
	var best_d := INF
	var towns: Dictionary = settlements.towns.data.towns
	for entry in settlements.data.settlements:
		var id := str(entry["id"])
		if not towns.has(id) or str(towns[id]["walls"]) != "none" or str(entry["kind"]) != "town":
			continue
		var d := (entry["px"] as Vector2).distance_to(px)
		if d > 0.5 and d < best_d:
			best_d = d
			best = id
	return best


func _summary(out: OutbuildingLayer, id: String) -> String:
	var parts := PackedStringArray()
	var growth := {}
	for inst: Dictionary in out.instances_of(id):
		var family := str(inst["family"])
		if family == "suburb" or family == "enclosure":
			growth[family] = int(growth.get(family, 0)) + 1
		else:
			parts.append("%s %d" % [family, int(inst["level"])])
	for family: String in growth:
		parts.append("%s ×%d" % [family, growth[family]])
	return "%s (%d instances, %d nœuds de rendu ; lecture de l'état %.1f ms, voisinage %.1f ms)" % [", ".join(parts), out.instance_count(), out.node_count(), float(out.stats.get("read_ms", 0.0)), float(out.stats.get("build_ms", 0.0))]


## Appels de dessin et temps par image, couche TB3 affichée puis masquée, aux distances de jeu
## (20, 45, 90) et à 400 (hors de portée).
func _bench(rig: CampaignCamera, data: MapData, out: OutbuildingLayer, px: Vector2) -> void:
	var ground := Vector3(px.x, data.surface_world_at(px.x, px.y), px.y)
	for distance: float in [20.0, 45.0, 90.0, 400.0]:
		rig.look_at_point(ground, maxf(distance, rig.min_distance_at(ground)))
		rig.snap()
		for enabled: bool in [true, false, true, false]:
			out.enabled = enabled
			for i in SETTLE_FRAMES if enabled else 30:
				await process_frame
			var draws := 0.0
			var t0 := Time.get_ticks_usec()
			for i in BENCH_FRAMES:
				await process_frame
				draws += Performance.get_monitor(Performance.RENDER_TOTAL_DRAW_CALLS_IN_FRAME)
			print("TB3 bench d=%d couche %s : %.0f appels de dessin, %.2f ms/image (%d instances, %d nœuds ; mise en place %.0f ms pour %d colonies)" % [int(distance), "oui" if enabled else "non", draws / BENCH_FRAMES, (Time.get_ticks_usec() - t0) / float(BENCH_FRAMES) / 1000.0, out.instance_count() if out.visible else 0, out.node_count() if out.visible else 0, float(out.stats.get("build_ms", 0.0)), int(out.stats.get("settlements", 0))])
	out.enabled = true


func _shot(out_dir: String, name: String) -> Image:
	var image := root.get_viewport().get_texture().get_image()
	var path := out_dir.path_join("%s.png" % name)
	image.save_png(path)
	print("TB3 shot %s" % path)
	return image


## Recadrage central de `TILE` px, sans réduction, posé dans la case `cell` de la planche.
func _tile(sheet: Image, image: Image, cell: Vector2i) -> void:
	var size := Vector2i(mini(TILE.x, image.get_width()), mini(TILE.y, image.get_height()))
	var region := image.get_region(Rect2i((image.get_size() - size) / 2, size))
	region.convert(Image.FORMAT_RGB8)
	sheet.blit_rect(region, Rect2i(Vector2i.ZERO, size), cell * TILE)
