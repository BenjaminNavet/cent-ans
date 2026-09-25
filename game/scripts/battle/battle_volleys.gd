class_name BattleVolleys
extends Node3D

## Volées massives (lot BV1, idée du joueur) : chaque tir d'archers ou d'arbalétriers résolu par
## la simulation (`BattleSim.get_shots()`) devient une vraie volée de centaines à milliers de
## traits. Rendu seulement, aucune règle :
## - traits en vol : MultiMesh de « paquets » de 256 traits (`battle_volley.gdshader`) ; le
##   processeur écrit 16 flottants par paquet, la carte graphique calcule chaque trajectoire à
##   partir de l'instant de tir (hachage entier par trait) ;
## - traits fichés : une partie des traits de chaque volée est confiée à une couche statique qui
##   grandit toute la bataille (plafond `MAX_STUCK`, puis remplacement au hasard), au sol, dans les
##   pavois et dans les pieux ; les autres restent fichés `STICK_HOLD` s puis s'effacent ;
## - pieux des archers (`stakes`) et pavois plantés (`pavise_cover`) dessinés à l'avant des
##   régiments, qui restent sur le champ ;
## - carreaux d'arbalète (plus courts, plus épais, plus tendus) ; flèches enflammées quand le cœur
##   marque la volée `incendiary` (assiégeant capable d'allumer un feu) ;
## - sons : événements nommés de la banque d'AU1 (`sound_event` : lâcher, sifflement au tiers du
##   vol, impact à l'arrivée) relayés par la scène à `BattleAudio` ; aucun lecteur ni bus ici.
## Les touches (`kills` de la volée) renvoient des points d'impact pour le sang (`BattleBlood`).

signal sound_event(event: StringName, position: Vector3, delay: float)

const VOLLEY_SHADER := preload("res://shaders/battle_volley.gdshader")
const STUCK_SHADER := preload("res://shaders/battle_stuck_arrow.gdshader")

const ARROWS_PER_CHUNK := 256
const MAX_CHUNKS := 256
## Traits dessinés par homme simulé et par volée (le cœur tire un trait par homme toutes les
## 6 à 9 s ; la volée visible en montre plusieurs, étalés sur `VOLLEY_STAGGER`).
const ARROWS_PER_MAN := 3.0
const MAX_ARROWS_PER_VOLLEY := 4096
## Au-delà, les volées sont allégées (elles restent visibles de loin comme un nuage).
const FAR_DISTANCE := 700.0
const STICK_HOLD := 8.0
const MAX_STUCK := 30000
## Traits confiés à la couche plantée : au plus `STUCK_PER_VOLLEY` par volée, un sur `STUCK_STRIDE`
## au moins.
const STUCK_PER_VOLLEY := 96
const STUCK_STRIDE := 5
## Doit rester identique à `battle_volley.gdshaderinc` (flèche, carreau).
const SPEED := [48.0, 62.0]
const ARC := [0.16, 0.06]
const STAGGER := [1.5, 0.8]
const MASK32 := 0xFFFFFFFF
const FIELD_AABB := AABB(Vector3(-600, -100, -600), Vector3(2800, 700, 2400))
const MAX_STAKE_ROWS := 60
const MAX_PAVISE_ROWS := 60

var time_now: float = 0.0
## Traits lancés depuis le début (banc d'essai, captures).
var launched: int = 0
var stuck_count: int = 0
## Multiplicateur de taille des unités (réglage, ADR 0016) : plus de figurines, plus de traits.
var figure_scale: float = 1.0

var _height_at: Callable
var _rng := RandomNumberGenerator.new()
var _chunks: MultiMesh
var _chunk_expiry := PackedFloat32Array()
var _chunk_high: int = 0
## Flammes des flèches enflammées : même tampon que les paquets, maillage de flammes seules,
## dessiné seulement tant qu'une volée enflammée vole ou brûle (les volées ordinaires
## n'envoient pas leurs sommets de flamme repliés à la carte graphique).
var _flames: MultiMesh
var _fire_until: float = -1.0
var _seed_counter: int = 1
var _volley_mat: ShaderMaterial
var _stuck: MultiMesh
var _stuck_mat: ShaderMaterial
var _stakes: MultiMesh
var _pavises: MultiMesh
var _stake_rows: int = 0
var _pavise_rows: int = 0
var _planted: Dictionary = {}  # unit id -> {stakes: bool, pavise: Vector2 (dernier plant), rows}


func setup(height_at: Callable) -> void:
	_rng.seed = 90210
	_height_at = height_at
	_volley_mat = ShaderMaterial.new()
	_volley_mat.shader = VOLLEY_SHADER
	_volley_mat.set_shader_parameter("stick_hold", STICK_HOLD)
	_chunks = MultiMesh.new()
	_chunks.transform_format = MultiMesh.TRANSFORM_3D
	_chunks.use_custom_data = true
	_chunks.mesh = _chunk_mesh(false)
	_chunks.instance_count = MAX_CHUNKS
	_chunks.visible_instance_count = 0
	_flames = MultiMesh.new()
	_flames.transform_format = MultiMesh.TRANSFORM_3D
	_flames.use_custom_data = true
	_flames.mesh = _chunk_mesh(true)
	_flames.instance_count = MAX_CHUNKS
	_flames.visible_instance_count = 0
	_chunk_expiry.resize(MAX_CHUNKS)
	_chunk_expiry.fill(-1.0)
	_add_layer("Volleys", _chunks, _volley_mat)
	_add_layer("VolleyFlames", _flames, _volley_mat)
	_stuck_mat = ShaderMaterial.new()
	_stuck_mat.shader = STUCK_SHADER
	_stuck = MultiMesh.new()
	_stuck.transform_format = MultiMesh.TRANSFORM_3D
	_stuck.use_custom_data = true
	_stuck.mesh = _stuck_mesh()
	_stuck.instance_count = MAX_STUCK
	_stuck.visible_instance_count = 0
	_add_layer("StuckArrows", _stuck, _stuck_mat)
	_stakes = _fieldwork_layer("Stakes", _stake_mesh(), MAX_STAKE_ROWS * 64)
	_pavises = _fieldwork_layer("Pavises", _pavise_mesh(), MAX_PAVISE_ROWS * 40)


func chunks_drawn() -> int:
	return _chunk_high


func tick_time(now: float) -> void:
	time_now = now
	_volley_mat.set_shader_parameter("time_now", now)
	_stuck_mat.set_shader_parameter("time_now", now)
	while _chunk_high > 0 and _chunk_expiry[_chunk_high - 1] < now:
		_chunk_high -= 1
	_chunks.visible_instance_count = _chunk_high
	_flames.visible_instance_count = _chunk_high if now < _fire_until else 0


## Pieux et pavois plantés à l'avant des régiments (état de la simulation).
func update_fieldworks(units: Array) -> void:
	for unit in units:
		if not bool(unit.get("present", false)):
			continue
		var id := int(unit["id"])
		var rec: Dictionary = _planted.get(id, {"stakes": false, "pavise": Vector2(INF, INF), "rows": 0})
		_planted[id] = rec
		if bool(unit.get("stakes", false)) and not bool(rec["stakes"]):
			rec["stakes"] = true
			_plant_stakes(unit)
		var still := str(unit.get("state", "")) != "marching" and str(unit.get("state", "")) != "charging"
		if bool(unit.get("pavise_cover", false)) and still and int(rec["rows"]) < 3:
			var here := Vector2(float(unit["x"]), float(unit["z"]))
			var last: Vector2 = rec["pavise"]
			if last.x == INF or here.distance_to(last) > 12.0:
				rec["pavise"] = here
				rec["rows"] = int(rec["rows"]) + 1
				_plant_pavises(unit)


## Volée résolue par le cœur (`get_shots()`), traits et carreaux seulement. Renvoie les points
## d'impact des touches `[{pos, time}]` (sang).
func on_shot(shot: Dictionary, by_id: Dictionary, camera_pos: Vector3) -> Array:
	var kind_key := str(shot.get("kind", "arrow"))
	if kind_key != "arrow" and kind_key != "bolt":
		return []
	var kind := 1 if kind_key == "bolt" else 0
	var shooter: Dictionary = by_id.get(int(shot["shooter"]), {})
	if shooter.is_empty():
		return []
	var from := Vector3(float(shooter["x"]), float(shooter.get("y", 0.0)), float(shooter["z"]))
	var aim2: Vector2 = shot["aim"]
	var target: Dictionary = by_id.get(int(shot.get("target", -1)), {})
	var aim := Vector3(aim2.x, _h(aim2.x, aim2.y), aim2.y)
	var thw := 6.0
	var thd := 1.5
	if not target.is_empty():
		aim.y = float(target.get("y", aim.y))
		thw = float(target.get("width", 12.0)) * 0.5
		thd = float(target.get("depth", 3.0)) * 0.5
	var mid := (from + aim) * 0.5
	var lod := 1.0 if camera_pos.distance_to(mid) < FAR_DISTANCE else 0.3
	var total := int(clampf(float(shot.get("missiles", 1)) * figure_scale * ARROWS_PER_MAN * lod, 1.0, MAX_ARROWS_PER_VOLLEY))
	var cover := 0
	match str(shot.get("cover", "none")):
		"pavise":
			cover = 1
		"stakes":
			cover = 2
	var fire := bool(shot.get("incendiary", false))
	var stride := maxi(STUCK_STRIDE, ceili(float(total) / STUCK_PER_VOLLEY))
	var code := kind + (2 if fire else 0) + 4 * cover + 16 * stride
	var slope := Vector2((_h(aim.x + 3.0, aim.z) - _h(aim.x - 3.0, aim.z)) / 6.0, (_h(aim.x, aim.z + 3.0) - _h(aim.x, aim.z - 3.0)) / 6.0)
	var chunk := {
		"src": from, "tgt": aim, "launch": time_now,
		"shw": float(shooter.get("width", 10.0)) * 0.5, "shd": float(shooter.get("depth", 3.0)) * 0.5,
		"thw": thw, "thd": thd, "slope": slope, "code": code,
	}
	var hits: Array = []
	var kills := float(shot.get("kills", 0.0)) * figure_scale
	var hit_count := mini(int(ceil(kills)) if kills > 0.05 else 0, 16)
	var left := total
	var flight_max := 0.0
	while left > 0:
		var count := mini(left, ARROWS_PER_CHUNK)
		left -= count
		chunk["seed"] = _seed_counter
		chunk["count"] = count
		_seed_counter = (_seed_counter % 16000000) + 1
		var sh := _pcg(int(chunk["seed"]))
		# Flèches confiées à la couche plantée (mêmes positions que dans le shader).
		var i := 0
		while i < count:
			var arrow := arrow_landing(chunk, sh, i)
			flight_max = maxf(flight_max, float(arrow["time"]) - time_now)
			_add_stuck(arrow["pos"], arrow["dir"], float(arrow["time"]), kind, bool(arrow["cover"]))
			i += stride
		var tries := 0
		while hits.size() < hit_count and tries < hit_count * 4:
			tries += 1
			var arrow := arrow_landing(chunk, sh, _rng.randi_range(0, count - 1))
			if not bool(arrow["cover"]):
				hits.append({"pos": arrow["pos"], "time": arrow["time"]})
		_write_chunk(chunk, time_now + flight_max + STAGGER[kind] + STICK_HOLD + 1.0)
		launched += count
	var flight := from.distance_to(aim) / float(SPEED[kind]) * (1.0 + float(ARC[kind]))
	sound_event.emit(&"crossbow_release" if kind == 1 else &"bow_release", from + Vector3(0, 1.5, 0), 0.0)
	sound_event.emit(&"arrow_whistle", from.lerp(aim, 0.55) + Vector3(0, 12, 0), minf(flight * 0.35, 1.5))
	sound_event.emit(&"arrow_impact", aim, flight + STAGGER[kind] * 0.5)
	return hits


## Trajectoire d'une flèche d'un paquet, calculée exactement comme `battle_volley.gdshaderinc`.
## `{pos, dir, time, cover}` au moment où elle se fiche (pointe à la surface).
func arrow_landing(chunk: Dictionary, sh: int, i: int) -> Dictionary:
	var code := int(chunk["code"])
	var kind := code & 1
	var cover := (code >> 2) & 3
	var src: Vector3 = chunk["src"]
	var tgt: Vector3 = chunk["tgt"]
	var d2 := Vector2(tgt.x - src.x, tgt.z - src.z)
	var dir := d2 / maxf(d2.length(), 1.0)
	var perp := Vector2(dir.y, -dir.x)
	var shw := float(chunk["shw"])
	var shd := float(chunk["shd"])
	var thw := float(chunk["thw"])
	var thd := float(chunk["thd"])
	var s_off := perp * ((_rand(sh, i, 0) * 2.0 - 1.0) * shw) + dir * ((_rand(sh, i, 1) * 2.0 - 1.0) * shd)
	var across := (_rand(sh, i, 2) + _rand(sh, i, 3) - 1.0) * (thw + 2.0)
	var along := (_rand(sh, i, 4) + _rand(sh, i, 5) - 1.0) * (thd * 1.6 + 3.0)
	var in_cover := cover > 0 and along < -0.35 * thd
	if in_cover:
		along = -(thd + 0.9)
	var t_off := perp * across + dir * along
	var slope: Vector2 = chunk["slope"]
	var start := Vector3(src.x + s_off.x, src.y + 1.5, src.z + s_off.y)
	var end := Vector3(tgt.x + t_off.x, tgt.y + slope.x * t_off.x + slope.y * t_off.y, tgt.z + t_off.y)
	if in_cover:
		end.y += 0.35 + _rand(sh, i, 8) * 0.8
	else:
		end.y = _h(end.x, end.z)  # la couche plantée suit le vrai sol (le shader, un plan)
	var launch := float(chunk["launch"]) + _rand(sh, i, 6) * float(STAGGER[kind])
	var dist := start.distance_to(end)
	var flight := maxf(dist / float(SPEED[kind]) * (1.0 + float(ARC[kind])), 0.05)
	var arc := dist * float(ARC[kind]) * (0.85 + 0.3 * _rand(sh, i, 7))
	var fly_dir := (end - start + Vector3(0.0, -arc * 4.0, 0.0)).normalized()
	return {"pos": end, "dir": fly_dir, "time": launch + flight, "cover": in_cover}


# --- Paquets -----------------------------------------------------------------------------


func _write_chunk(chunk: Dictionary, expiry: float) -> void:
	var slot := -1
	for s in MAX_CHUNKS:
		if _chunk_expiry[s] < time_now:
			slot = s
			break
	if slot < 0:
		# Plein : on écrase le paquet qui s'achève le plus tôt.
		var best := INF
		for s in MAX_CHUNKS:
			if _chunk_expiry[s] < best:
				best = _chunk_expiry[s]
				slot = s
	var src: Vector3 = chunk["src"]
	var tgt: Vector3 = chunk["tgt"]
	var slope: Vector2 = chunk["slope"]
	var basis := Basis(src, tgt, Vector3(float(chunk["launch"]), float(chunk["shw"]), float(chunk["shd"])))
	var xf := Transform3D(basis, Vector3(float(chunk["thw"]), float(chunk["thd"]), slope.y))
	_chunks.set_instance_transform(slot, xf)
	var custom := Color(float(chunk["seed"]), float(chunk["code"]), float(chunk["count"]), slope.x)
	_chunks.set_instance_custom_data(slot, custom)
	_flames.set_instance_transform(slot, xf)
	_flames.set_instance_custom_data(slot, custom)
	if int(chunk["code"]) & 2:
		_fire_until = maxf(_fire_until, expiry)
	_chunk_expiry[slot] = expiry
	_chunk_high = maxi(_chunk_high, slot + 1)
	_chunks.visible_instance_count = _chunk_high


static func _pcg(v: int) -> int:
	var state := (v * 747796405 + 2891336453) & MASK32
	var word := (((state >> ((state >> 28) + 4)) ^ state) * 277803737) & MASK32
	return ((word >> 22) ^ word) & MASK32


static func _rand(sh: int, i: int, k: int) -> float:
	return float(_pcg((sh + i * 16 + k) & MASK32) >> 8) / 16777216.0


func _h(x: float, z: float) -> float:
	return float(_height_at.call(x, z)) if _height_at.is_valid() else 0.0


# --- Traits fichés ---------------------------------------------------------------------------


func _add_stuck(tip: Vector3, dir: Vector3, appear: float, kind: int, in_cover: bool) -> void:
	var idx := stuck_count
	if stuck_count < MAX_STUCK:
		stuck_count += 1
		_stuck.visible_instance_count = stuck_count
	else:
		idx = _rng.randi_range(0, MAX_STUCK - 1)
	var z := dir.normalized()
	var x := Vector3.UP.cross(z)
	x = x.normalized() if x.length() > 0.01 else Vector3.RIGHT
	var y := z.cross(x)
	# Pointe enfoncée : 25 cm dans la terre, 8 cm dans le bois d'un pavois ou d'un pieu.
	var origin := tip + z * (0.08 if in_cover else 0.25)
	_stuck.set_instance_transform(idx, Transform3D(Basis(x, y, z), origin))
	_stuck.set_instance_custom_data(idx, Color(appear, float(kind), 0.0, 0.0))


# --- Pieux et pavois -------------------------------------------------------------------------


func _plant_stakes(unit: Dictionary) -> void:
	if _stake_rows >= MAX_STAKE_ROWS:
		return
	_stake_rows += 1
	var facing := float(unit.get("facing", 0.0))
	var fwd := Vector3(sin(facing), 0, cos(facing))
	var right := Vector3(cos(facing), 0, -sin(facing))
	var width := float(unit.get("width", 20.0))
	var front := Vector3(float(unit["x"]), 0, float(unit["z"])) + fwd * (float(unit.get("depth", 4.0)) * 0.5 + 1.5)
	var n := mini(int(width / 1.1), 32)
	for row in 2:
		for k in n:
			var lateral := (float(k) - (n - 1) * 0.5) * 1.1 + (0.55 if row == 1 else 0.0)
			var p := front + right * lateral + fwd * (row * 0.8) + right * _rng.randf_range(-0.15, 0.15)
			p.y = _h(p.x, p.z)
			# Pieu pointé vers l'ennemi, planté à ~40° (pointe à hauteur de poitrail).
			var basis := Basis(right, deg_to_rad(50.0 + _rng.randf_range(-6, 6))) * Basis(Vector3.UP, facing)
			_add_fieldwork(_stakes, Transform3D(basis, p), Color(1, 1, 1))


func _plant_pavises(unit: Dictionary) -> void:
	if _pavise_rows >= MAX_PAVISE_ROWS:
		return
	_pavise_rows += 1
	var facing := float(unit.get("facing", 0.0))
	var fwd := Vector3(sin(facing), 0, cos(facing))
	var right := Vector3(cos(facing), 0, -sin(facing))
	var width := float(unit.get("width", 20.0))
	var front := Vector3(float(unit["x"]), 0, float(unit["z"])) + fwd * (float(unit.get("depth", 4.0)) * 0.5 + 0.9)
	var n := mini(int(width / 1.05), 36)
	var tint := Color(0.62, 0.24, 0.2) if str(unit.get("side", "")) == "defender" else Color(0.3, 0.36, 0.62)
	for k in n:
		var p := front + right * ((float(k) - (n - 1) * 0.5) * 1.05) + fwd * _rng.randf_range(-0.12, 0.12)
		p.y = _h(p.x, p.z)
		var basis := Basis(right, deg_to_rad(-12.0 + _rng.randf_range(-3, 3))) * Basis(Vector3.UP, facing + _rng.randf_range(-0.08, 0.08))
		_add_fieldwork(_pavises, Transform3D(basis, p), tint)


func _add_fieldwork(mm: MultiMesh, xf: Transform3D, tint: Color) -> void:
	var idx := mm.visible_instance_count
	if idx >= mm.instance_count:
		return
	mm.set_instance_transform(idx, xf)
	mm.set_instance_color(idx, tint)
	mm.visible_instance_count = idx + 1


# --- Maillages et couches --------------------------------------------------------------------


func _add_layer(node_name: String, mm: MultiMesh, mat: Material) -> void:
	# Boîte fixe aussi sur la ressource : sans elle, chaque écriture d'instance fait recalculer
	# la boîte de toutes les instances (30 000 traits fichés).
	mm.custom_aabb = FIELD_AABB
	var instance := MultiMeshInstance3D.new()
	instance.name = node_name
	instance.multimesh = mm
	instance.material_override = mat
	instance.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	# Les transformées des paquets portent des trajectoires, pas des positions : boîte fixe.
	instance.custom_aabb = FIELD_AABB
	add_child(instance)


func _fieldwork_layer(node_name: String, mesh: Mesh, count: int) -> MultiMesh:
	var mm := MultiMesh.new()
	mm.transform_format = MultiMesh.TRANSFORM_3D
	mm.use_colors = true
	mm.mesh = mesh
	mm.instance_count = count
	mm.visible_instance_count = 0
	mm.custom_aabb = FIELD_AABB
	var mat := StandardMaterial3D.new()
	mat.vertex_color_use_as_albedo = true
	mat.roughness = 0.9
	var instance := MultiMeshInstance3D.new()
	instance.name = node_name
	instance.multimesh = mm
	instance.material_override = mat
	instance.custom_aabb = FIELD_AABB
	add_child(instance)
	return mm


const IRON := Color(0.11, 0.11, 0.12)
const WOOD := Color(0.17, 0.12, 0.07)
const FLETCH := Color(0.22, 0.2, 0.17)


## 256 traits en rubans (le shader les oriente vers la caméra) : pointe, fût, empennage (8
## sommets) ; ou, avec `flames`, 256 panneaux de flamme seuls (4 sommets, repliés sauf flèche
## enflammée). UV.x = partie (0 ruban, 1 flamme), UV2.x = numéro du trait.
static func _chunk_mesh(flames: bool) -> ArrayMesh:
	var st := SurfaceTool.new()
	st.begin(Mesh.PRIMITIVE_TRIANGLES)
	var profile := [[0.0, 0.004, IRON], [-0.06, 0.02, IRON], [-0.62, 0.011, WOOD], [-0.8, 0.034, FLETCH]]
	for i in ARROWS_PER_CHUNK:
		if flames:
			var base := i * 4
			for corner in [Vector2(-0.5, -0.5), Vector2(0.5, -0.5), Vector2(0.5, 0.5), Vector2(-0.5, 0.5)]:
				st.set_uv(Vector2(1, 0))
				st.set_uv2(Vector2(i, 0))
				st.set_color(Color(1, 0.6, 0.2))
				st.set_normal(Vector3.UP)
				st.add_vertex(Vector3(corner.x, corner.y, 0.0))
			for idx in [base, base + 1, base + 2, base, base + 2, base + 3]:
				st.add_index(idx)
			continue
		var first := i * 8
		for p in profile:
			for side in [-1.0, 1.0]:
				st.set_uv(Vector2(0, 0))
				st.set_uv2(Vector2(i, 0))
				st.set_color(p[2])
				st.set_normal(Vector3.UP)
				st.add_vertex(Vector3(side * float(p[1]), 0.0, float(p[0])))
		for seg in 3:
			var a := first + seg * 2
			for idx in [a, a + 1, a + 3, a, a + 3, a + 2]:
				st.add_index(idx)
	var mesh := st.commit()
	mesh.custom_aabb = FIELD_AABB
	return mesh


## Trait fiché : deux rubans en croix (pointe en z = 0, fût vers -Z).
static func _stuck_mesh() -> ArrayMesh:
	var st := SurfaceTool.new()
	st.begin(Mesh.PRIMITIVE_TRIANGLES)
	var profile := [[0.0, 0.004, IRON], [-0.06, 0.016, IRON], [-0.62, 0.009, WOOD], [-0.8, 0.03, FLETCH]]
	var n := 0
	for axis in [Vector3.RIGHT, Vector3.UP]:
		for p in profile:
			for side in [-1.0, 1.0]:
				st.set_color(p[2])
				st.set_normal(Vector3.UP if axis == Vector3.RIGHT else Vector3.RIGHT)
				st.add_vertex(axis * side * float(p[1]) + Vector3(0, 0, float(p[0])))
		for seg in 3:
			var a := n + seg * 2
			for idx in [a, a + 1, a + 3, a, a + 3, a + 2]:
				st.add_index(idx)
		n += 8
	return st.commit()


## Pieu d'archer : perche épointée de 1,8 m, base à l'origine, le long de +Y (inclinée par la
## transformée).
static func _stake_mesh() -> ArrayMesh:
	var st := SurfaceTool.new()
	st.begin(Mesh.PRIMITIVE_TRIANGLES)
	BattleMeshes.add_box(st, Vector3(0, 0.8, 0), Vector3(0.07, 1.6, 0.07), Color(0.42, 0.32, 0.2))
	BattleMeshes.add_box(st, Vector3(0, 1.68, 0), Vector3(0.035, 0.16, 0.035), Color(0.62, 0.52, 0.36))
	return st.commit()


## Pavois planté : planche bombée de 0,6 × 1,15 m (couleur d'instance = camp), béquille.
static func _pavise_mesh() -> ArrayMesh:
	var st := SurfaceTool.new()
	st.begin(Mesh.PRIMITIVE_TRIANGLES)
	BattleMeshes.add_box(st, Vector3(0, 0.6, 0), Vector3(0.6, 1.15, 0.05), Color(0.82, 0.78, 0.7))
	BattleMeshes.add_box(st, Vector3(0, 0.6, 0.03), Vector3(0.14, 1.0, 0.02), Color(0.35, 0.28, 0.18))
	BattleMeshes.add_box(st, Vector3(0, 0.45, -0.3), Vector3(0.04, 0.9, 0.04), Color(0.35, 0.28, 0.18))
	return st.commit()
