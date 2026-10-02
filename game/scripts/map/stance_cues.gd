class_name StanceCues
extends RefCounted

## Lot EN (ADR 0153) : signes de relation de la carte de campagne hors mode Diplomatie. Le rouge
## est réservé aux ennemis du joueur (frontières, plaques et anneaux d'armée, noms de ville) ; les
## factions en paix gardent leur couleur héraldique, assourdie. Rendu seulement : la position de
## chaque faction vient de `CampaignSim.get_faction_stances_for` (règle dans `sim_campaign::stance`).
## Réglages : `data/map/stance_cues.json` (schéma `stance_cues.schema.json`).

const MAP_PATHS_SCRIPT := preload("res://scripts/map/map_paths.gd")
const TUNING_PATH := "map/stance_cues.json"
const SELF := "self"
const ENEMY := "enemy"
const FRIEND := "friend"
const OTHER := "other"

static var _tuning: Dictionary = {}
static var _loaded: bool = false


## Réglages (dictionnaire vide si le fichier manque : aucun signe, couleurs héraldiques).
static func tuning() -> Dictionary:
	if not _loaded:
		_loaded = true
		var path := MAP_PATHS_SCRIPT.default_data_dir().path_join(TUNING_PATH)
		if FileAccess.file_exists(path):
			var parsed: Variant = JSON.parse_string(FileAccess.get_file_as_string(path))
			if parsed is Dictionary:
				_tuning = parsed
		else:
			push_warning("StanceCues: %s missing" % path)
	return _tuning


static func clear_cache() -> void:
	_tuning = {}
	_loaded = false


## Positions du joueur envers chaque faction ({id: clé}) ; vide si la simulation ne les calcule pas.
static func stances(sim: Object, player: String) -> Dictionary:
	return DiplomaticStances.faction_stances(sim, player)


## Catégorie de signe d'une position diplomatique (`war` → ennemi…) ; inconnue : « other ».
static func category(stance_key: String) -> String:
	return str(tuning().get("categories", {}).get(stance_key, OTHER))


## Catégorie de `faction` vue par `player`, d'après `faction_stances` (résultat de `stances`).
static func category_of(faction: String, player: String, faction_stances: Dictionary) -> String:
	if faction != "" and faction == player:
		return SELF
	return category(str(faction_stances.get(faction, "")))


## Couleur du trait de frontière : fixe pour nous, ennemis et amis ; héraldique assourdie sinon.
static func border_color(heraldic: Color, cue: String) -> Color:
	var border: Dictionary = tuning().get("border", {})
	if border.is_empty():
		return heraldic
	if cue != OTHER and border.has(cue):
		return Color.html(str(border[cue]))
	var muted: Dictionary = border.get(OTHER, {})
	var saturation := minf(heraldic.s * float(muted.get("saturation_scale", 1.0)), float(muted.get("saturation_max", 1.0)))
	var value := clampf(heraldic.v, float(muted.get("value_min", 0.0)), float(muted.get("value_max", 1.0)))
	return Color.from_hsv(heraldic.h, saturation, value)


## Poids de pleine intensité des frontières ennemies face au style « au repos » (TB2).
static func enemy_border_focus() -> float:
	return float(tuning().get("border", {}).get("enemy_focus", 0.0))


## Bordure de la plaque d'effectif d'une armée : {color: Color, width: int}.
static func plate_border(cue: String, fallback: Color, fallback_width: int) -> Dictionary:
	var plates: Dictionary = tuning().get("army", {}).get("plate", {})
	if not plates.has(cue):
		return {"color": fallback, "width": fallback_width}
	var plate: Dictionary = plates[cue]
	return {"color": Color.html(str(plate.get("color", "#000000"))), "width": int(plate.get("width_px", fallback_width))}


## Marque non colorée des armées ennemies ("" pour les autres catégories).
static func plate_glyph(cue: String) -> String:
	return str(tuning().get("army", {}).get("enemy_glyph", "")) if cue == ENEMY else ""


## Anneau au sol d'une armée non sélectionnée : {color: Color (alpha compris), emission: float} ;
## vide : anneau héraldique ordinaire.
static func army_ring(cue: String) -> Dictionary:
	var rings: Dictionary = tuning().get("army", {}).get("ring", {})
	if not rings.has(cue):
		return {}
	var ring: Dictionary = rings[cue]
	var color := Color.html(str(ring.get("color", "#ffffff")))
	color.a = float(ring.get("alpha", 0.5))
	return {"color": color, "emission": float(ring.get("emission", 0.08))}


## Encre du nom d'une ville selon la catégorie de son détenteur ; `ordinary` sinon.
static func town_label_color(cue: String, ordinary: Color) -> Color:
	var labels: Dictionary = tuning().get("town", {}).get("label", {})
	return Color.html(str(labels[cue])) if labels.has(cue) else ordinary
