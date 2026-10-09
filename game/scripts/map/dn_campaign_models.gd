class_name DnCampaignModels
extends RefCounted

## Lot DN camp-bati (D1) : table des glb générés semi-réalistes des lieux de la carte de campagne
## (`data/art/dn_campaign_models.json`, schéma `art_dn_campaign_models.schema.json`). Type de lieu
## × famille d'architecture → liste de modèles ; `TownMaquetteLayer` les affiche aux zooms proche
## et moyen, la maquette stylisée gardant le lointain. Table vide ou fichier absent : aucun
## remplacement, comportement inchangé. Purement visuel.

const DATA_FILE := "art/dn_campaign_models.json"
const WILDCARD := "*"

static var _doc: Dictionary = {}
static var _loaded := false


static func clear_cache() -> void:
	_doc = {}
	_loaded = false


static func document() -> Dictionary:
	if not _loaded:
		_loaded = true
		if DataFile.exists(DATA_FILE):
			var parsed: Variant = DataFile.read_json(DATA_FILE)
			if parsed is Dictionary:
				_doc = parsed
	return _doc


## Tests : remplace le document (table fournie en mémoire).
static func set_document(doc: Dictionary) -> void:
	_doc = doc
	_loaded = true


static func is_empty() -> bool:
	return (document().get("table", {}) as Dictionary).is_empty()


static func default_value(key: String, fallback: float) -> float:
	return float((document().get("defaults", {}) as Dictionary).get(key, fallback))


static func fade_margin() -> float:
	return default_value("fade_margin", 60.0)


static func banner_pole_ratio() -> float:
	return default_value("banner_pole_ratio", 0.12)


## Sous-famille d'architecture d'une province de la famille `family` (DN-MAQ) : première règle de
## `subfamilies` (l'ordre est la priorité) dont la culture, la région ou la religion correspond ;
## chaîne vide sinon.
static func subfamily_for(family: String, culture: String, region: String, religion: String) -> String:
	var rules: Variant = document().get("subfamilies", [])
	if not rules is Array:
		return ""
	for rule: Dictionary in rules:
		if str(rule.get("family", "")) != family:
			continue
		if culture in rule.get("cultures", []) or region in rule.get("regions", []) or religion in rule.get("religions", []):
			return str(rule.get("id", ""))
	return ""


## Clés de recherche dans la table, par priorité : sous-famille, ses `also`, famille, `*`.
static func lookup_keys(family: String, subfamily: String) -> Array:
	var keys: Array = []
	if subfamily != "":
		keys.append(subfamily)
		for rule: Dictionary in document().get("subfamilies", []):
			if str(rule.get("id", "")) == subfamily:
				keys.append_array(rule.get("also", []))
				break
	keys.append(family)
	keys.append(WILDCARD)
	return keys


## Entrées (liste de dictionnaires) pour un type, une famille et une sous-famille éventuelle : la
## première clé de `lookup_keys` qui existe pour ce type.
static func entries_for(kind: String, family: String, subfamily: String = "") -> Array:
	var by_family: Dictionary = (document().get("table", {}) as Dictionary).get(kind, {})
	for key: String in lookup_keys(family, subfamily):
		var list: Variant = by_family.get(key)
		if list is Array and not (list as Array).is_empty():
			return list
	return []


## Entrée retenue pour un lieu : variante tirée par `variant_index` (modulo la taille de la liste) ;
## dictionnaire vide si aucune.
static func entry_for(kind: String, family: String, variant_index: int, subfamily: String = "") -> Dictionary:
	var list := entries_for(kind, family, subfamily)
	if list.is_empty():
		return {}
	return list[absi(variant_index) % list.size()]


## Nom de modèle à charger pour une entrée : les glb ingérés existent en `_lod0/1/2` ; sans suffixe
## `_lodN` dans le chemin, on prend le niveau `lod` de l'entrée (défaut `defaults.lod`, 1 : bon
## compromis polygones / lisibilité pour des centaines de lieux instanciés).
static func model_name(entry: Dictionary) -> String:
	var path := str(entry.get("path", ""))
	if path.contains("_lod"):
		return path
	return "%s_lod%d" % [path, int(entry.get("lod", default_value("lod", 1.0)))]


## Les albédos baked par TRELLIS sont sombres (beaucoup de noir) : sur le sol de la carte les lieux
## ressortent en silhouettes noires. Gain d'albédo (`albedo_gain` de l'entrée, sinon des défauts) et
## rugosité maximale appliqués à une copie du matériau de chaque surface.
static func brighten(mesh: Mesh, entry: Dictionary) -> void:
	var gain := float(entry.get("albedo_gain", default_value("albedo_gain", 1.0)))
	for surface in mesh.get_surface_count():
		var source := mesh.surface_get_material(surface) as BaseMaterial3D
		if source == null:
			continue
		var material := source.duplicate() as BaseMaterial3D
		material.albedo_color = Color(gain, gain, gain, 1.0)
		material.roughness = 1.0
		mesh.surface_set_material(surface, material)


static func near_distance(entry: Dictionary) -> float:
	return float(entry.get("near_distance", default_value("near_distance", 400.0)))
