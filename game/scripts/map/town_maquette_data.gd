class_name TownMaquetteData
extends RefCounted

## Lot GC2 (ADR 0158) : données des maquettes stylisées des lieux de la carte de campagne
## (`data/art/town_maquettes.json`, schéma `town_maquettes.schema.json`). Tailles monde par type,
## portées, réduction des voisins trop proches, famille d'architecture d'une province (culture,
## puis région, puis religion, sinon la famille par défaut), nom du modèle avec repli sur la
## famille par défaut quand le fichier manque. Purement visuel ; lu une fois, sans repli chiffré
## (fichier absent : aucune maquette, avertissement).
##
## Style des lieux : `map.town_style` de `data/ui/campaign_map.json` (`maquette` ou `real`),
## remplacé par `--town-style=real|maquette` après `--`.

const DATA_FILE := "art/town_maquettes.json"
const MODELS_DIR := "res://assets/models/settlements/"
const STYLE_MAQUETTE := "maquette"
const STYLE_REAL := "real"
const KINDS: Array[String] = ["city", "town", "castle", "abbey", "village"]

static var _doc: Dictionary = {}
static var _loaded := false
static var _style := ""
static var _province_family: Dictionary = {}  # id de province → {family, subfamily}
static var _lookup: Dictionary = {}  # "cultures|cul_x" → famille
static var _model_names: Dictionary = {}  # "type|famille|variante" → nom de modèle


static func clear_cache() -> void:
	_doc = {}
	_loaded = false
	_style = ""
	_province_family.clear()
	_lookup.clear()
	_model_names.clear()


static func _read_json(relative: String) -> Variant:
	return DataFile.read_json(relative) if DataFile.exists(relative) else null


static func document() -> Dictionary:
	if not _loaded:
		_loaded = true
		var parsed: Variant = _read_json(DATA_FILE)
		if parsed is Dictionary:
			_doc = parsed
			for family: String in (_doc.get("families", {}) as Dictionary):
				var lists: Dictionary = _doc["families"][family]
				for list_name in ["cultures", "regions", "religions"]:
					for value in lists.get(list_name, []):
						_lookup["%s|%s" % [list_name, value]] = family
		else:
			push_warning("TownMaquetteData: %s introuvable, pas de maquette de lieu" % DATA_FILE)
	return _doc


## Style des lieux de la carte : `maquette` (défaut des données) ou `real` (villes 1:1).
static func style() -> String:
	if _style == "":
		_style = str(ArmyFigures.map_settings().get("town_style", STYLE_REAL))
		_style = CmdArgs.value("--town-style", _style)
		if _style != STYLE_MAQUETTE:
			_style = STYLE_REAL
		if _style == STYLE_MAQUETTE and document().is_empty():
			_style = STYLE_REAL  # sans données, les villes 1:1 restent
	return _style


static func enabled() -> bool:
	return style() == STYLE_MAQUETTE


## Tests : force le style (suivi de `clear_cache()` pour revenir aux données).
static func set_style(value: String) -> void:
	_style = value


static func kind_of(entry: Dictionary) -> String:
	var kind := str(entry.get("kind", "village"))
	return kind if kind in KINDS else "village"


## Largeur monde (unités) de la maquette d'un type de lieu.
static func width(kind: String) -> float:
	return float((document().get("sizes", {}) as Dictionary).get(kind, 0.0))


## Échelle à appliquer au modèle du kit pour atteindre `width(kind)`.
static func model_scale(kind: String) -> float:
	var native := float((document().get("native_width", {}) as Dictionary).get(kind, 0.0))
	return width(kind) / native if native > 0.0 else 0.0


## Gain de taille d'un lieu selon son poids (`weight_gain` : de `min` au poids bas de son type à
## `max` au poids haut, en racine carrée) ; 1 si le bloc est absent.
static func weight_gain(kind: String, weight: float) -> float:
	var block: Dictionary = document().get("weight_gain", {})
	var bounds: Array = block.get(kind, [])
	if bounds.size() < 2 or float(bounds[1]) <= float(bounds[0]):
		return 1.0
	var t := clampf((weight - float(bounds[0])) / (float(bounds[1]) - float(bounds[0])), 0.0, 1.0)
	return lerpf(float(block.get("min", 1.0)), float(block.get("max", 1.0)), sqrt(t))


## Réglage d'accessoire de la carte généralisée (`props` : hameaux, moulins, fumées grossis avec
## les maquettes) ; `fallback` (valeur 1:1 de `MapPropScale`) en style `real` ou si la clé manque.
static func prop(key: String, fallback: float) -> float:
	if not enabled():
		return fallback
	return float((document().get("props", {}) as Dictionary).get(key, fallback))


static func landmark_scale() -> float:
	return float(document().get("landmark_scale", 1.0))


## Distance au-delà de laquelle la maquette d'un type n'est plus dessinée.
static func visibility(kind: String) -> float:
	return float((document().get("visibility", {}) as Dictionary).get(kind, 0.0))


## Distance du rig au-delà de laquelle les maquettes d'un type ne portent plus d'ombre (INF si le
## type n'a pas de `shadow_range` : ombre gardée, dans la limite du préréglage de qualité).
static func shadow_range(kind: String) -> float:
	var ranges: Dictionary = document().get("shadow_range", {})
	return float(ranges[kind]) if ranges.has(kind) and (ranges[kind] is float or ranges[kind] is int) else INF


static func fade_margin() -> float:
	return float((document().get("visibility", {}) as Dictionary).get("fade_margin", 0.0))


static func tile_size() -> float:
	return maxf(float(document().get("tile_size", 256.0)), 1.0)


static func default_family() -> String:
	return str(document().get("default_family", "west"))


static func variants() -> Array:
	return document().get("variants", [])


## Famille d'architecture : culture, puis région, puis religion, sinon la famille par défaut.
static func family_for(culture: String, region: String, religion: String) -> String:
	document()
	for pair in [["cultures", culture], ["regions", region], ["religions", religion]]:
		var family: Variant = _lookup.get("%s|%s" % pair)
		if family != null:
			return str(family)
	return default_family()


## Famille de la province (`data/provinces/<id>.json`), famille par défaut si inconnue.
static func family_of_province(province_id: String) -> String:
	if province_id == "":
		return default_family()
	return str(_province_info(province_id)["family"])


## Sous-famille d'architecture des modèles générés (DN-MAQ, `DnCampaignModels`), "" si aucune.
static func subfamily_of_province(province_id: String) -> String:
	if province_id == "":
		return ""
	return str(_province_info(province_id)["subfamily"])


static func _province_info(province_id: String) -> Dictionary:
	if not _province_family.has(province_id):
		var parsed: Variant = _read_json("provinces/%s.json" % province_id)
		var province: Dictionary = parsed if parsed is Dictionary else {}
		var culture := str(province.get("culture", ""))
		var region := str(province.get("region", ""))
		var religion := str(province.get("religion", ""))
		var family := family_for(culture, region, religion)
		_province_family[province_id] = {"family": family, "subfamily": DnCampaignModels.subfamily_for(family, culture, region, religion)}
	return _province_family[province_id]


static func variant_of(id: String) -> String:
	var list := variants()
	return str(list[absi(id.hash()) % list.size()]) if not list.is_empty() else ""


## Lacet (radians) d'un lieu, tiré par son id (déterministe).
static func yaw_of(id: String) -> float:
	return float(absi((id + "|yaw").hash()) % 360) * PI / 180.0


## Nom de modèle (sans dossier ni extension) pour un type, une famille et une variante :
## `<type>_<famille>_<variante>`. Sans test de présence (voir `model_name`).
static func model_name_raw(kind: String, family: String, variant: String) -> String:
	return "%s_%s_%s" % [kind, family if family != "" else default_family(), variant]


## Nom du modèle à charger : celui de la famille s'il est importé, sinon celui de la famille par
## défaut, sinon l'ancien kit occidental `<type>_<variante>` (lot C6).
static func model_name(kind: String, family: String, variant: String) -> String:
	var key := "%s|%s|%s" % [kind, family, variant]
	if not _model_names.has(key):
		var name := model_name_raw(kind, family, variant)
		if not ResourceLoader.exists(MODELS_DIR + name + ".glb"):
			name = model_name_raw(kind, default_family(), variant)
		if not ResourceLoader.exists(MODELS_DIR + name + ".glb"):
			name = "%s_%s" % [kind, variant]
		_model_names[key] = name
	return _model_names[key]


## Réduction des lieux trop proches. `centers` et `radii` (demi-largeurs de base) sont donnés par
## ordre d'importance décroissante (ordre de `SettlementData.settlements` : type, poids, id) ;
## `fixed[i]` = 1 : jamais réduit (ville emblématique). Un lieu dont le disque touche (à `margin`
## près) celui d'un lieu plus important est réduit jusqu'à ce qu'il ne le touche plus, au plus
## jusqu'à `min_scale`. Rend le facteur par lieu (1 = taille de base). Déterministe.
static func solve_collisions(centers: PackedVector2Array, radii: PackedFloat32Array, fixed: PackedByteArray, min_scale: float, margin: float) -> PackedFloat32Array:
	var count := centers.size()
	var scales := PackedFloat32Array()
	scales.resize(count)
	scales.fill(1.0)
	var largest := 0.0
	for r in radii:
		largest = maxf(largest, r)
	var cell := maxf(largest * 2.0 + margin, 1.0)
	var grid := {}  # cellule → indices déjà posés
	for i in count:
		var c := centers[i]
		var cx := int(floor(c.x / cell))
		var cy := int(floor(c.y / cell))
		if radii[i] > 0.0 and (i >= fixed.size() or fixed[i] == 0):
			var allowed := radii[i]
			for dy in [-1, 0, 1]:
				for dx in [-1, 0, 1]:
					for j: int in grid.get(Vector2i(cx + dx, cy + dy), PackedInt32Array()):
						allowed = minf(allowed, c.distance_to(centers[j]) - radii[j] * scales[j] - margin)
			scales[i] = clampf(allowed / radii[i], min_scale, 1.0)
		var key := Vector2i(cx, cy)
		if not grid.has(key):
			grid[key] = PackedInt32Array()
		grid[key].append(i)
	return scales


## Un lieu dont le centre est à moins de `absorb_ratio` × le rayon d'une ville emblématique n'a
## pas de maquette propre (0 : jamais).
static func collision_absorb_ratio() -> float:
	return float((document().get("collision", {}) as Dictionary).get("absorb_ratio", 0.0))


static func collision_min_scale() -> float:
	return float((document().get("collision", {}) as Dictionary).get("min_scale", 1.0))


static func collision_margin() -> float:
	return float((document().get("collision", {}) as Dictionary).get("margin", 0.0))
