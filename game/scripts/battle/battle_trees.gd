class_name BattleTrees
extends Node3D

## Arbres des batailles (lot DA6, bible DA § 6) : feuillus ramifiés procéduraux par essence, en
## remplacement des houppiers « sucette » (boule de cartes sur un bâton) de `BattleMeshes.tree`.
## - Squelette : tronc, charpentières, branches, rameaux (récursif, graine par essence), tenu dans
##   l'enveloppe du houppier de l'essence ; les rameaux terminaux portent des bouquets de deux
##   cartes (`leaf_spray_<essence>.png`, vraies feuilles photographiées, lot FA1 ; à défaut le
##   rameau dessiné `leaf_spray.png`) : une dans l'axe du rameau, une tournée vers l'extérieur. Normales
##   arrondies (houppier + bouquet), ombre propre cuite dans la couleur de sommet (cœur et bas du
##   houppier plus sombres). Hiver : ramilles nues (`twig_spray.png`), chêne marcescent
##   (`dead_leaves_oak.png`).
## - Trois niveaux par arbre, choisis par instance dans les shaders (`battle_tree_lod.gdshaderinc`,
##   distance à la caméra principale, fondu tramé) : maillage complet jusqu'à `LOD1_DISTANCE`,
##   maillage allégé (même squelette, moitié des bouquets agrandis, rameaux sans écorce) jusqu'à
##   `IMPOSTOR_DISTANCE` (300 m, bible : aucun arbre « sucette » en deçà), imposteur au-delà.
## - Imposteurs : atlas cuit au lancement depuis les maillages complets (une ligne par essence,
##   `COLS` angles de vue), comme les figurines (ADR 0024) ; un quadrilatère par arbre.
## Rendu seulement.

const BARK_TEXTURE := preload("res://assets/textures/battle/bark_brown_02_diff.jpg")
const BARK_NORMAL := preload("res://assets/textures/battle/bark_brown_02_nor.jpg")
const LEAF_TEXTURE := preload("res://assets/textures/battle/leaf_spray.png")
const TWIG_TEXTURE := preload("res://assets/textures/battle/twig_spray.png")
const DEAD_TEXTURE := preload("res://assets/textures/battle/dead_leaves_oak.png")
## Rameau de l'essence (`data/art/battle_tree_leaves.json`, `build_fa_leaf_sprays.py`).
const SPECIES_LEAF_PATH := "res://assets/textures/battle/leaf_spray_%s.png"
const FOLIAGE_SHADER := preload("res://shaders/battle_tree_foliage.gdshader")
const BARK_SHADER := preload("res://shaders/battle_tree_bark.gdshader")
const IMPOSTOR_SHADER := preload("res://shaders/battle_tree_impostor.gdshader")

const LOD1_DISTANCE := 120.0
const IMPOSTOR_DISTANCE := 300.0
const LOD_BAND := 16.0
## Buissons et haies : maillage complet de près seulement.
const BUSH_LOD1_DISTANCE := 60.0
## Essences qui ont un imposteur (ligne de l'atlas = rang dans cette liste).
const IMPOSTOR_SPECIES := ["oak", "beech", "ash", "poplar", "willow", "fruit"]
const COLS := 4
const CELL_PX := 128
const CELL_WORLD := 30.0  # côté (m) d'une cellule dans la scène de cuisson
const FOOT := 0.04

## Paramètres des essences. height/width/crown_base : enveloppe du houppier (m) ; trunk : hauteur du
## fût ; radius : rayon du tronc ; limbs : charpentières ; spread : angle (rad) des branches filles ;
## leader : flèche qui prolonge le tronc ; levels : profondeur du squelette ; card : taille des
## bouquets (m) ; gnarl : tortuosité ; rise : redressement vers le ciel ; leafy : bouquets par
## rameau ; bark / smooth : teinte et lissé de l'écorce ; leaf : teinte du feuillage ; marcescent :
## garde ses feuilles sèches l'hiver (chêne).
const SPECIES := {
	"oak": {"height": 15.0, "width": 14.5, "crown_base": 3.6, "trunk": 3.4, "radius": 0.46, "limbs": 5, "spread": 0.95, "leader": false, "levels": 3, "card": 3.3, "gnarl": 0.4, "rise": 0.12, "leafy": 3, "bark": Color(0.74, 0.7, 0.64), "smooth": 0.0, "leaf": Color(0.9, 0.96, 0.82), "marcescent": true},
	"beech": {"height": 19.0, "width": 12.5, "crown_base": 4.8, "trunk": 5.2, "radius": 0.38, "limbs": 4, "spread": 0.72, "leader": true, "levels": 3, "card": 3.1, "gnarl": 0.14, "rise": 0.25, "leafy": 3, "bark": Color(0.92, 0.92, 0.9), "smooth": 1.0, "leaf": Color(1.0, 1.0, 0.8), "marcescent": false},
	"ash": {"height": 18.0, "width": 10.5, "crown_base": 6.0, "trunk": 6.2, "radius": 0.32, "limbs": 4, "spread": 0.62, "leader": true, "levels": 3, "card": 2.6, "gnarl": 0.2, "rise": 0.35, "leafy": 2, "bark": Color(0.9, 0.88, 0.84), "smooth": 0.5, "leaf": Color(1.0, 1.06, 0.88), "marcescent": false},
	"poplar": {"height": 22.0, "width": 9.0, "crown_base": 5.0, "trunk": 6.0, "radius": 0.42, "limbs": 6, "spread": 0.45, "leader": true, "levels": 3, "card": 2.8, "gnarl": 0.3, "rise": 0.5, "leafy": 3, "bark": Color(0.7, 0.68, 0.64), "smooth": 0.2, "leaf": Color(0.94, 1.0, 0.82), "marcescent": false},
	"willow": {"height": 8.5, "width": 7.5, "crown_base": 2.6, "trunk": 2.5, "radius": 0.52, "limbs": 12, "spread": 0.5, "leader": false, "levels": 1, "card": 2.1, "gnarl": 0.06, "rise": 0.55, "leafy": 4, "bark": Color(0.68, 0.66, 0.62), "smooth": 0.0, "leaf": Color(0.95, 1.0, 0.96), "marcescent": false},
	"fruit": {"height": 5.6, "width": 6.2, "crown_base": 1.8, "trunk": 1.6, "radius": 0.17, "limbs": 4, "spread": 1.0, "leader": false, "levels": 2, "card": 2.0, "gnarl": 0.45, "rise": 0.1, "leafy": 3, "bark": Color(0.7, 0.66, 0.6), "smooth": 0.0, "leaf": Color(0.95, 1.0, 0.84), "marcescent": false},
	"bush": {"height": 2.4, "width": 3.0, "crown_base": 0.2, "trunk": 0.0, "radius": 0.06, "limbs": 6, "spread": 0.55, "leader": false, "levels": 1, "card": 1.8, "gnarl": 0.3, "rise": 0.3, "leafy": 3, "bark": Color(0.7, 0.66, 0.6), "smooth": 0.0, "leaf": Color(0.9, 0.96, 0.82), "marcescent": false},
}

static var _cache: Dictionary = {}

var impostor_material: ShaderMaterial
var baked := false
var _impostor_nodes: Array[MultiMeshInstance3D] = []


## Maillage de `species` au niveau `lod` (0 complet, 1 allégé), feuillé ou d'hiver. Surface 0 =
## écorce, dernière surface = feuillage (buisson allégé : feuillage seul). `lod_near`/`lod_far` : plage de distances (m) de ce niveau.
static func mesh(species: String, lod: int, winter: bool, lod_near: float, lod_far: float) -> ArrayMesh:
	var key := "%s/%d/%s/%.0f/%.0f" % [species, lod, winter, lod_near, lod_far]
	if _cache.has(key):
		return _cache[key]
	var built := _build(species, lod, winter)
	var mesh := ArrayMesh.new()
	var bark: SurfaceTool = built["bark"]
	var leaves: SurfaceTool = built["leaves"]
	if int(built["bark_quads"]) > 0:
		bark.generate_tangents()
		bark.commit(mesh)
		mesh.surface_set_material(0, bark_material(species, lod_near, lod_far))
	leaves.commit(mesh)
	mesh.surface_set_material(mesh.get_surface_count() - 1, foliage_material(species, winter, lod_near, lod_far))
	_cache[key] = mesh
	return mesh


static func bark_material(species: String, lod_near: float, lod_far: float) -> ShaderMaterial:
	var p: Dictionary = SPECIES[species]
	var mat := ShaderMaterial.new()
	mat.shader = BARK_SHADER
	mat.set_shader_parameter("bark", BARK_TEXTURE)
	mat.set_shader_parameter("bark_normal", BARK_NORMAL)
	mat.set_shader_parameter("tint", p["bark"])
	mat.set_shader_parameter("smooth_bark", float(p["smooth"]))
	mat.set_shader_parameter("lod_near", lod_near)
	mat.set_shader_parameter("lod_far", lod_far)
	mat.set_shader_parameter("lod_band", LOD_BAND)
	return mat


static func foliage_material(species: String, winter: bool, lod_near: float, lod_far: float) -> ShaderMaterial:
	var p: Dictionary = SPECIES[species]
	var mat := ShaderMaterial.new()
	mat.shader = FOLIAGE_SHADER
	var texture: Texture2D = leaf_texture(species)
	var tint: Color = p["leaf"]
	if winter:
		texture = DEAD_TEXTURE if bool(p["marcescent"]) else TWIG_TEXTURE
		tint = Color(1, 1, 1)
		mat.set_shader_parameter("backlight", Color(0.08, 0.07, 0.05))
	mat.set_shader_parameter("leaves", texture)
	mat.set_shader_parameter("tint", tint)
	mat.set_shader_parameter("wind_strength", 0.5 if winter else 1.0)
	mat.set_shader_parameter("sway_height", maxf(float(p["height"]) * 0.8, 2.0))
	mat.set_shader_parameter("lod_near", lod_near)
	mat.set_shader_parameter("lod_far", lod_far)
	mat.set_shader_parameter("lod_band", LOD_BAND)
	return mat


## Rameau feuillu de `species` : sa carte de vraies feuilles, sinon le rameau dessiné commun.
static func leaf_texture(species: String) -> Texture2D:
	var path := SPECIES_LEAF_PATH % species
	if ResourceLoader.exists(path):
		return load(path)
	return LEAF_TEXTURE


## Squelette et bouquets (même tirage aléatoire quel que soit le niveau de détail : le maillage
## allégé garde la silhouette du complet).
static func _build(species: String, lod: int, winter: bool) -> Dictionary:
	var p: Dictionary = SPECIES[species]
	var rng := RandomNumberGenerator.new()
	rng.seed = hash(species) & 0x7fffffff
	var bark := SurfaceTool.new()
	bark.begin(Mesh.PRIMITIVE_TRIANGLES)
	var leaves := SurfaceTool.new()
	leaves.begin(Mesh.PRIMITIVE_TRIANGLES)
	var height := float(p["height"])
	var base := float(p["crown_base"])
	var ctx := {
		"p": p, "rng": rng, "lod": lod, "winter": winter, "bark": bark, "leaves": leaves,
		"center": Vector3(0, (base + height) * 0.5, 0),
		"radii": Vector3(float(p["width"]) * 0.5, (height - base) * 0.5, float(p["width"]) * 0.5),
		"clusters": 0,
		"bark_quads": 0,
	}
	var trunk := float(p["trunk"])
	var radius := float(p["radius"])
	var limbs := int(p["limbs"])
	var reach := maxf(ctx["radii"].x, ctx["radii"].y)
	if trunk > 0.0:
		var lean := Vector3(rng.randf_range(-0.07, 0.07), 1.0, rng.randf_range(-0.07, 0.07)).normalized()
		var top := Vector3(0, -0.4, 0) + lean * (trunk + 0.4)
		_cylinder(ctx, Vector3(0, -0.4, 0), top, radius * 1.15, radius * 0.8, 0)
		if species == "willow":
			# Saule têtard : tête renflée d'où partent les rejets droits.
			_cylinder(ctx, top - Vector3(0, 0.2, 0), top + Vector3(0, 0.35, 0), radius * 1.25, radius * 0.9, 0)
		for i in limbs:
			var az := TAU * float(i) / float(limbs) + rng.randf_range(-0.4, 0.4)
			var out := Vector3(cos(az), 0.0, sin(az))
			var tilt := float(p["spread"]) * rng.randf_range(0.75, 1.15)
			var dir := (Vector3.UP * cos(tilt) + out * sin(tilt)).normalized()
			var start := top.lerp(Vector3(0, -0.4, 0), rng.randf_range(0.0, 0.3) if i > 1 else 0.0)
			_branch(ctx, start, dir, reach * rng.randf_range(0.75, 1.0), radius * 0.62, 1)
		if bool(p["leader"]):
			_branch(ctx, top, lean, (height - trunk) * 0.8, radius * 0.7, 1)
	else:
		# Buisson : cépées depuis le sol.
		for i in limbs:
			var az := TAU * float(i) / float(limbs) + rng.randf_range(-0.5, 0.5)
			var tilt := float(p["spread"]) * rng.randf_range(0.5, 1.2)
			var dir := (Vector3.UP * cos(tilt) + Vector3(cos(az), 0.0, sin(az)) * sin(tilt)).normalized()
			_branch(ctx, Vector3(rng.randf_range(-0.2, 0.2), -0.1, rng.randf_range(-0.2, 0.2)), dir, reach * 1.2, radius, 1)
	return {"bark": bark, "leaves": leaves, "clusters": ctx["clusters"], "bark_quads": ctx["bark_quads"]}


static func _branch(ctx: Dictionary, start: Vector3, dir: Vector3, length: float, radius: float, level: int) -> void:
	var p: Dictionary = ctx["p"]
	var rng: RandomNumberGenerator = ctx["rng"]
	var center: Vector3 = ctx["center"]
	var radii: Vector3 = ctx["radii"]
	var levels := int(p["levels"])
	var segs := 3 if level <= 1 else 2
	var gnarl := float(p["gnarl"])
	var rise := float(p["rise"])
	var points: Array[Vector3] = [start]
	var dirs: Array[Vector3] = []
	var pos := start
	for s in segs:
		var jitter := Vector3(rng.randf_range(-1, 1), rng.randf_range(-1, 1), rng.randf_range(-1, 1)) * gnarl * 0.5
		dir = (dir + jitter + Vector3.UP * rise * 0.3).normalized()
		var next := pos + dir * length / float(segs)
		# Enveloppe du houppier : le rameau qui sort est ramené dedans (bord un peu bosselé).
		var rel := (next - center) / radii
		var limit := 1.0 + rng.randf_range(-0.12, 0.05)
		if rel.length() > limit:
			next = center + rel.normalized() * limit * radii
			if (next - pos).length() > 0.05:
				dir = (next - pos).normalized()
		var r0 := radius * (1.0 - 0.35 * float(s) / float(segs))
		var r1 := radius * (1.0 - 0.35 * float(s + 1) / float(segs))
		var drawn_level := 2 if int(ctx["lod"]) == 0 else 1
		if level <= drawn_level:
			_cylinder(ctx, pos, next, r0, r1, level)
		pos = next
		points.append(pos)
		dirs.append(dir)
	if level < levels:
		for c in 3:
			var idx := clampi(segs - c, 1, segs)
			var at: Vector3 = points[idx]
			var along: Vector3 = dirs[idx - 1]
			var side := along.cross(Vector3.UP if absf(along.y) < 0.95 else Vector3.RIGHT).normalized()
			side = side.rotated(along, rng.randf() * TAU)
			var tilt := float(p["spread"]) * rng.randf_range(0.55, 1.0)
			var child_dir := (along * cos(tilt) + side * sin(tilt)).normalized()
			_branch(ctx, at, child_dir, length * rng.randf_range(0.55, 0.72), radius * 0.55, level + 1)
	else:
		# Rameau terminal : bouquets au bout et le long.
		var leafy := int(p["leafy"])
		for k in maxi(leafy, 1):
			var t := 1.0 - float(k) / float(maxi(leafy, 1)) * 0.6
			var idx := clampi(int(round(t * float(segs))), 1, segs)
			var at: Vector3 = points[idx - 1].lerp(points[idx], rng.randf_range(0.4, 1.0))
			_cluster(ctx, at, dirs[idx - 1])


## Bouquet : carte dans l'axe du rameau (pied vers la branche) + carte tournée vers l'extérieur.
static func _cluster(ctx: Dictionary, at: Vector3, along: Vector3) -> void:
	var p: Dictionary = ctx["p"]
	var rng: RandomNumberGenerator = ctx["rng"]
	var center: Vector3 = ctx["center"]
	var radii: Vector3 = ctx["radii"]
	var index := int(ctx["clusters"])
	ctx["clusters"] = index + 1
	var size := float(p["card"]) * rng.randf_range(0.8, 1.2)
	var outward := ((at - center) / radii)
	if outward.length() < 0.05:
		outward = Vector3.UP
	var o := (outward.normalized() * 0.6 + along * 0.4).normalized()
	var spin := rng.randf() * TAU
	var shade := rng.randf_range(0.88, 1.08)
	var hue := Vector3(rng.randf_range(0.96, 1.04), 1.0, rng.randf_range(0.92, 1.03))
	if int(ctx["lod"]) == 1:
		# Allégé : un bouquet sur deux, agrandi (même masse lue au loin).
		if index % 2 == 1:
			return
		size *= 1.45
	var side := o.cross(Vector3.UP if absf(o.y) < 0.95 else Vector3.RIGHT).normalized().rotated(o, spin)
	var st: SurfaceTool = ctx["leaves"]
	# Carte 1 : plan contenant `o`, pied au rameau.
	var up := o
	var corners := [at - side * size * 0.5 - up * size * 0.15, at + side * size * 0.5 - up * size * 0.15, at + side * size * 0.5 + up * size * 0.85, at - side * size * 0.5 + up * size * 0.85]
	_card(ctx, st, corners, at, shade, hue)
	# Carte 2 : face à l'extérieur (lue de dessus comme de côté).
	var a := side
	var b := o.cross(a).normalized()
	var mid := at + o * size * 0.25
	corners = [mid - a * size * 0.5 - b * size * 0.5, mid + a * size * 0.5 - b * size * 0.5, mid + a * size * 0.5 + b * size * 0.5, mid - a * size * 0.5 + b * size * 0.5]
	_card(ctx, st, corners, at, shade, hue)


static func _card(ctx: Dictionary, st: SurfaceTool, corners: Array, cluster: Vector3, shade: float, hue: Vector3) -> void:
	var p: Dictionary = ctx["p"]
	var center: Vector3 = ctx["center"]
	var radii: Vector3 = ctx["radii"]
	var height := float(p["height"])
	var base := float(p["crown_base"])
	var uvs := [Vector2(0, 1), Vector2(1, 1), Vector2(1, 0), Vector2(0, 0)]
	for idx in [0, 1, 2, 0, 2, 3]:
		var v: Vector3 = corners[idx]
		var rel := (v - center) / radii
		var n := (rel.normalized() + (v - cluster).normalized() * 0.4 + Vector3.UP * 0.25).normalized()
		# Ombre propre : cœur et bas du houppier plus sombres.
		var depth := clampf(rel.length(), 0.0, 1.0)
		var low := clampf((v.y - base) / maxf(height - base, 0.5), 0.0, 1.0)
		var k := lerpf(0.5, 1.0, pow(depth, 1.3)) * lerpf(0.78, 1.0, low) * shade
		st.set_normal(n)
		st.set_uv(uvs[idx])
		st.set_color(Color(k * hue.x, k * hue.y, k * hue.z))
		st.add_vertex(v)


static func _cylinder(ctx: Dictionary, a: Vector3, b: Vector3, ra: float, rb: float, level: int) -> void:
	var st: SurfaceTool = ctx["bark"]
	var sides: int = [8, 6, 4][mini(level, 2)]
	if int(ctx["lod"]) == 1:
		sides = maxi(sides - 3, 3)
	if float(ctx["p"]["trunk"]) <= 0.0:
		# Buissons (haies : des milliers) : tiges à 3 pans au maillage complet, aucune au loin.
		if int(ctx["lod"]) == 1:
			return
		sides = 3
	ctx["bark_quads"] = int(ctx["bark_quads"]) + sides
	var axis := (b - a).normalized()
	var ref := Vector3.FORWARD if absf(axis.dot(Vector3.FORWARD)) < 0.9 else Vector3.RIGHT
	var u := axis.cross(ref).normalized()
	var w := axis.cross(u).normalized()
	var length := a.distance_to(b)
	var circ := maxf(ra * TAU, 0.3)
	for i in sides:
		var t0 := TAU * float(i) / float(sides)
		var t1 := TAU * float(i + 1) / float(sides)
		var n0 := u * cos(t0) + w * sin(t0)
		var n1 := u * cos(t1) + w * sin(t1)
		var pts := [a + n0 * ra, b + n0 * rb, b + n1 * rb, a + n1 * ra]
		var u0 := float(i) / float(sides) * circ
		var u1 := float(i + 1) / float(sides) * circ
		var uv := [Vector2(u0, length / 1.5), Vector2(u0, 0.0), Vector2(u1, 0.0), Vector2(u1, length / 1.5)]
		var ns := [n0, n0, n1, n1]
		for idx in [0, 2, 1, 0, 3, 2]:
			st.set_normal(ns[idx])
			st.set_uv(uv[idx])
			st.add_vertex(pts[idx])


# --- Imposteurs ----------------------------------------------------------------------------


## Rangée de l'atlas de `species` (−1 : pas d'imposteur).
static func impostor_row(species: String) -> int:
	return IMPOSTOR_SPECIES.find(species)


## Échelle de cuisson d'une essence : la plus grande dimension du maillage (hauteur, ou largeur
## vue sous n'importe quel angle) remplit 92 % de la cellule.
static func _bake_scale(species: String) -> float:
	var box := mesh(species, 0, false, 0.0, 100000.0).get_aabb()
	var half_w := maxf(maxf(absf(box.position.x), absf(box.end.x)), maxf(absf(box.position.z), absf(box.end.z)))
	var tall := box.end.y + FOOT * CELL_WORLD
	return CELL_WORLD * 0.92 / maxf(tall, half_w * 2.0 * 1.2)


## Pose une tuile d'imposteurs (instances : transformée, teinte, essence). Invisible tant que
## l'atlas n'est pas cuit (pas de rendu en mode sans affichage : jamais).
func add_impostor_tile(name_: String, transforms: Array, tints: Array, rows: Array) -> void:
	if impostor_material == null:
		impostor_material = ShaderMaterial.new()
		impostor_material.shader = IMPOSTOR_SHADER
		impostor_material.set_shader_parameter("cols", COLS)
		impostor_material.set_shader_parameter("rows", IMPOSTOR_SPECIES.size())
		var cells := PackedFloat32Array()
		cells.resize(8)
		for i in IMPOSTOR_SPECIES.size():
			cells[i] = CELL_WORLD / _bake_scale(IMPOSTOR_SPECIES[i])
		impostor_material.set_shader_parameter("cell_m", cells)
		impostor_material.set_shader_parameter("foot", FOOT)
		impostor_material.set_shader_parameter("lod_near", IMPOSTOR_DISTANCE)
		impostor_material.set_shader_parameter("lod_band", LOD_BAND)
	var mm := MultiMesh.new()
	mm.transform_format = MultiMesh.TRANSFORM_3D
	mm.use_colors = true
	mm.use_custom_data = true
	mm.mesh = BattleImpostors.quad_mesh()
	mm.instance_count = transforms.size()
	for i in transforms.size():
		mm.set_instance_transform(i, transforms[i])
		mm.set_instance_color(i, tints[i])
		mm.set_instance_custom_data(i, Color(float(rows[i]), 0, 0, 0))
	var aabb := AABB()
	for i in transforms.size():
		var o: Vector3 = (transforms[i] as Transform3D).origin
		aabb = AABB(o, Vector3.ZERO) if i == 0 else aabb.expand(o)
	var instance := MultiMeshInstance3D.new()
	instance.name = name_
	instance.multimesh = mm
	instance.material_override = impostor_material
	instance.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	instance.custom_aabb = AABB(aabb.position - Vector3(20, 5, 20), aabb.size + Vector3(40, 50, 40))
	instance.visible = baked
	add_child(instance)
	_impostor_nodes.append(instance)


## Cuisson de l'atlas : chaque essence rendue sous `COLS` angles, de profil, en lumière ambiante
## seule (albédo et ombre propre ; l'éclairage est refait au rendu avec une normale arrondie).
func bake_impostors(winter: bool) -> void:
	if impostor_material == null:
		return
	var rows := IMPOSTOR_SPECIES.size()
	var viewport := SubViewport.new()
	viewport.size = Vector2i(CELL_PX * COLS, CELL_PX * rows)
	viewport.own_world_3d = true
	viewport.transparent_bg = true
	viewport.msaa_3d = Viewport.MSAA_4X
	viewport.render_target_update_mode = SubViewport.UPDATE_ONCE
	add_child(viewport)
	var env := Environment.new()
	env.background_mode = Environment.BG_CLEAR_COLOR
	env.ambient_light_source = Environment.AMBIENT_SOURCE_COLOR
	env.ambient_light_color = Color(1, 1, 1)
	env.ambient_light_energy = 1.0
	env.reflected_light_source = Environment.REFLECTION_SOURCE_DISABLED
	env.tonemap_mode = Environment.TONE_MAPPER_LINEAR
	var world_env := WorldEnvironment.new()
	world_env.environment = env
	viewport.add_child(world_env)
	var width := CELL_WORLD * COLS
	var height := CELL_WORLD * rows
	var camera := Camera3D.new()
	camera.projection = Camera3D.PROJECTION_ORTHOGONAL
	camera.keep_aspect = Camera3D.KEEP_HEIGHT
	camera.size = height
	camera.near = 1.0
	camera.far = 1000.0
	camera.position = Vector3(0, 0, 400)
	camera.current = true
	viewport.add_child(camera)
	for r in rows:
		var species: String = IMPOSTOR_SPECIES[r]
		var source := mesh(species, 0, winter, 0.0, 100000.0)
		var bark_mat := bark_material(species, 0.0, 100000.0)
		var leaf_mat := foliage_material(species, winter, 0.0, 100000.0)
		leaf_mat.set_shader_parameter("decor_saturation", 1.0)  # désaturé au rendu de l'imposteur
		leaf_mat.set_shader_parameter("wind_strength", 0.0)
		var k := _bake_scale(species)
		for c in COLS:
			var inst := MeshInstance3D.new()
			inst.mesh = source
			inst.set_surface_override_material(0, bark_mat)
			inst.set_surface_override_material(1, leaf_mat)
			var x := (float(c) + 0.5) * CELL_WORLD - width * 0.5
			var y := height * 0.5 - float(r + 1) * CELL_WORLD + FOOT * CELL_WORLD
			inst.transform = Transform3D(Basis(Vector3.UP, float(c) * TAU / float(COLS) + 0.4).scaled(Vector3(k, k, k)), Vector3(x, y, 0))
			viewport.add_child(inst)
	await RenderingServer.frame_post_draw
	await get_tree().process_frame
	await RenderingServer.frame_post_draw
	var image := viewport.get_texture().get_image()
	viewport.queue_free()
	if image == null or image.is_empty():
		return  # sans rendu (mode sans affichage) : pas d'imposteurs
	image.generate_mipmaps()
	impostor_material.set_shader_parameter("atlas", ImageTexture.create_from_image(image))
	baked = true
	for node in _impostor_nodes:
		node.visible = true
	atlas_image = image


## Atlas cuit (captures, tests).
var atlas_image: Image = null
