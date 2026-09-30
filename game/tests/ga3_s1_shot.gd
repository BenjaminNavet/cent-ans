extends SceneTree

## Sonde GA3-S1 : maison à colombages issue de TRELLIS (fal.ai), nettoyée par
## `tools/blender_scripts/ga3_cleanup.py`. Charge les trois LOD, imprime triangles et
## dimensions (contrôle texte : LOD0 ≤ 8 000 triangles, ~8 m de long, pied à y = 0), puis
## capture le LOD0 en vue 3/4 sur un sol neutre (planche `docs/img/ga3/s1_house.jpg`).
## Usage (avec affichage, le rendu headless n'écrit pas d'image) :
##   godot --path game --resolution 800x800 --script res://tests/ga3_s1_shot.gd -- --out=<dossier>
## Sans `--out`, seul le contrôle texte est fait (headless possible).

const LODS := [
	"res://assets/models/props_ga/ga3_house_lod0.glb",
	"res://assets/models/props_ga/ga3_house_lod1.glb",
	"res://assets/models/props_ga/ga3_house_lod2.glb",
]
const MAX_LOD0_TRIANGLES := 8000

var _out := ""


func _init() -> void:
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--out="):
			_out = arg.trim_prefix("--out=")
	await process_frame
	var ok := true
	var houses: Array[Node3D] = []
	for i in LODS.size():
		var scene := load(LODS[i]) as PackedScene
		if scene == null:
			push_error("ga3_s1_shot: cannot load %s" % LODS[i])
			quit(1)
			return
		var house := scene.instantiate() as Node3D
		houses.append(house)
		var stats := _stats(house)
		var box: AABB = stats["aabb"]
		print("ga3_s1_shot: lod%d %d triangles, %.2f x %.2f x %.2f m, pied y=%.2f" % [
			i, stats["triangles"], box.size.x, box.size.y, box.size.z, box.position.y])
		if i == 0 and (int(stats["triangles"]) > MAX_LOD0_TRIANGLES or absf(box.position.y) > 0.05):
			ok = false
	if not ok:
		push_error("ga3_s1_shot: LOD0 over budget or origin not at the foot")
		quit(1)
		return
	if _out == "":
		for house in houses:
			house.free()
		print("ga3_s1_shot: OK")
		quit(0)
		return
	for i in range(1, houses.size()):
		houses[i].free()
	_build_stage(houses[0])
	for _i in 10:
		await process_frame
	await RenderingServer.frame_post_draw
	var image := root.get_viewport().get_texture().get_image()
	DirAccess.make_dir_recursive_absolute(_out)
	var path := _out.path_join("ga3_s1_lod0.png")
	print("ga3_s1_shot: %s (%s)" % [path, error_string(image.save_png(path))])
	quit(0)


func _build_stage(house: Node3D) -> void:
	var stage := Node3D.new()
	root.add_child(stage)
	stage.add_child(house)
	var env := Environment.new()
	env.background_mode = Environment.BG_COLOR
	env.background_color = Color(0.5, 0.5, 0.5)
	env.ambient_light_source = Environment.AMBIENT_SOURCE_COLOR
	env.ambient_light_color = Color(0.75, 0.76, 0.8)
	env.ambient_light_energy = 0.6
	env.tonemap_mode = Environment.TONE_MAPPER_FILMIC
	var world_env := WorldEnvironment.new()
	world_env.environment = env
	stage.add_child(world_env)
	var sun := DirectionalLight3D.new()
	sun.rotation_degrees = Vector3(-50, -35, 0)
	sun.light_energy = 1.3
	sun.shadow_enabled = true
	stage.add_child(sun)
	var ground := MeshInstance3D.new()
	var plane := PlaneMesh.new()
	plane.size = Vector2(40, 40)
	ground.mesh = plane
	var ground_mat := StandardMaterial3D.new()
	ground_mat.albedo_color = Color(0.36, 0.34, 0.28)
	ground.material_override = ground_mat
	stage.add_child(ground)
	var camera := Camera3D.new()
	camera.fov = 40.0
	stage.add_child(camera)
	var box := _stats(house)["aabb"] as AABB
	var centre := box.get_center()
	camera.position = centre + Vector3(1.0, 0.65, 1.2).normalized() * box.size.length() * 1.55
	camera.look_at(centre, Vector3.UP)
	camera.make_current()


func _stats(node: Node) -> Dictionary:
	var triangles := 0
	var box := AABB()
	var first := true
	var stack: Array[Node] = [node]
	while not stack.is_empty():
		var current: Node = stack.pop_back()
		stack.append_array(current.get_children())
		if current is MeshInstance3D and (current as MeshInstance3D).mesh != null:
			var mesh := (current as MeshInstance3D).mesh
			for s in mesh.get_surface_count():
				var arrays := mesh.surface_get_arrays(s)
				var indices: PackedInt32Array = arrays[Mesh.ARRAY_INDEX]
				triangles += (indices.size() if indices.size() > 0 else (arrays[Mesh.ARRAY_VERTEX] as PackedVector3Array).size()) / 3
			var local := (current as MeshInstance3D).get_aabb()
			var xf := _to_root(current as Node3D, node)
			local = xf * local
			box = local if first else box.merge(local)
			first = false
	return {"triangles": triangles, "aabb": box}


func _to_root(n: Node3D, top: Node) -> Transform3D:
	var xf := Transform3D.IDENTITY
	var current: Node = n
	while current != null and current != top.get_parent():
		if current is Node3D:
			xf = (current as Node3D).transform * xf
		current = current.get_parent()
	return xf
