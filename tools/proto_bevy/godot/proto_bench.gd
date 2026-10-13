extends SceneTree
## Engine comparison prototype, Godot client. Implements tools/proto_bevy/CONTRACT.md.
## Run (background window, no focus steal), from the repository root:
##   tools/godot_bg.sh --path tools/proto_bevy/godot --script res://proto_bench.gd -- \
##       --metrics /abs/out.csv --shot /abs/out.png --at 10 --dump-state /abs/out.json \
##       --set quality=best --set vegetation.grass.fraction=0.5
## Output paths must be absolute (`open` starts the process from "/").
##
## Data: scene.json and the .f32 files are copies in res://assets/ (gitignored); every path in
## scene.json is resolved by its basename inside res://assets/.

const WALK_SHADER := """
shader_type spatial;
render_mode cull_back;

uniform vec4 albedo : source_color = vec4(1.0);
uniform float roughness_value = 0.8;
uniform float metallic_value = 0.0;
uniform bool use_albedo_tex = false;
uniform bool use_normal_tex = false;
uniform bool use_rough_tex = false;
uniform sampler2D albedo_tex : source_color, filter_linear_mipmap_anisotropic, repeat_enable;
uniform sampler2D normal_tex : hint_normal, filter_linear_mipmap_anisotropic, repeat_enable;
uniform sampler2D rough_tex : hint_roughness_g, filter_linear_mipmap_anisotropic, repeat_enable;
uniform float bob_m = 0.06;
uniform float sway_deg = 3.0;
uniform float walk_hz = 1.8;

void vertex() {
	// Contract: phase = fract(sin(id * 12.9898) * 43758.5453) * 2pi, id = global army index.
	float id = INSTANCE_CUSTOM.x;
	float phase = fract(sin(id * 12.9898) * 43758.5453) * 6.28318530718;
	float s = sin(6.28318530718 * walk_hz * TIME + phase);
	float a = radians(sway_deg) * s;
	float c = cos(a);
	float sn = sin(a);
	// Rotation about the local Z axis, pivot at the feet (local origin), then bob along Y.
	mat2 rot = mat2(vec2(c, sn), vec2(-sn, c));
	VERTEX.xy = rot * VERTEX.xy;
	NORMAL.xy = rot * NORMAL.xy;
	VERTEX.y += bob_m * abs(s);
}

void fragment() {
	vec4 base = albedo;
	if (use_albedo_tex) {
		base *= texture(albedo_tex, UV);
	}
	ALBEDO = base.rgb;
	METALLIC = metallic_value;
	ROUGHNESS = roughness_value;
	if (use_rough_tex) {
		ROUGHNESS *= texture(rough_tex, UV).g;
	}
	if (use_normal_tex) {
		NORMAL_MAP = texture(normal_tex, UV).rgb;
	}
}
"""

var scene: Dictionary
var args := {}
var sets: Array = []
var world: Node3D
var camera: Camera3D
var path_time := 0.0
var frame := 0
var warm_frames := 120
var duration_s := 20.0
var quality := "comparable"
var counts := {"placements": 0, "army": 0, "grass": 0, "firs": 0}
var metrics_file: FileAccess = null
var metrics_rows := 0
var frame_times: Array = []
var shot_taken := false
var dump_done := false
var metrics_done := false
var dump_frame := 0
var last_us := 0
var _walk_shader: Shader
var _scene_cache := {}


func _initialize() -> void:
	_parse_args()
	var raw: Variant = JSON.parse_string(FileAccess.get_file_as_string("res://assets/scene.json"))
	scene = raw
	for entry in sets:
		_apply_set(entry)
	quality = str(scene.get("quality", "comparable"))
	warm_frames = int(scene["bench"]["warmup_frames"])
	duration_s = float(scene["bench"]["duration_s"])
	var res: Array = scene["bench"]["resolution"]
	DisplayServer.window_set_size(Vector2i(int(res[0]), int(res[1])))
	if not bool(scene["bench"]["vsync"]):
		DisplayServer.window_set_vsync_mode(DisplayServer.VSYNC_DISABLED)
	Engine.max_fps = 0
	RenderingServer.viewport_set_measure_render_time(root.get_viewport_rid(), true)

	world = Node3D.new()
	root.add_child(world)
	_build_terrain()
	_build_placements()
	_build_army()
	_build_vegetation("grass")
	_build_vegetation("firs")
	_build_camera()
	_build_sun_and_environment()
	_build_ui()
	_apply_quality()
	if args.has("metrics"):
		metrics_file = FileAccess.open(args["metrics"], FileAccess.WRITE)
		metrics_file.store_line("frame,t_s,frame_ms,cpu_ms,gpu_ms,draw_calls")
	print("PROTO godot scene ready quality=", quality, " counts=", counts, " vsync=", DisplayServer.window_get_vsync_mode(), " size=", DisplayServer.window_get_size())


# --- command line ---------------------------------------------------------------------------

func _parse_args() -> void:
	var user := OS.get_cmdline_user_args()
	var i := 0
	while i < user.size():
		var key := user[i]
		if key == "--set" and i + 1 < user.size():
			sets.append(user[i + 1])
			i += 2
		elif key.begins_with("--") and i + 1 < user.size():
			args[key.substr(2)] = user[i + 1]
			i += 2
		else:
			i += 1


func _apply_set(entry: String) -> void:
	var eq := entry.find("=")
	if eq < 0:
		return
	var path := entry.substr(0, eq).split(".")
	var text := entry.substr(eq + 1)
	var json := JSON.new()
	var value: Variant = json.data if json.parse(text) == OK else text
	var node: Variant = scene
	for k in range(path.size() - 1):
		node = _child(node, path[k])
	var last := path[path.size() - 1]
	if node is Array:
		node[int(last)] = value
	else:
		node[last] = value


func _child(node: Variant, key: String) -> Variant:
	if node is Array:
		return node[int(key)]
	if not node.has(key):
		node[key] = {}
	return node[key]


# --- assets ---------------------------------------------------------------------------------

func _glb(asset: String) -> Node3D:
	var path: String = "res://assets/" + str(scene["assets"][asset]).get_file()
	if not _scene_cache.has(asset):
		_scene_cache[asset] = load(path)
	return (_scene_cache[asset] as PackedScene).instantiate()


## Returns [mesh, transform of the node relative to the glTF root].
func _node_mesh(asset: String, node_name: String) -> Array:
	var holder := _glb(asset)
	var found: MeshInstance3D = null
	if node_name == "":
		found = _first_mesh(holder)
	else:
		found = holder.find_child(node_name, true, false) as MeshInstance3D
	assert(found != null, "mesh node %s missing in %s" % [node_name, asset])
	var xform := Transform3D.IDENTITY
	var n: Node = found
	while n != null and n != holder:
		xform = (n as Node3D).transform * xform
		n = n.get_parent()
	var mesh := found.mesh
	holder.free()
	return [mesh, xform]


func _first_mesh(n: Node) -> MeshInstance3D:
	if n is MeshInstance3D:
		return n
	for c in n.get_children():
		var r := _first_mesh(c)
		if r != null:
			return r
	return null


func _read_f32(file_name: String) -> PackedFloat32Array:
	var bytes := FileAccess.get_file_as_bytes("res://assets/" + file_name.get_file())
	return bytes.to_float32_array()


# --- terrain --------------------------------------------------------------------------------

func _build_terrain() -> void:
	var hm: Dictionary = scene["heightmap"]
	var grid := int(hm["grid"])
	var size := float(hm["size_m"])
	var tile := float(hm["uv_tile_m"])
	var h := _read_f32(str(hm["file"]))
	assert(h.size() == grid * grid, "heightmap size mismatch")
	var step := size / float(grid - 1)
	var verts := PackedVector3Array()
	var normals := PackedVector3Array()
	var uvs := PackedVector2Array()
	var tangents := PackedFloat32Array()
	verts.resize(grid * grid)
	normals.resize(grid * grid)
	uvs.resize(grid * grid)
	tangents.resize(grid * grid * 4)
	for row in grid:
		var z := -size / 2.0 + row * step
		var r0 := maxi(row - 1, 0)
		var r1 := mini(row + 1, grid - 1)
		for col in grid:
			var x := -size / 2.0 + col * step
			var idx := row * grid + col
			var y := h[idx]
			var c0 := maxi(col - 1, 0)
			var c1 := mini(col + 1, grid - 1)
			var dhdx := (h[row * grid + c1] - h[row * grid + c0]) / ((c1 - c0) * step)
			var dhdz := (h[r1 * grid + col] - h[r0 * grid + col]) / ((r1 - r0) * step)
			verts[idx] = Vector3(x, y, z)
			normals[idx] = Vector3(-dhdx, 1.0, -dhdz).normalized()
			uvs[idx] = Vector2(x / tile, z / tile)
			var t := Vector3(1.0, dhdx, 0.0).normalized()
			tangents[idx * 4] = t.x
			tangents[idx * 4 + 1] = t.y
			tangents[idx * 4 + 2] = t.z
			tangents[idx * 4 + 3] = 1.0
	var cells := grid - 1
	var indices := PackedInt32Array()
	indices.resize(cells * cells * 6)
	var k := 0
	for row in cells:
		for col in cells:
			var a := row * grid + col
			var b := a + 1
			var c := a + grid
			var d := c + 1
			# Counter-clockwise seen from above (+Y): a, c, b and b, c, d.
			indices[k] = a
			indices[k + 1] = c
			indices[k + 2] = b
			indices[k + 3] = b
			indices[k + 4] = c
			indices[k + 5] = d
			k += 6
	var arrays := []
	arrays.resize(Mesh.ARRAY_MAX)
	arrays[Mesh.ARRAY_VERTEX] = verts
	arrays[Mesh.ARRAY_NORMAL] = normals
	arrays[Mesh.ARRAY_TANGENT] = tangents
	arrays[Mesh.ARRAY_TEX_UV] = uvs
	arrays[Mesh.ARRAY_INDEX] = indices
	var mesh := ArrayMesh.new()
	mesh.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES, arrays)
	var tex := str(hm["texture"])
	var material := StandardMaterial3D.new()
	material.albedo_texture = load("res://assets/ground_%s_Diffuse.jpg" % tex)
	material.normal_enabled = true
	material.normal_texture = load("res://assets/ground_%s_nor_gl.jpg" % tex)
	material.roughness_texture = load("res://assets/ground_%s_Rough.jpg" % tex)
	material.roughness_texture_channel = BaseMaterial3D.TEXTURE_CHANNEL_GREEN
	material.roughness = 1.0
	material.texture_filter = BaseMaterial3D.TEXTURE_FILTER_LINEAR_WITH_MIPMAPS_ANISOTROPIC
	mesh.surface_set_material(0, material)
	var instance := MeshInstance3D.new()
	instance.mesh = mesh
	instance.name = "Terrain"
	world.add_child(instance)


# --- placements -----------------------------------------------------------------------------

func _build_placements() -> void:
	for placement in scene["placements"]:
		var node := _glb(placement["asset"])
		var part: String = placement["node"]
		if part != "":
			var child := node.find_child(part, true, false) as Node3D
			assert(child != null, "node %s missing" % part)
			child.get_parent().remove_child(child)
			node.free()
			node = child
			node.transform = Transform3D.IDENTITY
		var p: Array = placement["pos"]
		node.position = Vector3(p[0], p[1], p[2])
		node.rotation_degrees.y = placement["yaw"]
		node.scale *= float(placement["scale"])
		world.add_child(node)
		counts["placements"] += 1


# --- instancing -----------------------------------------------------------------------------

## Writes one instance transform (3x4 row-major, as MultiMesh.buffer expects) at `offset`.
func _put_transform(buf: PackedFloat32Array, offset: int, xform: Transform3D) -> void:
	var b := xform.basis
	buf[offset] = b.x.x
	buf[offset + 1] = b.y.x
	buf[offset + 2] = b.z.x
	buf[offset + 3] = xform.origin.x
	buf[offset + 4] = b.x.y
	buf[offset + 5] = b.y.y
	buf[offset + 6] = b.z.y
	buf[offset + 7] = xform.origin.y
	buf[offset + 8] = b.x.z
	buf[offset + 9] = b.y.z
	buf[offset + 10] = b.z.z
	buf[offset + 11] = xform.origin.z


func _record_transform(r: PackedFloat32Array, base: int, node_xform: Transform3D) -> Transform3D:
	var basis := Basis(Vector3.UP, deg_to_rad(r[base + 3])).scaled(Vector3.ONE * r[base + 4])
	return Transform3D(basis, Vector3(r[base], r[base + 1], r[base + 2])) * node_xform


func _build_army() -> void:
	var army: Dictionary = scene["army"]
	var records := _read_f32(str(army["file"]))
	var total := mini(int(army["count"]), records.size() / 6)
	var models: Array = army["models"]
	var walk: Dictionary = army["walk"]
	_walk_shader = Shader.new()
	_walk_shader.code = WALK_SHADER
	# Group instances by model, keeping the global index for the walk phase.
	var groups: Array = []
	for m in models.size():
		groups.append([])
	for i in total:
		groups[clampi(int(records[i * 6 + 5]), 0, models.size() - 1)].append(i)
	for m in models.size():
		var members: Array = groups[m]
		if members.is_empty():
			continue
		var pair := _node_mesh(str(models[m]), "")
		var mesh: Mesh = pair[0]
		var node_xform: Transform3D = pair[1]
		var walking_mesh: Mesh = mesh.duplicate()
		for s in walking_mesh.get_surface_count():
			walking_mesh.surface_set_material(s, _walk_material(mesh.surface_get_material(s), walk))
		var mm := MultiMesh.new()
		mm.transform_format = MultiMesh.TRANSFORM_3D
		mm.use_custom_data = true
		mm.mesh = walking_mesh
		mm.instance_count = members.size()
		var buf := PackedFloat32Array()
		buf.resize(members.size() * 16)
		for j in members.size():
			var gi: int = members[j]
			_put_transform(buf, j * 16, _record_transform(records, gi * 6, node_xform))
			buf[j * 16 + 12] = float(gi)
		mm.buffer = buf
		var inst := MultiMeshInstance3D.new()
		inst.name = "Army_%s" % models[m]
		inst.multimesh = mm
		world.add_child(inst)
	counts["army"] = total


func _walk_material(source: Material, walk: Dictionary) -> ShaderMaterial:
	var material := ShaderMaterial.new()
	material.shader = _walk_shader
	material.set_shader_parameter("bob_m", float(walk["bob_m"]))
	material.set_shader_parameter("sway_deg", float(walk["sway_deg"]))
	material.set_shader_parameter("walk_hz", float(walk["hz"]))
	if source is BaseMaterial3D:
		var std := source as BaseMaterial3D
		material.set_shader_parameter("albedo", std.albedo_color)
		material.set_shader_parameter("roughness_value", std.roughness)
		material.set_shader_parameter("metallic_value", std.metallic)
		if std.albedo_texture != null:
			material.set_shader_parameter("use_albedo_tex", true)
			material.set_shader_parameter("albedo_tex", std.albedo_texture)
		if std.normal_enabled and std.normal_texture != null:
			material.set_shader_parameter("use_normal_tex", true)
			material.set_shader_parameter("normal_tex", std.normal_texture)
		if std.roughness_texture != null:
			material.set_shader_parameter("use_rough_tex", true)
			material.set_shader_parameter("rough_tex", std.roughness_texture)
	return material


func _build_vegetation(kind: String) -> void:
	var spec: Dictionary = scene["vegetation"][kind]
	var records := _read_f32(str(spec["file"]))
	var total := mini(int(spec["count"]), records.size() / 6)
	var shown := int(floor(total * float(spec["fraction"])))
	counts[kind] = shown
	if shown <= 0:
		return
	var pair := _node_mesh(str(spec["asset"]), str(spec["node"]))
	var mm := MultiMesh.new()
	mm.transform_format = MultiMesh.TRANSFORM_3D
	mm.mesh = pair[0]
	mm.instance_count = total
	var buf := PackedFloat32Array()
	buf.resize(total * 12)
	for i in total:
		_put_transform(buf, i * 12, _record_transform(records, i * 6, pair[1]))
	mm.buffer = buf
	mm.visible_instance_count = shown
	var inst := MultiMeshInstance3D.new()
	inst.name = kind.capitalize()
	inst.multimesh = mm
	if not bool(spec.get("cast_shadows", true)):
		inst.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	world.add_child(inst)


# --- camera, sun, environment ---------------------------------------------------------------

func _build_camera() -> void:
	camera = Camera3D.new()
	world.add_child(camera)
	camera.fov = float(scene["camera"]["fov_y"])
	camera.far = 2000.0
	camera.make_current()
	_update_camera(0.0)


func _update_camera(t: float) -> void:
	var keys: Array = scene["camera_path"]
	var a: Dictionary = keys[0]
	var b: Dictionary = keys[keys.size() - 1]
	var u := 0.0
	if t >= float(keys[keys.size() - 1]["t"]):
		a = b
	elif t > float(keys[0]["t"]):
		for i in range(keys.size() - 1):
			if t <= float(keys[i + 1]["t"]):
				a = keys[i]
				b = keys[i + 1]
				u = (t - float(a["t"])) / (float(b["t"]) - float(a["t"]))
				break
	var pa: Array = a["pos"]
	var pb: Array = b["pos"]
	var la: Array = a["look_at"]
	var lb: Array = b["look_at"]
	var pos := Vector3(pa[0], pa[1], pa[2]).lerp(Vector3(pb[0], pb[1], pb[2]), u)
	var look := Vector3(la[0], la[1], la[2]).lerp(Vector3(lb[0], lb[1], lb[2]), u)
	camera.look_at_from_position(pos, look)


func _build_sun_and_environment() -> void:
	var sun_spec: Dictionary = scene["sun"]
	var sun := DirectionalLight3D.new()
	world.add_child(sun)
	# Azimuth measured from -Z (camera forward) turning toward +X, elevation above the horizon.
	sun.rotation_degrees = Vector3(-float(sun_spec["elevation"]), 180.0 - float(sun_spec["azimuth"]), 0)
	var c: Array = sun_spec["color"]
	sun.light_color = Color(c[0], c[1], c[2])
	sun.light_energy = 2.4
	sun.shadow_enabled = true
	sun.directional_shadow_mode = DirectionalLight3D.SHADOW_PARALLEL_4_SPLITS
	sun.directional_shadow_max_distance = 150.0
	sun.directional_shadow_blend_splits = true

	var fog: Dictionary = scene["fog"]
	var f: Array = fog["color"]
	var fog_color := Color(f[0], f[1], f[2])
	var env := Environment.new()
	env.background_mode = Environment.BG_SKY
	var sky := Sky.new()
	env.ambient_light_source = Environment.AMBIENT_SOURCE_SKY
	env.reflected_light_source = Environment.REFLECTION_SOURCE_SKY
	env.tonemap_mode = Environment.TONE_MAPPER_AGX
	env.tonemap_exposure = 1.1
	env.glow_enabled = true
	env.ssao_enabled = true
	if quality == "best":
		sun.light_angular_distance = 0.6
		sun.light_volumetric_fog_energy = 1.5
		var phys := PhysicalSkyMaterial.new()
		phys.turbidity = 8.0
		phys.mie_coefficient = 0.008
		phys.energy_multiplier = 6.0
		sky.sky_material = phys
		env.sdfgi_enabled = true
		env.sdfgi_use_occlusion = true
		env.sdfgi_cascades = 6
		env.sdfgi_min_cell_size = 0.1
		env.ssil_enabled = true
		env.glow_intensity = 0.4
		env.volumetric_fog_enabled = true
		env.volumetric_fog_density = float(fog["density"])
		env.volumetric_fog_albedo = fog_color
		env.volumetric_fog_anisotropy = 0.6
		env.adjustment_enabled = true
		env.adjustment_saturation = 1.05
	else:
		sky.sky_material = ProceduralSkyMaterial.new()
		env.glow_intensity = 0.25
		env.glow_bloom = 0.02
		env.fog_enabled = true
		env.fog_mode = Environment.FOG_MODE_EXPONENTIAL
		env.fog_density = float(fog["density"])
		env.fog_light_color = fog_color
	env.sky = sky
	var world_env := WorldEnvironment.new()
	world_env.environment = env
	world.add_child(world_env)


func _apply_quality() -> void:
	# project.godot is tuned for `best` (2x supersampling, MSAA, TAA); override per mode here.
	if quality == "best":
		# 2x supersampling (project.godot) makes the close-up camera exceed the Metal fence timeout
		# (black frames); render at native resolution instead.
		root.scaling_3d_scale = 1.0
		return
	root.scaling_3d_scale = 1.0
	root.msaa_3d = Viewport.MSAA_DISABLED
	root.use_taa = false
	root.screen_space_aa = Viewport.SCREEN_SPACE_AA_FXAA


# --- UI -------------------------------------------------------------------------------------

func _build_ui() -> void:
	var spec: Dictionary = scene["ui_panel"]
	var layer := CanvasLayer.new()
	root.add_child(layer)
	var holder := Control.new()
	holder.set_anchors_preset(Control.PRESET_FULL_RECT)
	holder.mouse_filter = Control.MOUSE_FILTER_IGNORE
	layer.add_child(holder)
	var panel := PanelContainer.new()
	var style := StyleBoxFlat.new()
	style.bg_color = Color("e8d9b5")
	style.border_color = Color("6b4a2b")
	style.set_border_width_all(3)
	style.set_corner_radius_all(4)
	style.set_content_margin_all(14)
	panel.add_theme_stylebox_override("panel", style)
	panel.set_anchors_preset(Control.PRESET_TOP_RIGHT)
	panel.offset_left = -340
	panel.offset_right = -20
	panel.offset_top = 20
	panel.grow_horizontal = Control.GROW_DIRECTION_BEGIN
	holder.add_child(panel)
	var box := VBoxContainer.new()
	box.add_theme_constant_override("separation", 10)
	panel.add_child(box)
	var title := Label.new()
	title.text = str(spec["title"])
	title.add_theme_font_size_override("font_size", 24)
	title.add_theme_color_override("font_color", Color("3b2a18"))
	box.add_child(title)
	var body := Label.new()
	body.text = str(spec["body"])
	body.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	body.custom_minimum_size = Vector2(290, 0)
	body.add_theme_color_override("font_color", Color("3b2a18"))
	box.add_child(body)
	var button := Button.new()
	button.text = str(spec["button"])
	button.tooltip_text = str(spec["tooltip"])
	box.add_child(button)


# --- frame loop -----------------------------------------------------------------------------

func _process(_smoothed_delta: float) -> bool:
	# Godot's delta smoothing snaps `delta` to display-refresh multiples; time the frames raw.
	var now_us := Time.get_ticks_usec()
	var delta := (now_us - last_us) / 1.0e6 if last_us > 0 else _smoothed_delta
	last_us = now_us
	frame += 1
	if frame <= warm_frames:
		return false
	path_time += delta
	_update_camera(path_time)
	var frame_ms := delta * 1000.0
	var rid := root.get_viewport_rid()
	if metrics_file != null and not metrics_done:
		var cpu := RenderingServer.viewport_get_measured_render_time_cpu(rid)
		var gpu := RenderingServer.viewport_get_measured_render_time_gpu(rid)
		var draws := Performance.get_monitor(Performance.RENDER_TOTAL_DRAW_CALLS_IN_FRAME)
		metrics_file.store_line("%d,%.4f,%.4f,%s,%s,%d" % [
			frame - warm_frames, path_time, frame_ms,
			("%.4f" % cpu) if cpu > 0.0 else "", ("%.4f" % gpu) if gpu > 0.0 else "", int(draws)])
		metrics_rows += 1
		if path_time >= duration_s:
			metrics_done = true
			metrics_file.close()
			print("PROTO godot metrics rows=", metrics_rows)
	frame_times.append(frame_ms)
	if args.has("dump-state") and not dump_done:
		dump_frame += 1
		if dump_frame >= 30:
			_dump_state(args["dump-state"])
			dump_done = true
	if args.has("shot") and not shot_taken and path_time >= float(args.get("at", "0")):
		root.get_viewport().get_texture().get_image().save_png(args["shot"])
		shot_taken = true
		print("PROTO godot shot saved ", args["shot"])
	var wants := args.has("metrics") or args.has("dump-state") or args.has("shot")
	var waiting := (args.has("metrics") and not metrics_done) \
		or (args.has("dump-state") and not dump_done) \
		or (args.has("shot") and not shot_taken)
	return wants and not waiting


func _dump_state(file_name: String) -> void:
	var recent := frame_times.slice(maxi(frame_times.size() - 30, 0))
	var sum := 0.0
	for v in recent:
		sum += v
	var state := {
		"engine": "godot",
		"quality": quality,
		"counts": {
			"placements": counts["placements"], "army": counts["army"],
			"grass": counts["grass"], "firs": counts["firs"],
			"entities_or_nodes": int(Performance.get_monitor(Performance.OBJECT_NODE_COUNT)),
			"draw_calls": int(Performance.get_monitor(Performance.RENDER_TOTAL_DRAW_CALLS_IN_FRAME)),
		},
		"effective": scene,
		"frame_ms_avg": sum / maxf(recent.size(), 1),
	}
	var f := FileAccess.open(file_name, FileAccess.WRITE)
	f.store_string(JSON.stringify(state, "  "))
	f.close()
	print("PROTO godot state saved ", file_name)
