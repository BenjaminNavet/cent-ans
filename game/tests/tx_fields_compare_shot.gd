extends SceneTree

## TX T2b4 : planche de comparaison des champs. Une parcelle de blé en Beauce (48,3 N 1,6 E) à
## hauteur de caméra proche, trois vues côte à côte (gauche -> droite) :
##   1. modèles DN sur le sol actuel (parcels_source hb) ;
##   2. modèles DN sur le sol TX (parcels_source tx) ;
##   3. sol TX seul (champs DN masqués).
## Usage : tools/godot_bg.sh --path game --resolution 960x600 --script res://tests/tx_fields_compare_shot.gd -- \
##   [--out=<fichier.png>] [--distance=9] [--lon=1.6] [--lat=48.3] [--material=wheat_ripe]
## Écrit hors dépôt (par défaut ~/dev/cent-ans-raw/textures/planches/fields_compare.png).

const MAP_PATHS := preload("res://scripts/map/map_paths.gd")
const SCAN := 14


func _wheat_site(fields: FieldLayer, center: Vector2, material: String) -> Vector2:
	var size := float(fields.plan._plan.get("parcel_px", 0.8))
	var origin := Vector2i(center / size)
	var best := Vector2.ZERO
	var best_d := 1e9
	for dy in range(-SCAN, SCAN + 1):
		for dx in range(-SCAN, SCAN + 1):
			var id := origin + Vector2i(dx, dy)
			var parcel: Dictionary = fields.plan.parcel(id)
			if parcel.is_empty() or str(parcel["material"]) != material:
				continue
			var d := (parcel["site"] as Vector2).distance_to(center)
			if d < best_d:
				best_d = d
				best = parcel["site"]
	return best


func _settle(frames: int) -> Image:
	for _i in frames:
		await process_frame
	await RenderingServer.frame_post_draw
	return root.get_texture().get_image()


func _init() -> void:
	var out := CmdArgs.value("--out", OS.get_environment("HOME") + "/dev/cent-ans-raw/textures/planches/fields_compare.png")
	var distance := CmdArgs.number("--distance", 9.0)
	var material := CmdArgs.value("--material", "wheat_ripe")
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
	HbGround.forced_source = "hb"
	var map: Node3D = (load("res://scenes/campaign_map.tscn") as PackedScene).instantiate()
	root.add_child(map)
	for _i in 5:
		await process_frame
	var rig: CampaignCamera = map.camera_rig
	rig.edge_pan_enabled = false
	for layer in root.find_children("*", "CanvasLayer", true, false):
		(layer as CanvasLayer).visible = false
	RenderingServer.global_shader_parameter_set("campaign_season", Vector4(0.0, 1.0, 0.0, 0.0))
	var map_data: MapData = map.get("map_data")
	var fields: FieldLayer = root.find_child("Fields", true, false)
	var center := FaunaLayer.lonlat_to_px(CmdArgs.number("--lon", 1.6), CmdArgs.number("--lat", 48.3), map_data)
	var site := _wheat_site(fields, center, material)
	if site == Vector2.ZERO:
		push_error("tx_fields_compare_shot: aucune parcelle %s près de %s" % [material, center])
		quit(1)
		return
	print("tx_fields_compare_shot: parcelle %s en px %s" % [material, site])
	rig.look_at_point(Vector3(site.x, map_data.surface_world_at(site.x, site.y), site.y), distance)
	rig.snap()
	var data_dir: String = map_data.map_dir.get_base_dir()
	var first: Image = await _settle(300)
	HbGround.forced_source = "tx"
	var applied := HbGround.apply(map.terrain.material, data_dir)
	print("tx_fields_compare_shot: sol TX appliqué=", applied, " source=", GroundMaterials.source)
	var second: Image = await _settle(60)
	fields.enabled = false
	fields.visible = false
	var third: Image = await _settle(30)
	var board := Image.create(first.get_width() * 3, first.get_height(), false, first.get_format())
	for index in 3:
		board.blit_rect([first, second, third][index], Rect2i(Vector2i.ZERO, first.get_size()), Vector2i(index * first.get_width(), 0))
	DirAccess.make_dir_recursive_absolute(out.get_base_dir())
	board.save_png(out)
	print("tx_fields_compare_shot: planche -> ", out)
	map.queue_free()
	await process_frame
	quit(0)
