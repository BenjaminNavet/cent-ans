class_name Ga3Vegetation
extends RefCounted

## Lot GA3-L2 : végétation réaliste de la carte de campagne (images générées, voir
## `tools/blender_scripts/ga3_vegetation.py` et `docs/archive/chantiers.md`). Purement visuel.
##
## - Imposteurs des chênes, hêtres et sapins : grille `IMPOSTOR_ALBEDO` / `IMPOSTOR_NORMAL`, même
##   format que l'atlas FC2 (3 lignes × 8 azimuts de 256², cadrage `ortho` / `foot` identique).
##   Ils remplacent aussi les cartes de feuillage des arbres proches (FC5) : 2 triangles au lieu
##   de ≈ 250 par arbre proche.
## - Atlas des cartes de feuillage `LEAF_CARDS` (même normalisation que FC5), employé pour
##   les cartes proches.
## - Touffe d'herbe dense `GRASS_TUFT` (`GroundClutter`, teinte du shader conservée).
## - Rochers TRELLIS (`ROCKS`, 3 variantes × 3 niveaux de détail) semés par `GroundClutter`.
##
## Chaque ressource manquante retombe sur l'existant (FC).

const DIR := "res://assets/textures/vegetation/ga3/"
const IMPOSTOR_ALBEDO := DIR + "ga3_impostors_albedo.png"
const IMPOSTOR_NORMAL := DIR + "ga3_impostors_normal.png"
const LEAF_CARDS := DIR + "ga3_leaf_cards.png"
const GRASS_TUFT := DIR + "ga3_grass_tuft.png"
const ROCK_DIR := "res://assets/models/vegetation/ga3/"
## Variantes de rochers : `ROCK_DIR + "<nom>_lod<n>.glb"` (n = 0, 1, 2 : 120, 60, 18 triangles).
const ROCKS: Array[String] = ["ga3_rock_a", "ga3_rock_b", "ga3_rock_c"]
const ROCK_LODS := 3

static var _forced: int = -1  # tests : -1 = défaut (actif), 0 = inactif, 1 = actif


## Végétation GA3 active.
static func enabled() -> bool:
	if _forced >= 0:
		return _forced == 1
	return true


## Imposteurs GA3 aussi pour les arbres proches (à la place des cartes FC5) .
static func near_impostors() -> bool:
	return enabled() and has_impostors()


## Tests : force l'état (`true` / `false`), ou `null` pour revenir à l'état par défaut (actif).
static func force(state: Variant) -> void:
	_forced = -1 if state == null else int(bool(state))


static func has_impostors() -> bool:
	return ResourceLoader.exists(IMPOSTOR_ALBEDO) and ResourceLoader.exists(IMPOSTOR_NORMAL)


## Chemin à employer : `ga3_path` si GA3 est actif et la ressource présente, sinon `fallback`.
static func pick(ga3_path: String, fallback: String) -> String:
	return ga3_path if enabled() and ResourceLoader.exists(ga3_path) else fallback


## Maillages des rochers : [variante][niveau] (tableau vide si GA3 inactif ou GLB absents).
static func rock_meshes() -> Array:
	var result: Array = []
	if not enabled():
		return result
	for name in ROCKS:
		var lods: Array = []
		for lod in ROCK_LODS:
			var mesh := _load_mesh(ROCK_DIR + "%s_lod%d.glb" % [name, lod])
			if mesh == null:
				break
			lods.append(mesh)
		if lods.size() == ROCK_LODS:
			result.append(lods)
	return result


static var _mesh_cache: Dictionary = {}


## Premier maillage d'un GLB (fusion des surfaces non requise : TRELLIS n'en produit qu'une).
static func _load_mesh(path: String) -> Mesh:
	if _mesh_cache.has(path):
		return _mesh_cache[path]
	var mesh: Mesh = null
	if ResourceLoader.exists(path):
		var scene := load(path) as PackedScene
		var root := scene.instantiate() if scene != null else null
		if root != null:
			var mi := root.find_children("*", "MeshInstance3D", true, false)
			if not mi.is_empty():
				mesh = (mi[0] as MeshInstance3D).mesh
			root.free()
	_mesh_cache[path] = mesh
	return mesh
