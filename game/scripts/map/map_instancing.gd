class_name MapInstancing
extends RefCounted

## Aides communes aux couches d'instances de la carte (MultiMesh) : création, écriture dans le
## tampon plat, graines pseudo-aléatoires stables dérivées d'un texte. Purement visuel.

## Floats par instance : transformation 3 × 4, puis données d'instance (couleur ou custom) 4 floats.
const TRANSFORM_FLOATS := 12
const INSTANCE_FLOATS := 16


## MultiMesh 3D de `count` instances de `mesh`. `buffer` (optionnel) est posé après le nombre.
static func make(mesh: Mesh, count: int, custom_data := false, colors := false, buffer := PackedFloat32Array()) -> MultiMesh:
	var multimesh := MultiMesh.new()
	multimesh.transform_format = MultiMesh.TRANSFORM_3D
	multimesh.use_colors = colors
	multimesh.use_custom_data = custom_data
	multimesh.mesh = mesh
	multimesh.instance_count = count
	if not buffer.is_empty():
		multimesh.buffer = buffer
	return multimesh


## Transformation au format du tampon MultiMesh (3 lignes de base + origine), à partir de `offset`.
static func write_transform(buffer: PackedFloat32Array, offset: int, t: Transform3D) -> void:
	buffer[offset] = t.basis.x.x
	buffer[offset + 1] = t.basis.y.x
	buffer[offset + 2] = t.basis.z.x
	buffer[offset + 3] = t.origin.x
	buffer[offset + 4] = t.basis.x.y
	buffer[offset + 5] = t.basis.y.y
	buffer[offset + 6] = t.basis.z.y
	buffer[offset + 7] = t.origin.y
	buffer[offset + 8] = t.basis.x.z
	buffer[offset + 9] = t.basis.y.z
	buffer[offset + 10] = t.basis.z.z
	buffer[offset + 11] = t.origin.z


## Données d'instance (4 floats) à `offset`.
static func write_custom(buffer: PackedFloat32Array, offset: int, c: Color) -> void:
	buffer[offset] = c.r
	buffer[offset + 1] = c.g
	buffer[offset + 2] = c.b
	buffer[offset + 3] = c.a


## Graine stable (positive) d'un texte : même texte, mêmes tirages d'une exécution à l'autre.
static func text_seed(text: String) -> int:
	return absi(text.hash())


## Graine d'un hameau (variante, lacet, taille, incendie), tirée de son nom et de sa position de données.
static func hamlet_seed(hamlet: Dictionary) -> int:
	return text_seed(str(hamlet["name"]) + str(hamlet["px"]))
