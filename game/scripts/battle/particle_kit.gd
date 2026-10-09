class_name ParticleKit
extends RefCounted
## Fabrique statique de GPUParticles3D / ParticleProcessMaterial du champ de bataille.
##
## Les dictionnaires de propriétés sont appliqués dans l'ordre d'insertion avec `set()` : les noms
## sont ceux de Godot (`amount`, `one_shot`, `emission_shape`, `initial_velocity_min`…).


## Applique `props` à `target` (clé = nom de propriété) et le renvoie.
static func apply(target: Object, props: Dictionary) -> Object:
	for key in props:
		target.set(key, props[key])
	return target


## ParticleProcessMaterial réglé par `props`.
static func process(props: Dictionary) -> ParticleProcessMaterial:
	return apply(ParticleProcessMaterial.new(), props) as ParticleProcessMaterial


## Émetteur : `props` sur le GPUParticles3D, puis matériau de processus et maillage de dessin.
static func emitter(props: Dictionary, process_material: ParticleProcessMaterial, draw_mesh: Mesh) -> GPUParticles3D:
	var particles := apply(GPUParticles3D.new(), props) as GPUParticles3D
	particles.process_material = process_material
	particles.draw_pass_1 = draw_mesh
	return particles


## Dégradé de couleur sur la durée de vie : deux couleurs, ou `offsets` explicites.
static func ramp(start: Color, end: Color) -> GradientTexture1D:
	var gradient := Gradient.new()
	gradient.set_color(0, start)
	gradient.set_color(1, end)
	var texture := GradientTexture1D.new()
	texture.gradient = gradient
	return texture


## Courbe d'échelle sur la durée de vie : points `Vector2(position, valeur)`.
static func curve(points: Array) -> CurveTexture:
	var shape := Curve.new()
	for point in points:
		shape.add_point(point)
	var texture := CurveTexture.new()
	texture.curve = shape
	return texture


## Quad de dessin avec sa matière.
static func quad(size: Vector2, material: Material, center_offset: Vector3 = Vector3.ZERO) -> QuadMesh:
	var mesh := QuadMesh.new()
	mesh.size = size
	if center_offset != Vector3.ZERO:
		mesh.center_offset = center_offset
	mesh.material = material
	return mesh
