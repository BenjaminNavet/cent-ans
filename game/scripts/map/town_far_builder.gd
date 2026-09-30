class_name TownFarBuilder
extends RefCounted

## Lot VT-B (ADR 0138) : maillage lointain des villes à l'échelle 1:1, sans `TownPlan`.
## - F1 (rig < ~300) : nappe de toits polaire sur les relèvements de `radii` (centre + 2 anneaux),
##   jupe, faubourgs, enceinte (mur, chemin de ronde, tours, portes), monuments simplifiés ;
## - F2 (au-delà) : polygone à 16 côtés, jupe, une flèche au plus ;
## - villes v2 (`data/landmarks_v2/`) : quartiers, enceintes et plus grands monuments.
## Fonctions statiques pures (aucun nœud, aucun accès fichier) : appelables depuis un fil de
## travail. `LandmarkV2Library.anchor_units` lit un fichier au premier appel : le fil principal
## doit l'avoir déjà appelé, ou passer `anchor` à `build_v2_far`.
##
## Contrat de sommets (partagé avec `town_far.gdshader`, lot VT-C) :
## - VERTEX.x / z : position monde en unités carte (ancre `px` + mètres locaux / m par unité ;
##   repère local de `TownPlan` : x vers +X monde (est), y vers +Z monde) ;
## - VERTEX.y : altitude ABSOLUE en mètres (sol + hauteur du bâti ; pied de jupe = sol − 30) ;
## - UV2.x : index de ville (masque d'enfoncement) ; UV2.y : hauteur (m) au-dessus du sol local
##   (0 au pied, −30 au bas de la jupe) ;
## - COLOR : teinte (rgb), alpha 1 ; NORMAL : calculée dans l'espace en mètres (non exagéré).
## Sol : `ground_m` (65 valeurs : [0] centre, [1..n] à 0,5 × radii[k], [n+1..2n] à radii[k]),
## sinon `z_m` partout. Relèvement k : angle k · 2π / n, direction (cos, sin) du repère local,
## comme `TownPlan.radius_at`.
## Faces avant : sens horaire vu de l'extérieur (convention Godot), normale = (c − a) × (b − a).
## Rendu seulement.

## Profondeur de la jupe sous le sol (m) : masque l'écart avec le relief.
const SKIRT_M := 30.0
## Égout de la nappe de toits (m au-dessus du sol) par type de colonie (maisons 1340 : 8-12 m).
const EAVE_M := {"city": 11.0, "town": 9.5, "village": 6.5, "castle": 9.0, "abbey": 9.0}
const EAVE_DEFAULT_M := 9.0
## Surélévation du cœur de la nappe (faîtages, maisons plus hautes au centre), en m.
const CROWN_M := 3.0
## Variation d'égout par relèvement (± m), déterministe par ville.
const EAVE_JITTER_M := 1.0
## Côté (m) des cellules qui découpent la nappe d'un quartier v2 (sommets posés sur le relief).
const DISTRICT_CELL_M := 200.0
## Fraction du rayon de l'anneau intérieur (convention de `ground_m`).
const INNER_RING := 0.5
## Bord de la nappe en fraction du rayon quand la ville est close (le mur est au rayon).
const WALLED_EDGE := 0.97
## Enceinte de repli (les valeurs de `towns_1340.json` → `walls` priment, 4e argument de F1).
const WALL_DEFAULTS := {
	"stone": {"height_m": 9.0, "thickness_m": 2.2, "tower_height_m": 14.0, "tower_radius_m": 4.5},
	"palisade": {"height_m": 4.5, "thickness_m": 0.6, "tower_height_m": 0.0, "tower_radius_m": 0.0},
}
## Une tour tous les N relèvements (32 / 4 = 8 tours) ; les autres ne se voient pas de loin.
const TOWER_EVERY := 4
## Porte : côté (m) et surplomb au-dessus du mur (m).
const GATE_SIZE_M := 12.0
const GATE_EXTRA_M := 5.0
## Faubourgs : largeur (deux rangs de parcelles + la rue), égout, faîtage (m).
const FAUBOURG_WIDTH_M := 56.0
const FAUBOURG_EAVE_M := 7.0
const FAUBOURG_RIDGE_M := 10.0
## Au-delà de cette longueur (m), un faubourg est coupé en deux blocs pour suivre le relief.
const FAUBOURG_SPLIT_M := 150.0
## Cathédrale (`size_m` = longueur, 60-130 m) : nef, tour occidentale 60-90 m, flèche.
const CATHEDRAL_WIDTH := 0.28
const CATHEDRAL_EAVE_M := 22.0
const CATHEDRAL_RIDGE_M := 34.0
const CATHEDRAL_TOWER_M := Vector2(60.0, 90.0)
const CATHEDRAL_SPIRE_M := 25.0
## Abbaye (`size_m` = enclos, 110-140 m) : église d'environ la moitié, tour de croisée.
const ABBEY_CHURCH := 0.5
const ABBEY_EAVE_M := 16.0
const ABBEY_RIDGE_M := 25.0
const ABBEY_TOWER_M := 38.0
const ABBEY_SPIRE_M := 14.0
## Château (`size_m` = enceinte, 60-75 m) : donjon carré.
const KEEP_FRACTION := 0.2
const KEEP_HEIGHT_M := 28.0
## Halle et moulin.
const HALL_EAVE_M := 9.0
const HALL_RIDGE_M := 17.0
const WINDMILL_HEIGHT_M := 10.0
const WINDMILL_CAP_M := 4.0
## Églises paroissiales (villes et cités) : clochers, un par paroisse, plafonnés.
const PARISH_MAX := 4
const PARISH_TOWER_M := 30.0
const PARISH_SPIRE_M := 16.0
const PARISH_HALF_M := 3.5
## F2 : nombre de côtés du polygone.
const F2_SIDES := 16
## Villes v2 : égout par zone de quartier, nombre de monuments retenus.
const V2_EAVE_M := {"intra": 11.0, "faubourg": 8.0}
const V2_EAVE_DEFAULT_M := 9.0
const V2_MAX_MONUMENTS := 40
const V2_YEAR := 1340

## Teintes (rgb). Tuile par défaut ; chaume pour les villages ; murs de pierre.
const ROOF_TILE := Color(0.60, 0.33, 0.24)
const ROOF_THATCH := Color(0.55, 0.47, 0.31)
const FACADE := Color(0.68, 0.62, 0.53)
const STONE := Color(0.72, 0.68, 0.60)
const CHURCH_STONE := Color(0.78, 0.74, 0.66)
const LEAD := Color(0.38, 0.40, 0.44)
const TIMBER := Color(0.45, 0.36, 0.26)


## Accumulateur de sommets : positions en mètres locaux gardées à part pour les normales.
class Acc:
	var anchor := Vector2.ZERO
	var inv_mpu := 1.0 / 719.0
	var town_index := 0.0
	var pm := PackedVector3Array()  # mètres : (x local, y absolu, z local)
	var vertices := PackedVector3Array()
	var normals := PackedVector3Array()
	var colors := PackedColorArray()
	var uv2 := PackedVector2Array()
	var indices := PackedInt32Array()
	var y_min := INF
	var y_max := -INF

	func _init(p_anchor: Vector2, meters_per_unit: float, index: int) -> void:
		anchor = p_anchor
		inv_mpu = 1.0 / maxf(meters_per_unit, 1e-6)
		town_index = float(index)

	## Sommet au point local `p` (m), sol `ground` (m absolu), hauteur `rel` au-dessus du sol.
	func vert(p: Vector2, ground: float, rel: float, normal: Vector3, color: Color) -> int:
		var y := ground + rel
		pm.append(Vector3(p.x, y, p.y))
		vertices.append(Vector3(anchor.x + p.x * inv_mpu, y, anchor.y + p.y * inv_mpu))
		normals.append(normal)
		colors.append(color)
		uv2.append(Vector2(town_index, rel))
		y_min = minf(y_min, y)
		y_max = maxf(y_max, y)
		return pm.size() - 1

	## Triangle orienté vers `hint` (direction extérieure approximative, m).
	func tri(a: int, b: int, c: int, hint: Vector3) -> void:
		var n := (pm[c] - pm[a]).cross(pm[b] - pm[a])
		if n.dot(hint) < 0.0:
			indices.append(a)
			indices.append(c)
			indices.append(b)
		else:
			indices.append(a)
			indices.append(b)
			indices.append(c)

	## Face plane (3 ou 4 sommets) à normale plate. `pts` : [[Vector2 local, sol, rel], ...].
	func face(pts: Array, hint: Vector3, color: Color) -> void:
		var p: Array[Vector3] = []
		for q: Array in pts:
			var xy: Vector2 = q[0]
			p.append(Vector3(xy.x, float(q[1]) + float(q[2]), xy.y))
		var n := (p[2] - p[0]).cross(p[1] - p[0])
		if pts.size() == 4 and n.length_squared() < 1e-8:
			n = (p[3] - p[0]).cross(p[2] - p[0])
		if n.length_squared() < 1e-8:
			return
		if n.dot(hint) < 0.0:
			n = -n
		n = n.normalized()
		var ids: Array[int] = []
		for q: Array in pts:
			ids.append(vert(q[0], float(q[1]), float(q[2]), n, color))
		tri(ids[0], ids[1], ids[2], n)
		if ids.size() == 4:
			tri(ids[0], ids[2], ids[3], n)

	## Normales lissées des sommets [from, to) d'après les triangles ajoutés depuis `i_from`.
	func smooth(from: int, to: int, i_from: int) -> void:
		for k in range(from, to):
			normals[k] = Vector3.ZERO
		for i in range(i_from, indices.size(), 3):
			var a := indices[i]
			var b := indices[i + 1]
			var c := indices[i + 2]
			var n := (pm[c] - pm[a]).cross(pm[b] - pm[a])
			for k in [a, b, c]:
				if k >= from and k < to:
					normals[k] += n
		for k in range(from, to):
			normals[k] = normals[k].normalized() if normals[k].length_squared() > 1e-12 else Vector3.UP

	func result() -> Dictionary:
		return {
			"vertices": vertices, "normals": normals, "colors": colors, "uv2": uv2, "indices": indices,
			"y_min": y_min, "y_max": y_max,
		}


## Sol d'une ville : grille polaire `ground_m` interpolée, sinon `z_m` constant.
class Ground:
	var radii := PackedFloat32Array()
	var g := PackedFloat32Array()
	var z := 0.0
	## Relief échantillonné (villes v2) : prime sur `ground_m` et `z_m`.
	var heights: TownPlan.Heights = null

	func _init(town: Dictionary) -> void:
		z = float(town.get("z_m", 0.0))
		for r in town.get("radii", []):
			radii.append(maxf(float(r), 1.0))
		var raw: Variant = town.get("ground_m", null)
		if raw != null and typeof(raw) in [TYPE_ARRAY, TYPE_PACKED_FLOAT32_ARRAY, TYPE_PACKED_FLOAT64_ARRAY]:
			if radii.size() >= 3 and raw.size() == 1 + 2 * radii.size():
				for v in raw:
					g.append(float(v))

	func at(p: Vector2) -> float:
		if heights != null:
			return heights.height_m(p.x, p.y)
		if g.is_empty():
			return z
		var d := p.length()
		if d < 1e-3:
			return g[0]
		var n := radii.size()
		var f := fposmod(atan2(p.y, p.x), TAU) / TAU * n
		var i := int(f) % n
		var j := (i + 1) % n
		var t := f - floorf(f)
		var r := lerpf(radii[i], radii[j], t)
		var ga := lerpf(g[1 + i], g[1 + j], t)
		var gb := lerpf(g[1 + n + i], g[1 + n + j], t)
		var s := d / r
		if s <= INNER_RING:
			return lerpf(g[0], ga, s / INNER_RING)
		return lerpf(ga, gb, clampf((s - INNER_RING) / (1.0 - INNER_RING), 0.0, 1.0))


# --- API ------------------------------------------------------------------------------------


## F1 : maillage lointain complet d'une ville de `towns_1340.json` (entrée de `towns`).
## `wall_params` : bloc `walls` de la racine du fichier (repli sur `WALL_DEFAULTS`).
static func build_f1(town: Dictionary, town_index: int, meters_per_unit: float, wall_params: Dictionary = {}) -> Dictionary:
	var acc := Acc.new(_anchor(town), meters_per_unit, town_index)
	var ground := Ground.new(town)
	var radii := _radii(town)
	var rng := RandomNumberGenerator.new()
	rng.seed = int(town.get("seed", 1))
	var kind := str(town.get("kind", "town"))
	var walls := str(town.get("walls", "none"))
	var walled := walls == "stone" or walls == "palisade"
	var roof := _roof_color(kind, rng)
	_nappe(acc, ground, radii, 2, float(EAVE_M.get(kind, EAVE_DEFAULT_M)), WALLED_EDGE if walled else 1.0, roof, rng)
	for f: Dictionary in town.get("faubourgs", []):
		_faubourg(acc, ground, f, roof)
	if walled:
		var params: Dictionary = (WALL_DEFAULTS[walls] as Dictionary).duplicate()
		params.merge(wall_params.get(walls, {}), true)
		_enclosure(acc, ground, radii, town.get("gates", []), params, walls == "palisade")
	var has_cathedral := false
	for m: Dictionary in town.get("monuments", []):
		_monument(acc, ground, m)
		has_cathedral = has_cathedral or str(m.get("kind", "")) == "cathedral"
	if kind == "city" or kind == "town":
		var count := mini(int(town.get("parishes", 0)) - (1 if has_cathedral else 0), PARISH_MAX)
		for _k in maxi(count, 0):
			var a := rng.randf() * TAU
			var p := Vector2(cos(a), sin(a)) * TownPlan.radius_at(radii, a) * rng.randf_range(0.3, 0.75)
			_spire_tower(acc, ground, p, PARISH_HALF_M, PARISH_TOWER_M, PARISH_SPIRE_M, CHURCH_STONE, LEAD)
	return acc.result()


## F2 : polygone à 16 côtés + jupe + une flèche au plus (cathédrale, sinon donjon).
static func build_f2(town: Dictionary, town_index: int, meters_per_unit: float) -> Dictionary:
	var acc := Acc.new(_anchor(town), meters_per_unit, town_index)
	var ground := Ground.new(town)
	var radii := _radii(town)
	var rng := RandomNumberGenerator.new()
	rng.seed = int(town.get("seed", 1))
	var kind := str(town.get("kind", "town"))
	var roof := _roof_color(kind, rng)
	var sub := PackedFloat32Array()
	for k in F2_SIDES:
		sub.append(TownPlan.radius_at(radii, float(k) / F2_SIDES * TAU))
	_nappe(acc, ground, sub, 1, float(EAVE_M.get(kind, EAVE_DEFAULT_M)), 1.0, roof, null)
	var best: Dictionary = {}
	for m: Dictionary in town.get("monuments", []):
		var mk := str(m.get("kind", ""))
		if mk == "cathedral":
			best = m
			break
		if mk == "castle" and best.is_empty():
			best = m
	if not best.is_empty():
		var at := _vec2(best.get("at", [0.0, 0.0]))
		var size := float(best.get("size_m", 60.0))
		if str(best["kind"]) == "cathedral":
			_pyramid(acc, ground, at, size * 0.08, 0.0, _cathedral_tower_m(size) + CATHEDRAL_SPIRE_M, LEAD)
		else:
			var half := size * KEEP_FRACTION * 0.5
			_box(acc, ground, at, Vector2.RIGHT, half, half, KEEP_HEIGHT_M, STONE, false)
	return acc.result()


## Lointain d'une ville v2 (`data/landmarks_v2/*.json`) : quartiers, enceintes, plus grands
## monuments. `anchor` (unités carte) : `LandmarkV2Library.anchor_units(city)` par défaut (le fil
## principal doit avoir chargé la bibliothèque) ; `heights` : relief sous la ville (repère local,
## le même ancrage), sinon sol uniforme `ground_z_m` (sinon `city.z_m`, sinon 0). Le relief
## affiché est exagéré : un sol uniforme laisse des dalles flotter ou s'enfoncer (Paris, 30/09).
static func build_v2_far(city: Dictionary, town_index: int, meters_per_unit: float, anchor: Vector2 = Vector2(NAN, NAN), ground_z_m: float = NAN, year: int = V2_YEAR, heights: TownPlan.Heights = null) -> Dictionary:
	if is_nan(anchor.x):
		anchor = LandmarkV2Library.anchor_units(city)
	var acc := Acc.new(anchor, meters_per_unit, town_index)
	var ground := Ground.new({"z_m": ground_z_m if not is_nan(ground_z_m) else float(city.get("z_m", 0.0))})
	ground.heights = heights
	var rng := RandomNumberGenerator.new()
	rng.seed = int(city.get("seed", 1))
	var roof := _roof_color("city", rng)
	for d: Dictionary in city.get("districts", []):
		if LandmarkV2Library.present(d, year):
			_district(acc, ground, LandmarkV2Library.local_line(d["polygon"]), float(V2_EAVE_M.get(str(d.get("zone", "intra")), V2_EAVE_DEFAULT_M)), roof)
	for w: Dictionary in city.get("walls", []):
		if LandmarkV2Library.present(w, year):
			_v2_wall(acc, ground, w, year)
	var ranked: Array = []
	for m: Dictionary in city.get("monuments", []):
		if LandmarkV2Library.present(m, year):
			ranked.append([_v2_weight(m), m])
	ranked.sort_custom(func(a: Array, b: Array) -> bool: return a[0] > b[0] or (a[0] == b[0] and str(a[1].get("id", "")) < str(b[1].get("id", ""))))
	for k in mini(ranked.size(), V2_MAX_MONUMENTS):
		_v2_monument(acc, ground, ranked[k][1])
	return acc.result()


## Ajoute `part` à `into` (tableaux fusionnés, indices décalés) ; `into` peut être vide.
static func append(into: Dictionary, part: Dictionary) -> void:
	if triangle_count(part) == 0:
		return
	if not into.has("vertices"):
		into["vertices"] = PackedVector3Array()
		into["normals"] = PackedVector3Array()
		into["colors"] = PackedColorArray()
		into["uv2"] = PackedVector2Array()
		into["indices"] = PackedInt32Array()
		into["y_min"] = INF
		into["y_max"] = -INF
	# Les Packed*Array sont des valeurs : on réaffecte après modification.
	var verts: PackedVector3Array = into["vertices"]
	var offset := verts.size()
	verts.append_array(part["vertices"])
	into["vertices"] = verts
	var normals: PackedVector3Array = into["normals"]
	normals.append_array(part["normals"])
	into["normals"] = normals
	var colors: PackedColorArray = into["colors"]
	colors.append_array(part["colors"])
	into["colors"] = colors
	var uv2: PackedVector2Array = into["uv2"]
	uv2.append_array(part["uv2"])
	into["uv2"] = uv2
	var src: PackedInt32Array = part["indices"]
	var dst: PackedInt32Array = into["indices"]
	var base := dst.size()
	dst.resize(base + src.size())
	for i in src.size():
		dst[base + i] = src[i] + offset
	into["indices"] = dst
	into["y_min"] = minf(float(into["y_min"]), float(part["y_min"]))
	into["y_max"] = maxf(float(into["y_max"]), float(part["y_max"]))


## Tableau `Mesh.ARRAY_MAX` pour `ArrayMesh.add_surface_from_arrays` ; [] si vide.
static func mesh_arrays(part: Dictionary) -> Array:
	if triangle_count(part) == 0:
		return []
	var arrays: Array = []
	arrays.resize(Mesh.ARRAY_MAX)
	arrays[Mesh.ARRAY_VERTEX] = part["vertices"]
	arrays[Mesh.ARRAY_NORMAL] = part["normals"]
	arrays[Mesh.ARRAY_COLOR] = part["colors"]
	arrays[Mesh.ARRAY_TEX_UV2] = part["uv2"]
	arrays[Mesh.ARRAY_INDEX] = part["indices"]
	return arrays


static func triangle_count(part: Dictionary) -> int:
	return (part.get("indices", PackedInt32Array()) as PackedInt32Array).size() / 3


# --- Données -------------------------------------------------------------------------------


static func _anchor(town: Dictionary) -> Vector2:
	var px: Array = town.get("px", [0.0, 0.0])
	return Vector2(float(px[0]), float(px[1]))


static func _radii(town: Dictionary) -> PackedFloat32Array:
	var radii := PackedFloat32Array()
	for r in town.get("radii", []):
		radii.append(maxf(float(r), 1.0))
	if radii.size() < 3:
		radii = PackedFloat32Array([60.0, 60.0, 60.0, 60.0])
	return radii


static func _vec2(v: Variant) -> Vector2:
	if v is Vector2:
		return v
	var a: Array = v
	return Vector2(float(a[0]), float(a[1]))


static func _roof_color(kind: String, rng: RandomNumberGenerator) -> Color:
	var base := ROOF_THATCH if kind == "village" else ROOF_TILE
	var k := rng.randf_range(-0.05, 0.05)
	return Color(clampf(base.r + k, 0.0, 1.0), clampf(base.g + k * 0.7, 0.0, 1.0), clampf(base.b + k * 0.5, 0.0, 1.0))


static func _cathedral_tower_m(size: float) -> float:
	return lerpf(CATHEDRAL_TOWER_M.x, CATHEDRAL_TOWER_M.y, clampf((size - 60.0) / 70.0, 0.0, 1.0))


# --- Nappe et jupe -------------------------------------------------------------------------


## Nappe polaire : centre + `rings` anneaux (1 : bord seul ; 2 : 0,5 r et bord) + jupe au bord.
static func _nappe(acc: Acc, ground: Ground, radii: PackedFloat32Array, rings: int, eave: float, edge: float, roof: Color, rng: RandomNumberGenerator) -> void:
	var n := radii.size()
	var v0 := acc.pm.size()
	var i0 := acc.indices.size()
	var center := acc.vert(Vector2.ZERO, ground.at(Vector2.ZERO), eave + CROWN_M, Vector3.UP, roof)
	var ring_ids: Array[PackedInt32Array] = []
	var edge_pts := PackedVector2Array()
	var edge_rel := PackedFloat32Array()
	for ring in rings:
		var last := ring == rings - 1
		var f := edge if last else INNER_RING
		var crown := 0.0 if last else CROWN_M * 0.5
		var ids := PackedInt32Array()
		for k in n:
			var a := float(k) / n * TAU
			var p := Vector2(cos(a), sin(a)) * radii[k] * f
			var jitter := rng.randf_range(-EAVE_JITTER_M, EAVE_JITTER_M) if rng != null else 0.0
			var rel := eave + crown + jitter
			ids.append(acc.vert(p, ground.at(p), rel, Vector3.UP, roof))
			if last:
				edge_pts.append(p)
				edge_rel.append(rel)
		ring_ids.append(ids)
	for k in n:
		var k1 := (k + 1) % n
		var first := ring_ids[0]
		acc.tri(center, first[k], first[k1], Vector3.UP)
		for ring in range(1, rings):
			var a := ring_ids[ring - 1]
			var b := ring_ids[ring]
			acc.tri(a[k], b[k], b[k1], Vector3.UP)
			acc.tri(a[k], b[k1], a[k1], Vector3.UP)
	acc.smooth(v0, acc.pm.size(), i0)
	_skirt(acc, ground, edge_pts, edge_rel, true, FACADE)


## Jupe d'un contour fermé : du bord (hauteur `top_rel`) jusqu'à sol − SKIRT_M, normales vers
## l'extérieur (radiales depuis l'origine locale si `radial`, sinon d'après le sens du contour).
static func _skirt(acc: Acc, ground: Ground, pts: PackedVector2Array, top_rel: PackedFloat32Array, radial: bool, color: Color) -> void:
	var n := pts.size()
	var ccw := _signed_area(pts) > 0.0
	for k in n:
		var k1 := (k + 1) % n
		var a := pts[k]
		var b := pts[k1]
		var out: Vector2
		if radial:
			out = (a + b).normalized()
		else:
			var e := b - a
			out = Vector2(e.y, -e.x) if ccw else Vector2(-e.y, e.x)
		var ga := ground.at(a)
		var gb := ground.at(b)
		acc.face([[a, ga, -SKIRT_M], [b, gb, -SKIRT_M], [b, gb, top_rel[k1]], [a, ga, top_rel[k]]], Vector3(out.x, 0.0, out.y), color)


## Aire signée (repère local x, y) : > 0 si le contour tourne de +x vers +y.
static func _signed_area(pts: PackedVector2Array) -> float:
	var s := 0.0
	for k in pts.size():
		s += pts[k].cross(pts[(k + 1) % pts.size()])
	return s * 0.5


# --- Faubourgs, enceinte -------------------------------------------------------------------


static func _faubourg(acc: Acc, ground: Ground, f: Dictionary, roof: Color) -> void:
	var a := deg_to_rad(float(f.get("bearing", 0.0)))
	var u := Vector2(cos(a), sin(a))
	var start := float(f.get("start_m", 0.0))
	var length := float(f.get("length_m", 0.0))
	if length < 5.0:
		return
	var parts := 2 if length > FAUBOURG_SPLIT_M else 1
	for i in parts:
		var p0 := u * (start + length * i / parts)
		var p1 := u * (start + length * (i + 1) / parts)
		_gable_block(acc, ground, p0, p1, FAUBOURG_WIDTH_M * 0.5, FAUBOURG_EAVE_M, FAUBOURG_RIDGE_M, roof, FACADE)


## Enceinte au rayon : face extérieure (jusqu'au pied de jupe) et chemin de ronde ; la face
## intérieure est omise (cachée par la nappe de loin). Tours tous les `TOWER_EVERY` relèvements,
## portes aux relèvements de `gates`.
static func _enclosure(acc: Acc, ground: Ground, radii: PackedFloat32Array, gates: Array, params: Dictionary, palisade: bool) -> void:
	var n := radii.size()
	var height := float(params.get("height_m", 9.0))
	var half := float(params.get("thickness_m", 2.2)) * 0.5
	var color := TIMBER if palisade else STONE
	for k in n:
		var k1 := (k + 1) % n
		var a0 := float(k) / n * TAU
		var a1 := float(k + 1) / n * TAU
		var d0 := Vector2(cos(a0), sin(a0))
		var d1 := Vector2(cos(a1), sin(a1))
		var o0 := d0 * (radii[k] + half)
		var o1 := d1 * (radii[k1] + half)
		var i0 := d0 * (radii[k] - half)
		var i1 := d1 * (radii[k1] - half)
		var go0 := ground.at(o0)
		var go1 := ground.at(o1)
		var gi0 := ground.at(i0)
		var gi1 := ground.at(i1)
		var mid := (d0 + d1).normalized()
		acc.face([[o0, go0, -SKIRT_M], [o1, go1, -SKIRT_M], [o1, go1, height], [o0, go0, height]], Vector3(mid.x, 0.0, mid.y), color)
		acc.face([[o0, go0, height], [o1, go1, height], [i1, gi1, go1 + height - gi1], [i0, gi0, go0 + height - gi0]], Vector3.UP, color)
	var tower_h := float(params.get("tower_height_m", 0.0))
	var tower_r := float(params.get("tower_radius_m", 0.0))
	if not palisade and tower_h > height and tower_r > 0.0:
		for k in range(0, n, TOWER_EVERY):
			var a := float(k) / n * TAU
			var d := Vector2(cos(a), sin(a))
			_box(acc, ground, d * (radii[k] + half), d, tower_r, tower_r, tower_h, STONE, true)
	for g: Dictionary in gates:
		var ag := deg_to_rad(float(g.get("bearing", 0.0)))
		var dg := Vector2(cos(ag), sin(ag))
		_box(acc, ground, dg * TownPlan.radius_at(radii, ag), dg, GATE_SIZE_M * 0.5, GATE_SIZE_M * 0.5, height + (0.0 if palisade else GATE_EXTRA_M), color, true)


# --- Monuments F1 --------------------------------------------------------------------------


static func _monument(acc: Acc, ground: Ground, m: Dictionary) -> void:
	var at := _vec2(m.get("at", [0.0, 0.0]))
	var yaw := float(m.get("yaw", 0.0))
	var u := Vector2(cos(yaw), sin(yaw))
	var size := float(m.get("size_m", 20.0))
	match str(m.get("kind", "")):
		"cathedral":
			var half_len := size * 0.5
			_gable_block(acc, ground, at - u * half_len, at + u * half_len, size * CATHEDRAL_WIDTH * 0.5, CATHEDRAL_EAVE_M, CATHEDRAL_RIDGE_M, LEAD, CHURCH_STONE)
			var half_t := size * 0.07
			_spire_tower(acc, ground, at - u * (half_len - half_t), half_t, _cathedral_tower_m(size), CATHEDRAL_SPIRE_M, CHURCH_STONE, LEAD)
		"abbey":
			var church_len := size * ABBEY_CHURCH
			var width := church_len * 0.25
			_gable_block(acc, ground, at - u * church_len * 0.5, at + u * church_len * 0.5, width * 0.5, ABBEY_EAVE_M, ABBEY_RIDGE_M, LEAD, CHURCH_STONE)
			_spire_tower(acc, ground, at, width * 0.35, ABBEY_TOWER_M, ABBEY_SPIRE_M, CHURCH_STONE, LEAD)
		"castle":
			var half := size * KEEP_FRACTION * 0.5
			_box(acc, ground, at, u, half, half, KEEP_HEIGHT_M, STONE, true)
		"hall":
			_gable_block(acc, ground, at - u * size * 0.5, at + u * size * 0.5, size * 0.25, HALL_EAVE_M, HALL_RIDGE_M, ROOF_TILE, STONE)
		"windmill":
			_spire_tower(acc, ground, at, size * 0.5, WINDMILL_HEIGHT_M, WINDMILL_CAP_M, TIMBER, TIMBER)
		_:
			_box(acc, ground, at, u, size * 0.3, size * 0.3, HALL_EAVE_M, STONE, true)


# --- Primitives ----------------------------------------------------------------------------


## Pavé d'axe `u` (demi-longueur, demi-largeur), du pied de jupe à `top_rel` au-dessus du sol
## sous son centre (sommet horizontal) ; couvercle plat si `cap`.
static func _box(acc: Acc, ground: Ground, center: Vector2, u: Vector2, half_len: float, half_w: float, top_rel: float, color: Color, cap: bool) -> void:
	var v := Vector2(-u.y, u.x)
	var top := ground.at(center) + top_rel
	var c: Array[Vector2] = [center - u * half_len - v * half_w, center + u * half_len - v * half_w, center + u * half_len + v * half_w, center - u * half_len + v * half_w]
	var g: Array[float] = []
	for p in c:
		g.append(ground.at(p))
	for i in 4:
		var j := (i + 1) % 4
		var out := ((c[i] + c[j]) * 0.5 - center).normalized()
		acc.face([[c[i], g[i], -SKIRT_M], [c[j], g[j], -SKIRT_M], [c[j], g[j], top - g[j]], [c[i], g[i], top - g[i]]], Vector3(out.x, 0.0, out.y), color)
	if cap:
		acc.face([[c[0], g[0], top - g[0]], [c[1], g[1], top - g[1]], [c[2], g[2], top - g[2]], [c[3], g[3], top - g[3]]], Vector3.UP, color)


## Pyramide à base carrée (côté 2 × `half`) de `base_rel` à `apex_rel` au-dessus du sol central.
static func _pyramid(acc: Acc, ground: Ground, center: Vector2, half: float, base_rel: float, apex_rel: float, color: Color) -> void:
	var gc := ground.at(center)
	var c: Array[Vector2] = [center + Vector2(-half, -half), center + Vector2(half, -half), center + Vector2(half, half), center + Vector2(-half, half)]
	for i in 4:
		var a := c[i]
		var b := c[(i + 1) % 4]
		var out := ((a + b) * 0.5 - center).normalized()
		var ga := ground.at(a)
		var gb := ground.at(b)
		acc.face([[a, ga, gc + base_rel - ga], [b, gb, gc + base_rel - gb], [center, gc, apex_rel]], Vector3(out.x, 0.3, out.y), color)


## Tour carrée (sans couvercle) surmontée d'une flèche.
static func _spire_tower(acc: Acc, ground: Ground, center: Vector2, half: float, tower_rel: float, spire_m: float, wall: Color, spire: Color) -> void:
	_box(acc, ground, center, Vector2.RIGHT, half, half, tower_rel, wall, false)
	_pyramid(acc, ground, center, half, tower_rel, tower_rel + spire_m, spire)


## Bloc à deux pans de `p0` à `p1` (faîtage selon l'axe), demi-largeur `half_w` ; murs du pied
## de jupe à l'égout ; hauteurs relatives au sol de chaque extrémité. 14 triangles.
static func _gable_block(acc: Acc, ground: Ground, p0: Vector2, p1: Vector2, half_w: float, eave: float, ridge: float, roof: Color, wall: Color) -> void:
	var axis := p1 - p0
	if axis.length_squared() < 1e-4:
		return
	var u := axis.normalized()
	var v := Vector2(-u.y, u.x)
	var g0 := ground.at(p0)
	var g1 := ground.at(p1)
	var a0 := p0 - v * half_w
	var b0 := p0 + v * half_w
	var a1 := p1 - v * half_w
	var b1 := p1 + v * half_w
	var hu := Vector3(u.x, 0.0, u.y)
	var hv := Vector3(v.x, 0.0, v.y)
	acc.face([[a0, g0, -SKIRT_M], [a1, g1, -SKIRT_M], [a1, g1, eave], [a0, g0, eave]], -hv, wall)
	acc.face([[b0, g0, -SKIRT_M], [b1, g1, -SKIRT_M], [b1, g1, eave], [b0, g0, eave]], hv, wall)
	acc.face([[a0, g0, -SKIRT_M], [b0, g0, -SKIRT_M], [b0, g0, eave], [a0, g0, eave]], -hu, wall)
	acc.face([[a0, g0, eave], [b0, g0, eave], [p0, g0, ridge]], -hu, wall)
	acc.face([[a1, g1, -SKIRT_M], [b1, g1, -SKIRT_M], [b1, g1, eave], [a1, g1, eave]], hu, wall)
	acc.face([[a1, g1, eave], [b1, g1, eave], [p1, g1, ridge]], hu, wall)
	acc.face([[a0, g0, eave], [a1, g1, eave], [p1, g1, ridge], [p0, g0, ridge]], Vector3(-v.x, 1.0, -v.y), roof)
	acc.face([[b0, g0, eave], [b1, g1, eave], [p1, g1, ridge], [p0, g0, ridge]], Vector3(v.x, 1.0, v.y), roof)


# --- Villes v2 -----------------------------------------------------------------------------


## Quartier : dessus triangulé à l'égout, jupe sur le contour.
static func _district(acc: Acc, ground: Ground, poly: PackedVector2Array, eave: float, roof: Color) -> void:
	if poly.size() > 3 and poly[0].is_equal_approx(poly[poly.size() - 1]):
		poly.remove_at(poly.size() - 1)
	if poly.size() < 3:
		return
	var v0 := acc.pm.size()
	var i0 := acc.indices.size()
	# Nappe découpée en cellules de DISTRICT_CELL_M : ses sommets suivent le relief (un quartier de
	# 2 km sur un seul triangle traverserait les collines). Sommets partagés entre cellules.
	var ids := {}
	var lo := Vector2(INF, INF)
	var hi := Vector2(-INF, -INF)
	for p in poly:
		lo = lo.min(p)
		hi = hi.max(p)
	var cells := Vector2i(ceili((hi.x - lo.x) / DISTRICT_CELL_M), ceili((hi.y - lo.y) / DISTRICT_CELL_M)).maxi(1)
	for cy in cells.y:
		for cx in cells.x:
			var c0 := lo + Vector2(cx, cy) * DISTRICT_CELL_M
			var cell := PackedVector2Array([c0, c0 + Vector2(DISTRICT_CELL_M, 0.0), c0 + Vector2.ONE * DISTRICT_CELL_M, c0 + Vector2(0.0, DISTRICT_CELL_M)])
			for piece: PackedVector2Array in Geometry2D.intersect_polygons(poly, cell):
				var tris := Geometry2D.triangulate_polygon(piece)
				if tris.size() < 3:
					continue
				var pid := PackedInt32Array()
				for p in piece:
					var key := Vector2i(roundi(p.x * 10.0), roundi(p.y * 10.0))
					if not ids.has(key):
						ids[key] = acc.vert(p, ground.at(p), eave, Vector3.UP, roof)
					pid.append(ids[key])
				for t in range(0, tris.size(), 3):
					acc.tri(pid[tris[t]], pid[tris[t + 1]], pid[tris[t + 2]], Vector3.UP)
	if acc.indices.size() == i0:
		# Contour qui se recoupe : éventail depuis le barycentre.
		var c := Vector2.ZERO
		for p in poly:
			c += p
		c /= poly.size()
		var ci := acc.vert(c, ground.at(c), eave, Vector3.UP, roof)
		var ring := PackedInt32Array()
		for p in poly:
			ring.append(acc.vert(p, ground.at(p), eave, Vector3.UP, roof))
		for k in poly.size():
			acc.tri(ci, ring[k], ring[(k + 1) % poly.size()], Vector3.UP)
	acc.smooth(v0, acc.pm.size(), i0)
	var edge := _densify(poly, DISTRICT_CELL_M * 0.5)
	var rel := PackedFloat32Array()
	rel.resize(edge.size())
	rel.fill(eave)
	_skirt(acc, ground, edge, rel, false, FACADE)


## Contour fermé dont aucun côté ne dépasse `step` (points intermédiaires réguliers).
static func _densify(pts: PackedVector2Array, step: float) -> PackedVector2Array:
	var out := PackedVector2Array()
	for k in pts.size():
		var a := pts[k]
		var b := pts[(k + 1) % pts.size()]
		var n := maxi(1, ceili(a.distance_to(b) / step))
		for j in n:
			out.append(a.lerp(b, float(j) / n))
	return out


static func _v2_wall(acc: Acc, ground: Ground, w: Dictionary, year: int) -> void:
	var pts := LandmarkV2Library.local_line(w.get("points", []))
	if pts.size() < 2:
		return
	if bool(w.get("closed", false)):
		pts.append(pts[0])
	var height := float(w.get("height_m", 9.0))
	var half := float(w.get("thickness_m", 2.2)) * 0.5
	for i in range(1, pts.size()):
		var a := pts[i - 1]
		var b := pts[i]
		if a.distance_squared_to(b) < 0.01:
			continue
		var u := (b - a).normalized()
		var v := Vector2(-u.y, u.x) * half
		var ga := ground.at(a)
		var gb := ground.at(b)
		var hv := Vector3(v.x, 0.0, v.y)
		acc.face([[a + v, ga, -SKIRT_M], [b + v, gb, -SKIRT_M], [b + v, gb, height], [a + v, ga, height]], hv, STONE)
		acc.face([[a - v, ga, -SKIRT_M], [b - v, gb, -SKIRT_M], [b - v, gb, height], [a - v, ga, height]], -hv, STONE)
		acc.face([[a + v, ga, height], [b + v, gb, height], [b - v, gb, height], [a - v, ga, height]], Vector3.UP, STONE)
	for g: Dictionary in w.get("gates", []):
		if LandmarkV2Library.present(g, year):
			_box(acc, ground, LandmarkV2Library.local(g["at"]), Vector2.RIGHT, GATE_SIZE_M * 0.5, GATE_SIZE_M * 0.5, float(g.get("height_m", height + GATE_EXTRA_M)), STONE, true)


## Poids d'un monument v2 (emprise × hauteur) : on garde les plus visibles de loin.
static func _v2_weight(m: Dictionary) -> float:
	var fp := LandmarkMonuments.footprint(m)
	return fp.x * fp.y * maxf(_v2_top(m), 1.0)


static func _v2_top(m: Dictionary) -> float:
	var p: Dictionary = m.get("params", {})
	var top := float(p.get("height_m", p.get("ridge_m", p.get("height", 12.0))))
	for key in ["tower", "crossing"]:
		var t: Dictionary = p.get(key, {})
		if not t.is_empty():
			top = maxf(top, float(t.get("height", 0.0)) + float(t.get("spire_m", 0.0)))
	for t: Dictionary in p.get("west_towers", []):
		top = maxf(top, float(t.get("height", 0.0)) + float(t.get("spire_m", 0.0)))
	var keep: Dictionary = p.get("keep", {})
	if not keep.is_empty():
		top = maxf(top, float(keep.get("height_m", 0.0)))
	return top


static func _v2_monument(acc: Acc, ground: Ground, m: Dictionary) -> void:
	var at := LandmarkV2Library.local(m["at"])
	var yaw := LandmarkV2Library.yaw_of(float(m.get("angle_deg", 0.0)))
	var u := Vector2(cos(yaw), sin(yaw))
	var v := Vector2(-u.y, u.x)
	var p: Dictionary = m.get("params", {})
	var fp := LandmarkMonuments.footprint(m)
	match str(m.get("model", "")):
		"gothic_cathedral", "church", "abbey":
			var length := float(p.get("length_m", fp.x))
			var width := float(p.get("width_m", fp.y))
			var eave := float(p.get("vault_m", p.get("height_m", 16.0)))
			var ridge := float(p.get("ridge_m", eave + width * 0.4))
			_gable_block(acc, ground, at - u * length * 0.5, at + u * length * 0.5, width * 0.5, eave, ridge, LEAD, CHURCH_STONE)
			for t: Dictionary in p.get("west_towers", []):
				var side := -1.0 if str(t.get("side", "north")) == "north" else 1.0
				var s := float(t.get("size", 10.0))
				var c := at - u * (length * 0.5 - s * 0.5) + v * side * (width * 0.5 - s * 0.5)
				_v2_tower(acc, ground, c, u, s * 0.5, float(t.get("height", 40.0)), float(t.get("spire_m", 0.0)))
			var crossing: Dictionary = p.get("crossing", {})
			if not crossing.is_empty():
				var cc := at + u * (float(p.get("transept_at", 0.5)) - 0.5) * length
				_v2_tower(acc, ground, cc, u, float(crossing.get("size", 8.0)) * 0.5, float(crossing.get("height", 30.0)), float(crossing.get("spire_m", 0.0)))
			var tower: Dictionary = p.get("tower", {})
			if not tower.is_empty():
				var ts := float(tower.get("size", 7.0))
				var tc := at if str(tower.get("at", "west")) == "crossing" else at - u * (length * 0.5 + ts * 0.5)
				_v2_tower(acc, ground, tc, u, ts * 0.5, float(tower.get("height", 30.0)), float(tower.get("spire_m", 0.0)))
		"castle":
			_box(acc, ground, at, u, fp.x * 0.5, fp.y * 0.5, float(p.get("height_m", 10.0)), STONE, true)
			var keep: Dictionary = p.get("keep", {})
			if not keep.is_empty():
				var ka := _vec2(keep.get("at", [0.0, 0.0]))
				var kr := float(keep.get("radius_m", 8.0))
				_box(acc, ground, at + u * ka.x - v * ka.y, u, kr, kr, float(keep.get("height_m", 30.0)), STONE, true)
		"belfry", "keep", "tower", "gate_tower":
			var top := float(p.get("height", p.get("height_m", 25.0)))
			var roofed := str(p.get("top", "")) == "pyramid"
			_v2_tower(acc, ground, at, u, fp.x * 0.5, top, fp.x * 0.6 if roofed else 0.0)
		"earthwork":
			pass
		_:
			var h := float(p.get("height_m", 12.0))
			_gable_block(acc, ground, at - u * fp.x * 0.5, at + u * fp.x * 0.5, fp.y * 0.5, h, h + fp.y * 0.3, ROOF_TILE, STONE)


static func _v2_tower(acc: Acc, ground: Ground, center: Vector2, u: Vector2, half: float, height: float, spire_m: float) -> void:
	_box(acc, ground, center, u, half, half, height, CHURCH_STONE, spire_m <= 0.0)
	if spire_m > 0.0:
		_pyramid(acc, ground, center, half, height, height + spire_m, LEAD)
