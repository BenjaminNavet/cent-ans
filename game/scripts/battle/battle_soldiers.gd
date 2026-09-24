class_name BattleSoldiers
extends Node3D

## Rendu des soldats (lot V4) : un `MultiMeshInstance3D` par régiment (maillage de sa famille et
## de sa variante, matériau `battle_soldier.gdshader` propre : livrée, blason, état d'animation),
## rempli chaque image par tranches du tampon `BattleSim.get_soldier_buffer(side, render)` —
## la simulation reste seule maîtresse des positions. Les soldats tombés deviennent des cadavres
## (MultiMesh par camp, famille et variante, à données perso : instant de mort, sens de chute)
## qui basculent au sol puis y restent.
##
## Le tampon Rust concatène les soldats des régiments d'un camp et d'une famille dans l'ordre de
## `get_units()` ; chaque régiment présent y occupe exactement `soldiers` transformées.

const SOLDIER_SHADER := preload("res://shaders/battle_soldier.gdshader")
const KINDS := ["infantry", "archer", "cavalry", "siege"]
const MAX_CORPSES := 4000
const TRIM_GOLD := Color(0.83, 0.66, 0.24)
const TRIM_SILVER := Color(0.85, 0.85, 0.82)
## Niveaux de détail (lots V4b, B1), distance caméra → régiment (m) : maillage complet en deçà
## de `DETAIL_DISTANCE`, moyen jusqu'à `LOD_DISTANCE` (leur ombre est portée par le maillage
## lointain), maillage lointain au-delà, sans ombre portée après `SHADOW_DISTANCE`.
const DETAIL_DISTANCE := 32.0
## Figurines skinnées (lot V2) : maillage complet plus tôt relayé (skinning plus coûteux).
const SKINNED_DETAIL_DISTANCE := 24.0
const LOD_DISTANCE := 75.0
const SHADOW_DISTANCE := 190.0
## A1-01 : fondu de lisibilité à distance (teinte de camp, liseré, échelle), en mètres.
const READABLE_NEAR := 80.0
const READABLE_FAR := 260.0

## unit id -> MultiMeshInstance3D (exposé à la scène : `_mm` du test de fumée).
var layers: Dictionary = {}
var anim_time: float = 0.0
var corpse_count: int = 0
var cast_shadows: bool = true

var _materials: Dictionary = {}  # unit id -> ShaderMaterial
var _unit_kind: Dictionary = {}  # unit id -> famille de rendu
var _lod_layers: Dictionary = {}  # unit id -> MultiMeshInstance3D (maillage lointain)
var _near_level: Dictionary = {}  # unit id -> niveau de détail du maillage proche (0 ou 1)
var _camera_pos: Vector3 = Vector3.ZERO
var _previous: Dictionary = {}  # unit id -> PackedFloat32Array (tranche de l'image précédente)
var _corpse_layers: Dictionary = {}  # "side/kind/variant" -> {mm, data, count, next}
var _side_colors: Dictionary = {}
var _side_heraldry: Dictionary = {}
var _rng := RandomNumberGenerator.new()
var _warned: bool = false
var _anim_track: Dictionary = {}  # unit id -> {ammo, state, since} (lot B4 : volées, chocs)
var _skinned: Dictionary = {}  # unit id -> true : figurine skinnée (lot V2, `BattleSkinned`)


## Crée les couches des régiments de `units` ; `side_colors` / `side_factions` par camp.
func setup(units: Array, side_colors: Dictionary, side_factions: Dictionary) -> void:
	_rng.seed = 4242
	_side_colors = side_colors
	for side in side_factions:
		_side_heraldry[side] = PortraitLoader.heraldry_texture(str(side_factions[side]))
	for unit in units:
		var kind := str(unit["render"])
		if not KINDS.has(kind):
			continue
		var id := int(unit["id"])
		var variant := BattleMeshes.variant_of(str(unit.get("type", "")))
		var side := str(unit["side"])
		var skinned := BattleSkinned.has_figure(kind, variant)
		var mat := _make_skinned_material(side, kind, variant, false) if skinned else _make_material(side, kind, variant, false)
		var mm := MultiMesh.new()
		mm.transform_format = MultiMesh.TRANSFORM_3D
		mm.mesh = BattleSkinned.mesh(kind, variant, 0) if skinned else BattleMeshes.soldier(kind, variant)
		# BV1 (ADR 0016) : `figures` = figurines dessinées (soldats × taille d'unité).
		var scale := float(unit.get("figures", unit["soldiers"])) / maxf(float(unit["soldiers"]), 1.0)
		mm.instance_count = maxi(int(ceil(int(unit.get("initial_soldiers", 0)) * scale)), int(unit.get("figures", unit["soldiers"])))
		mm.visible_instance_count = 0
		var instance := MultiMeshInstance3D.new()
		instance.name = "Unit%d_%s" % [id, kind]
		instance.multimesh = mm
		instance.material_override = mat
		instance.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
		add_child(instance)
		layers[id] = instance
		var lod_mm := MultiMesh.new()
		lod_mm.transform_format = MultiMesh.TRANSFORM_3D
		lod_mm.mesh = BattleSkinned.mesh(kind, variant, 2) if skinned else BattleMeshes.soldier_level(kind, variant, BattleMeshes.LEVEL_FAR)
		lod_mm.instance_count = mm.instance_count
		lod_mm.visible_instance_count = 0
		var lod := MultiMeshInstance3D.new()
		lod.name = "Unit%d_%s_lod" % [id, kind]
		lod.multimesh = lod_mm
		lod.material_override = mat
		add_child(lod)
		_lod_layers[id] = lod
		_materials[id] = mat
		_unit_kind[id] = kind
		if skinned:
			_skinned[id] = true


func _make_material(side: String, kind: String, variant: int, corpse: bool) -> ShaderMaterial:
	var mat := ShaderMaterial.new()
	mat.shader = SOLDIER_SHADER
	var color: Color = _side_colors.get(side, Color(0.5, 0.5, 0.5))
	mat.set_shader_parameter("livery", color)
	mat.set_shader_parameter("trim", TRIM_SILVER if color.get_luminance() > 0.55 or (color.r > 0.6 and color.g > 0.5) else TRIM_GOLD)
	var arms: Texture2D = _side_heraldry.get(side)
	mat.set_shader_parameter("heraldry", arms)
	mat.set_shader_parameter("has_heraldry", arms != null)
	mat.set_shader_parameter("weapon_mode", BattleMeshes.weapon_mode(kind, variant))
	var mounted := kind == "cavalry"
	mat.set_shader_parameter("mounted", mounted)
	mat.set_shader_parameter("hip", Vector2(1.68, -0.05) if mounted else Vector2(0.93, 0.0))
	mat.set_shader_parameter("shoulder", Vector2(2.18, -0.05) if mounted else Vector2(1.4, 0.0))
	mat.set_shader_parameter("elbow", Vector2(1.91, -0.07) if mounted else Vector2(1.13, -0.02))
	# Rechargement de la simulation : 6 s, 9 s derrière un pavois (Génois), 12 s les engins.
	mat.set_shader_parameter("reload_time", 12.0 if kind == "siege" else 9.0 if kind == "archer" and variant == 2 else 6.0)
	mat.set_shader_parameter("corpse", corpse)
	mat.set_shader_parameter("torso_y", 0.78 if mounted else 0.0)
	mat.set_shader_parameter("torso_z", -0.05 if mounted else 0.0)
	# Nobles (hommes d'armes, chevaliers) presque tous en livrée ; troupe plus mêlée.
	var noble := variant == 0 and (kind == "infantry" or kind == "cavalry")
	mat.set_shader_parameter("livery_share", 0.7 if noble else 0.4)
	return mat


## Matériau des figurines skinnées (lot V2) : même livrée et blason que `_make_material`,
## texture d'os et table des clips du rig ; cadavres en mode CUSTOM (clips de mort).
func _make_skinned_material(side: String, kind: String, variant: int, corpse: bool) -> ShaderMaterial:
	var mat := ShaderMaterial.new()
	mat.shader = BattleSkinned.SHADER
	var color: Color = _side_colors.get(side, Color(0.5, 0.5, 0.5))
	mat.set_shader_parameter("livery", color)
	mat.set_shader_parameter("trim", TRIM_SILVER if color.get_luminance() > 0.55 or (color.r > 0.6 and color.g > 0.5) else TRIM_GOLD)
	var arms: Texture2D = _side_heraldry.get(side)
	mat.set_shader_parameter("heraldry", arms)
	mat.set_shader_parameter("has_heraldry", arms != null)
	BattleSkinned.setup_material(mat, kind, variant)
	mat.set_shader_parameter("reload_time", 9.0 if kind == "archer" and variant == 2 else 6.0)
	var noble := variant == 0 and (kind == "infantry" or kind == "cavalry")
	mat.set_shader_parameter("livery_share", 0.92 if noble else 0.6)
	if corpse:
		BattleSkinned.apply_config(mat, BattleSkinned.death_config(kind, variant), anim_time)
	else:
		BattleSkinned.apply_config(mat, BattleSkinned.state_config(kind, variant, "idle", false), anim_time)
	return mat


## État d'animation (`anim_state` du shader) d'un régiment.
static func anim_state(unit: Dictionary) -> int:
	match str(unit.get("state", "idle")):
		"marching":
			return 2 if bool(unit.get("running", false)) else 1
		"charging":
			return 2
		"melee":
			return 3
		"shooting":
			return 4
		"routing":
			return 6
		"climbing":
			return 5
	return 0


## Met à jour les instances depuis la simulation ; `anim_dt` = temps simulé écoulé (0 en pause).
func update(battle: Object, units: Array, anim_dt: float, selected: Array) -> void:
	anim_time += anim_dt
	var camera := get_viewport().get_camera_3d()
	if camera != null:
		_camera_pos = camera.global_position
	for side in ["attacker", "defender"]:
		for kind in KINDS:
			var buffer: PackedFloat32Array = battle.call("get_soldier_buffer", side, kind)
			var total := buffer.size() / 12
			var offset := 0
			for unit in units:
				if str(unit["side"]) != side or str(unit["render"]) != kind:
					continue
				var id := int(unit["id"])
				if not layers.has(id):
					continue
				var n := int(unit.get("figures", unit["soldiers"])) if bool(unit["present"]) else 0
				if offset + n > total:
					if not _warned:
						push_warning("BattleSoldiers: soldier buffer shorter than expected (%s/%s)" % [side, kind])
						_warned = true
					n = maxi(total - offset, 0)
				var slice := buffer.slice(offset * 12, (offset + n) * 12)
				offset += n
				_update_unit(unit, id, kind, slice, n, selected.has(id))


func _update_unit(unit: Dictionary, id: int, kind: String, slice: PackedFloat32Array, n: int, is_selected: bool) -> void:
	var instance: MultiMeshInstance3D = layers[id]
	var mm := instance.multimesh
	# Soldats tombés depuis l'image précédente (régiment resté sur le champ).
	if _previous.has(id):
		var prev: PackedFloat32Array = _previous[id]
		var prev_n := prev.size() / 12
		if n < prev_n and n > 0 and bool(unit["present"]):
			_spawn_corpses(str(unit["side"]), kind, BattleMeshes.variant_of(str(unit.get("type", ""))), prev, prev_n - n)
	_previous[id] = slice
	var lod: MultiMeshInstance3D = _lod_layers[id]
	var lod_mm := lod.multimesh
	if n > mm.instance_count:
		mm.instance_count = n
		lod_mm.instance_count = n
	# Distance au régiment : caméra → centre du régiment (x, z de la simulation).
	var distance := _camera_pos.distance_to(Vector3(float(unit["x"]), float(unit.get("y", 0.0)), float(unit["z"])))
	var near := distance < LOD_DISTANCE
	var shadow := cast_shadows and distance < SHADOW_DISTANCE
	var skinned := _skinned.has(id)
	var level := BattleMeshes.LEVEL_FULL if distance < (SKINNED_DETAIL_DISTANCE if skinned else DETAIL_DISTANCE) else BattleMeshes.LEVEL_MEDIUM
	if near and int(_near_level.get(id, -1)) != level:
		_near_level[id] = level
		var variant := BattleMeshes.variant_of(str(unit.get("type", "")))
		mm.mesh = BattleSkinned.mesh(kind, variant, level) if skinned else BattleMeshes.soldier_level(kind, variant, level)
	instance.visible = n > 0 and near
	lod.visible = n > 0 and (not near or shadow)
	if near:
		lod.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_SHADOWS_ONLY
	else:
		lod.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_ON if shadow else GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	if n > 0:
		var padded := slice
		if padded.size() != mm.instance_count * 12:
			padded = slice.duplicate()
			padded.resize(mm.instance_count * 12)
		if instance.visible:
			mm.buffer = padded
		if lod.visible:
			lod_mm.buffer = padded
	mm.visible_instance_count = n
	lod_mm.visible_instance_count = n
	var mat: ShaderMaterial = _materials[id]
	mat.set_shader_parameter("anim_time", anim_time)
	mat.set_shader_parameter("anim_state", anim_state(unit))
	mat.set_shader_parameter("highlight", 1.0 if is_selected else 0.0)
	mat.set_shader_parameter("far_blend", smoothstep(READABLE_NEAR, READABLE_FAR, distance))
	# Lot B4 : décoche calée sur la volée (munitions qui baissent), choc au changement d'état.
	var ammo := int(unit.get("ammo", 0))
	var state := str(unit.get("state", ""))
	var track: Dictionary = _anim_track.get(id, {})
	if track.is_empty():
		track = {"ammo": ammo, "state": state, "since": anim_time - 100.0}
		_anim_track[id] = track
	if ammo < int(track["ammo"]):
		mat.set_shader_parameter("volley_time", anim_time)
	if state != str(track["state"]):
		track["since"] = anim_time
	track["ammo"] = ammo
	track["state"] = state
	mat.set_shader_parameter("state_time", anim_time - float(track["since"]))
	if skinned:
		var config := BattleSkinned.state_config(kind, BattleMeshes.variant_of(str(unit.get("type", ""))), state, bool(unit.get("running", false)))
		BattleSkinned.apply_config(mat, config, anim_time)


## Positions (au sol) d'au plus `count` soldats du régiment `id`, pris à intervalles réguliers
## depuis un point tiré au hasard (départs des traits d'une volée, lot B4).
func soldier_positions(id: int, count: int) -> PackedVector3Array:
	var out := PackedVector3Array()
	if not _previous.has(id) or count <= 0:
		return out
	var slice: PackedFloat32Array = _previous[id]
	var n := slice.size() / 12
	if n == 0:
		return out
	var step := maxf(float(n) / float(count), 1.0)
	var k := _rng.randf() * step
	while k < n and out.size() < count:
		var o := int(k) * 12
		out.append(Vector3(slice[o + 3], slice[o + 7], slice[o + 11]))
		k += step
	return out


## Ajoute `count` cadavres pris au hasard dans la tranche précédente (plutôt au premier rang).
func _spawn_corpses(side: String, kind: String, variant: int, prev: PackedFloat32Array, count: int) -> void:
	var key := "%s/%s/%d" % [side, kind, variant]
	var skinned := BattleSkinned.has_figure(kind, variant)
	var deaths: int = (BattleSkinned.death_config(kind, variant)["set"] as Array).size() if skinned else 1
	if not _corpse_layers.has(key):
		var mm := MultiMesh.new()
		mm.transform_format = MultiMesh.TRANSFORM_3D
		mm.use_custom_data = true
		mm.mesh = BattleSkinned.mesh(kind, variant, 1) if skinned else BattleMeshes.soldier(kind, variant, true)
		mm.instance_count = 0
		var instance := MultiMeshInstance3D.new()
		instance.name = "Corpses_%s_%s_%d" % [side, kind, variant]
		instance.multimesh = mm
		instance.material_override = _make_skinned_material(side, kind, variant, true) if skinned else _make_material(side, kind, variant, true)
		instance.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
		add_child(instance)
		_corpse_layers[key] = {"mm": mm, "data": PackedFloat32Array(), "count": 0, "next": 0, "material": instance.material_override}
	var layer: Dictionary = _corpse_layers[key]
	var data: PackedFloat32Array = layer["data"]
	var prev_n := prev.size() / 12
	for _i in mini(count, 60):
		var k := mini(int(pow(_rng.randf(), 1.6) * prev_n), prev_n - 1)
		var o := k * 12
		var record := PackedFloat32Array()
		record.resize(16)
		for j in 12:
			record[j] = prev[o + j]
		record[12] = anim_time
		# Figurines skinnées : clip de mort tiré au sort (INSTANCE_CUSTOM.y) ; w = sang (0).
		record[13] = float(_rng.randi_range(0, deaths - 1))
		record[14] = 1.0 if _rng.randf() < 0.5 else -1.0
		record[15] = 0.0
		var count_now: int = layer["count"]
		if count_now < MAX_CORPSES:
			data.append_array(record)
			layer["count"] = count_now + 1
		else:
			var slot: int = layer["next"]
			for j in 16:
				data[slot * 16 + j] = record[j]
			layer["next"] = (slot + 1) % MAX_CORPSES
		corpse_count += 1
	layer["data"] = data
	var mm: MultiMesh = layer["mm"]
	if mm.instance_count != int(layer["count"]):
		mm.instance_count = int(layer["count"])
	mm.buffer = data


## Banc d'essai : durées d'image relevées depuis `start_timing` (Metal ne donne pas le
## temps GPU) ; la médiane résiste aux à-coups des autres programmes de la machine.
var _frame_times: PackedFloat32Array = PackedFloat32Array()
var _timing: bool = false


func start_timing() -> void:
	_timing = true


## Texte « médiane x i/s » pour la ligne du banc d'essai.
func timing_report() -> String:
	if _frame_times.is_empty():
		return ""
	var sorted := _frame_times.duplicate()
	sorted.sort()
	var median := sorted[sorted.size() / 2]
	var prims := Performance.get_monitor(Performance.RENDER_TOTAL_PRIMITIVES_IN_FRAME)
	var draws := Performance.get_monitor(Performance.RENDER_TOTAL_DRAW_CALLS_IN_FRAME)
	return ", median %.1f FPS, %.2f M primitives, %d draw calls" % [1.0 / maxf(median, 0.0001), prims / 1.0e6, int(draws)]


## Temps d'animation propagé aux cadavres (chute).
func _process(delta: float) -> void:
	if _timing:
		_frame_times.append(delta)
	for key in _corpse_layers:
		var mat: ShaderMaterial = _corpse_layers[key]["material"]
		mat.set_shader_parameter("anim_time", anim_time)
