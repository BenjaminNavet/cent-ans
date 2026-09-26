class_name TownBuilder
extends RefCounted

## Lot ZG6 (ADR 0036) : nœuds d'une ville à l'échelle réelle depuis son plan (`TownPlan`),
## construits par petites étapes sur le fil principal (`step`, budget en µs, ADR 0051).
## Le nœud racine est posé à l'ancrage de la ville et mis à l'échelle 1 / (m par unité) : tout
## est en mètres dessous. Hauteurs de base en mètres dans les instances (`INSTANCE_CUSTOM.r`) ou
## les sommets (`UV2.x`), multipliées par `campaign_vertical_scale` dans `town_building.gdshader`.
## HLOD par maison (ZG7a) : maisons du kit bas détail (`game/assets/models/town_kit/`) de près,
## blocs simples (une boîte à pignon par maison) plus loin ; murailles, rues, monuments et pont
## toujours présents jusqu'à la portée des blocs. Blocs : un MultiMesh par ville ; maisons
## détaillées : un MultiMesh par modèle et par cellule de `DETAIL_CELL_M` (le nœud n'est envoyé
## que près de la caméra, ombres comprises). Le choix détail / bloc se fait par instance dans
## `town_building.gdshader` (`lod_mode`, distance à `lod_camera`), plus par nœud de cellule de
## 250 m (ZG6 : deux fois plus d'appels de dessin).
## Rendu seulement.

const KIT_DIR := "res://assets/models/town_kit/"
const SHADER := preload("res://shaders/town_building.gdshader")
## Hauteur moyenne (m) des blocs du HLOD lointain, par type de maison.
const GROUND_STRIP_ROWS := 24
const STREET_GROUP := 40
## ZG7a : côté des cellules des maisons détaillées (m). Une ville entière par nœud faisait passer
## toutes les maisons de chaque modèle dans le vertex shader (et chaque cascade d'ombre) dès
## qu'une seule était proche : plus lent que ZG6 malgré moitié moins d'appels de dessin.
const DETAIL_CELL_M := 1000.0
const BLOCK_HEIGHT := {"townhouse": 13.5, "timber": 11.2, "stonehouse": 11.2, "cottage": 7.2, "longere": 7.3, "barn": 11.6}

static var _manifest: Dictionary = {}
static var _meshes: Dictionary = {}  # nom → Mesh
static var _materials: Dictionary = {}  # clé → ShaderMaterial
## ZG7a : matériaux dont le HLOD dépend de la caméra (`lod_mode` > 0), mis à jour par image.
static var _lod_materials: Array[ShaderMaterial] = []
static var _lod_camera := Vector3(INF, INF, INF)
static var _lod_range := -1.0
## SZ4 : réglages du sol « masse de toits » (`TownRenderProfile.roofscape_*`), posés par `TownLayer`.
static var _roofscape: Dictionary = {}

var plan: Dictionary
var root: Node3D
var meters_per_unit := 719.0
## Portées (unités monde, distance caméra → maison) : maisons détaillées, blocs.
var detail_range := 1.6
var block_range := 14.0
var detail_shadows := true
var block_shadows := false
var done := false
## Nœuds géométriques avec leurs bornes de base (m) : recalage des AABB à l'échelle verticale.
var geometry: Array = []  # [GeometryInstance3D, base_min, base_max, top_m, Rect2 xz (m)]
var _tasks: Array[Callable] = []
## Tâche la plus longue (µs) : une tâche est indivisible, elle borne le coût d'une image.
var task_max_usec := 0
var task_max_name := ""


static func manifest() -> Dictionary:
	if _manifest.is_empty():
		var path := KIT_DIR + "manifest.json"
		if FileAccess.file_exists(path):
			var parsed: Variant = JSON.parse_string(FileAccess.get_file_as_string(path))
			if parsed is Dictionary:
				_manifest = parsed
	return _manifest


static func models_of(kind: String) -> Array:
	var out := []
	var all := manifest()
	for model_name in all:
		if str(all[model_name]["kind"]) == kind:
			out.append(model_name)
	out.sort()
	return out


## Modèle du kit de `kind` qui épouse le mieux `front` × `depth` (tirage parmi les deux meilleurs).
static func pick(kind: String, front: float, depth: float, key: int) -> String:
	var candidates := models_of(kind)
	if candidates.is_empty():
		return ""
	var target := front / maxf(depth, 0.1)
	var all := manifest()
	candidates.sort_custom(func(a: String, b: String) -> bool:
		var ra := float(all[a]["length"]) / float(all[a]["depth"])
		var rb := float(all[b]["length"]) / float(all[b]["depth"])
		return absf(log(ra / target)) < absf(log(rb / target)))
	return candidates[key % mini(2, candidates.size())]


func _load_model(model_name: String) -> void:
	kit_mesh(model_name)


static func kit_mesh(model_name: String) -> Mesh:
	if _meshes.has(model_name):
		return _meshes[model_name]
	var mesh: Mesh = null
	var path := KIT_DIR + model_name + ".glb"
	if ResourceLoader.exists(path):
		var scene := load(path) as PackedScene
		if scene != null:
			var node := scene.instantiate()
			for child in node.find_children("*", "MeshInstance3D", true, false):
				mesh = (child as MeshInstance3D).mesh
				break
			node.free()
	_meshes[model_name] = mesh
	return mesh


## Matériau atlas des villes : `base_source` 0 (instances) ou 1 (sommets), UV en boîte, levée,
## `lod_mode` (ZG7a : 0 toujours, 1 maisons détaillées de près, 2 blocs au-delà).
## `roofscape` (SZ4) : sol bâti teinté en masse de toits de loin (`set_roofscape`).
static func material(base_source: int, box_uv: bool, lift_m: float = 0.0, meters_per_unit: float = 719.0, lod_mode: int = 0, roofscape: bool = false) -> ShaderMaterial:
	var key := "%d|%s|%.2f|%.1f|%d|%s" % [base_source, box_uv, lift_m, meters_per_unit, lod_mode, roofscape]
	if _materials.has(key):
		return _materials[key]
	var mat := ShaderMaterial.new()
	mat.shader = SHADER
	var atlas := BuildingMaterials.material("Building", "far") as ShaderMaterial
	if atlas != null:
		for p in ["albedo_array", "layer_tint", "layer_tile", "first_plain", "roof_first", "roof_last"]:
			mat.set_shader_parameter(p, atlas.get_shader_parameter(p))
	mat.set_shader_parameter("base_source", base_source)
	mat.set_shader_parameter("box_uv", box_uv)
	mat.set_shader_parameter("lift_m", lift_m)
	mat.set_shader_parameter("meters_per_unit", meters_per_unit)
	mat.set_shader_parameter("lod_mode", lod_mode)
	if roofscape:
		_apply_roofscape(mat)
	if lod_mode > 0:
		_lod_materials.append(mat)
		if _lod_range > 0.0:
			mat.set_shader_parameter("lod_range", _lod_range)
			mat.set_shader_parameter("lod_camera", _lod_camera)
	_materials[key] = mat
	return mat


## ZG7a : position de la caméra (monde) et portée des maisons détaillées pour le HLOD par
## instance. Appelé à chaque image par `TownLayer` (ombres comprises : même choix dans la passe
## d'ombre, qui ne connaît pas la caméra principale).
static func set_lod_view(camera_world: Vector3, detail_range: float) -> void:
	if camera_world.is_equal_approx(_lod_camera) and is_equal_approx(detail_range, _lod_range):
		return
	_lod_camera = camera_world
	_lod_range = detail_range
	for mat in _lod_materials:
		mat.set_shader_parameter("lod_camera", camera_world)
		mat.set_shader_parameter("lod_range", detail_range)


## SZ4 : réglages du sol « masse de toits » (clés `near`, `far`, `strength`, `cell_m`, `gain`),
## appliqués aux matériaux de sol existants et futurs.
static func set_roofscape(settings: Dictionary) -> void:
	_roofscape = settings
	for key: String in _materials:
		if key.ends_with("|true"):
			_apply_roofscape(_materials[key])


static func _apply_roofscape(mat: ShaderMaterial) -> void:
	mat.set_shader_parameter("roofscape", float(_roofscape.get("strength", 0.0)))
	mat.set_shader_parameter("roofscape_near", float(_roofscape.get("near", 1.2)))
	mat.set_shader_parameter("roofscape_far", float(_roofscape.get("far", 3.5)))
	mat.set_shader_parameter("roofscape_cell_m", float(_roofscape.get("cell_m", 9.0)))
	mat.set_shader_parameter("roofscape_gain", float(_roofscape.get("gain", 1.0)))


static func clear_cache() -> void:
	_manifest.clear()
	_meshes.clear()
	_materials.clear()
	_lod_materials.clear()


## Couleur de sommet d'une couche de l'atlas (alpha = (indice + 0,5) / 16).
static func layer_color(layer_name: String, tint: Color = Color(1, 1, 1)) -> Color:
	var index := BuildingMaterials.ATLAS_LAYERS.find(layer_name)
	return Color(tint.r, tint.g, tint.b, (maxi(index, 0) + 0.5) / 16.0)


## Bloc de maison unitaire (1 × 1 × 1, pignon le long de X, sol à 0, fondation à -0,15) :
## murs enduits, toit de tuiles.
static func block_mesh() -> Mesh:
	if _meshes.has("__block"):
		return _meshes["__block"]
	var st := SurfaceTool.new()
	st.begin(Mesh.PRIMITIVE_TRIANGLES)
	var wall := layer_color("Plaster", Color(0.47, 0.42, 0.36))
	var roof := layer_color("RoofTile", Color(0.8, 0.58, 0.5))
	var eave := 0.52
	var y0 := -0.15
	var corners := [Vector3(-0.5, 0, -0.5), Vector3(0.5, 0, -0.5), Vector3(0.5, 0, 0.5), Vector3(-0.5, 0, 0.5)]
	for i in 4:
		var a: Vector3 = corners[i]
		var b: Vector3 = corners[(i + 1) % 4]
		_quad(st, Vector3(a.x, y0, a.z), Vector3(b.x, y0, b.z), Vector3(b.x, eave, b.z), Vector3(a.x, eave, a.z), wall)
	# Pignons (triangles aux extrémités X).
	for sx in [-0.5, 0.5]:
		var p0 := Vector3(sx, eave, -0.5)
		var p1 := Vector3(sx, eave, 0.5)
		var p2 := Vector3(sx, 1.0, 0.0)
		_tri(st, p0, p1, p2, wall)
	# Pans de toit avec débord.
	var o := 0.08
	_quad(st, Vector3(-0.5 - o, eave - 0.05, 0.5 + o), Vector3(0.5 + o, eave - 0.05, 0.5 + o), Vector3(0.5 + o, 1.0, 0.0), Vector3(-0.5 - o, 1.0, 0.0), roof)
	_quad(st, Vector3(0.5 + o, eave - 0.05, -0.5 - o), Vector3(-0.5 - o, eave - 0.05, -0.5 - o), Vector3(-0.5 - o, 1.0, 0.0), Vector3(0.5 + o, 1.0, 0.0), roof)
	var mesh := st.commit()
	_meshes["__block"] = mesh
	return mesh


## Boîte unitaire (x, z dans [-0,5 ; 0,5], y de -0,1 à 1) d'une couche de l'atlas.
static func box_mesh(layer_name: String = "Masonry") -> Mesh:
	var key := "__box_" + layer_name
	if _meshes.has(key):
		return _meshes[key]
	var st := SurfaceTool.new()
	st.begin(Mesh.PRIMITIVE_TRIANGLES)
	var c := layer_color(layer_name, Color(0.95, 0.93, 0.88))
	var corners := [Vector3(-0.5, 0, -0.5), Vector3(0.5, 0, -0.5), Vector3(0.5, 0, 0.5), Vector3(-0.5, 0, 0.5)]
	for i in 4:
		var a: Vector3 = corners[i]
		var b: Vector3 = corners[(i + 1) % 4]
		_quad(st, Vector3(a.x, -0.1, a.z), Vector3(b.x, -0.1, b.z), Vector3(b.x, 1.0, b.z), Vector3(a.x, 1.0, a.z), c)
	_quad(st, Vector3(-0.5, 1, 0.5), Vector3(0.5, 1, 0.5), Vector3(0.5, 1, -0.5), Vector3(-0.5, 1, -0.5), c)
	var mesh := st.commit()
	_meshes[key] = mesh
	return mesh


## Tour ronde unitaire (rayon 1, fût de 0 à 1, toit conique d'ardoise jusqu'à 1,45).
static func tower_mesh() -> Mesh:
	if _meshes.has("__tower"):
		return _meshes["__tower"]
	var st := SurfaceTool.new()
	st.begin(Mesh.PRIMITIVE_TRIANGLES)
	var wall := layer_color("Masonry", Color(0.95, 0.93, 0.88))
	var roof := layer_color("RoofSlate")
	var sides := 12
	for i in sides:
		var a0 := TAU * i / sides
		var a1 := TAU * (i + 1) / sides
		var p0 := Vector3(cos(a0), 0, sin(a0))
		var p1 := Vector3(cos(a1), 0, sin(a1))
		_quad(st, p1 + Vector3(0, -0.1, 0), p0 + Vector3(0, -0.1, 0), p0 + Vector3(0, 1, 0), p1 + Vector3(0, 1, 0), wall)
		_tri(st, p0 * 1.12 + Vector3(0, 0.98, 0), p1 * 1.12 + Vector3(0, 0.98, 0), Vector3(0, 1.45, 0), roof)
	var mesh := st.commit()
	_meshes["__tower"] = mesh
	return mesh


## Quadrilatère plan (a, b, c, d dans l'ordre du pourtour) d'un maillage unitaire centré sur l'axe
## Y : face avant (normale géométrique (p1 - p0) × (p2 - p0), convention de Godot) tournée vers
## l'extérieur, c'est-à-dire à l'opposé du point (0 ; 0,4 ; 0).
static func _quad(st: SurfaceTool, a: Vector3, b: Vector3, c: Vector3, d: Vector3, color: Color) -> void:
	_tri(st, a, b, c, color, (a + b + c + d) * 0.25)
	_tri(st, a, c, d, color, (a + b + c + d) * 0.25)


static func _tri(st: SurfaceTool, a: Vector3, b: Vector3, c: Vector3, color: Color, center: Variant = null) -> void:
	var mid: Vector3 = center if center is Vector3 else (a + b + c) / 3.0
	var outward := mid - Vector3(0, 0.4, 0)
	var n := (b - a).cross(c - a)
	if n.dot(outward) < 0.0:
		var t := b
		b = c
		c = t
		n = -n
	n = n.normalized()
	for p in [a, b, c]:
		st.set_color(color)
		st.set_normal(n)
		st.add_vertex(p)


## Base (x, z) d'un modèle dont l'axe X local suit la direction `d` (plan horizontal carte).
static func basis_x(d: Vector2, scale: Vector3 = Vector3.ONE) -> Basis:
	var x := Vector3(d.x, 0, d.y)
	var z := Vector3(-d.y, 0, d.x)
	return Basis(x * scale.x, Vector3.UP * scale.y, z * scale.z)


# --- Construction --------------------------------------------------------------------------


func _init(p_plan: Dictionary, anchor: Vector2, p_meters_per_unit: float, parent: Node3D) -> void:
	plan = p_plan
	meters_per_unit = p_meters_per_unit
	root = Node3D.new()
	root.name = "Town_" + str(plan.get("id", ""))
	var s := 1.0 / meters_per_unit
	root.transform = Transform3D(Basis().scaled(Vector3(s, s, s)), Vector3(anchor.x, 0.0, anchor.y))
	parent.add_child(root)
	if not plan.has("prepared"):
		prepare(plan)
	var prepared: Dictionary = plan["prepared"]
	var strips: Array = prepared["ground"]
	for k in strips.size():
		_tasks.append(_draped_node.bind("Ground_%d" % k, strips[k], 0.7, 2.0, false, true))
	var groups: Array = prepared["streets"]
	for k in groups.size():
		_tasks.append(_draped_node.bind("Streets_%d" % k, groups[k], 0.9, 2.0, false))
	_tasks.append(_build_walls)
	_tasks.append(_build_monuments)
	for k in (prepared.get("wall_rings", []) as Array).size():
		_tasks.append(_draped_node.bind("WallRing_%d" % k, prepared["wall_rings"][k], 0.0, float(prepared["wall_rings"][k].get("top", 12.0)), true))
	for extra: Dictionary in prepared.get("extras", []):
		_tasks.append(_build_extra.bind(extra))
	# Modèles du kit pas encore chargés : un chargement par tâche (étalé sur les images).
	var to_load := {}
	for cell: Dictionary in prepared["detail"]:
		for model in cell["models"]:
			if not _meshes.has(model):
				to_load[model] = true
	for model in to_load:
		_tasks.append(_load_model.bind(model))
	for cell: Dictionary in prepared["detail"]:
		_tasks.append(_build_detail.bind(cell))
	_tasks.append(_build_blocks)


## Avance la construction ; rend vrai quand tout est fait. Au moins une étape par appel.
func step(budget_usec: int) -> bool:
	var t0 := Time.get_ticks_usec()
	while not _tasks.is_empty():
		var task: Callable = _tasks.pop_front()
		var t_task := Time.get_ticks_usec()
		task.call()
		var spent := Time.get_ticks_usec() - t_task
		if spent > task_max_usec:
			task_max_usec = spent
			task_max_name = task.get_method()
		if Time.get_ticks_usec() - t0 >= budget_usec or not FrameBudget.has_time():
			break
	done = _tasks.is_empty()
	return done


func remaining() -> int:
	return _tasks.size()


func free_nodes() -> void:
	if is_instance_valid(root):
		root.queue_free()
	_tasks.clear()
	geometry.clear()


## Portées de visibilité (unités monde) : appliquées aux nœuds déjà construits.
func set_ranges(p_detail: float, p_block: float, p_detail_shadows: bool, p_block_shadows: bool) -> void:
	detail_range = p_detail
	block_range = p_block
	detail_shadows = p_detail_shadows
	block_shadows = p_block_shadows
	for entry in geometry:
		_apply_range(entry[0], str((entry[0] as Node).get_meta("lod", "all")))


func _apply_range(g: GeometryInstance3D, lod: String) -> void:
	match lod:
		"detail":
			# ZG7a : le shader choisit maison par maison (`lod_mode` 1) ; le nœud (une cellule)
			# n'est envoyé que si une maison peut être à moins de `detail_range` de la caméra.
			g.visibility_range_end = detail_range + float(g.get_meta("radius_units", 0.0))
			g.visibility_range_end_margin = 0.0
			g.visibility_range_fade_mode = GeometryInstance3D.VISIBILITY_RANGE_FADE_DISABLED
			g.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_ON if detail_shadows else GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
		"block":
			# Blocs repliés près de la caméra par le shader (`lod_mode` 2).
			g.visibility_range_end = block_range
			g.visibility_range_end_margin = block_range * 0.1
			g.visibility_range_fade_mode = GeometryInstance3D.VISIBILITY_RANGE_FADE_SELF
			g.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_ON if block_shadows else GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
		_:
			g.visibility_range_end = block_range
			g.visibility_range_end_margin = block_range * 0.1
			g.visibility_range_fade_mode = GeometryInstance3D.VISIBILITY_RANGE_FADE_SELF
			g.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_ON if detail_shadows else GeometryInstance3D.SHADOW_CASTING_SETTING_OFF


## Boîtes englobantes (espace local, mètres) selon l'échelle verticale courante : la base est
## ajoutée par le shader, Godot ne la voit pas. Appelé à chaque changement d'échelle.
func refresh_aabbs(vertical_scale: float) -> void:
	var k := vertical_scale * meters_per_unit
	var up := 1.0 + MapData.relief_gain_for_scale(vertical_scale)  # ZG8 : y ≤ s·(1 + g)·h
	var down := 1.0 - MapData.relief_squash_max_for_scale(vertical_scale)  # SZ1 : y ≥ s·(1 − c·k)·h
	for entry in geometry:
		var g: GeometryInstance3D = entry[0]
		if not is_instance_valid(g):
			continue
		var rect: Rect2 = entry[4]
		var y0 := float(entry[1]) * k * (down if float(entry[1]) > 0.0 else 1.0) - 5.0
		var y1 := float(entry[2]) * k * (up if float(entry[2]) > 0.0 else 1.0) + float(entry[3])
		g.custom_aabb = AABB(Vector3(rect.position.x, y0, rect.position.y), Vector3(rect.size.x, y1 - y0, rect.size.y))


func _register(g: GeometryInstance3D, lod: String, base_min: float, base_max: float, top: float, rect: Rect2) -> void:
	g.set_meta("lod", lod)
	g.set_meta("radius_units", rect.size.length() * 0.5 / meters_per_unit)
	_apply_range(g, lod)
	geometry.append([g, base_min, base_max, top, rect])
	var k := MapData.vertical_scale() * meters_per_unit
	var y0 := base_min * k * ((1.0 - MapData.relief_squash_max_for_scale(MapData.vertical_scale())) if base_min > 0.0 else 1.0) - 5.0
	var y1 := base_max * k * ((1.0 + MapData.relief_gain()) if base_max > 0.0 else 1.0) + top
	g.custom_aabb = AABB(Vector3(rect.position.x, y0, rect.position.y), Vector3(rect.size.x, y1 - y0, rect.size.y))
	root.add_child(g)


## Tampon MultiMesh (transformation 3 × 4 + données d'instance : base en m, teinte) et bornes.
static func pack_instances(xforms: Array, bases: Array, tints: Array) -> Dictionary:
	var buf := PackedFloat32Array()
	buf.resize(xforms.size() * 16)
	var lo := INF
	var hi := -INF
	var rect := Rect2()
	for i in xforms.size():
		var t: Transform3D = xforms[i]
		var o := i * 16
		buf[o] = t.basis.x.x
		buf[o + 1] = t.basis.y.x
		buf[o + 2] = t.basis.z.x
		buf[o + 3] = t.origin.x
		buf[o + 4] = t.basis.x.y
		buf[o + 5] = t.basis.y.y
		buf[o + 6] = t.basis.z.y
		buf[o + 7] = t.origin.y
		buf[o + 8] = t.basis.x.z
		buf[o + 9] = t.basis.y.z
		buf[o + 10] = t.basis.z.z
		buf[o + 11] = t.origin.z
		buf[o + 12] = float(bases[i])
		buf[o + 13] = float(tints[i]) if i < tints.size() else 0.5
		lo = minf(lo, float(bases[i]))
		hi = maxf(hi, float(bases[i]))
		var p := Vector2(t.origin.x, t.origin.z)
		rect = Rect2(p, Vector2.ZERO) if i == 0 else rect.expand(p)
	return {"buffer": buf, "count": xforms.size(), "lo": lo, "hi": hi, "rect": rect.grow(60.0)}


## Préparation hors fil principal (fil de travail du plan) : tampons des maisons par cellule et
## par modèle (détail) et des blocs de toute la ville (ZG7a), et tableaux des maillages drapés (sol, rues,
## murailles). Le fil principal n'a plus qu'à créer les nœuds (`step`). Le manifeste du kit doit
## être chargé avant (`manifest()`).
static func prepare(plan: Dictionary) -> void:
	var houses: Dictionary = plan["houses"]
	var xs: PackedFloat32Array = houses["x"]
	var ys: PackedFloat32Array = houses["y"]
	var all := manifest()
	var cells := {}  # Vector2i → {modèle → [xforms, bases, tints]}
	# VH4 : cellules plus petites (îlots) pour les villes emblématiques 1:1.
	var cell_m := float(plan.get("detail_cell_m", DETAIL_CELL_M))
	var block_x: Array = []
	var block_b: Array = []
	var block_t: Array = []
	for i in xs.size():
		var kind: String = TownPlan.HOUSE_KINDS[houses["kind"][i]]
		var front: float = houses["front"][i]
		var depth: float = houses["depth"][i]
		var yaw: float = houses["yaw"][i]
		var d := Vector2(cos(yaw), sin(yaw))
		var pos := Vector3(xs[i], 0.0, ys[i])
		var model := pick(kind, front, depth, i)
		if model != "":
			var entry: Dictionary = all[model]
			var sx := front / float(entry["length"])
			var sz := depth / float(entry["depth"])
			var sy := clampf(sqrt(sx * sz), 0.85, 1.2)
			var key := Vector2i(floori(xs[i] / cell_m), floori(ys[i] / cell_m))
			if not cells.has(key):
				cells[key] = {}
			var groups: Dictionary = cells[key]
			if not groups.has(model):
				groups[model] = [[], [], []]
			groups[model][0].append(Transform3D(basis_x(d, Vector3(sx, sy, sz)), pos))
			groups[model][1].append(houses["base"][i])
			groups[model][2].append(houses["tint"][i])
		var h: float = BLOCK_HEIGHT.get(kind, 10.0)
		# Faîtage du bloc le long du grand côté (maison de ville : pignon sur rue).
		if depth > front:
			block_x.append(Transform3D(basis_x(Vector2(-d.y, d.x), Vector3(depth, h, front)), pos))
		else:
			block_x.append(Transform3D(basis_x(d, Vector3(front, h, depth)), pos))
		block_b.append(houses["base"][i])
		block_t.append(houses["tint"][i])
	var detail: Array = []
	var keys := cells.keys()
	keys.sort()
	for key: Vector2i in keys:
		var groups: Dictionary = cells[key]
		var names := groups.keys()
		names.sort()
		var models := {}
		for model in names:
			var g: Array = groups[model]
			models[model] = pack_instances(g[0], g[1], g[2])
		detail.append({"key": key, "models": models})
	plan["prepared"] = {"detail": detail, "blocks": pack_instances(block_x, block_b, block_t), "ground": _ground_strips(plan), "streets": _street_groups(plan), "walls": _wall_arrays(plan), "wall_rings": _wall_ring_arrays(plan), "extras": _extra_meshes(plan)}


func _instances_node(mesh: Mesh, packed: Dictionary, mat: Material, lod: String, top: float) -> MultiMeshInstance3D:
	var mm := MultiMesh.new()
	mm.transform_format = MultiMesh.TRANSFORM_3D
	mm.use_custom_data = true
	mm.mesh = mesh
	mm.instance_count = int(packed["count"])
	mm.buffer = packed["buffer"]
	var mmi := MultiMeshInstance3D.new()
	mmi.multimesh = mm
	mmi.material_override = mat
	_register(mmi, lod, float(packed["lo"]), float(packed["hi"]), top, packed["rect"])
	return mmi


## MultiMesh depuis des transformations (mètres) et des (base, teinte) par instance.
func _multimesh(mesh: Mesh, xforms: Array, bases: Variant, tints: Variant, mat: Material, lod: String, top: float) -> MultiMeshInstance3D:
	return _instances_node(mesh, pack_instances(xforms, Array(bases), Array(tints)), mat, lod, top)


func _build_detail(cell: Dictionary) -> void:
	var key: Vector2i = cell["key"]
	var models: Dictionary = cell["models"]
	for model: String in models:
		var mesh := kit_mesh(model)
		if mesh != null:
			_instances_node(mesh, models[model], material(0, false, 0.0, meters_per_unit, 1), "detail", 30.0).name = "Detail_%d_%d_%s" % [key.x, key.y, model]


func _build_blocks() -> void:
	var blocks: Dictionary = plan["prepared"]["blocks"]
	if int(blocks["count"]) > 0:
		_instances_node(block_mesh(), blocks, material(0, true, 0.0, meters_per_unit, 2), "block", 20.0).name = "Blocks"


## Nœud d'un maillage drapé préparé (`prepare`).
func _draped_node(node_name: String, prepared: Dictionary, lift_m: float, top: float, shadows: bool, roofscape: bool = false) -> void:
	if prepared.is_empty():
		return
	var mesh := ArrayMesh.new()
	mesh.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES, prepared["arrays"])
	var mi := MeshInstance3D.new()
	mi.name = node_name
	mi.mesh = mesh
	mi.material_override = material(1, false, lift_m, meters_per_unit, 0, roofscape)
	_register(mi, "all", float(prepared["lo"]), float(prepared["hi"]), top, prepared["rect"])
	if not shadows:
		mi.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF


## Sol en bandes de `GROUND_STRIP_ROWS` rangées : une bande par tâche (envoi au GPU étalé).
static func _ground_strips(plan: Dictionary) -> Array:
	var ground: Dictionary = plan.get("ground", {})
	var out: Array = []
	if ground.is_empty():
		return out
	var n: int = ground["n"]
	var built := _built_density(plan, ground)
	var row := 0
	while row < n - 1:
		var strip := _ground_arrays(plan, built, row, mini(row + GROUND_STRIP_ROWS, n - 1))
		if not strip.is_empty():
			out.append(strip)
		row += GROUND_STRIP_ROWS
	return out


## Sol de terre battue, cours et jardins sous la ville (grille drapée, teinte fondue au bord),
## rangées de mailles [row0, row1).
## Part bâtie (0-1) autour de chaque sommet de la grille du sol : maisons à moins de ~25 m.
## Les grands vides de l'enceinte (jardins, prés, vignes intra-muros) restent verts.
static func _built_density(plan: Dictionary, ground: Dictionary) -> PackedFloat32Array:
	var n: int = ground["n"]
	var step: float = ground["step"]
	var origin: Vector2 = ground["origin"]
	var out := PackedFloat32Array()
	out.resize(n * n)
	var houses: Dictionary = plan["houses"]
	var xs: PackedFloat32Array = houses["x"]
	var ys: PackedFloat32Array = houses["y"]
	var reach := 2
	for i in xs.size():
		var ci := roundi((xs[i] - origin.x) / step)
		var cj := roundi((ys[i] - origin.y) / step)
		for dj in range(-reach, reach + 1):
			for di in range(-reach, reach + 1):
				var gi := ci + di
				var gj := cj + dj
				if gi < 0 or gj < 0 or gi >= n or gj >= n:
					continue
				var w := 1.0 - Vector2(di, dj).length() / (reach + 1.0)
				if w > 0.0:
					var k := gj * n + gi
					out[k] = minf(out[k] + w * 0.5, 1.0)
	for street in plan["streets"]:
		for p: Vector2 in street["points"]:
			var gi := roundi((p.x - origin.x) / step)
			var gj := roundi((p.y - origin.y) / step)
			if gi >= 0 and gj >= 0 and gi < n and gj < n:
				out[gj * n + gi] = maxf(out[gj * n + gi], 0.6)
	return out


static func _ground_arrays(plan: Dictionary, built: PackedFloat32Array, row0: int, row1: int) -> Dictionary:
	var ground: Dictionary = plan.get("ground", {})
	var n: int = ground["n"]
	var step: float = ground["step"]
	var origin: Vector2 = ground["origin"]
	var mask: PackedByteArray = ground["inside"]
	var h: PackedFloat32Array = ground["heights"]
	var radii: PackedFloat32Array = ground["radii"]
	var st := SurfaceTool.new()
	st.begin(Mesh.PRIMITIVE_TRIANGLES)
	var lo := INF
	var hi := -INF
	var colors := PackedColorArray()
	colors.resize(n * n)
	for k in range(row0 * n, mini((row1 + 1) * n, n * n)):
		if mask[k] == 1:
			var p: Vector2 = origin + Vector2(k % n, k / n) * step
			var edge := float(ground["edge"][k]) if ground.has("edge") else clampf((TownPlan.radius_at(radii, atan2(p.y, p.x)) - p.length()) / 40.0, 0.0, 1.0)
			var yard := Color(0.24, 0.26, 0.18).lerp(Color(0.25, 0.23, 0.18), edge)
			colors[k] = layer_color("Rubble", Color(0.17, 0.23, 0.11).lerp(yard, built[k]))
			lo = minf(lo, h[k])
			hi = maxf(hi, h[k])
	if lo == INF:
		return {}
	var used := false
	for j in range(row0, row1):
		for i in n - 1:
			var k0 := j * n + i
			if mask[k0] == 0 or mask[k0 + 1] == 0 or mask[k0 + n] == 0 or mask[k0 + n + 1] == 0:
				continue
			for k: int in [k0, k0 + 1, k0 + n + 1, k0, k0 + n + 1, k0 + n]:
				var p: Vector2 = origin + Vector2(k % n, k / n) * step
				st.set_color(colors[k])
				st.set_normal(Vector3.UP)
				st.set_uv(p)
				st.set_uv2(Vector2(h[k], built[k]))  # SZ4 : part bâtie (sol « masse de toits »)
				st.add_vertex(Vector3(p.x, 0.0, p.y))
				used = true
	if not used:
		return {}
	var half := (n - 1) * 0.5 * step
	var rect := Rect2(origin.x, origin.y + row0 * step, half * 2.0, (row1 - row0) * step)
	return {"arrays": st.commit_to_arrays(), "lo": lo, "hi": hi, "rect": rect}


## Rues par paquets de `STREET_GROUP` : un maillage par tâche.
static func _street_groups(plan: Dictionary) -> Array:
	var streets: Array = plan["streets"]
	var out: Array = []
	var k := 0
	while k < streets.size():
		var group := _street_arrays(streets.slice(k, k + STREET_GROUP))
		if not group.is_empty():
			out.append(group)
		k += STREET_GROUP
	return out


static func _street_arrays(streets: Array) -> Dictionary:
	if streets.is_empty():
		return {}
	var st := SurfaceTool.new()
	st.begin(Mesh.PRIMITIVE_TRIANGLES)
	var earth := layer_color("Rubble", Color(0.62, 0.55, 0.46))
	var paved := layer_color("Rubble", Color(0.78, 0.74, 0.68))
	var water := layer_color("Plaster", Color(0.16, 0.22, 0.24))
	var lo := INF
	var hi := -INF
	var rect := Rect2()
	var first := true
	for street in streets:
		var pts: PackedVector2Array = street["points"]
		var bl: PackedFloat32Array = street["bases_l"]
		var br: PackedFloat32Array = street["bases_r"]
		var half := float(street["width"]) * 0.5
		var color := paved if bool(street.get("market", false)) or bool(street.get("paved", false)) or half >= 3.5 else earth
		if bool(street.get("water", false)):
			color = water  # VH4 : ruisseau dessiné (Robec)
		var along := 0.0
		for i in range(1, pts.size()):
			var a := pts[i - 1]
			var b := pts[i]
			var ta := (pts[mini(i, pts.size() - 1)] - pts[maxi(i - 2, 0)]).normalized()
			var tb := (pts[mini(i + 1, pts.size() - 1)] - pts[i - 1]).normalized()
			var na := Vector2(-ta.y, ta.x) * half
			var nb := Vector2(-tb.y, tb.x) * half
			var seg := a.distance_to(b)
			var quad := [
				[a + na, bl[i - 1], Vector2(along, 0)], [b + nb, bl[i], Vector2(along + seg, 0)],
				[b - nb, br[i], Vector2(along + seg, half * 2.0)], [a - na, br[i - 1], Vector2(along, half * 2.0)],
			]
			for k in [0, 1, 2, 0, 2, 3]:
				var v: Array = quad[k]
				var p: Vector2 = v[0]
				st.set_color(color)
				st.set_normal(Vector3.UP)
				st.set_uv(v[2])
				st.set_uv2(Vector2(float(v[1]), 0.0))
				st.add_vertex(Vector3(p.x, 0.0, p.y))
				lo = minf(lo, float(v[1]))
				hi = maxf(hi, float(v[1]))
				rect = Rect2(p, Vector2.ZERO) if first else rect.expand(p)
				first = false
			along += seg
	return {"arrays": st.commit_to_arrays(), "lo": lo, "hi": hi, "rect": rect.grow(10.0)}


static func _wall_arrays(plan: Dictionary) -> Dictionary:
	var ring: PackedVector2Array = plan.get("wall_ring", PackedVector2Array())
	if ring.size() < 3:
		return {}
	var bases: PackedFloat32Array = plan["wall_bases"]
	var gaps: PackedInt32Array = plan["wall_gaps"]
	var height := float(plan.get("wall_height", 8.0))
	var half := float(plan.get("wall_thickness", 2.0)) * 0.5
	var palisade := str(plan.get("walls", "stone")) == "palisade"
	var color := layer_color("Planks" if palisade else "Masonry", Color(0.95, 0.93, 0.88))
	var st := SurfaceTool.new()
	st.begin(Mesh.PRIMITIVE_TRIANGLES)
	var along := 0.0
	var lo := INF
	var hi := -INF
	var rect := Rect2(ring[0], Vector2.ZERO)
	for i in range(1, ring.size()):
		var a := ring[i - 1]
		var b := ring[i]
		var seg := a.distance_to(b)
		rect = rect.expand(b)
		lo = minf(lo, bases[i])
		hi = maxf(hi, bases[i])
		if gaps[i] == 1 or gaps[i - 1] == 1:
			along += seg
			continue
		var na := a.normalized() * half
		var nb := b.normalized() * half
		var ha := bases[i - 1]
		var hb := bases[i]
		# Face extérieure, intérieure, chemin de ronde.
		_wall_quad(st, a + na, b + nb, ha, hb, -2.5, height, along, seg, color)
		_wall_quad(st, b - nb, a - na, hb, ha, -2.5, height, along, seg, color)
		_wall_top(st, a + na, b + nb, b - nb, a - na, ha, hb, height, color)
		along += seg
	return {"arrays": st.commit_to_arrays(), "lo": lo, "hi": hi, "rect": rect.grow(10.0), "top": height + 2.0}


## VH4 : enceintes polygonales quelconques (`plan.wall_rings`, normales explicites, fermées ou
## non) : faces extérieure et intérieure, chemin de ronde, crénelage simplifié.
static func _wall_ring_arrays(plan: Dictionary) -> Array:
	var out: Array = []
	for r: Dictionary in plan.get("wall_rings", []):
		var ring: PackedVector2Array = r["ring"]
		if ring.size() < 2:
			continue
		var normals: PackedVector2Array = r["normals"]
		var bases: PackedFloat32Array = r["bases"]
		var gaps: PackedInt32Array = r["gaps"]
		var height := float(r.get("height", 9.0))
		var half := float(r.get("thickness", 2.2)) * 0.5
		var color := layer_color("Masonry", Color(0.95, 0.93, 0.88))
		var st := SurfaceTool.new()
		st.begin(Mesh.PRIMITIVE_TRIANGLES)
		var along := 0.0
		var lo := INF
		var hi := -INF
		var rect := Rect2(ring[0], Vector2.ZERO)
		for i in range(1, ring.size()):
			var a := ring[i - 1]
			var b := ring[i]
			var seg := a.distance_to(b)
			rect = rect.expand(b)
			lo = minf(lo, bases[i])
			hi = maxf(hi, bases[i])
			if gaps[i] == 1 or gaps[i - 1] == 1:
				along += seg
				continue
			var na := normals[i - 1] * half
			var nb := normals[i] * half
			_wall_quad(st, a + na, b + nb, bases[i - 1], bases[i], -2.5, height, along, seg, color)
			_wall_quad(st, b - nb, a - na, bases[i], bases[i - 1], -2.5, height, along, seg, color)
			_wall_top(st, a + na, b + nb, b - nb, a - na, bases[i - 1], bases[i], height, color)
			# Parapet extérieur (merlons simplifiés en une lisse).
			var pa := a + na * 0.6
			var pb := b + nb * 0.6
			_wall_quad(st, pa + na * 0.4, pb + nb * 0.4, bases[i - 1], bases[i], height, height + 1.6, along, seg, color)
			_wall_quad(st, pb, pa, bases[i], bases[i - 1], height, height + 1.6, along, seg, color)
			along += seg
		if lo == INF:
			continue
		out.append({"arrays": st.commit_to_arrays(), "lo": lo, "hi": hi, "rect": rect.grow(10.0), "top": height + 3.0})
	return out


## VH4 : maillages uniques (monuments à gabarit réel) préparés dans le fil du plan.
static func _extra_meshes(plan: Dictionary) -> Array:
	var out: Array = []
	for m: Dictionary in plan.get("v2_monuments", []):
		out.append(m)
	return out


func _build_extra(m: Dictionary) -> void:
	var arrays: Array = m["arrays"]
	if arrays.is_empty() or (arrays[Mesh.ARRAY_VERTEX] as PackedVector3Array).is_empty():
		return
	var mesh := ArrayMesh.new()
	mesh.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES, arrays)
	var d := Vector2(cos(float(m["yaw"])), sin(float(m["yaw"])))
	var xform := Transform3D(basis_x(d), Vector3(float(m["x"]), 0.0, float(m["y"])))
	var node := _multimesh(mesh, [xform], [float(m["base"])], [0.5], material(0, true, 0.0, meters_per_unit), "all", float(m.get("top", 40.0)) + 10.0)
	node.name = "Monument_" + str(m.get("id", ""))
	# L'emprise d'un grand monument dépasse le rayon forfaitaire des instances.
	var r := maxf(float(m.get("length", 40.0)), float(m.get("depth", 40.0)))
	var rect := Rect2(Vector2(float(m["x"]), float(m["y"])) - Vector2(r, r), Vector2(r, r) * 2.0)
	geometry[geometry.size() - 1][4] = rect
	refresh_aabbs(MapData.vertical_scale())


func _build_walls() -> void:
	var walls: Dictionary = plan["prepared"]["walls"]
	_draped_node("Walls", walls, 0.0, float(walls.get("top", 12.0)), true)
	# Tours et portes.
	var towers: Array = plan.get("towers", [])
	if not towers.is_empty():
		var xs: Array = []
		var tb: Array = []
		for t in towers:
			var r := float(t["radius"])
			xs.append(Transform3D(Basis().scaled(Vector3(r, float(t["height"]), r)), Vector3(t["x"], 0, t["y"])))
			tb.append(t["base"])
		_multimesh(tower_mesh(), xs, tb, [], material(0, true, 0.0, meters_per_unit), "all", 30.0).name = "Towers"
	var gates: Array = plan.get("gates", [])
	if not gates.is_empty():
		var gx: Array = []
		var gb: Array = []
		var palisade := str(plan.get("walls", "stone")) == "palisade"
		for g in gates:
			var d := Vector2(cos(float(g["yaw"])), sin(float(g["yaw"])))
			var h := 7.0 if palisade else float(g.get("height", 16.0))
			gx.append(Transform3D(basis_x(d, Vector3(12.0, h, 11.0)), Vector3(g["x"], 0, g["y"])))
			gb.append(g["base"])
		_multimesh(box_mesh("Planks" if palisade else "Masonry"), gx, gb, [], material(0, true, 0.0, meters_per_unit), "all", 30.0).name = "Gates"


static func _wall_quad(st: SurfaceTool, p0: Vector2, p1: Vector2, h0: float, h1: float, bottom: float, top: float, along: float, seg: float, color: Color) -> void:
	var verts := [
		[Vector3(p0.x, bottom, p0.y), h0, Vector2(along, -bottom)], [Vector3(p1.x, bottom, p1.y), h1, Vector2(along + seg, -bottom)],
		[Vector3(p1.x, top, p1.y), h1, Vector2(along + seg, -top)], [Vector3(p0.x, top, p0.y), h0, Vector2(along, -top)],
	]
	var n3 := Vector3(p1.x - p0.x, 0, p1.y - p0.y).cross(Vector3.UP).normalized()
	for k in [0, 2, 1, 0, 3, 2]:
		var v: Array = verts[k]
		st.set_color(color)
		st.set_normal(-n3)
		st.set_uv(v[2])
		st.set_uv2(Vector2(float(v[1]), 0.0))
		st.add_vertex(v[0])


static func _wall_top(st: SurfaceTool, a: Vector2, b: Vector2, c: Vector2, d: Vector2, ha: float, hb: float, top: float, color: Color) -> void:
	var verts := [[a, ha], [b, hb], [c, hb], [d, ha]]
	for k in [0, 2, 1, 0, 3, 2]:
		var v: Array = verts[k]
		var p: Vector2 = v[0]
		st.set_color(color)
		st.set_normal(Vector3.UP)
		st.set_uv(p)
		st.set_uv2(Vector2(float(v[1]), 0.0))
		st.add_vertex(Vector3(p.x, top, p.y))


func _build_monuments() -> void:
	var mat := material(0, false, 0.0, meters_per_unit)
	var box_mat := material(0, true, 0.0, meters_per_unit)
	var kit := {}  # modèle → [xforms, bases, tints]
	var boxes := {}  # couche → [xforms, bases, tints]
	var tower_x: Array = []
	var tower_b := PackedFloat32Array()
	var block_x: Array = []
	var block_b := PackedFloat32Array()
	var all := manifest()
	var add_kit := func(model: String, xform: Transform3D, base: float) -> void:
		if model == "" or not all.has(model):
			return
		if not kit.has(model):
			kit[model] = [[], PackedFloat32Array(), PackedFloat32Array()]
		(kit[model][0] as Array).append(xform)
		kit[model][1].append(base)
		kit[model][2].append(0.5)
	var add_box := func(layer: String, xform: Transform3D, base: float) -> void:
		if not boxes.has(layer):
			boxes[layer] = [[], PackedFloat32Array(), PackedFloat32Array()]
		(boxes[layer][0] as Array).append(xform)
		boxes[layer][1].append(base)
		boxes[layer][2].append(0.5)
	for m in plan.get("monuments", []):
		var kind := str(m["kind"])
		var at := Vector3(float(m["x"]), 0.0, float(m["y"]))
		var yaw := float(m["yaw"])
		var d := Vector2(cos(yaw), sin(yaw))
		var side := Vector2(-d.y, d.x)
		var length := float(m["length"])
		var depth := float(m["depth"])
		var base := float(m["base"])
		match kind:
			"cathedral":
				var s := length / float(all.get("cathedral_0", {"length": 102.4})["length"])
				add_kit.call("cathedral_0", Transform3D(basis_x(d, Vector3(s, s, s)), at), base)
			"church":
				var model := "church_0" if length < 21.0 else ("church_1" if length < 30.0 else "church_2")
				var s2 := length / float(all.get(model, {"length": length})["length"])
				add_kit.call(model, Transform3D(basis_x(d, Vector3(s2, s2, s2)), at), base)
			"hall":
				add_kit.call("hall_0", Transform3D(basis_x(d), at), base)
			"windmill":
				add_kit.call("windmill_0", Transform3D(basis_x(d), at), base)
			"abbey":
				# Abbatiale au nord, cloître et bâtiments conventuels au sud, enclos.
				var church_len := length * 0.45
				var cs := church_len / float(all.get("church_2", {"length": 36.0})["length"])
				var church_at := at + Vector3(side.x, 0, side.y) * (-depth * 0.22)
				add_kit.call("church_2", Transform3D(basis_x(d, Vector3(cs, cs, cs)), church_at), base)
				var cl := at + Vector3(side.x, 0, side.y) * (depth * 0.12)
				var q := church_len * 0.5
				for k in 4:
					var ang := yaw + k * PI * 0.5
					var dd := Vector2(cos(ang), sin(ang))
					var off := Vector3(-dd.y, 0, dd.x) * q * 0.5
					block_x.append(Transform3D(basis_x(dd, Vector3(q + 9.0, 9.0, 9.0)), cl + off))
					block_b.append(base)
				_enclosure(add_box, at, d, length, depth, 4.0, 1.2, base, "Rubble")
			"castle":
				var half := length * 0.5
				_enclosure(add_box, at, d, length, length, 10.0, 2.5, base, "Masonry")
				for cx: float in [-1.0, 1.0]:
					for cz: float in [-1.0, 1.0]:
						var corner := at + Vector3(d.x, 0, d.y) * (half * cx) + Vector3(side.x, 0, side.y) * (half * cz)
						tower_x.append(Transform3D(Basis().scaled(Vector3(5.0, 16.0, 5.0)), corner))
						tower_b.append(base)
				if bool(m.get("keep", false)):
					var keep_at := at + Vector3(d.x, 0, d.y) * (half * 0.45) + Vector3(side.x, 0, side.y) * (-half * 0.45)
					tower_x.append(Transform3D(Basis().scaled(Vector3(8.0, 26.0, 8.0)), keep_at))
					tower_b.append(base)
				add_kit.call("manor_0", Transform3D(basis_x(d), at + Vector3(side.x, 0, side.y) * (half * 0.3)), base)
	for model in kit:
		var mesh := kit_mesh(model)
		if mesh != null:
			var g: Array = kit[model]
			_multimesh(mesh, g[0], g[1], g[2], mat, "all", 90.0).name = "Monument_" + model
	for layer in boxes:
		var b: Array = boxes[layer]
		_multimesh(box_mesh(layer), b[0], b[1], b[2], box_mat, "all", 20.0).name = "Enclosure_" + layer
	if not tower_x.is_empty():
		var tt := PackedFloat32Array()
		tt.resize(tower_x.size())
		tt.fill(0.5)
		_multimesh(tower_mesh(), tower_x, tower_b, tt, box_mat, "all", 40.0).name = "Keeps"
	if not block_x.is_empty():
		var bt := PackedFloat32Array()
		bt.resize(block_x.size())
		bt.fill(0.5)
		_multimesh(block_mesh(), block_x, block_b, bt, box_mat, "all", 12.0).name = "Cloister"
	# Pont : tablier de pierre et piles jusqu'à l'eau.
	var bridge: Dictionary = plan.get("bridge", {})
	if not bridge.is_empty():
		var byaw := float(bridge["yaw"])
		var bd := Vector2(cos(byaw), sin(byaw))
		var center := Vector3(float(bridge["x"]), 0, float(bridge["y"]))
		var deck := float(bridge["deck"])
		var water := float(bridge["water"])
		var blen := float(bridge["length"])
		add_box.call("Ashlar", Transform3D(basis_x(bd, Vector3(blen, 1.6, float(bridge["width"]))), center), deck - 1.5)
		var piers := maxi(1, int(blen / 15.0))
		for k in piers:
			var t := (float(k) + 0.5) / piers - 0.5
			var pc := center + Vector3(bd.x, 0, bd.y) * (t * blen * 0.8)
			add_box.call("Ashlar", Transform3D(basis_x(bd, Vector3(3.5, maxf(deck - water + 1.0, 2.0), float(bridge["width"]) + 1.0)), pc), water - 1.5)
		var bb: Array = boxes["Ashlar"]
		_multimesh(box_mesh("Ashlar"), bb[0], bb[1], bb[2], box_mat, "all", 12.0).name = "Bridge"


func _enclosure(add_box: Callable, at: Vector3, d: Vector2, length: float, depth: float, height: float, thickness: float, base: float, layer: String) -> void:
	var side := Vector2(-d.y, d.x)
	var hx := length * 0.5
	var hz := depth * 0.5
	var walls := [
		[Vector3(side.x, 0, side.y) * hz, d, length],
		[Vector3(side.x, 0, side.y) * -hz, d, length],
		[Vector3(d.x, 0, d.y) * hx, side, depth],
		[Vector3(d.x, 0, d.y) * -hx, side, depth],
	]
	for w in walls:
		add_box.call(layer, Transform3D(basis_x(w[1], Vector3(float(w[2]), height, thickness)), at + (w[0] as Vector3)), base)
