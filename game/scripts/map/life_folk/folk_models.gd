class_name FolkModels
extends RefCounted

## Chantier FK, lot FK3 (`docs/design/2026-09-29-carte-vivante-folk.md` § 2.2, § 2.3) : table des
## modèles de la carte vivante. Rendu seulement.
##
## - **Figurines** (`ROLES`) : figurines skinnées des batailles (`BattleSkinned`, texture d'os).
##   Chaque rôle liste des candidats [famille, variante] ; le premier présent dans le manifeste
##   `battle_skinned` l'emporte : villageois du lot FK2 (`villager_0` mains vides, `villager_1`
##   faucheurs, `villager_3` porteurs ; `villager_2` émeutiers réservés aux scènes FK4), sinon
##   servants d'engins (`crew`, sans armure) et miliciens.
## - **Activités** (`ACTIVITIES`) : villageois animés par `BattleSkinned.state_config` (états
##   `marching`, `plough`, `scythe`, `carry`, `idle` du style `folk`) ; autres figurines, jeu de
##   clips (absents du rig écartés). Vitesse de déplacement en m/s (0 : sur place).
## - **Accessoires** (`PROPS`) : modèles FK2 (`res://assets/models/folk/manifest.json`,
##   `models.<nom>.file` ; mètres, origine au sol, regard +X, tournés d'un quart de tour vers +Z,
##   sens du déplacement en shader), sinon maquette en boîtes (`_fallback_mesh`, regard +Z).
##   Emplacements des figurines autour d'un accessoire : `slots` du manifeste (`slot`).

const PROP_SHADER := preload("res://shaders/folk_prop.gdshader")
## LR-18 : nappe de crue translucide (matériau propre, bord adouci).
const FLOOD_SHADER := preload("res://shaders/folk_flood.gdshader")

## Rôle → candidats [famille, variante] (ordre de préférence), livrée et couleurs des habits.
const ROLES := {
	"peasant": {"figures": [["villager", 0], ["crew", 0], ["infantry", 2]], "livery": Color(0.42, 0.33, 0.22)},
	"peasant_b": {"figures": [["villager", 0], ["crew", 1], ["infantry", 3]], "livery": Color(0.36, 0.38, 0.3)},
	"reaper": {"figures": [["villager", 1], ["crew", 0], ["infantry", 2]], "livery": Color(0.45, 0.36, 0.24)},
	"porter": {"figures": [["villager", 3], ["crew", 1], ["infantry", 3]], "livery": Color(0.4, 0.3, 0.22)},
	"rioter": {"figures": [["villager", 2], ["infantry", 2], ["infantry", 5]], "livery": Color(0.38, 0.3, 0.2)},
	"merchant": {"figures": [["villager", 0], ["crew", 1], ["infantry", 5]], "livery": Color(0.35, 0.18, 0.12)},
	"pilgrim": {"figures": [["villager", 0], ["crew", 0], ["infantry", 2]], "livery": Color(0.3, 0.27, 0.24)},
	"guard": {"figures": [["infantry", 2], ["infantry", 5]], "livery": Color(0.5, 0.12, 0.1)},
	"rider": {"figures": [["cavalry", 1], ["cavalry", 0]], "livery": Color(0.33, 0.28, 0.2)},
	## FK4 : recrues à l'exercice (miliciens, livrée terne) et moines des processions.
	"recruit": {"figures": [["infantry", 2], ["infantry", 5], ["crew", 0]], "livery": Color(0.4, 0.36, 0.28)},
	"monk": {"figures": [["villager", 0], ["crew", 0], ["infantry", 2]], "livery": Color(0.2, 0.18, 0.17)},
}
## Habits ternes (laine écrue, brun, roux, gris) tirés par figurine (`plain_colors`).
const DRAB := [Color(0.55, 0.5, 0.4), Color(0.4, 0.3, 0.2), Color(0.5, 0.32, 0.2), Color(0.42, 0.42, 0.4)]

## Activité → état des villageois (`BattleSkinned.state_config`), clips des autres figurines
## (absents du rig écartés, repli sur `fallback`) et vitesse (m/s ; 0 = sur place).
const ACTIVITIES := {
	"walk": {"state": "marching", "clips": ["walk"], "fallback": ["idle"], "speed": 1.2},
	"guard_walk": {"state": "marching", "clips": ["pike_walk", "walk"], "fallback": ["walk"], "speed": 1.2},
	"ride": {"state": "marching", "clips": ["c_walk"], "fallback": ["c_idle"], "speed": 1.2},
	"plough": {"state": "plough", "clips": ["plough", "push"], "fallback": ["walk"], "speed": 0.8},
	"scythe": {"state": "scythe", "clips": ["scythe", "slash"], "fallback": ["idle"], "speed": 0.0},
	"harvest": {"state": "carry", "clips": ["carry", "haul", "push"], "fallback": ["walk"], "speed": 1.0},
	"chop": {"state": "scythe", "clips": ["overhead", "slash", "crank"], "fallback": ["idle"], "speed": 0.0},
	"herd": {"state": "idle", "clips": ["idle", "idle_look", "idle_lean"], "fallback": ["idle"], "speed": 0.0},
	"idle": {"state": "idle", "clips": ["idle", "idle_look", "idle_lean", "guard"], "fallback": ["idle"], "speed": 0.0},
	## FK4 : marche lente des processions, du convoi des morts et des fuyards (accessoires portés
	## à la même vitesse : `PROPS`), exercice des recrues.
	"procession": {"state": "marching", "clips": ["walk"], "fallback": ["idle"], "speed": 0.6},
	"drill": {"state": "idle", "clips": ["guard", "idle"], "fallback": ["idle"], "speed": 0.0},
}
## Familles animées par état (`BattleSkinned.state_config`, styles `folk` / `folk_carry` de FK2).
const STATE_KINDS := ["villager"]
const PROP_DIR := "res://assets/models/folk/"
const PROP_MANIFEST := PROP_DIR + "manifest.json"

## Accessoire → modèle FK2 (nom du manifeste), maquette de repli, vitesse de déplacement (m/s,
## celle des figurines qui l'accompagnent). Charrette de paysan : `stone_cart` (bœuf inclus ;
## FK2 n'a pas de charrette de paysan propre). Accessoires des scènes de province (FK4) : croix
## et bannière « portées » roulent à côté de leur porteur, à la vitesse `procession` ; fourche et
## torche (tenues en main) ne sont pas posées : le réservoir n'accroche rien à un os et les
## émeutiers `villager_2` les portent déjà. `flood_water` : nappe de crue (maquette seule).
const PROPS := {
	"merchant_cart": {"model": "merchant_cart", "fallback": "covered_cart", "speed": 1.2},
	"peasant_cart": {"model": "stone_cart", "fallback": "open_cart", "speed": 1.2},
	"plough": {"model": "plough", "fallback": "plough", "speed": 0.8},
	"sheep": {"model": "sheep", "fallback": "sheep", "speed": 0.0},
	"cow": {"model": "cow", "fallback": "cow", "speed": 0.0},
	"ox": {"model": "ox", "fallback": "cow", "speed": 0.0},
	"horse": {"model": "horse", "fallback": "cow", "speed": 0.0},
	"stone_cart": {"model": "stone_cart", "fallback": "open_cart", "speed": 1.2},
	"dead_cart": {"model": "dead_cart", "fallback": "open_cart", "speed": 0.6},
	"market_stall": {"model": "market_stall", "fallback": "stall", "speed": 0.0},
	"pyre": {"model": "pyre", "fallback": "pyre", "speed": 0.0},
	"scaffold": {"model": "scaffold", "fallback": "scaffold", "speed": 0.0},
	"procession_cross": {"model": "procession_cross", "fallback": "pole", "speed": 0.6},
	"procession_banner": {"model": "procession_banner", "fallback": "pole", "speed": 0.6},
	"flood_water": {"model": "", "fallback": "water", "speed": 0.0},
}

static var _manifest: Dictionary = {}

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
	if STATE_KINDS.has(kind):
		return BattleSkinned.state_config(kind, variant, str(entry["state"]), false)
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
	var path := model_path(str(entry.get("model", role)))
	if path != "" and ResourceLoader.exists(path):
		mesh = _mesh_from_scene(path)
		if mesh != null:
			_prop_sources[role] = path
			# Lot AS1 : allure des bêtes et roues des attelages (mesures de `animal_motion.json`).
			for s in mesh.get_surface_count():
				AnimalMotion.apply_prop(mesh.surface_get_material(s) as ShaderMaterial, str(entry.get("model", role)))
	if mesh == null:
		mesh = _fallback_mesh(str(entry.get("fallback", "open_cart")))
		_prop_sources[role] = "fallback:%s" % entry.get("fallback", "open_cart")
	_prop_meshes[role] = mesh
	return mesh


## Entrée `models.<nom>` du manifeste FK2 ({} si absent).
static func model_entry(model: String) -> Dictionary:
	if _manifest.is_empty():
		var parsed: Variant = DataFile.parse_file(PROP_MANIFEST) if FileAccess.file_exists(PROP_MANIFEST) else null
		_manifest = parsed if parsed is Dictionary else {"models": {}}
	var entry: Variant = (_manifest.get("models", {}) as Dictionary).get(model)
	return entry if entry is Dictionary else {}


## Chemin du glb d'un modèle FK2 (`file` du manifeste), vide si absent.
static func model_path(model: String) -> String:
	var file := str(model_entry(model).get("file", ""))
	return PROP_DIR + file if file != "" else ""


## Emplacement `slot` autour de l'accessoire `role` (manifeste FK2, repère du glb : +X devant,
## +Z à droite du sens de marche) en (latéral, avance) mètres pour `FolkPool.add`
## (`lateral_m` = latéral, `behind_m` = -avance) ; `fallback` (même convention) si absent.
static func slot(role: String, slot_name: String, fallback: Vector2) -> Vector2:
	var model := str((PROPS.get(role, {}) as Dictionary).get("model", role))
	var slots: Variant = model_entry(model).get("slots", {})
	if prop_source(role).begins_with("fallback:") or not (slots is Dictionary) or not (slots as Dictionary).has(slot_name):
		return fallback
	var p: Array = slots[slot_name]
	return Vector2(float(p[2]), float(p[0]))


static func clear_cache() -> void:
	_prop_meshes.clear()
	_prop_sources.clear()
	_manifest.clear()


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
	# Glb FK2 : regard +X ; le shader déplace le long de +Z (quart de tour : +X → +Z, +Z → -X).
	var facing := Transform3D(Basis(Vector3.UP, -PI * 0.5), Vector3.ZERO)
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
		xform = facing * xform
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
		"plough":
			_box(st, Vector3(0, 0.35, 0.9), Vector3(0.25, 0.25, 1.8), Color(0.42, 0.28, 0.16))
			_box(st, Vector3(0, 1.0, 4.0), Vector3(0.9, 0.9, 2.2), Color(0.5, 0.33, 0.2))
		"stall":
			_box(st, Vector3(0, 0.45, 0), Vector3(3.2, 0.9, 1.4), wood)
			_box(st, Vector3(0, 2.1, 0), Vector3(3.4, 0.08, 1.8), Color(0.7, 0.25, 0.18))
			for x in [-1.5, 1.5]:
				_box(st, Vector3(x, 1.1, -0.8), Vector3(0.1, 2.2, 0.1), dark)
		"pyre":
			_box(st, Vector3(0, 0.35, 0), Vector3(2.4, 0.7, 2.4), Color(0.3, 0.2, 0.12))
			_box(st, Vector3(0, 0.9, 0), Vector3(1.2, 0.5, 1.2), Color(0.15, 0.12, 0.1))
		"scaffold":
			for x in [-0.9, 2.9]:
				for z in [-2.2, 2.2]:
					_box(st, Vector3(x, 3.5, z), Vector3(0.15, 7.0, 0.15), wood)
			for y in [2.0, 3.9]:
				_box(st, Vector3(1.0, y, 0), Vector3(4.0, 0.1, 4.6), wood)
		"pole":
			_box(st, Vector3(0, 1.6, 0), Vector3(0.08, 3.2, 0.08), wood)
			_box(st, Vector3(0, 2.8, 0), Vector3(0.06, 0.8, 0.8), Color(0.75, 0.62, 0.3))
		"water":
			return _flood_mesh()
		"cow":
			_box(st, Vector3(0, 1.0, 0), Vector3(0.8, 0.8, 1.9), Color(0.5, 0.33, 0.2))
			_box(st, Vector3(0, 1.2, 1.15), Vector3(0.4, 0.4, 0.5), Color(0.45, 0.3, 0.18))
			_box(st, Vector3(0, 0.3, 0), Vector3(0.6, 0.6, 1.5), Color(0.3, 0.2, 0.12))
		_:
			_box(st, Vector3(0, 0.5, 0), Vector3(1, 1, 1), wood)
	var mesh := st.commit()
	mesh.surface_set_material(0, _prop_material(null, true))
	return mesh


## LR-18 : disque de crue (rayon moyen 15 m, contour irrégulier), alpha de sommet 1 au centre et
## 0 au bord ; posé 0,3 m au-dessus du point d'eau, le relief plus haut le coupe (et le shader
## efface la frange à l'affleurement).
static func _flood_mesh() -> ArrayMesh:
	const SEGMENTS := 28
	const RINGS := 4
	const RADIUS := 15.0
	var verts := PackedVector3Array([Vector3(0, 0.3, 0)])
	var colors := PackedColorArray([Color(1, 1, 1, 1)])
	var normals := PackedVector3Array([Vector3.UP])
	for ring in range(1, RINGS + 1):
		var t := float(ring) / float(RINGS)
		for i in SEGMENTS:
			var a := TAU * float(i) / float(SEGMENTS)
			var wobble := 0.78 + 0.22 * sin(a * 3.0 + 1.3) * cos(a * 5.0) + 0.1 * sin(a * 7.0 + 0.4)
			var r := RADIUS * t * wobble
			verts.append(Vector3(cos(a) * r, 0.3, sin(a) * r))
			colors.append(Color(1, 1, 1, 1.0 - t * t))
			normals.append(Vector3.UP)
	var indices := PackedInt32Array()
	for i in SEGMENTS:
		indices.append_array([0, 1 + (i + 1) % SEGMENTS, 1 + i])
	for ring in range(1, RINGS):
		var inner := 1 + (ring - 1) * SEGMENTS
		var outer := 1 + ring * SEGMENTS
		for i in SEGMENTS:
			var j := (i + 1) % SEGMENTS
			indices.append_array([inner + i, inner + j, outer + i, inner + j, outer + j, outer + i])
	var arrays := []
	arrays.resize(Mesh.ARRAY_MAX)
	arrays[Mesh.ARRAY_VERTEX] = verts
	arrays[Mesh.ARRAY_NORMAL] = normals
	arrays[Mesh.ARRAY_COLOR] = colors
	arrays[Mesh.ARRAY_INDEX] = indices
	var mesh := ArrayMesh.new()
	mesh.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES, arrays)
	var material := ShaderMaterial.new()
	material.shader = FLOOD_SHADER
	mesh.surface_set_material(0, material)
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
