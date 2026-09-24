class_name BattleMeshes
extends RefCounted

## Maillages procéduraux des batailles (SurfaceTool), lot V4 « semi-réaliste » :
## - figurines à proportions humaines (≈ 1,8 m) par famille et variante : hommes d'armes
##   (bassinet, cotte de mailles, surcot, épée, écu armorié), piquiers flamands (chapel de fer,
##   gambison, pique de 4,5 m), milice (lance, écu), archers longs (arc de 1,9 m, carquois),
##   arbalétriers (arbalète, pavois des Génois), chevaliers (heaume, caparaçon, lance), sergents
##   montés, archers montés ; engins (mangonneau, trébuchet, bombarde) avec leurs servants ;
## - arbres (tronc texturé + houppier en cartes alpha), buissons, rochers, hampe, drapeau.
##
## Lot B1 : les fantassins, archers et cavaliers viennent de maillages modelés sous Blender
## (`tools/blender/battle_figures.py` → `assets/models/battle/<famille>_<variante>[_lod|_far].glb`,
## pivots dans `figures.json`), convertis au chargement vers le format ci-dessous et mis en
## cache ; les engins de siège et le repli (modèle absent) restent procéduraux (SurfaceTool).
##
## Format des sommets des figurines (lu par `battle_soldier.gdshader`) :
## - COLOR.rgb = couleur (linéaire), COLOR.a = code matière / 5 : 0 livrée, 1 bordure (or),
##   2 métal, 3 tissu (teinte variée par soldat), 4 couleur exacte, 5 armoiries (UV → blason ;
##   UV hors de [0, 1] : livrée unie) ;
## - CUSTOM0 = (membre, pivot y, pivot z, poids de flexion) : le shader fait pivoter chaque
##   membre autour de l'axe X passant par son pivot (jambes aux hanches, bras aux épaules, arme
##   à la main…) ; les sommets pondérés des jambes (homme, cheval) plient d'abord au genou ou au
##   jarret, dont le pivot (y, z) est dans UV2.
## La figurine regarde +Z ; sa main droite est en -X.

## Membres (CUSTOM0.x), partagés avec le shader.
const P_BODY := 0
const P_LEG_L := 1
const P_LEG_R := 2
const P_ARM_R := 3
const P_ARM_L := 4
const P_HEAD := 5
const P_HORSE := 6
const P_HLEG_FL := 7
const P_HLEG_FR := 8
const P_HLEG_BL := 9
const P_HLEG_BR := 10
const P_HNECK := 11
const P_RIDER := 12
const P_WEAPON := 13
const P_STATIC := 14
const P_ENGINE_ARM := 15
const P_CLOTH := 16
const P_BOW := 17

## Codes matière (COLOR.a × 5).
const C_LIVERY := 0
const C_TRIM := 1
const C_METAL := 2
const C_CLOTH := 3
const C_EXACT := 4
const C_ARMS := 5

const SKIN := Color(0.80, 0.60, 0.47)
const STEEL := Color(0.60, 0.62, 0.66)
const MAIL := Color(0.45, 0.46, 0.48)
const WOOD := Color(0.42, 0.29, 0.17)
const WOOD_DARK := Color(0.26, 0.18, 0.11)
const LEATHER := Color(0.36, 0.24, 0.14)
const HOSE := Color(0.42, 0.36, 0.30)
const GAMBESON := Color(0.70, 0.63, 0.50)
const HORSE_COATS := [Color(0.36, 0.23, 0.14), Color(0.20, 0.14, 0.10), Color(0.52, 0.40, 0.28)]
const HORSE_DARK := Color(0.12, 0.09, 0.07)
const IRON := Color(0.22, 0.22, 0.24)
const STRING := Color(0.85, 0.82, 0.72)

## Variantes par type d'unité (données `data/unit_types`), repli 0.
const VARIANTS := {
	"unit_men_at_arms_foot": 0, "unit_flemish_pikemen": 1, "unit_urban_militia": 2,
	"unit_longbowmen": 0, "unit_crossbowmen": 1, "unit_genoese_crossbowmen": 2,
	"unit_knights": 0, "unit_mounted_sergeants": 1, "unit_mounted_archers": 2,
	"unit_mangonel": 0, "unit_trebuchet": 1, "unit_bombard": 2,
}

const MODELS_DIR := "res://assets/models/battle/"
const LEVEL_SUFFIX := ["", "_lod", "_far"]
## Niveaux de détail des figurines modelées.
const LEVEL_FULL := 0
const LEVEL_MEDIUM := 1
const LEVEL_FAR := 2

static var _cache: Dictionary = {}
static var _figures_meta: Dictionary = {}


static func variant_of(unit_type: String) -> int:
	return int(VARIANTS.get(unit_type, 0))


## Mode d'arme (uniforme `weapon_mode` du shader) d'une famille/variante :
## 0 épée, 1 arme d'hast, 2 arc, 3 arbalète, 4 lance de cavalier, 5 engin.
static func weapon_mode(kind: String, variant: int) -> int:
	match kind:
		"infantry":
			return 0 if variant == 0 else 1
		"archer":
			return 2 if variant == 0 else 3
		"cavalry":
			return 2 if variant == 2 else 4
		"siege":
			return 5
	return 0


# --- Constructeur de figurines -----------------------------------------------------------


## Accumulateur de sommets avec l'état courant (couleur, code, membre, pivot).
class Fig:
	var st := SurfaceTool.new()
	var color := Color.WHITE
	var code := C_EXACT
	var part := P_BODY
	var pivot := Vector2.ZERO
	## Niveau de détail allégé (figurines lointaines) : moins de côtés, pas de cordes ni de
	## tranches d'écu.
	var lod := false

	func _init() -> void:
		st.begin(Mesh.PRIMITIVE_TRIANGLES)
		st.set_custom_format(0, SurfaceTool.CUSTOM_RGBA_FLOAT)

	func set_style(p_color: Color, p_code: int = C_EXACT) -> Fig:
		color = p_color
		code = p_code
		return self

	func set_part(p_part: int, p_pivot: Vector2 = Vector2.ZERO) -> Fig:
		part = p_part
		pivot = p_pivot
		return self

	func vert(p: Vector3, n: Vector3, uv: Vector2 = Vector2.ZERO) -> void:
		var c := color.srgb_to_linear()
		st.set_color(Color(c.r, c.g, c.b, float(code) / 5.0))
		st.set_normal(n)
		st.set_uv(uv)
		st.set_custom(0, Color(float(part), pivot.x, pivot.y, 0.0))
		st.add_vertex(p)

	## Triangle en ordre horaire vu du côté de sa normale moyenne (face avant pour Godot).
	func tri(a: Vector3, b: Vector3, c: Vector3, na: Vector3, nb: Vector3, nc: Vector3, ua := Vector2.ZERO, ub := Vector2.ZERO, uc := Vector2.ZERO) -> void:
		if (b - a).cross(c - a).dot(na + nb + nc) > 0.0:
			vert(a, na, ua)
			vert(c, nc, uc)
			vert(b, nb, ub)
		else:
			vert(a, na, ua)
			vert(b, nb, ub)
			vert(c, nc, uc)

	func flat(a: Vector3, b: Vector3, c: Vector3, n: Vector3) -> void:
		tri(a, b, c, n, n, n)

	## Tronc de cône elliptique de `a` à `b` (rayons ra → rb, aplatissement `squash` en Z local).
	func cyl(a: Vector3, b: Vector3, ra: float, rb: float, sides: int = 6, caps: bool = true, squash: float = 1.0) -> void:
		if lod:
			if maxf(ra, rb) < 0.012:
				return
			sides = maxi(3, (sides + 1) / 2) if sides < 8 else sides / 2 + 1
		var axis := (b - a).normalized()
		var ref := Vector3.FORWARD if absf(axis.dot(Vector3.FORWARD)) < 0.9 else Vector3.RIGHT
		var u := axis.cross(ref).normalized()
		var w := axis.cross(u).normalized()
		# Aplatissement le long de l'axe Z du modèle quand c'est possible.
		if absf(u.z) > absf(w.z):
			var tmp := u
			u = w
			w = tmp
		var ring_a: Array[Vector3] = []
		var ring_b: Array[Vector3] = []
		var normals: Array[Vector3] = []
		for i in sides:
			var t := TAU * (float(i) + 0.5) / float(sides)
			var d := u * cos(t) + w * sin(t) * squash
			ring_a.append(a + d * ra)
			ring_b.append(b + d * rb)
			normals.append((u * cos(t) + w * sin(t) / maxf(squash, 0.1)).normalized())
		for i in sides:
			var j := (i + 1) % sides
			tri(ring_a[i], ring_b[i], ring_b[j], normals[i], normals[i], normals[j])
			tri(ring_a[i], ring_b[j], ring_a[j], normals[i], normals[j], normals[j])
		if caps:
			for i in range(1, sides - 1):
				if rb > 0.001:
					flat(ring_b[0], ring_b[i], ring_b[i + 1], axis)
				if ra > 0.001:
					flat(ring_a[0], ring_a[i + 1], ring_a[i], -axis)

	## Ellipsoïde low-poly (normales lisses).
	func ellipsoid(c: Vector3, r: Vector3, rings: int = 4, segs: int = 6, y_min: float = -1.0) -> void:
		if lod:
			if maxf(r.x, maxf(r.y, r.z)) < 0.06:
				return
			rings = maxi(2, rings / 2)
			segs = maxi(4, segs - 2 - segs / 4)
		var pts: Array = []
		for i in rings + 1:
			var v := lerpf(-PI * 0.5, PI * 0.5, float(i) / float(rings))
			v = maxf(v, asin(clampf(y_min, -1.0, 1.0)))
			var row: Array[Vector3] = []
			for j in segs:
				var h := TAU * float(j) / float(segs)
				row.append(Vector3(cos(v) * cos(h), sin(v), cos(v) * sin(h)))
			pts.append(row)
		for i in rings:
			for j in segs:
				var k := (j + 1) % segs
				var n00: Vector3 = pts[i][j]
				var n01: Vector3 = pts[i][k]
				var n10: Vector3 = pts[i + 1][j]
				var n11: Vector3 = pts[i + 1][k]
				var p00 := c + n00 * r
				var p01 := c + n01 * r
				var p10 := c + n10 * r
				var p11 := c + n11 * r
				if p00.distance_to(p01) > 0.0001:
					tri(p00, p10, p01, (n00 / r).normalized(), (n10 / r).normalized(), (n01 / r).normalized())
				if p10.distance_to(p11) > 0.0001:
					tri(p01, p10, p11, (n01 / r).normalized(), (n10 / r).normalized(), (n11 / r).normalized())

	## Pavé orienté (normales plates).
	func box(c: Vector3, size: Vector3, basis: Basis = Basis()) -> void:
		var h := size * 0.5
		var corners: Array[Vector3] = []
		for i in 8:
			var local := Vector3(h.x if i & 1 else -h.x, h.y if i & 2 else -h.y, h.z if i & 4 else -h.z)
			corners.append(c + basis * local)
		var faces := [
			[[0, 2, 3, 1], Vector3.BACK * -1.0], [[4, 5, 7, 6], Vector3.BACK],
			[[0, 4, 6, 2], Vector3.LEFT], [[1, 3, 7, 5], Vector3.RIGHT],
			[[0, 1, 5, 4], Vector3.DOWN], [[2, 6, 7, 3], Vector3.UP],
		]
		for face in faces:
			var idx: Array = face[0]
			var n: Vector3 = (basis * (face[1] as Vector3)).normalized()
			flat(corners[idx[0]], corners[idx[1]], corners[idx[2]], n)
			flat(corners[idx[0]], corners[idx[2]], corners[idx[3]], n)

	## Polygone plat à deux faces : avant en `front_code` (UV = coordonnées normalisées du
	## polygone, pour les armoiries), arrière en couleur `back`.
	func plate(points: PackedVector2Array, origin: Vector3, basis: Basis, thickness: float, back: Color, front_code: int) -> void:
		var minp := points[0]
		var maxp := points[0]
		for p in points:
			minp = minp.min(p)
			maxp = maxp.max(p)
		var n := (basis * Vector3.BACK).normalized()
		var off := n * thickness * 0.5
		var saved_color := color
		var saved_code := code
		var center := Vector2.ZERO
		for p in points:
			center += p
		center /= float(points.size())
		for i in points.size():
			var p0 := points[i]
			var p1 := points[(i + 1) % points.size()]
			var a := origin + basis * Vector3(center.x, center.y, 0.0)
			var b := origin + basis * Vector3(p0.x, p0.y, 0.0)
			var c := origin + basis * Vector3(p1.x, p1.y, 0.0)
			var uv := func(q: Vector2) -> Vector2: return Vector2((q.x - minp.x) / (maxp.x - minp.x), 1.0 - (q.y - minp.y) / (maxp.y - minp.y))
			color = saved_color
			code = front_code
			tri(a + off, b + off, c + off, n, n, n, uv.call(center), uv.call(p0), uv.call(p1))
			color = back
			code = C_EXACT
			tri(a - off, c - off, b - off, -n, -n, -n)
			if lod:
				continue
			# Tranche.
			var e := (basis * Vector3(p1.x - p0.x, p1.y - p0.y, 0.0)).cross(n).normalized()
			flat(b + off, c + off, c - off, e)
			flat(b + off, c - off, b - off, e)
		color = saved_color
		code = saved_code

	func commit() -> ArrayMesh:
		st.index()
		return st.commit()


# --- Figurines -----------------------------------------------------------------------------


## Figurine de `kind` (infantry, archer, cavalry, siege) et de variante `variant` (cf. VARIANTS) ;
## `lod` = niveau lointain (cadavres, ombres). Mise en cache : les régiments d'un même type
## partagent le maillage.
static func soldier(kind: String, variant: int = 0, lod: bool = false) -> ArrayMesh:
	return soldier_level(kind, variant, LEVEL_FAR if lod else LEVEL_FULL)


## Figurine au niveau de détail `level` (LEVEL_FULL, LEVEL_MEDIUM, LEVEL_FAR).
static func soldier_level(kind: String, variant: int, level: int) -> ArrayMesh:
	var key := "%s/%d/%d" % [kind, variant, level]
	if _cache.has(key):
		return _cache[key]
	var mesh: ArrayMesh = null
	# `--legacy-figures` (après `--`) : figurines procédurales V4, pour les comparaisons avant/après.
	if kind != "siege" and not OS.get_cmdline_user_args().has("--legacy-figures"):
		mesh = _load_figure(kind, variant, level)
	if mesh == null:
		mesh = _procedural_soldier(kind, variant, level != LEVEL_FULL)
	_cache[key] = mesh
	return mesh


## Maillage Blender converti au format du shader : membre (UV2.x du glb) → CUSTOM0 avec le pivot
## du membre, poids de flexion (UV2.y du glb) → CUSTOM0.w, pivot du genou → UV2. Null si absent.
static func _load_figure(kind: String, variant: int, level: int) -> ArrayMesh:
	var figure := "%s_%d" % [kind, variant]
	var path: String = MODELS_DIR + figure + LEVEL_SUFFIX[level] + ".glb"
	var meta := _figure_meta(figure)
	if meta.is_empty() or not ResourceLoader.exists(path):
		return null
	var scene := load(path) as PackedScene
	if scene == null:
		return null
	var root := scene.instantiate()
	var source := _first_mesh(root)
	root.free()
	if source == null or source.get_surface_count() == 0:
		return null
	var pivots: Dictionary = meta.get("pivots", {})
	var knees: Dictionary = meta.get("knees", {})
	var mesh := ArrayMesh.new()
	for s in source.get_surface_count():
		var arrays := source.surface_get_arrays(s)
		var verts: PackedVector3Array = arrays[Mesh.ARRAY_VERTEX]
		var parts: PackedVector2Array = arrays[Mesh.ARRAY_TEX_UV2]
		var n := verts.size()
		var custom := PackedFloat32Array()
		custom.resize(n * 4)
		var knee_uv := PackedVector2Array()
		knee_uv.resize(n)
		for i in n:
			var part := roundi(parts[i].x)
			var key := str(part)
			var pivot: Array = pivots.get(key, [0.0, 0.0])
			custom[i * 4] = float(part)
			custom[i * 4 + 1] = float(pivot[0])
			custom[i * 4 + 2] = float(pivot[1])
			custom[i * 4 + 3] = clampf(parts[i].y, 0.0, 1.0)
			if knees.has(key):
				var knee: Array = knees[key]
				knee_uv[i] = Vector2(float(knee[0]), float(knee[1]))
		var out := []
		out.resize(Mesh.ARRAY_MAX)
		out[Mesh.ARRAY_VERTEX] = verts
		out[Mesh.ARRAY_NORMAL] = arrays[Mesh.ARRAY_NORMAL]
		out[Mesh.ARRAY_COLOR] = arrays[Mesh.ARRAY_COLOR]
		out[Mesh.ARRAY_TEX_UV] = arrays[Mesh.ARRAY_TEX_UV]
		out[Mesh.ARRAY_TEX_UV2] = knee_uv
		out[Mesh.ARRAY_CUSTOM0] = custom
		out[Mesh.ARRAY_INDEX] = arrays[Mesh.ARRAY_INDEX]
		mesh.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES, out, [], {}, Mesh.ARRAY_CUSTOM_RGBA_FLOAT << Mesh.ARRAY_FORMAT_CUSTOM0_SHIFT)
	return mesh


## Pivots des membres d'une figurine (`figures.json`, écrit par le script Blender).
static func _figure_meta(figure: String) -> Dictionary:
	if _figures_meta.is_empty():
		var text := FileAccess.get_file_as_string(MODELS_DIR + "figures.json")
		var parsed = JSON.parse_string(text) if text != "" else null
		_figures_meta = parsed.get("figures", {}) if parsed is Dictionary else {"": {}}
	return _figures_meta.get(figure, {})


static func _first_mesh(node: Node) -> Mesh:
	if node is MeshInstance3D and (node as MeshInstance3D).mesh != null:
		return (node as MeshInstance3D).mesh
	for child in node.get_children():
		var found := _first_mesh(child)
		if found != null:
			return found
	return null


## Figurine procédurale (lot V4) : engins de siège, et repli si le modèle Blender manque.
static func _procedural_soldier(kind: String, variant: int, lod: bool) -> ArrayMesh:
	var f := Fig.new()
	f.lod = lod
	match kind:
		"archer":
			_archer(f, variant)
		"cavalry":
			_cavalry(f, variant)
		"siege":
			_engine(f, variant)
		_:
			_infantry(f, variant)
	return f.commit()


## Jambes, pieds, bassin : chausses (tissu varié) et souliers ; `armored` = grèves d'acier.
static func _legs(f: Fig, armored: bool, hip_y: float = 0.93) -> void:
	for side in [1.0, -1.0]:
		var x: float = 0.1 * side
		f.set_part(P_LEG_L if side > 0.0 else P_LEG_R, Vector2(hip_y, 0.0))
		f.set_style(MAIL if armored else HOSE, C_EXACT if armored else C_CLOTH)
		f.cyl(Vector3(x, hip_y, 0.0), Vector3(x * 1.05, 0.5, 0.01), 0.095, 0.07, 6, false)
		if armored:
			f.set_style(STEEL, C_METAL)
		f.cyl(Vector3(x * 1.05, 0.5, 0.01), Vector3(x * 1.08, 0.09, 0.0), 0.07, 0.052, 6, false)
		f.set_style(LEATHER, C_EXACT)
		f.box(Vector3(x * 1.08, 0.045, 0.045), Vector3(0.1, 0.09, 0.26))


## Buste : torse (maille ou gambison), surcot de livrée avec jupe, ceinture, cou, tête.
static func _torso(f: Fig, under: Color, under_code: int, surcoat: bool, skirt_to: float = 0.55) -> void:
	f.set_part(P_BODY)
	f.set_style(under, under_code)
	f.cyl(Vector3(0, 0.9, 0), Vector3(0, 1.45, 0), 0.17, 0.215, 8, true, 0.7)
	for side in [-1.0, 1.0]:
		f.ellipsoid(Vector3(0.2 * side, 1.4, 0.0), Vector3(0.085, 0.07, 0.085), 3, 6)
	if surcoat:
		f.set_style(Color.WHITE, C_LIVERY)
		f.cyl(Vector3(0, 0.95, 0), Vector3(0, 1.4, 0), 0.185, 0.22, 8, false, 0.72)
	f.set_part(P_CLOTH)
	f.set_style(Color.WHITE if surcoat else under, C_LIVERY if surcoat else under_code)
	f.cyl(Vector3(0, 0.97, 0), Vector3(0, skirt_to, 0), 0.19, 0.25, 8, false, 0.8)
	f.set_part(P_BODY)
	f.set_style(Color.WHITE, C_TRIM)
	f.cyl(Vector3(0, 0.93, 0), Vector3(0, 0.99, 0), 0.195, 0.195, 8, false, 0.76)
	f.set_part(P_HEAD)
	f.set_style(SKIN, C_EXACT)
	f.cyl(Vector3(0, 1.43, 0), Vector3(0, 1.53, 0), 0.055, 0.05, 5, false)
	f.ellipsoid(Vector3(0, 1.63, 0.01), Vector3(0.095, 0.115, 0.105), 4, 7)


## Bras droit (-X) et gauche (+X) : de l'épaule au poing, légèrement fléchis vers l'avant.
static func _arms(f: Fig, sleeve: Color, sleeve_code: int, gauntlet: Color, shoulder_y: float = 1.4) -> void:
	for side in [-1.0, 1.0]:
		f.set_part(P_ARM_R if side < 0.0 else P_ARM_L, Vector2(shoulder_y, 0.0))
		f.set_style(sleeve, sleeve_code)
		var shoulder := Vector3(0.23 * side, shoulder_y, 0.0)
		var elbow := Vector3(0.26 * side, shoulder_y - 0.3, -0.02)
		var hand := Vector3(0.25 * side, shoulder_y - 0.5, 0.12)
		f.cyl(shoulder, elbow, 0.068, 0.056, 6, false)
		f.cyl(elbow, hand, 0.056, 0.046, 6, false)
		f.set_style(gauntlet, C_METAL if gauntlet == STEEL else C_EXACT)
		f.ellipsoid(hand + Vector3(0, -0.02, 0.01), Vector3(0.045, 0.05, 0.05), 2, 5)


## Casques : bassinet à camail, chapel de fer, chaperon, heaume.
static func _helmet(f: Fig, style: String) -> void:
	f.set_part(P_HEAD)
	match style:
		"bassinet":
			f.set_style(STEEL, C_METAL)
			f.cyl(Vector3(0, 1.62, 0.0), Vector3(0, 1.72, -0.01), 0.112, 0.105, 8, false)
			f.cyl(Vector3(0, 1.72, -0.01), Vector3(0, 1.82, -0.04), 0.105, 0.0, 8, false)
			f.set_style(MAIL, C_METAL)
			f.cyl(Vector3(0, 1.62, -0.01), Vector3(0, 1.43, 0.0), 0.115, 0.17, 8, false)
		"kettle":
			f.set_style(STEEL, C_METAL)
			f.ellipsoid(Vector3(0, 1.67, 0.0), Vector3(0.12, 0.11, 0.12), 3, 8, 0.0)
			f.cyl(Vector3(0, 1.665, 0), Vector3(0, 1.675, 0), 0.21, 0.21, 10, true)
		"hood":
			f.set_style(Color.WHITE, C_LIVERY)
			f.ellipsoid(Vector3(0, 1.66, -0.01), Vector3(0.115, 0.12, 0.12), 3, 7, -0.2)
			f.cyl(Vector3(0, 1.55, -0.01), Vector3(0, 1.4, 0.0), 0.12, 0.2, 7, false)
		"greathelm":
			f.set_style(STEEL, C_METAL)
			f.cyl(Vector3(0, 1.52, 0.005), Vector3(0, 1.82, 0.005), 0.125, 0.12, 8, true)
			f.set_style(Color(0.08, 0.08, 0.08), C_EXACT)
			f.box(Vector3(0, 1.69, 0.128), Vector3(0.16, 0.02, 0.01))
			f.set_style(Color.WHITE, C_TRIM)
			f.box(Vector3(0, 1.85, -0.01), Vector3(0.03, 0.08, 0.2))


## Écu en forme de « fer à repasser », armorié, porté au bras gauche.
static func _heater(f: Fig, center: Vector3, width: float, height: float, yaw: float, part: int = P_ARM_L) -> void:
	var pts := PackedVector2Array()
	var hw := width * 0.5
	var hh := height * 0.5
	pts.append(Vector2(-hw, hh))
	pts.append(Vector2(hw, hh))
	for i in range(1, 6):
		var t := float(i) / 6.0
		pts.append(Vector2(hw * cos(t * PI * 0.5) , hh - height * 0.35 - (height * 0.65) * sin(t * PI * 0.5)))
	pts.append(Vector2(0, -hh))
	for i in range(1, 6):
		var t := 1.0 - float(i) / 6.0
		pts.append(Vector2(-hw * cos(t * PI * 0.5), hh - height * 0.35 - (height * 0.65) * sin(t * PI * 0.5)))
	f.set_part(part, f.pivot)
	f.set_style(Color.WHITE, C_ARMS)
	f.plate(pts, center, Basis(Vector3.UP, yaw), 0.03, WOOD_DARK, C_ARMS)


static func _infantry(f: Fig, variant: int) -> void:
	match variant:
		1:  # Piquiers flamands : gambison, chapel de fer, pique de 4,5 m.
			_legs(f, false)
			_torso(f, GAMBESON, C_CLOTH, true, 0.62)
			_arms(f, GAMBESON, C_CLOTH, LEATHER)
			_helmet(f, "kettle")
			f.set_part(P_WEAPON, Vector2(0.9, 0.12))
			f.set_style(WOOD, C_EXACT)
			f.cyl(Vector3(-0.25, 0.05, 0.12), Vector3(-0.25, 4.5, 0.12), 0.022, 0.018, 4, false)
			f.set_style(STEEL, C_METAL)
			f.cyl(Vector3(-0.25, 4.5, 0.12), Vector3(-0.25, 4.75, 0.12), 0.025, 0.0, 4, false)
		2:  # Milice urbaine : cotte de tissu, chapel de fer, lance et écu.
			_legs(f, false)
			_torso(f, HOSE, C_CLOTH, true, 0.6)
			_arms(f, HOSE, C_CLOTH, SKIN)
			_helmet(f, "kettle")
			f.set_part(P_ARM_L, Vector2(1.4, 0.0))
			_heater(f, Vector3(0.33, 1.02, 0.14), 0.44, 0.56, 0.45)
			f.set_part(P_WEAPON, Vector2(0.9, 0.12))
			f.set_style(WOOD, C_EXACT)
			f.cyl(Vector3(-0.25, 0.1, 0.12), Vector3(-0.25, 2.5, 0.12), 0.02, 0.017, 4, false)
			f.set_style(STEEL, C_METAL)
			f.cyl(Vector3(-0.25, 2.5, 0.12), Vector3(-0.25, 2.72, 0.12), 0.03, 0.0, 4, false)
		_:  # Hommes d'armes à pied : maille, bassinet, surcot, épée, écu armorié.
			_legs(f, true)
			_torso(f, MAIL, C_METAL, true)
			_arms(f, MAIL, C_METAL, STEEL)
			_helmet(f, "bassinet")
			f.set_part(P_ARM_L, Vector2(1.4, 0.0))
			_heater(f, Vector3(0.34, 1.05, 0.13), 0.46, 0.6, 0.5)
			f.set_part(P_WEAPON, Vector2(0.9, 0.12))
			f.set_style(LEATHER, C_EXACT)
			f.cyl(Vector3(-0.25, 0.84, 0.12), Vector3(-0.25, 0.98, 0.12), 0.02, 0.02, 4, false)
			f.set_style(Color.WHITE, C_TRIM)
			f.box(Vector3(-0.25, 0.99, 0.12), Vector3(0.2, 0.03, 0.04))
			f.set_style(STEEL, C_METAL)
			f.cyl(Vector3(-0.25, 1.0, 0.12), Vector3(-0.25, 1.8, 0.12), 0.024, 0.006, 4, false, 0.35)


static func _archer(f: Fig, variant: int) -> void:
	_legs(f, false)
	match variant:
		0:  # Archers à l'arc long : jaque de livrée, chaperon, arc de 1,9 m, carquois.
			_torso(f, GAMBESON, C_CLOTH, true, 0.62)
			_arms(f, HOSE, C_CLOTH, SKIN)
			_helmet(f, "hood")
			# Arc tenu dans la main gauche : douves courbes (vers l'avant) et corde.
			var grip := Vector3(0.25, 0.9, 0.14)
			f.set_part(P_BOW, Vector2(grip.y, grip.z))
			f.set_style(WOOD, C_EXACT)
			var prev := grip + Vector3(0, -0.95, 0.0)
			for i in range(1, 9):
				var t := -0.95 + 1.9 * float(i) / 8.0
				var p := grip + Vector3(0, t, 0.16 * (1.0 - pow(t / 0.95, 2.0)))
				f.cyl(prev, p, 0.018, 0.018, 4, false)
				prev = p
			f.set_style(STRING, C_EXACT)
			f.cyl(grip + Vector3(0, -0.95, 0), grip + Vector3(0, 0.95, 0), 0.004, 0.004, 3, false)
			f.set_part(P_BODY)
			f.set_style(LEATHER, C_EXACT)
			f.cyl(Vector3(-0.2, 0.75, -0.12), Vector3(-0.2, 1.2, -0.16), 0.06, 0.07, 6, true)
			f.set_style(Color(0.9, 0.9, 0.85), C_EXACT)
			f.cyl(Vector3(-0.2, 1.2, -0.16), Vector3(-0.2, 1.3, -0.17), 0.05, 0.06, 6, true)
		_:  # Arbalétriers : gambison, chapel de fer, arbalète ; pavois dans le dos (Génois).
			_torso(f, GAMBESON, C_CLOTH, true, 0.62)
			_arms(f, GAMBESON, C_CLOTH, LEATHER)
			_helmet(f, "kettle")
			f.set_part(P_WEAPON, Vector2(0.9, 0.12))
			f.set_style(WOOD, C_EXACT)
			f.box(Vector3(-0.25, 0.95, 0.3), Vector3(0.06, 0.06, 0.7))
			f.set_style(IRON, C_METAL)
			f.box(Vector3(-0.25, 0.95, 0.63), Vector3(0.7, 0.035, 0.035))
			f.set_style(STRING, C_EXACT)
			f.cyl(Vector3(-0.6, 0.95, 0.62), Vector3(-0.25, 0.95, 0.45), 0.004, 0.004, 3, false)
			f.cyl(Vector3(0.1, 0.95, 0.62), Vector3(-0.25, 0.95, 0.45), 0.004, 0.004, 3, false)
			f.set_part(P_BODY)
			f.set_style(LEATHER, C_EXACT)
			f.box(Vector3(0.2, 0.9, -0.05), Vector3(0.08, 0.3, 0.14))
			if variant == 2:
				var pts := PackedVector2Array([Vector2(-0.3, 0.6), Vector2(0.3, 0.6), Vector2(0.3, -0.6), Vector2(-0.3, -0.6)])
				f.set_style(Color.WHITE, C_ARMS)
				f.plate(pts, Vector3(0, 1.05, -0.2), Basis(Vector3.UP, PI).rotated(Vector3.RIGHT, -0.12), 0.04, WOOD_DARK, C_ARMS)


## Cheval (robe `coat`), selle, caparaçon de livrée pour les chevaliers (lot V4b) : corps en
## deux masses (poitrail, croupe) reliées par le ventre, encolure épaisse à la base et effilée,
## tête allongée (ganache, chanfrein, bout du nez), oreilles ; jambes fines à articulations
## (avant-bras, genou, canon, boulet, paturon) ; queue. Le caparaçon tombe à mi-canon.
static func _horse(f: Fig, coat: Color, caparison: bool) -> void:
	f.set_part(P_HORSE)
	f.set_style(coat, C_EXACT)
	# Corps : poitrail, ventre, croupe (ellipsoïdes lisses qui se chevauchent).
	f.ellipsoid(Vector3(0, 1.36, 0.42), Vector3(0.3, 0.35, 0.42), 5, 10)
	f.ellipsoid(Vector3(0, 1.3, -0.05), Vector3(0.29, 0.31, 0.55), 5, 10)
	f.ellipsoid(Vector3(0, 1.36, -0.5), Vector3(0.31, 0.34, 0.4), 5, 10)
	# Encolure : de l'épaule au garrot vers la nuque, puis la tête.
	f.set_part(P_HNECK, Vector2(1.5, 0.6))
	f.cyl(Vector3(0, 1.42, 0.6), Vector3(0, 1.78, 0.86), 0.2, 0.15, 8, false, 0.72)
	f.cyl(Vector3(0, 1.78, 0.86), Vector3(0, 2.02, 0.98), 0.15, 0.1, 8, false, 0.7)
	f.ellipsoid(Vector3(0, 2.02, 1.0), Vector3(0.09, 0.1, 0.11), 3, 7)
	# Tête : ganache large, chanfrein qui s'affine, bout du nez arrondi.
	f.cyl(Vector3(0, 2.0, 1.02), Vector3(0, 1.74, 1.38), 0.09, 0.055, 7, false, 1.25)
	f.ellipsoid(Vector3(0, 1.72, 1.4), Vector3(0.06, 0.06, 0.075), 3, 7)
	f.set_style(HORSE_DARK, C_EXACT)
	for x in [-0.045, 0.045]:
		f.cyl(Vector3(x, 2.08, 0.99), Vector3(x * 1.5, 2.2, 0.96), 0.022, 0.0, 4, false)  # oreilles
	f.cyl(Vector3(0, 2.1, 0.96), Vector3(0, 1.84, 0.84), 0.025, 0.035, 4, false)  # crinière haute
	f.cyl(Vector3(0, 1.84, 0.84), Vector3(0, 1.58, 0.62), 0.035, 0.04, 4, false)
	# Jambes : avant-bras / cuisse, canon fin, boulet, sabot sombre.
	var legs := [[P_HLEG_FL, 0.14, 0.52, true], [P_HLEG_FR, -0.14, 0.52, true], [P_HLEG_BL, 0.15, -0.58, false], [P_HLEG_BR, -0.15, -0.58, false]]
	for leg in legs:
		var x: float = leg[1]
		var z: float = leg[2]
		var front: bool = leg[3]
		f.set_part(int(leg[0]), Vector2(1.2, z))
		f.set_style(coat, C_EXACT)
		if front:
			f.cyl(Vector3(x, 1.2, z), Vector3(x, 0.62, z + 0.02), 0.1, 0.058, 6, false)
			f.cyl(Vector3(x, 0.62, z + 0.02), Vector3(x, 0.16, z), 0.046, 0.04, 5, false)
		else:
			# Jarret : la jambe arrière part vers l'arrière puis revient.
			f.cyl(Vector3(x, 1.22, z), Vector3(x, 0.78, z - 0.14), 0.12, 0.06, 6, false)
			f.cyl(Vector3(x, 0.78, z - 0.14), Vector3(x, 0.16, z - 0.08), 0.048, 0.04, 5, false)
		var hoof_z: float = z + (0.02 if front else -0.08)
		f.ellipsoid(Vector3(x, 0.16, hoof_z), Vector3(0.045, 0.045, 0.045), 2, 5)
		f.cyl(Vector3(x, 0.16, hoof_z), Vector3(x, 0.07, hoof_z + 0.03), 0.038, 0.045, 5, false)
		f.set_style(HORSE_DARK, C_EXACT)
		f.cyl(Vector3(x, 0.07, hoof_z + 0.03), Vector3(x, 0.0, hoof_z + 0.04), 0.05, 0.055, 5, true)
	# Queue.
	f.set_part(P_HORSE)
	f.set_style(HORSE_DARK, C_EXACT)
	f.cyl(Vector3(0, 1.5, -0.84), Vector3(0, 1.2, -0.98), 0.05, 0.06, 5, false)
	f.cyl(Vector3(0, 1.2, -0.98), Vector3(0, 0.72, -0.98), 0.06, 0.03, 5, false)
	if caparison:
		# Housse ajustée : couvre le corps et tombe à mi-jambe, sans élargir la silhouette.
		f.set_style(Color.WHITE, C_LIVERY)
		f.cyl(Vector3(0, 1.66, 0.0), Vector3(0, 0.62, 0.0), 0.33, 0.36, 12, false, 2.7)
		f.set_part(P_HNECK, Vector2(1.5, 0.6))
		f.cyl(Vector3(0, 1.42, 0.62), Vector3(0, 1.8, 0.87), 0.22, 0.165, 8, false, 0.76)
		f.cyl(Vector3(0, 1.8, 0.87), Vector3(0, 2.05, 0.99), 0.165, 0.11, 8, false, 0.72)
		f.set_part(P_HORSE)
	f.set_style(LEATHER, C_EXACT)
	f.box(Vector3(0, 1.68, -0.08), Vector3(0.4, 0.08, 0.52))
	f.box(Vector3(0, 1.75, -0.32), Vector3(0.34, 0.12, 0.06))
	# Bride.
	f.set_part(P_HNECK, Vector2(1.5, 0.6))
	f.set_style(LEATHER, C_EXACT)
	f.cyl(Vector3(0, 1.81, 1.27), Vector3(0, 1.79, 1.3), 0.075, 0.073, 6, false, 1.2)  # muserolle


## Cavalier assis (buste, jambes pendantes le long des flancs).
static func _rider(f: Fig, under: Color, under_code: int, surcoat: bool, helmet: String) -> void:
	var dy := 0.78  # assise sur la selle
	f.set_part(P_RIDER)
	for side in [1.0, -1.0]:
		f.set_style(MAIL if under_code == C_METAL else HOSE, C_METAL if under_code == C_METAL else C_CLOTH)
		f.cyl(Vector3(0.12 * side, 1.73, -0.05), Vector3(0.3 * side, 1.5, 0.15), 0.08, 0.065, 5, false)
		f.cyl(Vector3(0.3 * side, 1.5, 0.15), Vector3(0.32 * side, 1.05, 0.05), 0.06, 0.05, 5, false)
		f.set_style(LEATHER, C_EXACT)
		f.box(Vector3(0.32 * side, 1.02, 0.1), Vector3(0.09, 0.08, 0.22))
	f.set_style(under, under_code)
	f.cyl(Vector3(0, 0.9 + dy, -0.05), Vector3(0, 1.45 + dy, -0.05), 0.17, 0.2, 8, true, 0.72)
	if surcoat:
		f.set_style(Color.WHITE, C_LIVERY)
		f.cyl(Vector3(0, 0.95 + dy, -0.05), Vector3(0, 1.43 + dy, -0.05), 0.185, 0.215, 8, false, 0.74)
		f.cyl(Vector3(0, 0.97 + dy, -0.05), Vector3(0, 0.75 + dy, -0.05), 0.2, 0.32, 8, false, 1.2)
	f.set_style(SKIN, C_EXACT)
	f.ellipsoid(Vector3(0, 1.63 + dy, -0.04), Vector3(0.095, 0.115, 0.105), 4, 7)
	# Casque : même dessin que les fantassins, décalé à l'assise.
	var g := Fig.new()
	g.lod = f.lod
	g.set_part(P_RIDER)
	var helmet_fig := g
	_helmet(helmet_fig, helmet)
	_merge_offset(f, helmet_fig, Vector3(0, dy, -0.05), P_RIDER)
	# Bras (le shader les fait pivoter à l'épaule du cavalier).
	for side in [-1.0, 1.0]:
		f.set_part(P_ARM_R if side < 0.0 else P_ARM_L, Vector2(1.4 + dy, -0.05))
		f.set_style(under, under_code)
		var shoulder := Vector3(0.23 * side, 1.4 + dy, -0.05)
		var hand := Vector3(0.25 * side, 0.95 + dy, 0.18)
		f.cyl(shoulder, hand, 0.06, 0.045, 5, false)
		f.set_style(STEEL if under_code == C_METAL else LEATHER, C_METAL if under_code == C_METAL else C_EXACT)
		f.ellipsoid(hand, Vector3(0.045, 0.05, 0.05), 2, 5)


## Recopie les sommets d'un autre constructeur (déjà rempli) décalés de `offset`.
static func _merge_offset(dest: Fig, src: Fig, offset: Vector3, part: int) -> void:
	var arrays := src.st.commit_to_arrays()
	var verts: PackedVector3Array = arrays[Mesh.ARRAY_VERTEX]
	var normals: PackedVector3Array = arrays[Mesh.ARRAY_NORMAL]
	var colors: PackedColorArray = arrays[Mesh.ARRAY_COLOR]
	var indices: PackedInt32Array = arrays[Mesh.ARRAY_INDEX] if arrays[Mesh.ARRAY_INDEX] != null else PackedInt32Array()
	var order: Array = range(verts.size()) if indices.is_empty() else Array(indices)
	for i in order:
		var c: Color = colors[i]
		dest.st.set_color(c)
		dest.st.set_normal(normals[i])
		dest.st.set_uv(Vector2.ZERO)
		dest.st.set_custom(0, Color(float(part), 0.0, 0.0, 0.0))
		dest.st.add_vertex(verts[i] + offset)


static func _cavalry(f: Fig, variant: int) -> void:
	var dy := 0.78
	match variant:
		1:  # Sergents montés : cheval nu, gambison, chapel de fer, lance.
			_horse(f, HORSE_COATS[2], false)
			_rider(f, GAMBESON, C_CLOTH, true, "kettle")
			_lance(f, dy)
		2:  # Archers montés : jaque, chaperon, arc.
			_horse(f, HORSE_COATS[0], false)
			_rider(f, HOSE, C_CLOTH, true, "hood")
			var grip := Vector3(0.25, 0.95 + dy, 0.18)
			f.set_part(P_BOW, Vector2(grip.y, grip.z))
			f.set_style(WOOD, C_EXACT)
			var prev := grip + Vector3(0, -0.85, 0)
			for i in range(1, 7):
				var t := -0.85 + 1.7 * float(i) / 6.0
				var p := grip + Vector3(0, t, 0.14 * (1.0 - pow(t / 0.85, 2.0)))
				f.cyl(prev, p, 0.018, 0.018, 4, false)
				prev = p
		_:  # Chevaliers : caparaçon, heaume, surcot, lance, écu.
			_horse(f, HORSE_COATS[1], true)
			_rider(f, MAIL, C_METAL, true, "bassinet")
			f.set_part(P_ARM_L, Vector2(1.4 + dy, -0.05))
			_heater(f, Vector3(0.36, 1.12 + dy, 0.1), 0.44, 0.56, 0.35)
			_lance(f, dy)


static func _lance(f: Fig, dy: float) -> void:
	f.set_part(P_WEAPON, Vector2(0.95 + dy, 0.18))
	f.set_style(WOOD, C_EXACT)
	f.cyl(Vector3(-0.25, 0.95 + dy - 0.6, 0.18), Vector3(-0.25, 0.95 + dy + 3.0, 0.18), 0.03, 0.018, 5, false)
	f.set_style(STEEL, C_METAL)
	f.cyl(Vector3(-0.25, 0.95 + dy + 3.0, 0.18), Vector3(-0.25, 0.95 + dy + 3.25, 0.18), 0.022, 0.0, 4, false)
	f.set_style(Color.WHITE, C_LIVERY)
	var pennon := PackedVector2Array([Vector2(0, 0.12), Vector2(0.45, 0.06), Vector2(0.0, -0.1)])
	f.plate(pennon, Vector3(-0.25, 0.95 + dy + 2.8, 0.18), Basis(Vector3.UP, -PI * 0.5), 0.01, Color.WHITE, C_LIVERY)


## Servant d'engin (fantassin simplifié sans arme), posé à `pos`.
static func _crew(f: Fig, pos: Vector3, yaw: float) -> void:
	var g := Fig.new()
	g.lod = f.lod
	_legs(g, false)
	_torso(g, HOSE, C_CLOTH, true, 0.62)
	_arms(g, HOSE, C_CLOTH, SKIN)
	_helmet(g, "hood")
	var arrays := g.st.commit_to_arrays()
	var verts: PackedVector3Array = arrays[Mesh.ARRAY_VERTEX]
	var normals: PackedVector3Array = arrays[Mesh.ARRAY_NORMAL]
	var colors: PackedColorArray = arrays[Mesh.ARRAY_COLOR]
	var customs: PackedFloat32Array = arrays[Mesh.ARRAY_CUSTOM0]
	var indices: PackedInt32Array = arrays[Mesh.ARRAY_INDEX] if arrays[Mesh.ARRAY_INDEX] != null else PackedInt32Array()
	var order: Array = range(verts.size()) if indices.is_empty() else Array(indices)
	var basis := Basis(Vector3.UP, yaw)
	for i in order:
		f.st.set_color(colors[i])
		f.st.set_normal(basis * normals[i])
		f.st.set_uv(Vector2.ZERO)
		# Les servants restent immobiles (membres figés), sauf une légère respiration.
		f.st.set_custom(0, Color(float(P_STATIC), customs[i * 4 + 1], customs[i * 4 + 2], 0.0))
		f.st.add_vertex(pos + basis * verts[i])


## Engins : 0 mangonneau, 1 trébuchet à contrepoids, 2 bombarde sur affût ; deux servants.
static func _engine(f: Fig, variant: int) -> void:
	f.set_part(P_STATIC)
	match variant:
		1:
			f.set_style(WOOD, C_EXACT)
			for x in [-0.9, 0.9]:
				f.box(Vector3(x, 0.2, 0), Vector3(0.3, 0.3, 5.0))
				f.cyl(Vector3(x, 0.3, -1.6), Vector3(x, 4.2, 0), 0.12, 0.1, 5, false)
				f.cyl(Vector3(x, 0.3, 1.6), Vector3(x, 4.2, 0), 0.12, 0.1, 5, false)
			f.box(Vector3(0, 0.2, -2.0), Vector3(2.1, 0.3, 0.3))
			f.box(Vector3(0, 0.2, 2.0), Vector3(2.1, 0.3, 0.3))
			f.set_style(IRON, C_METAL)
			f.cyl(Vector3(-1.0, 4.2, 0), Vector3(1.0, 4.2, 0), 0.08, 0.08, 6, true)
			f.set_part(P_ENGINE_ARM, Vector2(4.2, 0.0))
			f.set_style(WOOD_DARK, C_EXACT)
			f.cyl(Vector3(0, 4.2 - 1.4, 1.2), Vector3(0, 4.2 + 3.6, -3.0), 0.12, 0.06, 5, false)
			f.set_style(WOOD, C_EXACT)
			f.box(Vector3(0, 2.4, 1.5), Vector3(1.1, 1.1, 1.1))
		2:
			f.set_style(WOOD, C_EXACT)
			f.box(Vector3(0, 0.35, -0.3), Vector3(0.9, 0.4, 2.8))
			f.box(Vector3(0, 0.15, -1.8), Vector3(1.2, 0.3, 0.4))
			f.set_style(IRON, C_METAL)
			f.cyl(Vector3(0, 0.85, -1.3), Vector3(0, 0.85, 1.2), 0.32, 0.3, 10, true)
			for z in [-1.0, -0.4, 0.2, 0.8]:
				f.cyl(Vector3(0, 0.85, z), Vector3(0, 0.85, z + 0.1), 0.36, 0.36, 10, false)
			f.set_style(Color(0.05, 0.05, 0.05), C_EXACT)
			f.cyl(Vector3(0, 0.85, 1.21), Vector3(0, 0.85, 1.22), 0.2, 0.2, 10, true)
		_:
			f.set_style(WOOD, C_EXACT)
			for x in [-0.8, 0.8]:
				f.box(Vector3(x, 0.25, 0), Vector3(0.22, 0.25, 3.2))
				f.cyl(Vector3(x, 0.35, 0.5), Vector3(x, 1.9, 0.1), 0.1, 0.08, 5, false)
			f.box(Vector3(0, 0.25, -1.4), Vector3(1.8, 0.2, 0.22))
			f.box(Vector3(0, 0.25, 1.4), Vector3(1.8, 0.2, 0.22))
			f.box(Vector3(0, 1.9, 0.1), Vector3(1.9, 0.2, 0.2))
			f.set_style(LEATHER, C_EXACT)
			f.cyl(Vector3(-0.7, 0.8, -0.6), Vector3(0.7, 0.8, -0.6), 0.2, 0.2, 8, true)
			f.set_part(P_ENGINE_ARM, Vector2(0.8, -0.6))
			f.set_style(WOOD_DARK, C_EXACT)
			f.cyl(Vector3(0, 0.8, -0.6), Vector3(0, 2.5, 1.0), 0.09, 0.07, 5, false)
			f.set_style(LEATHER, C_EXACT)
			f.ellipsoid(Vector3(0, 2.55, 1.05), Vector3(0.22, 0.12, 0.22), 2, 6, 0.0)
	_crew(f, Vector3(-1.5, 0, -0.8), 0.4)
	_crew(f, Vector3(1.4, 0, -1.4), -0.3)


# --- Décor ---------------------------------------------------------------------------------


const BARK_TEXTURE := preload("res://assets/textures/battle/bark_brown_02_diff.jpg")
const BARK_NORMAL := preload("res://assets/textures/battle/bark_brown_02_nor.jpg")
const LEAVES_TEXTURE := preload("res://assets/textures/battle/foliage_leaves.png")
const FOLIAGE_SHADER := preload("res://shaders/battle_foliage.gdshader")


## Arbre de `kind` : oak (chêne étalé), poplar (peuplier élancé), bush (buisson), far (arbre
## lointain simplifié) ; suffixe `_lod` = même silhouette allégée (moins de cartes, plus
## grandes, sans branches) pour la distance. Surface 0 = écorce, surface 1 = houppier.
static func tree(kind: String = "oak") -> ArrayMesh:
	var key := "tree/" + kind
	if _cache.has(key):
		return _cache[key]
	var lod := kind.ends_with("_lod")
	kind = kind.trim_suffix("_lod")
	var rng := RandomNumberGenerator.new()
	rng.seed = hash(kind)
	var bark := SurfaceTool.new()
	bark.begin(Mesh.PRIMITIVE_TRIANGLES)
	var leaves := SurfaceTool.new()
	leaves.begin(Mesh.PRIMITIVE_TRIANGLES)
	var crown_center := Vector3(0, 7.5, 0)
	var crown_radii := Vector3(4.2, 3.4, 4.2)
	var clusters := 16
	var card := 3.6
	var trunk_h := 5.5
	var trunk_r := 0.32
	match kind:
		"poplar":
			crown_center = Vector3(0, 9.5, 0)
			crown_radii = Vector3(1.9, 6.0, 1.9)
			clusters = 16
			card = 2.8
			trunk_h = 6.0
			trunk_r = 0.25
		"bush":
			crown_center = Vector3(0, 1.0, 0)
			crown_radii = Vector3(1.4, 0.9, 1.4)
			clusters = 6
			card = 1.9
			trunk_h = 0.0
		"far":
			clusters = 5
			card = 5.5
			crown_radii = Vector3(3.2, 2.6, 3.2)
	if lod:
		clusters = maxi(clusters / 3, 4)
		card *= 1.55
	if trunk_h > 0.0:
		_bark_cyl(bark, Vector3(0, -0.3, 0), Vector3(0, trunk_h, 0), trunk_r, trunk_r * 0.6, 4 if lod else 7)
		if kind != "far" and not lod:
			for i in 4:
				var a := TAU * float(i) / 4.0 + rng.randf() * 0.8
				var start := Vector3(0, trunk_h * rng.randf_range(0.55, 0.85), 0)
				var end := start + Vector3(cos(a) * crown_radii.x * 0.6, crown_radii.y * 0.7, sin(a) * crown_radii.z * 0.6)
				_bark_cyl(bark, start, end, trunk_r * 0.45, trunk_r * 0.15, 5)
	for i in clusters:
		var dir := Vector3(rng.randf_range(-1, 1), rng.randf_range(-0.7, 1), rng.randf_range(-1, 1)).normalized()
		var center := crown_center + dir * crown_radii * rng.randf_range(0.35, 0.8)
		var size := card * rng.randf_range(0.8, 1.2)
		for k in 2:
			var yaw := rng.randf() * PI + float(k) * PI * 0.5
			var pitch := rng.randf_range(-0.5, 0.5)
			var basis := Basis(Vector3.UP, yaw).rotated(Vector3.RIGHT, pitch)
			_leaf_card(leaves, center, basis, size, crown_center, crown_radii)
		# Une carte presque horizontale pour la vue plongeante.
		_leaf_card(leaves, center + Vector3(0, 0.3, 0), Basis(Vector3.RIGHT, -PI * 0.5 + rng.randf_range(-0.3, 0.3)).rotated(Vector3.UP, rng.randf() * TAU), size, crown_center, crown_radii)
	var mesh := ArrayMesh.new()
	var bark_mat := StandardMaterial3D.new()
	bark_mat.albedo_texture = BARK_TEXTURE
	bark_mat.normal_enabled = true
	bark_mat.normal_texture = BARK_NORMAL
	bark_mat.roughness = 0.95
	bark_mat.albedo_color = Color(0.8, 0.78, 0.72)
	bark_mat.uv1_scale = Vector3(1.0, 1.0, 1.0)
	if trunk_h > 0.0:
		bark.generate_tangents()
		bark.commit(mesh)
		mesh.surface_set_material(mesh.get_surface_count() - 1, bark_mat)
	leaves.commit(mesh)
	var leaf_mat := ShaderMaterial.new()
	leaf_mat.shader = FOLIAGE_SHADER
	leaf_mat.set_shader_parameter("leaves", LEAVES_TEXTURE)
	leaf_mat.set_shader_parameter("wind_strength", 0.3 if kind == "far" else 1.0)
	leaf_mat.set_shader_parameter("sway_height", 2.5 if kind == "bush" else 12.0)
	mesh.surface_set_material(mesh.get_surface_count() - 1, leaf_mat)
	_cache[key] = mesh
	return mesh


static func _bark_cyl(st: SurfaceTool, a: Vector3, b: Vector3, ra: float, rb: float, sides: int) -> void:
	var axis := (b - a).normalized()
	var ref := Vector3.FORWARD if absf(axis.dot(Vector3.FORWARD)) < 0.9 else Vector3.RIGHT
	var u := axis.cross(ref).normalized()
	var w := axis.cross(u).normalized()
	var length := a.distance_to(b)
	for i in sides:
		var t0 := TAU * float(i) / float(sides)
		var t1 := TAU * float(i + 1) / float(sides)
		var n0 := u * cos(t0) + w * sin(t0)
		var n1 := u * cos(t1) + w * sin(t1)
		var p := [a + n0 * ra, b + n0 * rb, b + n1 * rb, a + n1 * ra]
		var uv := [Vector2(float(i) / sides, length / 2.0), Vector2(float(i) / sides, 0.0), Vector2(float(i + 1) / sides, 0.0), Vector2(float(i + 1) / sides, length / 2.0)]
		var n := [n0, n0, n1, n1]
		for idx in [0, 2, 1, 0, 3, 2]:
			st.set_normal(n[idx])
			st.set_uv(uv[idx])
			st.add_vertex(p[idx])


## Carte de feuillage : quad centré, normales « sphériques » (depuis le centre du houppier)
## pour un éclairage de volume doux.
static func _leaf_card(st: SurfaceTool, center: Vector3, basis: Basis, size: float, crown_center: Vector3, radii: Vector3) -> void:
	var h := size * 0.5
	var corners := [Vector3(-h, -h, 0), Vector3(h, -h, 0), Vector3(h, h, 0), Vector3(-h, h, 0)]
	var uvs := [Vector2(0, 1), Vector2(1, 1), Vector2(1, 0), Vector2(0, 0)]
	for idx in [0, 1, 2, 0, 2, 3]:
		var p: Vector3 = center + basis * corners[idx]
		var n := ((p - crown_center) / radii).normalized()
		n = (n + Vector3(0, 0.35, 0)).normalized()
		st.set_normal(n)
		st.set_uv(uvs[idx])
		st.set_color(Color.WHITE)
		st.add_vertex(p)


## Rocher : icosaèdre subdivisé bosselé, texture triplanaire de roche.
static func rock() -> ArrayMesh:
	if _cache.has("rock"):
		return _cache["rock"]
	var sphere := SphereMesh.new()
	sphere.radius = 1.0
	sphere.height = 2.0
	sphere.radial_segments = 10
	sphere.rings = 6
	var arrays := sphere.get_mesh_arrays()
	var verts: PackedVector3Array = arrays[Mesh.ARRAY_VERTEX]
	var noise := FastNoiseLite.new()
	noise.seed = 3
	noise.frequency = 0.9
	for i in verts.size():
		var v := verts[i]
		verts[i] = v * (1.0 + noise.get_noise_3dv(v * 1.3) * 0.35)
	arrays[Mesh.ARRAY_VERTEX] = verts
	arrays[Mesh.ARRAY_NORMAL] = null
	arrays[Mesh.ARRAY_TANGENT] = null
	var st := SurfaceTool.new()
	st.create_from_arrays(arrays)
	st.generate_normals()
	var mesh := st.commit()
	var mat := StandardMaterial3D.new()
	mat.albedo_texture = load("res://assets/textures/battle/castle_wall_varriation_diff.jpg")
	mat.albedo_color = Color(0.72, 0.7, 0.66)
	mat.uv1_triplanar = true
	mat.uv1_scale = Vector3(0.35, 0.35, 0.35)
	mat.roughness = 0.95
	mesh.surface_set_material(0, mat)
	_cache["rock"] = mesh
	return mesh


## Hampe de bannière (bois), 1 m de haut : mise à l'échelle par le nœud.
static func pole() -> ArrayMesh:
	var st := SurfaceTool.new()
	st.begin(Mesh.PRIMITIVE_TRIANGLES)
	add_box(st, Vector3(0, 0.5, 0), Vector3(0.04, 1.0, 0.04), WOOD)
	add_box(st, Vector3(0, 1.0, 0), Vector3(0.09, 0.05, 0.09), Color(0.8, 0.66, 0.2))
	add_box(st, Vector3(0.18, 0.96, 0), Vector3(0.4, 0.015, 0.015), WOOD)
	var mesh := st.commit()
	var mat := StandardMaterial3D.new()
	mat.vertex_color_use_as_albedo = true
	mat.vertex_color_is_srgb = true
	mesh.surface_set_material(0, mat)
	return mesh


## Drapeau subdivisé (animé par `battle_banner.gdshader`) : `length` le long de +X depuis la
## hampe, `height` en Y, bord supérieur à y = 0.
static func flag(length: float, height: float) -> ArrayMesh:
	var st := SurfaceTool.new()
	st.begin(Mesh.PRIMITIVE_TRIANGLES)
	var nx := 12
	var ny := 5
	for j in ny:
		for i in nx:
			var quad := [Vector2(i, j), Vector2(i + 1, j), Vector2(i + 1, j + 1), Vector2(i, j + 1)]
			for idx in [0, 1, 2, 0, 2, 3]:
				var q: Vector2 = quad[idx]
				var uv := Vector2(q.x / nx, q.y / ny)
				st.set_normal(Vector3.BACK)
				st.set_uv(uv)
				st.add_vertex(Vector3(uv.x * length, -uv.y * height, 0.0))
	return st.commit()


## Ajoute un pavé centré en `center`, de taille `size`, tourné de `pitch` autour de X
## (couleurs de sommets ; utilisé par les échelles de siège et la hampe).
static func add_box(st: SurfaceTool, center: Vector3, size: Vector3, color: Color, pitch: float = 0.0) -> void:
	var h := size * 0.5
	var basis := Basis(Vector3.RIGHT, pitch)
	var corners: Array[Vector3] = []
	for i in 8:
		var local := Vector3(h.x if i & 1 else -h.x, h.y if i & 2 else -h.y, h.z if i & 4 else -h.z)
		corners.append(center + basis * local)
	var faces := [
		[[0, 2, 3, 1], Vector3.BACK * -1.0], [[4, 5, 7, 6], Vector3.BACK],
		[[0, 4, 6, 2], Vector3.LEFT], [[1, 3, 7, 5], Vector3.RIGHT],
		[[0, 1, 5, 4], Vector3.DOWN], [[2, 6, 7, 3], Vector3.UP],
	]
	for face in faces:
		var idx: Array = face[0]
		var normal: Vector3 = basis * (face[1] as Vector3)
		tri(st, corners[idx[0]], corners[idx[1]], corners[idx[2]], normal, color)
		tri(st, corners[idx[0]], corners[idx[2]], corners[idx[3]], normal, color)


## Ajoute un triangle en ordre horaire vu du côté de `normal` (face avant pour Godot).
static func tri(st: SurfaceTool, p0: Vector3, p1: Vector3, p2: Vector3, normal: Vector3, color: Color) -> void:
	if (p1 - p0).cross(p2 - p0).dot(normal) > 0.0:
		var swap := p1
		p1 = p2
		p2 = swap
	for v in [p0, p1, p2]:
		st.set_color(color)
		st.set_normal(normal)
		st.add_vertex(v)


## Contour rectangulaire `width` × `depth` (épaisseur `t` en mètres) posé à plat, centré,
## pour l'anneau de sélection d'un régiment.
static func outline(width: float, depth: float, t: float) -> ArrayMesh:
	var st := SurfaceTool.new()
	st.begin(Mesh.PRIMITIVE_TRIANGLES)
	var hw := width * 0.5
	var hd := depth * 0.5
	var quads := [
		Rect2(-hw, -hd, width, t), Rect2(-hw, hd - t, width, t),
		Rect2(-hw, -hd, t, depth), Rect2(hw - t, -hd, t, depth),
	]
	for q in quads:
		var r: Rect2 = q
		var a := Vector3(r.position.x, 0, r.position.y)
		var b := Vector3(r.end.x, 0, r.position.y)
		var c := Vector3(r.end.x, 0, r.end.y)
		var d := Vector3(r.position.x, 0, r.end.y)
		for v in [a, b, c, a, c, d]:
			st.set_normal(Vector3.UP)
			st.add_vertex(v)
	return st.commit()
