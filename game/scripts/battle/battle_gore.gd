class_name BattleGore
extends Node3D

## Lot BV2 — gerbes de sang à l'impact et morceaux tranchés (rendu seulement ; qui tombe et qui
## meurt vient du cœur). Deux tampons circulaires en `MultiMesh`, trajectoires balistiques
## calculées dans le shader (`battle_gore.gdshader`) : l'instance porte le point de départ au
## sol et INSTANCE_CUSTOM = (vitesse initiale, instant). Aucun corps physique : coût fixe, rien à
## simuler côté GDScript hors des écritures d'instances au moment des événements.
##
## Réglage « Sang » (`battle/blood` : 0 désactivé, 1 modéré, 2 complet ; `--blood=off|moderate|full`
## après `--` pour les captures) ; intensités et plafonds : `data/fx/battle_gore.json`.

const SETTINGS_FILE := "fx/battle_gore.json"
const SHADER := preload("res://shaders/battle_gore.gdshader")
const LEVEL_NAMES := ["off", "moderate", "full"]
## Morceaux : 0 tête, 1 bras, 2 jambe (maillage et teinte).
const PIECE_KINDS := {"head": 0, "arm_r": 1, "arm_l": 1, "leg_r": 2, "leg_l": 2}

static var _settings: Dictionary = {}
static var _settings_loaded: bool = false

var anim_time: float = 0.0
var camera_pos: Vector3 = Vector3.ZERO
var drops_emitted: int = 0
var pieces_emitted: int = 0

var _level: Dictionary = {}
var _drops: MultiMesh
var _drops_data := PackedFloat32Array()
var _drops_next: int = 0
var _drops_dirty: bool = false
var _drop_material: ShaderMaterial
var _pieces: Array = []  # [{mm, data, next, dirty, material}] par sorte de morceau
var _rng := RandomNumberGenerator.new()


## Lecture de `data/fx/battle_gore.json` (dossier de données du jeu, puis `data/` du dépôt).
static func settings() -> Dictionary:
	if _settings_loaded:
		return _settings
	_settings_loaded = true
	if DataFile.exists(SETTINGS_FILE):
		var parsed: Variant = DataFile.read_json(SETTINGS_FILE)
		if parsed is Dictionary:
			_settings = parsed
			return _settings
	push_warning("BattleGore: %s introuvable, sang et démembrements désactivés" % SETTINGS_FILE)
	return _settings


## Niveau du réglage « Sang » : 0 désactivé, 1 modéré, 2 complet.
static func blood_level() -> int:
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--blood="):
			# Noms (off, moderate, full) ou chiffres (0, 1, 2 : forme du lot BV1).
			var value := arg.get_slice("=", 1)
			var k := LEVEL_NAMES.find(value)
			if k >= 0:
				return k
			if value.is_valid_int():
				return clampi(int(value), 0, 2)
	var tree := Engine.get_main_loop() as SceneTree
	var store: Node = tree.root.get_node_or_null("/root/Settings") if tree != null else null
	if store != null:
		var value: Variant = store.call("get_value", "battle/blood")
		if value != null:
			return clampi(int(value), 0, 2)
	return 1


## Intensités du niveau courant ({stains, sprays, corpse_blood, dismember}).
static func level_settings() -> Dictionary:
	var levels: Dictionary = settings().get("levels", {})
	if OS.get_cmdline_user_args().has("--no-bv2"):
		return levels.get("off", {})
	return levels.get(LEVEL_NAMES[blood_level()], {"stains": 0.0, "sprays": 0.0, "corpse_blood": 0.0, "dismember": false})


func _ready() -> void:
	_rng.seed = 1337
	_level = level_settings()
	var sprays: Dictionary = settings().get("sprays", {})
	var drop_mesh := QuadMesh.new()
	drop_mesh.size = Vector2(1.0, 1.0)
	_drop_material = ShaderMaterial.new()
	_drop_material.shader = SHADER
	_drop_material.set_shader_parameter("mode", 0)
	_drop_material.set_shader_parameter("lifetime", float(sprays.get("lifetime", 1.4)))
	_drops = _make_layer("BloodDrops", drop_mesh, _drop_material, int(sprays.get("max_droplets", 0)))
	_drops_data.resize(_drops.instance_count * 16)
	_drops_data.fill(0.0)
	var severed: Dictionary = settings().get("severed", {})
	var pieces_max := int(severed.get("max_pieces", 0))
	for k in 3:
		var mat := ShaderMaterial.new()
		mat.shader = SHADER
		mat.set_shader_parameter("mode", 1)
		mat.set_shader_parameter("piece", k)
		mat.set_shader_parameter("rest_seconds", float(severed.get("rest_seconds", 40.0)))
		var count := pieces_max / 2 if k == 0 else pieces_max / 4
		var mm := _make_layer("Pieces%d" % k, _piece_mesh(k), mat, count, true)
		var data: PackedFloat32Array = mm.buffer
		_pieces.append({"mm": mm, "data": data, "next": 0, "dirty": false, "material": mat})


## Couche en tampon circulaire ; `colors` : couleur d'instance (morceaux, 20 flottants par instance).
func _make_layer(layer_name: String, mesh: Mesh, mat: ShaderMaterial, count: int, colors: bool = false) -> MultiMesh:
	var mm := MultiMesh.new()
	mm.transform_format = MultiMesh.TRANSFORM_3D
	mm.use_colors = colors
	mm.use_custom_data = true
	mm.mesh = mesh
	mm.instance_count = maxi(count, 1)
	mm.visible_instance_count = 0 if count <= 0 else -1
	var inst := MultiMeshInstance3D.new()
	inst.name = layer_name
	inst.multimesh = mm
	inst.material_override = mat
	inst.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	# Trajectoires calculées dans le shader : boîte englobante de tout le champ.
	inst.custom_aabb = AABB(Vector3(-2000, -200, -2000), Vector3(6400, 600, 5600))  # EP1 : jusqu’au champ 2400 × 1600
	add_child(inst)
	# Instances inactives : instant très ancien (repliées par le shader).
	var stride := 20 if colors else 16
	var data := PackedFloat32Array()
	data.resize(mm.instance_count * stride)
	data.fill(0.0)
	for i in mm.instance_count:
		data[i * stride + stride - 1] = -1.0e6
	mm.buffer = data
	return mm


## Maillages des morceaux : tête (sphère), avant-bras (capsule fine), jambe (capsule).
static func _piece_mesh(kind: int) -> Mesh:
	if kind == 0:
		var head := SphereMesh.new()
		head.radius = 0.11
		head.height = 0.24
		head.radial_segments = 10
		head.rings = 6
		return head
	var limb := CapsuleMesh.new()
	limb.radius = 0.045 if kind == 1 else 0.06
	limb.height = 0.42 if kind == 1 else 0.5
	limb.radial_segments = 8
	limb.rings = 2
	return limb


func sprays_enabled() -> bool:
	return float(_level.get("sprays", 0.0)) > 0.0 and _drops.instance_count > 1


func dismember_enabled() -> bool:
	return bool(_level.get("dismember", false))


## Gerbe de `count` gouttes (avant le réglage) depuis `pos` (au sol), poussée vers `dir`.
func spray(pos: Vector3, dir: Vector3, count: int, height: float = 1.2, strength: float = 1.0) -> void:
	if not sprays_enabled() or count <= 0:
		return
	var sprays: Dictionary = settings().get("sprays", {})
	if camera_pos.distance_to(pos) > float(sprays.get("max_distance_m", 220.0)):
		return
	var n := int(round(float(count) * float(_level.get("sprays", 0.0))))
	var speed: Array = sprays.get("speed", [1.2, 4.2])
	var flat := Vector3(dir.x, 0.0, dir.z)
	flat = flat.normalized() if flat.length() > 0.01 else Vector3.FORWARD
	for _i in n:
		var v := flat.rotated(Vector3.UP, _rng.randf_range(-0.9, 0.9)) * _rng.randf_range(float(speed[0]), float(speed[1])) * strength
		v.y = _rng.randf_range(0.6, 2.8) * strength
		var o := _drops_next * 16
		_write(_drops_data, o, pos, height + _rng.randf_range(-0.2, 0.2), _rng.randf_range(0.025, 0.06))
		_drops_data[o + 12] = v.x
		_drops_data[o + 13] = v.y
		_drops_data[o + 14] = v.z
		_drops_data[o + 15] = anim_time + _rng.randf_range(0.0, 0.08)
		_drops_next = (_drops_next + 1) % _drops.instance_count
		drops_emitted += 1
	_drops_dirty = true


## Morceau tranché (`part` : head, arm_r, arm_l, leg_r, leg_l) projeté depuis `pos` vers `dir`.
func sever(pos: Vector3, dir: Vector3, part: String, height: float) -> void:
	if not dismember_enabled() or not PIECE_KINDS.has(part):
		return
	var layer: Dictionary = _pieces[int(PIECE_KINDS[part])]
	var mm: MultiMesh = layer["mm"]
	if mm.instance_count <= 1:
		return
	var severed: Dictionary = settings().get("severed", {})
	var speed: Array = severed.get("speed", [2.0, 5.0])
	var flat := Vector3(dir.x, 0.0, dir.z)
	flat = flat.normalized() if flat.length() > 0.01 else Vector3.FORWARD
	var v := flat.rotated(Vector3.UP, _rng.randf_range(-0.6, 0.6)) * _rng.randf_range(float(speed[0]), float(speed[1]))
	v.y = _rng.randf_range(1.5, 3.5)
	var data: PackedFloat32Array = layer["data"]
	var slot: int = layer["next"]
	var o := slot * 20
	# Base identité, couleur d'instance = (hauteur de départ / 4, teinte : casque, cheveux, manche).
	_write(data, o, pos, 0.0, 1.0)
	data[o + 5] = 1.0
	data[o + 12] = clampf(height / 4.0, 0.0, 1.0)
	data[o + 13] = _rng.randf()
	data[o + 14] = 0.0
	data[o + 15] = 1.0
	o += 4
	data[o + 12] = v.x
	data[o + 13] = v.y
	data[o + 14] = v.z
	data[o + 15] = anim_time
	layer["data"] = data
	layer["next"] = (slot + 1) % mm.instance_count
	layer["dirty"] = true
	pieces_emitted += 1
	# Le sang gicle du moignon.
	spray(pos, dir, int(settings().get("sprays", {}).get("droplets_per_critical", 20)), height, 1.2)


## Transformée d'instance : origine au sol `pos`, hauteur de départ dans la colonne y.y, taille
## en colonne z.z (le shader relit ces valeurs, la base n'est pas une rotation).
static func _write(data: PackedFloat32Array, o: int, pos: Vector3, height: float, size: float) -> void:
	data[o + 0] = 1.0
	data[o + 1] = 0.0
	data[o + 2] = 0.0
	data[o + 3] = pos.x
	data[o + 4] = 0.0
	data[o + 5] = height
	data[o + 6] = 0.0
	data[o + 7] = pos.y
	data[o + 8] = 0.0
	data[o + 9] = 0.0
	data[o + 10] = size
	data[o + 11] = pos.z


func update(dt: float, camera: Vector3) -> void:
	anim_time += dt
	camera_pos = camera
	_drop_material.set_shader_parameter("anim_time", anim_time)
	if _drops_dirty:
		_drops.buffer = _drops_data
		_drops_dirty = false
	for layer in _pieces:
		(layer["material"] as ShaderMaterial).set_shader_parameter("anim_time", anim_time)
		if layer["dirty"]:
			(layer["mm"] as MultiMesh).buffer = layer["data"]
			layer["dirty"] = false
