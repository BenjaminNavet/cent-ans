class_name TownPlan
extends RefCounted

## Lot ZG6 (ADR 0036) : plan procédural d'une ville ordinaire vers 1340, à l'échelle réelle.
## Calcul pur et déterministe (graine de la colonie), sans accès à la scène : exécutable dans un
## fil de travail (`WorkerThreadPool`). Entrée : l'emprise de `data/map/towns_1340.json`
## (polygone d'enceinte adapté au relief, portes, faubourgs, monuments, fleuve, pont) et un
## échantillonneur de hauteurs en mètres (`Heights`). Sortie : rues (tracé drapé, pente adoucie),
## parcelles en lanières le long des rues (maison sur rue, jardin derrière), marché, églises,
## monuments, murailles, tours et portes, pont, arbres des jardins, avec leurs hauteurs de base.
## Coordonnées locales en mètres depuis l'ancrage (x vers +X monde, y vers +Z monde).
## Rendu seulement : aucune règle de jeu.

## Types de maisons et de bâtiments (indices des tableaux `house_kind`).
const HOUSE_KINDS: Array[String] = ["townhouse", "timber", "stonehouse", "cottage", "longere", "barn"]
## Profondeur de la maison sur rue (m) par type (le reste de la parcelle est jardin ou cour).
const HOUSE_DEPTH := {"townhouse": [10.5, 13.0], "timber": [7.0, 8.5], "stonehouse": [7.5, 8.5], "cottage": [5.2, 6.0], "longere": [5.8, 6.4], "barn": [7.5, 9.0]}
## Façade maximale d'un bâtiment (m) : au-delà, la parcelle garde une cour ou une ruelle latérale.
const HOUSE_MAX_FRONT := {"townhouse": 7.0, "timber": 11.5, "stonehouse": 12.5, "cottage": 10.0, "longere": 18.0, "barn": 14.0}
const CELL_M := 2.5
const STREET_STEP_M := 10.0
const ZONE_INTRA := 0
const ZONE_FAUBOURG := 1
const ZONE_VILLAGE := 2


## Échantillonneur de hauteurs (m) aux points locaux, lisible depuis un fil de travail :
## pages du quadtree (instantané `ReliefQuadtree.surface_snapshot`), repli `MapData`, ou
## fonction (tests).
class Heights:
	extends RefCounted

	var anchor := Vector2.ZERO  # unités monde
	var meters_per_unit := 719.0
	var pages: Dictionary = {}
	var top_level := 0
	var h_min := 0.0
	var h_range := 1.0
	var map_data: MapData
	var func_m: Callable
	var fallback_m := 0.0

	func height_m(lx: float, ly: float) -> float:
		if func_m.is_valid():
			return float(func_m.call(lx, ly))
		var x := anchor.x + lx / meters_per_unit
		var y := anchor.y + ly / meters_per_unit
		if not pages.is_empty():
			for level in range(top_level, -1, -1):
				var t := ReliefPyramid.tile_at(level, x, y)
				if t.x < 0 or t.y < 0:
					break
				var key := ReliefPyramid.key_of(level, t.x, t.y)
				if not pages.has(key):
					continue
				var origin := ReliefPyramid.tile_origin(level, t.x, t.y)
				var px_units := ReliefPyramid.pixel_units(level)
				return _bilinear_m(pages[key], (x - origin.x) / px_units - 0.5, (y - origin.y) / px_units - 0.5)
		if map_data != null:
			return map_data.height_m_at(x, y)
		return fallback_m

	func _bilinear_m(bytes: PackedByteArray, fx: float, fy: float) -> float:
		var n := ReliefPyramid.TILE_PX
		fx = clampf(fx, 0.0, n - 1.0)
		fy = clampf(fy, 0.0, n - 1.0)
		var i := mini(int(fx), n - 2)
		var j := mini(int(fy), n - 2)
		var tx := fx - i
		var ty := fy - j
		var o := (j * n + i) * 2
		var a := bytes.decode_u16(o)
		var b := bytes.decode_u16(o + 2)
		var c := bytes.decode_u16(o + n * 2)
		var d := bytes.decode_u16(o + n * 2 + 2)
		var top := a + (b - a) * tx
		var v := (top + (c + (d - c) * tx - top) * ty) / 65535.0
		return maxf(h_min + v * h_range, 0.0)


## Grille d'occupation (rues, parcelles, monuments, fleuve) pour éviter les chevauchements.
class Occupancy:
	extends RefCounted

	var origin := Vector2.ZERO
	var side := 0
	var cells := PackedByteArray()

	func _init(half_extent: float) -> void:
		side = int(ceil(half_extent * 2.0 / CELL_M)) + 2
		origin = Vector2(-half_extent, -half_extent) - Vector2(CELL_M, CELL_M)
		cells.resize(side * side)

	func _index(p: Vector2) -> int:
		var i := int((p.x - origin.x) / CELL_M)
		var j := int((p.y - origin.y) / CELL_M)
		if i < 0 or j < 0 or i >= side or j >= side:
			return -1
		return j * side + i

	func is_free(p: Vector2) -> bool:
		var k := _index(p)
		return k >= 0 and cells[k] == 0

	func mark(p: Vector2) -> void:
		var k := _index(p)
		if k >= 0:
			cells[k] = 1

	## Rectangle orienté : centre, axe « façade » `u` (unitaire), demi-longueurs (le long de u,
	## perpendiculaire). `test` : vrai si toutes les cases sont libres ; sinon marque.
	func rect(center: Vector2, u: Vector2, half_u: float, half_v: float, test: bool) -> bool:
		var v := Vector2(-u.y, u.x)
		var nu := maxi(1, int(ceil(half_u * 2.0 / CELL_M)))
		var nv := maxi(1, int(ceil(half_v * 2.0 / CELL_M)))
		for a in nu + 1:
			var su := -half_u + half_u * 2.0 * a / nu
			for b in nv + 1:
				var sv := -half_v + half_v * 2.0 * b / nv
				var p := center + u * su + v * sv
				if test:
					if not is_free(p):
						return false
				else:
					mark(p)
		return true

	## Segment épaissi (rue) : marque les cases à moins de `half_width`.
	func segment(a: Vector2, b: Vector2, half_width: float) -> void:
		var length := a.distance_to(b)
		if length < 0.01:
			return
		var u := (b - a) / length
		rect((a + b) * 0.5, u, length * 0.5 + half_width * 0.5, half_width, false)


# --- Entrée principale ------------------------------------------------------------------


## Plan complet d'une ville. `town` : entrée de `towns_1340.json` ; `params` : ses sections
## `plan` et `walls` réunies (`{"plan": ..., "walls": ...}`) ; `heights` : échantillonneur.
static func generate(town: Dictionary, params: Dictionary, heights: Heights) -> Dictionary:
	var t0 := Time.get_ticks_usec()
	var plan: Dictionary = params.get("plan", {})
	var rng := RandomNumberGenerator.new()
	rng.seed = int(town.get("seed", 1))
	var kind := str(town.get("kind", "town"))
	var radii := PackedFloat32Array()
	for r in town.get("radii", []):
		radii.append(float(r))
	if radii.size() < 3:
		radii = PackedFloat32Array([60.0, 60.0, 60.0, 60.0])
	var r_max := 0.0
	for r in radii:
		r_max = maxf(r_max, r)
	var reach := r_max
	for f in town.get("faubourgs", []):
		reach = maxf(reach, float(f["start_m"]) + float(f["length_m"]))
	reach += 80.0
	var out := {
		"id": str(town.get("id", "")),
		"kind": kind,
		"walls": str(town.get("walls", "none")),
		"radii": radii,
		"extent_m": reach,
		"streets": [],  # {points: PackedVector2Array, width, bases: PackedFloat32Array}
		"houses": {},  # parallèles : kind (Int32), x, y, yaw, front, depth, base, tint, zone
		"monuments": [],  # {kind, x, y, yaw, length, depth, base}
		"wall_ring": PackedVector2Array(),
		"wall_bases": PackedFloat32Array(),
		"wall_gaps": PackedInt32Array(),
		"towers": [],  # {x, y, radius, height, base}
		"gates": [],  # {x, y, yaw, base}
		"trees": PackedVector3Array(),  # x, y, base
		"bridge": {},
	}
	var occ := Occupancy.new(reach)
	var village := kind == "village" or kind == "castle" or kind == "abbey"
	var gate_points: Array[Vector2] = []
	var gate_types: Array[String] = []
	for g in town.get("gates", []):
		var angle := deg_to_rad(float(g["bearing"]))
		var r := radius_at(radii, angle)
		gate_points.append(Vector2(cos(angle), sin(angle)) * r)
		gate_types.append(str(g["type"]))
	# Fleuve : couloir interdit.
	var river: Variant = town.get("river")
	if river is Dictionary:
		var line := PackedVector2Array()
		for p in (river as Dictionary).get("line", []):
			line.append(Vector2(float(p[0]), float(p[1])))
		var half := float(river.get("width_m", 20.0)) * 0.5 + 6.0
		for i in line.size() - 1:
			occ.segment(line[i], line[i + 1], half)
	# Marché (place) au point d'ancrage, orienté sur la première route.
	var population := int(town.get("population", 500))
	var market_axis := (gate_points[0].normalized() if not gate_points.is_empty() else Vector2.RIGHT)
	var market_min: Array = plan.get("market_min_m", [30.0, 40.0])
	var market_area := maxf(float(plan.get("market_area_m2_per_inhabitant", 0.4)) * population, float(market_min[0]) * float(market_min[1]))
	if village:
		market_area = float(market_min[0]) * float(market_min[1]) * 0.6
	var market_len := sqrt(market_area * 1.5)
	var market_wid := market_area / market_len
	occ.rect(Vector2.ZERO, market_axis, market_len * 0.5, market_wid * 0.5, false)
	out["market"] = {"x": 0.0, "y": 0.0, "yaw": atan2(market_axis.y, market_axis.x), "length": market_len, "width": market_wid, "base": heights.height_m(0.0, 0.0)}
	var market_strip := PackedVector2Array()
	for i in 5:
		market_strip.append(market_axis * market_len * (float(i) / 4.0 - 0.5))
	var market_street := {"points": market_strip, "width": market_wid, "market": true}
	street_bases(market_street, heights)
	(out["streets"] as Array).append(market_street)
	# Monuments réservés avant les parcelles.
	for m in town.get("monuments", []):
		_reserve_monument(m, occ, out, heights, rng)
	# Rues principales : du marché à chaque porte, puis le long du faubourg.
	var widths: Dictionary = plan.get("street_width_m", {})
	var main_w := float(widths.get("main", 8.0))
	var sec_w := float(widths.get("secondary", 5.0))
	var faubourgs := {}
	for f in town.get("faubourgs", []):
		faubourgs[snappedf(float(f["bearing"]), 0.1)] = f
	var main_streets: Array = []
	for gi in gate_points.size():
		var gate := gate_points[gi]
		var dir := gate.normalized()
		var start := dir * minf(market_len, market_wid) * 0.5
		var path := drape_path(start, gate, heights, 24.0 if not village else 12.0)
		var bearing := fposmod(rad_to_deg(atan2(dir.y, dir.x)), 360.0)
		var fb: Dictionary = _closest_faubourg(faubourgs, bearing)
		var outer := 140.0 + (float(fb.get("length_m", 0.0)) if not fb.is_empty() else 0.0)
		var tail := drape_path(gate, gate + dir * outer, heights, 30.0)
		var full := path.duplicate()
		for i in range(1, tail.size()):
			full.append(tail[i])
		var width := main_w if gate_types[gi] == "main" or not village else sec_w
		main_streets.append({"points": full, "width": width, "gate_index": path.size() - 1, "faubourg": fb})
		_add_street(out, occ, full, width, heights)
	# Rue de ronde (lice) derrière l'enceinte.
	var walled := str(town.get("walls", "none")) != "none"
	var ring := PackedVector2Array()
	if walled:
		var n := radii.size()
		for i in n * 2 + 1:
			var a := TAU * i / (n * 2)
			ring.append(Vector2(cos(a), sin(a)) * maxf(radius_at(radii, a) - 14.0, 10.0))
		_add_street(out, occ, ring, float(widths.get("ring", 5.0)), heights)
	# Rues secondaires : branches perpendiculaires aux rues principales, suivant les courbes de niveau.
	var spacing: Array = plan.get("secondary_spacing_m", [70.0, 110.0])
	var secondary: Array = []
	if not village or population > 600:
		for s in main_streets:
			var pts: PackedVector2Array = s["points"]
			var gate_index: int = s["gate_index"]
			var along := float(spacing[0]) * 0.6
			var side := 1.0 if rng.randf() < 0.5 else -1.0
			for i in range(1, mini(gate_index + 1, pts.size())):
				var seg := pts[i] - pts[i - 1]
				along -= seg.length()
				if along > 0.0:
					continue
				along = rng.randf_range(float(spacing[0]), float(spacing[1])) * (1.6 if village else 1.0)
				side = -side
				var branch := _grow_branch(pts[i], seg.normalized(), side, radii, occ, heights, walled)
				if branch.size() >= 3:
					secondary.append(branch)
					_add_street(out, occ, branch, sec_w, heights)
	# Venelles : branches des rues secondaires (îlots plus petits au cœur des grandes villes).
	var lanes: Array = []
	var lane_w := float(widths.get("lane", 3.0))
	var lane_spacing: Array = plan.get("lane_spacing_m", [60.0, 95.0])
	if not village:
		for b in secondary:
			var bp: PackedVector2Array = b
			var along_l := float(lane_spacing[0]) * 0.5
			for i in range(1, bp.size()):
				along_l -= bp[i].distance_to(bp[i - 1])
				if along_l > 0.0:
					continue
				along_l = rng.randf_range(float(lane_spacing[0]), float(lane_spacing[1]))
				var lane := _grow_branch(bp[i], (bp[i] - bp[i - 1]).normalized(), 1.0 if rng.randf() < 0.5 else -1.0, radii, occ, heights, walled)
				if lane.size() >= 3:
					lanes.append(lane)
					_add_street(out, occ, lane, lane_w, heights)
	# Églises paroissiales le long des rues (espacées), orientées est-ouest.
	_place_parishes(int(town.get("parishes", 0)), main_streets, secondary, occ, heights, rng, out)
	# Parcelles le long des rues : principales d'abord (les plus denses), puis secondaires, lice.
	var households := int(town.get("households", 100))
	var core_households := int(ceil(float(town.get("core_population", population)) / 4.5))
	var per_house: Dictionary = plan.get("households_per_house", {})
	var per_core := float(per_house.get("village" if village else "intra", 1.0))
	var per_out := float(per_house.get("village" if village else "faubourg", 1.0))
	var budget := {"core": int(ceil(core_households * 1.05 / per_core)), "out": maxi(int(ceil((households - core_households) * 1.05 / per_out)), 0)}
	out["house_budget"] = budget.duplicate()
	var houses := {"kind": PackedInt32Array(), "x": PackedFloat32Array(), "y": PackedFloat32Array(), "yaw": PackedFloat32Array(), "front": PackedFloat32Array(), "depth": PackedFloat32Array(), "base": PackedFloat32Array(), "tint": PackedFloat32Array(), "zone": PackedInt32Array()}
	out["houses"] = houses
	var mixes: Dictionary = plan.get("house_kinds", {})
	for s in main_streets:
		_line_parcels(s["points"], float(s["width"]), radii, occ, heights, rng, plan, mixes, budget, houses, out, village, true)
	for b in secondary:
		_line_parcels(b, sec_w, radii, occ, heights, rng, plan, mixes, budget, houses, out, village, false)
	for lane in lanes:
		_line_parcels(lane, lane_w, radii, occ, heights, rng, plan, mixes, budget, houses, out, village, false)
	if ring.size() > 0:
		_line_parcels(ring, float(widths.get("ring", 5.0)), radii, occ, heights, rng, plan, mixes, budget, houses, out, village, false)
	# Enceinte, tours et portes.
	if walled:
		_build_walls(town, params, radii, gate_points, heights, out)
	# Pont.
	var bridge: Variant = town.get("bridge")
	if bridge is Dictionary:
		var at := Vector2(float(bridge["at"][0]), float(bridge["at"][1]))
		var d := Vector2(float(bridge["dir"][0]), float(bridge["dir"][1])).normalized()
		out["bridge"] = {"x": at.x, "y": at.y, "yaw": atan2(d.y, d.x) + PI * 0.5, "length": float(bridge["width_m"]) + 24.0, "width": 7.0, "deck": float(bridge["z_deck"]), "water": float(bridge["z_water"])}
	out["ground"] = _ground_grid(radii, heights, 10.0)
	var dwellings := int(out["house_budget"]["core"]) + int(out["house_budget"]["out"]) - maxi(int(budget["core"]), 0) - maxi(int(budget["out"]), 0)
	out["stats"] = {"dwellings": dwellings, "houses": houses["x"].size(), "streets": out["streets"].size(), "usec": Time.get_ticks_usec() - t0}
	return out


## Sol de la ville (cours, jardins, terre battue) : grille drapée de `step` m, sommets à
## l'intérieur du noyau ; hauteurs (m) aux sommets. {origin, step, n, inside, heights, radii}.
static func _ground_grid(radii: PackedFloat32Array, heights: Heights, step: float) -> Dictionary:
	var r_max := 0.0
	for r in radii:
		r_max = maxf(r_max, r)
	var n := int(ceil(r_max / step)) * 2 + 1
	var origin := Vector2(-(n - 1) * 0.5 * step, -(n - 1) * 0.5 * step)
	var inside_mask := PackedByteArray()
	inside_mask.resize(n * n)
	var h := PackedFloat32Array()
	h.resize(n * n)
	for j in n:
		for i in n:
			var p := origin + Vector2(i, j) * step
			var k := j * n + i
			if inside(radii, p, -step):
				inside_mask[k] = 1
				h[k] = heights.height_m(p.x, p.y)
	return {"origin": origin, "step": step, "n": n, "inside": inside_mask, "heights": h, "radii": radii}


## Recalcule seulement les hauteurs de base (m) d'un plan (pages plus fines arrivées).
static func reground(plan: Dictionary, heights: Heights) -> void:
	var houses: Dictionary = plan["houses"]
	var xs: PackedFloat32Array = houses["x"]
	var bases: PackedFloat32Array = houses["base"]
	for i in xs.size():
		bases[i] = footprint_base(heights, Vector2(xs[i], houses["y"][i]), houses["yaw"][i], houses["front"][i], houses["depth"][i])
	houses["base"] = bases
	for m in plan["monuments"]:
		m["base"] = footprint_base(heights, Vector2(m["x"], m["y"]), m["yaw"], m["length"], m["depth"])
	for s in plan["streets"]:
		street_bases(s, heights)
	var ring: PackedVector2Array = plan["wall_ring"]
	var wb := PackedFloat32Array()
	for p in ring:
		wb.append(heights.height_m(p.x, p.y))
	plan["wall_bases"] = wb
	for tower in plan["towers"]:
		tower["base"] = footprint_base(heights, Vector2(tower["x"], tower["y"]), 0.0, tower["radius"] * 2.0, tower["radius"] * 2.0)
	for gate in plan["gates"]:
		gate["base"] = footprint_base(heights, Vector2(gate["x"], gate["y"]), gate["yaw"], 10.0, 12.0)
	var trees: PackedVector3Array = plan["trees"]
	for i in trees.size():
		trees[i].z = heights.height_m(trees[i].x, trees[i].y)
	plan["trees"] = trees
	plan["market"]["base"] = heights.height_m(0.0, 0.0)
	var ground: Dictionary = plan.get("ground", {})
	if not ground.is_empty():
		plan["ground"] = _ground_grid(ground["radii"], heights, float(ground["step"]))


# --- Géométrie ------------------------------------------------------------------------------


## Rayon du polygone (relèvements réguliers, 0 = +x, sens de +y) à l'angle `angle`.
static func radius_at(radii: PackedFloat32Array, angle: float) -> float:
	var n := radii.size()
	var f := fposmod(angle, TAU) / TAU * n
	var i := int(f) % n
	var t := f - floorf(f)
	return radii[i] * (1.0 - t) + radii[(i + 1) % n] * t


static func inside(radii: PackedFloat32Array, p: Vector2, margin: float = 0.0) -> bool:
	var d := p.length()
	if d < 1.0:
		return true
	return d <= radius_at(radii, atan2(p.y, p.x)) - margin


## Base (m) d'une emprise : minimum du centre et des quatre coins, un peu enfoncée.
static func footprint_base(heights: Heights, center: Vector2, yaw: float, length: float, depth: float) -> float:
	var u := Vector2(cos(yaw), sin(yaw))
	var v := Vector2(-u.y, u.x)
	var low := heights.height_m(center.x, center.y)
	for c in [Vector2(1, 1), Vector2(-1, 1), Vector2(1, -1), Vector2(-1, -1)]:
		var p: Vector2 = center + u * (length * 0.5 * c.x) + v * (depth * 0.5 * c.y)
		low = minf(low, heights.height_m(p.x, p.y))
	return low - 0.3


## Tracé drapé de `a` à `b` : pas de 10 m, décalage latéral (≤ `lateral` m) choisi par
## programmation dynamique pour adoucir la pente (rues qui suivent le relief).
static func drape_path(a: Vector2, b: Vector2, heights: Heights, lateral: float) -> PackedVector2Array:
	var length := a.distance_to(b)
	var steps := maxi(1, int(ceil(length / STREET_STEP_M)))
	var dir := (b - a) / maxf(length, 0.001)
	var normal := Vector2(-dir.y, dir.x)
	var offsets := PackedFloat32Array([-1.0, -0.66, -0.33, 0.0, 0.33, 0.66, 1.0])
	var k := offsets.size()
	if steps < 3 or lateral <= 0.0:
		var straight := PackedVector2Array()
		for i in steps + 1:
			straight.append(a.lerp(b, float(i) / steps))
		return straight
	# Enveloppe : décalage nul aux extrémités, maximal au milieu.
	var pos: Array = []
	var h: Array = []
	for i in steps + 1:
		var t := float(i) / steps
		var env := sin(PI * t) * lateral
		var row_p := PackedVector2Array()
		var row_h := PackedFloat32Array()
		for j in k:
			var p := a.lerp(b, t) + normal * offsets[j] * env
			row_p.append(p)
			row_h.append(heights.height_m(p.x, p.y))
		pos.append(row_p)
		h.append(row_h)
	var cost := PackedFloat32Array()
	cost.resize(k)
	var back: Array = [PackedInt32Array()]
	for j in k:
		cost[j] = 0.0 if j == k / 2 else INF
	for i in range(1, steps + 1):
		var next := PackedFloat32Array()
		next.resize(k)
		var prev_idx := PackedInt32Array()
		prev_idx.resize(k)
		for j in k:
			var best := INF
			var arg := 0
			for q in range(maxi(0, j - 1), mini(k, j + 2)):
				if cost[q] == INF:
					continue
				var seg: float = (pos[i][j] as Vector2).distance_to(pos[i - 1][q])
				var grade: float = absf(float(h[i][j]) - float(h[i - 1][q])) / maxf(seg, 0.1)
				var c := cost[q] + seg * (1.0 + 60.0 * grade * grade) + absf(offsets[j]) * 0.3
				if c < best:
					best = c
					arg = q
			next[j] = best
			prev_idx[j] = arg
		cost = next
		back.append(prev_idx)
	var j_end := k / 2
	var result := PackedVector2Array()
	result.resize(steps + 1)
	for i in range(steps, -1, -1):
		result[i] = pos[i][j_end]
		if i > 0:
			j_end = (back[i] as PackedInt32Array)[j_end]
	return result


static func _closest_faubourg(faubourgs: Dictionary, bearing: float) -> Dictionary:
	var best: Dictionary = {}
	var best_d := 12.0
	for key in faubourgs:
		var d := absf(fposmod(float(key) - bearing + 180.0, 360.0) - 180.0)
		if d < best_d:
			best_d = d
			best = faubourgs[key]
	return best


static func _add_street(out: Dictionary, occ: Occupancy, pts: PackedVector2Array, width: float, heights: Heights) -> void:
	for i in range(1, pts.size()):
		occ.segment(pts[i - 1], pts[i], width * 0.5 + 0.5)
	var street := {"points": pts, "width": width}
	street_bases(street, heights)
	(out["streets"] as Array).append(street)


## Hauteurs (m) des deux bords d'une rue (ruban drapé).
static func street_bases(street: Dictionary, heights: Heights) -> void:
	var pts: PackedVector2Array = street["points"]
	var half := float(street["width"]) * 0.5
	var left := PackedFloat32Array()
	var right := PackedFloat32Array()
	for i in pts.size():
		var t := (pts[mini(i + 1, pts.size() - 1)] - pts[maxi(i - 1, 0)]).normalized()
		var n := Vector2(-t.y, t.x) * half
		left.append(heights.height_m(pts[i].x + n.x, pts[i].y + n.y))
		right.append(heights.height_m(pts[i].x - n.x, pts[i].y - n.y))
	street["bases_l"] = left
	street["bases_r"] = right


## Rue secondaire : part de `origin` perpendiculairement à la rue (côté `side`), tourne vers la
## courbe de niveau sur les pentes, s'arrête au bord du noyau ou près d'une autre rue.
static func _grow_branch(origin: Vector2, along: Vector2, side: float, radii: PackedFloat32Array, occ: Occupancy, heights: Heights, walled: bool) -> PackedVector2Array:
	var dir := Vector2(-along.y, along.x) * side
	var pts := PackedVector2Array([origin])
	var p := origin + dir * 9.0
	var margin := 22.0 if walled else 5.0
	for step in 45:
		# Pente locale : on glisse vers la direction de moindre pente (courbes de niveau).
		var here := heights.height_m(p.x, p.y)
		var gx := heights.height_m(p.x + 8.0, p.y) - here
		var gy := heights.height_m(p.x, p.y + 8.0) - here
		var grad := Vector2(gx, gy) / 8.0
		if grad.length() > 0.06:
			var contour := Vector2(-grad.y, grad.x).normalized()
			if contour.dot(dir) < 0.0:
				contour = -contour
			dir = dir.lerp(contour, 0.25).normalized()
		var next := p + dir * STREET_STEP_M
		if not inside(radii, next, margin) or not occ.is_free(next + dir * 8.0):
			break
		pts.append(p)
		p = next
	pts.append(p)
	return pts if pts.size() >= 4 else PackedVector2Array()


static func _pick(mix: Dictionary, rng: RandomNumberGenerator) -> String:
	var total := 0.0
	for key in mix:
		total += float(mix[key])
	var r := rng.randf() * total
	for key in mix:
		r -= float(mix[key])
		if r <= 0.0:
			return str(key)
	return str(mix.keys()[0])


## Parcelles en lanières des deux côtés d'une rue : façade et profondeur tirées dans les
## fourchettes des données, maison sur rue (façade du modèle vers la rue), jardin derrière
## (parfois un arbre fruitier). Zone : intra-muros, faubourg (hors du noyau) ou village.
static func _line_parcels(pts: PackedVector2Array, width: float, radii: PackedFloat32Array, occ: Occupancy, heights: Heights, rng: RandomNumberGenerator, plan: Dictionary, mixes: Dictionary, budget: Dictionary, houses: Dictionary, out: Dictionary, village: bool, main: bool) -> void:
	for side: float in [1.0, -1.0]:
		var carry := 0.0
		for i in range(1, pts.size()):
			var a := pts[i - 1]
			var b := pts[i]
			var seg := b - a
			var seg_len := seg.length()
			if seg_len < 0.5:
				continue
			var u := seg / seg_len
			var n := Vector2(-u.y, u.x) * side
			var s := carry
			while s < seg_len:
				var mid := a + u * s
				var in_core := inside(radii, mid)
				var zone := ZONE_VILLAGE if village else (ZONE_INTRA if in_core else ZONE_FAUBOURG)
				var key := "core" if in_core else "out"
				if int(budget[key]) <= 0:
					s += 8.0
					continue
				var fr: Array = plan.get("frontage_m", [5.0, 8.0])
				var dp: Array = plan.get("depth_m", [20.0, 40.0])
				if zone == ZONE_FAUBOURG:
					fr = plan.get("faubourg_frontage_m", [8.0, 16.0])
					dp = plan.get("faubourg_depth_m", [30.0, 60.0])
				elif zone == ZONE_VILLAGE:
					fr = plan.get("village_frontage_m", [14.0, 26.0])
					dp = plan.get("faubourg_depth_m", [30.0, 60.0])
				var front := rng.randf_range(float(fr[0]), float(fr[1]))
				var depth := rng.randf_range(float(dp[0]), float(dp[1]))
				if not main and zone == ZONE_INTRA:
					depth *= 0.8
				var center_along := mid + u * (front * 0.5)
				# Parcelle aussi profonde que possible : pleine, puis raccourcie (îlots étroits).
				var placed := false
				for factor: float in [1.0, 0.7, 0.5]:
					var d := maxf(depth * factor, 13.0)
					var parcel_center := center_along + n * (width * 0.5 + 0.8 + d * 0.5)
					if _parcel_ok(zone, parcel_center, radii, occ, u, front, d):
						# Marquage en retrait de 1,3 m le long de la rue : la parcelle suivante (testée en
						# retrait de 1,5 m) reste contiguë malgré les cases de 2,5 m.
						occ.rect(parcel_center, u, maxf(front * 0.5 - 1.3, 0.5), d * 0.5, false)
						_add_house(center_along, n, width, front, d, zone, rng, mixes, heights, houses, out)
						budget[key] = int(budget[key]) - 1
						placed = true
						break
				s += front if placed else 3.0
			carry = s - seg_len


## Maison sur rue d'une parcelle (façade vers la rue), dépendance au fond (cour, atelier,
## grange) et arbre du jardin selon la profondeur libre.
static func _add_house(center_along: Vector2, n: Vector2, width: float, front: float, depth: float, zone: int, rng: RandomNumberGenerator, mixes: Dictionary, heights: Heights, houses: Dictionary, out: Dictionary) -> void:
	var mix_key := "intra" if zone == ZONE_INTRA else ("faubourg" if zone == ZONE_FAUBOURG else "village")
	var hk := _pick(mixes.get(mix_key, {"townhouse": 1.0}), rng)
	var range_d: Array = HOUSE_DEPTH.get(hk, [8.0, 10.0])
	var house_depth := minf(rng.randf_range(float(range_d[0]), float(range_d[1])), depth * 0.7)
	var house_front := minf(front - (0.0 if zone == ZONE_INTRA else rng.randf_range(1.0, 3.0)), float(HOUSE_MAX_FRONT.get(hk, 12.0)))
	var hc := center_along + n * (width * 0.5 + 0.8 + house_depth * 0.5)
	# Façade (+Z du modèle) vers la rue (-n) ; `yaw` = direction de l'axe X du modèle (façade),
	# voir `TownBuilder.basis_x`.
	var yaw := atan2(n.x, -n.y)
	_append_house(houses, HOUSE_KINDS.find(hk), hc, yaw, house_front, house_depth, zone, rng.randf(), heights)
	var free_depth := depth - house_depth
	# Dépendance au fond de la parcelle (intra-muros : arrière-boutique, atelier ; ailleurs : grange).
	if free_depth > 14.0 and rng.randf() < (0.45 if zone == ZONE_INTRA else 0.35):
		var annex_kind := "stonehouse" if zone == ZONE_INTRA else "barn"
		var annex_front := minf(front - 1.0, 7.0 if zone == ZONE_INTRA else 10.0)
		var annex_depth := rng.randf_range(5.0, 7.0)
		var ac := center_along + n * (width * 0.5 + 0.8 + depth - annex_depth * 0.5 - 1.0)
		_append_house(houses, HOUSE_KINDS.find(annex_kind), ac, yaw + PI, annex_front, annex_depth, zone, rng.randf(), heights)
		free_depth -= annex_depth + 2.0
	if free_depth > 10.0 and rng.randf() < 0.4:
		var tp := center_along + n * (width * 0.5 + 0.8 + house_depth + free_depth * 0.5)
		out["trees"].append(Vector3(tp.x, tp.y, heights.height_m(tp.x, tp.y)))


static func _append_house(houses: Dictionary, kind: int, center: Vector2, yaw: float, front: float, depth: float, zone: int, tint: float, heights: Heights) -> void:
	houses["kind"].append(kind)
	houses["x"].append(center.x)
	houses["y"].append(center.y)
	houses["yaw"].append(yaw)
	houses["front"].append(front)
	houses["depth"].append(depth)
	houses["base"].append(footprint_base(heights, center, yaw, front, depth))
	houses["tint"].append(tint)
	houses["zone"].append(zone)


static func _parcel_ok(zone: int, center: Vector2, radii: PackedFloat32Array, occ: Occupancy, u: Vector2, front: float, depth: float) -> bool:
	if zone == ZONE_INTRA and not inside(radii, center, 4.0):
		return false
	# Faubourg : hors de l'enceinte (fossé et glacis dégagés).
	if zone == ZONE_FAUBOURG and inside(radii, center, -6.0):
		return false
	return occ.rect(center, u, maxf(front * 0.5 - 1.5, 0.5), maxf(depth * 0.5 - 1.5, 0.5), true)


static func _reserve_monument(m: Dictionary, occ: Occupancy, out: Dictionary, heights: Heights, rng: RandomNumberGenerator) -> void:
	var kind := str(m["kind"])
	var at := Vector2(float(m["at"][0]), float(m["at"][1]))
	var size := float(m["size_m"])
	var yaw := float(m.get("yaw", 0.0))
	var length := size
	var depth := size
	match kind:
		"cathedral", "church":
			yaw = rng.randf_range(-0.12, 0.12)  # nef est-ouest
			depth = size * 0.39
			occ.rect(at, Vector2(cos(yaw), sin(yaw)), length * 0.5 + 12.0, depth * 0.5 + 10.0, false)
		"abbey":
			yaw = rng.randf_range(-0.12, 0.12)
			depth = size * 0.75
			occ.rect(at, Vector2(cos(yaw), sin(yaw)), length * 0.5, depth * 0.5, false)
		"castle":
			occ.rect(at, Vector2(cos(yaw), sin(yaw)), length * 0.5 + 6.0, depth * 0.5 + 6.0, false)
		"hall":
			length = 23.0
			depth = 12.2
		"windmill":
			length = 7.0
			depth = 7.0
	(out["monuments"] as Array).append({"kind": kind, "x": at.x, "y": at.y, "yaw": yaw, "length": length, "depth": depth, "keep": bool(m.get("keep", false)), "base": footprint_base(heights, at, yaw, length, depth)})


static func _place_parishes(count: int, main_streets: Array, secondary: Array, occ: Occupancy, heights: Heights, rng: RandomNumberGenerator, out: Dictionary) -> void:
	if count <= 0:
		return
	var candidates: Array = []
	for s in main_streets:
		var pts: PackedVector2Array = s["points"]
		for i in range(2, mini(int(s["gate_index"]), pts.size() - 1), 3):
			candidates.append(pts[i])
	for b in secondary:
		var pb: PackedVector2Array = b
		for i in range(2, pb.size() - 1, 3):
			candidates.append(pb[i])
	if candidates.is_empty():
		candidates.append(Vector2(30.0, 30.0))
	var placed: Array[Vector2] = []
	var tries := 0
	while placed.size() < count and tries < count * 40:
		tries += 1
		var c: Vector2 = candidates[rng.randi_range(0, candidates.size() - 1)]
		var off := Vector2(rng.randf_range(-1.0, 1.0), rng.randf_range(-1.0, 1.0)).normalized() * rng.randf_range(18.0, 30.0)
		var at := c + off
		var too_close := false
		for q in placed:
			if q.distance_to(at) < 120.0:
				too_close = true
				break
		if too_close:
			continue
		var size := 19.0 if rng.randf() < 0.5 else 24.0
		var yaw := rng.randf_range(-0.15, 0.15)
		if not occ.rect(at, Vector2(cos(yaw), sin(yaw)), size * 0.5 + 4.0, size * 0.22 + 4.0, true):
			continue
		occ.rect(at, Vector2(cos(yaw), sin(yaw)), size * 0.5 + 6.0, size * 0.22 + 6.0, false)
		placed.append(at)
		(out["monuments"] as Array).append({"kind": "church", "x": at.x, "y": at.y, "yaw": yaw, "length": size, "depth": size * 0.4, "keep": false, "base": footprint_base(heights, at, yaw, size, size * 0.4)})


static func _build_walls(town: Dictionary, params: Dictionary, radii: PackedFloat32Array, gate_points: Array[Vector2], heights: Heights, out: Dictionary) -> void:
	var walls_rules: Dictionary = params.get("walls", {})
	var spec: Dictionary = walls_rules.get(str(town.get("walls", "stone")), {})
	var perimeter := 0.0
	var n := radii.size()
	for i in n:
		var a := TAU * i / n
		var b := TAU * (i + 1) / n
		perimeter += (Vector2(cos(a), sin(a)) * radii[i]).distance_to(Vector2(cos(b), sin(b)) * radii[(i + 1) % n])
	var count := maxi(24, int(perimeter / 8.0))
	var ring := PackedVector2Array()
	var bases := PackedFloat32Array()
	var gaps := PackedInt32Array()
	for i in count + 1:
		var a := TAU * i / count
		var p := Vector2(cos(a), sin(a)) * radius_at(radii, a)
		ring.append(p)
		bases.append(heights.height_m(p.x, p.y))
		var gap := 0
		for g in gate_points:
			if g.distance_to(p) < 7.0:
				gap = 1
		gaps.append(gap)
	out["wall_ring"] = ring
	out["wall_bases"] = bases
	out["wall_gaps"] = gaps
	out["wall_height"] = float(spec.get("height_m", 8.0))
	out["wall_thickness"] = float(spec.get("thickness_m", 2.0))
	var spacing := float(spec.get("tower_spacing_m", 0.0))
	var tower_r := float(spec.get("tower_radius_m", 0.0))
	var tower_h := float(spec.get("tower_height_m", 0.0))
	if spacing > 0.0 and tower_r > 0.0:
		var along := 0.0
		for i in range(1, ring.size()):
			along += ring[i].distance_to(ring[i - 1])
			if along >= spacing and gaps[i] == 0:
				along = 0.0
				var p := ring[i]
				(out["towers"] as Array).append({"x": p.x, "y": p.y, "radius": tower_r, "height": tower_h, "base": footprint_base(heights, p, 0.0, tower_r * 2.0, tower_r * 2.0)})
	for g in gate_points:
		var yaw := atan2(g.y, g.x)
		(out["gates"] as Array).append({"x": g.x, "y": g.y, "yaw": yaw, "base": footprint_base(heights, g, yaw, 10.0, 12.0), "height": float(spec.get("tower_height_m", 6.0)) + 2.0, "palisade": tower_r <= 0.0})
