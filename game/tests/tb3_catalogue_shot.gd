extends SceneTree

## Lot TB3 (ADR 0162) : planche « catalogue » des maquettes hors les murs et des signes de colonie,
## sur sol neutre, vues comme en jeu (caméra au sud, plongée de 31°, champ de 55°) et à la taille
## d'écran qu'elles tiennent sur la carte (`render.screen` de `data/map/building_models.json`).
## Sert à juger si chaque famille se reconnaît ; les images ne sont pas lues par ce script.
## Usage : godot --path game --resolution 1600x900 --script res://tests/tb3_catalogue_shot.gd --
##   [--out=<dossier>] [--zoom=1] (2 : deux fois plus gros qu'en jeu, pour juger les formes)
##   [--families=farm,mill,…] (colonnes ; défaut : les 8 familles) [--no-signs]
##   [--crop=800x470] (recadrage central ; 0x0 : image entière)
## Rangs, du premier plan au fond : niveau 1, niveau 2, niveau 3, puis signes de colonie
## (village, ville, ville murée, cité, château) et chantier. Écrit `tb3-catalogue[-zN].png`.

const MAP_PATHS := preload("res://scripts/map/map_paths.gd")
const MODEL_DIR := "res://assets/models/outbuildings/"
const MPU := 719.0
const FOV := 55.0
const PITCH_DEG := 31.0
## Distance du rig de référence : les tailles sont celles tenues à l'écran à cette distance.
const RIG := 45.0
const COLUMN_PX := 92.0
const ROW_UNITS := 5.6
const SIGNS: Array[String] = ["sign_village", "sign_town", "sign_walled", "sign_city", "sign_castle", "worksite_1"]


func _init() -> void:
	var out_dir := "user://tb3"
	var zoom := 1.0
	var crop := Vector2i(800, 470)
	var with_signs := true
	var config := OutbuildingLayer.load_config(MAP_PATHS.default_data_dir())
	var families: Array = (config.get("families", {}) as Dictionary).keys()
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--out="):
			out_dir = arg.trim_prefix("--out=")
		elif arg.begins_with("--zoom="):
			zoom = maxf(float(arg.trim_prefix("--zoom=")), 0.1)
		elif arg.begins_with("--families="):
			families = Array(arg.trim_prefix("--families=").split(","))
		elif arg.begins_with("--crop="):
			var parts := arg.trim_prefix("--crop=").split("x")
			crop = Vector2i(int(parts[0]), int(parts[1]))
		elif arg == "--no-signs":
			with_signs = false
	DirAccess.make_dir_recursive_absolute(out_dir)
	await process_frame
	var manifest := OutbuildingLayer.load_manifest()
	var screen: Dictionary = OutbuildingLayer.screen_config(config)
	var world := Node3D.new()
	root.add_child(world)
	_stage(world)
	var span := OutbuildingLayer.view_span(RIG, FOV)
	var px_per_unit := 900.0 / span
	var column := COLUMN_PX / px_per_unit
	for c in families.size():
		for level in [1, 2, 3]:
			var model := "%s_%d" % [families[c], level]
			var entry: Dictionary = manifest.get(model, {})
			if entry.is_empty():
				continue
			var width := maxf(float(entry["length"]), float(entry["depth"]))
			var factor := OutbuildingLayer.model_factor(config, width, level, RIG, MPU, FOV)
			_put(world, model, Vector3((float(c) - float(families.size() - 1) * 0.5) * column, 0.0, (1.5 - float(level)) * ROW_UNITS + ROW_UNITS * 0.5), factor / MPU, float(screen.get("vertical", 1.0)))
	if with_signs:
		var signs: Dictionary = screen.get("signs", {})
		var fractions := {}
		for kind: String in signs:
			fractions[str(signs[kind]["model"])] = float(signs[kind]["fraction"])
			if (signs[kind] as Dictionary).has("walled_model"):
				fractions[str(signs[kind]["walled_model"])] = float(signs[kind]["fraction"])
		for n in SIGNS.size():
			var entry: Dictionary = manifest.get(SIGNS[n], {})
			if entry.is_empty():
				continue
			var width := maxf(float(entry["length"]), float(entry["depth"]))
			var factor := float(fractions.get(SIGNS[n], 0.0)) * span * MPU / width
			var vertical := float(screen.get("sign_vertical", 1.0))
			if factor <= 0.0:  # chantier : maquette de niveau 1
				factor = OutbuildingLayer.model_factor(config, width, 1, RIG, MPU, FOV)
				vertical = float(screen.get("vertical", 1.0))
			_put(world, SIGNS[n], Vector3((float(n) - float(SIGNS.size() - 1) * 0.5) * column * 1.33, 0.0, -2.6 * ROW_UNITS), factor / MPU, vertical)
	var camera := Camera3D.new()
	camera.fov = FOV
	camera.near = 0.05
	camera.far = 4000.0
	world.add_child(camera)
	var focus := Vector3(0.0, 0.0, -0.4 * ROW_UNITS if with_signs else 0.4 * ROW_UNITS)
	var pitch := deg_to_rad(PITCH_DEG)
	camera.look_at_from_position(focus + Vector3(0.0, sin(pitch), cos(pitch)) * RIG / zoom, focus, Vector3.UP)
	camera.current = true
	for i in 30:
		await process_frame
	var image := root.get_viewport().get_texture().get_image()
	if crop.x > 0 and crop.y > 0 and (image.get_width() > crop.x or image.get_height() > crop.y):
		var size := Vector2i(mini(crop.x, image.get_width()), mini(crop.y, image.get_height()))
		image = image.get_region(Rect2i((image.get_size() - size) / 2, size))
	var path := out_dir.path_join("tb3-catalogue%s.png" % ("" if is_equal_approx(zoom, 1.0) else "-z%d" % roundi(zoom)))
	image.save_png(path)
	print("TB3 catalogue %s (%d × %d, familles : %s)" % [path, image.get_width(), image.get_height(), ", ".join(PackedStringArray(families))])
	quit(0)


## Sol neutre (herbe de terroir), soleil du sud-ouest avec ombres, ciel clair.
func _stage(world: Node3D) -> void:
	var ground := MeshInstance3D.new()
	var plane := PlaneMesh.new()
	plane.size = Vector2(600.0, 600.0)
	ground.mesh = plane
	var grass := StandardMaterial3D.new()
	grass.albedo_color = Color(0.36, 0.42, 0.24)
	grass.roughness = 1.0
	ground.material_override = grass
	world.add_child(ground)
	var sun := DirectionalLight3D.new()
	sun.rotation_degrees = Vector3(-48.0, -40.0, 0.0)
	sun.light_energy = 1.3
	sun.shadow_enabled = true
	sun.directional_shadow_max_distance = 120.0
	world.add_child(sun)
	var environment := Environment.new()
	environment.background_mode = Environment.BG_COLOR
	environment.background_color = Color(0.62, 0.72, 0.82)
	environment.ambient_light_source = Environment.AMBIENT_SOURCE_COLOR
	environment.ambient_light_color = Color(0.75, 0.8, 0.9)
	environment.ambient_light_energy = 0.55
	environment.tonemap_mode = Environment.TONE_MAPPER_FILMIC
	var holder := WorldEnvironment.new()
	holder.environment = environment
	world.add_child(holder)


func _put(world: Node3D, model: String, at: Vector3, unit: float, vertical: float) -> void:
	var mesh := BuildingMaterials.remap_mesh(OutbuildingLayer._load_glb_mesh(MODEL_DIR + model + ".glb"), "far")
	if mesh == null:
		push_error("TB3 catalogue: %s missing" % model)
		return
	var node := MeshInstance3D.new()
	node.name = model
	node.mesh = mesh
	node.position = at
	node.scale = Vector3(unit, unit * vertical, unit)
	world.add_child(node)
