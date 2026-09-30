extends SceneTree

## Lot NT10 : captures de contrôle des imposteurs (écrites, jamais lues par l'agent ; la session
## principale juge le rendu). Hors simulation, prairie plate, comme `bv3_shot.gd`.
## Usage (avec affichage, pas en headless) :
##   godot --path game --resolution 1280x720 --script res://tests/nt10_shot.gd -- \
##     [--out-dir=<dossier>] [--fig=infantry_0] [--no-nt10]
## Écrit dans `docs/audit/captures/nt/` (défaut) :
## - `nt10_atlas.png` : atlas cuit (NT10 : une bande de 8 colonnes par copie, casque et habit) ;
## - `nt10_crowd_near.png` : deux blocs d'imposteurs vus à ~45 m (gauche sans sang, droite
##   ensanglantée), ombres en disque au sol ;
## - `nt10_crowd_far.png` : mêmes blocs vus de ~350 m, distance réelle des imposteurs.

const SKY_SHADER := preload("res://shaders/battle_sky.gdshader")

var _center := Vector3(600, 0, 400)
var _out_dir := "../docs/audit/captures/nt"


func _init() -> void:
	var fig := "infantry_0"
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--out-dir="):
			_out_dir = arg.trim_prefix("--out-dir=")
		elif arg.begins_with("--fig="):
			fig = arg.trim_prefix("--fig=")
	_out_dir = ProjectSettings.globalize_path("res://").path_join(_out_dir).simplify_path() if _out_dir.is_relative_path() else _out_dir
	DirAccess.make_dir_recursive_absolute(_out_dir)
	await process_frame
	var world := Node3D.new()
	root.add_child(world)
	_environment(world)
	var terrain := BattleTerrain.new()
	world.add_child(terrain)
	var heights := PackedFloat32Array()
	heights.resize(121 * 81)
	terrain.build({"nx": 121, "nz": 81, "resolution": 10.0, "heights": heights, "terrain": "plains", "season": "summer", "ground": "dry", "woodland": 0.0}, "clear")
	var camera := Camera3D.new()
	camera.fov = 45.0
	camera.far = 3000.0
	world.add_child(camera)
	var soldiers := BattleSoldiers.new()
	world.add_child(soldiers)
	soldiers.setup([], {"attacker": Color(0.16, 0.25, 0.62), "defender": Color(0.72, 0.12, 0.12)}, {"attacker": "fac_france", "defender": "fac_england"})
	var parts := fig.rsplit("_", true, 1)
	var kind := parts[0]
	var variant := int(parts[1])
	var impostors := BattleImpostors.new()
	world.add_child(impostors)
	var mat: ShaderMaterial = soldiers.call("_make_skinned_material", "attacker", kind, variant, false)
	var key := BattleImpostors.key_of("attacker", kind, variant)
	impostors.request(key, kind, variant, mat)
	for i in 30:
		await process_frame
		if impostors.is_ready(key):
			break
	if not impostors.is_ready(key):
		push_error("nt10_shot: atlas not baked")
		quit(1)
		return
	var failures := 0
	var atlas_path := _out_dir.path_join("nt10_atlas.png")
	failures += 0 if impostors.atlas_image(key).save_png(atlas_path) == OK else 1
	print("nt10_shot: %s" % atlas_path)
	var spacing := 3.0 if kind == "cavalry" else 1.2
	for block in 2:
		var mm := MultiMesh.new()
		mm.transform_format = MultiMesh.TRANSFORM_3D
		mm.mesh = BattleImpostors.quad_mesh()
		mm.instance_count = 16 * 12
		for k in mm.instance_count:
			var yaw := PI + (float(k % 7) - 3.0) * 0.08
			var x := _center.x + (float(block) - 0.5) * spacing * 20.0 + (float(k % 16) - 7.5) * spacing
			var z := _center.z + (float(k / 16) - 5.5) * spacing * 1.1
			mm.set_instance_transform(k, Transform3D(Basis(Vector3.UP, yaw), Vector3(x, terrain.world_height(x, z), z)))
		var imp := MultiMeshInstance3D.new()
		imp.multimesh = mm
		var imp_mat := impostors.make_material(key)
		imp_mat.set_shader_parameter("imp_set", 3)
		imp_mat.set_shader_parameter("blood", 0.0 if block == 0 else 0.8)
		imp.material_override = imp_mat
		imp.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
		world.add_child(imp)
		if BattleImpostors.nt10_enabled():
			var shadow := MultiMeshInstance3D.new()
			shadow.multimesh = mm
			shadow.material_override = impostors.make_shadow_material(key)
			imp.add_child(shadow)
	var views := {
		"nt10_crowd_near.png": [Vector3(0, 18, 42), Vector3(0, 0.8, 0)],
		"nt10_crowd_far.png": [Vector3(0, 140, 320), Vector3(0, 0.8, 0)],
	}
	for name in views:
		var view: Array = views[name]
		camera.look_at_from_position(_center + (view[0] as Vector3), _center + (view[1] as Vector3))
		for i in 8:
			await process_frame
		await RenderingServer.frame_post_draw
		var path := _out_dir.path_join(name)
		failures += 0 if root.get_texture().get_image().save_png(path) == OK else 1
		print("nt10_shot: %s" % path)
	print("nt10_shot: %s (%s)" % ["OK" if failures == 0 else "FAIL", _out_dir])
	quit(0 if failures == 0 else 1)


func _environment(world: Node3D) -> void:
	var sky_mat := ShaderMaterial.new()
	sky_mat.shader = SKY_SHADER
	var sky := Sky.new()
	sky.sky_material = sky_mat
	var env := Environment.new()
	env.background_mode = Environment.BG_SKY
	env.sky = sky
	env.ambient_light_source = Environment.AMBIENT_SOURCE_SKY
	env.tonemap_mode = Environment.TONE_MAPPER_AGX
	var we := WorldEnvironment.new()
	we.environment = env
	world.add_child(we)
	var sun := DirectionalLight3D.new()
	sun.rotation = Vector3(deg_to_rad(-48), deg_to_rad(-35), 0)
	sun.light_color = Color(1, 0.94, 0.84)
	sun.light_energy = 1.6
	sun.shadow_enabled = true
	world.add_child(sun)
