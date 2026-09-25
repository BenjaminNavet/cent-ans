class_name NavalEffects
extends Node3D

## Effets d'abordage des batailles navales (lot NV1), rendu seulement : grappins et leurs
## filins tendus entre deux navires accrochés (`ship.grappled`), passerelles (planches jetées
## d'un bord à l'autre) dès que la mêlée commence, gerbes d'eau (coup d'éperon, hommes à la mer,
## navire qui coule). Les positions viennent des vues des navires (`NavalShipView`).

const ROPES_PER_PAIR := 5
const PLANKS_PER_PAIR := 3
const MAX_PAIRS := 48

var _ropes: MultiMesh
var _planks: MultiMesh
var _splashes: GPUParticles3D
## Paires accrochées dessinées (banc d'essai, captures).
var pairs_drawn: int = 0


func setup() -> void:
	name = "NavalEffects"
	var rope_mesh := CylinderMesh.new()
	rope_mesh.top_radius = 0.035
	rope_mesh.bottom_radius = 0.035
	rope_mesh.height = 1.0
	rope_mesh.radial_segments = 4
	rope_mesh.rings = 1
	var rope_mat := StandardMaterial3D.new()
	rope_mat.albedo_color = Color(0.3, 0.24, 0.15)
	rope_mat.roughness = 0.95
	rope_mesh.material = rope_mat
	_ropes = _layer("GrappleRopes", rope_mesh, ROPES_PER_PAIR * MAX_PAIRS)
	var plank_mesh := BoxMesh.new()
	plank_mesh.size = Vector3(0.7, 1.0, 0.08)
	var plank_mat := StandardMaterial3D.new()
	plank_mat.albedo_color = Color(0.42, 0.32, 0.2)
	plank_mat.roughness = 0.9
	plank_mesh.material = plank_mat
	_planks = _layer("Gangplanks", plank_mesh, PLANKS_PER_PAIR * MAX_PAIRS)
	_splashes = _make_splashes()
	add_child(_splashes)


func _layer(node_name: String, mesh: Mesh, count: int) -> MultiMesh:
	var mm := MultiMesh.new()
	mm.transform_format = MultiMesh.TRANSFORM_3D
	mm.mesh = mesh
	mm.instance_count = count
	mm.visible_instance_count = 0
	var instance := MultiMeshInstance3D.new()
	instance.name = node_name
	instance.multimesh = mm
	instance.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	instance.custom_aabb = AABB(Vector3(-5000, -50, -5000), Vector3(10000, 200, 10000))
	add_child(instance)
	return mm


## `views` : id -> NavalShipView ; `ships` : état courant (`get_ships`).
func update(views: Dictionary, ships: Array) -> void:
	var rope_count := 0
	var plank_count := 0
	var pairs := 0
	for ship in ships:
		var a := int(ship["id"])
		if not views.has(a):
			continue
		for other in ship.get("grappled", PackedInt32Array()):
			var b := int(other)
			if b <= a or not views.has(b) or pairs >= MAX_PAIRS:
				continue
			pairs += 1
			var va: NavalShipView = views[a]
			var vb: NavalShipView = views[b]
			var melee := float(ship.get("melee_time", 0.0)) > 0.0
			for k in ROPES_PER_PAIR:
				var t := (float(k) + 0.5) / ROPES_PER_PAIR - 0.5
				var from := va.rail_point(t * 0.7, vb.global_position, 1.2)
				var to := vb.rail_point(-t * 0.6, va.global_position, 0.9)
				# Filin légèrement détendu : le milieu s'affaisse.
				var mid := (from + to) * 0.5 + Vector3(0, -0.4, 0)
				_ropes.set_instance_transform(rope_count, _segment(from, mid, 1.0))
				_ropes.set_instance_transform(rope_count + 1, _segment(mid, to, 1.0))
				rope_count += 2
				if rope_count + 2 > _ropes.instance_count:
					break
			if melee:
				for k in PLANKS_PER_PAIR:
					var t := (float(k) - 1.0) * 0.18
					var from := va.rail_point(t, vb.global_position, 0.0)
					var to := vb.rail_point(-t, va.global_position, 0.0)
					if plank_count < _planks.instance_count:
						_planks.set_instance_transform(plank_count, _segment(from, to, 1.0))
						plank_count += 1
	_ropes.visible_instance_count = rope_count
	_planks.visible_instance_count = plank_count
	pairs_drawn = pairs


## Transformée qui étire un maillage unitaire (axe Y pour les filins, Z pour les planches)
## de `from` à `to`.
static func _segment(from: Vector3, to: Vector3, width: float) -> Transform3D:
	var d := to - from
	var len := maxf(d.length(), 0.01)
	var y := d / len
	var x := y.cross(Vector3.UP)
	if x.length() < 0.01:
		x = Vector3.RIGHT
	x = x.normalized()
	var z := x.cross(y).normalized()
	# Colonnes : x (largeur, horizontale), y (le long du segment), z (épaisseur, presque verticale).
	var basis := Basis(x * width, y * len, z)
	return Transform3D(basis, (from + to) * 0.5)


## Gerbe d'eau au point `at` ; `size` 1 = homme à la mer, 4 = coup d'éperon.
func splash(at: Vector3, size: float) -> void:
	var count := int(clampf(6.0 * size, 4.0, 40.0))
	for i in count:
		var dir := Vector3(randf_range(-1.0, 1.0), 0.0, randf_range(-1.0, 1.0)).normalized()
		var vel := dir * randf_range(0.5, 2.0) * size + Vector3.UP * randf_range(3.0, 6.0) * sqrt(size)
		var xf := Transform3D(Basis.IDENTITY.scaled(Vector3.ONE * randf_range(0.6, 1.4) * sqrt(size)), at + dir * randf_range(0.0, 1.0) * size)
		_splashes.emit_particle(xf, vel, Color(1, 1, 1, 1), Color(0, 0, 0, 0), GPUParticles3D.EMIT_FLAG_POSITION | GPUParticles3D.EMIT_FLAG_VELOCITY | GPUParticles3D.EMIT_FLAG_ROTATION_SCALE)


func _make_splashes() -> GPUParticles3D:
	var particles := GPUParticles3D.new()
	particles.name = "Splashes"
	particles.amount = 512
	particles.lifetime = 1.6
	particles.emitting = false
	particles.one_shot = false
	particles.local_coords = false
	particles.visibility_aabb = AABB(Vector3(-5000, -20, -5000), Vector3(10000, 100, 10000))
	var process := ParticleProcessMaterial.new()
	process.gravity = Vector3(0, -9.8, 0)
	process.scale_min = 0.8
	process.scale_max = 1.6
	var ramp := Gradient.new()
	ramp.set_color(0, Color(0.92, 0.95, 0.96, 0.85))
	ramp.set_color(1, Color(0.85, 0.9, 0.92, 0.0))
	var ramp_texture := GradientTexture1D.new()
	ramp_texture.gradient = ramp
	process.color_ramp = ramp_texture
	particles.process_material = process
	var quad := QuadMesh.new()
	quad.size = Vector2(1.2, 1.2)
	var mat := StandardMaterial3D.new()
	mat.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	mat.billboard_mode = BaseMaterial3D.BILLBOARD_PARTICLES
	mat.billboard_keep_scale = true
	mat.vertex_color_use_as_albedo = true
	mat.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA
	var gradient := Gradient.new()
	gradient.set_color(0, Color(1, 1, 1, 1))
	gradient.set_color(1, Color(1, 1, 1, 0))
	var texture := GradientTexture2D.new()
	texture.gradient = gradient
	texture.fill = GradientTexture2D.FILL_RADIAL
	texture.fill_from = Vector2(0.5, 0.5)
	texture.fill_to = Vector2(0.5, 0.0)
	texture.width = 32
	texture.height = 32
	mat.albedo_texture = texture
	quad.material = mat
	particles.draw_pass_1 = quad
	particles.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	return particles
