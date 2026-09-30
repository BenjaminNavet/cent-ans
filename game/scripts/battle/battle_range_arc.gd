class_name BattleRangeArc
extends Node3D

## Lot CB-M4 : portée au sol façon Total War. Pour chaque tireur sélectionné ou survolé, un arc
## tracé à sa portée effective (`effective_range` de `get_units`, 0 sans tir ni munitions) dans le
## secteur de tir centré sur son cap (`fire_arc`, demi-angle en radians, `battle_hover.json`),
## avec les deux bords du secteur en plus pâle. Plaqué au relief : chaque sommet prend la hauteur
## de marche du cœur (`get_walk_height` : sol, pont, chemin de ronde). Aucun chiffre de règle ici :
## portée et secteur viennent du cœur ; seules les dimensions du tracé sont locales.

const MAX_ARCS := 8  # au-delà (grosse sélection d'archers), les premiers seulement
const ARC_WIDTH := 1.4  # épaisseur du trait de l'arc (m)
const EDGE_WIDTH := 0.7  # épaisseur des bords du secteur (m)
const LIFT := 0.35  # au-dessus du sol, contre le scintillement
const SEGMENT_M := 4.0  # longueur visée d'un segment de l'arc (m)
const EDGE_STEP_M := 6.0  # pas des sommets des bords, pour suivre le relief (m)
const ALPHA_SELECTED := 0.85
const ALPHA_HOVERED := 0.5
const EDGE_ALPHA := 0.45  # part de l'alpha de l'arc

var _battle: Object = null
var _colors: Dictionary = {}
var _player_side := "attacker"
var _nodes: Array[MeshInstance3D] = []
var _signatures: Array[String] = []
## Identifiants des tireurs dont l'arc est affiché (tests).
var shown: Array[int] = []


func setup(battle: Object, side_colors: Dictionary, player_side: String) -> void:
	_battle = battle
	_colors = side_colors
	_player_side = player_side


## Tireurs à tracer : sélectionnés puis survolés, sans doublon, portée > 0, au plus MAX_ARCS.
static func shooters(units: Array, selected: Array, hovered: Array) -> Array[Dictionary]:
	var out: Array[Dictionary] = []
	var seen := {}
	for group in [selected, hovered]:
		for id in group:
			if seen.has(int(id)) or out.size() >= MAX_ARCS:
				continue
			seen[int(id)] = true
			for unit in units:
				if int(unit["id"]) == int(id):
					if float(unit.get("effective_range", 0.0)) > 0.0 and bool(unit.get("present", true)):
						out.append(unit)
					break
	return out


func update(units: Array, selected: Array, hovered: Array, camera_distance: float = INF) -> void:
	# CR1 : pas d'arc (ruban sans test de profondeur) en gros plan, comme les trajets d'ordres.
	var list: Array[Dictionary] = []
	if camera_distance >= BattlePathPreview.ORDERS_NEAR_HIDE_M:
		list = shooters(units, selected, hovered)
	shown.clear()
	for i in list.size():
		var unit: Dictionary = list[i]
		shown.append(int(unit["id"]))
		var is_selected := selected.has(int(unit["id"]))
		var node := _node(i)
		node.visible = true
		var signature := "%d|%.0f|%.0f|%.2f|%.0f|%.3f|%s" % [int(unit["id"]), float(unit["x"]), float(unit["z"]), float(unit["facing"]), float(unit["effective_range"]), float(unit.get("fire_arc", 0.0)), is_selected]
		if _signatures[i] != signature:
			_signatures[i] = signature
			_build(node.mesh as ImmediateMesh, unit, is_selected)
	for i in range(list.size(), _nodes.size()):
		_nodes[i].visible = false
		_signatures[i] = ""


func _node(i: int) -> MeshInstance3D:
	while _nodes.size() <= i:
		var node := MeshInstance3D.new()
		node.name = "RangeArc%d" % _nodes.size()
		node.mesh = ImmediateMesh.new()
		node.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
		var material := StandardMaterial3D.new()
		material.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
		material.vertex_color_use_as_albedo = true
		material.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA
		material.cull_mode = BaseMaterial3D.CULL_DISABLED
		material.no_depth_test = true  # lisible par-dessus l'herbe et les figurines, comme le trajet
		material.render_priority = 1
		node.material_override = material
		add_child(node)
		_nodes.append(node)
		_signatures.append("")
	return _nodes[i]


func _height(x: float, z: float) -> float:
	if _battle != null and _battle.has_method("get_walk_height"):
		return float(_battle.call("get_walk_height", x, z)) + LIFT
	return LIFT


## Point au sol à l'angle `angle` (0 = +z, sens du cap du cœur) et à la distance `r`.
func _ground(cx: float, cz: float, angle: float, r: float) -> Vector3:
	var x := cx + sin(angle) * r
	var z := cz + cos(angle) * r
	return Vector3(x, _height(x, z), z)


func _build(mesh: ImmediateMesh, unit: Dictionary, is_selected: bool) -> void:
	mesh.clear_surfaces()
	var cx := float(unit["x"])
	var cz := float(unit["z"])
	var facing := float(unit["facing"])
	var r := float(unit["effective_range"])
	var half := clampf(float(unit.get("fire_arc", PI)), 0.01, PI)
	var base: Color = _colors.get(str(unit["side"]), Color(0.95, 0.85, 0.4))
	var color := Color(base.lightened(0.25), ALPHA_SELECTED if is_selected else ALPHA_HOVERED)
	var edge_color := Color(color, color.a * EDGE_ALPHA)
	mesh.surface_begin(Mesh.PRIMITIVE_TRIANGLES)
	# Arc extérieur.
	var segments := clampi(int(ceil(2.0 * half * r / SEGMENT_M)), 8, 160)
	var inner := maxf(r - ARC_WIDTH, 0.0)
	for s in segments:
		var a0 := facing - half + 2.0 * half * float(s) / segments
		var a1 := facing - half + 2.0 * half * float(s + 1) / segments
		_quad(mesh, _ground(cx, cz, a0, inner), _ground(cx, cz, a0, r), _ground(cx, cz, a1, r), _ground(cx, cz, a1, inner), color)
	# Bords du secteur, depuis l'avant du régiment.
	if half < PI - 0.01:
		var start := minf(float(unit.get("depth", 0.0)) * 0.5, r * 0.5)
		for side_sign: float in [-1.0, 1.0]:
			var angle := facing + side_sign * half
			var across := Vector2(cos(angle), -sin(angle)) * (EDGE_WIDTH * 0.5)
			var steps := maxi(1, int(ceil((r - start) / EDGE_STEP_M)))
			for s in steps:
				var d0 := lerpf(start, inner, float(s) / steps)
				var d1 := lerpf(start, inner, float(s + 1) / steps)
				var p0 := _ground(cx, cz, angle, d0)
				var p1 := _ground(cx, cz, angle, d1)
				var off := Vector3(across.x, 0.0, across.y)
				_quad(mesh, p0 - off, p0 + off, p1 + off, p1 - off, edge_color)
	mesh.surface_end()


func _quad(mesh: ImmediateMesh, a: Vector3, b: Vector3, c: Vector3, d: Vector3, color: Color) -> void:
	for p: Vector3 in [a, b, c, a, c, d]:
		mesh.surface_set_color(color)
		mesh.surface_add_vertex(p)
