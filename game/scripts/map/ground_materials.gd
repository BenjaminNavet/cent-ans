class_name GroundMaterials
extends RefCounted

## HB2 (ADR 0143) : matières de sol de la carte de campagne (textures fal.ai vues du ciel,
## tuilables), empaquetées en deux tableaux par `cent-ans assets ground-materials pack` :
## albédo (luminance moyenne égalisée, la carte de couleur donne la teinte) et normale OpenGL
## (R, G) + rugosité (B). La correspondance id → couche vient du manifeste généré
## `data/art/ground_materials_pack.json` (catalogue source `data/art/ground_materials.yaml`).
## Pas encore branché dans `terrain.gdshader` (HB3).

const MANIFEST_FILE := "art/ground_materials_pack.json"

static var _manifest: Dictionary = {}
static var _manifest_loaded: bool = false


## Manifeste (dossier de données du jeu, puis `data/` du dépôt) ; {} s'il est introuvable.
static func manifest() -> Dictionary:
	if _manifest_loaded:
		return _manifest
	_manifest_loaded = true
	if DataFile.exists(MANIFEST_FILE):
		var parsed: Variant = DataFile.read_json(MANIFEST_FILE)
		if parsed is Dictionary and (parsed as Dictionary).get("layers") is Array:
			_manifest = parsed
			return _manifest
	push_warning("GroundMaterials: %s introuvable" % MANIFEST_FILE)
	return _manifest


## Identifiant → couche des tableaux.
static func layer_map() -> Dictionary:
	var result := {}
	for layer in manifest().get("layers", []):
		var entry := layer as Dictionary
		result[str(entry.get("id", ""))] = int(entry.get("layer", -1))
	return result


## {"albedo": TextureLayered, "normal": TextureLayered, "layers": id → couche,
## "means": PackedVector3Array (albédo linéaire moyen par couche)} ; {} si indisponibles
## ou si le nombre de couches diffère du manifeste.
static func load_arrays() -> Dictionary:
	var data := manifest()
	if data.is_empty():
		return {}
	var albedo_path := str(data.get("albedo", ""))
	var normal_path := str(data.get("normal", ""))
	if not ResourceLoader.exists(albedo_path) or not ResourceLoader.exists(normal_path):
		push_warning("GroundMaterials: tableaux absents (%s, %s)" % [albedo_path, normal_path])
		return {}
	var albedo := load(albedo_path) as TextureLayered
	var normal := load(normal_path) as TextureLayered
	var count: int = (data["layers"] as Array).size()
	if albedo == null or normal == null or albedo.get_layers() != count or normal.get_layers() != count:
		push_warning("GroundMaterials: tableaux invalides (couches ≠ %d)" % count)
		return {}
	var means := PackedVector3Array()
	means.resize(count)
	for layer in data["layers"]:
		var entry := layer as Dictionary
		var m: Array = entry.get("mean_linear", [0.18, 0.18, 0.18])
		means[int(entry["layer"])] = Vector3(float(m[0]), float(m[1]), float(m[2]))
	return {"albedo": albedo, "normal": normal, "layers": layer_map(), "means": means}
