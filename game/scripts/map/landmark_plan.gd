class_name LandmarkPlan
extends RefCounted

## Lot VH4 (ADR 0078) : plan 1:1 d'une ville emblématique (format v2, `LandmarkV2Library`).
## Calcul pur et déterministe (graine de la ville), exécutable dans un fil de travail. Sortie au
## format de `TownPlan.generate` (rues drapées, parcelles et maisons, murailles, tours, portes,
## pont, arbres, sol) que `TownBuilder` construit, plus :
## - `wall_rings` : enceintes polygonales quelconques (normales explicites), fermées ou non ;
## - `v2_monuments` : gabarits réels (`LandmarkMonuments`, tableaux de maillage préparés ici) ;
## - `fixed_bases` : maisons dont la base ne suit pas le relief (maisons du pont, sur le tablier).
## Les rues sont les vraies rues (OSM ou tracées à la main) ; les parcelles en lanières sont
## tirées le long de chaque rue, dans le quartier qui les contient (densité, mélange de maisons),
## sans empiéter sur l'eau, les murailles, les places, les monuments ni les autres parcelles :
## le fond des parcelles dos à dos découpe les îlots, les cours et jardins restent au cœur.
## Repère local : mètres depuis l'origine, x vers +X monde (est), y vers +Z monde (sud).
## Rendu seulement : aucune règle de jeu.

const DENSIFY_M := 10.0
const GROUND_STEP_M := 12.0
const WALL_STEP_M := 8.0
const GATE_GAP_M := 7.0
const WATER_CELL_M := 40.0

## Paramètres par défaut du parcellaire (remplacés par la section `plan` de la ville).
const DEFAULT_PLAN := {
	"frontage_m": [5.0, 8.0],
	"depth_m": [20.0, 40.0],
	"faubourg_frontage_m": [8.0, 16.0],
	"faubourg_depth_m": [25.0, 50.0],
	"street_width_m": {"main": 8.0, "secondary": 5.0, "lane": 3.0, "quay": 14.0, "square": 6.0},
	"house_kinds": {
		"intra": {"townhouse": 0.55, "timber": 0.3, "stonehouse": 0.15},
		"faubourg": {"cottage": 0.4, "timber": 0.3, "longere": 0.3},
	},
	"detail_cell_m": 250.0,
}


## Quartiers (polygones locaux) : accès par point. Index en grille (`build_index`) : chaque case
## liste les quartiers dont le rectangle la recoupe, testés dans l'ordre (le premier l'emporte,
## même résultat que le parcours complet) ; Paris compte des dizaines de quartiers (VH5).
class Districts:
	extends RefCounted

	const INDEX_CELL_M := 50.0

	var polys: Array = []  # {poly: PackedVector2Array, rect: Rect2, zone, density, houses}
	var _index: Dictionary = {}  # Vector2i → PackedInt32Array

	func add(poly: PackedVector2Array, zone: String, density: float, houses: Dictionary) -> void:
		var rect := Rect2(poly[0], Vector2.ZERO)
		for q in poly:
			rect = rect.expand(q)
		polys.append({"poly": poly, "rect": rect, "zone": zone, "density": density, "houses": houses})
		_index.clear()

	func build_index() -> void:
		_index.clear()
		for i in polys.size():
			var r: Rect2 = polys[i]["rect"]
			for cy in range(floori(r.position.y / INDEX_CELL_M), floori(r.end.y / INDEX_CELL_M) + 1):
				for cx in range(floori(r.position.x / INDEX_CELL_M), floori(r.end.x / INDEX_CELL_M) + 1):
					var key := Vector2i(cx, cy)
					var list: PackedInt32Array = _index.get(key, PackedInt32Array())
					list.append(i)
					_index[key] = list

	## Indice du premier quartier qui contient `p`, -1 sinon.
	func at(p: Vector2) -> int:
		if not _index.is_empty():
			var list: Variant = _index.get(Vector2i(floori(p.x / INDEX_CELL_M), floori(p.y / INDEX_CELL_M)))
			if list == null:
				return -1
			for i: int in list as PackedInt32Array:
				var d: Dictionary = polys[i]
				if (d["rect"] as Rect2).has_point(p) and Geometry2D.is_point_in_polygon(p, d["poly"]):
					return i
			return -1
		for i in polys.size():
			var d: Dictionary = polys[i]
			if (d["rect"] as Rect2).has_point(p) and Geometry2D.is_point_in_polygon(p, d["poly"]):
				return i
		return -1

	func bounds() -> Rect2:
		var r := Rect2()
		for i in polys.size():
			r = polys[i]["rect"] if i == 0 else r.merge(polys[i]["rect"])
		return r


## Plan complet de `city` pour l'année `year`. `heights` : échantillonneur (mètres).
static func generate(city: Dictionary, year: int, heights: TownPlan.Heights) -> Dictionary:
	var t0 := Time.get_ticks_usec()
	var plan := _merged_plan(city)
	var rng := RandomNumberGenerator.new()
	rng.seed = int(city.get("seed", 1))
	var extent := float(city.get("extent_m", 1500.0))
	var out := {
		"id": str(city.get("id", "")),
		"kind": "landmark",
		"walls": "stone",
		"radii": PackedFloat32Array([extent, extent, extent, extent]),
		"extent_m": extent,
		"streets": [],
		"houses": {"kind": PackedInt32Array(), "x": PackedFloat32Array(), "y": PackedFloat32Array(), "yaw": PackedFloat32Array(), "front": PackedFloat32Array(), "depth": PackedFloat32Array(), "base": PackedFloat32Array(), "tint": PackedFloat32Array(), "zone": PackedInt32Array()},
		"monuments": [],
		"wall_ring": PackedVector2Array(),
		"wall_bases": PackedFloat32Array(),
		"wall_gaps": PackedInt32Array(),
		"wall_rings": [],
		"towers": [],
		"gates": [],
		"trees": PackedVector3Array(),
		"bridge": {},
		"v2_monuments": [],
		"fixed_bases": {},
		"detail_cell_m": float(plan.get("detail_cell_m", 250.0)),
	}
	var occ := TownPlan.Occupancy.new(extent + 60.0)
	var districts := Districts.new()
	for d in city.get("districts", []):
		if LandmarkV2Library.present(d, year):
			districts.add(LandmarkV2Library.local_line(d["polygon"]), str(d["zone"]), float(d.get("density", 0.9)), d.get("houses", {}))
	districts.build_index()
	out["districts"] = districts
	var widths: Dictionary = plan["street_width_m"]
	# 1. Eau : couloirs interdits (fleuve de la carte fine), ruisseaux dessinés ; lits en
	# polygone (Paris : Seine de 1380 d'ALPAGE, îles en trous) interdits en entier.
	var waters: Array = []
	for w in city.get("waters", []):
		if w.has("polygon"):
			var rings: Array = [LandmarkV2Library.local_line(w["polygon"])]
			for hole in w.get("holes", []):
				rings.append(LandmarkV2Library.local_line(hole))
			var rect := Rect2(rings[0][0], Vector2.ZERO)
			for q in rings[0]:
				rect = rect.expand(q)
			waters.append({"rings": rings, "rect": rect})
			_fill_rings(occ, rings, rect)
			continue
		var line := PackedVector2Array()
		var ws := PackedFloat32Array()
		for q in w["points"]:
			line.append(Vector2(float(q[0]), -float(q[1])))
			ws.append(float(q[2]))
		waters.append({"line": line, "widths": ws})
		for i in line.size() - 1:
			occ.segment(line[i], line[i + 1], (ws[i] + ws[i + 1]) * 0.25 + 4.0)
		if bool(w.get("draw", false)):
			var dense := _densify(line, DENSIFY_M)
			var water_street := {"points": dense, "width": ws[0], "water": true}
			TownPlan.street_bases(water_street, heights)
			(out["streets"] as Array).append(water_street)
	out["waters"] = waters
	var marks := {}
	marks["water"] = Time.get_ticks_usec() - t0
	# 2. Monuments (emprise réservée avant tout le reste).
	for m in city.get("monuments", []):
		if LandmarkV2Library.present(m, year):
			_add_monument(m, occ, heights, out)
	marks["monuments"] = Time.get_ticks_usec() - t0
	# 3. Murailles, tours, portes.
	for wall in city.get("walls", []):
		if LandmarkV2Library.present(wall, year):
			_add_wall(wall, occ, heights, out)
	# 4. Pont(s).
	for b in city.get("bridges", []):
		if LandmarkV2Library.present(b, year):
			_add_bridge(b, occ, heights, rng, out)
	# 5. Quais : grève pavée sans maisons.
	for q in city.get("quays", []):
		var qline := _densify(LandmarkV2Library.local_line(q["points"]), DENSIFY_M)
		var qw := float(q.get("width_m", widths.get("quay", 14.0)))
		for i in qline.size() - 1:
			occ.segment(qline[i], qline[i + 1], qw * 0.5)
		var quay := {"points": qline, "width": qw, "paved": true}
		TownPlan.street_bases(quay, heights)
		(out["streets"] as Array).append(quay)
	# 6. Places, marchés, cimetières, jardins : sans maisons ; places pavées.
	for s in city.get("open_spaces", []):
		if LandmarkV2Library.present(s, year):
			_add_open_space(s, occ, heights, rng, out)
	marks["walls_spaces"] = Time.get_ticks_usec() - t0
	# 7. Rues réelles.
	var ranked: Array = []
	for s in city.get("streets", []):
		if not LandmarkV2Library.present(s, year):
			continue
		var rank := str(s.get("rank", "secondary"))
		var width := float(s.get("width_m", widths.get(rank, 5.0)))
		var pts := _densify(LandmarkV2Library.local_line(s["points"]), DENSIFY_M)
		if pts.size() < 2:
			continue
		for i in pts.size() - 1:
			occ.segment(pts[i], pts[i + 1], width * 0.5 + 0.5)
		var street := {"points": pts, "width": width, "rank": rank}
		TownPlan.street_bases(street, heights)
		(out["streets"] as Array).append(street)
		ranked.append(street)
	ranked.sort_custom(func(a: Dictionary, b: Dictionary) -> bool: return _rank_order(a["rank"]) < _rank_order(b["rank"]))
	marks["streets"] = Time.get_ticks_usec() - t0
	# 8. Parcellaire importé (Paris : parcelles Vasserot d'ALPAGE, VH5), puis parcelles en
	# lanières tirées le long des rues pour les façades restées libres.
	var imported: Array = city.get("parcels", [])
	if not imported.is_empty():
		_imported_parcels(imported, districts, occ, heights, rng, plan, out)
		marks["imported"] = Time.get_ticks_usec() - t0
	for s in ranked:
		_line_parcels(s["points"], float(s["width"]), s["rank"] == "main", districts, occ, heights, rng, plan, out)
	marks["parcels"] = Time.get_ticks_usec() - t0
	out["water_index"] = _water_index(waters)
	out["ground"] = _ground_grid(districts, waters, heights, out["houses"], out["water_index"])
	out["stats"] = {"houses": (out["houses"]["x"] as PackedFloat32Array).size(), "streets": (out["streets"] as Array).size(), "monuments": (out["v2_monuments"] as Array).size(), "towers": (out["towers"] as Array).size(), "usec": Time.get_ticks_usec() - t0, "marks_usec": marks}
	return out


static func _merged_plan(city: Dictionary) -> Dictionary:
	var plan := DEFAULT_PLAN.duplicate(true)
	var own: Dictionary = city.get("plan", {})
	for key in own:
		if key == "street_width_m":
			for k in own[key]:
				plan[key][k] = own[key][k]
		else:
			plan[key] = own[key]
	return plan


static func _rank_order(rank: String) -> int:
	return {"main": 0, "secondary": 1, "lane": 2}.get(rank, 3)


static func _densify(line: PackedVector2Array, step: float) -> PackedVector2Array:
	var out := PackedVector2Array()
	for i in line.size():
		if i == 0:
			out.append(line[0])
			continue
		var a := line[i - 1]
		var b := line[i]
		var n := maxi(1, int(ceil(a.distance_to(b) / step)))
		for k in range(1, n + 1):
			out.append(a.lerp(b, float(k) / n))
	return out


# --- Monuments ------------------------------------------------------------------------------


static func _add_monument(m: Dictionary, occ: TownPlan.Occupancy, heights: TownPlan.Heights, out: Dictionary) -> void:
	var at := LandmarkV2Library.local(m["at"])
	var yaw := LandmarkV2Library.yaw_of(float(m.get("angle_deg", 0.0)))
	var fp := LandmarkMonuments.footprint(m)
	var clear := float(m.get("clear_m", 6.0))
	var u := Vector2(cos(yaw), sin(yaw))
	occ.rect(at, u, fp.x * 0.5 + clear, fp.y * 0.5 + clear, false)
	var built := LandmarkMonuments.build(m)
	(out["v2_monuments"] as Array).append({
		"id": str(m["id"]), "model": str(m["model"]), "x": at.x, "y": at.y, "yaw": yaw,
		"length": fp.x, "depth": fp.y, "base": TownPlan.footprint_base(heights, at, yaw, fp.x, fp.y),
		"arrays": built["arrays"], "top": float(built["top"]),
	})


# --- Murailles ------------------------------------------------------------------------------


static func _add_wall(wall: Dictionary, occ: TownPlan.Occupancy, heights: TownPlan.Heights, out: Dictionary) -> void:
	var pts := LandmarkV2Library.local_line(wall["points"])
	var closed := bool(wall.get("closed", false))
	if closed:
		pts.append(pts[0])
	var ring := _densify(pts, WALL_STEP_M)
	var gates: Array[Vector2] = []
	var gate_meta: Array = []
	for g in wall.get("gates", []):
		gates.append(LandmarkV2Library.local(g["at"]))
		gate_meta.append(g)
	var normals := PackedVector2Array()
	var bases := PackedFloat32Array()
	var gaps := PackedInt32Array()
	var outline := LandmarkV2Library.local_line(wall["points"])
	for i in ring.size():
		var t := (ring[mini(i + 1, ring.size() - 1)] - ring[maxi(i - 1, 0)]).normalized()
		var n := Vector2(-t.y, t.x)
		# Normale vers l'extérieur de l'enceinte fermée (fossé de ce côté).
		if closed and Geometry2D.is_point_in_polygon(ring[i] + n * 6.0, outline):
			n = -n
		normals.append(n)
		bases.append(heights.height_m(ring[i].x, ring[i].y))
		var gap := 0
		for g in gates:
			if g.distance_to(ring[i]) < GATE_GAP_M:
				gap = 1
		gaps.append(gap)
	var thickness := float(wall.get("thickness_m", 2.4))
	var height := float(wall["height_m"])
	var ditch := float(wall.get("ditch_m", 0.0))
	for i in ring.size() - 1:
		occ.segment(ring[i], ring[i + 1], thickness * 0.5 + 4.0)
		if ditch > 0.0:
			var off := normals[i] * (thickness * 0.5 + ditch * 0.5 + 1.0)
			occ.segment(ring[i] + off, ring[i + 1] + off, ditch * 0.5)
	(out["wall_rings"] as Array).append({"ring": ring, "normals": normals, "bases": bases, "gaps": gaps, "height": height, "thickness": thickness})
	# Tours régulières (hors des portes).
	var spacing := float(wall.get("tower_spacing_m", 0.0))
	var tower_r := float(wall.get("tower_radius_m", 0.0))
	var tower_h := float(wall.get("tower_height_m", height + 4.0))
	if spacing > 0.0 and tower_r > 0.0:
		var along := spacing * 0.5
		for i in range(1, ring.size()):
			along += ring[i].distance_to(ring[i - 1])
			if along < spacing or gaps[i] == 1:
				continue
			var near_gate := false
			for g in gates:
				if g.distance_to(ring[i]) < 18.0:
					near_gate = true
			if near_gate:
				continue
			along = 0.0
			var p := ring[i] + normals[i] * (thickness * 0.5 + tower_r * 0.35)
			(out["towers"] as Array).append({"x": p.x, "y": p.y, "radius": tower_r, "height": tower_h, "base": TownPlan.footprint_base(heights, p, 0.0, tower_r * 2.0, tower_r * 2.0)})
	# Portes : châtelet traversé par la rue (axe = normale du mur).
	for k in gates.size():
		var g := gates[k]
		var best := 0
		for i in ring.size():
			if ring[i].distance_squared_to(g) < ring[best].distance_squared_to(g):
				best = i
		var n := normals[best]
		var yaw := atan2(n.y, n.x)
		var h := float(gate_meta[k].get("height_m", tower_h + 2.0))
		(out["gates"] as Array).append({"x": g.x, "y": g.y, "yaw": yaw, "base": TownPlan.footprint_base(heights, g, yaw, 12.0, 11.0), "height": h, "palisade": false, "name": str(gate_meta[k]["name"])})
		occ.rect(g, Vector2(cos(yaw), sin(yaw)), 10.0, 9.0, false)


# --- Ponts ----------------------------------------------------------------------------------


static func _add_bridge(b: Dictionary, occ: TownPlan.Occupancy, heights: TownPlan.Heights, rng: RandomNumberGenerator, out: Dictionary) -> void:
	var a := LandmarkV2Library.local(b["from"])
	var c := LandmarkV2Library.local(b["to"])
	var d := (c - a).normalized()
	var length := a.distance_to(c)
	var width := float(b["width_m"])
	var mid := (a + c) * 0.5
	var bank := minf(heights.height_m(a.x, a.y), heights.height_m(c.x, c.y))
	var water := heights.height_m(mid.x, mid.y)
	for k in 5:
		water = minf(water, heights.height_m(lerpf(a.x, c.x, 0.3 + 0.1 * k), lerpf(a.y, c.y, 0.3 + 0.1 * k)))
	var deck := maxf(bank, water + float(b.get("deck_m", 7.0)))
	out["bridge"] = {"x": mid.x, "y": mid.y, "yaw": atan2(d.y, d.x), "length": length + 8.0, "width": width, "deck": deck, "water": water, "arches": int(b.get("arches", maxi(1, int(length / 18.0))))}
	occ.segment(a - d * 10.0, c + d * 10.0, width * 0.5 + 1.0)
	var n := Vector2(-d.y, d.x)
	# Maisons du pont : façade sur le tablier, en encorbellement au-dessus de l'eau.
	if bool(b.get("houses", false)):
		var span: Array = b.get("house_span", [0.15, 0.85])
		var s := float(span[0]) * length
		var houses: Dictionary = out["houses"]
		var fixed: Dictionary = out["fixed_bases"]
		while s < float(span[1]) * length:
			var front := rng.randf_range(5.0, 7.0)
			for side: float in [-1.0, 1.0]:
				var nn := n * side
				var hd := rng.randf_range(6.0, 8.0)
				var center := a + d * (s + front * 0.5) + nn * (width * 0.5 + hd * 0.5)
				fixed[houses["x"].size()] = true
				houses["kind"].append(TownPlan.HOUSE_KINDS.find("timber"))
				houses["x"].append(center.x)
				houses["y"].append(center.y)
				houses["yaw"].append(atan2(nn.x, -nn.y))
				houses["front"].append(front - 0.3)
				houses["depth"].append(hd)
				houses["base"].append(deck - 0.2)
				houses["tint"].append(rng.randf())
				houses["zone"].append(TownPlan.ZONE_INTRA)
			s += front
	for f in b.get("gatehouses_at", []):
		var p := a + d * float(f) * length
		(out["gates"] as Array).append({"x": p.x, "y": p.y, "yaw": atan2(d.y, d.x), "base": deck - 0.5, "height": 13.0, "palisade": false, "fixed": true})


# --- Places et espaces libres ---------------------------------------------------------------


static func _add_open_space(s: Dictionary, occ: TownPlan.Occupancy, heights: TownPlan.Heights, rng: RandomNumberGenerator, out: Dictionary) -> void:
	var poly := LandmarkV2Library.local_line(s["polygon"])
	var rect := Rect2(poly[0], Vector2.ZERO)
	for q in poly:
		rect = rect.expand(q)
	var kind := str(s.get("kind", "square"))
	var paved := kind in ["square", "market", "parvis", "strand"]
	var step := TownPlan.CELL_M
	var y := rect.position.y + step * 0.5
	while y < rect.end.y:
		var x := rect.position.x + step * 0.5
		var run_start := INF
		while x < rect.end.x + step:
			var p := Vector2(x, y)
			var inside := x < rect.end.x and Geometry2D.is_point_in_polygon(p, poly)
			if inside:
				occ.mark(p)
				if run_start == INF:
					run_start = x
			elif run_start != INF:
				if paved:
					var strip := {"points": PackedVector2Array([Vector2(run_start - step * 0.5, y), Vector2(x - step * 0.5, y)]), "width": step + 0.4, "paved": true}
					TownPlan.street_bases(strip, heights)
					(out["streets"] as Array).append(strip)
				run_start = INF
			x += step
		y += step
	# Arbres des jardins, prés et cimetières.
	var tree_every := {"garden": 90.0, "cemetery": 160.0, "meadow": 400.0, "cloister": 250.0, "vineyard": 0.0}
	var every := float(tree_every.get(kind, 0.0))
	if every > 0.0:
		var count := int(rect.get_area() / every)
		for k in count:
			var p := Vector2(rng.randf_range(rect.position.x, rect.end.x), rng.randf_range(rect.position.y, rect.end.y))
			if Geometry2D.is_point_in_polygon(p, poly):
				(out["trees"] as PackedVector3Array).append(Vector3(p.x, p.y, heights.height_m(p.x, p.y)))


# --- Parcelles ------------------------------------------------------------------------------


## Parcelles des deux côtés d'une rue : façade et profondeur tirées selon la zone du quartier,
## maison sur rue (façade du modèle vers la rue), jardin ou cour derrière (`TownPlan._add_house`).
static func _line_parcels(pts: PackedVector2Array, width: float, main: bool, districts: Districts, occ: TownPlan.Occupancy, heights: TownPlan.Heights, rng: RandomNumberGenerator, plan: Dictionary, out: Dictionary) -> void:
	var houses: Dictionary = out["houses"]
	var mixes: Dictionary = plan["house_kinds"]
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
				var probe := mid + n * (width * 0.5 + 6.0)
				var di := districts.at(probe)
				if di < 0:
					s += 4.0
					continue
				var dist: Dictionary = districts.polys[di]
				var faubourg := str(dist["zone"]) == "faubourg"
				var fr: Array = plan["faubourg_frontage_m" if faubourg else "frontage_m"]
				var dp: Array = plan["faubourg_depth_m" if faubourg else "depth_m"]
				var front := rng.randf_range(float(fr[0]), float(fr[1]))
				if rng.randf() > float(dist["density"]):
					s += front  # façade libre : entrée de cour, jardin
					continue
				var depth := rng.randf_range(float(dp[0]), float(dp[1]))
				if not main and not faubourg:
					depth *= 0.85
				var center_along := mid + u * (front * 0.5)
				var placed := false
				for factor: float in [1.0, 0.7, 0.5, 0.35]:
					var d := maxf(depth * factor, 9.0)
					var parcel_center := center_along + n * (width * 0.5 + 0.8 + d * 0.5)
					if districts.at(parcel_center) != di:
						continue
					# Test sans la bande de 2,5 m sur rue : les cases de la rue (arrondies à 2,5 m)
					# débordent sur le front des parcelles, qui toucheraient sinon toujours la rue.
					var test_center := center_along + n * (width * 0.5 + 0.8 + 2.5 + (d - 2.5) * 0.5)
					if occ.rect(test_center, u, maxf(front * 0.5 - 1.5, 0.5), maxf((d - 2.5) * 0.5 - 1.5, 0.5), true):
						occ.rect(parcel_center, u, maxf(front * 0.5 - 1.3, 0.5), d * 0.5, false)
						var zone := TownPlan.ZONE_FAUBOURG if faubourg else TownPlan.ZONE_INTRA
						var mix: Dictionary = dist["houses"] if not (dist["houses"] as Dictionary).is_empty() else mixes.get("faubourg" if faubourg else "intra", {"townhouse": 1.0})
						TownPlan._add_house(center_along, n, width, front, d, zone, rng, {"intra": mix, "faubourg": mix}, heights, houses, out)
						placed = true
						break
				s += front if placed else 3.0
			carry = s - seg_len


## Parcellaire importé (section `parcels` : [dE, dN, angle°, façade, profondeur], milieu de la
## façade sur rue et normale vers l'intérieur de la parcelle). Une façade longue est partagée en
## plusieurs maisons (≤ 11 m) ; la parcelle doit être libre (monuments, murailles, rues, eau), sa
## profondeur est réduite sinon ; densité et mélange de maisons du quartier.
static func _imported_parcels(items: Array, districts: Districts, occ: TownPlan.Occupancy, heights: TownPlan.Heights, rng: RandomNumberGenerator, plan: Dictionary, out: Dictionary) -> void:
	var houses: Dictionary = out["houses"]
	var mixes: Dictionary = plan["house_kinds"]
	for item: Array in items:
		var p := LandmarkV2Library.local([item[0], item[1]])
		var yaw := LandmarkV2Library.yaw_of(float(item[2]))
		var n := Vector2(cos(yaw), sin(yaw))
		var t := Vector2(-n.y, n.x)
		var di := districts.at(p + n * 3.0)
		if di < 0:
			continue
		var dist: Dictionary = districts.polys[di]
		var faubourg := str(dist["zone"]) == "faubourg"
		var zone := TownPlan.ZONE_FAUBOURG if faubourg else TownPlan.ZONE_INTRA
		var mix: Dictionary = dist["houses"] if not (dist["houses"] as Dictionary).is_empty() else mixes.get("faubourg" if faubourg else "intra", {"townhouse": 1.0})
		var front_total := float(item[3])
		var depth := float(item[4])
		var k := maxi(1, ceili(front_total / 11.0))
		var front := front_total / k
		for j in k:
			if rng.randf() > float(dist["density"]):
				continue
			var center_along := p + t * (-front_total * 0.5 + front * (j + 0.5))
			for factor: float in [1.0, 0.6, 0.4]:
				var d := maxf(depth * factor, 6.0)
				var test_center := center_along + n * (1.5 + (d - 1.5) * 0.5)
				if occ.rect(test_center, t, maxf(front * 0.5 - 0.8, 0.5), maxf((d - 1.5) * 0.5 - 0.8, 0.5), true):
					occ.rect(center_along + n * (d * 0.5), t, maxf(front * 0.5 - 0.3, 0.5), d * 0.5, false)
					TownPlan._add_house(center_along, n, 0.0, front, d, zone, rng, {"intra": mix, "faubourg": mix}, heights, houses, out)
					break
				if d <= 6.0:
					break


# --- Sol ------------------------------------------------------------------------------------


## Sol des quartiers (cours, jardins, terre battue) : grille drapée carrée couvrant les
## quartiers, sommets dans un quartier et hors de l'eau ; `edge` : 1 en ville close, 0,4 aux
## faubourgs (teinte des cours plus verte).
static func _ground_grid(districts: Districts, waters: Array, heights: TownPlan.Heights, houses: Dictionary, water_index: Dictionary = {}) -> Dictionary:
	# Faubourgs : sol de la ville seulement autour des maisons (cases de 30 m), les champs du
	# parcellaire ZG5b restent visibles entre les rues.
	var near := {}
	var xs: PackedFloat32Array = houses["x"]
	var ys: PackedFloat32Array = houses["y"]
	for i in xs.size():
		near[Vector2i(floori(xs[i] / 30.0), floori(ys[i] / 30.0))] = true
	var box := districts.bounds().grow(GROUND_STEP_M)
	var side := maxf(box.size.x, box.size.y)
	var step := GROUND_STEP_M
	var n := int(ceil(side / step)) + 1
	var origin := box.position
	var mask := PackedByteArray()
	mask.resize(n * n)
	var h := PackedFloat32Array()
	h.resize(n * n)
	var edge := PackedFloat32Array()
	edge.resize(n * n)
	for j in n:
		for i in n:
			var p := origin + Vector2(i, j) * step
			var di := districts.at(p)
			if di < 0 or _in_water(p, waters, water_index):
				continue
			if str(districts.polys[di]["zone"]) == "faubourg" and not near.has(Vector2i(floori(p.x / 30.0), floori(p.y / 30.0))):
				continue
			var k := j * n + i
			mask[k] = 1
			h[k] = heights.height_m(p.x, p.y)
			edge[k] = 1.0 if str(districts.polys[di]["zone"]) == "intra" else 0.4
	return {"origin": origin, "step": step, "n": n, "inside": mask, "heights": h, "edge": edge, "radii": PackedFloat32Array([side, side, side, side])}


## Vrai si `p` est dans le lit d'un cours d'eau (demi-largeur). `index` (`_water_index`) : ne
## teste que les segments proches (même résultat).
static func _in_water(p: Vector2, waters: Array, index: Dictionary = {}) -> bool:
	if not index.is_empty():
		var list: Variant = index.get(Vector2i(floori(p.x / WATER_CELL_M), floori(p.y / WATER_CELL_M)))
		if list == null:
			return false
		var ids: PackedInt32Array = list
		for k in range(0, ids.size(), 2):
			var w: Dictionary = waters[ids[k]]
			var i := ids[k + 1]
			if i == -2:
				return true
			if i < 0:
				if _in_rings(p, w):
					return true
				continue
			var line: PackedVector2Array = w["line"]
			var ws: PackedFloat32Array = w["widths"]
			var q := Geometry2D.get_closest_point_to_segment(p, line[i], line[i + 1])
			if q.distance_to(p) < ws[i] * 0.5:
				return true
		return false
	for w in waters:
		if w.has("rings"):
			if _in_rings(p, w):
				return true
			continue
		var line: PackedVector2Array = w["line"]
		var ws: PackedFloat32Array = w["widths"]
		for i in line.size() - 1:
			var q := Geometry2D.get_closest_point_to_segment(p, line[i], line[i + 1])
			if q.distance_to(p) < ws[i] * 0.5:
				return true
	return false


## Lit en polygone : dans l'anneau extérieur et hors des îles (règle pair-impair).
static func _in_rings(p: Vector2, w: Dictionary) -> bool:
	if not (w["rect"] as Rect2).has_point(p):
		return false
	var inside := false
	for ring: PackedVector2Array in w["rings"]:
		if Geometry2D.is_point_in_polygon(p, ring):
			inside = not inside
	return inside


## Marque les cases d'occupation dans des anneaux (pair-impair : les trous restent libres),
## par balayage de lignes.
static func _fill_rings(occ: TownPlan.Occupancy, rings: Array, rect: Rect2) -> void:
	var step := TownPlan.CELL_M
	var y := rect.position.y + step * 0.5
	while y < rect.end.y:
		var xs := PackedFloat32Array()
		for ring: PackedVector2Array in rings:
			var n := ring.size()
			for i in n:
				var a := ring[i]
				var b := ring[(i + 1) % n]
				if (a.y <= y) != (b.y <= y):
					xs.append(a.x + (y - a.y) / (b.y - a.y) * (b.x - a.x))
		xs.sort()
		for k in range(0, xs.size() - 1, 2):
			var x := xs[k] + step * 0.5
			while x < xs[k + 1]:
				occ.mark(Vector2(x, y))
				x += step
		y += step


## Index en grille des segments d'eau : case → paires (cours d'eau, segment) dont la boîte,
## élargie de la demi-largeur, recoupe la case.
static func _water_index(waters: Array) -> Dictionary:
	var index := {}
	for wi in waters.size():
		if waters[wi].has("rings"):
			# Cases traversées par une rive : test exact (-1) ; cases entièrement dans le lit : -2.
			var edge_cells := {}
			for ring: PackedVector2Array in waters[wi]["rings"]:
				for i in ring.size():
					var a := ring[i]
					var b := ring[(i + 1) % ring.size()]
					var steps := maxi(1, ceili(a.distance_to(b) / (WATER_CELL_M * 0.25)))
					for k in steps + 1:
						var q := a.lerp(b, float(k) / steps)
						edge_cells[Vector2i(floori(q.x / WATER_CELL_M), floori(q.y / WATER_CELL_M))] = true
			var rr: Rect2 = waters[wi]["rect"]
			for cy in range(floori(rr.position.y / WATER_CELL_M), floori(rr.end.y / WATER_CELL_M) + 1):
				for cx in range(floori(rr.position.x / WATER_CELL_M), floori(rr.end.x / WATER_CELL_M) + 1):
					var key := Vector2i(cx, cy)
					var code := -1
					if not edge_cells.has(key):
						if not _in_rings(Vector2(cx + 0.5, cy + 0.5) * WATER_CELL_M, waters[wi]):
							continue
						code = -2
					var list: PackedInt32Array = index.get(key, PackedInt32Array())
					list.append(wi)
					list.append(code)
					index[key] = list
			continue
		var line: PackedVector2Array = waters[wi]["line"]
		var ws: PackedFloat32Array = waters[wi]["widths"]
		for i in line.size() - 1:
			var r := Rect2(line[i], Vector2.ZERO).expand(line[i + 1]).grow(ws[i] * 0.5 + 1.0)
			for cy in range(floori(r.position.y / WATER_CELL_M), floori(r.end.y / WATER_CELL_M) + 1):
				for cx in range(floori(r.position.x / WATER_CELL_M), floori(r.end.x / WATER_CELL_M) + 1):
					var key := Vector2i(cx, cy)
					var list: PackedInt32Array = index.get(key, PackedInt32Array())
					list.append(wi)
					list.append(i)
					index[key] = list
	return index


# --- Hauteurs -------------------------------------------------------------------------------


## Recalcule les hauteurs de base (pages de relief plus fines arrivées), comme `TownPlan.reground`.
static func reground(plan: Dictionary, heights: TownPlan.Heights) -> void:
	var houses: Dictionary = plan["houses"]
	var xs: PackedFloat32Array = houses["x"]
	var bases: PackedFloat32Array = houses["base"]
	var fixed: Dictionary = plan.get("fixed_bases", {})
	for i in xs.size():
		if not fixed.has(i):
			bases[i] = TownPlan.footprint_base(heights, Vector2(xs[i], houses["y"][i]), houses["yaw"][i], houses["front"][i], houses["depth"][i])
	houses["base"] = bases
	for m in plan["v2_monuments"]:
		m["base"] = TownPlan.footprint_base(heights, Vector2(m["x"], m["y"]), m["yaw"], m["length"], m["depth"])
	for s in plan["streets"]:
		TownPlan.street_bases(s, heights)
	for r in plan["wall_rings"]:
		var ring: PackedVector2Array = r["ring"]
		var wb := PackedFloat32Array()
		for p in ring:
			wb.append(heights.height_m(p.x, p.y))
		r["bases"] = wb
	for tower in plan["towers"]:
		tower["base"] = TownPlan.footprint_base(heights, Vector2(tower["x"], tower["y"]), 0.0, tower["radius"] * 2.0, tower["radius"] * 2.0)
	for gate in plan["gates"]:
		if not bool(gate.get("fixed", false)):
			gate["base"] = TownPlan.footprint_base(heights, Vector2(gate["x"], gate["y"]), gate["yaw"], 12.0, 11.0)
	var trees: PackedVector3Array = plan["trees"]
	for i in trees.size():
		trees[i].z = heights.height_m(trees[i].x, trees[i].y)
	plan["trees"] = trees
	var ground: Dictionary = plan.get("ground", {})
	if not ground.is_empty():
		var n: int = ground["n"]
		var mask: PackedByteArray = ground["inside"]
		var gh: PackedFloat32Array = ground["heights"]
		var origin: Vector2 = ground["origin"]
		var step := float(ground["step"])
		for k in n * n:
			if mask[k] == 1:
				var p := origin + Vector2(k % n, k / n) * step
				gh[k] = heights.height_m(p.x, p.y)
		ground["heights"] = gh
