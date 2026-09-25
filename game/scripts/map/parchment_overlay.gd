class_name ParchmentOverlay
extends Control

## Lot CM2 : couche 2D de la vue stratégique parchemin, dessinée au-dessus de la carte selon
## la projection de la caméra : noms des royaumes (capitales espacées, en arc) et des provinces
## (italique calligraphique), villes en vignettes, armées en jetons à blason, navires et
## monstres marins. Opacité = poids du parchemin (fondu). Purement visuel ; la sélection des
## armées reste celle des marqueurs 3D (mêmes positions à l'écran).

const FONT_ITALIC := preload("res://assets/third_party/fonts/im_fell_english/IMFeENit28P.ttf")
const FONT_ROMAN := preload("res://assets/third_party/fonts/im_fell_english/IMFeENrm28P.ttf")

const INK := Color(0.20, 0.13, 0.07)
const RED_INK := Color(0.62, 0.14, 0.09)
const PAPER := Color(0.93, 0.86, 0.70)
const ROOF := Color(0.70, 0.25, 0.15)
const SEA_GREEN := Color(0.38, 0.52, 0.40)
const GOLD := Color(0.95, 0.76, 0.28)
## Distance caméra de référence (échelle 1 des vignettes et des jetons).
const REF_DISTANCE := 1400.0

var weight: float = 0.0
var camera_distance: float = REF_DISTANCE
var camera: Camera3D
var map_data: MapData
var decor: ParchmentDecor
var armies: ArmyMarkers

var _realms: Array[Dictionary] = []  # {name, points: PackedVector2Array (arc), size}
var _provinces: Array[Dictionary] = []  # {name, px, area}
var _towns: Array[Dictionary] = []  # {px, capital: bool, color}
var _heraldry_cache: Dictionary = {}


func _ready() -> void:
	mouse_filter = Control.MOUSE_FILTER_IGNORE
	set_anchors_preset(Control.PRESET_FULL_RECT)


## Relit propriétaires, noms et villes (une fois par tour, ou au chargement).
func refresh(sim: Object, settlement_data: SettlementData) -> void:
	_provinces.clear()
	_realms.clear()
	_towns.clear()
	if map_data == null:
		return
	var owner_of: Dictionary = {}
	for index in range(1, map_data.province_count + 1):
		var province := map_data.get_province(index)
		if province.is_empty():
			continue
		var id := str(province.get("id", ""))
		var owner := str(province.get("owner", ""))
		if sim != null and sim.has_method("get_province_state"):
			var state: Dictionary = sim.call("get_province_state", id)
			owner = str(state.get("owner", owner))
		owner_of[id] = owner
		var name := str(province.get("name", id))
		var paren := name.find(" (")
		_provinces.append({"name": name.substr(0, paren) if paren > 0 else name, "px": province["centroid"], "area": float(province.get("area_px", 0.0))})
	_provinces.sort_custom(func(a: Dictionary, b: Dictionary) -> bool: return a["area"] > b["area"])
	_build_realms(owner_of)
	if settlement_data != null:
		var capitals: Dictionary = {}
		for index in range(1, map_data.province_count + 1):
			var province := map_data.get_province(index)
			if not province.is_empty():
				capitals[str(province.get("id", ""))] = province.get("capital_px", Vector2(-1, -1))
		for entry in settlement_data.settlements:
			if str(entry.get("kind", "")) != "city":
				continue
			var px: Vector2 = entry["px"]
			var province_id := str(entry.get("province", ""))
			var owner := str(owner_of.get(province_id, ""))
			var cap: Vector2 = capitals.get(province_id, Vector2(-1, -1))
			_towns.append({"px": px, "capital": cap.distance_to(px) < 6.0, "color": SimFacade.faction_color(owner) if owner != "" else INK})
	queue_redraw()


## Regroupe les provinces de chaque faction par voisinage ; nomme le plus grand bloc.
func _build_realms(owner_of: Dictionary) -> void:
	var seen: Dictionary = {}
	var best: Dictionary = {}  # owner → {area, provinces}
	for index in range(1, map_data.province_count + 1):
		var province := map_data.get_province(index)
		if province.is_empty():
			continue
		var id := str(province.get("id", ""))
		var owner := str(owner_of.get(id, ""))
		if owner == "" or seen.has(id):
			continue
		var cluster: Array[Dictionary] = []
		var area := 0.0
		var stack: Array[String] = [id]
		seen[id] = true
		while not stack.is_empty():
			var current: String = stack.pop_back()
			var entry := map_data.get_province(map_data.index_of_id(current))
			cluster.append(entry)
			area += float(entry.get("area_px", 0.0))
			for n in entry.get("neighbors", []):
				var nid := str(n)
				if not seen.has(nid) and str(owner_of.get(nid, "")) == owner:
					seen[nid] = true
					stack.append(nid)
		if area > float(best.get(owner, {}).get("area", 0.0)):
			best[owner] = {"area": area, "cluster": cluster}
	for owner in best:
		var area: float = best[owner]["area"]
		if area < 9000.0:
			continue
		var cluster: Array = best[owner]["cluster"]
		# Centre et axe principal (pondérés par l'aire) : le nom suit l'allongement du royaume.
		var c := Vector2.ZERO
		for e in cluster:
			c += Vector2(e["centroid"]) * float(e.get("area_px", 0.0))
		c /= area
		var cov := Vector3.ZERO
		for e in cluster:
			var d: Vector2 = Vector2(e["centroid"]) - c
			var w := float(e.get("area_px", 0.0))
			cov += Vector3(d.x * d.x, d.x * d.y, d.y * d.y) * w
		var angle := 0.5 * atan2(2.0 * cov.y, cov.x - cov.z)
		angle = clampf(angle, deg_to_rad(-28.0), deg_to_rad(28.0))
		var info := SimFacade.faction_info(owner)
		var name := str(info.get("name", owner)).to_upper()
		var half := clampf(sqrt(area) * 0.42, 120.0, 520.0)
		var dir := Vector2(cos(angle), sin(angle))
		_realms.append({"name": name, "a": c - dir * half, "b": c + dir * half, "c": c, "size": clampf(sqrt(area) / 22.0, 14.0, 30.0)})


func set_weight(value: float, distance: float) -> void:
	weight = value
	camera_distance = distance
	visible = weight > 0.01


func _process(_delta: float) -> void:
	if visible:
		queue_redraw()


func _screen(px: Vector2) -> Vector2:
	var y := map_data.surface_world_at(px.x, px.y) if map_data != null else 0.0
	var world := Vector3(px.x, y, px.y)
	if camera.is_position_behind(world):
		return Vector2(-99999, -99999)
	return camera.unproject_position(world)


func _draw() -> void:
	if camera == null or map_data == null or weight <= 0.01:
		return
	var s := clampf(REF_DISTANCE / maxf(camera_distance, 1.0), 0.7, 1.6)
	var a := weight
	var view := get_viewport_rect().grow(60.0)
	_draw_sea_decor(s, a, view)
	for town in _towns:
		var p := _screen(town["px"])
		if view.has_point(p):
			_draw_town(p, (15.0 if town["capital"] else 10.0) * s, town, a)
	var placed: Array[Rect2] = []
	for realm in _realms:
		placed.append_array(_draw_arched(realm, s, a))
	_draw_province_names(s, a, view, placed)
	_draw_army_tokens(s, a, view)


# --- Mer ------------------------------------------------------------------------------


func _draw_sea_decor(s: float, a: float, view: Rect2) -> void:
	if decor == null:
		return
	var t := Time.get_ticks_msec() / 1000.0
	for ship in decor.ships:
		var p := _screen(Vector2(ship.x, ship.y))
		if view.has_point(p):
			_draw_ship(p + Vector2(0.0, sin(t * 1.3 + ship.x) * 1.2), 16.0 * s, ship.z, a)
	for monster in decor.monsters:
		var p := _screen(Vector2(monster.x, monster.y))
		if view.has_point(p):
			if monster.z < 0.5:
				_draw_serpent(p, 20.0 * s, a, t + monster.x)
			else:
				_draw_whale(p, 20.0 * s, a, t + monster.y)


func _draw_ship(p: Vector2, u: float, facing: float, a: float) -> void:
	var f := facing
	var hull := PackedVector2Array()
	for i in 9:
		var x := lerpf(-1.1, 1.1, i / 8.0)
		hull.append(p + Vector2(x * f, 0.28 * (x * x) - 0.05) * u)
	for i in range(8, -1, -1):
		var x := lerpf(-0.8, 0.85, i / 8.0)
		hull.append(p + Vector2(x * f, 0.42 + 0.06 * x * x) * u)
	draw_colored_polygon(hull, Color(0.52, 0.33, 0.18, a))
	draw_polyline(_closed(hull), Color(INK, a), 1.2, true)
	# Mât, vergue, voile carrée gonflée à croix rouge, flamme.
	draw_line(p + Vector2(0.05 * f, 0.0) * u, p + Vector2(0.05 * f, -1.55) * u, Color(INK, a), 1.3, true)
	var sail := PackedVector2Array()
	for i in 7:
		var y := lerpf(-1.35, -0.2, i / 6.0)
		sail.append(p + Vector2((-0.55 + 0.12 * sin(i / 6.0 * PI)) * f, y) * u)
	for i in range(6, -1, -1):
		var y := lerpf(-1.35, -0.2, i / 6.0)
		sail.append(p + Vector2((0.65 + 0.22 * sin(i / 6.0 * PI)) * f, y) * u)
	draw_colored_polygon(sail, Color(PAPER.lightened(0.15), a))
	draw_polyline(_closed(sail), Color(INK, a), 1.0, true)
	draw_line(p + Vector2(0.08 * f, -1.2) * u, p + Vector2(0.08 * f, -0.35) * u, Color(RED_INK, a), 1.6)
	draw_line(p + Vector2(-0.35 * f, -0.8) * u, p + Vector2(0.55 * f, -0.8) * u, Color(RED_INK, a), 1.6)
	var flag := PackedVector2Array([p + Vector2(0.05 * f, -1.55) * u, p + Vector2(0.55 * f, -1.45) * u, p + Vector2(0.05 * f, -1.38) * u])
	draw_colored_polygon(flag, Color(RED_INK, a))
	_draw_waves(p + Vector2(0.0, 0.55) * u, u, a)


func _draw_serpent(p: Vector2, u: float, a: float, t: float) -> void:
	var body := Color(SEA_GREEN, a)
	# Trois ondulations au-dessus de l'eau, tête dressée à gauche, queue fourchue à droite.
	for k in 3:
		var cx := p.x + (k - 1) * 0.85 * u
		var arc := PackedVector2Array()
		for i in 11:
			var ang := PI + PI * i / 10.0
			arc.append(Vector2(cx + cos(ang) * 0.36 * u, p.y + sin(ang) * (0.42 + 0.04 * sin(t * 2.0 + k)) * u))
		for i in range(10, -1, -1):
			var ang := PI + PI * i / 10.0
			arc.append(Vector2(cx + cos(ang) * 0.2 * u, p.y + sin(ang) * 0.25 * u))
		draw_colored_polygon(arc, body)
		draw_polyline(_closed(arc), Color(INK, a), 1.0, true)
	var head := PackedVector2Array([
		p + Vector2(-1.35, 0.0) * u, p + Vector2(-1.45, -0.75) * u, p + Vector2(-1.9, -0.95) * u,
		p + Vector2(-2.15, -0.8) * u, p + Vector2(-1.75, -0.7) * u, p + Vector2(-1.95, -0.55) * u,
		p + Vector2(-1.55, -0.5) * u, p + Vector2(-1.15, 0.0) * u])
	draw_colored_polygon(head, body)
	draw_polyline(_closed(head), Color(INK, a), 1.0, true)
	draw_circle(p + Vector2(-1.72, -0.82) * u, maxf(0.06 * u, 1.0), Color(RED_INK, a))
	var tail := PackedVector2Array([p + Vector2(1.25, 0.0) * u, p + Vector2(1.6, -0.55) * u, p + Vector2(1.55, -0.2) * u, p + Vector2(1.95, -0.45) * u, p + Vector2(1.45, 0.0) * u])
	draw_colored_polygon(tail, body)
	draw_polyline(_closed(tail), Color(INK, a), 1.0, true)
	_draw_waves(p + Vector2(0.0, 0.1) * u, u * 1.4, a)


func _draw_whale(p: Vector2, u: float, a: float, t: float) -> void:
	var back := PackedVector2Array()
	for i in 17:
		var x := lerpf(-1.4, 1.2, i / 16.0)
		back.append(p + Vector2(x, -0.55 * sqrt(maxf(1.0 - pow((x + 0.1) / 1.3, 2.0), 0.0))) * u)
	back.append(p + Vector2(1.2, 0.0) * u)
	back.append(p + Vector2(-1.4, 0.0) * u)
	draw_colored_polygon(back, Color(0.36, 0.40, 0.42, a))
	draw_polyline(_closed(back), Color(INK, a), 1.1, true)
	draw_circle(p + Vector2(-0.95, -0.25) * u, maxf(0.05 * u, 1.0), Color(INK, a))
	# Jet d'eau en gerbe.
	var top := p + Vector2(-0.6, -0.55) * u
	var puff := 0.8 + 0.2 * sin(t * 2.5)
	for k in 5:
		var ang := deg_to_rad(-150.0 + k * 30.0)
		draw_line(top, top + Vector2(cos(ang), sin(ang) - 0.6) * u * 0.5 * puff, Color(0.30, 0.42, 0.50, a), 1.2, true)
	# Queue relevée.
	var fluke := PackedVector2Array([p + Vector2(1.1, -0.05) * u, p + Vector2(1.55, -0.65) * u, p + Vector2(1.4, -0.3) * u, p + Vector2(1.85, -0.5) * u, p + Vector2(1.3, 0.0) * u])
	draw_colored_polygon(fluke, Color(0.36, 0.40, 0.42, a))
	draw_polyline(_closed(fluke), Color(INK, a), 1.0, true)
	_draw_waves(p + Vector2(0.0, 0.1) * u, u * 1.5, a)


func _draw_waves(p: Vector2, u: float, a: float) -> void:
	for k in 3:
		var y := p.y + k * 0.18 * u
		var pts := PackedVector2Array()
		for i in 9:
			var x := lerpf(-1.2, 1.2, i / 8.0) * (1.0 - k * 0.2)
			pts.append(Vector2(p.x + x * u, y + sin(i * 1.6) * 0.06 * u))
		draw_polyline(pts, Color(INK, a * (0.55 - k * 0.15)), 1.0, true)


# --- Villes ----------------------------------------------------------------------------


func _draw_town(p: Vector2, u: float, town: Dictionary, a: float) -> void:
	var capital: bool = town["capital"]
	draw_set_transform(p, 0.0, Vector2.ONE)
	# Tertre au lavis.
	var ground := PackedVector2Array()
	for i in 16:
		var ang := TAU * i / 16.0
		ground.append(Vector2(cos(ang) * 1.05, 0.05 + sin(ang) * 0.28) * u)
	draw_colored_polygon(ground, Color(0.55, 0.45, 0.28, 0.45 * a))
	# Enceinte crénelée.
	var wall := PackedVector2Array([Vector2(-0.8, 0.0) * u, Vector2(-0.8, -0.45) * u])
	var merlons := 6
	for i in merlons:
		var x0 := lerpf(-0.8, 0.8, float(i) / merlons)
		var x1 := lerpf(-0.8, 0.8, (i + 0.5) / merlons)
		wall.append(Vector2(x0, -0.55) * u)
		wall.append(Vector2(x1, -0.55) * u)
		wall.append(Vector2(x1, -0.45) * u)
		wall.append(Vector2(lerpf(-0.8, 0.8, float(i + 1) / merlons), -0.45) * u)
	wall.append(Vector2(0.8, 0.0) * u)
	# Clocher (derrière l'enceinte), puis enceinte et tours devant.
	var spire_x := 0.15
	_draw_box(Rect2(Vector2(spire_x - 0.14, -1.05) * u, Vector2(0.28, 0.7) * u), PAPER.darkened(0.08), a)
	_draw_roof(Vector2(spire_x, -1.05) * u, 0.2 * u, 0.75 * u, Color(0.35, 0.33, 0.36), a)
	if capital:
		draw_line(Vector2(spire_x, -1.8) * u, Vector2(spire_x, -2.25) * u, Color(INK, a), 1.2)
		var flag := PackedVector2Array([Vector2(spire_x, -2.25) * u, Vector2(spire_x + 0.55, -2.12) * u, Vector2(spire_x, -1.98) * u])
		draw_colored_polygon(flag, Color(town["color"], a))
		draw_polyline(_closed(flag), Color(INK, a), 1.0)
	draw_colored_polygon(wall, Color(PAPER.lightened(0.1), a))
	draw_polyline(_closed(wall), Color(INK, a), 1.1, true)
	draw_rect(Rect2(Vector2(-0.12, -0.28) * u, Vector2(0.24, 0.28) * u), Color(INK, 0.8 * a))
	for tx in [-0.8, 0.8]:
		_draw_box(Rect2(Vector2(tx - 0.16, -0.75) * u, Vector2(0.32, 0.75) * u), PAPER, a)
		_draw_roof(Vector2(tx, -0.75) * u, 0.22 * u, 0.45 * u, ROOF, a)
	draw_set_transform(Vector2.ZERO, 0.0, Vector2.ONE)


func _draw_box(rect: Rect2, fill: Color, a: float) -> void:
	draw_rect(rect, Color(fill, a))
	draw_rect(rect, Color(INK, a), false, 1.0)


func _draw_roof(base_center: Vector2, half_width: float, height: float, color: Color, a: float) -> void:
	var tri := PackedVector2Array([base_center + Vector2(-half_width, 0.0), base_center + Vector2(0.0, -height), base_center + Vector2(half_width, 0.0)])
	draw_colored_polygon(tri, Color(color, a))
	draw_polyline(_closed(tri), Color(INK, a), 1.0, true)


# --- Noms -----------------------------------------------------------------------------


## Nom de royaume en capitales espacées, légèrement arqué ; renvoie les rectangles occupés.
func _draw_arched(realm: Dictionary, s: float, a: float) -> Array[Rect2]:
	var rects: Array[Rect2] = []
	var pa := _screen(realm["a"])
	var pb := _screen(realm["b"])
	var pc := _screen(realm["c"])
	if pa.x < -9999.0 or pb.x < -9999.0:
		return rects
	var text: String = realm["name"]
	var size := int(realm["size"] * s)
	var chord := pb - pa
	if chord.x < 0.0:
		var tmp := pa
		pa = pb
		pb = tmp
		chord = -chord
	var length := chord.length()
	var widths: Array[float] = []
	var total := 0.0
	for ch in text:
		var w := FONT_ROMAN.get_string_size(ch, HORIZONTAL_ALIGNMENT_LEFT, -1, size).x
		widths.append(w)
		total += w
	var spacing := clampf((length - total) / maxf(text.length() - 1, 1.0), size * 0.08, size * 0.9)
	total += spacing * (text.length() - 1)
	var dir := chord / maxf(length, 1.0)
	var normal := Vector2(dir.y, -dir.x)
	var start := pc - dir * total * 0.5
	var bend := total * 0.06
	var x := 0.0
	var color := Color(RED_INK.darkened(0.1), 0.85 * a)
	var outline := Color(PAPER, 0.55 * a)
	for i in text.length():
		var ch := text[i]
		var t := (x + widths[i] * 0.5) / maxf(total, 1.0) * 2.0 - 1.0
		var pos := start + dir * x + normal * bend * (1.0 - t * t)
		var ang := dir.angle() - atan(2.0 * bend * t / maxf(total * 0.5, 1.0)) * 0.5
		draw_set_transform(pos, ang, Vector2.ONE)
		draw_string_outline(FONT_ROMAN, Vector2(0.0, size * 0.35), ch, HORIZONTAL_ALIGNMENT_LEFT, -1, size, maxi(2, size / 5), outline)
		draw_string(FONT_ROMAN, Vector2(0.0, size * 0.35), ch, HORIZONTAL_ALIGNMENT_LEFT, -1, size, color)
		x += widths[i] + spacing
	draw_set_transform(Vector2.ZERO, 0.0, Vector2.ONE)
	var box := Rect2(start, Vector2.ZERO).expand(start + dir * total).grow(size * 0.7)
	rects.append(box)
	return rects


func _draw_province_names(s: float, a: float, view: Rect2, placed: Array[Rect2]) -> void:
	var size := int(clampf(13.0 * s, 11.0, 20.0))
	var color := Color(INK, 0.9 * a)
	var outline := Color(PAPER, 0.6 * a)
	for province in _provinces:
		var p := _screen(province["px"])
		if not view.has_point(p):
			continue
		var text: String = province["name"]
		var w := FONT_ITALIC.get_string_size(text, HORIZONTAL_ALIGNMENT_LEFT, -1, size).x
		var rect := Rect2(p - Vector2(w * 0.5, size * 0.9), Vector2(w, size * 1.2))
		var clear := true
		for r in placed:
			if r.intersects(rect):
				clear = false
				break
		if not clear:
			continue
		placed.append(rect.grow(3.0))
		var origin := p + Vector2(-w * 0.5, size * 0.1)
		draw_string_outline(FONT_ITALIC, origin, text, HORIZONTAL_ALIGNMENT_LEFT, -1, size, 3, outline)
		draw_string(FONT_ITALIC, origin, text, HORIZONTAL_ALIGNMENT_LEFT, -1, size, color)


# --- Armées ----------------------------------------------------------------------------


func _draw_army_tokens(s: float, a: float, view: Rect2) -> void:
	if armies == null:
		return
	var r := 11.0 * s
	for id in armies._markers:
		var marker: ArmyMarker = armies._markers[id]
		if marker == null or not is_instance_valid(marker):
			continue
		var p := camera.unproject_position(marker.global_position)
		if camera.is_position_behind(marker.global_position) or not view.has_point(p):
			continue
		var selected: bool = id == armies.selected_army
		# Ombre, disque à la couleur de la faction, blason, double filet d'encre (or si joueur).
		draw_circle(p + Vector2(1.5, 2.0), r * 1.05, Color(0.0, 0.0, 0.0, 0.3 * a))
		draw_circle(p, r, Color(marker.faction_color, a))
		var tex := _heraldry(marker.faction_id)
		if tex != null:
			var side := r * 1.25
			draw_texture_rect(tex, Rect2(p - Vector2(side, side) * 0.5, Vector2(side, side)), false, Color(1, 1, 1, a))
		draw_arc(p, r, 0.0, TAU, 32, Color(INK, a), 1.6, true)
		draw_arc(p, r + 2.0, 0.0, TAU, 32, Color(GOLD if marker.is_player else PAPER, a), 1.4, true)
		if selected:
			var pulse := 0.5 + 0.5 * sin(Time.get_ticks_msec() / 250.0)
			draw_arc(p, r + 5.0 + pulse * 2.0, 0.0, TAU, 40, Color(GOLD, a), 2.0, true)
		var men := ArmyMarkers.format_men(marker.men)
		var fs := int(clampf(10.0 * s, 9.0, 14.0))
		var w := FONT_ROMAN.get_string_size(men, HORIZONTAL_ALIGNMENT_LEFT, -1, fs).x
		var origin := p + Vector2(-w * 0.5, r + fs + 3.0)
		draw_string_outline(FONT_ROMAN, origin, men, HORIZONTAL_ALIGNMENT_LEFT, -1, fs, 3, Color(PAPER, a))
		draw_string(FONT_ROMAN, origin, men, HORIZONTAL_ALIGNMENT_LEFT, -1, fs, Color(INK, a))


func _heraldry(faction: String) -> Texture2D:
	if not _heraldry_cache.has(faction):
		_heraldry_cache[faction] = ArmyMarker._heraldry(faction)
	return _heraldry_cache[faction]


static func _closed(points: PackedVector2Array) -> PackedVector2Array:
	var out := points.duplicate()
	if not points.is_empty():
		out.append(points[0])
	return out
