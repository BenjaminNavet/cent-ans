class_name BattleDroppedArms
extends Node3D

## Lot EP12 (ADR 0070) : armes et boucliers laissés au sol par les fuyards et les blessés.
## Rendu seulement : la déroute est décidée par le cœur (état `routing` du régiment) ; ici, un
## `MultiMeshInstance3D` par sorte d'objet (épée, bouclier, arme d'hast, arc, arbalète), tampon
## circulaire plafonné (`dropped_arms` de `data/fx/battle_gore.json`) : au-delà, l'objet le plus
## ancien de la même sorte est réutilisé. Maillages bâtis ici (quelques boîtes, couleurs de
## sommet) : ils ne se voient que de près, masqués au-delà de `far_m`.
## Tirage déterministe : même régiment et même rang, mêmes objets (hachage, pas de hasard).

const KINDS := ["sword", "shield", "polearm", "bow", "crossbow"]
## Objets lâchés selon le style d'animation de la figurine (`BattleSkinned.style_of`) : sorte,
## part des figurines qui en laissent un.
const BY_STYLE := {
	"sword": [["sword", 0.8], ["shield", 0.5]],
	"militia": [["polearm", 0.9]],
	"pike": [["polearm", 0.95]],
	"bow": [["bow", 0.8]],
	"crossbow": [["crossbow", 0.85]],
}
const STEEL := Color(0.62, 0.63, 0.66)
const DARK_STEEL := Color(0.35, 0.36, 0.38)
const WOOD := Color(0.42, 0.29, 0.17)
const LEATHER := Color(0.3, 0.2, 0.12)

var dropped_count: int = 0
var _layers: Dictionary = {}  # sorte -> {mm, data, next, capacity, instance, dirty}
var _config: Dictionary = {}


## `config` : section `dropped_arms` des réglages de rendu des morts.
func setup(config: Dictionary) -> void:
	_config = config
	var per_kind := int(config.get("max_per_kind", 400))
	var far := float(config.get("far_m", 220.0))
	for kind in KINDS:
		var mm := MultiMesh.new()
		mm.transform_format = MultiMesh.TRANSFORM_3D
		mm.use_colors = true
		mm.mesh = _build_mesh(kind)
		mm.instance_count = per_kind
		mm.visible_instance_count = 0
		var data := PackedFloat32Array()
		data.resize(per_kind * 16)
		data.fill(0.0)
		var instance := MultiMeshInstance3D.new()
		instance.name = "Dropped_%s" % kind
		instance.multimesh = mm
		instance.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
		instance.visibility_range_end = far
		instance.visibility_range_end_margin = 20.0
		instance.visibility_range_fade_mode = GeometryInstance3D.VISIBILITY_RANGE_FADE_SELF
		instance.custom_aabb = AABB(Vector3(-1.0e5, -1.0e3, -1.0e5), Vector3(2.0e5, 2.0e3, 2.0e5))
		add_child(instance)
		_layers[kind] = {"mm": mm, "data": data, "next": 0, "count": 0, "capacity": per_kind, "instance": instance, "dirty": false}


## Objets lâchés par les figurines d'un régiment qui se débande. `slice` : tranche du tampon
## de soldats (12 flottants par figurine), `n` figurines, `style` d'animation, `livery` pour
## les boucliers, `seed` (id du régiment) pour le tirage.
func drop_from_rout(slice: PackedFloat32Array, n: int, style: String, livery: Color, seed: int) -> void:
	var items: Array = BY_STYLE.get(style, [])
	if items.is_empty() or n <= 0:
		return
	var cap := mini(n, int(_config.get("max_per_rout", 80)))
	var step := float(n) / float(cap)
	for j in cap:
		var slot := mini(int(j * step), n - 1)
		var o := slot * 12
		var at := Vector3(slice[o + 3], slice[o + 7], slice[o + 11])
		var facing := atan2(slice[o + 2], slice[o + 10])
		_drop_items(items, at, facing, livery, seed * 7919 + slot)


## Arme lâchée par un blessé (EP12) à l'endroit où il tombe.
func drop_one(style: String, at: Vector3, facing: float, livery: Color, seed: int) -> void:
	var items: Array = BY_STYLE.get(style, [])
	if not items.is_empty():
		_drop_items([items[0]], at, facing, livery, seed)


func _drop_items(items: Array, at: Vector3, facing: float, livery: Color, seed: int) -> void:
	var scatter := float(_config.get("scatter_m", 0.8))
	for k in items.size():
		var entry: Array = items[k]
		var h := hash4(seed * 31 + k)
		if h.x >= float(entry[1]):
			continue
		# Jetés devant les pieds, un peu de côté, à plat dans n'importe quel sens.
		var offset := Vector3((h.y - 0.5) * scatter, 0.0, (h.z - 0.5) * scatter)
		var yaw := facing + h.w * TAU
		var roll := (h.y - 0.5) * 0.3
		var basis := Basis(Vector3.UP, yaw) * Basis(Vector3.FORWARD, roll)
		var color := Color(1, 1, 1)
		if str(entry[0]) == "shield":
			color = livery
			# Bouclier retourné une fois sur deux : face peinte contre terre.
			if h.z > 0.5:
				basis = basis * Basis(Vector3.FORWARD, PI)
				color = Color(1, 1, 1)
		_place(str(entry[0]), Transform3D(basis, at + offset + Vector3(0, 0.03, 0)), color)


func _place(kind: String, xform: Transform3D, color: Color) -> void:
	var layer: Dictionary = _layers.get(kind, {})
	if layer.is_empty():
		return
	var data: PackedFloat32Array = layer["data"]
	var slot: int = layer["next"]
	var o := slot * 16
	var b := xform.basis
	var values := [b.x.x, b.y.x, b.z.x, xform.origin.x, b.x.y, b.y.y, b.z.y, xform.origin.y, b.x.z, b.y.z, b.z.z, xform.origin.z, color.r, color.g, color.b, 1.0]
	for q in 16:
		data[o + q] = values[q]
	layer["data"] = data
	layer["next"] = (slot + 1) % int(layer["capacity"])
	layer["count"] = mini(int(layer["count"]) + 1, int(layer["capacity"]))
	layer["dirty"] = true
	dropped_count += 1


## Envoi au GPU une fois par image et par sorte (des centaines d'objets par déroute).
func _process(_delta: float) -> void:
	for kind in _layers:
		var layer: Dictionary = _layers[kind]
		if not bool(layer["dirty"]):
			continue
		layer["dirty"] = false
		var mm: MultiMesh = layer["mm"]
		mm.buffer = layer["data"]
		mm.visible_instance_count = int(layer["count"])


## Nombre d'objets au sol (toutes sortes, plafond compris).
func shown_count() -> int:
	var total := 0
	for kind in _layers:
		total += int(_layers[kind]["count"])
	return total


## Quatre valeurs pseudo-aléatoires stables dans [0, 1) pour un entier.
static func hash4(n: int) -> Vector4:
	var x := float(n)
	return Vector4(
		fposmod(sin(x * 12.9898 + 1.31) * 43758.5453, 1.0),
		fposmod(sin(x * 78.233 + 2.17) * 24634.6345, 1.0),
		fposmod(sin(x * 39.425 + 0.71) * 35714.1123, 1.0),
		fposmod(sin(x * 93.989 + 3.37) * 51872.7719, 1.0))


# --- Maillages (à plat sur le sol, longueur selon +Z) ------------------------------------


func _build_mesh(kind: String) -> ArrayMesh:
	var st := SurfaceTool.new()
	st.begin(Mesh.PRIMITIVE_TRIANGLES)
	match kind:
		"sword":
			_box(st, Vector3(0, 0.008, 0.25), Vector3(0.022, 0.006, 0.4), STEEL)
			_box(st, Vector3(0, 0.012, -0.16), Vector3(0.11, 0.012, 0.014), DARK_STEEL)
			_box(st, Vector3(0, 0.012, -0.24), Vector3(0.016, 0.014, 0.07), LEATHER)
			_box(st, Vector3(0, 0.014, -0.32), Vector3(0.024, 0.02, 0.022), DARK_STEEL)
		"shield":
			# Écu : face peinte (couleur d'instance) sur une planche de bois, pointe vers +Z.
			_box(st, Vector3(0, 0.02, -0.06), Vector3(0.25, 0.014, 0.2), Color(1, 1, 1))
			_box(st, Vector3(0, 0.02, 0.2), Vector3(0.17, 0.014, 0.08), Color(1, 1, 1))
			_box(st, Vector3(0, 0.02, 0.32), Vector3(0.07, 0.014, 0.05), Color(1, 1, 1))
			_box(st, Vector3(0, 0.004, 0.02), Vector3(0.27, 0.006, 0.32), WOOD)
		"polearm":
			_box(st, Vector3(0, 0.02, 0.0), Vector3(0.018, 0.018, 1.1), WOOD)
			_box(st, Vector3(0, 0.02, 1.22), Vector3(0.03, 0.01, 0.14), STEEL)
		"bow":
			# Arc détendu, légèrement courbé (trois segments).
			_box(st, Vector3(0, 0.012, 0.0), Vector3(0.018, 0.012, 0.3), WOOD)
			_box_rot(st, Vector3(0.05, 0.012, 0.58), Vector3(0.014, 0.01, 0.3), 0.18, WOOD)
			_box_rot(st, Vector3(0.05, 0.012, -0.58), Vector3(0.014, 0.01, 0.3), -0.18, WOOD)
		"crossbow":
			_box(st, Vector3(0, 0.03, 0.0), Vector3(0.03, 0.03, 0.4), WOOD)
			_box(st, Vector3(0, 0.03, 0.34), Vector3(0.36, 0.015, 0.02), DARK_STEEL)
			_box(st, Vector3(0, 0.02, 0.44), Vector3(0.05, 0.01, 0.06), DARK_STEEL)
	st.generate_normals()
	var mesh := st.commit()
	var mat := StandardMaterial3D.new()
	mat.vertex_color_use_as_albedo = true
	mat.roughness = 0.72
	mat.metallic = 0.15
	mesh.surface_set_material(0, mat)
	return mesh


func _box(st: SurfaceTool, center: Vector3, half: Vector3, color: Color) -> void:
	_box_rot(st, center, half, 0.0, color)


## Boîte de demi-tailles `half` centrée en `center`, tournée de `yaw` autour de Y.
func _box_rot(st: SurfaceTool, center: Vector3, half: Vector3, yaw: float, color: Color) -> void:
	var basis := Basis(Vector3.UP, yaw)
	var corners: Array[Vector3] = []
	for i in 8:
		var local := Vector3(half.x if i & 1 else -half.x, half.y if i & 2 else -half.y, half.z if i & 4 else -half.z)
		corners.append(center + basis * local)
	# Faces (sommets dans le sens des aiguilles d'une montre vus de l'extérieur, convention Godot).
	var faces := [[0, 1, 3, 2], [4, 6, 7, 5], [0, 4, 5, 1], [2, 3, 7, 6], [0, 2, 6, 4], [1, 5, 7, 3]]
	st.set_color(color)
	for f in faces:
		var a: Vector3 = corners[f[0]]
		var b: Vector3 = corners[f[1]]
		var c: Vector3 = corners[f[2]]
		var d: Vector3 = corners[f[3]]
		st.add_vertex(a)
		st.add_vertex(b)
		st.add_vertex(c)
		st.add_vertex(a)
		st.add_vertex(c)
		st.add_vertex(d)
