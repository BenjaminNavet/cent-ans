class_name BattlePathPreview
extends Node3D

## CB-M2 : aperçu du trajet (spec CB-M, « Aperçu du trajet »). Rendu seulement : les chemins
## viennent du cœur (`BattleSim.preview_paths` / `preview_path`, le même calcul que l'ordre réel,
## sans rien changer à la bataille).
## - Pendant le clic droit maintenu (ou le glisser-droit) : pointillés au sol aux couleurs du camp
##   depuis chaque régiment sélectionné jusqu'à sa place d'arrivée, fantôme de la formation
##   d'arrivée (décale de contour CB-M1 en semi-transparence) ; au-delà de
##   `hover_preview_max_paths` régiments, un seul trajet depuis le centre du groupe (le cœur en
##   décide) ; destination inaccessible : trajet rouge (le curseur passe à `forbidden`).
## - Recalcul seulement si le point visé a bougé de plus de `hover_preview_recompute_m` mètres,
##   au plus `hover_preview_max_per_s` fois par seconde (`data/rules/battle_hover.json`, lus par
##   `RuleValues`).
## - Après l'ordre, tant que le régiment reste sélectionné : trajet et fantôme vers sa
##   `destination`, flèche d'attaque vers sa `target` (lus dans `get_units`).
## - CB-M3 : ordres en file (`get_units()[i].queue`) : trajet segment par segment depuis le
##   dernier point (`preview_path_from`), flèche rouge pour une attaque en file, points de passage
##   numérotés au sol, fantôme au dernier point. Avec Maj, l'aperçu en direct part du dernier
##   point de la file (`preview_paths_queued`).
## - CB1 : glisser-droit = largeur du front : le cœur renvoie pour chaque régiment la largeur et
##   la profondeur prises à l'arrivée (fantôme à cette taille) ; groupe verrouillé : trajets vers
##   ses places rigides (`compute_places`) ; déploiement : fantômes seuls (`show_ghosts`).

const LIFT := 0.6  # m au-dessus du sol
const DASH := 3.0  # m
const GAP := 2.0  # m
const WIDTH := 1.1  # m
const ARROW_HEAD := 7.0  # m
const RED := Color(0.9, 0.12, 0.08, 0.95)
const GHOST_ALPHA := 0.45
## Trajets des ordres donnés : recalculés au plus toutes les `ORDER_REFRESH_S` secondes.
const ORDER_REFRESH_S := 0.25

var battle: Object = null
var _height_at: Callable
var _color := Color(0.9, 0.8, 0.3, 0.9)

var recompute_distance := 5.0
var min_interval := 0.1
## Nombre d'appels au cœur pour l'aperçu en direct (tests d'étranglement).
var recompute_count := 0
## Dernier résultat du cœur : [{unit, ok, path, reason}].
var legs: Array = []
var live_active := false

var _last_point := Vector3(INF, INF, INF)
var _last_facing := NAN
var _last_width := 0.0
var _last_time := -INF
var _live_mesh: MeshInstance3D
var _orders_mesh: MeshInstance3D
var _live_ghosts: Array[Decal] = []
var _order_ghosts: Array[Decal] = []
var _orders_time := -INF
var _orders_key := ""
## CB-M3 : aperçu en direct d'un ordre en file (Maj) ; numéros des points de passage.
var live_queued := false
var _order_labels: Array[Label3D] = []
var _segment_cache: Dictionary = {}  # "id:fx,fz>tx,tz" -> PackedVector3Array


func setup(p_battle: Object, height_at: Callable, color: Color) -> void:
	name = "PathPreview"
	battle = p_battle
	_height_at = height_at
	_color = Color(color.lerp(Color.WHITE, 0.3), 0.9)
	recompute_distance = RuleValues.value("hover_preview_recompute_m", 5.0)
	min_interval = 1.0 / maxf(1.0, RuleValues.value("hover_preview_max_per_s", 10.0))
	_live_mesh = _make_mesh("LivePath")
	_orders_mesh = _make_mesh("OrderPaths")


## Étranglement : vrai si le point visé a assez bougé (ou si l'orientation a changé) et que
## l'intervalle minimal est écoulé depuis le dernier calcul.
func should_recompute(point: Vector3, facing: float, now_s: float, queued: bool = false, width: float = 0.0) -> bool:
	if queued != live_queued and live_active:
		return true  # Maj pressée ou relâchée : l'aperçu change de départ
	if now_s - _last_time < min_interval:
		return false
	var moved := Vector2(point.x - _last_point.x, point.z - _last_point.z).length() > recompute_distance
	var turned := is_finite(facing) != is_finite(_last_facing) or (is_finite(facing) and absf(angle_difference(facing, _last_facing)) > 0.05)
	# CB1 : la largeur du glisser change la forme d'arrivée.
	var widened := absf(width - _last_width) > 1.0
	return moved or turned or widened or not live_active


## Aperçu en direct vers `point` (orientation `facing`, NaN sinon) pour `ids` ; recalculé seulement
## si l'étranglement le permet. Renvoie vrai si le cœur a été interrogé.
func request(ids: Array, units: Array, point: Vector3, facing: float, now_s: float, queued: bool = false, width: float = 0.0) -> bool:
	if not should_recompute(point, facing, now_s, queued, width):
		return false
	compute(ids, units, point, facing, now_s, queued, width)
	return true


## Interroge le cœur sans étranglement (lâcher du clic : l'ordre suit ce verdict). `queued` (Maj) :
## chaque trajet part du dernier point de la file du régiment (CB-M3).
## CB1 : `width` (> 0) = largeur du glisser-droit (répartie par le cœur).
func compute(ids: Array, units: Array, point: Vector3, facing: float, now_s: float, queued: bool = false, width: float = 0.0) -> Array:
	recompute_count += 1
	_last_point = point
	_last_facing = facing
	_last_width = width
	_last_time = now_s
	live_active = true
	live_queued = queued
	var query := "preview_paths_queued" if queued else "preview_paths"
	legs = battle.call(query, PackedInt32Array(ids), point.x, point.z, facing, width) if battle != null else []
	_hide_orders()
	_draw_live(units, point, facing)
	return legs


## CB1 : aperçu d'un groupe verrouillé : un trajet par régiment vers sa place rigide `places`
## ({id, x, z, facing}), depuis sa position ou, `queued`, depuis la fin de sa file. `point` et
## `facing` servent à l'étranglement (centre et orientation du groupe).
func compute_places(places: Array, units: Array, now_s: float, queued: bool = false, point: Vector3 = Vector3.ZERO, facing: float = NAN) -> Array:
	recompute_count += 1
	_last_point = point
	_last_facing = facing
	_last_width = 0.0
	_last_time = now_s
	live_active = true
	live_queued = queued
	legs = []
	for place in places:
		var id := int(place["id"])
		var path := PackedVector3Array()
		if battle != null:
			var from := queue_end(_find(units, id)) if queued else Vector2(INF, INF)
			if is_finite(from.x):
				path = battle.call("preview_path_from", id, from.x, from.y, float(place["x"]), float(place["z"]))
			else:
				path = battle.call("preview_path", id, float(place["x"]), float(place["z"]))
		var ok := path.size() >= 2
		legs.append({"unit": id, "ok": ok, "path": path, "reason": "" if ok else "Aucun chemin jusque-là", "facing": float(place["facing"])})
	_hide_orders()
	_draw_live(units, point, facing)
	return legs


## CB-M3 / CB1 : point où la file d'ordres d'une unité la laisse (dernier point de la file, sinon
## sa destination) ; (INF, INF) sans ordre en cours.
static func queue_end(unit: Dictionary) -> Vector2:
	var queue: Array = unit.get("queue", [])
	for k in range(queue.size() - 1, -1, -1):
		if not (queue[k] as Dictionary).has("target"):
			return Vector2(float(queue[k]["x"]), float(queue[k]["z"]))
	if unit.has("destination"):
		return unit["destination"]
	return Vector2(INF, INF)


## CB1 : fantômes seuls (déploiement : les régiments sont posés, pas menés) aux places `places`
## ({id, x, z, facing, width, depth}).
func show_ghosts(places: Array, units: Array) -> void:
	live_active = true
	legs = []
	(_live_mesh.mesh as ImmediateMesh).clear_surfaces()
	_live_mesh.visible = false
	var used := 0
	for place in places:
		var unit := _find(units, int(place["id"]))
		if unit.is_empty():
			continue
		var at := Vector3(float(place["x"]), float(unit["y"]), float(place["z"]))
		at.y = _ground(at.x, at.z, at.y) - LIFT
		_ghost(_live_ghosts, used, unit, at, float(place["facing"]), _color, Vector2(float(place.get("width", 0.0)), float(place.get("depth", 0.0))))
		used += 1
	for k in range(used, _live_ghosts.size()):
		_live_ghosts[k].visible = false


## Vrai si au moins un régiment a un chemin (sinon l'ordre n'est pas envoyé).
func reachable() -> bool:
	for leg in legs:
		if bool(leg["ok"]):
			return true
	return legs.is_empty()


## Raison française du refus (premier trajet vide), "" sinon.
func refusal() -> String:
	for leg in legs:
		if not bool(leg["ok"]):
			return str(leg["reason"])
	return ""


func _hide_orders() -> void:
	_orders_key = ""  # redessin dès la fin de l'aperçu
	(_orders_mesh.mesh as ImmediateMesh).clear_surfaces()
	_orders_mesh.visible = false
	for ghost in _order_ghosts:
		ghost.visible = false
	for label in _order_labels:
		label.visible = false


func clear_live() -> void:
	live_active = false
	live_queued = false
	legs = []
	_last_width = 0.0
	_last_point = Vector3(INF, INF, INF)
	_last_time = -INF
	(_live_mesh.mesh as ImmediateMesh).clear_surfaces()
	_live_mesh.visible = false
	for ghost in _live_ghosts:
		ghost.visible = false


## Trajets, fantômes et flèches d'attaque des ordres en cours des régiments `selected`.
## Masqués pendant l'aperçu en direct : l'ordre préparé remplace l'ordre en cours.
func update_orders(units: Array, selected: Array, now_s: float) -> void:
	if live_active:
		_hide_orders()
		return
	var key := str(selected)
	if key == _orders_key and now_s - _orders_time < ORDER_REFRESH_S:
		return
	_orders_key = key
	_orders_time = now_s
	var mesh := _orders_mesh.mesh as ImmediateMesh
	mesh.clear_surfaces()
	var used := 0
	var labels := 0
	var drawn := false
	var max_paths := int(RuleValues.value("hover_preview_max_paths", 6.0))
	var shown := 0
	for unit in units:
		if not selected.has(int(unit["id"])) or not bool(unit["present"]) or str(unit["state"]) == "routing":
			continue
		if shown >= max_paths:
			break
		shown += 1
		var id := int(unit["id"])
		var queue: Array = unit.get("queue", [])
		var here := Vector3(float(unit["x"]), float(unit["y"]), float(unit["z"]))
		# Point d'où part le premier ordre en file (fin de l'ordre en cours).
		var last := here
		var last_path := PackedVector3Array()
		var target := int(unit.get("target", -1))
		if target >= 0:
			var foe := _find(units, target)
			if not foe.is_empty():
				if not drawn:
					mesh.surface_begin(Mesh.PRIMITIVE_TRIANGLES)
					drawn = true
				var to := Vector3(float(foe["x"]), float(foe["y"]), float(foe["z"]))
				_dashes(mesh, PackedVector3Array([here, to]), RED, true)
				last = to
		elif unit.has("destination"):
			var dest: Vector2 = unit["destination"]
			var path: PackedVector3Array = battle.call("preview_path", id, dest.x, dest.y) if battle != null else PackedVector3Array()
			if path.size() >= 2:
				if not drawn:
					mesh.surface_begin(Mesh.PRIMITIVE_TRIANGLES)
					drawn = true
				_dashes(mesh, path, _color, false)
				last = path[path.size() - 1]
				last_path = path
		if queue.is_empty():
			if last_path.size() >= 2:
				_ghost(_order_ghosts, used, unit, last, _end_facing(last_path, NAN), _color)
				used += 1
			continue
		# CB-M3 : ordres en file, numérotés à partir de l'ordre en cours (1).
		if last != here:
			_number(labels, last, 1)
			labels += 1
		var number := 2 if last != here else 1
		var end_facing := NAN
		for entry in queue:
			var to := Vector3(float(entry["x"]), last.y, float(entry["z"]))
			if not drawn:
				mesh.surface_begin(Mesh.PRIMITIVE_TRIANGLES)
				drawn = true
			if entry.has("target"):
				var foe := _find(units, int(entry["target"]))
				if not foe.is_empty():
					to.y = float(foe["y"])
				_dashes(mesh, PackedVector3Array([last, to]), RED, true)
				last_path = PackedVector3Array()
			else:
				var segment := _segment(id, last, to)
				if segment.size() >= 2:
					_dashes(mesh, segment, _color, false)
					to = segment[segment.size() - 1]
					last_path = segment
				else:
					_dashes(mesh, PackedVector3Array([last, to]), RED, false)
					last_path = PackedVector3Array()
				end_facing = float(entry["facing"]) if entry.has("facing") else NAN
			_number(labels, to, number)
			labels += 1
			number += 1
			last = to
		if last_path.size() >= 2:
			_ghost(_order_ghosts, used, unit, last, _end_facing(last_path, end_facing), _color)
			used += 1
	if drawn:
		for _i in 3:  # triangle dégénéré : une surface sans sommet est refusée par Godot
			mesh.surface_set_color(Color(0, 0, 0, 0))
			mesh.surface_add_vertex(Vector3.ZERO)
		mesh.surface_end()
	_orders_mesh.visible = drawn
	for k in range(used, _order_ghosts.size()):
		_order_ghosts[k].visible = false
	for k in range(labels, _order_labels.size()):
		_order_labels[k].visible = false


## CB-M3 : trajet d'un ordre en file de `from` à `to` (`preview_path_from`), mis en cache : les
## points de la file ne bougent pas d'un rafraîchissement à l'autre.
func _segment(id: int, from: Vector3, to: Vector3) -> PackedVector3Array:
	var key := "%d:%.1f,%.1f>%.1f,%.1f" % [id, from.x, from.z, to.x, to.z]
	if _segment_cache.has(key):
		return _segment_cache[key]
	if _segment_cache.size() > 256:
		_segment_cache.clear()
	var path: PackedVector3Array = battle.call("preview_path_from", id, from.x, from.z, to.x, to.z) if battle != null else PackedVector3Array()
	_segment_cache[key] = path
	return path


## CB-M3 : numéro `number` du point de passage `at` (étiquette au sol, lisible de loin).
func _number(index: int, at: Vector3, number: int) -> void:
	while _order_labels.size() <= index:
		var label := Label3D.new()
		label.name = "Waypoint%d" % _order_labels.size()
		label.billboard = BaseMaterial3D.BILLBOARD_ENABLED
		label.no_depth_test = true
		label.fixed_size = true
		label.pixel_size = 0.0012
		label.font_size = 28
		label.outline_size = 10
		label.outline_modulate = Color(0.08, 0.05, 0.02, 0.95)
		label.modulate = _color.lerp(Color.WHITE, 0.4)
		label.render_priority = 3
		label.outline_render_priority = 2
		add_child(label)
		_order_labels.append(label)
	var label := _order_labels[index]
	label.text = str(number)
	label.position = Vector3(at.x, _ground(at.x, at.z, at.y) + 1.5, at.z)
	label.visible = true


## CB-M3 : numéros de points de passage visibles (tests, sonde).
func waypoint_numbers() -> Array[String]:
	var out: Array[String] = []
	for label in _order_labels:
		if label.visible:
			out.append(label.text)
	return out


## CB-M3 : vrai si un des régiments `selected` a déjà sa file pleine (`battle_queue_max`,
## `data/rules/battle_queue.json` par RuleValues) : un ordre en file serait refusé.
static func queue_full(units: Array, selected: Array) -> bool:
	var limit := int(RuleValues.value("battle_queue_max", 8.0))
	for unit in units:
		if selected.has(int(unit["id"])) and (unit.get("queue", []) as Array).size() >= limit:
			return true
	return false


## CB1 : tailles (largeur, profondeur) des fantômes visibles de l'aperçu en direct (tests, sonde).
func live_ghost_sizes() -> Array[Vector2]:
	var out: Array[Vector2] = []
	for ghost in _live_ghosts:
		if ghost.visible:
			out.append(Vector2(ghost.size.x, ghost.size.z))
	return out


## Nombre de fantômes visibles (tests).
func ghost_count() -> int:
	var n := 0
	for ghost in _live_ghosts + _order_ghosts:
		if ghost.visible:
			n += 1
	return n


## Nœud du trajet en direct (tests, sonde de rendu).
func live_mesh() -> MeshInstance3D:
	return _live_mesh


func orders_mesh() -> MeshInstance3D:
	return _orders_mesh


func _draw_live(units: Array, point: Vector3, facing: float) -> void:
	var mesh := _live_mesh.mesh as ImmediateMesh
	mesh.clear_surfaces()
	mesh.surface_begin(Mesh.PRIMITIVE_TRIANGLES)
	var used := 0
	for leg in legs:
		var path: PackedVector3Array = leg["path"]
		var unit := _find(units, int(leg["unit"]))
		if not bool(leg["ok"]):
			# Inaccessible : trait rouge du régiment (ou du centre) au point visé.
			var from := Vector3(float(unit["x"]), float(unit["y"]), float(unit["z"])) if not unit.is_empty() else point
			_dashes(mesh, PackedVector3Array([from, point]), RED, false)
			continue
		_dashes(mesh, path, _color, false)
		if not unit.is_empty():
			# CB1 : taille d'arrivée (largeur du glisser) et orientation propre d'un groupe verrouillé.
			var size := Vector2(float(leg.get("width", 0.0)), float(leg.get("depth", 0.0)))
			var end_facing := float(leg["facing"]) if leg.has("facing") else facing
			_ghost(_live_ghosts, used, unit, path[path.size() - 1], _end_facing(path, end_facing), _color, size)
			used += 1
	# Au moins un triangle dégénéré : une surface vide est refusée par Godot.
	for _i in 3:
		mesh.surface_set_color(Color(0, 0, 0, 0))
		mesh.surface_add_vertex(point)
	mesh.surface_end()
	_live_mesh.visible = true
	for k in range(used, _live_ghosts.size()):
		_live_ghosts[k].visible = false


## Orientation d'arrivée : celle de l'ordre, sinon celle du dernier tronçon.
func _end_facing(path: PackedVector3Array, facing: float) -> float:
	if is_finite(facing) or path.size() < 2:
		return facing if is_finite(facing) else 0.0
	var a := path[path.size() - 2]
	var b := path[path.size() - 1]
	return atan2(b.x - a.x, b.z - a.z)


## Pointillés au sol le long de `path` (et pointe de flèche au bout si `arrow`).
func _dashes(mesh: ImmediateMesh, path: PackedVector3Array, color: Color, arrow: bool) -> void:
	var carry := 0.0  # position dans le motif tiret + espace, reportée d'un tronçon à l'autre
	for k in range(path.size() - 1):
		var a := path[k]
		var b := path[k + 1]
		var flat := Vector2(b.x - a.x, b.z - a.z)
		var length := flat.length()
		if length < 0.01:
			continue
		var dir := flat / length
		var side := Vector2(-dir.y, dir.x) * WIDTH * 0.5
		var s := -carry
		while s < length:
			var s0 := maxf(s, 0.0)
			var s1 := minf(s + DASH, length)
			if s1 > s0:
				_quad(mesh, a, b, length, s0, s1, side, color)
			s += DASH + GAP
		carry = fmod(length + carry, DASH + GAP)
	if arrow and path.size() >= 2:
		var a := path[path.size() - 2]
		var b := path[path.size() - 1]
		var flat := Vector2(b.x - a.x, b.z - a.z).normalized()
		var tip := Vector2(b.x, b.z)
		var base := tip - flat * ARROW_HEAD
		var wing := Vector2(-flat.y, flat.x) * ARROW_HEAD * 0.45
		for p in [tip, base + wing, base - wing]:
			mesh.surface_set_color(color)
			mesh.surface_add_vertex(Vector3(p.x, _ground(p.x, p.y, b.y), p.y))


func _quad(mesh: ImmediateMesh, a: Vector3, b: Vector3, length: float, s0: float, s1: float, side: Vector2, color: Color) -> void:
	var p0 := a.lerp(b, s0 / length)
	var p1 := a.lerp(b, s1 / length)
	var y0 := _ground(p0.x, p0.z, p0.y)
	var y1 := _ground(p1.x, p1.z, p1.y)
	var v := [
		Vector3(p0.x + side.x, y0, p0.z + side.y),
		Vector3(p0.x - side.x, y0, p0.z - side.y),
		Vector3(p1.x + side.x, y1, p1.z + side.y),
		Vector3(p1.x - side.x, y1, p1.z - side.y),
	]
	for index in [0, 1, 2, 2, 1, 3]:
		mesh.surface_set_color(color)
		mesh.surface_add_vertex(v[index])


## Hauteur de dessin : au-dessus du sol et du point du trajet (tablier d'un pont).
func _ground(x: float, z: float, path_y: float) -> float:
	var ground := float(_height_at.call(x, z)) if _height_at.is_valid() else path_y
	return maxf(ground, path_y) + LIFT


## `size` (largeur, profondeur ; CB1) : taille d'arrivée, sinon celle de l'unité.
func _ghost(pool: Array[Decal], index: int, unit: Dictionary, at: Vector3, facing: float, color: Color, size: Vector2 = Vector2.ZERO) -> void:
	while pool.size() <= index:
		var decal := Decal.new()
		decal.name = "Ghost%d" % pool.size()
		decal.albedo_mix = 1.0
		decal.emission_energy = 0.6
		decal.upper_fade = 0.05
		decal.lower_fade = 0.05
		decal.normal_fade = 0.0
		add_child(decal)
		pool.append(decal)
	var ghost := pool[index]
	var w := size.x if size.x > 0.0 else float(unit["width"])
	var d := size.y if size.y > 0.0 else float(unit["depth"])
	var key := BattleFormationOutline.texture_key(w + BattleFormationOutline.MARGIN, d + BattleFormationOutline.MARGIN, false)
	var pair := BattleFormationOutline._texture_pair(key)
	ghost.texture_albedo = pair[0]
	ghost.texture_emission = pair[1]
	ghost.size = Vector3(key.x * BattleFormationOutline.SIZE_STEP, BattleFormationOutline.HEIGHT, key.y * BattleFormationOutline.SIZE_STEP)
	ghost.position = at
	ghost.rotation = Vector3(0, facing, 0)
	ghost.modulate = Color(color, GHOST_ALPHA)
	ghost.visible = true


func _find(units: Array, id: int) -> Dictionary:
	for unit in units:
		if int(unit["id"]) == id:
			return unit
	return {}


func _make_mesh(node_name: String) -> MeshInstance3D:
	var node := MeshInstance3D.new()
	node.name = node_name
	node.mesh = ImmediateMesh.new()
	node.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	node.visible = false
	var material := StandardMaterial3D.new()
	material.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	material.vertex_color_use_as_albedo = true
	material.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA
	material.cull_mode = BaseMaterial3D.CULL_DISABLED
	material.no_depth_test = true  # lisible par-dessus l'herbe et les figurines, comme dans TW
	material.render_priority = 2
	node.material_override = material
	add_child(node)
	return node
