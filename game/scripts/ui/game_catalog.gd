class_name GameCatalog
extends RefCounted

## Définitions descriptives de `data/` pour l'affichage (infobulles F2) : types d'unités,
## bâtiments, ressources, technologies, traits, compétences. Lecture seule des JSON validés
## par `data/schemas/` ; aucune règle de jeu ici (coûts effectifs, disponibilités et refus
## viennent toujours de `CampaignSim`). `GameDataStore` n'expose pas encore ces définitions
## (seulement provinces, factions, personnages, traits, compétences) : ce catalogue comble
## l'écart côté affichage en attendant des accesseurs Rust (voir `docs/status.md`).
##
## N'utilise aucun autoload par son nom (le smoke `--script` est compilé avant leur
## enregistrement) : `data/` est résolu via `/root/MapPaths` à l'exécution.


static var _cache: Dictionary = {}  # dossier → {id → définition}
static var _data_dir: String = ""


## Définitions d'un dossier de `data/` (rechargées si `MapPaths.data_dir` a changé).
static func definitions(directory: String) -> Dictionary:
	var current := DataFile.data_dir()
	if current != _data_dir:
		_cache.clear()
		_data_dir = current
	if _cache.has(directory):
		return _cache[directory]
	var result: Dictionary = {}
	var path := current.path_join(directory)
	var dir := DirAccess.open(path)
	if dir != null:
		for file_name in dir.get_files():
			if not file_name.ends_with(".json"):
				continue
			var parsed: Variant = DataFile.parse_file(path.path_join(file_name))
			if parsed is Dictionary and parsed.has("id"):
				result[str(parsed["id"])] = parsed
	_cache[directory] = result
	return result


static func unit_type(id: String) -> Dictionary:
	return definitions("unit_types").get(id, {})


static func building(id: String) -> Dictionary:
	return definitions("buildings").get(id, {})


static func resource(id: String) -> Dictionary:
	return definitions("resources").get(id, {})


static func technology(id: String) -> Dictionary:
	return definitions("technologies").get(id, {})


static func trait_definition(id: String) -> Dictionary:
	return definitions("traits").get(id, {})


static func skill(id: String) -> Dictionary:
	return definitions("skills").get(id, {})


## Définition de n'importe quel id selon son préfixe (vide si inconnu).
static func any(id: String) -> Dictionary:
	if id.begins_with("unit_"):
		return unit_type(id)
	if id.begins_with("bld_"):
		return building(id)
	if id.begins_with("res_"):
		return resource(id)
	if id.begins_with("tech_"):
		return technology(id)
	if id.begins_with("trait_"):
		return trait_definition(id)
	if id.begins_with("skill_"):
		return skill(id)
	return {}


## Nom affiché (`name.display`), sinon l'id rendu lisible.
static func display_name(id: String) -> String:
	var definition := any(id)
	var name: Variant = definition.get("name", null)
	if name is Dictionary and str(name.get("display", "")) != "":
		return str(name["display"])
	for prefix in ["unit_", "bld_", "res_", "tech_", "trait_", "skill_"]:
		if id.begins_with(prefix):
			return id.trim_prefix(prefix).replace("_", " ").capitalize()
	return id


## Moyenne d'une statistique sur tous les types d'unités (repère des forces/faiblesses).
static func unit_stat_average(stat: String) -> float:
	var total := 0.0
	var count := 0
	for definition in definitions("unit_types").values():
		var stats: Dictionary = definition.get("stats", {})
		if stats.has(stat):
			total += float(stats[stat])
			count += 1
	return total / count if count > 0 else 0.0


static func clear_cache() -> void:
	_cache.clear()
	_data_dir = ""
