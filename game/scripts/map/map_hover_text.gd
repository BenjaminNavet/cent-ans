class_name MapHoverText
extends RefCounted

## WH hover (ADR 0271) : texte BBCode des bulles riches de la carte de campagne (armée,
## colonie, armée perdue de vue). Fonctions pures sur les dictionnaires du cœur
## (`get_army`, `settlement_detail`) : aucune règle de jeu, aucune lecture hors de la vue —
## l'appelant dit ce qui est visible (`opts.visible`, `opts.is_player`). L'embuscade d'un ennemi
## n'est jamais montrée.

const CATEGORY_NAMES := {"infantry": "fantassins", "ranged": "archers", "cavalry": "cavaliers", "siege": "engins"}
const CATEGORY_ORDER := ["infantry", "ranged", "cavalry", "siege"]
const STANCE_NAMES := {"raid": "Chevauchée", "siege": "Siège", "ambush": "Embuscade", "forced_march": "Marche forcée", "entrenched": "Retranchée"}
const WALL_NAMES := {"bld_stone_walls": "murailles de pierre", "bld_walls": "murailles", "bld_palisade": "palissade"}
const LEVEL_TITLES := ["Village", "Bourg", "Ville", "Cité"]


## Hommes par catégorie d'unité, dans l'ordre d'affichage : [[catégorie, hommes], …].
## `category_of` : Callable(unit_type) -> catégorie ("infantry" par défaut, injectable en test).
static func composition(army: Dictionary, category_of: Callable = Callable()) -> Array:
	var totals: Dictionary = {}
	for unit: Dictionary in army.get("units", []):
		var unit_type := str(unit.get("unit_type", ""))
		var category := str(category_of.call(unit_type)) if category_of.is_valid() else str(ModelLibrary.unit_category(unit_type))
		if not CATEGORY_NAMES.has(category):
			category = "infantry"
		totals[category] = int(totals.get(category, 0)) + int(unit.get("strength", 0))
	var rows: Array = []
	for category: String in CATEGORY_ORDER:
		if totals.has(category):
			rows.append([category, totals[category]])
	return rows


## Posture à montrer ("" = rien) : jamais « normale », jamais l'embuscade d'un ennemi.
static func visible_stance(army: Dictionary, is_player: bool) -> String:
	var stance := str(army.get("stance", "normal"))
	if stance == "" or stance == "normal" or (stance == "ambush" and not is_player):
		return ""
	return STANCE_NAMES.get(stance, "")


## Bulle d'une armée. `opts` : is_player, faction_name, liege_name, destination_name,
## category_of (Callable, tests), seen_ago (≥ 0 : armée perdue de vue, dernière situation connue).
static func army_text(army: Dictionary, opts: Dictionary = {}) -> String:
	var is_player := bool(opts.get("is_player", false))
	var faction_name := str(opts.get("faction_name", army.get("faction", "")))
	var general := str(army.get("general_name", ""))
	var seen_ago := int(opts.get("seen_ago", -1))
	var title := "Ost de %s" % general if general != "" else "Ost de %s" % faction_name
	if seen_ago >= 0:
		title = "Dernière position connue — " + title
	var lines: PackedStringArray = ["[b]%s[/b]" % title]
	var faction_line := faction_name
	var liege := str(opts.get("liege_name", ""))
	if liege != "":
		faction_line += " (vassal de %s)" % liege
	if general != "" and faction_line != "":
		lines.append(faction_line)
	var men := int(opts.get("men", -1))
	if men < 0:
		men = 0
		for unit: Dictionary in army.get("units", []):
			men += int(unit.get("strength", 0))
	lines.append("%s hommes" % ArmyPlate.format_men(men))
	if seen_ago >= 0:
		lines.append("[i]%s[/i]" % ("Vue ce tour-ci" if seen_ago == 0 else "Vue il y a %s" % FrText.count(seen_ago, "saison")))
		return "\n".join(lines)
	var rows := composition(army, opts.get("category_of", Callable()))
	if rows.size() > 0:
		var parts: PackedStringArray = []
		for row: Array in rows:
			parts.append("%s %s" % [ArmyPlate.format_men(int(row[1])), CATEGORY_NAMES[row[0]]])
		lines.append(" · ".join(parts))
	var stance := visible_stance(army, is_player)
	if stance != "":
		lines.append("Posture : %s" % stance)
	var state := _state_line(army, str(opts.get("destination_name", "")))
	if state != "":
		lines.append(state)
	if is_player:
		lines.append("Vivres : %d" % int(army.get("supply", 0)))
	return "\n".join(lines)


static func _state_line(army: Dictionary, destination_name: String) -> String:
	if bool(army.get("embarked", false)) or bool(army.get("at_sea", false)):
		return "À bord"
	if str(army.get("stance", "")) == "siege":
		return "Met le siège"
	var moving := not (army.get("path", []) as Array).is_empty() or not (army.get("planned_path", PackedVector2Array()) as PackedVector2Array).is_empty()
	if moving:
		return "En marche vers %s" % destination_name if destination_name != "" else "En marche"
	return "Au repos"


## Enceinte d'une colonie d'après ses bâtiments et sa fortification.
static func wall_text(detail: Dictionary) -> String:
	var buildings: Array = detail.get("buildings", [])
	for id: String in ["bld_stone_walls", "bld_walls", "bld_palisade"]:
		if buildings.has(id):
			return WALL_NAMES[id]
	var fortification := int(detail.get("fortification_level", 0))
	return "enceinte (fortification %d)" % fortification if fortification >= 2 else "sans enceinte"


## Bulle d'une colonie. `detail` : `settlement_detail`. `opts` : visible (la province est en vue
## ou nous appartient), is_player, level (niveau visuel, -1 inconnu), owner_name, controller_name,
## attacker_name. Hors de vue : nom et propriétaire seulement.
static func settlement_text(detail: Dictionary, opts: Dictionary = {}) -> String:
	var lines: PackedStringArray = ["[b]%s[/b]" % str(detail.get("name", ""))]
	var owner_name := str(opts.get("owner_name", detail.get("owner", "")))
	var controller_name := str(opts.get("controller_name", detail.get("controller", "")))
	lines.append("Propriétaire : %s" % owner_name)
	if controller_name != "" and controller_name != owner_name:
		lines.append("Occupée par %s" % controller_name)
	if not bool(opts.get("visible", false)):
		return "\n".join(lines)
	var level := int(opts.get("level", -1))
	if level >= 0 and level < LEVEL_TITLES.size():
		lines.append("%s · %s" % [LEVEL_TITLES[level], wall_text(detail)])
	else:
		var walls := wall_text(detail)
		lines.append(walls.substr(0, 1).to_upper() + walls.substr(1))
	var siege: Dictionary = detail.get("siege", {})
	if not siege.is_empty():
		var attacker := str(opts.get("attacker_name", siege.get("attacker", "")))
		lines.append("[color=#8f1510]Assiégée par %s depuis %s[/color]" % [attacker, FrText.count(int(siege.get("turns_elapsed", 0)), "tour")])
		lines.append("Vivres de la place : %d · brèche : %d" % [int(siege.get("supplies", 0)), int(siege.get("breach", 0))])
	var garrison: Array = detail.get("garrison", [])
	var men := int(detail.get("garrison_strength", 0))
	if bool(opts.get("is_player", false)) or not garrison.is_empty():
		lines.append("Garnison : %s hommes" % ArmyPlate.format_men(men) if men > 0 else "Garnison : aucune")
	return "\n".join(lines)
