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
##
## Lot BV2 (bataille vivante) : la mort tirée suit la cause des pertes venue du cœur
## (`loss_cause`, `loss_by` : flèche, mêlée, charge, boulet, pieux, piques…) ; les cadavres
## restent toute la bataille, rangés par cellules de terrain (LOD et masquage au loin, plafond
## global) ; les chocs de cavalerie (`get_impacts`) renversent et projettent des fantassins qui se
## relèvent (couche « renversés »), ralentissent les chevaux ; sang sur les figurines, gerbes et
## démembrements (`BattleGore`, réglage « Sang »).

## Émis à chaque cadavre posé (point d'accroche pour les flèches plantées du lot BV1) :
## position au sol, camp, famille, cause des pertes.
signal corpse_fallen(position: Vector3, side: String, kind: String, cause: String)

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
## EP1 (ADR 0031) : budget d'animation décroissant avec la distance. Au-delà de
## `BUDGET_NEAR` mètres, un régiment n'est remis à jour (tampon d'instances, matériau) qu'une
## image sur 2, au-delà de `BUDGET_FAR` une sur 3 (décalé selon l'id : charge étalée).
const BUDGET_NEAR := 450.0
const BUDGET_FAR := 800.0
## EP1 : au-delà de `THIN_DISTANCE` mètres, imposteurs à demi-densité (`thin_out` du shader).
const THIN_DISTANCE := 700.0

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
## Lot BV2.
var gore: BattleGore = null
var knocked_count: int = 0
var severed_count: int = 0
var _gore: Dictionary = {}  # data/fx/battle_gore.json
var _unit_info: Dictionary = {}  # unit id -> {type, kind, variant, side}
var _unit_pos: Dictionary = {}  # unit id -> Vector3 (centre, dernière image)
var _unit_scale: Dictionary = {}  # unit id -> figurines par soldat simulé
var _corpse_materials: Dictionary = {}  # "side/kind/variant" -> ShaderMaterial
var _corpse_total: int = 0
var _dirty_corpses: Dictionary = {}
## EP1 : budget d'animation par distance (`--no-ep1-budget` le coupe, mesures A/B).
var budget_enabled: bool = not OS.get_cmdline_user_args().has("--no-ep1-budget")
var skipped_updates: int = 0
var _frame_index: int = 0  # EP1 : cellules de cadavres à renvoyer au GPU
var _tumble_layers: Dictionary = {}  # "side/kind/variant" -> {mm, data, next, material}
var _hidden: Dictionary = {}  # unit id -> {rang: instant de retour}
var _lag: Dictionary = {}  # unit id -> retard d'horloge d'animation (chevaux ralentis)
var _slow: Dictionary = {}  # unit id -> {since, kind}
var _drive: Dictionary = {}  # unit id -> {since, depth, dir} : chevaux qui entrent dans la masse
var _charge_mass: Dictionary = {}  # unit id -> poids de la dernière charge
var _melee_time: Dictionary = {}  # unit id -> secondes de mêlée cumulées
var _speed: Dictionary = {}  # unit id -> vitesse au sol lissée (m/s)
var _braced: Dictionary = {}  # unit id -> true : piques abaissées devant une charge
var _frame_dt: float = 0.0
var _audio: Script = null
## `--no-bv2` après `--` : rendu d'avant BV2 (mesures A/B) — ni chocs, ni sang, ni cadence.
## BV3 : imposteurs lointains (au-delà de `BattleImpostors.DISTANCE`), null si coupés.
var impostors: BattleImpostors = null
var _imp_layers: Dictionary = {}  # unit id -> MultiMeshInstance3D (quadrilatères)
## BV3 : les pavois des génois sont plantés en rangée (`BattleVolleys`) : celui du dos disparaît.
var hide_planted_pavise: bool = false
var bv2_enabled: bool = not OS.get_cmdline_user_args().has("--no-bv2")
var _level: Dictionary = {}  # intensités du réglage « Sang » (lues au début de la bataille)
## EP5 : figurines du tampon remplacées par un porte-étendard ou un musicien dédié
## (`BattleStandards`) : unit id -> PackedInt32Array des rangs masqués.
var reserved: Dictionary = {}


## Crée les couches des régiments de `units` ; `side_colors` / `side_factions` par camp.
func setup(units: Array, side_colors: Dictionary, side_factions: Dictionary) -> void:
	_rng.seed = 4242
	_gore = BattleGore.settings()
	_level = BattleGore.level_settings()
	gore = BattleGore.new()
	gore.name = "Gore"
	add_child(gore)
	if ResourceLoader.exists("res://scripts/audio/battle_audio.gd"):
		_audio = load("res://scripts/audio/battle_audio.gd")
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
		_unit_info[id] = {"type": str(unit.get("type", "")), "kind": kind, "variant": variant, "side": side}
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
			if impostors != null:
				impostors.request(BattleImpostors.key_of(side, kind, variant), kind, variant, mat)


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
	var noble := BattleSkinned.is_noble(kind, variant)
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
	var noble := BattleSkinned.is_noble(kind, variant)
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
	_frame_dt = anim_dt
	_frame_index += 1
	var smooth := 1.0 - exp(-anim_dt / 0.6)
	for unit in units:
		var uid := int(unit["id"])
		var pos := Vector3(float(unit["x"]), float(unit.get("y", 0.0)), float(unit["z"]))
		if anim_dt > 0.0 and _unit_pos.has(uid):
			var step := Vector2(pos.x - (_unit_pos[uid] as Vector3).x, pos.z - (_unit_pos[uid] as Vector3).z).length()
			var v := minf(step / anim_dt, 30.0)
			_speed[uid] = float(_speed.get(uid, v)) + (v - float(_speed.get(uid, v))) * smooth
		_unit_pos[uid] = pos
		if str(unit.get("state", "")) == "melee":
			_melee_time[uid] = float(_melee_time.get(uid, 0.0)) + anim_dt
	_advance_lags(anim_dt)
	if bv2_enabled:
		_find_braced(units)
	if gore != null:
		gore.update(anim_dt, _camera_pos)
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
				if budget_enabled and _skip_far(unit, id, n):
					offset += n
					skipped_updates += 1
					continue
				var slice := buffer.slice(offset * 12, (offset + n) * 12)
				offset += n
				_unit_scale[id] = float(n) / maxf(float(unit["soldiers"]), 1.0) if bool(unit["present"]) else 1.0
				_update_unit(unit, id, kind, slice, n, selected.has(id))
	# Lot BV2 : chocs de cavalerie résolus par le cœur depuis l'image précédente.
	if bv2_enabled and battle.has_method("get_impacts"):
		var impacts: Array = battle.call("get_impacts")
		if not impacts.is_empty():
			apply_impacts(impacts)
	if not _dirty_corpses.is_empty():
		_flush_corpses()


## EP1 : `true` quand le régiment lointain saute cette image (budget d'animation). Jamais quand
## son effectif dessiné change (morts, renforts) ni pour un régiment encore jamais dessiné.
func _skip_far(unit: Dictionary, id: int, n: int) -> bool:
	if not _previous.has(id) or (_previous[id] as PackedFloat32Array).size() != n * 12:
		return false
	var distance := _camera_pos.distance_to(Vector3(float(unit["x"]), float(unit.get("y", 0.0)), float(unit["z"])))
	if distance < BUDGET_NEAR:
		return false
	var period := 3 if distance >= BUDGET_FAR else 2
	return (_frame_index + id) % period != 0


func _update_unit(unit: Dictionary, id: int, kind: String, slice: PackedFloat32Array, n: int, is_selected: bool) -> void:
	var instance: MultiMeshInstance3D = layers[id]
	var mm := instance.multimesh
	# Soldats tombés depuis l'image précédente (régiment resté sur le champ).
	if _previous.has(id):
		var prev: PackedFloat32Array = _previous[id]
		var prev_n := prev.size() / 12
		if n < prev_n and n > 0 and bool(unit["present"]):
			_spawn_corpses(unit, str(unit["side"]), kind, BattleMeshes.variant_of(str(unit.get("type", ""))), prev, prev_n - n)
	_previous[id] = slice
	# Renversés (lot BV2) : leur place dans la formation est vide jusqu'à ce qu'ils se relèvent.
	if _hidden.has(id):
		slice = _hide_knocked(id, slice, n)
	if _drive.has(id):
		slice = _drive_in(id, slice, n)
	if reserved.has(id):
		slice = _hide_reserved(reserved[id], slice, n)
	var lod: MultiMeshInstance3D = _lod_layers[id]
	var lod_mm := lod.multimesh
	if n > mm.instance_count:
		mm.instance_count = n
		lod_mm.instance_count = n
	# Distance au régiment : caméra → centre du régiment (x, z de la simulation).
	var distance := _camera_pos.distance_to(Vector3(float(unit["x"]), float(unit.get("y", 0.0)), float(unit["z"])))
	# PF1 : distances de LOD, d'ombre et d'imposteurs selon le préréglage de qualité.
	var lod_k := RenderQuality.battle_lod_scale
	var near := distance < LOD_DISTANCE * lod_k
	var shadow := cast_shadows and distance < SHADOW_DISTANCE * lod_k
	var skinned := _skinned.has(id)
	var level := BattleMeshes.LEVEL_FULL if distance < (SKINNED_DETAIL_DISTANCE if skinned else DETAIL_DISTANCE) * lod_k else BattleMeshes.LEVEL_MEDIUM
	if near and int(_near_level.get(id, -1)) != level:
		_near_level[id] = level
		var variant := BattleMeshes.variant_of(str(unit.get("type", "")))
		mm.mesh = BattleSkinned.mesh(kind, variant, level) if skinned else BattleMeshes.soldier_level(kind, variant, level)
	instance.visible = n > 0 and near
	lod.visible = n > 0 and (not near or shadow)
	# BV3 : imposteurs au-delà de 300 m (atlas cuit au début de la bataille).
	var imp: MultiMeshInstance3D = null
	# Imposteurs pas avant 80 % de leur distance : plus près, leur teinte pâle se remarque.
	if impostors != null and skinned and distance > BattleImpostors.DISTANCE * maxf(lod_k, 0.8):
		imp = _impostor_layer(id, str(unit["side"]), kind, BattleMeshes.variant_of(str(unit.get("type", ""))), mm.instance_count)
		if imp != null:
			lod.visible = false
	if _imp_layers.has(id):
		(_imp_layers[id] as MultiMeshInstance3D).visible = imp != null and n > 0
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
		if imp != null:
			if imp.multimesh.instance_count != mm.instance_count:
				imp.multimesh.instance_count = mm.instance_count
			imp.multimesh.buffer = padded
	mm.visible_instance_count = n
	lod_mm.visible_instance_count = n
	if imp != null:
		imp.multimesh.visible_instance_count = n
		var imp_mat := imp.material_override as ShaderMaterial
		imp_mat.set_shader_parameter("anim_time", anim_time - float(_lag.get(id, 0.0)))
		imp_mat.set_shader_parameter("imp_set", BattleImpostors.state_set(str(unit.get("state", "")), bool(unit.get("running", false))))
		imp_mat.set_shader_parameter("highlight", 1.0 if is_selected else 0.0)
		imp_mat.set_shader_parameter("thin_out", 1.0 if budget_enabled and distance > THIN_DISTANCE and not is_selected else 0.0)
	var mat: ShaderMaterial = _materials[id]
	var ammo := int(unit.get("ammo", 0))
	var state := str(unit.get("state", ""))
	var config: Dictionary = {}
	if skinned:
		var shown_state := "brace" if _braced.has(id) and ["idle", "marching", "rallied"].has(state) else state
		config = BattleSkinned.state_config(kind, BattleMeshes.variant_of(str(unit.get("type", ""))), shown_state, bool(unit.get("running", false)))
		# Lot BV2 : cadence de marche calée sur la vitesse réelle du régiment (pieds qui ne
		# glissent plus) — l'horloge du régiment avance plus ou moins vite, sans saut de phase.
		var cadence := _cadence(id, config) if bv2_enabled else 1.0
		if cadence != 1.0:
			_lag[id] = float(_lag.get(id, 0.0)) + _frame_dt * (1.0 - cadence)
	# Horloge propre du régiment (retard des chevaux ralentis, cadence) : tous les instants
	# du matériau (fondus, volées, état) sont pris sur elle.
	var local := anim_time - float(_lag.get(id, 0.0))
	mat.set_shader_parameter("anim_time", local)
	mat.set_shader_parameter("anim_state", anim_state(unit))
	mat.set_shader_parameter("highlight", 1.0 if is_selected else 0.0)
	mat.set_shader_parameter("far_blend", smoothstep(READABLE_NEAR, READABLE_FAR, distance))
	# Lot B4 : décoche calée sur la volée (munitions qui baissent), choc au changement d'état.
	var track: Dictionary = _anim_track.get(id, {})
	if track.is_empty():
		track = {"ammo": ammo, "state": state, "since": local - 100.0}
		_anim_track[id] = track
	if ammo < int(track["ammo"]):
		mat.set_shader_parameter("volley_time", local)
	if state != str(track["state"]):
		track["since"] = local
	track["ammo"] = ammo
	track["state"] = state
	mat.set_shader_parameter("state_time", local - float(track["since"]))
	if skinned:
		BattleSkinned.apply_config(mat, config, local)
		# SG1 : soldats de tête sur les échelles ou le pont du beffroi (clip d'escalade).
		mat.set_shader_parameter("split_count", int(unit.get("climbers_shown", 0)))
		# Sang : uniforme mis à jour seulement quand il change sensiblement.
		# BV3 : pavois du dos masqué tant que la rangée est plantée (même règle que BV1).
		if hide_planted_pavise:
			var planted := bool(unit.get("pavise_cover", false)) and state != "marching" and state != "charging"
			if planted != bool(mat.get_meta("bv3_pavise", false)):
				mat.set_meta("bv3_pavise", planted)
				mat.set_shader_parameter("hide_pavise", planted)
		var blood := snappedf(_living_blood(unit, id), 0.02)
		if not is_equal_approx(float(mat.get_meta("bv2_blood", -1.0)), blood):
			mat.set_meta("bv2_blood", blood)
			mat.set_shader_parameter("blood", blood)


## BV3 : repère (position au sol, cap) de la figurine placée à `rank` (0 première, 1 dernière)
## dans le tampon courant du régiment ; null si le régiment n'a pas de figurine dessinée.
func figure_frame(id: int, rank: float) -> Variant:
	if not _previous.has(id):
		return null
	var slice: PackedFloat32Array = _previous[id]
	var n := slice.size() / 12
	if n <= 0:
		return null
	var o := clampi(int(rank * float(n - 1)), 0, n - 1) * 12
	var basis := Basis(Vector3(slice[o], slice[o + 4], slice[o + 8]), Vector3(slice[o + 1], slice[o + 5], slice[o + 9]), Vector3(slice[o + 2], slice[o + 6], slice[o + 10]))
	return Transform3D(basis, Vector3(slice[o + 3], slice[o + 7], slice[o + 11]))


## EP5 : repère de la figurine d'indice `index` du tampon courant du régiment (null sinon).
func figure_at(id: int, index: int) -> Variant:
	if not _previous.has(id):
		return null
	var slice: PackedFloat32Array = _previous[id]
	if index < 0 or index >= slice.size() / 12:
		return null
	var o := index * 12
	var basis := Basis(Vector3(slice[o], slice[o + 4], slice[o + 8]), Vector3(slice[o + 1], slice[o + 5], slice[o + 9]), Vector3(slice[o + 2], slice[o + 6], slice[o + 10]))
	return Transform3D(basis, Vector3(slice[o + 3], slice[o + 7], slice[o + 11]))


## EP5 : figurines dessinées du régiment (tampon courant).
func figure_count(id: int) -> int:
	return (_previous[id] as PackedFloat32Array).size() / 12 if _previous.has(id) else 0


## EP5 : masque (échelle nulle) les figurines remplacées par un porte-étendard ou un musicien.
func _hide_reserved(slots: PackedInt32Array, slice: PackedFloat32Array, n: int) -> PackedFloat32Array:
	# Copie : `_previous` (même tableau) garde les vraies places (`figure_at`).
	var out := slice.duplicate()
	for slot in slots:
		if slot >= 0 and slot < n:
			var o := slot * 12
			for q in [0, 1, 2, 4, 5, 6, 8, 9, 10]:
				out[o + q] = 0.0
	return out


## BV3 : rang (indice dans le tampon courant) de la figurine du régiment la plus proche de
## `point` (-1 : aucune figurine dessinée).
func figure_slot_near(id: int, point: Vector3) -> int:
	if not _previous.has(id):
		return -1
	var slice: PackedFloat32Array = _previous[id]
	var best := -1
	var best_d := INF
	for i in slice.size() / 12:
		var dx := slice[i * 12 + 3] - point.x
		var dz := slice[i * 12 + 11] - point.z
		var d := dx * dx + dz * dz
		if d < best_d:
			best_d = d
			best = i
	return best


## BV3 : repère de la figurine au rang `slot` du tampon courant.
func slot_frame(id: int, slot: int) -> Transform3D:
	var slice: PackedFloat32Array = _previous[id]
	var o := slot * 12
	return Transform3D(Basis.IDENTITY, Vector3(slice[o + 3], slice[o + 7], slice[o + 11]))


## BV3 : retire la figurine `slot` de la formation jusqu'à l'instant `until` (horloge
## d'animation), comme les renversés de BV2 (duels).
func hide_figure(id: int, slot: int, until: float) -> void:
	var hidden: Dictionary = _hidden.get(id, {})
	hidden[slot] = maxf(float(hidden.get(slot, 0.0)), until)
	_hidden[id] = hidden


## BV3 : figurine skinnée du régiment (famille, variante, matériau) ; vide sinon.
func skinned_info(id: int) -> Dictionary:
	if not _skinned.has(id) or not _unit_info.has(id):
		return {}
	var info: Dictionary = _unit_info[id]
	return {"kind": info["kind"], "variant": info["variant"], "side": info["side"], "material": _materials[id]}


## BV3 : nombre de régiments dessinés en imposteurs (bancs d'essai).
func impostor_regiments() -> int:
	var n := 0
	for id in _imp_layers:
		if (_imp_layers[id] as MultiMeshInstance3D).visible:
			n += 1
	return n


## BV3 : couche d'imposteurs du régiment (créée quand l'atlas est prêt ; null avant).
func _impostor_layer(id: int, side: String, kind: String, variant: int, count: int) -> MultiMeshInstance3D:
	if _imp_layers.has(id):
		return _imp_layers[id]
	var key := BattleImpostors.key_of(side, kind, variant)
	if not impostors.is_ready(key):
		return null
	var mm := MultiMesh.new()
	mm.transform_format = MultiMesh.TRANSFORM_3D
	mm.mesh = BattleImpostors.quad_mesh()
	mm.instance_count = count
	mm.visible_instance_count = 0
	var imp := MultiMeshInstance3D.new()
	imp.name = "Unit%d_%s_impostor" % [id, kind]
	imp.multimesh = mm
	imp.material_override = impostors.make_material(key)
	imp.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	add_child(imp)
	_imp_layers[id] = imp
	return imp


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
## Lot BV2 : mort tirée selon la cause des pertes (`loss_cause`), projection en arrière loin du
## tueur (charge, boulet), démembrement sur coup critique (réglage « complet »), sang, gerbe ;
## cadavres rangés par cellules de terrain (LOD, plafond global `corpses.max_total`).
func _spawn_corpses(unit: Dictionary, side: String, kind: String, variant: int, prev: PackedFloat32Array, count: int) -> void:
	var skinned := BattleSkinned.has_figure(kind, variant)
	var mounted := kind == "cavalry"
	var limits: Dictionary = _gore.get("corpses", {})
	var level := _level
	var cause := str(unit.get("loss_cause", "other")) if bv2_enabled else "other"
	var deaths: Dictionary = _gore.get("deaths", {})
	var death: Dictionary = deaths.get(cause, deaths.get("other", {}))
	var names: Array = death.get(("mounted" if mounted else "foot"), [])
	var impulse: Array = death.get("impulse", [0.0, 0.5])
	var killer := int(unit.get("loss_by", -1))
	var killer_pos: Variant = _unit_pos.get(killer)
	var crit_chance := _critical_chance(cause, killer) * float(death.get("critical", 0.0)) if gore != null and gore.dismember_enabled() else 0.0
	var death_count: int = (BattleSkinned.death_config(kind, variant)["set"] as Array).size() if skinned else 1
	var prev_n := prev.size() / 12
	var sprays: Dictionary = _gore.get("sprays", {})
	for _i in mini(count, int(limits.get("spawn_per_update", 60))):
		var k := mini(int(pow(_rng.randf(), 1.6) * prev_n), prev_n - 1)
		var o := k * 12
		var pos := Vector3(prev[o + 3], prev[o + 7], prev[o + 11])
		var record := PackedFloat32Array()
		record.resize(16)
		for j in 12:
			record[j] = prev[o + j]
		# Clip de mort selon la cause ; figurines rigides : clip unique.
		var clip := str(names[_rng.randi_range(0, names.size() - 1)]) if not names.is_empty() else ""
		var index := BattleSkinned.death_index(kind, variant, clip) if skinned else 0
		if index < 0:
			index = _rng.randi_range(0, death_count - 1)
		var speed := _rng.randf_range(float(impulse[0]), float(impulse[1]))
		# Projeté loin du tueur : la figurine lui fait face, le vol part vers -Z du modèle.
		var away := Vector3(prev[o + 2], 0.0, prev[o + 10]) * -1.0
		if killer_pos != null and speed > 0.3:
			var d: Vector3 = pos - (killer_pos as Vector3)
			d.y = 0.0
			if d.length() > 0.5:
				away = d.normalized()
				var angle := atan2(-away.x, -away.z)
				var c := cos(angle)
				var sn := sin(angle)
				record[0] = c
				record[2] = sn
				record[8] = -sn
				record[10] = c
		var code := 0
		if mounted and clip == "c_fall":
			code = BattleSkinned.CODE_HORSE_FLEES
		elif crit_chance > 0.0 and _rng.randf() < crit_chance:
			var part := _pick_part(mounted)
			if part != "":
				code = int(BattleSkinned.SEVER_PARTS[part][0])
				var height := {"head": 1.6, "arm_r": 1.25, "arm_l": 1.25, "leg_r": 0.45, "leg_l": 0.45}[part] as float
				gore.sever(pos, away, part, height + (1.0 if mounted else 0.0))
				severed_count += 1
		var blood := float(level.get("corpse_blood", 0.0)) * _rng.randf_range(0.5, 0.99)
		record[12] = anim_time
		record[13] = float(index)
		record[14] = speed if skinned else 0.0
		record[15] = float(code) + minf(blood, 0.99)
		_add_corpse(side, kind, variant, skinned, pos, record, limits)
		if gore != null:
			gore.spray(pos, away, int(sprays.get("droplets_per_death", 6)), 2.0 if mounted else 1.2)
		corpse_fallen.emit(pos, side, kind, cause)
		corpse_count += 1


## Chance de coup critique (démembrement) selon la cause et l'arme du tueur.
func _critical_chance(cause: String, killer: int) -> float:
	var chances: Dictionary = _gore.get("critical_chance", {})
	if cause == "ball" or cause == "stone":
		return float(chances.get(cause, 0.0))
	var info: Dictionary = _unit_info.get(killer, {})
	var weapon := str(_gore.get("weapons", {}).get(str(info.get("type", "")), "sword"))
	var chance := float(chances.get(weapon, 0.0))
	if cause == "charge" and float(_charge_mass.get(killer, 0.0)) >= float(_gore.get("heavy_charge_mass", 1.2)):
		chance = maxf(chance, float(chances.get("heavy_charge", 0.0)))
	return chance


func _pick_part(mounted: bool) -> String:
	var weights: Dictionary = _gore.get("pieces", {}).get("mounted" if mounted else "foot", {})
	var total := 0.0
	for part in weights:
		total += float(weights[part])
	var r := _rng.randf() * total
	for part in weights:
		r -= float(weights[part])
		if r <= 0.0:
			return str(part)
	return ""


## Range un cadavre dans la cellule de terrain de `pos` (tampon circulaire par cellule).
func _add_corpse(side: String, kind: String, variant: int, skinned: bool, pos: Vector3, record: PackedFloat32Array, limits: Dictionary) -> void:
	var cell_m := float(limits.get("cell_m", 80.0))
	var cx := int(floor(pos.x / cell_m))
	var cz := int(floor(pos.z / cell_m))
	# Démembrés : couche à part avec la variante `discard` du shader (early-z conservé ailleurs).
	var severed := int(record[15]) >= 1 and int(record[15]) <= 5
	var skey := "%s/%s/%d%s" % [side, kind, variant, "/cut" if severed else ""]
	var key := "%s/%d/%d" % [skey, cx, cz]
	if not _corpse_layers.has(key):
		if not _corpse_materials.has(skey):
			var material := _make_skinned_material(side, kind, variant, true) if skinned else _make_material(side, kind, variant, true)
			if severed:
				material.shader = BattleSkinned.corpse_shader()
			if skinned:
				material.set_shader_parameter("gravity", 9.8)
			_corpse_materials[skey] = material
		var mm := MultiMesh.new()
		mm.transform_format = MultiMesh.TRANSFORM_3D
		mm.use_custom_data = true
		mm.mesh = BattleSkinned.mesh(kind, variant, 1) if skinned else BattleMeshes.soldier(kind, variant, true)
		mm.instance_count = 0
		var instance := MultiMeshInstance3D.new()
		instance.name = "Corpses_%s_%s_%d_%d_%d" % [side, kind, variant, cx, cz]
		instance.multimesh = mm
		instance.material_override = _corpse_materials[skey]
		instance.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
		add_child(instance)
		var center := Vector3((cx + 0.5) * cell_m, pos.y, (cz + 0.5) * cell_m)
		_corpse_layers[key] = {"mm": mm, "data": PackedFloat32Array(), "count": 0, "next": 0, "instance": instance, "center": center, "kind": kind, "variant": variant, "skinned": skinned, "level": 1}
	var layer: Dictionary = _corpse_layers[key]
	var data: PackedFloat32Array = layer["data"]
	var count_now: int = layer["count"]
	var full := count_now >= int(limits.get("max_per_layer", 3000)) or _corpse_total >= int(limits.get("max_total", 14000))
	if not full:
		data.append_array(record)
		layer["count"] = count_now + 1
		_corpse_total += 1
	elif count_now > 0:
		# Plafond atteint : le cadavre remplace un plus ancien de la même cellule.
		var slot: int = int(layer["next"]) % count_now
		for j in 16:
			data[slot * 16 + j] = record[j]
		layer["next"] = (slot + 1) % count_now
	else:
		return
	layer["data"] = data
	# EP1 : envoi au GPU une fois par image et par cellule (`_flush_corpses`), pas à chaque mort.
	_dirty_corpses[key] = true


## EP1 : envoie les cellules de cadavres modifiées pendant l'image (une copie par cellule au lieu
## d'une par mort : des centaines de morts par image dans les très grandes batailles).
func _flush_corpses() -> void:
	for key in _dirty_corpses:
		var layer: Dictionary = _corpse_layers[key]
		var mm: MultiMesh = layer["mm"]
		if mm.instance_count != int(layer["count"]):
			mm.instance_count = int(layer["count"])
		mm.buffer = layer["data"]
	_dirty_corpses.clear()


## LOD des cellules de cadavres : maillage moyen en deçà de `near_m`, lointain au-delà,
## masquées après `far_m` (distance caméra → centre de cellule).
func _update_corpse_lods() -> void:
	var limits: Dictionary = _gore.get("corpses", {})
	var near := float(limits.get("near_m", 60.0))
	var far := float(limits.get("far_m", 380.0))
	var half := float(limits.get("cell_m", 80.0)) * 0.7
	for key in _corpse_layers:
		var layer: Dictionary = _corpse_layers[key]
		var dist := maxf(_camera_pos.distance_to(layer["center"]) - half, 0.0)
		var instance: MultiMeshInstance3D = layer["instance"]
		instance.visible = dist < far
		if not layer["skinned"] or not instance.visible:
			continue
		var level := 1 if dist < near else 2
		if level != int(layer["level"]):
			layer["level"] = level
			(layer["mm"] as MultiMesh).mesh = BattleSkinned.mesh(str(layer["kind"]), int(layer["variant"]), level)


## Lot BV2 : applique les chocs de cavalerie du cœur (renversés projetés qui se relèvent,
## chevaux ralentis ou arrêtés, gerbes, sons).
func apply_impacts(impacts: Array) -> void:
	var knock: Dictionary = _gore.get("knockdown", {})
	for hit in impacts:
		var attacker := int(hit["attacker"])
		var defender := int(hit["defender"])
		var kind := str(hit["kind"])
		_charge_mass[attacker] = float(hit.get("mass", 0.0))
		_slow[attacker] = {"since": anim_time, "kind": kind}
		if kind == "shock":
			var horses: Dictionary = _gore.get("horses", {})
			var h := float(hit.get("heading", 0.0))
			_drive[attacker] = {"since": anim_time, "depth": float(hit.get("depth", 0.0)) + float(horses.get("drive_extra_m", 2.5)), "dir": Vector2(sin(h), cos(h))}
		var point: Vector2 = hit["point"]
		var ground := _unit_pos.get(defender, Vector3(point.x, 0.0, point.y)) as Vector3
		var at := Vector3(point.x, ground.y, point.y)
		if kind == "pikes" or kind == "stakes":
			_play_sound("horse_neigh", at)
		elif kind == "shock":
			_play_sound("shield_bash", at)
		var knocked := int(hit.get("knocked", 0))
		if knocked > 0:
			_knock_down(defender, at, float(hit.get("heading", 0.0)), float(hit.get("mass", 1.0)), knocked, knock)


## Renverse `knocked` soldats (× taille d'unité) du régiment `id` les plus proches du point
## d'impact : figurines de la couche « renversés », place vide dans la formation le temps du clip.
func _knock_down(id: int, at: Vector3, heading: float, mass: float, knocked: int, knock: Dictionary) -> void:
	if not _previous.has(id) or not _unit_info.has(id):
		return
	var info: Dictionary = _unit_info[id]
	var kind := str(info["kind"])
	var variant := int(info["variant"])
	if kind == "cavalry" or not BattleSkinned.has_figure(kind, variant):
		return
	var slice: PackedFloat32Array = _previous[id]
	var n := slice.size() / 12
	var k := mini(mini(int(round(knocked * float(_unit_scale.get(id, 1.0)))), int(knock.get("max_per_impact", 48))), n)
	if k <= 0:
		return
	# Rangs les plus proches du point d'impact ; tirage parmi un peu plus de candidats.
	var order: Array = []
	for i in n:
		var dx := slice[i * 12 + 3] - at.x
		var dz := slice[i * 12 + 11] - at.z
		order.append(Vector2(dx * dx + dz * dz, i))
	order.sort()
	var pool := mini(n, int(k * 1.5) + 1)
	var layer := _tumble_layer(str(info["side"]), kind, variant, int(knock.get("max_active", 600)))
	var mm: MultiMesh = layer["mm"]
	var data: PackedFloat32Array = layer["data"]
	var impulse: Array = knock.get("impulse", [2.0, 4.5])
	var hidden: Dictionary = _hidden.get(id, {})
	var clip_len := BattleSkinned.clip_seconds(kind, variant, "knockdown")
	var level := _level
	var sprays: Dictionary = _gore.get("sprays", {})
	var picks: Array = order.slice(0, pool)
	picks.shuffle()
	# Face au cavalier : le vol part dans le sens de la charge (-Z du modèle).
	var angle := heading + PI
	var c := cos(angle)
	var sn := sin(angle)
	var push := Vector3(sin(heading), 0.0, cos(heading))
	for j in k:
		var slot := int((picks[j] as Vector2).y)
		var o := slot * 12
		var speed := _rng.randf_range(float(impulse[0]), float(impulse[1])) * clampf(0.6 + 0.4 * mass, 0.5, 1.4)
		var fly := 2.0 * (0.35 * speed + 0.6) / 9.8
		var r: int = layer["next"]
		var w := r * 16
		var values := [c, 0.0, sn, slice[o + 3], 0.0, 1.0, 0.0, slice[o + 7], -sn, 0.0, c, slice[o + 11], anim_time + _rng.randf_range(0.0, 0.15), 0.0, speed, minf(float(level.get("corpse_blood", 0.0)) * 0.3, 0.99)]
		for q in 16:
			data[w + q] = values[q]
		layer["next"] = (r + 1) % mm.instance_count
		hidden[slot] = anim_time + fly + clip_len + 0.2
		knocked_count += 1
		if gore != null:
			gore.spray(Vector3(slice[o + 3], slice[o + 7], slice[o + 11]), push, int(sprays.get("droplets_per_knock", 3)), 1.3)
	_hidden[id] = hidden
	layer["data"] = data
	mm.buffer = data


## Couche « renversés » d'un camp, d'une famille et d'une variante (tampon circulaire).
func _tumble_layer(side: String, kind: String, variant: int, capacity: int) -> Dictionary:
	var key := "%s/%s/%d" % [side, kind, variant]
	if _tumble_layers.has(key):
		return _tumble_layers[key]
	var mat := _make_skinned_material(side, kind, variant, true)
	BattleSkinned.apply_config(mat, BattleSkinned.knockdown_config(kind, variant), anim_time)
	mat.set_shader_parameter("tumble", true)
	var mm := MultiMesh.new()
	mm.transform_format = MultiMesh.TRANSFORM_3D
	mm.use_custom_data = true
	mm.mesh = BattleSkinned.mesh(kind, variant, 0)
	mm.instance_count = maxi(capacity / 4, 16)
	var data := PackedFloat32Array()
	data.resize(mm.instance_count * 16)
	data.fill(0.0)
	for i in mm.instance_count:
		data[i * 16 + 12] = -1.0e6
	mm.buffer = data
	var instance := MultiMeshInstance3D.new()
	instance.name = "Knocked_%s_%s_%d" % [side, kind, variant]
	instance.multimesh = mm
	instance.material_override = mat
	instance.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	add_child(instance)
	var layer := {"mm": mm, "data": data, "next": 0, "material": mat}
	_tumble_layers[key] = layer
	return layer


## Places vides des renversés dans la tranche rendue (base nulle = figurine repliée).
func _hide_knocked(id: int, slice: PackedFloat32Array, n: int) -> PackedFloat32Array:
	var hidden: Dictionary = _hidden[id]
	var out := slice
	for slot in hidden.keys():
		if float(hidden[slot]) <= anim_time:
			hidden.erase(slot)
			continue
		if int(slot) < n:
			var o := int(slot) * 12
			for q in [0, 1, 2, 4, 5, 6, 8, 9, 10]:
				out[o + q] = 0.0
	if hidden.is_empty():
		_hidden.erase(id)
	return out


## Piquiers et miliciens qui abaissent leurs armes d'hast devant une charge de cavalerie
## ennemie à moins de 90 m (rendu seulement, lot BV2).
func _find_braced(units: Array) -> void:
	_braced.clear()
	for unit in units:
		if str(unit.get("render", "")) != "cavalry" or str(unit.get("state", "")) != "charging" or not bool(unit.get("present", true)):
			continue
		var at := Vector2(float(unit["x"]), float(unit["z"]))
		for other in units:
			var info: Dictionary = _unit_info.get(int(other["id"]), {})
			if info.is_empty() or str(other["side"]) == str(unit["side"]) or str(info["kind"]) != "infantry" or int(info["variant"]) == 0:
				continue
			if at.distance_to(Vector2(float(other["x"]), float(other["z"]))) < 90.0:
				_braced[int(other["id"])] = true


## Facteur de cadence d'un régiment en marche : vitesse lissée / vitesse nominale du clip de
## locomotion (`cadence` de battle_gore.json, m/s à la vitesse 1) ; 1 hors locomotion.
func _cadence(id: int, config: Dictionary) -> float:
	if int(config.get("mode", 0)) != BattleSkinned.M_LOOP:
		return 1.0
	var names: Array = config.get("names", [])
	var table: Dictionary = _gore.get("cadence", {})
	if names.is_empty() or not table.has(str(names[0])):
		return 1.0
	var nominal := float(table[str(names[0])]) * float(config.get("speed", 1.0))
	return clampf(float(_speed.get(id, nominal)) / maxf(nominal, 0.1), 0.35, 1.8)


## Cavaliers qui entrent dans la masse après un choc (lot BV2) : la formation rendue avance de
## la pénétration du cœur (`depth`) plus l'écart de contact, s'y tient, puis se replie.
func _drive_in(id: int, slice: PackedFloat32Array, n: int) -> PackedFloat32Array:
	var entry: Dictionary = _drive[id]
	var horses: Dictionary = _gore.get("horses", {})
	var t := anim_time - float(entry["since"])
	var t_in := float(horses.get("drive_in_seconds", 0.7))
	var hold := float(horses.get("drive_hold_seconds", 2.5))
	var t_out := float(horses.get("drive_out_seconds", 3.0))
	if t > t_in + hold + t_out:
		_drive.erase(id)
		return slice
	var f := smoothstep(0.0, t_in, t) * (1.0 - smoothstep(t_in + hold, t_in + hold + t_out, t))
	var dir: Vector2 = entry["dir"]
	var d := float(entry["depth"]) * f
	var out := slice
	for i in n:
		out[i * 12 + 3] += dir.x * d
		out[i * 12 + 11] += dir.y * d
	return out


## Chevaux qui ralentissent dans la masse (choc) ou s'arrêtent (piques, pieux) : l'horloge
## d'animation du régiment prend du retard (continu, sans saut de phase).
func _advance_lags(dt: float) -> void:
	if dt <= 0.0 or _slow.is_empty():
		return
	var horses: Dictionary = _gore.get("horses", {})
	for id in _slow.keys():
		var entry: Dictionary = _slow[id]
		var t := anim_time - float(entry["since"])
		var stop := str(entry["kind"]) != "shock"
		var span := float(horses.get("stop_seconds", 1.2)) if stop else float(horses.get("slow_seconds", 2.2))
		if t > span:
			_slow.erase(id)
			continue
		var low := 0.1 if stop else float(horses.get("slow_min", 0.35))
		var factor := 1.0 - (1.0 - low) * sin(PI * clampf(t / span, 0.0, 1.0))
		_lag[id] = float(_lag.get(id, 0.0)) + dt * (1.0 - factor)


## Taches progressives d'un régiment vivant (pertes subies, durée de mêlée) × réglage « Sang ».
func _living_blood(unit: Dictionary, id: int) -> float:
	var stains: Dictionary = _gore.get("living_stains", {})
	var level := float(_level.get("stains", 0.0))
	if level <= 0.0:
		return 0.0
	var initial := maxf(float(unit.get("initial_soldiers", unit["soldiers"])), 1.0)
	var lost := clampf(1.0 - float(unit["soldiers"]) / initial, 0.0, 1.0)
	var melee := float(_melee_time.get(id, 0.0)) / float(stains.get("melee_seconds", 60.0))
	var amount := lost * float(stains.get("per_loss_share", 1.6)) + minf(melee, 1.0) * 0.4
	return minf(amount, float(stains.get("max", 0.85))) * level


## Son d'AU1 (`BattleAudio.play_at`) s'il est présent dans le projet.
func _play_sound(event_name: String, at: Vector3) -> void:
	if _audio != null:
		_audio.call("play_at", event_name, at)


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


## Temps d'animation propagé aux cadavres (chute) et aux renversés ; LOD des cellules de
## cadavres (lot BV2).
func _process(delta: float) -> void:
	if _timing:
		_frame_times.append(delta)
	for key in _corpse_materials:
		(_corpse_materials[key] as ShaderMaterial).set_shader_parameter("anim_time", anim_time)
	for key in _tumble_layers:
		(_tumble_layers[key]["material"] as ShaderMaterial).set_shader_parameter("anim_time", anim_time)
	_update_corpse_lods()
