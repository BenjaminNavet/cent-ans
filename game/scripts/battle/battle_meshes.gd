class_name BattleMeshes
extends RefCounted

## Maillages low-poly procéduraux des batailles (SurfaceTool) : fantassin, archer, cavalier,
## engin de siège, arbre, hampe et drapeau. Chaque figurine a deux surfaces : 0 = livrée
## (couleur de faction, via le matériau) et 1 = neutre (couleurs de sommets : peau, bois,
## acier, robe du cheval). Échelle réelle : un fantassin mesure ~1,8 m.

const SKIN := Color(0.86, 0.68, 0.52)
const STEEL := Color(0.62, 0.64, 0.68)
const WOOD := Color(0.45, 0.31, 0.18)
const CLOTH_DARK := Color(0.28, 0.22, 0.16)
const HORSE := Color(0.40, 0.27, 0.17)
const HORSE_DARK := Color(0.22, 0.15, 0.10)


## Ajoute un pavé centré en `center`, de taille `size`, tourné de `pitch` autour de X.
static func add_box(st: SurfaceTool, center: Vector3, size: Vector3, color: Color, pitch: float = 0.0) -> void:
	var h := size * 0.5
	var basis := Basis(Vector3.RIGHT, pitch)
	var corners: Array[Vector3] = []
	for i in 8:
		var local := Vector3(h.x if i & 1 else -h.x, h.y if i & 2 else -h.y, h.z if i & 4 else -h.z)
		corners.append(center + basis * local)
	# Faces : (indices des coins, normale locale).
	var faces := [
		[[0, 2, 3, 1], Vector3.BACK * -1.0],  # -Z
		[[4, 5, 7, 6], Vector3.BACK],  # +Z
		[[0, 4, 6, 2], Vector3.LEFT],  # -X
		[[1, 3, 7, 5], Vector3.RIGHT],  # +X
		[[0, 1, 5, 4], Vector3.DOWN],  # -Y
		[[2, 6, 7, 3], Vector3.UP],  # +Y
	]
	for face in faces:
		var idx: Array = face[0]
		var normal: Vector3 = basis * (face[1] as Vector3)
		tri(st, corners[idx[0]], corners[idx[1]], corners[idx[2]], normal, color)
		tri(st, corners[idx[0]], corners[idx[2]], corners[idx[3]], normal, color)


## Ajoute un triangle en ordre horaire vu du côté de `normal` (face avant pour Godot).
static func tri(st: SurfaceTool, p0: Vector3, p1: Vector3, p2: Vector3, normal: Vector3, color: Color) -> void:
	if (p1 - p0).cross(p2 - p0).dot(normal) > 0.0:
		var swap := p1
		p1 = p2
		p2 = swap
	for v in [p0, p1, p2]:
		st.set_color(color)
		st.set_normal(normal)
		st.add_vertex(v)


## Cône (ou tronc de cône) à `sides` pans, de `base_y` à `top_y`.
static func add_cone(st: SurfaceTool, center: Vector3, radius: float, height: float, sides: int, color: Color, top_radius: float = 0.0) -> void:
	for i in sides:
		var a0 := TAU * float(i) / float(sides)
		var a1 := TAU * float(i + 1) / float(sides)
		var p0 := center + Vector3(cos(a0) * radius, 0.0, sin(a0) * radius)
		var p1 := center + Vector3(cos(a1) * radius, 0.0, sin(a1) * radius)
		var t0 := center + Vector3(cos(a0) * top_radius, height, sin(a0) * top_radius)
		var t1 := center + Vector3(cos(a1) * top_radius, height, sin(a1) * top_radius)
		var normal := (Vector3(cos((a0 + a1) * 0.5), radius / maxf(height, 0.01), sin((a0 + a1) * 0.5))).normalized()
		tri(st, p0, t0, p1, normal, color)
		if top_radius > 0.0:
			tri(st, p1, t0, t1, normal, color)


## Assemble les deux surfaces (livrée, neutre) en un ArrayMesh avec leurs matériaux.
static func _commit_two(livery: SurfaceTool, neutral: SurfaceTool, color: Color) -> ArrayMesh:
	var mesh := ArrayMesh.new()
	livery.commit(mesh)
	neutral.commit(mesh)
	var livery_mat := StandardMaterial3D.new()
	livery_mat.albedo_color = color
	livery_mat.roughness = 0.9
	livery_mat.cull_mode = BaseMaterial3D.CULL_DISABLED
	var neutral_mat := StandardMaterial3D.new()
	neutral_mat.vertex_color_use_as_albedo = true
	neutral_mat.vertex_color_is_srgb = true
	neutral_mat.roughness = 0.85
	neutral_mat.cull_mode = BaseMaterial3D.CULL_DISABLED
	mesh.surface_set_material(0, livery_mat)
	mesh.surface_set_material(1, neutral_mat)
	return mesh


static func _tool() -> SurfaceTool:
	var st := SurfaceTool.new()
	st.begin(Mesh.PRIMITIVE_TRIANGLES)
	return st


## Figurine d'un camp : `kind` ∈ infantry, archer, cavalry, siege.
static func soldier(kind: String, color: Color) -> ArrayMesh:
	var lv := _tool()
	var nt := _tool()
	match kind:
		"archer":
			add_box(lv, Vector3(0, 1.1, 0), Vector3(0.48, 0.62, 0.3), Color.WHITE)
			add_box(lv, Vector3(0, 1.72, -0.02), Vector3(0.3, 0.2, 0.3), Color.WHITE)
			add_box(nt, Vector3(-0.1, 0.4, 0), Vector3(0.16, 0.8, 0.2), CLOTH_DARK)
			add_box(nt, Vector3(0.1, 0.4, 0), Vector3(0.16, 0.8, 0.2), CLOTH_DARK)
			add_box(nt, Vector3(0, 1.57, 0), Vector3(0.24, 0.24, 0.24), SKIN)
			add_box(nt, Vector3(0.34, 1.2, 0.12), Vector3(0.05, 1.5, 0.05), WOOD, 0.15)
			add_box(nt, Vector3(-0.18, 1.2, -0.2), Vector3(0.14, 0.5, 0.14), WOOD)
		"cavalry":
			add_box(lv, Vector3(0, 1.18, 0), Vector3(0.78, 0.55, 1.75), Color.WHITE)
			add_box(lv, Vector3(0, 2.2, -0.1), Vector3(0.46, 0.6, 0.32), Color.WHITE)
			add_box(lv, Vector3(-0.32, 2.15, 0.05), Vector3(0.06, 0.55, 0.45), Color.WHITE)
			add_box(nt, Vector3(0, 1.35, 0), Vector3(0.62, 0.6, 1.7), HORSE)
			for x in [-0.22, 0.22]:
				for z in [-0.62, 0.62]:
					add_box(nt, Vector3(x, 0.5, z), Vector3(0.14, 1.0, 0.14), HORSE_DARK)
			add_box(nt, Vector3(0, 1.85, 0.95), Vector3(0.3, 0.75, 0.32), HORSE, -0.6)
			add_box(nt, Vector3(0, 2.15, 1.25), Vector3(0.24, 0.3, 0.5), HORSE)
			add_box(nt, Vector3(0, 2.7, -0.1), Vector3(0.28, 0.3, 0.28), STEEL)
			add_box(nt, Vector3(0.3, 2.35, 0.9), Vector3(0.06, 0.06, 3.2), WOOD, -0.08)
		"siege":
			add_box(lv, Vector3(0, 1.0, -0.6), Vector3(1.2, 0.3, 0.9), Color.WHITE)
			add_box(nt, Vector3(0, 0.6, 0), Vector3(2.2, 0.35, 3.2), WOOD)
			for x in [-1.15, 1.15]:
				for z in [-1.1, 1.1]:
					add_box(nt, Vector3(x, 0.45, z), Vector3(0.15, 0.9, 0.9), HORSE_DARK)
			add_box(nt, Vector3(-0.7, 1.6, 0), Vector3(0.2, 1.9, 0.2), WOOD)
			add_box(nt, Vector3(0.7, 1.6, 0), Vector3(0.2, 1.9, 0.2), WOOD)
			add_box(nt, Vector3(0, 2.3, 0.3), Vector3(0.18, 0.18, 3.4), WOOD, 0.5)
		_:
			add_box(lv, Vector3(0, 1.12, 0), Vector3(0.52, 0.66, 0.32), Color.WHITE)
			add_box(lv, Vector3(-0.3, 1.1, 0.12), Vector3(0.08, 0.62, 0.46), Color.WHITE)
			add_box(nt, Vector3(-0.11, 0.4, 0), Vector3(0.17, 0.8, 0.22), CLOTH_DARK)
			add_box(nt, Vector3(0.11, 0.4, 0), Vector3(0.17, 0.8, 0.22), CLOTH_DARK)
			add_box(nt, Vector3(0, 1.6, 0), Vector3(0.25, 0.25, 0.25), SKIN)
			add_box(nt, Vector3(0, 1.76, 0), Vector3(0.3, 0.12, 0.3), STEEL)
			add_box(nt, Vector3(0.3, 1.35, 0.1), Vector3(0.05, 2.6, 0.05), WOOD)
	return _commit_two(lv, nt, color)


## Arbre low-poly (tronc + deux étages de feuillage), couleurs de sommets.
static func tree() -> ArrayMesh:
	var st := SurfaceTool.new()
	st.begin(Mesh.PRIMITIVE_TRIANGLES)
	add_box(st, Vector3(0, 1.2, 0), Vector3(0.45, 2.4, 0.45), WOOD)
	add_cone(st, Vector3(0, 2.0, 0), 2.6, 4.0, 7, Color(0.17, 0.33, 0.14))
	add_cone(st, Vector3(0, 4.2, 0), 1.9, 3.6, 7, Color(0.21, 0.40, 0.17))
	var mesh := st.commit()
	var mat := StandardMaterial3D.new()
	mat.vertex_color_use_as_albedo = true
	mat.vertex_color_is_srgb = true
	mat.roughness = 1.0
	mat.cull_mode = BaseMaterial3D.CULL_DISABLED
	mesh.surface_set_material(0, mat)
	return mesh


## Hampe de bannière (bois), 1 m de haut : mise à l'échelle par le nœud.
static func pole() -> ArrayMesh:
	var st := SurfaceTool.new()
	st.begin(Mesh.PRIMITIVE_TRIANGLES)
	add_box(st, Vector3(0, 0.5, 0), Vector3(0.04, 1.0, 0.04), WOOD)
	add_box(st, Vector3(0, 1.0, 0), Vector3(0.09, 0.05, 0.09), Color(0.8, 0.66, 0.2))
	var mesh := st.commit()
	var mat := StandardMaterial3D.new()
	mat.vertex_color_use_as_albedo = true
	mat.vertex_color_is_srgb = true
	mesh.surface_set_material(0, mat)
	return mesh


## Contour rectangulaire `width` × `depth` (épaisseur `t` en mètres) posé à plat, centré,
## pour l'anneau de sélection d'un régiment.
static func outline(width: float, depth: float, t: float) -> ArrayMesh:
	var st := SurfaceTool.new()
	st.begin(Mesh.PRIMITIVE_TRIANGLES)
	var hw := width * 0.5
	var hd := depth * 0.5
	var quads := [
		Rect2(-hw, -hd, width, t), Rect2(-hw, hd - t, width, t),
		Rect2(-hw, -hd, t, depth), Rect2(hw - t, -hd, t, depth),
	]
	for q in quads:
		var r: Rect2 = q
		var a := Vector3(r.position.x, 0, r.position.y)
		var b := Vector3(r.end.x, 0, r.position.y)
		var c := Vector3(r.end.x, 0, r.end.y)
		var d := Vector3(r.position.x, 0, r.end.y)
		for v in [a, b, c, a, c, d]:
			st.set_normal(Vector3.UP)
			st.add_vertex(v)
	return st.commit()
