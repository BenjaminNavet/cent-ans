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


## Entrées (liste de dictionnaires) pour un type et une famille : famille exacte, sinon `*`.
static func entries_for(kind: String, family: String) -> Array:
	var by_family: Dictionary = (document().get("table", {}) as Dictionary).get(kind, {})
	var list: Variant = by_family.get(family, by_family.get(WILDCARD, []))
	return list if list is Array else []


## Entrée retenue pour un lieu : variante tirée par `variant_index` (modulo la taille de la liste) ;
## dictionnaire vide si aucune.
static func entry_for(kind: String, family: String, variant_index: int) -> Dictionary:
	var list := entries_for(kind, family)
	if list.is_empty():
		return {}
	return list[absi(variant_index) % list.size()]


static func near_distance(entry: Dictionary) -> float:
	return float(entry.get("near_distance", default_value("near_distance", 400.0)))
