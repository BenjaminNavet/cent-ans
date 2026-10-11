class_name CouncilRoom
extends Node3D

## Salle du conseil en 3D (RG13c, ADR 0341 § 4) : chambre de pierre sombre et intime, longue table
## de chêne, chandeliers de fer, un rai de jour froid par une haute fenêtre étroite, tapisseries
## aux couleurs de la faction. Rendu seulement : les charges viennent de `CouncilChamber`.
## Figures : `res://assets/models/council/council_<rôle>.glb` si présentes, sinon marionnette
## (capsule + sphère) à la couleur du rôle. Charge vacante : siège vide et étiquette fantôme.
## Caméra fixe au pied de la table, léger balancement.

const MODEL_DIR := "res://assets/models/council/"
const STONE_WALL := "res://assets/textures/buildings/stone_wall_diff.jpg"
const FLOOR_TEX := "res://assets/textures/battle/cobblestone_floor_01_diff.jpg"
const WOOD_TEX := "res://assets/textures/buildings/weathered_brown_planks_diff.jpg"

## Ordre des cinq sièges (gauche/droite de la table, du fond vers la caméra).
const SEAT_OFFICES: Array[String] = ["chancellor", "constable", "treasurer", "chaplain", "chamberlain"]
const SEAT_POSITIONS: Array[Vector3] = [
	Vector3(-1.3, 0.0, -1.2), Vector3(1.3, 0.0, -1.2), Vector3(-1.3, 0.0, 0.5),
	Vector3(1.3, 0.0, 0.5), Vector3(-1.3, 0.0, 2.1),
]
const RULER_POSITION := Vector3(0.0, 0.0, -3.0)
const ROLE_COLORS := {
	"ruler": Color(0.55, 0.12, 0.14), "chancellor": Color(0.25, 0.30, 0.55),
	"constable": Color(0.45, 0.45, 0.5), "treasurer": Color(0.55, 0.45, 0.15),
	"chaplain": Color(0.2, 0.2, 0.22), "chamberlain": Color(0.3, 0.5, 0.35),
}
const CAMERA_POSITION := Vector3(0.0, 2.1, 5.6)
const CAMERA_TARGET := Vector3(0.0, 1.05, -2.2)
const SWAY_AMPLITUDE := 0.05
const HIT_RADIUS := 0.5

var camera: Camera3D
var faction_color: Color = Color(0.6, 0.15, 0.15)
var faction_id: String = ""
## Figures par clé de rôle (`ruler` + les cinq charges) : `Node3D` ou null (charge vacante).
var figures: Dictionary = {}
var vacant_labels: Dictionary = {}
var candles: Array[OmniLight3D] = []
var window_light: SpotLight3D
var hovered_key: String = ""
var _figure_root: Node3D
var _time: float = 0.0
var _sway: bool = true


## Construit la salle ; à appeler une fois la salle dans l'arbre ou non (aucune dépendance).
func build(color: Color, faction: String = "") -> void:
	faction_color = color
	faction_id = faction
	_build_environment()
	_build_shell()
	_build_table()
	_build_candles()
	_build_window()
	_build_tapestries()
	_figure_root = Node3D.new()
	_figure_root.name = "Figures"
	add_child(_figure_root)
	camera = Camera3D.new()
	camera.name = "Camera"
	camera.fov = 52.0
	camera.position = CAMERA_POSITION
	add_child(camera)
	camera.look_at_from_position(CAMERA_POSITION, CAMERA_TARGET)
	camera.current = true


func set_sway(enabled: bool) -> void:
	_sway = enabled


func _process(delta: float) -> void:
	if camera == null or not _sway:
		return
	_time += delta
	var offset := Vector3(sin(_time * 0.21), sin(_time * 0.33) * 0.5, 0.0) * SWAY_AMPLITUDE
	camera.look_at_from_position(CAMERA_POSITION + offset, CAMERA_TARGET)
	for index in candles.size():  # Vacillement des flammes
		candles[index].light_energy = 1.7 + 0.25 * sin(_time * (7.0 + index) + index * 1.7)


# --- Décor ------------------------------------------------------------------------------------


func _material(texture_path: String, tint: Color, uv_scale: float, roughness: float = 0.9) -> StandardMaterial3D:
	var material := StandardMaterial3D.new()
	material.albedo_color = tint
	material.roughness = roughness
	if ResourceLoader.exists(texture_path):
		material.albedo_texture = load(texture_path) as Texture2D
	material.uv1_triplanar = true
	material.uv1_scale = Vector3.ONE * uv_scale
	return material


func _box(parent: Node, size: Vector3, position_: Vector3, material: Material, node_name: String = "") -> MeshInstance3D:
	var mesh := BoxMesh.new()
	mesh.size = size
	var instance := MeshInstance3D.new()
	instance.mesh = mesh
	instance.position = position_
	instance.material_override = material
	if node_name != "":
		instance.name = node_name
	parent.add_child(instance)
	return instance


func _build_environment() -> void:
	var environment := Environment.new()
	environment.background_mode = Environment.BG_COLOR
	environment.background_color = Color(0.02, 0.018, 0.016)
	environment.ambient_light_source = Environment.AMBIENT_SOURCE_COLOR
	environment.ambient_light_color = Color(0.16, 0.14, 0.13)
	environment.ambient_light_energy = 0.6
	environment.fog_enabled = true  # Brume basse et chaude, bon marché
	environment.fog_light_color = Color(0.16, 0.12, 0.09)
	environment.fog_density = 0.035
	environment.tonemap_mode = Environment.TONE_MAPPER_FILMIC
	environment.adjustment_enabled = true
	environment.adjustment_saturation = 0.78  # Palette terreuse, désaturée
	var world := WorldEnvironment.new()
	world.environment = environment
	add_child(world)


func _build_shell() -> void:
	var stone := _material(STONE_WALL, Color(0.55, 0.52, 0.5), 0.35)
	var floor_material := _material(FLOOR_TEX, Color(0.5, 0.47, 0.45), 0.3)
	_box(self, Vector3(8.0, 0.2, 14.0), Vector3(0, -0.1, -0.5), floor_material, "Floor")
	_box(self, Vector3(0.3, 5.0, 14.0), Vector3(-4.0, 2.4, -0.5), stone, "WallLeft")
	_box(self, Vector3(0.3, 5.0, 14.0), Vector3(4.0, 2.4, -0.5), stone, "WallRight")
	_box(self, Vector3(8.3, 5.0, 0.3), Vector3(0, 2.4, -7.2), stone, "WallBack")
	_box(self, Vector3(8.3, 5.0, 0.3), Vector3(0, 2.4, 6.7), stone, "WallFront")
	_box(self, Vector3(8.0, 0.3, 14.0), Vector3(0, 5.0, -0.5), stone, "Ceiling")
	# Colonnes engagées et nervures d'arc : voûte suggérée par des demi-cercles de poutres.
	for z_position in [-4.5, -1.0, 2.5]:
		for side in [-1.0, 1.0]:
			var column := MeshInstance3D.new()
			var cylinder := CylinderMesh.new()
			cylinder.top_radius = 0.28
			cylinder.bottom_radius = 0.32
			cylinder.height = 3.4
			column.mesh = cylinder
			column.material_override = stone
			column.position = Vector3(side * 3.6, 1.7, z_position)
			column.name = "Column"
			add_child(column)
		_build_arch(float(z_position), stone)
	# Cheminée au fond : braise discrète.
	_box(self, Vector3(2.2, 2.0, 0.5), Vector3(2.4, 1.0, -6.9), _material(STONE_WALL, Color(0.35, 0.33, 0.32), 0.5), "Hearth")
	var ember := OmniLight3D.new()
	ember.position = Vector3(2.4, 0.5, -6.4)
	ember.light_color = Color(1.0, 0.45, 0.15)
	ember.light_energy = 0.9
	ember.omni_range = 3.5
	add_child(ember)


func _build_arch(z_position: float, material: Material) -> void:
	var segments := 9
	var radius := 3.6
	for index in segments:
		var angle := PI * (float(index) + 0.5) / float(segments)
		var piece := _box(self, Vector3(0.45, 0.45, 0.5), Vector3(cos(angle) * radius, 3.4 + sin(angle) * 1.35, z_position), material, "ArchRib")
		piece.rotation.z = angle + PI * 0.5


func _build_table() -> void:
	var wood := _material(WOOD_TEX, Color(0.32, 0.22, 0.16), 0.6, 0.55)
	var top := _box(self, Vector3(1.8, 0.12, 7.0), Vector3(0, 0.92, -0.3), wood, "TableTop")
	var table := Node3D.new()
	table.name = "Table"
	add_child(table)
	if ResourceLoader.exists(MODEL_DIR + "council_table.glb"):
		var packed := load(MODEL_DIR + "council_table.glb") as PackedScene
		if packed != null:
			top.visible = false
			table.add_child(packed.instantiate())
	for x_position in [-0.75, 0.75]:
		for z_position in [-3.4, 2.8]:
			_box(self, Vector3(0.14, 0.9, 0.14), Vector3(x_position, 0.45, z_position), wood, "TableLeg")
	# Grande carte du royaume, parchemins épars.
	var parchment := StandardMaterial3D.new()
	parchment.albedo_color = Color(0.72, 0.62, 0.45)
	parchment.roughness = 0.95
	_box(self, Vector3(1.3, 0.01, 1.8), Vector3(0, 0.99, -0.2), parchment, "Map")
	var ink := StandardMaterial3D.new()
	ink.albedo_color = Color(0.25, 0.12, 0.08)
	_box(self, Vector3(0.9, 0.012, 0.02), Vector3(0.05, 0.996, -0.45), ink, "MapLine")
	_box(self, Vector3(0.02, 0.012, 0.7), Vector3(-0.2, 0.996, -0.2), ink, "MapLine")
	for scrap in [Vector3(-0.55, 0.0, 1.6), Vector3(0.5, 0.0, -2.0), Vector3(0.45, 0.0, 1.1), Vector3(-0.5, 0.0, -1.7)]:
		var sheet := _box(self, Vector3(0.32, 0.008, 0.42), Vector3(scrap.x, 0.989, scrap.z), parchment, "Scroll")
		sheet.rotation.y = scrap.x * 1.7 + scrap.z


func _build_candles() -> void:
	var iron := StandardMaterial3D.new()
	iron.albedo_color = Color(0.08, 0.08, 0.09)
	iron.metallic = 0.7
	iron.roughness = 0.5
	var flame := StandardMaterial3D.new()
	flame.albedo_color = Color(1.0, 0.7, 0.3)
	flame.emission_enabled = true
	flame.emission = Color(1.0, 0.6, 0.2)
	flame.emission_energy_multiplier = 3.0
	flame.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	var index := 0
	for z_position in [-2.3, -0.3, 1.7]:
		var stand := _box(self, Vector3(0.05, 0.4, 0.05), Vector3(0.0, 1.18, z_position), iron, "Candelabrum")
		_box(stand, Vector3(0.4, 0.03, 0.05), Vector3(0, 0.2, 0), iron, "CandelabrumArm")
		for dx in [-0.18, 0.0, 0.18]:
			var sphere := SphereMesh.new()
			sphere.radius = 0.025
			sphere.height = 0.07
			var tip := MeshInstance3D.new()
			tip.mesh = sphere
			tip.material_override = flame
			tip.position = Vector3(dx, 0.26, 0)
			stand.add_child(tip)
		var light := OmniLight3D.new()
		light.name = "Candle%d" % index
		light.position = Vector3(0.0, 1.55, z_position)
		light.light_color = Color(1.0, 0.62, 0.32)  # ~2400 K
		light.light_energy = 1.7
		light.omni_range = 5.0
		light.shadow_enabled = index != 1  # Ombres douces sur deux flammes seulement
		light.omni_attenuation = 1.4
		add_child(light)
		candles.append(light)
		index += 1


func _build_window() -> void:
	var glass := StandardMaterial3D.new()
	glass.albedo_color = Color(0.7, 0.8, 1.0)
	glass.emission_enabled = true
	glass.emission = Color(0.55, 0.7, 1.0)
	glass.emission_energy_multiplier = 1.6
	glass.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	_box(self, Vector3(0.05, 2.6, 0.5), Vector3(-3.83, 2.6, -2.2), glass, "Window")
	window_light = SpotLight3D.new()
	window_light.name = "WindowLight"
	window_light.position = Vector3(-3.6, 3.4, -2.2)
	window_light.light_color = Color(0.6, 0.72, 1.0)  # Jour froid
	window_light.light_energy = 5.0
	window_light.spot_range = 9.0
	window_light.spot_angle = 24.0
	window_light.shadow_enabled = true
	add_child(window_light)
	window_light.look_at_from_position(window_light.position, Vector3(0.4, 0.9, -1.0))


func _build_tapestries() -> void:
	var texture := PortraitLoader.heraldry_texture(faction_id)
	for spec in [[Vector3(0.0, 2.7, -7.0), 0.0, Vector2(1.3, 2.4)], [Vector3(3.82, 2.5, -2.5), -PI * 0.5, Vector2(1.1, 2.0)],
			[Vector3(3.82, 2.5, 1.5), -PI * 0.5, Vector2(1.1, 2.0)], [Vector3(-3.82, 2.5, 3.0), PI * 0.5, Vector2(1.1, 2.0)]]:
		var quad := MeshInstance3D.new()
		var mesh := QuadMesh.new()
		mesh.size = spec[2]
		quad.mesh = mesh
		var cloth := StandardMaterial3D.new()
		cloth.albedo_color = faction_color.darkened(0.45)
		cloth.roughness = 1.0
		cloth.cull_mode = BaseMaterial3D.CULL_DISABLED
		quad.material_override = cloth
		quad.position = spec[0]
		quad.rotation.y = spec[1]
		quad.name = "Tapestry"
		add_child(quad)
		if texture != null:
			var emblem := MeshInstance3D.new()
			var emblem_mesh := QuadMesh.new()
			emblem_mesh.size = Vector2(0.8, 0.8)
			emblem.mesh = emblem_mesh
			var emblem_material := StandardMaterial3D.new()
			emblem_material.albedo_texture = texture
			emblem_material.albedo_color = Color(0.75, 0.75, 0.75)
			emblem_material.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA
			emblem.material_override = emblem_material
			emblem.position = Vector3(0, 0.1, 0.01)
			quad.add_child(emblem)


# --- Figures ----------------------------------------------------------------------------------


## Clés des sièges : `ruler` puis les charges dans l'ordre de `SEAT_OFFICES`.
func seat_keys() -> Array[String]:
	var keys: Array[String] = ["ruler"]
	keys.append_array(SEAT_OFFICES)
	return keys


func seat_position(key: String) -> Vector3:
	if key == "ruler":
		return RULER_POSITION
	var index := SEAT_OFFICES.find(key)
	return SEAT_POSITIONS[index] if index >= 0 else Vector3.ZERO


## Place les figures : `occupied` = clé → bool (le souverain est toujours présent).
func populate(occupied: Dictionary) -> void:
	for child in _figure_root.get_children():
		child.queue_free()
		_figure_root.remove_child(child)
	figures.clear()
	vacant_labels.clear()
	hovered_key = ""
	for key in seat_keys():
		var position_ := seat_position(key)
		_add_seat_furniture(key, position_)
		if key != "ruler" and not bool(occupied.get(key, false)):
			figures[key] = null
			vacant_labels[key] = _add_vacant_label(position_)
			continue
		var figure := _make_figure(key)
		figure.position = position_
		figure.name = "Figure_" + key
		_figure_root.add_child(figure)
		figure.rotation.y = _facing(position_, Vector3(0.0, 0.0, position_.z) if key != "ruler" else Vector3(0.0, 0.0, 4.0))
		figures[key] = figure


func _add_seat_furniture(key: String, position_: Vector3) -> void:
	var path := MODEL_DIR + ("council_throne.glb" if key == "ruler" else "council_stool.glb")
	var node: Node3D = null
	if ResourceLoader.exists(path):
		var packed := load(path) as PackedScene
		if packed != null:
			node = packed.instantiate() as Node3D
	if node == null:
		node = Node3D.new()
		var wood := _material(WOOD_TEX, Color(0.25, 0.17, 0.12), 0.8)
		if key == "ruler":
			_box(node, Vector3(0.8, 0.5, 0.7), Vector3(0, 0.25, 0), wood, "Seat")
			_box(node, Vector3(0.8, 1.7, 0.12), Vector3(0, 0.85, -0.38), wood, "Back")
		else:
			_box(node, Vector3(0.5, 0.45, 0.5), Vector3(0, 0.225, 0), wood, "Seat")
	node.position = position_
	node.name = "Seat_" + key
	if key == "ruler":
		node.rotation.y = 0.0
		node.position = position_ + Vector3(0, 0, -0.1)
	else:
		node.rotation.y = _facing(position_, Vector3(0.0, 0.0, position_.z))
	_figure_root.add_child(node)


## Lacet pour que l'avant (-Z) de la figure en `from` regarde `to` (modèles : face vers -Z).
static func _facing(from: Vector3, to: Vector3) -> float:
	return atan2(-(to.x - from.x), -(to.z - from.z))


func _add_vacant_label(position_: Vector3) -> Label3D:
	var label := Label3D.new()
	label.text = "Charge vacante"
	label.font_size = 40
	label.pixel_size = 0.006
	label.modulate = Color(0.85, 0.8, 0.7, 0.45)
	label.outline_size = 6
	label.billboard = BaseMaterial3D.BILLBOARD_ENABLED
	label.no_depth_test = true
	label.position = position_ + Vector3(0, 1.1, 0)
	label.name = "Vacant"
	_figure_root.add_child(label)
	return label


func _make_figure(key: String) -> Node3D:
	var path := MODEL_DIR + "council_%s.glb" % key
	if ResourceLoader.exists(path):
		var packed := load(path) as PackedScene
		if packed != null:
			var model := packed.instantiate() as Node3D
			if model != null:
				var holder := Node3D.new()
				holder.add_child(model)
				return holder
	return _placeholder(key)


func _placeholder(key: String) -> Node3D:
	var color: Color = ROLE_COLORS.get(key, Color(0.4, 0.4, 0.4))
	if key == "ruler":
		color = faction_color.darkened(0.2)
	var root := Node3D.new()
	var body_material := StandardMaterial3D.new()
	body_material.albedo_color = color
	body_material.roughness = 0.8
	var body := MeshInstance3D.new()
	var capsule := CapsuleMesh.new()
	capsule.radius = 0.26
	capsule.height = 0.9
	body.mesh = capsule
	body.material_override = body_material
	body.position = Vector3(0, 0.95, 0)
	root.add_child(body)
	var head := MeshInstance3D.new()
	var sphere := SphereMesh.new()
	sphere.radius = 0.15
	sphere.height = 0.3
	head.mesh = sphere
	var skin := StandardMaterial3D.new()
	skin.albedo_color = Color(0.72, 0.58, 0.46)
	head.material_override = skin
	head.position = Vector3(0, 1.62, 0)
	root.add_child(head)
	if key == "ruler":  # Couronne
		var crown := MeshInstance3D.new()
		var ring := CylinderMesh.new()
		ring.top_radius = 0.14
		ring.bottom_radius = 0.14
		ring.height = 0.08
		crown.mesh = ring
		var gold := StandardMaterial3D.new()
		gold.albedo_color = Color(0.85, 0.68, 0.2)
		gold.metallic = 0.8
		gold.roughness = 0.35
		crown.material_override = gold
		crown.position = Vector3(0, 1.8, 0)
		root.add_child(crown)
	return root


# --- Sélection --------------------------------------------------------------------------------


## Clé du siège touché par le rayon (sièges vacants compris), `""` sinon.
func pick_ray(origin: Vector3, direction: Vector3) -> String:
	var best := ""
	var best_distance := INF
	var ray := direction.normalized()
	for key in seat_keys():
		var base := seat_position(key)
		for height in [0.6, 1.0, 1.4]:
			var center := base + Vector3(0.0, height, 0.0)
			var to_center := center - origin
			var along := to_center.dot(ray)
			if along <= 0.0:
				continue
			var miss := (to_center - ray * along).length()
			if miss < HIT_RADIUS * (0.7 if height > 1.2 else 1.0) and along < best_distance:
				best_distance = along
				best = key
	return best


func pick_screen(position_: Vector2) -> String:
	if camera == null:
		return ""
	return pick_ray(camera.project_ray_origin(position_), camera.project_ray_normal(position_))


## Surbrillance (liseré chaud) de la figure `key` ; `""` l'éteint.
func set_hover(key: String) -> void:
	if key == hovered_key:
		return
	if figures.get(hovered_key) is Node3D:
		_set_overlay(figures[hovered_key], null)
	hovered_key = key
	if figures.get(key) is Node3D:
		_set_overlay(figures[key], _highlight_material())


func _highlight_material() -> StandardMaterial3D:
	var material := StandardMaterial3D.new()
	material.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	material.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA
	material.blend_mode = BaseMaterial3D.BLEND_MODE_ADD
	material.albedo_color = Color(0.9, 0.62, 0.25, 0.28)
	material.emission_enabled = true
	material.emission = Color(1.0, 0.7, 0.3)
	return material


func _set_overlay(node: Node, material: Material) -> void:
	if node is MeshInstance3D:
		(node as MeshInstance3D).material_overlay = material
	for child in node.get_children():
		_set_overlay(child, material)


## Position d'écran approximative de la tête de la figure `key` (pour l'infobulle).
func head_screen_position(key: String) -> Vector2:
	if camera == null:
		return Vector2.ZERO
	return camera.unproject_position(seat_position(key) + Vector3(0, 1.8, 0))
