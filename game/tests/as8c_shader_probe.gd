extends SceneTree

## Lot AS8c : sonde de compilation des shaders de bêtes (le rendu headless ne les compile pas).
## À lancer en fenêtre via `tools/godot_bg.sh --path game --script res://tests/as8c_shader_probe.gd`
## et à lire dans le journal : aucune ligne « SHADER ERROR » / « Parse Error » attendue, `PROBE DONE`
## en fin de sortie. Place une charrette, un bœuf et un cheval de camp devant une caméra pendant
## quelques images.

var _frames := 0


func _init() -> void:
	var world := Node3D.new()
	root.add_child(world)
	var cam := Camera3D.new()
	world.add_child(cam)
	cam.look_at_from_position(Vector3(0, 3, 9), Vector3(0, 1, 0))
	var light := DirectionalLight3D.new()
	world.add_child(light)
	light.rotation_degrees = Vector3(-50, 30, 0)
	var x := -4.0
	for role in ["stone_cart", "ox", "sheep"]:
		var mesh := FolkModels.prop_mesh(role)
		if mesh == null:
			print("PROBE missing ", role)
			continue
		var mm := MultiMesh.new()
		mm.transform_format = MultiMesh.TRANSFORM_3D
		mm.use_custom_data = true
		mm.mesh = mesh
		mm.instance_count = 1
		mm.set_instance_transform(0, Transform3D(Basis(), Vector3(x, 0, 0)))
		mm.set_instance_custom_data(0, Color(8.0, 1.2, 0.0, 0.0))
		var mmi := MultiMeshInstance3D.new()
		mmi.multimesh = mm
		world.add_child(mmi)
		x += 4.0
	var parent := Node3D.new()
	world.add_child(parent)
	parent.position = Vector3(0, 0, -4)
	AnimalMotion.build_camp_horses(parent, {"horse_0": [Transform3D.IDENTITY]}, 0.0)


func _process(_delta: float) -> bool:
	_frames += 1
	if _frames == 12:
		print("PROBE DONE")
		quit(0)
	return false
