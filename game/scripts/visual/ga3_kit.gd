class_name Ga3Kit
extends RefCounted

## Lot GA3-L1 (ADR 0140) : variantes générées (image → TRELLIS, `data/art/ga3_decor.json`) des
## modèles du kit BR1 dans le décor de bataille. `BuildingKit.Batch.add` demande ici si un modèle
## du kit intact (maison, église, moulin, tente, puits, charrette) est remplacé par sa variante
## GA3 (`share` du manifeste, tirage déterministe sur la position) ; la variante reprend l'emprise
## visée par le kit. Les instances GA3 sont posées par tuiles de `CELL` m, trois `MultiMesh` par
## tuile et par modèle (LOD0/1/2 selon la distance). Désactivé par `--no-ga3` après `--`, sous la
## neige (pas de variante enneigée) et hors bataille (`active` posé par `BattleTerrain.build`).
## Purement visuel.

const DIR := "res://assets/models/props_ga/"
## Côté des tuiles (m) et fin des LOD0 / LOD1 (m, avant `RenderQuality.battle_lod_scale`).
const CELL := 120.0
const LOD_ENDS: Array[float] = [90.0, 280.0]
const LOD_MARGIN := 8.0
## Enfoncement des bâtiments générés (m) : ils n'ont pas de fondations, le kit en a jusqu'à -2 m.
const BUILDING_SINK := 0.35
## Anisotropie maximale de l'emprise (la texture cuite supporte mal un étirement fort).
const MAX_STRETCH := 1.15

## Vrai pendant une bataille (posé par `BattleTerrain.build`, `--no-ga3` le coupe).
static var active := false
static var _manifest: Dictionary = {}
static var _loaded := false
static var _meshes: Dictionary = {}  # "nom|lod" → Mesh


static func clear_cache() -> void:
	_manifest.clear()
	_loaded = false
	_meshes.clear()


## Option de ligne de commande : `--no-ga3` revient au kit seul.
static func requested() -> bool:
	return not OS.get_cmdline_user_args().has("--no-ga3")


static func manifest() -> Dictionary:
	if not _loaded:
		_loaded = true
		var path := DIR + "manifest.json"
		if FileAccess.file_exists(path):
			var parsed: Variant = DataFile.parse_file(path)
			if parsed is Dictionary:
				_manifest = parsed
	return _manifest


## Variantes branchées d'un type du kit (`cottage`, `church`…), triées.
static func variants_of(kit_kind: String) -> Array:
	var out := []
	var all := manifest()
	for model_name in all:
		var entry: Dictionary = all[model_name]
		if bool(entry.get("wired", false)) and str(entry["kit_kind"]) == kit_kind:
			out.append(model_name)
	out.sort()
	return out


## Variante GA3 qui remplace le modèle du kit `kit_model` posé en `xform`, ou "" (kit gardé).
static func variant_for(kit_model: String, xform: Transform3D) -> String:
	if not active:
		return ""
	var kit_entry: Dictionary = BuildingKit.manifest().get(kit_model, {})
	if kit_entry.is_empty() or bool(kit_entry.get("ruined", false)):
		return ""
	var candidates := variants_of(str(kit_entry["kind"]))
	if candidates.is_empty():
		return ""
	var key := int(xform.origin.x * 7.0) * 92821 + int(xform.origin.z * 7.0)
	var pick: String = candidates[int(BuildingKit.hash01(key, 41) * candidates.size()) % candidates.size()]
	if BuildingKit.hash01(key, 83) >= float(manifest()[pick].get("share", 0.0)):
		return ""
	return pick


## Transformation de la variante `ga3_model` à la place de `kit_model` posé en `xform` : même
## orientation ; mise à l'échelle selon `fit` du manifeste : `footprint` = emprise visée (échelle
## du kit × emprise du kit), étirement borné ; `length` = uniforme sur la longueur (±20 %) ;
## `real` = taille réelle générée (accessoires, moulin). Les accessoires posés sans échelle par
## le kit gardent aussi la taille réelle.
static func fitted_transform(ga3_model: String, kit_model: String, xform: Transform3D) -> Transform3D:
	var kit_entry: Dictionary = BuildingKit.manifest().get(kit_model, {})
	var entry: Dictionary = manifest().get(ga3_model, {})
	var scale := xform.basis.get_scale()
	var rot := xform.basis.orthonormalized()
	var origin := xform.origin
	var fit := str(entry.get("fit", "footprint"))
	if fit == "real" or (absf(scale.x - 1.0) < 0.01 and absf(scale.z - 1.0) < 0.01):
		return Transform3D(rot, origin)
	if fit == "length":
		var s := clampf(scale.x * float(kit_entry["length"]) / float(entry["length"]), 0.8, 1.2)
		return Transform3D(rot * Basis.from_scale(Vector3.ONE * s), origin - Vector3(0, BUILDING_SINK, 0))
	var sx := scale.x * float(kit_entry["length"]) / float(entry["length"])
	var sz := scale.z * float(kit_entry["depth"]) / float(entry["depth"])
	var mean := sqrt(sx * sz)
	sx = clampf(sx, mean / MAX_STRETCH, mean * MAX_STRETCH)
	sz = clampf(sz, mean / MAX_STRETCH, mean * MAX_STRETCH)
	var sy := clampf(mean, 0.75, 1.3)
	return Transform3D(rot * Basis.from_scale(Vector3(sx, sy, sz)), origin - Vector3(0, BUILDING_SINK, 0))


## Maillage importé d'un LOD (premier `MeshInstance3D` du glb), matériau d'origine (albédo cuit).
static func mesh(model_name: String, lod: int) -> Mesh:
	var key := "%s|%d" % [model_name, lod]
	if _meshes.has(key):
		return _meshes[key]
	var found: Mesh = null
	var path := DIR + "%s_lod%d.glb" % [model_name, lod]
	if ResourceLoader.exists(path):
		var scene := load(path) as PackedScene
		if scene != null:
			var root := scene.instantiate()
			for child in root.find_children("*", "MeshInstance3D", true, false):
				found = (child as MeshInstance3D).mesh
				break
			root.free()
	_meshes[key] = found
	return found


## Instances GA3 d'un `BuildingKit.Batch` : tuiles de `CELL` m, trois LOD par tuile.
class Batch:
	extends RefCounted

	var visibility_end := 0.0
	var _items: Dictionary = {}  # nom → Array[Transform3D]
	## nom → Array[[tuile, indice dans la tuile]] (après build)
	var _slots: Dictionary = {}
	## "nom|tuile" → Array[MultiMeshInstance3D] (LOD0..2)
	var _tiles: Dictionary = {}

	func _init(p_visibility_end: float = 0.0) -> void:
		visibility_end = p_visibility_end

	func add(model_name: String, xform: Transform3D) -> Array:
		if not _items.has(model_name):
			_items[model_name] = []
		(_items[model_name] as Array).append(xform)
		return [model_name, (_items[model_name] as Array).size() - 1]

	func count() -> int:
		var total := 0
		for model_name in _items:
			total += (_items[model_name] as Array).size()
		return total

	func build(parent: Node3D) -> void:
		var k := RenderQuality.battle_lod_scale
		var ends: Array[float] = [Ga3Kit.LOD_ENDS[0] * k, Ga3Kit.LOD_ENDS[1] * k, visibility_end]
		for model_name in _items:
			var transforms: Array = _items[model_name]
			var groups: Dictionary = {}  # Vector2i → Array[indice]
			for i in transforms.size():
				var o: Vector3 = (transforms[i] as Transform3D).origin
				var cell := Vector2i(floori(o.x / Ga3Kit.CELL), floori(o.z / Ga3Kit.CELL))
				if not groups.has(cell):
					groups[cell] = []
				(groups[cell] as Array).append(i)
			var slots := []
			slots.resize(transforms.size())
			for cell in groups:
				var members: Array = groups[cell]
				var centre := Vector3((cell.x + 0.5) * Ga3Kit.CELL, 0.0, (cell.y + 0.5) * Ga3Kit.CELL)
				var total := 0.0
				for i in members:
					total += (transforms[i] as Transform3D).origin.y
				centre.y = total / members.size()
				var lods: Array = []
				for lod in 3:
					var mesh := Ga3Kit.mesh(model_name, lod)
					if mesh == null:
						continue
					var mm := MultiMesh.new()
					mm.transform_format = MultiMesh.TRANSFORM_3D
					mm.mesh = mesh
					mm.instance_count = members.size()
					for j in members.size():
						var xform: Transform3D = transforms[members[j]]
						mm.set_instance_transform(j, Transform3D(xform.basis, xform.origin - centre))
					var mmi := MultiMeshInstance3D.new()
					mmi.name = "Ga3_%s_lod%d_%d_%d" % [model_name, lod, cell.x, cell.y]
					mmi.multimesh = mm
					mmi.position = centre
					mmi.visibility_range_begin = 0.0 if lod == 0 else ends[lod - 1]
					mmi.visibility_range_begin_margin = 0.0 if lod == 0 else Ga3Kit.LOD_MARGIN
					mmi.visibility_range_end = ends[lod]
					mmi.visibility_range_end_margin = 0.0 if lod == 2 else Ga3Kit.LOD_MARGIN
					parent.add_child(mmi)
					lods.append(mmi)
				var tile_key := "%s|%d,%d" % [model_name, cell.x, cell.y]
				_tiles[tile_key] = lods
				for j in members.size():
					slots[members[j]] = [tile_key, j, centre]
			_slots[model_name] = slots

	## Transformation d'une poignée (copie tenue côté CPU : lisible aussi sans serveur de rendu).
	func get_transform(handle: Array) -> Transform3D:
		var items: Array = _items.get(handle[0], [])
		var index := int(handle[1])
		return items[index] if index >= 0 and index < items.size() else Transform3D()

	func set_transform(handle: Array, xform: Transform3D) -> void:
		var items: Array = _items.get(handle[0], [])
		var index := int(handle[1])
		if index < 0 or index >= items.size():
			return
		items[index] = xform
		var slot: Variant = _slot(handle)
		if slot == null:
			return
		for mmi in _tiles[slot[0]]:
			(mmi as MultiMeshInstance3D).multimesh.set_instance_transform(int(slot[1]), Transform3D(xform.basis, xform.origin - (slot[2] as Vector3)))

	## Nombre de `MultiMeshInstance3D` posés (tuiles × LOD).
	func instance_nodes() -> int:
		var total := 0
		for tile_key in _tiles:
			total += (_tiles[tile_key] as Array).size()
		return total

	func _slot(handle: Array) -> Variant:
		var slots: Array = _slots.get(handle[0], [])
		var index := int(handle[1])
		if index < 0 or index >= slots.size():
			return null
		return slots[index]
