class_name BattleDuels
extends Node3D

## Duels appariés (lot BV3, audit A1-20), purement cosmétiques et déterministes.
## Le cœur ne modélise pas de duel : aucune perte, aucun moral n'en dépend. Quand un régiment
## de champions (`duels.enabled_for` : le général, chevaliers, hommes d'armes) est en mêlée
## contre sa cible depuis `min_melee_s`, deux figurines au contact (la sienne la plus proche du
## centre ennemi, et l'ennemie la plus proche d'elle) quittent la formation et s'affrontent en
## passes synchronisées : chaque passe donne un clip à chacun (attaque / parade, estoc / coup
## reçu), la dernière met le perdant à terre (il se relève : clip `knockdown`).
## Choix du perdant, des figurines et du moment : fonctions de l'état de la simulation (ids,
## temps de bataille), sans tirage aléatoire. Visible de près (`max_camera_distance_m`).

var active_count: int = 0
var started_count: int = 0
## Dernier duel commencé (captures : cadrer la caméra dessus).
var last_center: Vector3 = Vector3.ZERO

var _cfg: Dictionary = {}
var _melee_since: Dictionary = {}  # unit id -> instant (horloge d'animation) du début de mêlée
var _cooldown: Dictionary = {}  # unit id -> instant avant lequel pas de nouveau duel
var _duels: Array = []  # [{a, b: {inst, mat, track, clips}, start, steps, step}]


func setup() -> void:
	_cfg = BattleStandards.settings().get("duels", {})


## `now` = horloge d'animation des soldats (`BattleSoldiers.anim_time`).
func update(units: Array, soldiers: BattleSoldiers, now: float, camera_pos: Vector3) -> void:
	if _cfg.is_empty():
		return
	_advance(now)
	var by_id := {}
	for unit in units:
		by_id[int(unit["id"])] = unit
	var allowed: Array = _cfg.get("enabled_for", [])
	for unit in units:
		var id := int(unit["id"])
		if not bool(unit["present"]) or str(unit.get("state", "")) != "melee":
			_melee_since.erase(id)
			continue
		if not _melee_since.has(id):
			_melee_since[id] = now
		if _duels.size() >= int(_cfg.get("max_active", 4)):
			continue
		var champion := (bool(unit.get("is_general", false)) and allowed.has("general")) or allowed.has(str(unit.get("type", "")))
		if not champion or now < float(_cooldown.get(id, -1.0)) or now - float(_melee_since[id]) < float(_cfg.get("min_melee_s", 1.5)):
			continue
		var target: Dictionary = by_id.get(int(unit.get("target", -1)), {})
		if target.is_empty() or not bool(target["present"]) or str(target.get("state", "")) != "melee":
			continue
		var center := Vector3(float(target["x"]), float(target.get("y", 0.0)), float(target["z"]))
		if camera_pos.distance_to(center) > float(_cfg.get("max_camera_distance_m", 160.0)):
			continue
		if _start(unit, target, soldiers, now):
			_cooldown[id] = now + float(_cfg.get("cooldown_s", 25.0))
			_cooldown[int(target["id"])] = now + float(_cfg.get("cooldown_s", 25.0)) * 0.5


func _start(unit: Dictionary, target: Dictionary, soldiers: BattleSoldiers, now: float) -> bool:
	var ia := int(unit["id"])
	var ib := int(target["id"])
	var info_a := soldiers.skinned_info(ia)
	var info_b := soldiers.skinned_info(ib)
	if info_a.is_empty() or info_b.is_empty():
		return false
	var enemy_center := Vector3(float(target["x"]), float(target.get("y", 0.0)), float(target["z"]))
	var slot_a := soldiers.figure_slot_near(ia, enemy_center)
	if slot_a < 0:
		return false
	var pos_a := soldiers.slot_frame(ia, slot_a).origin
	var slot_b := soldiers.figure_slot_near(ib, pos_a)
	if slot_b < 0:
		return false
	var pos_b := soldiers.slot_frame(ib, slot_b).origin
	var mounted_a := str(info_a["kind"]) == "cavalry"
	var mounted_b := str(info_b["kind"]) == "cavalry"
	# Perdant déterministe : le champion l'emporte deux fois sur trois.
	var champion_loses := (ia * 31 + ib * 17 + int(now)) % 3 == 0
	var track_a: Array = _cfg.get("mounted_sequence" if mounted_a else "sequence", [])
	var track_b: Array = _cfg.get("mounted_sequence" if mounted_b else "sequence", [])
	var role_a := "b" if champion_loses else "a"
	var role_b := "a" if champion_loses else "b"
	var steps := maxi(track_a.size(), track_b.size())
	var dir := pos_b - pos_a
	dir.y = 0.0
	if dir.length() < 0.01:
		dir = Vector3(sin(float(unit.get("facing", 0.0))), 0, cos(float(unit.get("facing", 0.0))))
	dir = dir.normalized()
	var gap := float(_cfg.get("mounted_gap_m" if mounted_a or mounted_b else "gap_m", 1.3))
	var mid := (pos_a + pos_b) * 0.5
	var duel := {
		"start": now,
		"steps": steps,
		"step": -1,
		"a": _fighter(info_a, _names(track_a, role_a), mid - dir * gap * 0.5, dir),
		"b": _fighter(info_b, _names(track_b, role_b), mid + dir * gap * 0.5, -dir),
	}
	var exchange := float(_cfg.get("exchange_s", 0.9))
	# Fin : dernière passe, plus le temps de se relever pour le perdant à pied.
	var tail := 0.0
	for side in ["a", "b"]:
		var fighter: Dictionary = duel[side]
		var last := str((fighter["names"] as Array)[-1])
		if last == "knockdown":
			tail = maxf(tail, BattleSkinned.clip_seconds(str(fighter["kind"]), int(fighter["variant"]), last))
	duel["end"] = now + exchange * float(steps - 1) + maxf(tail, exchange)
	soldiers.hide_figure(ia, slot_a, float(duel["end"]) + 0.1)
	soldiers.hide_figure(ib, slot_b, float(duel["end"]) + 0.1)
	_duels.append(duel)
	started_count += 1
	active_count = _duels.size()
	last_center = mid
	_advance(now)
	return true


## Noms des clips d'une piste pour le rôle `role` (a : attaquant, b : défenseur).
static func _names(track: Array, role: String) -> Array:
	var names: Array = []
	for exchange in track:
		names.append(str((exchange as Dictionary)[role]))
	return names


## Figurine de duel : une instance en mode CUSTOM (clips de la piste), matériau du régiment.
func _fighter(info: Dictionary, names: Array, pos: Vector3, facing_dir: Vector3) -> Dictionary:
	var kind := str(info["kind"])
	var variant := int(info["variant"])
	var rig_entry := BattleSkinned.rig(kind, variant)
	var unique: Array = []
	for name in names:
		if not unique.has(name):
			unique.append(name)
	var ids: Array[int] = []
	for name in unique:
		ids.append(BattleSkinned.clip_index(rig_entry, str(name)))
	var mat: ShaderMaterial = (info["material"] as ShaderMaterial).duplicate()
	mat.set_meta("v2_config", {})
	BattleSkinned.apply_config(mat, {"key": "duel", "set": ids.slice(0, 4), "mode": BattleSkinned.M_CUSTOM, "speed": 1.0, "cycle": 1.0, "release": 1.0}, 0.0)
	mat.set_shader_parameter("blend_since", -1000.0)
	mat.set_shader_parameter("hide_pavise", false)
	var mm := MultiMesh.new()
	mm.transform_format = MultiMesh.TRANSFORM_3D
	mm.use_custom_data = true
	mm.mesh = BattleSkinned.mesh(kind, variant, 0)
	mm.instance_count = 1
	mm.set_instance_transform(0, Transform3D(Basis(Vector3.UP, atan2(facing_dir.x, facing_dir.z)), pos))
	var inst := MultiMeshInstance3D.new()
	inst.multimesh = mm
	inst.material_override = mat
	add_child(inst)
	return {"inst": inst, "mat": mat, "names": names, "unique": unique, "kind": kind, "variant": variant}


## Passes synchronisées : à chaque passe, les deux figurines repartent au début de leur clip.
func _advance(now: float) -> void:
	var exchange := float(_cfg.get("exchange_s", 0.9))
	for i in range(_duels.size() - 1, -1, -1):
		var duel: Dictionary = _duels[i]
		if now >= float(duel["end"]):
			for side in ["a", "b"]:
				((duel[side] as Dictionary)["inst"] as Node).queue_free()
			_duels.remove_at(i)
			continue
		var step := mini(int((now - float(duel["start"])) / exchange), int(duel["steps"]) - 1)
		for side in ["a", "b"]:
			var fighter: Dictionary = duel[side]
			(fighter["mat"] as ShaderMaterial).set_shader_parameter("anim_time", now)
		if step == int(duel["step"]):
			continue
		duel["step"] = step
		var at := float(duel["start"]) + exchange * float(step)
		for side in ["a", "b"]:
			var fighter: Dictionary = duel[side]
			var names: Array = fighter["names"]
			var name := str(names[mini(step, names.size() - 1)])
			var mm: MultiMesh = (fighter["inst"] as MultiMeshInstance3D).multimesh
			mm.set_instance_custom_data(0, Color(at, float((fighter["unique"] as Array).find(name)), 0.0, 0.0))
	active_count = _duels.size()
