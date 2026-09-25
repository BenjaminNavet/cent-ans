class_name ArmyMovementBubble
extends MeshInstance3D

## Lot M4 : bulle des cases atteignables ce tour par l'armée sélectionnée, dessinée au sol
## (`reachable_bubble.gdshader`). Rendu seulement : le masque vient de
## `CampaignSim.get_reachable_area` (une case de la grille de navigation par texel, recadré
## sur la bulle) ; le maillage est une grille posée sur le relief couvrant ce cadre.

const SHADER := preload("res://shaders/reachable_bubble.gdshader")
## Pas de la grille du maillage (pixels carte), borné pour rester sous ~100 × 100 quads.
const MIN_STEP_PX := 4.0
const MAX_QUADS := 96
const LIFT := 0.35

var map_data: MapData
var _material: ShaderMaterial
var _cells := 0
var _rect := Rect2()


func setup(data: MapData) -> void:
	map_data = data
	name = "ArmyMovementBubble"
	cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	_material = ShaderMaterial.new()
	_material.shader = SHADER
	_material.render_priority = 0
	material_override = _material
	visible = false


## `area` : dictionnaire de `get_reachable_area` ({image, origin, size, cells, ...}).
func show_area(area: Dictionary) -> void:
	var image: Image = area.get("image")
	_cells = int(area.get("cells", 0))
	if image == null or _cells <= 1 or map_data == null:
		hide_bubble()
		return
	var origin: Vector2 = area.get("origin", Vector2.ZERO)
	var extent: Vector2 = area.get("size", Vector2.ONE)
	_rect = Rect2(origin, extent)
	var mask := image.duplicate() as Image
	mask.generate_mipmaps()  # contour stable au dézoom (une case < un pixel écran)
	_material.set_shader_parameter("mask", ImageTexture.create_from_image(mask))
	_material.set_shader_parameter("origin", origin)
	_material.set_shader_parameter("extent", extent)
	_material.set_shader_parameter("texel", Vector2(1.0 / image.get_width(), 1.0 / image.get_height()))
	mesh = _build_mesh(_rect)
	visible = true


func hide_bubble() -> void:
	_cells = 0
	_rect = Rect2()
	visible = false


## Nombre de cases de la bulle affichée (0 si masquée).
func cell_count() -> int:
	return _cells if visible else 0


## Cadre carte (pixels) couvert par la bulle affichée.
func covered_rect() -> Rect2:
	return _rect


func _build_mesh(rect: Rect2) -> ArrayMesh:
	var step := maxf(MIN_STEP_PX, maxf(rect.size.x, rect.size.y) / MAX_QUADS)
	var nx := maxi(int(ceil(rect.size.x / step)), 1)
	var nz := maxi(int(ceil(rect.size.y / step)), 1)
	var vertices := PackedVector3Array()
	vertices.resize((nx + 1) * (nz + 1))
	var indices := PackedInt32Array()
	indices.resize(nx * nz * 6)
	var v := 0
	for j in nz + 1:
		var z := minf(rect.position.y + j * step, rect.end.y)
		for i in nx + 1:
			var x := minf(rect.position.x + i * step, rect.end.x)
			vertices[v] = Vector3(x, map_data.surface_world_at(x, z) + LIFT, z)
			v += 1
	var k := 0
	for j in nz:
		for i in nx:
			var a := j * (nx + 1) + i
			var b := a + nx + 1
			indices[k] = a
			indices[k + 1] = a + 1
			indices[k + 2] = b + 1
			indices[k + 3] = a
			indices[k + 4] = b + 1
			indices[k + 5] = b
			k += 6
	var arrays := []
	arrays.resize(Mesh.ARRAY_MAX)
	arrays[Mesh.ARRAY_VERTEX] = vertices
	arrays[Mesh.ARRAY_INDEX] = indices
	var result := ArrayMesh.new()
	result.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES, arrays)
	return result
