extends SceneTree

## DN-PAYS : captures de la campagne vivante hors champs, sans interface. Chaque vue vise l'instance
## de la règle donnée la plus proche d'un point (lon, lat).
## Usage : tools/godot_bg.sh --path game --resolution 800x500 \
##   --script res://tests/dn_pays_shot.gd -- --out=<dossier> [--only=bocage] [--distances=6,20]
## Les PNG sont écrits hors dépôt (jamais commités).

const MAP_PATHS := preload("res://scripts/map/map_paths.gd")
# nom, règle, lon, lat
const VIEWS := [
	["bocage", "bocage_enclos", -1.4, 48.0],
	["salines", "salines_atlantique", -2.45, 47.3],
	["moulins", "moulins_polder", 4.3, 52.0],
	["route", "charrettes_bord_de_route", 2.5, 48.5],
	["caravane", "caravanes_chameaux", 6.0, 34.0],
]


func _init() -> void:
	var out := CmdArgs.value("--out", "user://pays_shots")
	var distances := CmdArgs.value("--distances", "6,20").split(",")
	var only := CmdArgs.value("--only", "")
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
	for _i in 5:
		await process_frame
	var rig: CampaignCamera = map.camera_rig
	rig.edge_pan_enabled = false
	rig.floor_distance = 0.0
	for layer in root.find_children("*", "CanvasLayer", true, false):
		(layer as CanvasLayer).visible = false
	var data: MapData = map.get("map_data")
	var layer_node: CountrysideLayer = map.life.countryside
	layer_node.force_active = true
	# Sans simulation, personne ne pose la saison (poids du shader des props) : plein été.
	RenderingServer.global_shader_parameter_set("campaign_season", Vector4(0.0, 1.0, 0.0, 0.0))
	DirAccess.make_dir_recursive_absolute(out)
	for view in VIEWS:
		if only != "" and not String(view[0]).contains(only):
			continue
		var px := FaunaLayer.lonlat_to_px(float(view[2]), float(view[3]), data)
		var best := px
		var best_d := 1e9
		for cy in range(floori(px.y / 96.0) - 1, floori(px.y / 96.0) + 2):
			for cx in range(floori(px.x / 96.0) - 1, floori(px.x / 96.0) + 2):
				for inst: Dictionary in layer_node.cell_instances(Vector2i(cx, cy)):
					var d := (inst["pos"] as Vector2).distance_to(px)
					if inst["rule"] == view[1] and d < best_d:
						best_d = d
						best = inst["pos"]
		for dist_text in distances:
			rig.look_at_point(Vector3(best.x, data.surface_world_at(best.x, best.y), best.y), float(dist_text))
			rig.snap()
			for _i in 150:
				await process_frame
			await RenderingServer.frame_post_draw
			var path := out.path_join("%s_d%s.png" % [view[0], dist_text])
			root.get_texture().get_image().save_png(path)
			var st: Dictionary = layer_node._states.get("wattle_fence", {})
			print("season=", RenderingServer.global_shader_parameter_get("campaign_season"), " fence mult=", st.get("last_mult"), " real=", st.get("real_units"))
			for node in layer_node.get_children():
				var mmi := node as MultiMeshInstance3D
				if mmi != null and str(mmi.get_meta("prop")) == "wattle_fence":
					var mat := mmi.multimesh.mesh.surface_get_material(0) as ShaderMaterial
					print("MMI ", mmi.name, " vis=", mmi.is_visible_in_tree(), " n=", mmi.multimesh.instance_count, " xf0=", mmi.multimesh.get_instance_transform(0), " custom0=", mmi.multimesh.get_instance_custom_data(0), " aabb=", mmi.multimesh.mesh.get_aabb(), " size_mult=", mat.get_shader_parameter("size_mult"), " x_mult=", mat.get_shader_parameter("x_mult"), " fade=", mat.get_shader_parameter("fade"), " thin=", mat.get_shader_parameter("thin"), " origin=", mat.get_shader_parameter("mesh_origin"))
					break
			print("rig focus=", rig.get("focus"), " dist=", rig.get("distance"), " cam=", root.get_camera_3d().global_position if root.get_camera_3d() else "none", " target=", best)
			var cam := root.get_camera_3d()
			for node in layer_node.get_children():
				var mmi2 := node as MultiMeshInstance3D
				if mmi2 == null or not mmi2.visible:
					continue
				var mm := mmi2.multimesh
				var st2: Dictionary = layer_node._states[str(mmi2.get_meta("prop"))]
				var spread: float = pow(float(st2.get("last_mult", 1.0)), 0.5)
				var onscreen := 0
				for n in mm.instance_count:
					var xf := mm.get_instance_transform(n)
					var cd := mm.get_instance_custom_data(n)
					var wx := xf.origin.x + cd.r * (spread - 1.0)
					var wz := xf.origin.z + cd.g * (spread - 1.0)
					var world := Vector3(wx, data.display_height(xf.origin.y, wx, wz), wz)
					var sp := cam.unproject_position(world)
					if not cam.is_position_behind(world) and sp.x > 0 and sp.y > 0 and sp.x < 800 and sp.y < 500:
						onscreen += 1
				print("PROJ ", mmi2.name, " onscreen=", onscreen, " mult=", st2.get("last_mult"))
			print("dn_pays_shot: ", path, " px=", best, " stats=", layer_node.stats)
	map.queue_free()
	await process_frame
	quit(0)
