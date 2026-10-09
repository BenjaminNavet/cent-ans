extends SceneTree

## DN-FLEUVE : planche-contact des glb d'eau (orientation de la proue, échelle relative). Chaque
## modèle du registre est posé avec le cap +X, étiqueté ; une image par groupe.
## Usage : tools/godot_bg.sh --path game --resolution 1280x800 \
##   --script res://tests/dn_water_sheet_shot.gd -- --out=<dossier> [--group=ships|props|bridges]
## Fenêtre en arrière-plan uniquement (tools/godot_bg.sh).

const COLUMNS := 8
const CELL := 7.0

const GROUPS := {
	"ships": ["cog", "cog_baltic", "hulk", "knarr", "longship", "balinger", "keel_humber", "carrack", "caravel", "nao", "galley", "galley_venetian", "galley_merchant_venetian", "galley_genoese", "galley_aragonese", "dromon", "fusta", "ghurab", "felucca", "dhow", "sambuk", "lateen", "barca", "ushkuy", "lodka", "volga_ladia", "rhine_barge", "river_barge", "tow_barge", "dugout", "dugout_reeds", "fishing_boat", "ferry_cable", "shipwreck", "shipyard_slip"],
	"props": ["mill_small", "mill_undershot", "mill_horizontal", "mill_tidal", "mill_floating", "port_pier", "port_crane", "port_beach", "port_arsenal"],
	"bridges": ["bridge_stone_arch", "bridge_stone", "bridge_gothic", "bridge_roman", "bridge_avignon", "bridge_andalusi", "bridge_gate", "bridge_wood", "bridge_trestle", "bridge_covered", "bridge_pontoon"],
}


func _init() -> void:
	var out := CmdArgs.value("--out", "user://dn_water")
	var group := CmdArgs.value("--group", "ships")
	await process_frame
	var ids: Array = GROUPS[group]
	var scene := Node3D.new()
	root.add_child(scene)
	var sun := DirectionalLight3D.new()
	sun.rotation_degrees = Vector3(-55.0, 35.0, 0.0)
	scene.add_child(sun)
	var environment := Environment.new()
	environment.background_mode = Environment.BG_COLOR
	environment.background_color = Color(0.45, 0.55, 0.62)
	environment.ambient_light_source = Environment.AMBIENT_SOURCE_COLOR
	environment.ambient_light_color = Color(0.8, 0.8, 0.8)
	environment.ambient_light_energy = 0.6
	var world := WorldEnvironment.new()
	world.environment = environment
	scene.add_child(world)
	var rows := ceili(float(ids.size()) / COLUMNS)
	for n in ids.size():
		var info := DnWaterModels.info(ids[n], 1)
		var column := n % COLUMNS
		var row := n / COLUMNS
		var center := Vector3(column * CELL, 0.0, row * CELL)
		if info.is_empty():
			continue
		var instance := MeshInstance3D.new()
		instance.mesh = info["mesh"]
		# Pont : longueur 1 dans le registre, agrandi pour la planche.
		var grow := 5.0 if DnWaterModels.model_entry(ids[n]).has("length") == false else 1.0
		instance.transform = DnWaterModels.pose(info, center, Vector2.RIGHT, grow)
		scene.add_child(instance)
		var label := Label3D.new()
		label.text = String(ids[n])
		label.position = center + Vector3(0.0, 0.05, CELL * 0.42)
		label.rotation_degrees = Vector3(-90.0, 0.0, 0.0)
		label.pixel_size = 0.012
		label.modulate = Color(1, 1, 0.6)
		scene.add_child(label)
	var camera := Camera3D.new()
	camera.projection = Camera3D.PROJECTION_ORTHOGONAL
	camera.size = maxf(COLUMNS, rows) * CELL * 0.72
	var middle := Vector3((COLUMNS - 1) * CELL * 0.5, 0.0, (rows - 1) * CELL * 0.5)
	camera.position = middle + Vector3(0.0, 40.0, 30.0)
	camera.look_at_from_position(camera.position, middle, Vector3.UP)
	scene.add_child(camera)
	camera.current = true
	for _i in 12:
		await process_frame
	await RenderingServer.frame_post_draw
	DirAccess.make_dir_recursive_absolute(out)
	var path := out.path_join("sheet_%s.png" % group)
	root.get_texture().get_image().save_png(path)
	print("dn_water_sheet_shot: ", path)
	quit(0)
