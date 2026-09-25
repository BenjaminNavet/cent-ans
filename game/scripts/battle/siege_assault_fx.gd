class_name SiegeAssaultFx
extends Node3D

## SG1 — l'assaut d'une ville mis en scène d'après les événements du cœur
## (`BattleSim.get_siege_events()`) et l'état des régiments (`get_units()`) : échelles dressées
## contre la muraille là où la simulation les pose (`ladder_lines`), beffroi qui abaisse son
## pont-levis en accostant, bélier sous son manteau qui frappe la porte en rythme
## (`ram_strike`, `ram_period`), pierres et boulets des engins avec leur traînée jusqu'au point
## d'impact sur le pan visé (`engine_shot`), éclats de pierre, carreaux des tours de l'enceinte
## (`tower_volley`), huile bouillante (`boiling_oil`), porte qui vole en éclats (`gate_broken`).
## Rendu seulement : aucune règle ici (tout est décidé par `sim-battle`). Les maisons (BR1) et
## l'effondrement des pans (S1) restent dans `battle_siege.gd` / `wall_collapse_fx.gd`.

const LADDER_RAISE := 1.4  # durée du dressage d'une échelle (s)
const BRIDGE_LOWER := 1.6  # durée de la descente du pont-levis (s)
const STONE_ARC := 0.22  # flèche de la trajectoire d'une pierre, × distance
const MAX_FLIGHTS := 12
const CHIP_POOL := 6
const WOOD := Color(0.45, 0.31, 0.18)
const STONE_CHIP := Color(0.62, 0.58, 0.52)
const OIL := Color(0.30, 0.19, 0.07)

var siege_view: BattleSiege
var engines_fx: SiegeEnginesFx  # SG2 : engins animés (point et instant du lâcher)
var marks: SiegeMarksFx  # SG2 : cratères des impacts, huile (vapeur, coulures, taches)
var effects: BattleEffects
var soldiers: BattleSoldiers
var height_at: Callable
var siege: Dictionary = {}
var time_now := 0.0
var wall_height := 8.0
var thickness := 3.0
var ram_period := 3.0

var _by_id: Dictionary = {}
var _ram_snap: Dictionary = {}  # id -> Transform3D imposée (bélier au contact de la porte)
var _ladder_sets: Dictionary = {}  # "id/piece" -> {piece, nodes: [MeshInstance3D], raised: float, dirs: [...]}
var _ladder_meshes: Dictionary = {}  # longueur arrondie (m) -> ArrayMesh
var _rams: Dictionary = {}  # id -> {beam, head, beam_z, head_z, anchor}
var _towers: Dictionary = {}  # id -> {pivot, lowered_at, docked}
var _flights: Array = []  # [{node, trail, start, end, t0, flight, arc, piece, breached, kind}]
var _debris: Array = []  # [{node, vel, spin, until}]
var _stone_chips: Array[GPUParticles3D] = []
var _wood_chips: Array[GPUParticles3D] = []
var _chip_next := {"stone": 0, "wood": 0}
var _oil: GPUParticles3D
var _stone_mesh: SphereMesh
var _ball_mesh: SphereMesh
var _stone_mat: StandardMaterial3D
var _trail_mat: StandardMaterial3D
var _wood_mat: StandardMaterial3D
var _door_shake := 0.0
var _gate_broken := false
var shots_seen := 0  # tests et captures
var strikes_seen := 0
var oils_seen := 0


## Tours de la porte (rendu) : le cœur les centre sur les extrémités du passage de 14 m, où
## leur fût (≈ 6 m de rayon) masquait la porte. Elles sont écartées le long de la courtine pour
## encadrer un passage visible, comme un châtelet ; les autres tours sont inchangées.
static func gatehouse_tower(p_siege: Dictionary, tower: Dictionary) -> Dictionary:
	var pieces: Array = p_siege.get("pieces", [])
	var gate_index := int(p_siege.get("gate", -1))
	if gate_index < 0 or gate_index >= pieces.size():
		return tower
	var gate: Dictionary = pieces[gate_index]
	var a: Vector2 = gate["a"]
	var b: Vector2 = gate["b"]
	var p := Vector2(float(tower["x"]), float(tower["z"]))
	var end := a if p.distance_to(a) < 0.5 else (b if p.distance_to(b) < 0.5 else Vector2.INF)
	if end == Vector2.INF:
		return tower
	var away := (end - (b if end == a else a)).normalized()
	var r := float(tower["radius"])
	var moved := tower.duplicate()
	var q := end + away * (r * 1.15 - 0.6)
	moved["x"] = q.x
	moved["z"] = q.y
	return moved


## Captures et bancs d'essai (`--siege-engines=unit_trebuchet,unit_bombard,…` après `--`) :
## ajoute ces régiments à l'assiégeant de `setup` (types lus dans `data/unit_types/`).
static func add_engines(p_setup: Dictionary, ids: PackedStringArray) -> void:
	var data_dir := ProjectSettings.globalize_path("res://").path_join("../data/unit_types").simplify_path()
	var list: Array = p_setup["attacker"]["units"]
	for id in ids:
		var file := FileAccess.open(data_dir.path_join(id + ".json"), FileAccess.READ)
		if file == null:
			push_warning("SiegeAssaultFx: unknown unit type %s" % id)
			continue
		var t: Dictionary = JSON.parse_string(file.get_as_text())
		var stats := {}
		for key in (t["stats"] as Dictionary):
			stats[key] = int(t["stats"][key])  # JSON : nombres flottants, le cœur attend des entiers
		list.append({
			"unit_type": id, "name": t["name"]["display"], "category": t["category"],
			"mounted": bool(t.get("mounted", false)), "soldiers": int(t["soldiers"]),
			"max_soldiers": int(t["soldiers"]), "morale": int(stats.get("morale", 50)), "experience": 0,
			"stats": stats, "abilities": t.get("abilities", []),
		})


func setup(p_siege_view: BattleSiege, p_effects: BattleEffects, p_soldiers: BattleSoldiers, p_height_at: Callable) -> void:
	siege_view = p_siege_view
	effects = p_effects
	soldiers = p_soldiers
	height_at = p_height_at
	siege = siege_view.siege
	wall_height = siege_view.wall_height
	thickness = siege_view.thickness
	ram_period = float(siege.get("ram_period", 3.0))
	siege_view.external_ladders = true
	if effects != null:
		effects.siege_walls = true
	_stone_mesh = SphereMesh.new()
	# Pierre et boulet grossis (lisibilité de loin : ~2× la taille réelle).
	_stone_mesh.radius = 0.8
	_stone_mesh.height = 1.4
	_stone_mesh.radial_segments = 8
	_stone_mesh.rings = 4
	_ball_mesh = SphereMesh.new()
	_ball_mesh.radius = 0.45
	_ball_mesh.height = 0.9
	_ball_mesh.radial_segments = 8
	_ball_mesh.rings = 4
	_stone_mat = StandardMaterial3D.new()
	_stone_mat.albedo_color = Color(0.42, 0.40, 0.37)
	_stone_mat.roughness = 0.95
	_wood_mat = BattleSiege._textured("wood", Color(0.55, 0.42, 0.3))
	_trail_mat = StandardMaterial3D.new()
	_trail_mat.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA
	_trail_mat.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	_trail_mat.billboard_mode = BaseMaterial3D.BILLBOARD_PARTICLES
	_trail_mat.billboard_keep_scale = true
	_trail_mat.vertex_color_use_as_albedo = true
	_trail_mat.albedo_texture = BattleEffects._puff_texture()
	_trail_mat.albedo_color = Color(0.86, 0.84, 0.8, 0.8)
	for i in CHIP_POOL:
		_stone_chips.append(_chips("StoneChips%d" % i, STONE_CHIP, 0.28))
		_wood_chips.append(_chips("WoodChips%d" % i, WOOD, 0.22))
	_oil = _oil_emitter()
	marks = SiegeMarksFx.new()
	marks.name = "Marks"
	add_child(marks)
	marks.setup(SiegeEnginesFx.settings())


## Chaque image (et à chaque pas de l'avance rapide des captures) : `events` = nouveaux
## événements de siège, `now` = temps d'animation des effets.
func update(events: Array, units: Array, now: float, dt: float) -> void:
	time_now = now
	if BattleAudio.active != null:
		BattleAudio.active.wall_impacts_external = true
	_by_id.clear()
	for unit in units:
		_by_id[int(unit["id"])] = unit
	for event in events:
		_on_event(event)
	_update_ladders(units)
	_update_rams(units)
	_update_towers(units)
	_update_flights()
	_update_debris(dt)
	marks.update(now)
	if _door_shake > 0.0:
		_door_shake = maxf(_door_shake - dt, 0.0)
		_shake_doors(_door_shake)


## Après `BattleSiege.update` (le nœud parent traite son image avant ses enfants) : impose la
## pose du bélier à la porte.
func _process(_delta: float) -> void:
	for id in _ram_snap:
		var machine: Node3D = siege_view._machines.get(id)
		if machine != null and machine.visible:
			machine.transform = _ram_snap[id]


func _on_event(event: Dictionary) -> void:
	match str(event.get("kind", "")):
		"engine_shot":
			_on_engine_shot(event)
		"ram_strike":
			_on_ram_strike(event)
		"tower_volley":
			_on_tower_volley(event)
		"tower_docked":
			var id := int(event["unit"])
			if _towers.has(id):
				_towers[id]["docked"] = true
				_towers[id]["since"] = time_now
		"tower_undocked":
			var id := int(event["unit"])
			if _towers.has(id):
				_towers[id]["docked"] = false
				_towers[id]["since"] = time_now
		"boiling_oil":
			_on_oil(event)
		"gate_broken":
			_on_gate_broken(int(event["piece"]))
		"wall_breached":
			_drop_ladders(int(event["piece"]))
			marks.clear_piece(int(event["piece"]))


# --- Pans et porte ---------------------------------------------------------------------


func _piece(index: int) -> Dictionary:
	var pieces: Array = siege.get("pieces", [])
	return pieces[index] if index >= 0 and index < pieces.size() else {}


func _outward(piece: Dictionary) -> Vector3:
	var a: Vector2 = piece["a"]
	var b: Vector2 = piece["b"]
	var dir := (b - a).normalized()
	var out := Vector2(dir.y, -dir.x)
	var center: Vector2 = siege.get("center", Vector2(600, 560))
	if out.dot((a + b) * 0.5 - center) < 0.0:
		out = -out
	return Vector3(out.x, 0.0, out.y)


func _ground(x: float, z: float) -> float:
	return float(height_at.call(x, z)) if height_at.is_valid() else 0.0


func _gate_front(offset: float = 0.0) -> Vector3:
	var gate := _piece(int(siege.get("gate", -1)))
	if gate.is_empty():
		return Vector3.ZERO
	var mid: Vector2 = ((gate["a"] as Vector2) + (gate["b"] as Vector2)) * 0.5
	var out := _outward(gate)
	var p := Vector3(mid.x, 0.0, mid.y) + out * (thickness * 0.5 + offset)
	p.y = _ground(p.x, p.z)
	return p


# --- Échelles --------------------------------------------------------------------------


## Échelles des régiments qui escaladent : posées sur `ladder_lines` (pied, haut), dressées en
## 1,4 s (décalées d'une échelle à l'autre), laissées contre le mur après l'escalade jusqu'à ce
## que le pan tombe.
func _update_ladders(units: Array) -> void:
	for unit in units:
		if not unit.has("ladder_lines") or not bool(unit["present"]):
			continue
		var lines: PackedVector3Array = unit["ladder_lines"]
		if lines.size() < 2:
			continue
		var key := "%d/%d" % [int(unit["id"]), int(unit.get("climbing", -1))]
		if not _ladder_sets.has(key):
			_ladder_sets[key] = _make_ladder_set(int(unit.get("climbing", -1)), lines)
	for key in _ladder_sets:
		var entry: Dictionary = _ladder_sets[key]
		var nodes: Array = entry["nodes"]
		for k in nodes.size():
			var node: MeshInstance3D = nodes[k]
			var t := clampf((time_now - float(entry["raised"]) - 0.22 * k) / LADDER_RAISE, 0.0, 1.0)
			if t >= 1.0 and bool(node.get_meta("up", false)):
				continue
			node.set_meta("up", t >= 1.0)
			_pose_ladder(node, entry["lines"][k * 2], entry["lines"][k * 2 + 1], _ease_raise(t))


func _make_ladder_set(piece: int, lines: PackedVector3Array) -> Dictionary:
	var nodes: Array = []
	for k in lines.size() / 2:
		var foot := lines[k * 2]
		var top := lines[k * 2 + 1]
		var length := foot.distance_to(top) + 0.9
		var node := MeshInstance3D.new()
		node.mesh = _ladder_mesh(length)
		add_child(node)
		nodes.append(node)
	return {"piece": piece, "nodes": nodes, "raised": time_now, "lines": lines}


func _ladder_mesh(length: float) -> ArrayMesh:
	var key := int(round(length))
	if not _ladder_meshes.has(key):
		_ladder_meshes[key] = BattleSiege._make_ladder(float(key))
	return _ladder_meshes[key]


## Dressage : l'échelle part couchée vers le mur et pivote sur son pied jusqu'au créneau.
func _pose_ladder(node: MeshInstance3D, foot: Vector3, top: Vector3, t: float) -> void:
	var up_dir := (top - foot).normalized()
	var flat := Vector3(up_dir.x, 0.0, up_dir.z).normalized()
	var dir := flat.slerp(up_dir, t).normalized() if t < 1.0 else up_dir
	var across := Vector3(-flat.z, 0.0, flat.x)
	var z_axis := across.cross(dir).normalized()
	node.transform = Transform3D(Basis(across, dir, z_axis), foot + Vector3(0, 0.05, 0))


static func _ease_raise(t: float) -> float:
	return 1.0 - pow(1.0 - t, 2.2)


## Le pan est tombé : ses échelles tombent avec lui.
func _drop_ladders(piece: int) -> void:
	for key in _ladder_sets.keys():
		var entry: Dictionary = _ladder_sets[key]
		if int(entry["piece"]) != piece:
			continue
		for node in entry["nodes"]:
			(node as Node3D).queue_free()
		_ladder_sets.erase(key)


# --- Bélier ----------------------------------------------------------------------------


## Balancement de la poutre sous le manteau : recul lent, frappe brève, calé sur `ram_strike`
## (le coup tombe à la fin de chaque période). Hors de la porte, la poutre est au repos.
func _update_rams(units: Array) -> void:
	var gate := _gate_front(1.0)
	for unit in units:
		if str(unit.get("render", "")) != "ram":
			continue
		var id := int(unit["id"])
		var machine: Node3D = siege_view._machines.get(id)
		if machine == null or machine.get_child_count() < 1:
			continue
		if not _rams.has(id):
			# SG2 : modèle Blender, poutre pendue sous le faîte (`BeamPivot`, balancée) ;
			# sinon bélier procédural de `BattleSiege` (poutre et tête qui coulissent).
			var pivot := machine.find_child("BeamPivot", true, false) as Node3D
			if pivot != null:
				_rams[id] = {"pivot": pivot, "anchor": -1.0}
			elif machine.get_child_count() >= 3:
				var beam := machine.get_child(1) as Node3D
				var head := machine.get_child(2) as Node3D
				_rams[id] = {"beam": beam, "head": head, "beam_z": beam.position.z, "head_z": head.position.z, "anchor": -1.0}
			else:
				continue
		var ram: Dictionary = _rams[id]
		var pos := Vector3(float(unit["x"]), 0.0, float(unit["z"]))
		var at_gate := bool(unit["present"]) and not _gate_broken and Vector2(pos.x, pos.z).distance_to(Vector2(gate.x, gate.z)) < thickness + 8.0
		var offset := 0.0
		if at_gate:
			# La poutre frappe le milieu des vantaux : le manteau est calé face à la porte
			# (la simulation le tient à moins de 4 m du pan, parfois décalé par ses voisins).
			var out := _outward(_piece(int(siege.get("gate", -1))))
			var spot := _gate_front(5.6)
			_ram_snap[id] = Transform3D(Basis(Vector3.UP, atan2(-out.x, -out.z)), spot)
		else:
			_ram_snap.erase(id)
		if at_gate:
			if float(ram["anchor"]) < 0.0:
				ram["anchor"] = time_now
			var phase := fposmod(time_now - float(ram["anchor"]), ram_period) / ram_period
			offset = -1.5 * _swing(phase)
		else:
			ram["anchor"] = -1.0
		if ram.has("pivot"):
			# Pendule : reculer la tête de `offset` m sous des cordes de `beam_drop` m.
			var ram_cfg: Dictionary = SiegeEnginesFx.settings().get("ram", {})
			var drop := float(ram_cfg.get("beam_drop", 1.95))
			var limit := deg_to_rad(float(ram_cfg.get("max_swing_deg", 48.0)))
			(ram["pivot"] as Node3D).rotation.x = clampf(asin(clampf(-offset / drop, -0.95, 0.95)), -limit, limit)
		else:
			(ram["beam"] as Node3D).position.z = float(ram["beam_z"]) + offset
			(ram["head"] as Node3D).position.z = float(ram["head_z"]) + offset


## 0 = au contact de la porte, 1 = poutre ramenée en arrière.
static func _swing(phase: float) -> float:
	if phase < 0.75:
		return smoothstep(0.0, 0.75, phase)
	var k := (phase - 0.75) / 0.25
	return 1.0 - k * k


func _on_ram_strike(event: Dictionary) -> void:
	strikes_seen += 1
	var id := int(event["unit"])
	if _rams.has(id):
		# Recale le balancement : le coup du cœur tombe maintenant.
		_rams[id]["anchor"] = time_now
	var hit := _gate_front(0.3) + Vector3(0, 1.3, 0)
	var out := _outward(_piece(int(event["piece"])))
	_chips_at("wood", hit, out, 0.8)
	if effects != null:
		effects.burst(hit + out * 0.6, "impact", 1.1)
	_door_shake = 0.25


func _shake_doors(amount: float) -> void:
	var gate := int(siege.get("gate", -1))
	if gate < 0 or gate >= siege_view._pieces.size():
		return
	var wall: Node3D = siege_view._pieces[gate]["wall"]
	for door in wall.get_children():
		if str(door.name).begins_with("Door"):
			var node := door as Node3D
			if not node.has_meta("rest_z"):
				node.set_meta("rest_z", node.position.z)
			node.position.z = float(node.get_meta("rest_z")) - 0.25 * amount * sin(amount * 60.0)


# --- Beffrois --------------------------------------------------------------------------


## Pont-levis articulé au sommet du beffroi : relevé en marche, abaissé sur le chemin de ronde
## quand la tour accoste (`tower_docked`), relevé si elle s'en va.
func _update_towers(units: Array) -> void:
	for unit in units:
		if str(unit.get("render", "")) != "tower":
			continue
		var id := int(unit["id"])
		var machine: Node3D = siege_view._machines.get(id)
		if machine == null or machine.get_child_count() < 1:
			continue
		if not _towers.has(id) and machine.find_child("BridgePivot", true, false) != null:
			# SG2 : beffroi modélisé (pont-levis déjà articulé, caisse à la hauteur du mur).
			_towers[id] = {"pivot": machine.find_child("BridgePivot", true, false), "docked": false, "since": -1000.0}
		if not _towers.has(id):
			if machine.get_child_count() < 3:
				continue
			var bridge := machine.get_child(2) as Node3D
			var pivot := Node3D.new()
			pivot.name = "BridgePivot"
			pivot.position = Vector3(0.0, bridge.position.y, 2.4)
			machine.add_child(pivot)
			bridge.reparent(pivot, false)
			bridge.position = Vector3(0.0, 0.0, 1.9)
			_dress_tower(machine)
			_towers[id] = {"pivot": pivot, "docked": false, "since": -1000.0}
		var tower: Dictionary = _towers[id]
		var t := clampf((time_now - float(tower["since"])) / BRIDGE_LOWER, 0.0, 1.0)
		var lowered := t if bool(tower["docked"]) else 1.0 - t
		# Chute amortie : le pont tombe vite puis rebondit un peu sur le rempart.
		var eased := 1.0 - pow(1.0 - lowered, 3.0)
		(tower["pivot"] as Node3D).rotation.x = lerpf(-PI * 0.5, 0.08, eased)


## Beffroi plus lisible : poteaux d'angle, lisses d'étage sombres, plate-forme crénelée de
## planches au sommet (la caisse de `BattleSiege._make_tower` reste en dessous).
func _dress_tower(machine: Node3D) -> void:
	var h := wall_height + 4.0
	var dark := BattleSiege._textured("wood", Color(0.36, 0.26, 0.17))
	var post_box := BoxMesh.new()
	post_box.size = Vector3(0.45, h + 1.6, 0.45)
	for x in [-2.55, 2.55]:
		for z in [-2.55, 2.55]:
			var post := MeshInstance3D.new()
			post.mesh = post_box
			post.material_override = dark
			post.position = Vector3(x, (h + 1.6) * 0.5 + 0.8, z)
			machine.add_child(post)
	var band_box := BoxMesh.new()
	band_box.size = Vector3(5.5, 0.3, 5.5)
	var floors := int(h / 3.2)
	for k in floors:
		var band := MeshInstance3D.new()
		band.mesh = band_box
		band.material_override = dark
		band.position = Vector3(0, 0.8 + (k + 1) * h / float(floors + 1), 0)
		machine.add_child(band)
	var merlon := BoxMesh.new()
	merlon.size = Vector3(0.9, 1.1, 0.25)
	for side in 4:
		for k in 3:
			var m := MeshInstance3D.new()
			m.mesh = merlon
			m.material_override = dark
			var along := -1.8 + k * 1.8
			var basis := Basis(Vector3.UP, side * PI * 0.5)
			m.position = basis * Vector3(along, h + 1.35, 2.6)
			m.basis = basis
			machine.add_child(m)


# --- Engins ----------------------------------------------------------------------------


## Pierre (trébuchet, mangonneau) ou boulet (bombarde) lancé vers le point d'impact que le cœur
## a choisi sur le pan. Le cœur applique les dégâts au tir : le vol est court (1,5-3,5 s).
func _on_engine_shot(event: Dictionary) -> void:
	shots_seen += 1
	var piece := _piece(int(event["piece"]))
	if piece.is_empty():
		return
	var out := _outward(piece)
	var x := float(event["x"])
	var z := float(event["z"])
	var end := Vector3(x, _ground(x, z) + wall_height * float(event.get("height", 0.5)), z) + out * (thickness * 0.5)
	var unit: Dictionary = _by_id.get(int(event["unit"]), {})
	var bombard := str(unit.get("type", "")) == "unit_bombard"
	var breached := bool(event.get("breached", false))
	# SG2 : une pierre par engin animé, lâchée par la fronde (ou la bouche) à l'instant du
	# basculement ; la première frappe le point du cœur, les autres à côté sur le même pan.
	var releases: Array = engines_fx.release(int(event["unit"])) if engines_fx != null and not unit.is_empty() else []
	if releases.is_empty():
		var start := end + out * 150.0
		if not unit.is_empty():
			start = Vector3(float(unit["x"]), float(unit.get("y", 0.0)), float(unit["z"]))
			if soldiers != null:
				var engines := soldiers.soldier_positions(int(unit["id"]), 1)
				if not engines.is_empty():
					start = engines[0]
		var dir := (end - start)
		dir.y = 0.0
		dir = dir.normalized()
		if bombard:
			start += Vector3(0, 0.85, 0) + dir * 1.4
		else:
			start += Vector3(0, 9.0, 0) - dir * 2.0
		releases = [{"pos": start, "dir": dir, "delay": 0.0}]
	var a: Vector2 = piece["a"]
	var b: Vector2 = piece["b"]
	var along := Vector3(b.x - a.x, 0.0, b.y - a.y).normalized()
	var half := a.distance_to(b) * 0.5
	var mid := (a + b) * 0.5
	for k in releases.size():
		var r: Dictionary = releases[k]
		var target := end
		if k > 0:
			# Écart déterministe le long du pan (hachage du tir et du rang), sans quitter le pan.
			var h := fposmod(sin(float(shots_seen) * 12.9898 + float(k) * 78.233) * 43758.5453, 1.0)
			var along_2d := Vector2(along.x, along.z)
			var along_now := Vector2(end.x, end.z).dot(along_2d) - mid.dot(along_2d)
			var shift := (h - 0.5) * 2.0 * minf(8.0, half * 0.8)
			shift = clampf(along_now + shift, -half * 0.85, half * 0.85) - along_now
			target = end + along * shift + Vector3(0, (h - 0.5) * wall_height * 0.3, 0)
		_launch_stone(r["pos"], target, out, time_now + float(r["delay"]), bombard, breached and k == 0, r.get("dir", Vector3.ZERO), int(event["piece"]))


## Pierre ou boulet lancé de `start` à l'instant `t0` (invisible avant), vol raccourci du délai
## de lâcher pour que l'impact reste proche du tir du cœur (les dégâts sont déjà appliqués).
func _launch_stone(start: Vector3, end: Vector3, out: Vector3, t0: float, bombard: bool, breached: bool, dir: Vector3, piece: int) -> void:
	var distance := start.distance_to(end)
	var delay := maxf(t0 - time_now, 0.0)
	var flight := clampf(distance / (140.0 if bombard else 60.0), 0.6 if bombard else 1.5, 3.5)
	flight = maxf(flight - delay, 0.5 if bombard else 1.0)
	if _flights.size() >= MAX_FLIGHTS:
		_finish_flight(_flights.pop_front())
	var node := MeshInstance3D.new()
	node.mesh = _ball_mesh if bombard else _stone_mesh
	node.material_override = _stone_mat
	node.position = start
	node.visible = false
	add_child(node)
	var trail := _trail_emitter(bombard)
	trail.emitting = false
	node.add_child(trail)
	if dir == Vector3.ZERO:
		dir = (end - start).normalized()
	var entry := {"node": node, "trail": trail, "start": start, "end": end, "t0": t0, "flight": flight, "arc": 0.0 if bombard else distance * STONE_ARC, "out": out, "breached": breached, "bombard": bombard, "dir": dir, "launched": false, "piece": piece}
	_flights.append(entry)
	if delay <= 0.0:
		_on_launch(entry)
	BattleAudio.play_at_delayed("stone_impact", end, delay + flight)


## Départ effectif d'un projectile : éclair et fumée à la bouche d'une bombarde.
func _on_launch(f: Dictionary) -> void:
	f["launched"] = true
	(f["node"] as Node3D).visible = true
	(f["trail"] as GPUParticles3D).emitting = true
	if bool(f["bombard"]) and effects != null:
		effects.cannon_fire(f["start"], f["dir"], time_now)


func _update_flights() -> void:
	var landed: Array = []
	for f in _flights:
		var t := (time_now - float(f["t0"])) / float(f["flight"])
		var node: MeshInstance3D = f["node"]
		if t < 0.0:
			continue
		if not bool(f["launched"]):
			_on_launch(f)
		if t >= 1.0:
			landed.append(f)
			continue
		var p: Vector3 = (f["start"] as Vector3).lerp(f["end"], t)
		p.y += float(f["arc"]) * 4.0 * t * (1.0 - t)
		node.position = p
		node.rotation = Vector3(t * 9.0, t * 5.0, 0.0)
	for f in landed:
		_flights.erase(f)
		_finish_flight(f)


func _finish_flight(f: Dictionary) -> void:
	var end: Vector3 = f["end"]
	var out: Vector3 = f["out"]
	var node: MeshInstance3D = f["node"]
	# La traînée survit au projectile le temps de se dissiper.
	var trail: GPUParticles3D = f["trail"]
	trail.emitting = false
	trail.reparent(self)
	get_tree().create_timer(2.0).timeout.connect(trail.queue_free)
	node.queue_free()
	_chips_at("stone", end, out, 1.4 if bool(f["breached"]) else 1.0)
	if int(f.get("piece", -1)) >= 0 and not bool(f["breached"]):
		marks.mark_impact(int(f["piece"]), end, out, false)
	if effects != null:
		effects.burst(end + out * 1.2, "impact", 2.4 if bool(f["breached"]) else 1.6)
		effects.burst(end + out * 0.5 + Vector3(0, -2.0, 0), "smoke", 1.4)


func _trail_emitter(bombard: bool) -> GPUParticles3D:
	var particles := GPUParticles3D.new()
	particles.amount = 160
	particles.lifetime = 2.2 if not bombard else 1.0
	particles.local_coords = false
	particles.fixed_fps = 0
	var process := ParticleProcessMaterial.new()
	process.gravity = Vector3(0, 0.6, 0)
	process.initial_velocity_min = 0.0
	process.initial_velocity_max = 0.4
	process.scale_min = 0.7
	process.scale_max = 1.2
	var curve := Curve.new()
	curve.add_point(Vector2(0.0, 0.5))
	curve.add_point(Vector2(1.0, 1.8))
	var scale_tex := CurveTexture.new()
	scale_tex.curve = curve
	process.scale_curve = scale_tex
	var ramp := Gradient.new()
	ramp.set_color(0, Color(1, 1, 1, 0.7))
	ramp.set_color(1, Color(1, 1, 1, 0.0))
	var ramp_tex := GradientTexture1D.new()
	ramp_tex.gradient = ramp
	process.color_ramp = ramp_tex
	particles.process_material = process
	var quad := QuadMesh.new()
	# Boulet rapide (≈ 2 m par image) : bouffées plus grosses pour qu'elles se chevauchent.
	quad.size = Vector2(2.4, 2.4) if not bombard else Vector2(3.2, 3.2)
	quad.material = _trail_mat
	particles.draw_pass_1 = quad
	particles.emitting = true
	return particles


## Éclats (pierre ou bois) projetés hors du mur au point d'impact.
func _chips(node_name: String, color: Color, size: float) -> GPUParticles3D:
	var particles := GPUParticles3D.new()
	particles.name = node_name
	particles.amount = 28
	particles.lifetime = 1.8
	particles.one_shot = true
	particles.explosiveness = 0.95
	particles.emitting = false
	particles.local_coords = false
	var process := ParticleProcessMaterial.new()
	process.direction = Vector3(0, 0.5, 1)
	process.spread = 55.0
	process.initial_velocity_min = 3.0
	process.initial_velocity_max = 9.0
	process.gravity = Vector3(0, -9.8, 0)
	process.angular_velocity_min = -540.0
	process.angular_velocity_max = 540.0
	process.scale_min = 0.5
	process.scale_max = 1.4
	process.collision_mode = ParticleProcessMaterial.COLLISION_DISABLED
	particles.process_material = process
	var box := BoxMesh.new()
	box.size = Vector3(size, size * 0.7, size * 0.8)
	var mat := StandardMaterial3D.new()
	mat.albedo_color = color
	mat.roughness = 1.0
	box.material = mat
	particles.draw_pass_1 = box
	particles.visibility_aabb = AABB(Vector3(-12, -12, -12), Vector3(24, 24, 24))
	add_child(particles)
	return particles


func _chips_at(kind: String, pos: Vector3, out: Vector3, scale: float) -> void:
	var pool: Array[GPUParticles3D] = _stone_chips if kind == "stone" else _wood_chips
	var i: int = _chip_next[kind]
	_chip_next[kind] = (i + 1) % pool.size()
	var particles := pool[i]
	# Axe +Z local = normale extérieure du mur.
	particles.transform = Transform3D(Basis.looking_at(-out, Vector3.UP).scaled(Vector3.ONE * scale), pos)
	particles.restart()
	particles.emitting = true


# --- Tours de l'enceinte ---------------------------------------------------------------


func _on_tower_volley(event: Dictionary) -> void:
	if effects == null:
		return
	var towers: Array = siege.get("towers", [])
	var k := int(event["tower"])
	var target: Dictionary = _by_id.get(int(event["target"]), {})
	if k < 0 or k >= towers.size() or target.is_empty():
		return
	var tower: Dictionary = towers[k]
	var top := Vector3(float(tower["x"]), _ground(float(tower["x"]), float(tower["z"])) + float(tower["height"]) + 0.6, float(tower["z"]))
	var starts := PackedVector3Array()
	var r := float(tower["radius"])
	for i in 5:
		var a := TAU * float(i) / 5.0
		starts.append(top + Vector3(cos(a) * r * 0.7, 0.0, sin(a) * r * 0.7))
	var aim := Vector3(float(target["x"]), float(target.get("y", 0.0)), float(target["z"]))
	effects.volley(starts, aim, maxf(float(target.get("width", 10.0)) * 0.4, 4.0), BattleEffects.BOLT, time_now)
	BattleAudio.play_at("crossbow_release", top)


# --- Huile bouillante ------------------------------------------------------------------


func _oil_emitter() -> GPUParticles3D:
	var particles := GPUParticles3D.new()
	particles.name = "BoilingOil"
	particles.amount = 160
	particles.lifetime = 1.1
	particles.one_shot = true
	particles.explosiveness = 0.15
	particles.emitting = false
	particles.local_coords = false
	var process := ParticleProcessMaterial.new()
	process.emission_shape = ParticleProcessMaterial.EMISSION_SHAPE_BOX
	process.emission_box_extents = Vector3(3.5, 0.2, 0.4)
	process.direction = Vector3(0, -1, 0)
	process.spread = 6.0
	process.initial_velocity_min = 1.0
	process.initial_velocity_max = 2.5
	process.gravity = Vector3(0, -9.8, 0)
	process.scale_min = 0.6
	process.scale_max = 1.3
	particles.process_material = process
	var quad := QuadMesh.new()
	quad.size = Vector2(0.18, 0.7)
	var mat := StandardMaterial3D.new()
	mat.albedo_color = OIL
	mat.metallic_specular = 0.6
	mat.roughness = 0.3
	mat.emission_enabled = true
	mat.emission = Color(0.25, 0.1, 0.02)
	mat.billboard_mode = BaseMaterial3D.BILLBOARD_FIXED_Y
	mat.billboard_keep_scale = true
	quad.material = mat
	particles.draw_pass_1 = quad
	particles.visibility_aabb = AABB(Vector3(-8, -14, -8), Vector3(16, 18, 16))
	add_child(particles)
	return particles


func _on_oil(event: Dictionary) -> void:
	oils_seen += 1
	var gate := _piece(int(event["piece"]))
	if gate.is_empty():
		return
	var out := _outward(gate)
	var mid := _gate_front(0.6)
	var top := mid + Vector3(0, wall_height + 0.2, 0)
	_oil.transform = Transform3D(Basis.looking_at(-out, Vector3.UP), top)
	_oil.restart()
	_oil.emitting = true
	var x := float(event["x"])
	var z := float(event["z"])
	var ground := Vector3(x, _ground(x, z) + 0.3, z)
	if effects != null:
		# Vapeur et fumée âcre au pied de la porte, après la chute.
		get_tree().create_timer(0.9).timeout.connect(func() -> void:
			if is_instance_valid(effects):
				effects.burst(ground, "smoke", 2.2)
				effects.burst(ground + out * 2.0, "smoke", 1.6))
	BattleAudio.play_at_delayed("death_groan", ground, 1.0)
	var a: Vector2 = gate["a"]
	var b: Vector2 = gate["b"]
	marks.pour_oil(top, ground, out, a.distance_to(b), wall_height)


# --- Porte enfoncée --------------------------------------------------------------------


## La porte vole en éclats : planches projetées vers l'intérieur, un vantail arraché resté
## pendu de travers, poussière et échardes.
func _on_gate_broken(piece_index: int) -> void:
	if _gate_broken:
		return
	_gate_broken = true
	var gate := _piece(piece_index)
	if gate.is_empty():
		return
	var a: Vector2 = gate["a"]
	var b: Vector2 = gate["b"]
	var out := _outward(gate)
	var along := Vector3(b.x - a.x, 0.0, b.y - a.y).normalized()
	var front := _gate_front(0.0)
	var rng := RandomNumberGenerator.new()
	rng.seed = 1356
	var width := a.distance_to(b)
	for i in 14:
		var plank := MeshInstance3D.new()
		var box := BoxMesh.new()
		box.size = Vector3(rng.randf_range(0.3, 0.5), rng.randf_range(1.6, 3.4), 0.14)
		plank.mesh = box
		plank.material_override = _wood_mat
		plank.position = front + along * rng.randf_range(-width * 0.3, width * 0.3) + Vector3(0, rng.randf_range(1.0, 4.0), 0)
		plank.rotation = Vector3(rng.randf() * 0.4, rng.randf() * TAU, rng.randf() * 0.4)
		add_child(plank)
		var vel := -out * rng.randf_range(3.0, 8.0) + along * rng.randf_range(-3.0, 3.0) + Vector3(0, rng.randf_range(2.0, 6.0), 0)
		_debris.append({"node": plank, "vel": vel, "spin": Vector3(rng.randf_range(-6, 6), rng.randf_range(-4, 4), rng.randf_range(-6, 6)), "until": time_now + 3.0})
	# Vantail arraché, pendu à un gond, couché vers l'intérieur.
	var leaf := MeshInstance3D.new()
	var leaf_box := BoxMesh.new()
	leaf_box.size = Vector3(width * 0.5 - 0.2, 5.0, 0.5)
	leaf.mesh = leaf_box
	leaf.material_override = _wood_mat
	var hinge := front - out * (thickness + 0.4) + along * (width * 0.5 - 0.3)
	var leaf_basis := Basis.looking_at(-out, Vector3.UP) * Basis(Vector3.UP, 1.1) * Basis(Vector3.FORWARD, 0.18)
	leaf.transform = Transform3D(leaf_basis, hinge + leaf_basis * Vector3(-(width * 0.25), 2.4, 0.0))
	add_child(leaf)
	_chips_at("wood", front + Vector3(0, 2.0, 0), -out, 1.8)
	_chips_at("wood", front + Vector3(0, 3.5, 0), out, 1.4)
	if effects != null:
		effects.burst(front - out * 2.0 + Vector3(0, 1.0, 0), "impact", 3.0)
		effects.burst(front + out * 2.0, "impact", 2.2)


func _update_debris(dt: float) -> void:
	if _debris.is_empty() or dt <= 0.0:
		return
	var done: Array = []
	for d in _debris:
		var node: Node3D = d["node"]
		var vel: Vector3 = d["vel"]
		vel.y -= 9.8 * dt
		node.position += vel * dt
		node.rotation += (d["spin"] as Vector3) * dt
		var floor_y := _ground(node.position.x, node.position.z) + 0.1
		if node.position.y <= floor_y:
			node.position.y = floor_y
			vel *= 0.35
			vel.y = absf(vel.y) * 0.3
			d["spin"] = (d["spin"] as Vector3) * 0.4
		d["vel"] = vel
		if time_now > float(d["until"]):
			node.position.y = floor_y
			node.rotation = Vector3(PI * 0.5, node.rotation.y, 0.0)
			done.append(d)
	for d in done:
		_debris.erase(d)
