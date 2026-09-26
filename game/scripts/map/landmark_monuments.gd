class_name LandmarkMonuments
extends RefCounted

## Lot VH4 (ADR 0078) : gabarits paramétrés des monuments des villes emblématiques 1:1, à
## l'échelle réelle (mètres). Calcul pur (fil de travail) : `build(monument)` rend les tableaux
## d'un maillage (`SurfaceTool.commit_to_arrays`) dans le repère du monument : +X le long de
## l'axe (églises : vers le chœur), +Z à droite de l'axe, +Y en haut, origine au centre de
## l'emprise, y = 0 à la base (les murs descendent à -`FOUNDATION_M` pour les pentes).
## Couleurs de sommet = couche de l'atlas BR1 (`TownBuilder.layer_color`) : matériau
## `TownBuilder.material(0, true)` (UV en boîte, base par instance). Paramètres dans
## docs/landmarks-v2.md. Rendu seulement.

const FOUNDATION_M := 8.0
const WALL := "Ashlar"
const MASONRY := "Masonry"
const ROOF := "RoofSlate"
const ROOF_TILE := "RoofTile"
const STONE_TINT := Color(0.97, 0.94, 0.86)
const RUBBLE_TINT := Color(0.9, 0.86, 0.78)
const RUBBLE := "Rubble"
const TIMBER := "Timber"
const EARTH_TINT := Color(0.62, 0.52, 0.38)
const WOOD_TINT := Color(0.72, 0.6, 0.46)


## Emprise (longueur le long de l'axe, largeur) en mètres, pour réserver le sol.
static func footprint(m: Dictionary) -> Vector2:
	var p: Dictionary = m.get("params", {})
	match str(m["model"]):
		"gothic_cathedral":
			return Vector2(float(p.get("length_m", 100.0)), maxf(float(p.get("width_m", 30.0)), float(p.get("transept_m", 0.0))))
		"church":
			var tower: Dictionary = p.get("tower", {})
			var extra := float(tower.get("size", 0.0)) if str(tower.get("at", "")) == "west" else 0.0
			return Vector2(float(p.get("length_m", 30.0)) + extra, float(p.get("width_m", 14.0)))
		"abbey":
			var cl: Dictionary = p.get("cloister", {})
			return Vector2(float(p.get("length_m", 40.0)), float(p.get("width_m", 16.0)) + float(cl.get("size", 30.0)) + 4.0)
		"castle":
			var lo := Vector2(INF, INF)
			var hi := Vector2(-INF, -INF)
			for q in p.get("ring", [[-40, -40], [40, -40], [40, 40], [-40, 40]]):
				lo = lo.min(Vector2(float(q[0]), float(q[1])))
				hi = hi.max(Vector2(float(q[0]), float(q[1])))
			var r := float(p.get("tower_radius_m", 5.0))
			return (hi - lo) + Vector2(r, r) * 2.0
		"earthwork":
			var lo_e := Vector2(INF, INF)
			var hi_e := Vector2(-INF, -INF)
			for q in p.get("ring", [[-20, -20], [20, -20], [20, 20], [-20, 20]]):
				lo_e = lo_e.min(Vector2(float(q[0]), float(q[1])))
				hi_e = hi_e.max(Vector2(float(q[0]), float(q[1])))
			var b := float(p.get("base_m", 9.0))
			return (hi_e - lo_e) + Vector2(b, b)
		"belfry", "keep", "tower", "gate_tower":
			var s := float(p.get("size", float(p.get("radius_m", 5.0)) * 2.0))
			return Vector2(s, s)
		_:
			return Vector2(float(p.get("length_m", 30.0)), float(p.get("width_m", 15.0)))


## Tableaux du maillage d'un monument et hauteur du sommet (m) : {arrays, top}.
static func build(m: Dictionary) -> Dictionary:
	var st := SurfaceTool.new()
	st.begin(Mesh.PRIMITIVE_TRIANGLES)
	var p: Dictionary = m.get("params", {})
	var top := 0.0
	match str(m["model"]):
		"gothic_cathedral":
			top = _cathedral(st, p)
		"church":
			top = _church(st, p, Transform3D.IDENTITY)
		"abbey":
			top = _abbey(st, p)
		"castle":
			top = _castle(st, p)
		"belfry":
			top = _belfry(st, p)
		"hall":
			top = _hall(st, p)
		"royal_palace":
			top = _palace(st, p)
		"enclosure":
			top = _enclosure(st, p)
		"earthwork":
			top = _earthwork(st, p)
		"keep", "tower", "gate_tower":
			var r := float(p.get("radius_m", float(p.get("size", 10.0)) * 0.5))
			var h := float(p.get("height", float(p.get("height_m", 20.0))))
			_cylinder(st, Transform3D.IDENTITY, r, -FOUNDATION_M, h, 14, MASONRY, STONE_TINT)
			_cone(st, Transform3D.IDENTITY, r * 1.1, h, h + r * 1.4, 14, ROOF)
			top = h + r * 1.4
	return {"arrays": st.commit_to_arrays(), "top": top}


# --- Gabarits -----------------------------------------------------------------------------


static func _cathedral(st: SurfaceTool, p: Dictionary) -> float:
	var length := float(p.get("length_m", 100.0))
	var width := float(p.get("width_m", 30.0))
	var nave_w := float(p.get("nave_width_m", 11.0))
	var vault := float(p.get("vault_m", 25.0))
	var ridge := float(p.get("ridge_m", vault + 10.0))
	var aisle := float(p.get("aisle_m", 12.0))
	var round_apse := str(p.get("apse", "round")) == "round"
	var x0 := -length * 0.5
	var r_apse := width * 0.5 if round_apse else 0.0
	var x1 := length * 0.5 - r_apse
	var id := Transform3D.IDENTITY
	var aisle_top := aisle + (vault - aisle) * 0.45
	# Bas-côtés (toits en appentis) et nef (toit à deux pans).
	for side: float in [-1.0, 1.0]:
		var zc := side * (nave_w + (width - nave_w) * 0.5) * 0.5
		var zc2 := side * (width * 0.5 + nave_w * 0.5) * 0.5
		_box(st, id, Vector3((x0 + x1) * 0.5, 0, zc2), Vector3(x1 - x0, aisle, (width - nave_w) * 0.5), WALL, STONE_TINT)
		var z_out := side * width * 0.5
		var z_in := side * nave_w * 0.5
		_quad_oriented(st, id, Vector3(x0, aisle, z_out + side * 0.6), Vector3(x1, aisle, z_out + side * 0.6), Vector3(x1, aisle_top, z_in), Vector3(x0, aisle_top, z_in), ROOF, Color(1, 1, 1), Vector3((x0 + x1) * 0.5, 0, zc))
		# Contreforts et culées des arcs-boutants (un par travée).
		var bays := int(p.get("bays", maxi(4, int((x1 - x0) / 9.0))))
		for b in bays + 1:
			var bx := lerpf(x0 + 2.0, x1 - 2.0, float(b) / bays)
			_box(st, id, Vector3(bx, 0, side * (width * 0.5 + 1.2)), Vector3(1.8, aisle + 4.0, 2.4), WALL, STONE_TINT)
			_quad_oriented(st, id, Vector3(bx - 0.5, aisle + 3.0, side * (width * 0.5 + 0.5)), Vector3(bx + 0.5, aisle + 3.0, side * (width * 0.5 + 0.5)), Vector3(bx + 0.5, vault - 3.0, side * nave_w * 0.5), Vector3(bx - 0.5, vault - 3.0, side * nave_w * 0.5), WALL, STONE_TINT, Vector3(bx, 0, 0))
	_box(st, id, Vector3((x0 + x1) * 0.5, 0, 0), Vector3(x1 - x0, vault, nave_w), WALL, STONE_TINT)
	_gable_roof_x(st, id, x0, x1, nave_w * 0.5 + 0.6, vault, ridge, ROOF, not bool(p.get("open_west", false)), not round_apse)
	# Chevet : abside (nef) et déambulatoire à chapelles (bas-côtés).
	if round_apse:
		var c := Transform3D(Basis(), Vector3(x1, 0, 0))
		_half_cylinder(st, c, width * 0.5, -FOUNDATION_M, aisle, 10, WALL, STONE_TINT)
		_half_cone(st, c, width * 0.5 + 0.6, aisle, nave_w * 0.5, aisle_top, 10, ROOF)
		_half_cylinder(st, c, nave_w * 0.5, aisle_top - 0.5, vault, 8, WALL, STONE_TINT)
		_half_cone(st, c, nave_w * 0.5 + 0.6, vault, 0.3, ridge, 8, ROOF)
		if bool(p.get("chapels", false)):
			for k in 5:
				var a := -PI * 0.5 + PI * (k + 0.5) / 5.0
				var cc := Vector3(x1 + cos(a) * (width * 0.5 + 2.0), 0, sin(a) * (width * 0.5 + 2.0))
				_cylinder(st, Transform3D(Basis(), cc), 3.2, -FOUNDATION_M, aisle - 1.0, 8, WALL, STONE_TINT)
				_cone(st, Transform3D(Basis(), cc), 3.6, aisle - 1.0, aisle + 2.5, 8, ROOF)
	# Transept.
	var t_len := float(p.get("transept_m", 0.0))
	if t_len > width:
		var tw := float(p.get("transept_width_m", nave_w + 2.0))
		var tx := x0 + length * float(p.get("transept_at", 0.55))
		var basis := Basis(Vector3.UP, PI * 0.5)
		var t := Transform3D(basis, Vector3(tx, 0, 0))
		_box(st, t, Vector3.ZERO, Vector3(t_len, vault, tw), WALL, STONE_TINT)
		_gable_roof_x(st, t, -t_len * 0.5, t_len * 0.5, tw * 0.5 + 0.6, vault, ridge, ROOF, true, true)
	var top := ridge
	# Tours de façade.
	for tower in p.get("west_towers", []):
		var size := float(tower.get("size", 12.0))
		var h := float(tower.get("height", vault + 20.0))
		var side_z := (-1.0 if str(tower.get("side", "north")) == "north" else 1.0) * (width * 0.5 - size * 0.5)
		var tc := Vector3(x0 + size * 0.5 - 1.0, 0, side_z)
		_box(st, id, tc, Vector3(size, h, size), WALL, STONE_TINT)
		var spire := float(tower.get("spire_m", 0.0))
		match str(tower.get("top", "flat")):
			"pyramid", "spire":
				var sh := maxf(spire, size * 0.7)
				_pyramid(st, Transform3D(Basis(), tc), size * 0.5 + 0.4, h, h + sh, ROOF)
				top = maxf(top, h + sh)
			_:
				top = maxf(top, h)
	# Tour-lanterne de croisée et flèche.
	var crossing: Dictionary = p.get("crossing", {})
	if not crossing.is_empty():
		var cx := x0 + length * float(p.get("transept_at", 0.55))
		var cs := float(crossing.get("size", nave_w + 2.0))
		var ch := float(crossing.get("height", ridge + 10.0))
		var cc := Transform3D(Basis(), Vector3(cx, 0, 0))
		_box(st, cc, Vector3(0, ridge - 4.0, 0), Vector3(cs, ch - ridge + 4.0, cs), WALL, STONE_TINT, ridge - 4.0)
		var spire := float(crossing.get("spire_m", 0.0))
		if spire > 0.0:
			_cone(st, cc, cs * 0.5, ch, ch + spire, 8, ROOF)
		top = maxf(top, ch + spire)
	return top


static func _church(st: SurfaceTool, p: Dictionary, t: Transform3D) -> float:
	var length := float(p.get("length_m", 30.0))
	var width := float(p.get("width_m", 14.0))
	var h := float(p.get("height_m", 12.0))
	var ridge := h + width * 0.45
	var choir_len := length * 0.3
	var x0 := -length * 0.5
	var xc := length * 0.5 - choir_len
	_box(st, t, Vector3((x0 + xc) * 0.5, 0, 0), Vector3(xc - x0, h, width), WALL, STONE_TINT)
	_gable_roof_x(st, t, x0, xc, width * 0.5 + 0.5, h, ridge, ROOF, true, false)
	var cw := width * 0.72
	var ch := h * 0.9
	var x1 := length * 0.5
	if str(p.get("apse", "flat")) == "round":
		x1 -= cw * 0.5
	_box(st, t, Vector3((xc + x1) * 0.5, 0, 0), Vector3(x1 - xc, ch, cw), WALL, STONE_TINT)
	_gable_roof_x(st, t, xc - 0.3, x1, cw * 0.5 + 0.5, ch, ch + cw * 0.45, ROOF, false, str(p.get("apse", "flat")) != "round")
	if str(p.get("apse", "flat")) == "round":
		var a := t * Transform3D(Basis(), Vector3(x1, 0, 0))
		_half_cylinder(st, a, cw * 0.5, -FOUNDATION_M, ch, 8, WALL, STONE_TINT)
		_half_cone(st, a, cw * 0.5 + 0.5, ch, 0.2, ch + cw * 0.45, 8, ROOF)
	var top := ridge
	var tower: Dictionary = p.get("tower", {})
	if not tower.is_empty():
		var size := float(tower.get("size", 6.0))
		var th := float(tower.get("height", h + 12.0))
		var tx := x0 - size * 0.5 + 1.0 if str(tower.get("at", "west")) == "west" else xc
		var tc := t * Transform3D(Basis(), Vector3(tx, 0, 0))
		_box(st, tc, Vector3.ZERO, Vector3(size, th, size), WALL, STONE_TINT)
		var spire := maxf(float(tower.get("spire_m", size * 0.9)), size * 0.6)
		_pyramid(st, tc, size * 0.5 + 0.3, th, th + spire, ROOF)
		top = maxf(top, th + spire)
	return top


static func _abbey(st: SurfaceTool, p: Dictionary) -> float:
	var width := float(p.get("width_m", 16.0))
	var cl: Dictionary = p.get("cloister", {})
	var size := float(cl.get("size", 30.0))
	var side := -1.0 if str(cl.get("side", "south")) == "north" else 1.0
	var total := width + size + 4.0
	# Abbatiale d'un côté, cloître de l'autre (repère centré sur l'emprise).
	var church_z := -side * (total * 0.5 - width * 0.5)
	var top := _church(st, p, Transform3D(Basis(), Vector3(0, 0, church_z)))
	var cz := side * (total * 0.5 - size * 0.5)
	var range_w := 8.0
	for k in 4:
		var yaw := k * PI * 0.5
		var off := Vector3(cos(yaw), 0, sin(yaw)) * (size * 0.5 - range_w * 0.5)
		var t := Transform3D(Basis(Vector3.UP, -yaw + PI * 0.5), Vector3(0, 0, cz) + off)
		_box(st, t, Vector3.ZERO, Vector3(size, 8.0, range_w), MASONRY, RUBBLE_TINT)
		_gable_roof_x(st, t, -size * 0.5, size * 0.5, range_w * 0.5 + 0.4, 8.0, 12.0, ROOF_TILE, false, false)
	return top


static func _castle(st: SurfaceTool, p: Dictionary) -> float:
	var ring: Array = p.get("ring", [])
	var h := float(p.get("height_m", 10.0))
	var thick := float(p.get("thickness_m", 3.0))
	var tr := float(p.get("tower_radius_m", 5.0))
	var th := float(p.get("tower_height_m", h + 6.0))
	var pts: Array[Vector3] = []
	for q in ring:
		pts.append(Vector3(float(q[0]), 0, -float(q[1])))
	for i in pts.size():
		var a := pts[i]
		var b := pts[(i + 1) % pts.size()]
		var d := b - a
		var t := Transform3D(Basis(Vector3.UP, -atan2(d.z, d.x)), (a + b) * 0.5)
		_box(st, t, Vector3.ZERO, Vector3(d.length(), h, thick), MASONRY, STONE_TINT)
		_cylinder(st, Transform3D(Basis(), a), tr, -FOUNDATION_M, th, 12, MASONRY, STONE_TINT)
		_cone(st, Transform3D(Basis(), a), tr * 1.12, th, th + tr * 1.5, 12, ROOF)
	var top := th + tr * 1.5
	var keep: Dictionary = p.get("keep", {})
	if not keep.is_empty():
		var at: Array = keep.get("at", [0, 0])
		var kc := Transform3D(Basis(), Vector3(float(at[0]), 0, -float(at[1])))
		var kr := float(keep.get("radius_m", 7.0))
		var kh := float(keep.get("height_m", 28.0))
		_cylinder(st, kc, kr, -FOUNDATION_M, kh, 16, MASONRY, STONE_TINT)
		_cone(st, kc, kr * 1.1, kh, kh + kr * 1.6, 16, ROOF)
		top = maxf(top, kh + kr * 1.6)
	for hall in p.get("halls", []):
		var t := Transform3D(Basis(Vector3.UP, deg_to_rad(float(hall[5]))), Vector3(float(hall[0]), 0, -float(hall[1])))
		var hl := float(hall[2])
		var hd := float(hall[3])
		var hh := float(hall[4])
		_box(st, t, Vector3.ZERO, Vector3(hl, hh, hd), MASONRY, STONE_TINT)
		_gable_roof_x(st, t, -hl * 0.5, hl * 0.5, hd * 0.5 + 0.5, hh, hh + hd * 0.5, ROOF, true, true)
	return top


static func _belfry(st: SurfaceTool, p: Dictionary) -> float:
	var size := float(p.get("size", 10.0))
	var h := float(p.get("height", 30.0))
	var id := Transform3D.IDENTITY
	_box(st, id, Vector3.ZERO, Vector3(size, h, size), WALL, STONE_TINT)
	# VH6 : donjon à toit plat et tourelles d'angle (Tour Blanche, tour du Joyau).
	if str(p.get("top", "pyramid")) == "turrets":
		var t := maxf(size * 0.16, 2.5)
		var th := maxf(size * 0.18, 3.0)
		for sx: float in [-1.0, 1.0]:
			for sz: float in [-1.0, 1.0]:
				_box(st, id, Vector3(sx * (size - t) * 0.5, h, sz * (size - t) * 0.5), Vector3(t, th, t), WALL, STONE_TINT, h)
		_box(st, id, Vector3(0, h, 0), Vector3(size - 2.0 * t, 1.2, size - 2.0 * t), MASONRY, STONE_TINT, h)
		return h + th
	if str(p.get("top", "pyramid")) == "lantern":
		_box(st, id, Vector3(0, h, 0), Vector3(size * 0.55, size * 0.6, size * 0.55), WALL, STONE_TINT, h)
		_pyramid(st, id, size * 0.35, h + size * 0.6, h + size * 1.3, ROOF)
		return h + size * 1.3
	_pyramid(st, id, size * 0.5 + 0.4, h, h + size * 0.9, ROOF)
	return h + size * 0.9


static func _hall(st: SurfaceTool, p: Dictionary) -> float:
	var length := float(p.get("length_m", 50.0))
	var width := float(p.get("width_m", 20.0))
	var h := float(p.get("height_m", 12.0))
	var id := Transform3D.IDENTITY
	var eave := h * 0.4
	_box(st, id, Vector3.ZERO, Vector3(length, eave, width), MASONRY, RUBBLE_TINT)
	_gable_roof_x(st, id, -length * 0.5 - 0.5, length * 0.5 + 0.5, width * 0.5 + 0.8, eave, h, ROOF_TILE, true, true)
	return h


static func _palace(st: SurfaceTool, p: Dictionary) -> float:
	var length := float(p.get("length_m", 35.0))
	var width := float(p.get("width_m", 12.0))
	var h := float(p.get("height_m", 11.0))
	var id := Transform3D.IDENTITY
	_box(st, id, Vector3.ZERO, Vector3(length, h, width), WALL, STONE_TINT)
	_gable_roof_x(st, id, -length * 0.5, length * 0.5, width * 0.5 + 0.5, h, h + width * 0.55, ROOF, true, true)
	for r in p.get("ranges", []):
		var t := Transform3D(Basis(Vector3.UP, deg_to_rad(float(r[5]))), Vector3(float(r[0]), 0, -float(r[1])))
		_box(st, t, Vector3.ZERO, Vector3(float(r[2]), float(r[4]), float(r[3])), WALL, STONE_TINT)
		_gable_roof_x(st, t, -float(r[2]) * 0.5, float(r[2]) * 0.5, float(r[3]) * 0.5 + 0.4, float(r[4]), float(r[4]) + float(r[3]) * 0.5, ROOF, true, true)
	return h + width * 0.55


static func _enclosure(st: SurfaceTool, p: Dictionary) -> float:
	var length := float(p.get("length_m", 60.0))
	var width := float(p.get("width_m", 40.0))
	var h := float(p.get("height_m", 6.0))
	var id := Transform3D.IDENTITY
	for side: float in [-1.0, 1.0]:
		_box(st, id, Vector3(0, 0, side * width * 0.5), Vector3(length, h, 1.4), MASONRY, RUBBLE_TINT)
		_box(st, id, Vector3(side * length * 0.5, 0, 0), Vector3(1.4, h, width), MASONRY, RUBBLE_TINT)
	var top := h
	if bool(p.get("ranges", false)):
		var rw := minf(10.0, width * 0.22)
		for side: float in [-1.0, 1.0]:
			var t := Transform3D(Basis(), Vector3(0, 0, side * (width * 0.5 - rw * 0.5 - 0.7)))
			var rl := length * 0.8
			_box(st, t, Vector3.ZERO, Vector3(rl, h + 3.0, rw), MASONRY, RUBBLE_TINT)
			_gable_roof_x(st, t, -rl * 0.5, rl * 0.5, rw * 0.5 + 0.4, h + 3.0, h + 3.0 + rw * 0.5, ROOF_TILE, true, true)
			top = h + 3.0 + rw * 0.5
	return top


## VH7 : ouvrage de terre et de bois (boulevard) : levée trapézoïdale le long de `ring`
## ([[u, v]…], u le long de l'axe, v à gauche, comme `castle`), haute de `bank_m`, large de
## `base_m` à la base, surmontée d'une palissade de `palisade_m` ; `closed` : faux pour un fer à
## cheval ouvert (côté de la porte ou du fort).
static func _earthwork(st: SurfaceTool, p: Dictionary) -> float:
	var pts: Array[Vector3] = []
	for q in p.get("ring", []):
		pts.append(Vector3(float(q[0]), 0, -float(q[1])))
	var bank := float(p.get("bank_m", 4.0))
	var base := float(p.get("base_m", 9.0))
	var crest := base * 0.3
	var palisade := float(p.get("palisade_m", 2.5))
	var closed := bool(p.get("closed", false))
	var earth := EARTH_TINT
	var count := pts.size() if closed else pts.size() - 1
	for i in count:
		var a := pts[i]
		var b := pts[(i + 1) % pts.size()]
		var d := b - a
		var length := d.length() + base * 0.25  # recouvrement aux angles
		var t := Transform3D(Basis(Vector3.UP, -atan2(d.z, d.x)), (a + b) * 0.5)
		var hx := length * 0.5
		var hb := base * 0.5
		var hc := crest * 0.5
		var ref := Vector3(0, bank * 0.3, 0)
		for side: float in [-1.0, 1.0]:
			_quad_oriented(st, t, Vector3(-hx, -2.0, side * hb), Vector3(hx, -2.0, side * hb), Vector3(hx, bank, side * hc), Vector3(-hx, bank, side * hc), RUBBLE, earth, ref)
		_quad_oriented(st, t, Vector3(-hx, bank, -hc), Vector3(hx, bank, -hc), Vector3(hx, bank, hc), Vector3(-hx, bank, hc), RUBBLE, earth, Vector3(0, 0, 0))
		for sx: float in [-hx, hx]:
			_quad_oriented(st, t, Vector3(sx, -2.0, -hb), Vector3(sx, -2.0, hb), Vector3(sx, bank, hc), Vector3(sx, bank, -hc), RUBBLE, earth, Vector3(0, bank * 0.3, 0))
		if palisade > 0.0:
			_box(st, t, Vector3(0, bank, 0), Vector3(d.length(), palisade, 0.45), TIMBER, WOOD_TINT, bank - 0.5)
	return bank + palisade


# --- Primitives (repère du monument, mètres) ------------------------------------------------


static func _color(layer: String, tint: Color = Color(1, 1, 1)) -> Color:
	return TownBuilder.layer_color(layer, tint)


## Triangle orienté à l'opposé de `ref` (normale plane).
static func _tri_oriented(st: SurfaceTool, a: Vector3, b: Vector3, c: Vector3, color: Color, ref: Vector3) -> void:
	var n := (b - a).cross(c - a)
	if n.length_squared() < 1e-10:
		return
	if n.dot((a + b + c) / 3.0 - ref) < 0.0:
		var tmp := b
		b = c
		c = tmp
		n = -n
	n = n.normalized()
	for q in [a, b, c]:
		st.set_color(color)
		st.set_normal(n)
		st.set_uv(Vector2.ZERO)
		st.add_vertex(q)


static func _quad_oriented(st: SurfaceTool, t: Transform3D, a: Vector3, b: Vector3, c: Vector3, d: Vector3, layer: String, tint: Color, ref: Vector3) -> void:
	var col := _color(layer, tint)
	var ra := t * a
	var rb := t * b
	var rc := t * c
	var rd := t * d
	var rr := t * ref
	_tri_oriented(st, ra, rb, rc, col, rr)
	_tri_oriented(st, ra, rc, rd, col, rr)


## Pavé : pied au centre `center` (x, z ; y = départ), hauteur `size.y` au-dessus de `center.y`,
## côtés `size.x` × `size.z` ; les murs descendent jusqu'à `bottom` (fondation par défaut).
static func _box(st: SurfaceTool, t: Transform3D, center: Vector3, size: Vector3, layer: String, tint: Color, bottom: float = -FOUNDATION_M) -> void:
	var hx := size.x * 0.5
	var hz := size.z * 0.5
	var y0 := bottom
	var y1 := center.y + size.y
	var c := [Vector3(-hx, 0, -hz), Vector3(hx, 0, -hz), Vector3(hx, 0, hz), Vector3(-hx, 0, hz)]
	var mid := Vector3(center.x, (y0 + y1) * 0.5, center.z)
	for i in 4:
		var a: Vector3 = center + c[i]
		var b: Vector3 = center + c[(i + 1) % 4]
		_quad_oriented(st, t, Vector3(a.x, y0, a.z), Vector3(b.x, y0, b.z), Vector3(b.x, y1, b.z), Vector3(a.x, y1, a.z), layer, tint, mid)
	var t0: Vector3 = center + c[0]
	var t1: Vector3 = center + c[1]
	var t2: Vector3 = center + c[2]
	var t3: Vector3 = center + c[3]
	_quad_oriented(st, t, Vector3(t0.x, y1, t0.z), Vector3(t1.x, y1, t1.z), Vector3(t2.x, y1, t2.z), Vector3(t3.x, y1, t3.z), layer, tint, mid)


## Toit à deux pans le long de X (de `x0` à `x1`), demi-largeur `half_w`, égout `eave`,
## faîtage `ridge` ; pignons maçonnés aux extrémités demandées.
static func _gable_roof_x(st: SurfaceTool, t: Transform3D, x0: float, x1: float, half_w: float, eave: float, ridge: float, layer: String, gable0: bool, gable1: bool) -> void:
	var ref := Vector3((x0 + x1) * 0.5, eave, 0)
	_quad_oriented(st, t, Vector3(x0, eave, -half_w), Vector3(x1, eave, -half_w), Vector3(x1, ridge, 0), Vector3(x0, ridge, 0), layer, Color(1, 1, 1), ref)
	_quad_oriented(st, t, Vector3(x0, eave, half_w), Vector3(x1, eave, half_w), Vector3(x1, ridge, 0), Vector3(x0, ridge, 0), layer, Color(1, 1, 1), ref)
	var wall := _color(WALL, STONE_TINT)
	if gable0:
		_tri_oriented(st, t * Vector3(x0, eave, -half_w), t * Vector3(x0, eave, half_w), t * Vector3(x0, ridge, 0), wall, t * ref)
	if gable1:
		_tri_oriented(st, t * Vector3(x1, eave, -half_w), t * Vector3(x1, eave, half_w), t * Vector3(x1, ridge, 0), wall, t * ref)


static func _pyramid(st: SurfaceTool, t: Transform3D, half: float, y0: float, y1: float, layer: String) -> void:
	var col := _color(layer)
	var c := [Vector3(-half, y0, -half), Vector3(half, y0, -half), Vector3(half, y0, half), Vector3(-half, y0, half)]
	var apex := t * Vector3(0, y1, 0)
	var ref := t * Vector3(0, y0, 0)
	for i in 4:
		_tri_oriented(st, t * (c[i] as Vector3), t * (c[(i + 1) % 4] as Vector3), apex, col, ref)


static func _cylinder(st: SurfaceTool, t: Transform3D, r: float, y0: float, y1: float, sides: int, layer: String, tint: Color) -> void:
	_arc_wall(st, t, r, y0, y1, sides, 0.0, TAU, layer, tint)


static func _half_cylinder(st: SurfaceTool, t: Transform3D, r: float, y0: float, y1: float, sides: int, layer: String, tint: Color) -> void:
	_arc_wall(st, t, r, y0, y1, sides, -PI * 0.5, PI * 0.5, layer, tint)


static func _arc_wall(st: SurfaceTool, t: Transform3D, r: float, y0: float, y1: float, sides: int, a0: float, a1: float, layer: String, tint: Color) -> void:
	var col := _color(layer, tint)
	var ref := t * Vector3(0, (y0 + y1) * 0.5, 0)
	for i in sides:
		var u0 := lerpf(a0, a1, float(i) / sides)
		var u1 := lerpf(a0, a1, float(i + 1) / sides)
		var p0 := Vector3(cos(u0) * r, 0, sin(u0) * r)
		var p1 := Vector3(cos(u1) * r, 0, sin(u1) * r)
		var q0 := t * (p0 + Vector3(0, y0, 0))
		var q1 := t * (p1 + Vector3(0, y0, 0))
		var q2 := t * (p1 + Vector3(0, y1, 0))
		var q3 := t * (p0 + Vector3(0, y1, 0))
		_tri_oriented(st, q0, q1, q2, col, ref)
		_tri_oriented(st, q0, q2, q3, col, ref)
	if a1 - a0 >= TAU - 0.001:
		return
	# Toit plat de la demi-tour (fermeture), la toiture le recouvre.


static func _cone(st: SurfaceTool, t: Transform3D, r: float, y0: float, y1: float, sides: int, layer: String) -> void:
	_frustum(st, t, r, y0, 0.0, y1, sides, 0.0, TAU, layer)


static func _half_cone(st: SurfaceTool, t: Transform3D, r0: float, y0: float, r1: float, y1: float, sides: int, layer: String) -> void:
	_frustum(st, t, r0, y0, r1, y1, sides, -PI * 0.5, PI * 0.5, layer)


static func _frustum(st: SurfaceTool, t: Transform3D, r0: float, y0: float, r1: float, y1: float, sides: int, a0: float, a1: float, layer: String) -> void:
	var col := _color(layer)
	var ref := t * Vector3(0, y0, 0)
	for i in sides:
		var u0 := lerpf(a0, a1, float(i) / sides)
		var u1 := lerpf(a0, a1, float(i + 1) / sides)
		var b0 := t * Vector3(cos(u0) * r0, y0, sin(u0) * r0)
		var b1 := t * Vector3(cos(u1) * r0, y0, sin(u1) * r0)
		var c0 := t * Vector3(cos(u0) * r1, y1, sin(u0) * r1)
		var c1 := t * Vector3(cos(u1) * r1, y1, sin(u1) * r1)
		_tri_oriented(st, b0, b1, c1, col, ref)
		if r1 > 0.01:
			_tri_oriented(st, b0, c1, c0, col, ref)
