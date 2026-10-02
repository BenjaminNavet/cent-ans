extends SceneTree

## Lot TB4 : captures de contrôle des traces de guerre et de peste (fichiers écrits, à juger par
## la session principale). Paris et ses abords, états forcés (aucune règle de jeu touchée) :
##   brulis   : Île-de-France dévastée à 100 %, vue à 40 ;
##   peste    : fosses et charrette des morts au bord de Paris (scène `plague` forcée), groupe
##              cadré à 20 ; tailles à l'écran imprimées ;
##   portes   : portes marquées d'une croix dans la ville 1:1 ordinaire la plus proche de Paris,
##              au plancher de la caméra ; taille de la croix imprimée ;
##   bataille : champ de bataille frais (tertre, corbeaux, débris) à l'est de Paris, vue à 22 ;
##   siege    : première armée du joueur en posture de siège, échelles prêtes, bélier presque
##              prêt, vue à 30.
## Usage : godot --path game --resolution 1600x900 --script res://tests/tb4_shot.gd --
##   [--out=<dossier>] [--only=brulis,peste,portes,bataille,siege] [--season=summer]
## Écrit `tb4-<scène>.png` (960 px de large) ; HUD masqué.

const MAP_PATHS := preload("res://scripts/map/map_paths.gd")
const PARIS := Vector2(2213.2, 3203.9)
const PROVINCE := "prov_ile_de_france"
const SETTLEMENT := "set_paris"
const WIDTH := 960
const SETTLE_FRAMES := 150


func _init() -> void:
	var out_dir := "user://tb4"
	var only := PackedStringArray()
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--out="):
			out_dir = arg.trim_prefix("--out=")
		elif arg.begins_with("--only="):
			only = arg.trim_prefix("--only=").split(",", false)
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
		push_error("TB4 shot: campaign map failed to load")
		quit(1)
		return
	for layer in root.find_children("*", "CanvasLayer", true, false):
		(layer as CanvasLayer).visible = false
	var life: CampaignLife = map.get("life")
	var scars: WarScars = life.scars if life != null else null
	if scars == null:
		push_error("TB4 shot: war scars disabled")
		quit(1)
		return
	var rig: CampaignCamera = map.camera_rig
	var data: MapData = map.get("map_data")
	var sim: Object = map.get("sim")

	if _wanted(only, "brulis"):
		life.forced_devastation[PROVINCE] = 100.0
		life.invalidate()
		map.call("refresh_all")
		await _look(rig, data, PARIS, 40.0)
		_shot(out_dir, "tb4-brulis")
		life.forced_devastation.clear()
		life.invalidate()
		map.call("refresh_all")

	if _wanted(only, "peste") and life.folk_scenes != null:
		life.folk_scenes.forced = [{"province": PROVINCE, "kind": "plague", "settlement": SETTLEMENT, "intensity": 1.0}]
		life.invalidate()
		map.call("refresh_all")
		var site := scars.plague_node(SETTLEMENT)
		if site == null:
			push_error("TB4 shot: no plague site at %s" % SETTLEMENT)
		else:
			# Cadrage sur le groupe (fosses et charrette), à une distance de jeu du palier proche.
			await _look(rig, data, PARIS, 20.0)
			var middle := Vector2.ZERO
			for child in site.get_children():
				middle += Vector2((child as Node3D).position.x, (child as Node3D).position.z) / site.get_child_count()
			await _look(rig, data, middle, 20.0)
			print("TB4 shot plague %s centre %s %s" % [scars.stats, middle, _screen_report(map.get("camera"), site)])
			_shot(out_dir, "tb4-peste")
		life.folk_scenes.forced = []
		life.invalidate()
		map.call("refresh_all")

	if _wanted(only, "portes") and life.folk_scenes != null:
		# Ville 1:1 ordinaire la plus proche de Paris (pas de portes marquées dans les villes
		# emblématiques), vue au plancher de la caméra.
		var layer: SettlementLayer = map.get("settlement_layer")
		var town := ""
		var best := INF
		for id in layer.towns.town_ids():
			if layer.landmark_cities != null and layer.landmark_cities.has_city(id):
				continue
			var d := layer.towns.data.anchor_of(id).distance_to(PARIS)
			if d < best:
				best = d
				town = id
		var entry: Dictionary = layer.data.get_settlement(town) if town != "" else {}
		if entry.is_empty():
			push_error("TB4 shot: no ordinary 1:1 town")
		else:
			life.folk_scenes.forced = [{"province": str(entry["province"]), "kind": "plague", "settlement": town, "intensity": 1.0}]
			life.invalidate()
			map.call("refresh_all")
			var anchor: Vector2 = layer.towns.data.anchor_of(town)
			await _look(rig, data, anchor, 0.5)  # charge la ville 1:1 : les portes se posent
			var site := scars.plague_node(town)
			var door := site.get_node_or_null("Door_0") as Node3D if site != null else null
			if door == null:
				push_error("TB4 shot: no marked door at %s (town plan not loaded)" % town)
			else:
				await _look(rig, data, Vector2(door.position.x, door.position.z), 0.0)
				var camera: Camera3D = map.get("camera")
				var cross_px := (camera.unproject_position(door.global_position + Vector3.UP * WarScarMeshes.DOOR_CROSS_M / 719.0) - camera.unproject_position(door.global_position)).length()
				print("TB4 shot doors %s town %s at %s rig distance %.2f cross %.1f px" % [scars.stats, town, door.position, rig.distance, cross_px])
				_shot(out_dir, "tb4-portes")
			life.folk_scenes.forced = []
			life.invalidate()
			map.call("refresh_all")

	if _wanted(only, "bataille"):
		var field := PARIS + Vector2(14.0, 6.0)
		scars.add_battlefield("shot", field, int(sim.call("get_turn")), "")
		await _look(rig, data, field, 22.0)
		print("TB4 shot battlefield %s at %s" % [scars.stats, field])
		_shot(out_dir, "tb4-bataille")

	if _wanted(only, "siege"):
		var ids: PackedStringArray = map.call("player_army_ids")
		if ids.is_empty():
			push_error("TB4 shot: no player army")
		else:
			var army_id := str(ids[0])
			sim.call("submit_order", {"type": "set_stance", "army": army_id, "stance": "siege"})
			scars.forced_engines[army_id] = [
				{"kind": "ladders", "ready": true, "turns_left": 0},
				{"kind": "ram", "ready": false, "turns_left": 1},
				{"kind": "tower", "ready": false, "turns_left": 6},
			]
			life.invalidate()
			map.call("refresh_all")
			var world: Vector3 = map.armies.world_position_of(army_id)
			await _look(rig, data, Vector2(world.x, world.z), 30.0)
			print("TB4 shot siege %s engines %s at %s" % [army_id, scars.engine_names(army_id), world])
			_shot(out_dir, "tb4-siege")
	quit(0)


## Taille à l'écran (px, plus grande dimension au sol) et présence dans le champ de chaque élément
## d'un site de peste, portes exclues.
func _screen_report(camera: Camera3D, site: Node3D) -> String:
	var view := root.get_viewport().get_visible_rect()
	var parts := PackedStringArray()
	for child in site.get_children():
		var prop := child as MeshInstance3D
		if prop == null or str(prop.name).begins_with("Door_"):
			continue
		var box := prop.mesh.get_aabb()
		var axis := Vector3.RIGHT if box.size.x >= box.size.z else Vector3.BACK
		var half := maxf(box.size.x, box.size.z) * 0.5
		var a := camera.unproject_position(prop.global_transform * (box.get_center() - axis * half))
		var b := camera.unproject_position(prop.global_transform * (box.get_center() + axis * half))
		parts.append("%s %.0f px %s" % [prop.name, a.distance_to(b), "in" if view.has_point(camera.unproject_position(prop.global_position)) else "OUT"])
	return ", ".join(parts)


func _wanted(only: PackedStringArray, scene: String) -> bool:
	return only.is_empty() or only.has(scene)


func _look(rig: CampaignCamera, data: MapData, at: Vector2, distance: float) -> void:
	var ground := Vector3(at.x, data.surface_world_at(at.x, at.y), at.y)
	rig.look_at_point(ground, maxf(distance, rig.min_distance_at(ground)))
	rig.snap()
	for i in SETTLE_FRAMES:
		await process_frame


func _shot(out_dir: String, shot_name: String) -> void:
	var image := root.get_viewport().get_texture().get_image()
	if image.get_width() > WIDTH:
		image.resize(WIDTH, roundi(image.get_height() * float(WIDTH) / image.get_width()), Image.INTERPOLATE_LANCZOS)
	var path := out_dir.path_join("%s.png" % shot_name)
	image.save_png(path)
	print("TB4 shot %s" % path)
