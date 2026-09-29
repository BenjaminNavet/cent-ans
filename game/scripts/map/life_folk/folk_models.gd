class_name FolkModels
extends RefCounted

## Chantier FK, lot FK3 (`docs/design/2026-09-29-carte-vivante-folk.md` § 2.2, § 2.3) : table des
## modèles de la carte vivante. Rendu seulement.
##
## - **Figurines** (`ROLES`) : figurines skinnées des batailles (`BattleSkinned`, texture d'os).
##   Chaque rôle liste des candidats [famille, variante] ; le premier présent dans le manifeste
##   `battle_skinned` l'emporte. Les figurines civiles du lot FK2 se branchent en tête de liste
##   (`civilian_*`) ; en attendant, maquettes provisoires : servants d'engins (`crew`, sans
##   armure) et miliciens.
## - **Activités** (`ACTIVITIES`) : jeu de clips (les clips absents du rig sont écartés : les
##   clips `plough`, `scythe`, `carry` du lot FK2 prennent le relais dès qu'ils existent) et
##   vitesse de marche (m/s, unités du modèle).
## - **Accessoires** (`PROPS`) : modèles `res://assets/models/folk/*.glb` du lot FK2 (premier
##   chemin présent), sinon maquette en boîtes (`_fallback_mesh`). Unités : mètres.

const PROP_SHADER := preload("res://shaders/folk_prop.gdshader")

## Rôle → candidats [famille, variante] (ordre de préférence), livrée et couleurs des habits.
const ROLES := {
	"peasant": {"figures": [["civilian", 0], ["crew", 0], ["infantry", 2]], "livery": Color(0.42, 0.33, 0.22)},
	"peasant_b": {"figures": [["civilian", 1], ["crew", 1], ["infantry", 3]], "livery": Color(0.36, 0.38, 0.3)},
	"merchant": {"figures": [["civilian", 2], ["crew", 1], ["infantry", 5]], "livery": Color(0.35, 0.18, 0.12)},
	"pilgrim": {"figures": [["civilian", 3], ["crew", 0], ["infantry", 2]], "livery": Color(0.3, 0.27, 0.24)},
	"guard": {"figures": [["infantry", 2], ["infantry", 5]], "livery": Color(0.5, 0.12, 0.1)},
	"rider": {"figures": [["cavalry", 1], ["cavalry", 0]], "livery": Color(0.33, 0.28, 0.2)},
}
## Habits ternes (laine écrue, brun, roux, gris) tirés par figurine (`plain_colors`).
const DRAB := [Color(0.55, 0.5, 0.4), Color(0.4, 0.3, 0.2), Color(0.5, 0.32, 0.2), Color(0.42, 0.42, 0.4)]

## Activité → clips (les absents du rig sont écartés, repli sur `fallback`) et vitesse (m/s).
const ACTIVITIES := {
	"walk": {"clips": ["walk"], "fallback": ["idle"], "speed": 1.2},
	"guard_walk": {"clips": ["pike_walk", "walk"], "fallback": ["walk"], "speed": 1.2},
	"ride": {"clips": ["c_walk"], "fallback": ["c_idle"], "speed": 1.2},
	"plough": {"clips": ["plough", "push"], "fallback": ["walk"], "speed": 0.0},
	"scythe": {"clips": ["scythe", "slash"], "fallback": ["idle"], "speed": 0.0},
	"harvest": {"clips": ["carry", "haul", "push"], "fallback": ["idle"], "speed": 0.0},
	"chop": {"clips": ["overhead", "slash", "crank"], "fallback": ["idle"], "speed": 0.0},
	"herd": {"clips": ["idle", "idle_look", "idle_lean"], "fallback": ["idle"], "speed": 0.0},
	"idle": {"clips": ["idle", "idle_look", "idle_lean", "guard"], "fallback": ["idle"], "speed": 0.0},
}

## Accessoire → chemins candidats (lot FK2), maquette de repli, vitesse de roulage (m/s).
const PROPS := {
	"merchant_cart": {"paths": ["res://assets/models/folk/merchant_cart.glb"], "fallback": "covered_cart", "speed": 1.2},
	"peasant_cart": {"paths": ["res://assets/models/folk/peasant_cart.glb", "res://assets/models/folk/stone_cart.glb"], "fallback": "open_cart", "speed": 1.2},
	"sheep": {"paths": ["res://assets/models/folk/sheep.glb", "res://assets/models/folk/herd_sheep.glb"], "fallback": "sheep", "speed": 0.0},
	"cow": {"paths": ["res://assets/models/folk/cow.glb", "res://assets/models/folk/herd_cows.glb"], "fallback": "cow", "speed": 0.0},
}

static var _prop_meshes: Dictionary = {}
static var _prop_sources: Dictionary = {}


static func is_prop(role: String) -> bool:
	return PROPS.has(role)


## [famille, variante] de la figurine du rôle, ou [] si aucune n'existe.
static func figure_of(role: String) -> Array:
	var entry: Dictionary = ROLES.get(role, ROLES["peasant"])
	for candidate in entry["figures"]:
		if BattleSkinned.has_figure(str(candidate[0]), int(candidate[1])):
			return [str(candidate[0]), int(candidate[1])]
	return []


static func livery_of(role: String) -> Color:
	return (ROLES.get(role, ROLES["peasant"]) as Dictionary)["livery"]


## Vitesse (m/s) d'une activité ou d'un accessoire qui roule.
static func speed_of(role: String, activity: String) -> float:
	if PROPS.has(role):
		return float(PROPS[role]["speed"])
	return float((ACTIVITIES.get(activity, ACTIVITIES["idle"]) as Dictionary)["speed"])


## Configuration d'animation (format `BattleSkinned.apply_config`) d'une activité.
static func activity_config(kind: String, variant: int, activity: String) -> Dictionary:
	var entry: Dictionary = ACTIVITIES.get(activity, ACTIVITIES["idle"])
	var rig_entry := BattleSkinned.rig(kind, variant)
	var names := BattleSkinned._present(rig_entry, entry["clips"])
	if names.is_empty():
		names = BattleSkinned._present(rig_entry, entry["fallback"])
	if names.is_empty():
		names = ["c_idle"] if kind == "cavalry" else ["idle"]
	var ids: Array[int] = []
	for c in names:
		ids.append(BattleSkinned.clip_index(rig_entry, str(c)))
	return {"key": "fk/%s/%d/%s" % [kind, variant, activity], "names": names, "set": ids, "mode": BattleSkinned.M_LOOP, "speed": 1.0, "cycle": 1.5, "release": 1.0}


## Source du modèle d'accessoire retenue (chemin FK2 ou `fallback:<nom>`), pour les tests.
static func prop_source(role: String) -> String:
	prop_mesh(role)
	return str(_prop_sources.get(role, ""))


## Maillage de l'accessoire (surfaces aux matériaux `folk_prop.gdshader`), mis en cache.
static func prop_mesh(role: String) -> ArrayMesh:
	if _prop_meshes.has(role):
		return _prop_meshes[role]
	var entry: Dictionary = PROPS.get(role, {})
	var mesh: ArrayMesh = null
	for path in entry.get("paths", []):
		if ResourceLoader.exists(str(path)):
			mesh = _mesh_from_scene(str(path))
			if mesh != null:
				_prop_sources[role] = str(path)
				break
	if mesh == null:
		mesh = _fallback_mesh(str(entry.get("fallback", "open_cart")))
		_prop_sources[role] = "fallback:%s" % entry.get("fallback", "open_cart")
	_prop_meshes[role] = mesh
	return mesh


static func clear_cache() -> void:
	_prop_meshes.clear()
	_prop_sources.clear()


## Fusionne les maillages d'une scène glTF (transformations appliquées) en un seul maillage ;
## chaque surface reçoit le matériau de déplacement, avec la couleur et la texture d'origine.
static func _mesh_from_scene(path: String) -> ArrayMesh:
	var packed := load(path) as PackedScene
	if packed == null:
		return null
	var root := packed.instantiate() as Node3D
	if root == null:
		return null
	var out := ArrayMesh.new()
	for node in root.find_children("*", "MeshInstance3D", true, false):
		var instance := node as MeshInstance3D
		if instance.mesh == null:
			continue
		var xform := Transform3D.IDENTITY
		var walk: Node = instance
		while walk != null and walk != root:
			if walk is Node3D:
				xform = (walk as Node3D).transform * xform
			walk = walk.get_parent()
		for s in instance.mesh.get_surface_count():
			var arrays := instance.mesh.surface_get_arrays(s)
			var verts: PackedVector3Array = arrays[Mesh.ARRAY_VERTEX]
			for i in verts.size():
				verts[i] = xform * verts[i]
			arrays[Mesh.ARRAY_VERTEX] = verts
			if arrays[Mesh.ARRAY_NORMAL] is PackedVector3Array:
				var normals: PackedVector3Array = arrays[Mesh.ARRAY_NORMAL]
				for i in normals.size():
					normals[i] = (xform.basis * normals[i]).normalized()
				arrays[Mesh.ARRAY_NORMAL] = normals
			arrays[Mesh.ARRAY_TANGENT] = null
			for k in [Mesh.ARRAY_BONES, Mesh.ARRAY_WEIGHTS, Mesh.ARRAY_CUSTOM0, Mesh.ARRAY_CUSTOM1, Mesh.ARRAY_CUSTOM2, Mesh.ARRAY_CUSTOM3]:
				arrays[k] = null
			out.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES, arrays)
			var source := instance.get_active_material(s)
			out.surface_set_material(out.get_surface_count() - 1, _prop_material(source, arrays[Mesh.ARRAY_COLOR] is PackedColorArray))
	root.free()
	return out if out.get_surface_count() > 0 else null


static func _prop_material(source: Material, has_colors: bool) -> ShaderMaterial:
	var material := ShaderMaterial.new()
	material.shader = PROP_SHADER
	var base := source as BaseMaterial3D
	if base != null:
		material.set_shader_parameter("albedo", base.albedo_color)
		if base.albedo_texture != null:
			material.set_shader_parameter("albedo_tex", base.albedo_texture)
			material.set_shader_parameter("use_tex", true)
		material.set_shader_parameter("use_vertex_color", base.vertex_color_use_as_albedo)
	else:
		material.set_shader_parameter("use_vertex_color", has_colors)
	return material


## Maquettes provisoires en boîtes (couleurs par sommet), en mètres, regard +Z.
static func _fallback_mesh(kind: String) -> ArrayMesh:
	var st := SurfaceTool.new()
	st.begin(Mesh.PRIMITIVE_TRIANGLES)
	var wood := Color(0.42, 0.28, 0.16)
	var dark := Color(0.22, 0.15, 0.1)
	match kind:
		"covered_cart":
			_box(st, Vector3(0, 0.75, 0), Vector3(1.4, 0.35, 2.6), wood)
			_box(st, Vector3(0, 1.4, -0.1), Vector3(1.35, 0.95, 2.2), Color(0.86, 0.82, 0.7))
			_wheels(st, dark)
			_box(st, Vector3(0, 0.7, 1.9), Vector3(0.12, 0.1, 1.4), wood)
		"open_cart":
			_box(st, Vector3(0, 0.7, 0), Vector3(1.3, 0.3, 2.2), wood)
			_box(st, Vector3(0, 1.0, -0.2), Vector3(1.1, 0.35, 1.4), Color(0.72, 0.62, 0.35))
			_wheels(st, dark)
			_box(st, Vector3(0, 0.65, 1.7), Vector3(0.12, 0.1, 1.2), wood)
		"sheep":
			_box(st, Vector3(0, 0.55, 0), Vector3(0.55, 0.5, 1.0), Color(0.88, 0.86, 0.8))
			_box(st, Vector3(0, 0.7, 0.6), Vector3(0.25, 0.25, 0.3), Color(0.2, 0.18, 0.16))
			_box(st, Vector3(0, 0.15, 0), Vector3(0.4, 0.3, 0.7), Color(0.2, 0.18, 0.16))
		"cow":
			_box(st, Vector3(0, 1.0, 0), Vector3(0.8, 0.8, 1.9), Color(0.5, 0.33, 0.2))
			_box(st, Vector3(0, 1.2, 1.15), Vector3(0.4, 0.4, 0.5), Color(0.45, 0.3, 0.18))
			_box(st, Vector3(0, 0.3, 0), Vector3(0.6, 0.6, 1.5), Color(0.3, 0.2, 0.12))
		_:
			_box(st, Vector3(0, 0.5, 0), Vector3(1, 1, 1), wood)
	var mesh := st.commit()
	mesh.surface_set_material(0, _prop_material(null, true))
	return mesh


static func _wheels(st: SurfaceTool, color: Color) -> void:
	for side in [-0.75, 0.75]:
		for z in [-0.7, 0.7]:
			_box(st, Vector3(side, 0.45, z), Vector3(0.1, 0.9, 0.9), color)


static func _box(st: SurfaceTool, center: Vector3, size: Vector3, color: Color) -> void:
	var h := size * 0.5
	var corners := []
	for i in 8:
		corners.append(center + Vector3(h.x if i & 1 else -h.x, h.y if i & 2 else -h.y, h.z if i & 4 else -h.z))
	# Faces (sens anti-horaire vu de l'extérieur).
	# Normales explicites (matériau sans élagage des faces arrière : sens des faces indifférent).
	var faces := [[0, 2, 3, 1], [4, 5, 7, 6], [0, 1, 5, 4], [2, 6, 7, 3], [0, 4, 6, 2], [1, 3, 7, 5]]
	var normals := [Vector3.FORWARD, Vector3.BACK, Vector3.DOWN, Vector3.UP, Vector3.LEFT, Vector3.RIGHT]
	st.set_color(color)
	for n in faces.size():
		var f: Array = faces[n]
		st.set_normal(normals[n])
		for tri in [[f[0], f[1], f[2]], [f[0], f[2], f[3]]]:
			for k in tri:
				st.add_vertex(corners[k])
