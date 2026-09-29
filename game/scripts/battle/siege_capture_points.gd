class_name SiegeCapturePoints
extends Node3D

## T4 (TW2, ADR 0108) : points de capture d'une bataille de siège, façon Total War. Sur chaque
## point (place du marché = point de victoire, porte côté ville) : un drapeau planté au sol aux
## couleurs du camp qui le tient et un cercle au sol du rayon du point, qui vire à la couleur de
## l'assaillant à mesure qu'il le prend ; au-dessus, une barre de capture enluminée (même style
## que les barres de vie SB, `SiegeHealthBars.Bar`) avec « Place du marché : 24/60 s ». La barre
## n'apparaît que lorsque la prise est entamée ou disputée. Aucune règle ici : rayon, progression,
## durée et statut viennent de `get_siege().points` (cœur `sim-battle/src/capture.rs`).

const NAMES := {"square": "Place du marché", "gate": "Porte"}
const POLE_HEIGHT := 9.0
const FLAG_SIZE := Vector2(3.2, 2.0)
## Hauteur de la barre au-dessus du sol (m) : au-dessus du drapeau.
const BAR_LIFT := 12.0
const DEFAULT_COLORS := {"attacker": Color(0.20, 0.33, 0.62), "defender": Color(0.62, 0.13, 0.08)}

var side_colors: Dictionary = DEFAULT_COLORS.duplicate()
var height_at: Callable

var _flags: Dictionary = {}  # kind -> {root, flag_mat, ring_mat}
var _layer: CanvasLayer
var _bars: Dictionary = {}  # kind -> SiegeHealthBars.Bar
var _state: Dictionary = {}  # kind -> état (voir `states`)


func _init() -> void:
	name = "SiegeCapturePoints"
	_layer = CanvasLayer.new()
	_layer.name = "CaptureBars"
	_layer.layer = 2
	add_child(_layer)


## États des points tirés de `get_siege()` (fonction pure, testée) : kind -> {x, z, ground, radius,
## share, holder: « attacker »|« defender », shown (barre), text, contested, world (barre)}.
static func states(siege: Dictionary, ground: Callable) -> Dictionary:
	var out := {}
	for point in siege.get("points", []):
		var kind := str(point.get("kind", "square"))
		var x := float(point.get("x", 0.0))
		var z := float(point.get("z", 0.0))
		var g := float(ground.call(x, z)) if ground.is_valid() else 0.0
		var status := str(point.get("status", "held"))
		var progress := float(point.get("progress", 0.0))
		var hold := maxf(float(point.get("hold_s", 1.0)), 0.001)
		var share := clampf(float(point.get("share", progress / hold)), 0.0, 1.0)
		var taken := status == "taken"
		var text := "%s : %d/%d s" % [NAMES.get(kind, kind), floori(progress), roundi(hold)]
		if taken:
			text = "%s prise" % NAMES.get(kind, kind)
		out[kind] = {
			"x": x,
			"z": z,
			"ground": g,
			"radius": float(point.get("radius", 25.0)),
			"share": share,
			"holder": "attacker" if taken else "defender",
			"contested": status == "contested" or status == "capturing",
			"shown": taken or share > 0.0 or status == "contested" or status == "capturing",
			"text": text,
			"world": Vector3(x, g + BAR_LIFT, z),
		}
	return out


## Met drapeaux, cercles et barres à jour depuis `get_siege()` (appelé par `BattleSiege.update`).
func sync(siege: Dictionary) -> void:
	_state = states(siege, height_at)
	for kind in _state:
		var s: Dictionary = _state[kind]
		if not _flags.has(kind):
			_flags[kind] = _make_flag(kind, s)
		var view: Dictionary = _flags[kind]
		var defender: Color = side_colors.get("defender", DEFAULT_COLORS["defender"])
		var attacker: Color = side_colors.get("attacker", DEFAULT_COLORS["attacker"])
		(view["flag_mat"] as StandardMaterial3D).albedo_color = attacker if s["holder"] == "attacker" else defender
		var ring := defender.lerp(attacker, float(s["share"]))
		ring.a = 0.55 if s["contested"] else 0.35
		(view["ring_mat"] as StandardMaterial3D).albedo_color = ring
		var bar: SiegeHealthBars.Bar = _bars.get(kind)
		if bar == null and s["shown"]:
			bar = SiegeHealthBars.Bar.new()
			bar.name = "Capture_" + str(kind)
			_layer.add_child(bar)
			_bars[kind] = bar
		if bar != null:
			if absf(bar.ratio - float(s["share"])) > 0.001 or bar.fill != attacker:
				bar.ratio = float(s["share"])
				bar.fill = attacker
				bar.queue_redraw()
			bar.caption.text = s["text"]
	_place()


## État d'un point (tests) : {} si inconnu.
func state(kind: String) -> Dictionary:
	return _state.get(kind, {})


## Barre de capture affichée pour `kind` (null si jamais créée).
func bar(kind: String) -> Control:
	return _bars.get(kind)


## Drapeau planté pour `kind` (null si absent).
func flag(kind: String) -> Node3D:
	return _flags.get(kind, {}).get("root")


func _process(_delta: float) -> void:
	_place()


func _make_flag(kind: String, s: Dictionary) -> Dictionary:
	var root := Node3D.new()
	root.name = "Flag_" + kind
	root.position = Vector3(float(s["x"]), float(s["ground"]), float(s["z"]))
	add_child(root)
	var pole := MeshInstance3D.new()
	var pole_mesh := CylinderMesh.new()
	pole_mesh.top_radius = 0.08
	pole_mesh.bottom_radius = 0.12
	pole_mesh.height = POLE_HEIGHT
	pole.mesh = pole_mesh
	pole.position.y = POLE_HEIGHT * 0.5
	pole.material_override = BattleSiege._material(BattleSiege.WOOD_DARK)
	root.add_child(pole)
	var flag_mat := StandardMaterial3D.new()
	flag_mat.cull_mode = BaseMaterial3D.CULL_DISABLED
	flag_mat.roughness = 0.9
	var cloth := MeshInstance3D.new()
	var quad := QuadMesh.new()
	quad.size = FLAG_SIZE
	cloth.mesh = quad
	cloth.name = "Cloth"
	cloth.position = Vector3(FLAG_SIZE.x * 0.5 + 0.1, POLE_HEIGHT - FLAG_SIZE.y * 0.5 - 0.2, 0.0)
	cloth.material_override = flag_mat
	root.add_child(cloth)
	# Cercle au sol du rayon du point : un tore aplati, sans ombre ni éclairage.
	var ring_mat := StandardMaterial3D.new()
	ring_mat.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	ring_mat.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA
	ring_mat.no_depth_test = false
	var ring := MeshInstance3D.new()
	var torus := TorusMesh.new()
	var radius := float(s["radius"])
	torus.inner_radius = maxf(radius - 0.8, 0.1)
	torus.outer_radius = radius
	torus.rings = 48
	ring.mesh = torus
	ring.name = "Ring"
	ring.scale = Vector3(1.0, 0.15, 1.0)
	ring.position.y = 0.25
	ring.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	ring.material_override = ring_mat
	root.add_child(ring)
	return {"root": root, "flag_mat": flag_mat, "ring_mat": ring_mat}


## Place chaque barre au-dessus de son drapeau (projection caméra, taille fixe à l'écran).
func _place() -> void:
	var viewport := get_viewport()
	var camera: Camera3D = viewport.get_camera_3d() if viewport != null else null
	var screen := viewport.get_visible_rect() if viewport != null else Rect2()
	for kind in _bars:
		var bar_node: SiegeHealthBars.Bar = _bars[kind]
		var s: Dictionary = _state.get(kind, {})
		if s.is_empty() or not s["shown"] or camera == null:
			bar_node.visible = false
			continue
		var world: Vector3 = s["world"]
		if camera.is_position_behind(world):
			bar_node.visible = false
			continue
		var point := camera.unproject_position(world)
		if not screen.grow(SiegeHealthBars.BAR_SIZE.x).has_point(point):
			bar_node.visible = false
			continue
		bar_node.visible = true
		bar_node.position = (point - SiegeHealthBars.BAR_SIZE * 0.5).round()
		bar_node.caption.visible = true
