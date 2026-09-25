class_name LivingPortrait
extends RefCounted

## DA2 (ADR 0056) : portraits vivants. Choisit l'image d'un personnage selon son état courant :
## - figure historique : portrait fixe (`assets/portraits/<id>.png`) tant qu'elle est dans la
##   tranche d'âge de ce portrait ; variante âgée (`assets/portraits/aged/<id>_<tranche>.jpg`) pour
##   les grandes figures listées ; sinon archétype ;
## - autre personnage (né en jeu) : archétype `assets/portraits/archetypes/<rang>_<sexe>_<tranche>_
##   <aire>_<visage>.jpg`, visage choisi par hachage FNV-1a de l'id (stable d'une sauvegarde à
##   l'autre, même « lignée » de visage d'une tranche à l'autre).
## Tout vient de `data/portraits/archetypes.json`. Rendu seulement : l'âge, le rang d'affichage et
## les marques sont lus dans les dictionnaires de la façade (`get_character`, `get_family_tree`).

const DATA_PATH := "portraits/archetypes.json"
const MAP_PATHS_SCRIPT := preload("res://scripts/map/map_paths.gd")
const ASSETS_ROOT := "res://assets/"
const BAND_ORDER := ["child", "young", "adult", "old"]
const START_YEAR := 1337
## Âge minimal représenté par les portraits fixes (les figures nées après 1337 sont peintes vers 20 ans).
const FIXED_MIN_AGE := 20

static var _config: Dictionary = {}
static var _faction_culture: Dictionary = {}
static var _aged: Dictionary = {}  # id → [tranches]
## Tests : simulation / magasin de données injectés (défaut : `SimFacade.sim` / `.store`).
static var sim_override: Object = null
static var store_override: Object = null
static var _heads: Dictionary = {}  # faction → {ruler, heir, house}
static var _heads_stamp := ""
static var _roles: Dictionary = {}  # id → rôle de données (personnages historiques)


static func config() -> Dictionary:
	if _config.is_empty():
		var path := _data_dir().path_join(DATA_PATH)
		if not FileAccess.file_exists(path):
			path = MAP_PATHS_SCRIPT.project_root().path_join("data").path_join(DATA_PATH)
		if FileAccess.file_exists(path):
			var parsed: Variant = JSON.parse_string(FileAccess.get_file_as_string(path))
			if parsed is Dictionary:
				_config = parsed
		for culture in _config.get("cultures", []):
			for faction in (culture as Dictionary).get("factions", []):
				_faction_culture[str(faction)] = str(culture["id"])
		for variant in _config.get("aged_variants", []):
			var id := str(variant["character"])
			if not _aged.has(id):
				_aged[id] = []
			(_aged[id] as Array).append(str(variant["band"]))
	return _config


static func _data_dir() -> String:
	var tree := Engine.get_main_loop() as SceneTree
	if tree != null and tree.root != null:
		var map_paths := tree.root.get_node_or_null("MapPaths")
		if map_paths != null:
			return str(map_paths.get("data_dir"))
	return MAP_PATHS_SCRIPT.default_data_dir()


## Oublie les caches (nouvelle partie, tests).
static func clear_cache() -> void:
	_heads.clear()
	_heads_stamp = ""
	_roles.clear()


# --- Axes ---------------------------------------------------------------------------------------


static func age_band(age: int) -> String:
	for band in config().get("age_bands", []):
		if not (band as Dictionary).has("max_age") or age <= int(band["max_age"]):
			return str(band["id"])
	return "old"


static func culture_of(faction_id: String) -> String:
	config()
	return str(_faction_culture.get(faction_id, _config.get("default_culture", "france")))


## FNV-1a 32 bits sur l'UTF-8 de `text` : stable entre versions de Godot et sauvegardes.
static func fnv1a(text: String) -> int:
	var hash_value := 2166136261
	for byte in text.to_utf8_buffer():
		hash_value = ((hash_value ^ byte) * 16777619) & 0xFFFFFFFF
	return hash_value


static func _band_index(band: String) -> int:
	return BAND_ORDER.find(band)


static func _sex(character: Dictionary) -> String:
	return "female" if str(character.get("sex", "")) == "female" else "male"


# --- Rang d'affichage ---------------------------------------------------------------------------


## Rang d'affichage (clé de `ranks`) : enfant, souverain (dirigeant ou conjoint), rôle de données
## des figures historiques, mots-clés du titre, armée, maison régnante, défaut par sexe.
## `context` : `{ruler, heir, ruler_house, data_role}` (sinon interrogés via la façade).
static func rank_of(character: Dictionary, context: Dictionary = {}) -> String:
	var rules: Dictionary = config().get("rank_rules", {})
	var sex := _sex(character)
	if age_band(int(character.get("age", 30))) == "child":
		return "child"
	var id := str(character.get("id", ""))
	var ruler := str(context.get("ruler", ""))
	if ruler != "" and (id == ruler or str(character.get("spouse", "")) == ruler):
		return "sovereign"
	var role := str(context.get("data_role", ""))
	var by_role: Dictionary = rules.get("data_roles", {})
	if role != "" and by_role.has(role):
		var mapped := str(by_role[role])
		# Un souverain de 1337 qui n'est plus sur le trône (abdication, captivité) reste peint roi ;
		# un conjoint devenu veuf aussi : seules les règles au-dessus promeuvent.
		return mapped
	var title := " %s " % str(character.get("title", "")).to_lower()
	for rule in rules.get("title_keywords", []):
		for word in (rule as Dictionary).get("words", []):
			if title.contains(str(word)):
				return str(rule["rank"])
	if str(character.get("army", "")) != "" and sex == "male":
		return str(rules.get("army_rank", "knight"))
	var house := str(context.get("ruler_house", ""))
	if house != "" and str(character.get("house", "")) == house:
		return str(rules.get("ruler_house_rank", "noble"))
	return str((rules.get("default_rank", {}) as Dictionary).get(sex, "noble"))


## Contexte de rang d'un personnage, lu via la façade (mis en cache par date de campagne).
static func context_for(character: Dictionary) -> Dictionary:
	var context := {}
	var id := str(character.get("id", ""))
	var store := _store()
	if id != "" and store != null and store.has_method("get_character"):
		if not _roles.has(id):
			var data: Dictionary = store.call("get_character", id)
			_roles[id] = str(data.get("role", ""))
		context["data_role"] = _roles[id]
	var faction := str(character.get("faction", ""))
	var sim := _sim()
	if sim == null or faction == "" or id == "" or not sim.has_method("get_family_tree"):
		return context
	var stamp := str(sim.call("get_date_label")) if sim.has_method("get_date_label") else ""
	if stamp != _heads_stamp:
		_heads.clear()
		_heads_stamp = stamp
	if not _heads.has(faction):
		var tree: Dictionary = sim.call("get_family_tree", id, 0, 0)
		var ruler := str(tree.get("ruler", ""))
		var house := ""
		if ruler != "" and sim.has_method("get_character"):
			house = str((sim.call("get_character", ruler) as Dictionary).get("house", ""))
		_heads[faction] = {"ruler": ruler, "heir": str(tree.get("heir", "")), "ruler_house": house}
	context.merge(_heads[faction])
	return context


static func _sim() -> Object:
	if sim_override != null:
		return sim_override
	var tree := Engine.get_main_loop() as SceneTree
	var facade := tree.root.get_node_or_null("SimFacade") if tree != null and tree.root != null else null
	return facade.get("sim") if facade != null else null


static func _store() -> Object:
	if store_override != null:
		return store_override
	var tree := Engine.get_main_loop() as SceneTree
	var facade := tree.root.get_node_or_null("SimFacade") if tree != null and tree.root != null else null
	return facade.get("store") if facade != null else null


# --- Résolution de l'image ----------------------------------------------------------------------


## `{path, kind ("fixed" | "aged" | "archetype" | ""), rank, band, culture, headwear}` : image
## choisie pour l'état courant du personnage (`path` vide si aucune image n'existe).
static func resolve(character: Dictionary, context: Dictionary = {}) -> Dictionary:
	var cfg := config()
	var id := str(character.get("id", ""))
	var age := int(character.get("age", 30))
	var band := age_band(age)
	var rank := rank_of(character, context)
	var culture := culture_of(str(character.get("faction", "")))
	var result := {"path": "", "kind": "", "rank": rank, "band": band, "culture": culture, "headwear": "none",
		"data_role": str(context.get("data_role", ""))}
	var historical := _historical(id, character, band)
	if historical != "":
		result["path"] = historical
		result["kind"] = "aged" if historical.contains("/aged/") else "fixed"
		return result
	if cfg.is_empty():
		return result
	var archetype := _archetype(id, _sex(character), rank, band, culture)
	if not archetype.is_empty():
		result.merge(archetype, true)
		result["kind"] = "archetype"
		result["headwear"] = _rank_headwear(str(archetype["rank"]))
	return result


## Portrait fixe ou variante âgée si la tranche courante en a un ; "" sinon.
static func _historical(id: String, character: Dictionary, band: String) -> String:
	if id == "":
		return ""
	var fixed := PortraitLoader.PORTRAITS_DIR + id + ".png"
	var has_fixed := PortraitLoader.load_texture(fixed) != null
	var birth := int(character.get("birth_year", 0))
	var base_band := age_band(maxi(START_YEAR - birth, FIXED_MIN_AGE)) if birth > 0 else band
	var candidates := {}  # tranche → chemin
	if has_fixed:
		candidates[base_band] = fixed
	var aged_dir := ASSETS_ROOT + str(config().get("aged_dir", "portraits/aged")) + "/"
	for aged_band in _aged.get(id, []):
		var path := aged_dir + "%s_%s.jpg" % [id, aged_band]
		if PortraitLoader.load_texture(path) != null:
			candidates[aged_band] = path
	if candidates.has(band):
		return str(candidates[band])
	if not _aged.has(id):
		return ""  # figure secondaire : archétype dès qu'elle change de tranche
	# Grande figure : la plus récente image dont la tranche ne dépasse pas l'âge courant.
	var best := ""
	var best_index := -1
	for candidate_band in candidates:
		var index := _band_index(str(candidate_band))
		if index <= _band_index(band) and index > best_index:
			best_index = index
			best = str(candidates[candidate_band])
	return best


static func _rank_headwear(rank: String) -> String:
	for entry in config().get("ranks", []):
		if str(entry["id"]) == rank:
			return str(entry.get("headwear", "none"))
	return "none"


## Nombre de visages de la case (0 si la case n'existe pas).
static func _faces(rank: String, sex: String, band: String, culture: String) -> int:
	for cell in config().get("cells", []):
		if str(cell["rank"]) == rank and (cell["sexes"] as Array).has(sex) \
				and (cell["bands"] as Array).has(band) and (cell["cultures"] as Array).has(culture):
			return int(cell["faces"])
	return 0


static func _chain(start: String, links: Dictionary) -> Array:
	var chain: Array = [start]
	var current := start
	while links.has(current) and not chain.has(str(links[current])):
		current = str(links[current])
		chain.append(current)
	return chain


## Archétype existant le plus proche : rang puis replis de rang, tranche puis replis de tranche,
## aire culturelle puis « any », puis les autres aires ; visage = hachage de l'id.
static func _archetype(id: String, sex: String, rank: String, band: String, culture: String) -> Dictionary:
	var cfg := config()
	var fallbacks: Dictionary = cfg.get("fallbacks", {})
	var ranks := _chain(rank, fallbacks.get("rank", {}))
	var bands := [band] if band == "child" else _chain(band, fallbacks.get("band", {}))
	var cultures: Array = [culture, "any"]
	for entry in cfg.get("cultures", []):
		if not cultures.has(str(entry["id"])):
			cultures.append(str(entry["id"]))
	var lineage := fnv1a(id)
	var directory := ASSETS_ROOT + str(cfg.get("archetype_dir", "portraits/archetypes")) + "/"
	for culture_id in cultures:
		for rank_id in ranks:
			for band_id in bands:
				var faces := _faces(str(rank_id), sex, str(band_id), str(culture_id))
				for offset in faces:
					var face := (lineage + offset) % faces
					var key := "%s_%s_%s_%s_%d" % [rank_id, sex, band_id, culture_id, face]
					var path := directory + key + ".jpg"
					if PortraitLoader.load_texture(path) != null:
						return {"path": path, "rank": str(rank_id), "band": str(band_id), "culture": str(culture_id), "face": face}
	return {}


## Texture du portrait vivant (null si aucune image).
static func texture_for(character: Dictionary, context: Dictionary = {}) -> Texture2D:
	if context.is_empty():
		context = context_for(character)
	var path := str(resolve(character, context).get("path", ""))
	return PortraitLoader.load_texture(path) if path != "" else null


## Personnage courant lu dans la simulation ({} hors campagne).
static func character_for(character_id: String) -> Dictionary:
	var sim := _sim()
	if character_id == "" or sim == null or not sim.has_method("get_character"):
		return {}
	var result: Variant = sim.call("get_character", character_id)
	return result if result is Dictionary else {}


# --- Marques ------------------------------------------------------------------------------------


## `{dead, captive, sick, wounded, mourning, crown, mitre}` d'après l'état exposé par la façade.
## `mourning` : conjoint ou parent mort depuis moins de `marks.mourning_years` (lecture via `lookup`,
## un Callable id → dictionnaire ; ignoré s'il est invalide). `resolved` : sortie de `resolve`.
static func marks_for(character: Dictionary, resolved: Dictionary = {}, lookup: Callable = Callable()) -> Dictionary:
	var marks_cfg: Dictionary = config().get("marks", {})
	var trait_ids: Array = []
	for entry in character.get("traits", []):
		trait_ids.append(str((entry as Dictionary).get("id", "")) if entry is Dictionary else str(entry))
	var marks := {
		"dead": not bool(character.get("alive", true)),
		"captive": bool(character.get("captive", false)),
		"sick": trait_ids.any(func(t: String) -> bool: return (marks_cfg.get("sick_traits", []) as Array).has(t)),
		"wounded": trait_ids.any(func(t: String) -> bool: return (marks_cfg.get("wound_traits", []) as Array).has(t)),
		"mourning": false,
		"crown": false,
		"mitre": false,
	}
	var rank := str(resolved.get("rank", ""))
	var painted := str(resolved.get("headwear", "none"))
	var kind := str(resolved.get("kind", ""))
	# Coiffe absente de l'image : souverain ou prélat peint avant de l'être (portrait fixe d'un
	# héritier devenu roi) ou archétype de repli sans couronne.
	if rank == "sovereign" and painted != "crown":
		marks["crown"] = kind == "archetype" or str(resolved.get("data_role", "")) not in ["ruler", "consort"]
	if rank == "prelate" and painted != "mitre":
		marks["mitre"] = kind == "archetype" or str(resolved.get("data_role", "")) != "prelate"
	if not marks["dead"] and lookup.is_valid():
		var year := int(character.get("birth_year", 0)) + int(character.get("age", 0))
		var window := int(marks_cfg.get("mourning_years", 1))
		for key in ["spouse", "father", "mother"]:
			var other_id := str(character.get(key, ""))
			if other_id == "":
				continue
			var other: Dictionary = lookup.call(other_id)
			var death := int(other.get("death_year", 0))
			if not other.is_empty() and not bool(other.get("alive", true)) and death > 0 and year - death <= window:
				marks["mourning"] = true
				break
	return marks


## Libellé des marques pour une infobulle (« Défunt · Captif… »), vide si aucune.
static func marks_text(marks: Dictionary, female: bool) -> String:
	var parts: Array = []
	if marks.get("dead", false):
		parts.append("Défunte" if female else "Défunt")
	if marks.get("captive", false):
		parts.append("Captive" if female else "Captif")
	if marks.get("sick", false):
		parts.append("Maladive" if female else "Maladif")
	if marks.get("wounded", false):
		parts.append("Blessée" if female else "Blessé")
	if marks.get("mourning", false):
		parts.append("En deuil")
	return " · ".join(parts)
