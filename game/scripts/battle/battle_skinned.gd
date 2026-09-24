class_name BattleSkinned
extends RefCounted

## Figurines skinnées (lot V2) : chargement des maillages et des textures d'os cuits par
## `tools/blender_scripts/battle_skinned.py` (`assets/models/battle_skinned/`), matériau
## `battle_soldier_skinned.gdshader` et correspondance état du régiment → clips.
## Repli : sans manifeste, ou avec `--rigid-figures` / `--legacy-figures` après `--`, les
## figurines à membres rigides (lots B1/B4, `BattleMeshes`) restent utilisées.

const DIR := "res://assets/models/battle_skinned/"
const SHADER := preload("res://shaders/battle_soldier_skinned.gdshader")
const MAX_CLIPS := 48
## Modes du shader.
const M_LOOP := 0
const M_CYCLE := 1
const M_VOLLEY := 2
const M_CUSTOM := 3

static var _manifest: Dictionary = {}
static var _loaded: bool = false
static var _meshes: Dictionary = {}
static var _textures: Dictionary = {}
static var _configs: Dictionary = {}  # "kind/variant/state" -> configuration (chaque image)


static func manifest() -> Dictionary:
	if not _loaded:
		_loaded = true
		var text := FileAccess.get_file_as_string(DIR + "manifest.json")
		var parsed = JSON.parse_string(text) if text != "" else null
		_manifest = parsed if parsed is Dictionary else {}
	return _manifest


static func enabled() -> bool:
	var args := OS.get_cmdline_user_args()
	return not args.has("--rigid-figures") and not args.has("--legacy-figures")


static func figure_name(kind: String, variant: int) -> String:
	return "%s_%d" % [kind, variant]


## Vrai si la figurine skinnée de `kind`/`variant` existe (et que le rendu skinné est actif).
static func has_figure(kind: String, variant: int) -> bool:
	if not enabled():
		return false
	var figures: Dictionary = manifest().get("figures", {})
	return figures.has(figure_name(kind, variant))


static func figure(kind: String, variant: int) -> Dictionary:
	return manifest().get("figures", {}).get(figure_name(kind, variant), {})


static func rig(kind: String, variant: int) -> Dictionary:
	return manifest().get("rigs", {}).get(str(figure(kind, variant).get("rig", "")), {})


## Maillage de la figurine au niveau `level` (0 complet, 1 moyen, 2 lointain/ombres).
static func mesh(kind: String, variant: int, level: int) -> ArrayMesh:
	var fig := figure(kind, variant)
	var lods: Array = fig.get("lods", [])
	if lods.is_empty():
		return null
	var file: String = lods[clampi(level, 0, lods.size() - 1)]
	if _meshes.has(file):
		return _meshes[file]
	var mesh := _load_mesh(DIR + file, str(fig.get("rig", "")) != "human")
	_meshes[file] = mesh
	return mesh


## Maillage binaire `CAM1` (zlib) : positions, normales, couleurs (rgb + code), UV, os, poids,
## masques de variante, indices.
static func _load_mesh(path: String, large: bool) -> ArrayMesh:
	var bytes := FileAccess.get_file_as_bytes(path)
	if bytes.size() < 16 or bytes.slice(0, 4).get_string_from_ascii() != "CAM1":
		push_warning("BattleSkinned: bad mesh file %s" % path)
		return null
	var n := bytes.decode_u32(4)
	var m := bytes.decode_u32(8)
	var raw := bytes.slice(16).decompress(bytes.decode_u32(12), FileAccess.COMPRESSION_DEFLATE)
	var floats := raw.slice(0, n * 21 * 4).to_float32_array()
	var indices := raw.slice(n * 21 * 4, n * 21 * 4 + m * 4).to_int32_array()
	var verts := PackedVector3Array()
	var normals := PackedVector3Array()
	var colors := PackedColorArray()
	var uvs := PackedVector2Array()
	var uv2 := PackedVector2Array()
	verts.resize(n)
	normals.resize(n)
	colors.resize(n)
	uvs.resize(n)
	uv2.resize(n)
	var o_n := n * 3
	var o_c := n * 6
	var o_uv := n * 10
	var o_b := n * 12
	var o_w := n * 16
	var o_m := n * 20
	for i in n:
		verts[i] = Vector3(floats[i * 3], floats[i * 3 + 1], floats[i * 3 + 2])
		normals[i] = Vector3(floats[o_n + i * 3], floats[o_n + i * 3 + 1], floats[o_n + i * 3 + 2])
		# Couleurs stockées en 8 bits : le code matière passe en alpha / 16.
		colors[i] = Color(floats[o_c + i * 4], floats[o_c + i * 4 + 1], floats[o_c + i * 4 + 2], floats[o_c + i * 4 + 3] / 16.0)
		uvs[i] = Vector2(floats[o_uv + i * 2], floats[o_uv + i * 2 + 1])
		uv2[i] = Vector2(floats[o_m + i], 0.0)
	var arrays := []
	arrays.resize(Mesh.ARRAY_MAX)
	arrays[Mesh.ARRAY_VERTEX] = verts
	arrays[Mesh.ARRAY_NORMAL] = normals
	arrays[Mesh.ARRAY_COLOR] = colors
	arrays[Mesh.ARRAY_TEX_UV] = uvs
	arrays[Mesh.ARRAY_TEX_UV2] = uv2
	arrays[Mesh.ARRAY_CUSTOM0] = floats.slice(o_b, o_b + n * 4)
	arrays[Mesh.ARRAY_CUSTOM1] = floats.slice(o_w, o_w + n * 4)
	arrays[Mesh.ARRAY_INDEX] = indices
	var mesh := ArrayMesh.new()
	var flags := (Mesh.ARRAY_CUSTOM_RGBA_FLOAT << Mesh.ARRAY_FORMAT_CUSTOM0_SHIFT) | (Mesh.ARRAY_CUSTOM_RGBA_FLOAT << Mesh.ARRAY_FORMAT_CUSTOM1_SHIFT)
	mesh.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES, arrays, [], {}, flags)
	# Boîte englobante élargie : les animations (coups, chutes) sortent de la pose de repos.
	mesh.custom_aabb = AABB(Vector3(-2.5, -0.5, -3.0), Vector3(5.0, 4.5, 6.5)) if large else AABB(Vector3(-1.6, -0.3, -1.8), Vector3(3.2, 2.8, 3.8))
	return mesh


## Texture d'os d'un rig (`CAB1` : os, images, puis zlib de 3 texels RGBA32F par os et image).
static func bone_texture(rig_name: String) -> ImageTexture:
	if _textures.has(rig_name):
		return _textures[rig_name]
	var entry: Dictionary = manifest().get("rigs", {}).get(rig_name, {})
	var bytes := FileAccess.get_file_as_bytes(DIR + str(entry.get("texture", "")))
	var tex: ImageTexture = null
	if bytes.size() > 12 and bytes.slice(0, 4).get_string_from_ascii() == "CAB1":
		var bones := bytes.decode_u32(4)
		var frames := bytes.decode_u32(8)
		var raw := bytes.slice(12).decompress(bones * 3 * frames * 16, FileAccess.COMPRESSION_DEFLATE)
		var image := Image.create_from_data(bones * 3, frames, false, Image.FORMAT_RGBAF, raw)
		tex = ImageTexture.create_from_image(image)
	else:
		push_warning("BattleSkinned: missing bone texture for rig %s" % rig_name)
	_textures[rig_name] = tex
	return tex


static func clip_index(rig_entry: Dictionary, clip: String) -> int:
	var names: Array = rig_entry.get("clips", {}).keys()
	names.sort()
	return maxi(names.find(clip), 0)


## Uniformes propres au rig et à la figurine (texture, table des clips, variantes).
static func setup_material(mat: ShaderMaterial, kind: String, variant: int) -> void:
	var rig_entry := rig(kind, variant)
	mat.set_shader_parameter("bone_tex", bone_texture(str(figure(kind, variant).get("rig", ""))))
	mat.set_shader_parameter("bone_fps", float(rig_entry.get("fps", 24)))
	var table: Array[Vector4] = []
	var clips: Dictionary = rig_entry.get("clips", {})
	var names: Array = clips.keys()
	names.sort()
	for name in names:
		var c: Dictionary = clips[name]
		table.append(Vector4(float(c["start"]), float(c["frames"]), 1.0 if bool(c["loop"]) else 0.0, 0.0))
	while table.size() < MAX_CLIPS:
		table.append(Vector4(0, 1, 1, 0))
	mat.set_shader_parameter("clips", table)
	mat.set_shader_parameter("variant_count", int(figure(kind, variant).get("variants", 1)))
	mat.set_shader_parameter("size_jitter", 0.0 if kind == "cavalry" else 0.05)


## Configuration d'animation {set: [clips], mode, speed, cycle} d'un régiment dans l'état
## `state` (clé de la simulation : idle, marching, charging, melee, shooting, routing,
## climbing ; `running` = marche au pas de course).
static func state_config(kind: String, variant: int, state: String, running: bool) -> Dictionary:
	var key := state
	if state == "marching" and running:
		key = "running"
	var cache_key := "%s/%d/%s" % [kind, variant, key]
	if _configs.has(cache_key):
		return _configs[cache_key]
	var sets: Dictionary = STYLES.get(_style(kind, variant), STYLES["sword"])
	if not sets.has(key):
		key = "idle"
	var entry: Dictionary = sets[key]
	var rig_entry := rig(kind, variant)
	var ids: Array[int] = []
	for c in entry["set"]:
		ids.append(clip_index(rig_entry, str(c)))
	var config := {"key": cache_key, "set": ids, "mode": int(entry.get("mode", M_LOOP)), "speed": float(entry.get("speed", 1.0)), "cycle": float(entry.get("cycle", 1.5)), "release": float(entry.get("release", 1.0))}
	_configs[cache_key] = config
	return config


## Clips de mort (mode CUSTOM des cadavres, INSTANCE_CUSTOM.y = indice dans ce jeu).
static func death_config(kind: String, variant: int) -> Dictionary:
	var rig_entry := rig(kind, variant)
	var names: Array = DEATHS_CAVALRY if kind == "cavalry" else DEATHS_FOOT
	var ids: Array[int] = []
	for c in names:
		if rig_entry.get("clips", {}).has(c):
			ids.append(clip_index(rig_entry, str(c)))
	if ids.is_empty():
		ids.append(0)
	return {"key": "%s/%d/dead" % [kind, variant], "set": ids, "mode": M_CUSTOM, "speed": 1.0, "cycle": 1.0, "release": 1.0}


static func _style(kind: String, variant: int) -> String:
	match kind:
		"infantry":
			return "sword" if variant == 0 else "pike" if variant == 1 else "militia"
		"archer":
			return "bow" if variant == 0 else "crossbow"
		"cavalry":
			return "horse_bow" if variant == 2 else "lance"
	return "sword"


const DEATHS_FOOT := ["death", "death_m", "death_back", "death_knees"]
const DEATHS_CAVALRY := ["c_death", "c_death_m"]

## Jeux de clips par style et par état.
const STYLES := {
	"sword": {
		"idle": {"set": ["guard", "idle"]},
		"marching": {"set": ["walk"]},
		"running": {"set": ["run"]},
		"charging": {"set": ["run"], "speed": 1.05},
		"melee": {"set": ["slash", "thrust", "hit", "guard"], "mode": M_CYCLE, "cycle": 1.3},
		"routing": {"set": ["run"], "speed": 1.1},
		"climbing": {"set": ["run"]},
	},
	"militia": {
		"idle": {"set": ["idle", "guard"]},
		"marching": {"set": ["walk"]},
		"running": {"set": ["run"]},
		"charging": {"set": ["run"]},
		"melee": {"set": ["slash", "thrust", "hit", "guard"], "mode": M_CYCLE, "cycle": 1.5},
		"routing": {"set": ["run"], "speed": 1.15},
	},
	"pike": {
		"idle": {"set": ["pike_idle"]},
		"marching": {"set": ["pike_walk"]},
		"running": {"set": ["run"]},
		"charging": {"set": ["pike_walk"], "speed": 1.4},
		"melee": {"set": ["pike_thrust", "pike_thrust", "pike_idle"], "mode": M_CYCLE, "cycle": 1.2},
		"routing": {"set": ["run"], "speed": 1.1},
	},
	"bow": {
		"idle": {"set": ["bow_idle", "idle"]},
		"marching": {"set": ["walk"]},
		"running": {"set": ["run"]},
		"charging": {"set": ["run"]},
		"shooting": {"set": ["bow_shoot"], "mode": M_VOLLEY, "release": 1.55},
		"melee": {"set": ["slash", "thrust", "guard"], "mode": M_CYCLE, "cycle": 1.5},
		"routing": {"set": ["run"], "speed": 1.15},
	},
	"crossbow": {
		"idle": {"set": ["xbow_idle"]},
		"marching": {"set": ["walk"]},
		"running": {"set": ["run"]},
		"charging": {"set": ["run"]},
		"shooting": {"set": ["xbow_shoot"], "mode": M_VOLLEY, "release": 0.3},
		"melee": {"set": ["slash", "thrust", "guard"], "mode": M_CYCLE, "cycle": 1.5},
		"routing": {"set": ["run"], "speed": 1.15},
	},
	"lance": {
		"idle": {"set": ["c_idle"]},
		"marching": {"set": ["c_walk"]},
		"running": {"set": ["c_gallop"]},
		"charging": {"set": ["c_charge"]},
		"melee": {"set": ["c_thrust", "c_thrust", "c_idle"], "mode": M_CYCLE, "cycle": 1.4},
		"routing": {"set": ["c_gallop"]},
	},
	"horse_bow": {
		"idle": {"set": ["c_bow_idle"]},
		"marching": {"set": ["c_bow_walk"]},
		"running": {"set": ["c_gallop"]},
		"charging": {"set": ["c_gallop"]},
		"shooting": {"set": ["c_bow_shoot"], "mode": M_VOLLEY, "release": 1.55},
		"melee": {"set": ["c_thrust", "c_bow_idle"], "mode": M_CYCLE, "cycle": 1.4},
		"routing": {"set": ["c_gallop"]},
	},
}


## Applique `config` (cf. `state_config`) au matériau ; au changement, l'ancienne
## configuration devient la source du fondu.
static func apply_config(mat: ShaderMaterial, config: Dictionary, anim_time: float) -> void:
	var previous: Dictionary = mat.get_meta("v2_config", {})
	if str(previous.get("key", "")) == str(config["key"]):
		return
	if not previous.is_empty():
		mat.set_shader_parameter("prev_set", _ivec(previous["set"]))
		mat.set_shader_parameter("prev_set_size", (previous["set"] as Array).size())
		mat.set_shader_parameter("prev_mode", int(previous["mode"]))
		mat.set_shader_parameter("prev_speed", float(previous["speed"]))
		mat.set_shader_parameter("blend_since", anim_time)
	mat.set_shader_parameter("clip_set", _ivec(config["set"]))
	mat.set_shader_parameter("clip_set_size", (config["set"] as Array).size())
	mat.set_shader_parameter("clip_mode", int(config["mode"]))
	mat.set_shader_parameter("anim_speed", float(config["speed"]))
	mat.set_shader_parameter("cycle_len", float(config["cycle"]))
	mat.set_shader_parameter("release_at", float(config["release"]))
	mat.set_meta("v2_config", config)


static func _ivec(ids: Array) -> Vector4i:
	var v := Vector4i.ZERO
	for i in mini(ids.size(), 4):
		v[i] = int(ids[i])
	return v
