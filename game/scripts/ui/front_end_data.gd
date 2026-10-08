class_name FrontEndData
extends RefCounted

## Lot MM1 — textes et réglages des écrans d'accueil (`data/ui/front_end.json`, schéma
## `data/schemas/front_end.schema.json`) : décor 3D du menu, présentation des factions, dates de
## départ, introduction, citations, conseils et illustrations des écrans de chargement.

const DATA_PATH := "ui/front_end.json"

static var _lookup := JsonLookup.new(DATA_PATH)
## Indices des citations pas encore montrées dans la session (voir `random_quote`).
static var _quote_bag: Array = []
static var _quote_bag_key: String = ""


static func data() -> Dictionary:
	return _lookup.data()


static func backdrop() -> Dictionary:
	return data().get("backdrop", {})


## Présentations des factions jouables, dans l'ordre du fichier.
static func factions() -> Array:
	return data().get("factions", [])


static func faction(faction_id: String) -> Dictionary:
	for entry in factions():
		if str((entry as Dictionary).get("id", "")) == faction_id:
			return entry
	return {}


## JR6 : onglet « Défis singuliers » (factions à mécanique propre, ex. les Croisés) : titre, texte
## et factions, réduites aux factions présentées ; dictionnaire vide si l'onglet n'a rien à montrer.
static func special_starts() -> Dictionary:
	var entry: Dictionary = data().get("special_starts", {})
	var ids := PackedStringArray()
	for id in entry.get("factions", []):
		if not faction(str(id)).is_empty():
			ids.append(str(id))
	if ids.is_empty():
		return {}
	return {"title": str(entry.get("title", "Défis singuliers")), "text": str(entry.get("text", "")), "factions": ids}


## FE6 : départs recommandés (cartes du menu), dans l'ordre des données, réduits aux factions
## présentées (jouables) ; à défaut, les trois premières présentations.
static func recommended() -> PackedStringArray:
	var presented := {}
	for entry in factions():
		presented[str((entry as Dictionary).get("id", ""))] = true
	var result := PackedStringArray()
	for id in data().get("recommended_factions", []):
		if presented.has(str(id)):
			result.append(str(id))
	if result.is_empty():
		for entry in factions().slice(0, 3):
			result.append(str((entry as Dictionary).get("id", "")))
	return result


static func difficulty_label(level: int) -> String:
	var labels: Array = data().get("difficulty_labels", [])
	return str(labels[clampi(level, 0, labels.size() - 1)]) if not labels.is_empty() else ""


static func start_dates() -> Array:
	return data().get("start_dates", [])


static func intro() -> Dictionary:
	return data().get("intro", {})


static func loading() -> Dictionary:
	return data().get("loading", {})


## Culture (`cul_*`) d'une faction, lue dans `data/factions/<id>.json` ("" si inconnue).
static func faction_culture(faction_id: String) -> String:
	if faction_id == "":
		return ""
	var rel_path := "factions/%s.json" % faction_id
	if not DataFile.exists(rel_path):
		return ""
	var parsed: Variant = DataFile.read_json(rel_path)
	return str((parsed as Dictionary).get("culture", "")) if parsed is Dictionary else ""


## Citations montrables à une faction (A6-L11, U3) : celles sans `cultures` (générales, guerre de
## Cent Ans) et celles dont `cultures` contient la culture de la faction. Sans faction, ou sans
## culture connue : uniquement les générales.
static func quotes_for(faction_id: String) -> Array:
	var culture := faction_culture(faction_id)
	var result: Array = []
	for quote: Variant in loading().get("quotes", []):
		var cultures: Array = (quote as Dictionary).get("cultures", [])
		if cultures.is_empty() or (culture != "" and cultures.has(culture)):
			result.append(quote)
	return result


## Tirage d'une citation {text, author, source, date} ({} si aucune), filtrée par la culture de
## `faction_id` (voir `quotes_for` ; repli : citations générales). Sans `rng`, tirage « sac
## mélangé » : aucune citation ne revient avant que toutes aient été montrées dans la session.
static func random_quote(rng: RandomNumberGenerator = null, faction_id: String = "") -> Dictionary:
	var quotes: Array = quotes_for(faction_id)
	if quotes.is_empty():
		return {}
	if rng != null:
		return quotes[_pick(quotes.size(), rng)]
	if _quote_bag_key != faction_id or _quote_bag.is_empty():
		_quote_bag_key = faction_id
		_quote_bag.clear()
		for index in quotes.size():
			_quote_bag.append(index)
		_quote_bag.shuffle()
	return quotes[mini(int(_quote_bag.pop_back()), quotes.size() - 1)]


static func random_tip(rng: RandomNumberGenerator = null) -> String:
	var tips: Array = loading().get("tips", [])
	return str(tips[_pick(tips.size(), rng)]) if not tips.is_empty() else ""


## Illustration d'écran de chargement : de préférence une propre à la faction (une fois sur
## deux), sinon une du fonds commun.
static func random_illustration(faction_id: String = "", rng: RandomNumberGenerator = null) -> String:
	var info := loading()
	var own: Array = (info.get("faction_illustrations", {}) as Dictionary).get(faction_id, [])
	var common: Array = info.get("illustrations", [])
	if not own.is_empty() and (common.is_empty() or _pick(2, rng) == 0):
		return str(own[_pick(own.size(), rng)])
	return str(common[_pick(common.size(), rng)]) if not common.is_empty() else ""


static func _pick(count: int, rng: RandomNumberGenerator) -> int:
	if count <= 1:
		return 0
	return rng.randi_range(0, count - 1) if rng != null else randi() % count
