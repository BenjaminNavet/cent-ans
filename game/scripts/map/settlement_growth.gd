class_name SettlementGrowth
extends RefCounted

## Lot CV1 : colonies qui grandissent avec leur niveau (rendu seulement). Niveau visuel :
## 0 village, 1 bourg (ouvert, halle), 2 ville (murailles), 3 cité (cathédrale, halle,
## château, faubourgs). Calculé depuis le type, les bâtiments et la fortification
## (`CampaignSim.settlement_detail`) et la population de la province (`get_province_state`) :
## - village → bourg : marché bâti ou province très peuplée ;
## - bourg → ville : murailles (`bld_stone_walls`, `bld_palisade`) ou fortification ≥ 2 ;
## - ville → cité : cité épiscopale (cathédrale) ou capitale de province très peuplée ;
## - cité : château de Kenney Castle Kit (CC0) en plus si fortification ≥ 6.
## Châteaux et abbayes gardent leur maquette. Paris (monument dédié, lot L1) est exclu.

const LEVEL_NAMES: Array[String] = ["village", "bourg", "ville", "cité"]
## Colonies exclues de la croissance générique (monuments dédiés, lot L1).
const EXCLUDED: Array[String] = ["set_paris"]
## Maquettes par niveau (`game/assets/models/settlements/`).
const LEVEL_MODELS := [["village_a", "village_b"], ["bourg_a", "bourg_b"], ["town_a", "town_b"], ["cite_a", "cite_b"]]
## Échelle monde par niveau (unités Blender → pixels carte).
const LEVEL_SCALE := [3.8, 4.1, 4.4, 4.6]
const WALL_BUILDINGS: Array[String] = ["bld_stone_walls", "bld_palisade", "bld_walls"]
## Population de province (habitants) au-delà de laquelle un village devient bourg, une ville cité.
const BOURG_POPULATION := 420000.0
const CITE_POPULATION := 600000.0
const CASTLE_FORTIFICATION := 6
## Pièces Kenney du château (tour carrée + toit, tours hexagonales).
const KENNEY_DIR := "res://assets/third_party/buildings/kenney_castle_kit/"
## Lot TF : mètres par unité Kenney (une tour carrée ≈ 8 m) : UV de l'atlas et usure en mètres.
const KENNEY_METERS := 8.0


## Niveau visuel d'une colonie, -1 si elle n'en a pas (château, abbaye) ou est exclue.
## `detail` : `settlement_detail` (buildings, fortification_level, is_city).
static func level_of(entry: Dictionary, detail: Dictionary, province_population: float) -> int:
	if EXCLUDED.has(str(entry.get("id", ""))):
		return -1
	var kind := str(entry.get("kind", ""))
	var buildings: Array = detail.get("buildings", [])
	var fortification := int(detail.get("fortification_level", entry.get("fortification_level", 0)))
	var walled := fortification >= 2
	for building in WALL_BUILDINGS:
		if buildings.has(building):
			walled = true
	match kind:
		"village":
			return 1 if buildings.has("bld_market") or province_population >= BOURG_POPULATION else 0
		"town":
			return 2 if walled else 1
		"city":
			if buildings.has("bld_cathedral") or (bool(detail.get("is_city", false)) and province_population >= CITE_POPULATION):
				return 3
			return 2 if walled else 1
	return -1


## Château (Kenney) à côté de la cité.
static func has_castle(detail: Dictionary) -> bool:
	return int(detail.get("fortification_level", 0)) >= CASTLE_FORTIFICATION


## Maquette à l'échelle monde pour un niveau (variante déterministe), château éventuel ;
## null si les modèles manquent.
static func build_model(level: int, variant_seed: int, castle: bool) -> Node3D:
	if level < 0 or level >= LEVEL_MODELS.size():
		return null
	var variants: Array = LEVEL_MODELS[level]
	var name := str(variants[absi(variant_seed) % variants.size()])
	var model := ModelLibrary.instantiate("settlements/" + name, LEVEL_SCALE[level])
	if model == null:
		return null
	var root := Node3D.new()
	root.name = "CV1_%s" % name
	root.add_child(model)
	if castle:
		var chateau := kenney_castle()
		if chateau != null:
			var angle := float(absi(variant_seed / 3) % 628) / 100.0
			chateau.position = Vector3(cos(angle), 0.0, sin(angle)) * LEVEL_SCALE[level] * 1.65
			chateau.rotation.y = angle
			root.add_child(chateau)
	return root


## Château de Kenney Castle Kit : donjon carré coiffé, deux tours hexagonales, courtine ;
## pièces fusionnées en un seul maillage (un appel de rendu par château, mis en cache).
## Lot TF : converti au matériau atlas `Building` des maquettes (`BuildingMaterials`, variante
## `far`) au lieu de la palette Kenney teintée (toits bleus) : chaque face lit sa couleur dans la
## palette du kit, les bleus deviennent ardoise (`RoofSlate`), les bruns sombres bois (`Timber`),
## le reste pierre de taille (`Masonry`) ; UV en mètres projetées selon la normale.
static var _castle_mesh: ArrayMesh = null
static var _castle_loaded := false


static func kenney_castle() -> Node3D:
	var mesh := _kenney_castle_mesh()
	if mesh == null:
		return null
	var instance := MeshInstance3D.new()
	instance.name = "KenneyCastle"
	instance.mesh = mesh
	instance.scale = Vector3.ONE * (0.85 / KENNEY_METERS)
	return instance


static func _kenney_castle_mesh() -> ArrayMesh:
	if _castle_loaded:
		return _castle_mesh
	_castle_loaded = true
	var pieces := [
		["tower-square", Vector3(0, 0, 0)],
		["tower-square-top-roof-high", Vector3(0, 1, 0)],
		["tower-hexagon-base", Vector3(1.4, 0, 0.9)],
		["tower-hexagon-roof", Vector3(1.4, 1, 0.9)],
		["tower-hexagon-base", Vector3(-1.3, 0, 1.0)],
		["tower-hexagon-roof", Vector3(-1.3, 1, 1.0)],
		["wall", Vector3(0.05, 0, 1.1)],
	]
	var tool := SurfaceTool.new()
	tool.begin(Mesh.PRIMITIVE_TRIANGLES)
	var material: StandardMaterial3D = null
	for piece in pieces:
		var path := KENNEY_DIR + str(piece[0]) + ".glb"
		if not ResourceLoader.exists(path):
			return null
		var root := (load(path) as PackedScene).instantiate() as Node3D
		for child in root.find_children("*", "MeshInstance3D", true, false):
			var mesh_instance := child as MeshInstance3D
			if mesh_instance.mesh == null:
				continue
			var xform := Transform3D(Basis.IDENTITY, piece[1]) * _relative(root, mesh_instance)
			for surface in mesh_instance.mesh.get_surface_count():
				tool.append_from(mesh_instance.mesh, surface, xform)
				if material == null:
					material = mesh_instance.mesh.surface_get_material(surface) as StandardMaterial3D
		root.free()
	var palette: Image = null
	if material != null and material.albedo_texture != null:
		palette = material.albedo_texture.get_image()
		if palette != null and palette.is_compressed():
			palette.decompress()
	_castle_mesh = _to_atlas(tool.commit(), palette)
	return _castle_mesh


## Lot TF : maillage Kenney (palette) → maillage de l'atlas `Building` : sommets en mètres
## (`KENNEY_METERS`), couche de chaque face dans l'alpha de la couleur de sommet, RVB = nuance
## de la palette (relief des pièces conservé), UV projetées selon la normale de la face.
static func _to_atlas(source: ArrayMesh, palette: Image) -> ArrayMesh:
	var arrays := source.surface_get_arrays(0)
	var vertices: PackedVector3Array = arrays[Mesh.ARRAY_VERTEX]
	var uvs: Variant = arrays[Mesh.ARRAY_TEX_UV]
	var indices: Variant = arrays[Mesh.ARRAY_INDEX]
	var order := PackedInt32Array()
	if indices != null and (indices as PackedInt32Array).size() > 0:
		order = indices
	else:
		order.resize(vertices.size())
		for i in vertices.size():
			order[i] = i
	var layers := BuildingMaterials.atlas_layers()
	var slate := layers.find("RoofSlate")
	var timber := layers.find("Timber")
	var stone := layers.find("Masonry")
	var out_v := PackedVector3Array()
	var out_n := PackedVector3Array()
	var out_uv := PackedVector2Array()
	var out_c := PackedColorArray()
	for t in range(0, order.size() - 2, 3):
		var a := vertices[order[t]] * KENNEY_METERS
		var b := vertices[order[t + 1]] * KENNEY_METERS
		var c := vertices[order[t + 2]] * KENNEY_METERS
		var normal := (c - a).cross(b - a).normalized()
		var sample := Color(0.6, 0.6, 0.6)
		if palette != null and uvs != null:
			var uv_list: PackedVector2Array = uvs
			var uv: Vector2 = (uv_list[order[t]] + uv_list[order[t + 1]] + uv_list[order[t + 2]]) / 3.0
			var px := clampi(int(fposmod(uv.x, 1.0) * palette.get_width()), 0, palette.get_width() - 1)
			var py := clampi(int(fposmod(uv.y, 1.0) * palette.get_height()), 0, palette.get_height() - 1)
			sample = palette.get_pixel(px, py)
		var layer := stone
		if sample.b > sample.r + 0.12 and sample.b > sample.g + 0.04:
			layer = slate
		elif sample.r > sample.b + 0.08 and sample.get_luminance() < 0.42:
			layer = timber
		var shade := clampf(0.8 + (sample.get_luminance() - 0.5) * 0.5, 0.6, 1.05)
		var color := Color(shade, shade, shade, (float(layer) + 0.5) / 16.0)
		var n := normal.abs()
		for p in [a, b, c]:
			var q: Vector3 = p
			var uv_m := Vector2(q.x, q.z) if n.y > maxf(n.x, n.z) else (Vector2(q.z, -q.y) if n.x > n.z else Vector2(q.x, -q.y))
			out_v.append(q)
			out_n.append(normal)
			out_uv.append(uv_m)
			out_c.append(color)
	var out := []
	out.resize(Mesh.ARRAY_MAX)
	out[Mesh.ARRAY_VERTEX] = out_v
	out[Mesh.ARRAY_NORMAL] = out_n
	out[Mesh.ARRAY_TEX_UV] = out_uv
	out[Mesh.ARRAY_COLOR] = out_c
	var mesh := ArrayMesh.new()
	mesh.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES, out)
	mesh.surface_set_material(0, BuildingMaterials.material("Building", "far"))
	return mesh


static func _relative(root: Node, node: Node3D) -> Transform3D:
	var xform := Transform3D.IDENTITY
	var current: Node = node
	while current != null and current != root:
		if current is Node3D:
			xform = (current as Node3D).transform * xform
		current = current.get_parent()
	return xform
