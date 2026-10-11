class_name CouncilRoom
extends Node3D

## Salle du conseil en 3D (RG13c, ADR 0341 § 4) : chambre de pierre sombre et intime, longue table
## de chêne, chandeliers de fer, un rai de jour froid par une haute fenêtre étroite, tapisseries
## aux couleurs de la faction. Rendu seulement : les charges viennent de `CouncilChamber`.
## Figures : `res://assets/models/council/council_<rôle>.glb` si présentes, sinon marionnette
## (capsule + sphère) à la couleur du rôle. Charge vacante : siège vide et étiquette fantôme.
## Caméra fixe au pied de la table, léger balancement.

const MODEL_DIR := "res://assets/models/council/"
const MULTIVIEW_DIR := "res://assets/models/council/multiview/"
const HANDMADE_DIR := "res://assets/models/council/handmade/"
const PAINTED_DIR := "res://assets/illustrations/council/"
const CATALOG_PATH := "art/council_catalog.json"
## Variantes de rendu des figures (RG13-V23) : modèles image→3D, idem multivues, images détourées.
const STYLES: Array[String] = ["mesh", "mesh_multiview", "painted"]
const DEFAULT_STYLE := "painted"
const PAINTED_HEIGHT := 1.3
const PAINTED_BASE := Color(0.82, 0.8, 0.78)
const PAINTED_HOVER := Color(1.25, 1.1, 0.9)
const STONE_WALL := "res://assets/textures/buildings/stone_wall_diff.jpg"
const FLOOR_TEX := "res://assets/textures/battle/cobblestone_floor_01_diff.jpg"
const WOOD_TEX := "res://assets/textures/buildings/weathered_brown_planks_diff.jpg"

## Composition frontale (tableau) : table en travers face à la caméra, tous assis en rang derrière,
## de face ; le souverain au centre devant son trône, le chancelier à sa droite.
const SEAT_OFFICES: Array[String] = ["chancellor", "constable", "treasurer", "chaplain", "chamberlain"]
const SEAT_POSITIONS: Array[Vector3] = [
	Vector3(-0.85, 0.0, -0.8), Vector3(0.85, 0.0, -0.8), Vector3(-1.7, 0.0, -0.8),
	Vector3(1.7, 0.0, -0.8), Vector3(-2.55, 0.0, -0.8),
]
const RULER_POSITION := Vector3(0.0, 0.0, -1.0)
## Centre de la table et de la caméra (le rang est plus long à gauche : chambellan au bout).
const ROW_CENTER_X := -0.4
const TABLE_LENGTH := 4.6
const TABLE_WIDTH := 1.1
const TABLE_HEIGHT := 0.85
const ROLE_COLORS := {
	"ruler": Color(0.55, 0.12, 0.14), "chancellor": Color(0.25, 0.30, 0.55),
	"constable": Color(0.45, 0.45, 0.5), "treasurer": Color(0.55, 0.45, 0.15),
	"chaplain": Color(0.2, 0.2, 0.22), "chamberlain": Color(0.3, 0.5, 0.35),
}
const CAMERA_POSITION := Vector3(ROW_CENTER_X, 1.5, 3.6)
const CAMERA_TARGET := Vector3(ROW_CENTER_X, 1.0, -0.8)
const SWAY_AMPLITUDE := 0.05
const HIT_RADIUS := 0.35

var camera: Camera3D
var faction_color: Color = Color(0.6, 0.15, 0.15)
var faction_id: String = ""
## Figures par clé de rôle (`ruler` + les cinq charges) : `Node3D` ou null (charge vacante).
var figures: Dictionary = {}
var vacant_labels: Dictionary = {}
var candles: Array[OmniLight3D] = []
var window_light: SpotLight3D
var hovered_key: String = ""
## Style demandé (argument `--council-style=`, sinon `figure_style` du catalogue, sinon `mesh`) et style
## effectivement rendu (`mesh_multiview` retombe sur `mesh` si le dossier est absent).
var requested_style: String = DEFAULT_STYLE
var style: String = DEFAULT_STYLE
static var _warned_missing: bool = false
## Style imposé par le code (tests), prioritaire sur l'argument et le catalogue ; `""` = aucun.
static var style_override: String = ""
var _figure_root: Node3D
var _time: float = 0.0
var _sway: bool = true


## Construit la salle ; à appeler une fois la salle dans l'arbre ou non (aucune dépendance).
func build(color: Color, faction: String = "") -> void:
	faction_color = color
	faction_id = faction
	requested_style = resolve_style()
	style = _effective_style(requested_style)
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
	_box(self, Vector3(8.3, 5.0, 0.3), Vector3(0, 2.4, -4.6), stone, "WallBack")
	_box(self, Vector3(8.3, 5.0, 0.3), Vector3(0, 2.4, 6.7), stone, "WallFront")
	_box(self, Vector3(8.0, 0.3, 14.0), Vector3(0, 5.0, -0.5), stone, "Ceiling")
	# Colonnes engagées et nervures d'arc : voûte suggérée par des demi-cercles de poutres.
	for z_position in [-3.5, -0.5, 2.5]:
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
	_box(self, Vector3(2.2, 2.0, 0.5), Vector3(2.4, 1.0, -4.3), _material(STONE_WALL, Color(0.35, 0.33, 0.32), 0.5), "Hearth")
	var ember := OmniLight3D.new()
	ember.position = Vector3(2.4, 0.5, -3.8)
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
	var top := _box(self, Vector3(TABLE_LENGTH, 0.08, TABLE_WIDTH), Vector3(ROW_CENTER_X, TABLE_HEIGHT - 0.04, 0.0), wood, "TableTop")
	var table := Node3D.new()
	table.name = "Table"
	table.position = Vector3(ROW_CENTER_X, 0.0, 0.0)
	table.rotation.y = PI * 0.5  # Modèles : longueur sur Z -> en travers (X)
	add_child(table)
	var model := _load_model("council_table.glb")
	if model != null:
		top.visible = false
		model.scale = Vector3(1.0, 1.0, TABLE_LENGTH / 3.2)  # Modèles de 3,2 m
		table.add_child(model)
	else:
		for x_position in [-TABLE_LENGTH * 0.45, TABLE_LENGTH * 0.45]:
			for z_position in [-0.48, 0.48]:
				_box(self, Vector3(0.12, TABLE_HEIGHT, 0.12), Vector3(ROW_CENTER_X + x_position, TABLE_HEIGHT * 0.5, z_position), wood, "TableLeg")
	# Grande carte du royaume, parchemins épars.
	var parchment := StandardMaterial3D.new()
	parchment.albedo_color = Color(0.72, 0.62, 0.45)
	parchment.roughness = 0.95
	var y_top := TABLE_HEIGHT + 0.005
	_box(self, Vector3(1.3, 0.01, 0.75), Vector3(ROW_CENTER_X + 0.4, y_top, 0.05), parchment, "Map")
	var ink := StandardMaterial3D.new()
	ink.albedo_color = Color(0.25, 0.12, 0.08)
	_box(self, Vector3(0.5, 0.012, 0.02), Vector3(0.1, y_top + 0.006, 0.0), ink, "MapLine")
	_box(self, Vector3(0.02, 0.012, 0.45), Vector3(-0.05, y_top + 0.006, 0.05), ink, "MapLine")
	for scrap in [Vector3(-2.2, 0.0, 0.2), Vector3(1.6, 0.0, 0.25), Vector3(-1.3, 0.0, 0.3), Vector3(-2.6, 0.0, -0.15)]:
		var sheet := _box(self, Vector3(0.22, 0.008, 0.3), Vector3(scrap.x, y_top, scrap.z), parchment, "Scroll")
		sheet.rotation.y = scrap.x * 1.7 + scrap.z


## Style demandé : argument utilisateur `--council-style=<v>`, sinon `figure_style` du catalogue.
static func resolve_style() -> String:
	if style_override in STYLES:
		return style_override
	for argument in OS.get_cmdline_user_args():
		if argument.begins_with("--council-style="):
			var value := argument.substr("--council-style=".length())
			if value in STYLES:
				return value
			push_warning("CouncilRoom: style inconnu « %s », catalogue utilisé" % value)
	var catalog: Variant = DataFile.read_json(CATALOG_PATH)
	if catalog is Dictionary and str((catalog as Dictionary).get("figure_style", "")) in STYLES:
		return str(catalog["figure_style"])
	return DEFAULT_STYLE


## `mesh_multiview` sans modèles dans son dossier : repli sur `mesh`, avertissement unique.
static func _effective_style(wanted: String) -> String:
	if wanted == "mesh_multiview" and not ResourceLoader.exists(MULTIVIEW_DIR + "council_ruler.glb"):
		if not _warned_missing:
			_warned_missing = true
			push_warning("CouncilRoom: %s absent, repli sur le style mesh" % MULTIVIEW_DIR)
		return "mesh"
	return wanted


## Dossiers de modèles consultés pour le mobilier du style courant, par ordre de priorité.
func _furniture_dirs() -> Array[String]:
	match style:
		"painted":
			return [HANDMADE_DIR, MODEL_DIR]
		"mesh_multiview":
			return [MULTIVIEW_DIR, MODEL_DIR]
	return [MODEL_DIR]


func _load_model(file: String) -> Node3D:
	for directory in _furniture_dirs():
		var path := directory + file
		if ResourceLoader.exists(path):
			var packed := load(path) as PackedScene
			return packed.instantiate() as Node3D if packed != null else null
	return null


## Chemin du modèle de la figure `key` dans le style courant (repli sur `mesh`), `""` si absent.
func _figure_glb_path(key: String) -> String:
	var directories: Array[String] = [MODEL_DIR]
	if style == "mesh_multiview":
		directories.push_front(MULTIVIEW_DIR)
	for directory in directories:
		var path := directory + "council_%s.glb" % key
		if ResourceLoader.exists(path):
			return path
	return ""


## Vrai si la figure de `key` existe dans le style courant (image détourée ou modèle).
func _figure_available(key: String) -> bool:
	if style == "painted":
		return ResourceLoader.exists(PAINTED_DIR + "council_%s.png" % key)
	return _figure_glb_path(key) != ""


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
	for candle_spot in [Vector2(-2.1, 0.3), Vector2(-0.42, 0.3), Vector2(1.27, 0.3)]:
		var z_position: float = candle_spot.y
		var stand := _box(self, Vector3(0.04, 0.24, 0.04), Vector3(candle_spot.x, 0.97, z_position), iron, "Candelabrum")
		_box(stand, Vector3(0.3, 0.025, 0.04), Vector3(0, 0.12, 0), iron, "CandelabrumArm")
		for dx in [-0.18, 0.0, 0.18]:
			var sphere := SphereMesh.new()
			sphere.radius = 0.025
			sphere.height = 0.07
			var tip := MeshInstance3D.new()
			tip.mesh = sphere
			tip.material_override = flame
			tip.position = Vector3(dx * 0.75, 0.17, 0)
			stand.add_child(tip)
		var light := OmniLight3D.new()
		light.name = "Candle%d" % index
		light.position = Vector3(candle_spot.x, 1.3, z_position + 0.2)
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
	glass.albedo_color = Color(0.35, 0.42, 0.55)
	glass.emission_enabled = true
	glass.emission = Color(0.55, 0.7, 1.0)
	glass.emission_energy_multiplier = 0.5
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
	for spec in [[Vector3(0.0, 2.6, -4.43), 0.0, Vector2(1.3, 2.4)], [Vector3(3.82, 2.5, -2.5), -PI * 0.5, Vector2(1.1, 2.0)],
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
		if key != "ruler" and not bool(occupied.get(key, false)):
			_add_seat_furniture(key, position_, false)
			figures[key] = null
			vacant_labels[key] = _add_vacant_label(position_)
			continue
		var figure := _make_figure(key)
		_add_seat_furniture(key, position_, _figure_available(key))
		figure.position = position_
		figure.name = "Figure_" + key
		_figure_root.add_child(figure)
		figure.rotation.y = _facing(position_, Vector3(position_.x, 0.0, 4.0))
		figures[key] = figure


## Mobilier d'un siège : tabouret pour une charge vacante ou une marionnette ; les figures GLB
## portent leur propre banc. Le souverain n'a pas de trône sous lui : le trône sert de fond.
func _add_seat_furniture(key: String, position_: Vector3, has_model: bool) -> void:
	var node: Node3D = null
	if key == "ruler":
		node = _load_model("council_throne.glb")
		if node == null:
			return
		node.position = position_ + Vector3(0, 0, -0.3)
		node.name = "Throne"
		_figure_root.add_child(node)
		return
	if has_model:
		return
	node = _load_model("council_stool.glb")
	if node == null:
		node = Node3D.new()
		var wood := _material(WOOD_TEX, Color(0.25, 0.17, 0.12), 0.8)
		_box(node, Vector3(0.45, 0.45, 0.45), Vector3(0, 0.225, 0), wood, "Seat")
	node.position = position_
	node.name = "Seat_" + key
	node.rotation.y = _facing(position_, Vector3(position_.x, 0.0, 4.0))
	_figure_root.add_child(node)


## Lacet pour que l'avant (-Z) de la figure en `from` regarde `to` (modèles : face vers -Z).
static func _facing(from: Vector3, to: Vector3) -> float:
	return atan2(-(to.x - from.x), -(to.z - from.z))


func _add_vacant_label(position_: Vector3) -> Label3D:
	var label := Label3D.new()
	label.text = "Charge vacante"
	label.font_size = 28
	label.pixel_size = 0.004
	label.modulate = Color(0.85, 0.8, 0.7, 0.6)
	label.outline_size = 6
	label.billboard = BaseMaterial3D.BILLBOARD_ENABLED
	label.position = position_ + Vector3(0, 1.05, 0)
	label.name = "Vacant"
	_figure_root.add_child(label)
	return label


func _make_figure(key: String) -> Node3D:
	if style == "painted" and _figure_available(key):
		return _painted_figure(key)
	var path := _figure_glb_path(key) if style != "painted" else ""
	if path != "":
		var packed := load(path) as PackedScene
		var model := packed.instantiate() as Node3D if packed != null else null
		if model != null:
			var holder := Node3D.new()
			holder.add_child(model)
			return holder
	return _placeholder(key)


## Figure peinte : image détourée sur un quad vertical qui fait face à la caméra (billboard Y),
## pieds au sol, hauteur `PAINTED_HEIGHT`, petite ombre floue au sol.
func _painted_figure(key: String) -> Node3D:
	var texture := load(PAINTED_DIR + "council_%s.png" % key) as Texture2D
	var root := Node3D.new()
	var aspect := float(texture.get_width()) / float(maxi(texture.get_height(), 1))
	var quad := MeshInstance3D.new()
	var mesh := QuadMesh.new()
	mesh.size = Vector2(PAINTED_HEIGHT * aspect, PAINTED_HEIGHT)
	quad.mesh = mesh
	var material := StandardMaterial3D.new()
	material.albedo_texture = texture
	material.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA_SCISSOR
	material.alpha_scissor_threshold = 0.5
	material.billboard_mode = BaseMaterial3D.BILLBOARD_FIXED_Y
	material.roughness = 1.0
	material.albedo_color = PAINTED_BASE
	material.texture_filter = BaseMaterial3D.TEXTURE_FILTER_LINEAR_WITH_MIPMAPS_ANISOTROPIC
	quad.material_override = material
	quad.position = Vector3(0.0, PAINTED_HEIGHT * 0.5, 0.0)
	quad.name = "Painted"
	quad.set_meta("painted", true)
	quad.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	root.add_child(quad)
	root.add_child(_ground_shadow(aspect))
	return root


## Décalque sombre et flou (dégradé radial) sous une figure peinte.
func _ground_shadow(aspect: float) -> MeshInstance3D:
	var gradient := Gradient.new()
	gradient.set_color(0, Color(0, 0, 0, 0.6))
	gradient.set_color(1, Color(0, 0, 0, 0.0))
	var texture := GradientTexture2D.new()
	texture.gradient = gradient
	texture.fill = GradientTexture2D.FILL_RADIAL
	texture.fill_from = Vector2(0.5, 0.5)
	texture.fill_to = Vector2(1.0, 0.5)
	texture.width = 64
	texture.height = 64
	var mesh := QuadMesh.new()
	mesh.size = Vector2(PAINTED_HEIGHT * aspect * 1.1, 0.7)
	mesh.orientation = PlaneMesh.FACE_Y
	var material := StandardMaterial3D.new()
	material.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	material.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA
	material.albedo_texture = texture
	var shadow := MeshInstance3D.new()
	shadow.mesh = mesh
	shadow.material_override = material
	shadow.position = Vector3(0.0, 0.012, 0.0)
	shadow.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	shadow.name = "GroundShadow"
	return shadow


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
	var best_miss := INF
	var ray := direction.normalized()
	for key in seat_keys():
		var base := seat_position(key)
		for height in [0.5, 0.9, 1.2]:
			var center := base + Vector3(0.0, height, 0.0)
			var to_center := center - origin
			var along := to_center.dot(ray)
			if along <= 0.0:
				continue
			var miss := (to_center - ray * along).length()
			if miss < HIT_RADIUS * (0.7 if height > 1.2 else 1.0) and miss < best_miss:  # Le plus proche de l'axe
				best_miss = miss
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
	if node is MeshInstance3D and node.has_meta("painted"):  # Quad détouré : éclaircir plutôt que superposer
		((node as MeshInstance3D).material_override as StandardMaterial3D).albedo_color = (
			PAINTED_BASE if material == null else PAINTED_HOVER
		)
		return
	if node is MeshInstance3D:
		(node as MeshInstance3D).material_overlay = material
	for child in node.get_children():
		_set_overlay(child, material)


## Position d'écran approximative de la tête de la figure `key` (pour l'infobulle).
func head_screen_position(key: String) -> Vector2:
	if camera == null:
		return Vector2.ZERO
	return camera.unproject_position(seat_position(key) + Vector3(0, 1.8, 0))
