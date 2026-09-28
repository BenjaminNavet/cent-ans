class_name FeudalSection
extends VBoxContainer

## Lot FE6 (spec FE § 4.1, § 4.8, § 6) : section « Féodalité » du panneau de faction — rang et
## suzerain, obligations (tribut, ost, vassaux à protéger, cas de félonie) et objectifs (titres
## historiques et voies génériques de victoire). Lit `CampaignSim.get_feudal_sheet` et
## `get_feudal_obligations` ; « Arbre féodal… » ouvre le panneau de l'arbre.

signal tree_requested(faction_id: String)

const MET := "✔"
const OPEN := "○"

var faction_id: String = ""
var sheet: Dictionary = {}
var obligations: Dictionary = {}
var _body: VBoxContainer
var tree_button: Button


func _init() -> void:
	name = "FeudalSection"
	add_theme_constant_override("separation", 4)
	var header := HBoxContainer.new()
	add_child(header)
	var title := HudStyle.label("Féodalité", UiType.size(UiType.HEADING), HudStyle.RUBRIC)
	title.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	header.add_child(title)
	tree_button = RichButton.new()
	tree_button.name = "FeudalTreeButton"
	tree_button.text = "Arbre féodal…"
	tree_button.tooltip_text = "Suzerains, vassaux, loyautés et actions féodales"
	tree_button.pressed.connect(func() -> void: tree_requested.emit(faction_id))
	header.add_child(tree_button)
	_body = VBoxContainer.new()
	_body.name = "Body"
	_body.add_theme_constant_override("separation", 3)
	add_child(_body)


static func available(sim: Object) -> bool:
	return sim != null and sim.has_method("get_feudal_sheet")


## Remplit la section pour `id` ; cachée si la simulation n'expose pas la féodalité.
func refresh(sim: Object, id: String) -> void:
	faction_id = id
	visible = available(sim) and id != ""
	if not visible:
		return
	sheet = sim.call("get_feudal_sheet", id)
	obligations = sim.call("get_feudal_obligations", id)
	for child in _body.get_children():
		_body.remove_child(child)
		child.queue_free()
	if sheet.is_empty():
		_line("Cette faction ne tient aucun titre.", HudStyle.INK_FADED, "NoTitle")
		return
	for entry in lines(sheet, obligations):
		_line(str(entry[0]), entry[1], str(entry[2]))
	_body.add_child(_heading("Objectifs"))
	for entry in objective_lines(sheet):
		_line(str(entry[0]), entry[1], str(entry[2]))


func _heading(text: String) -> Label:
	return HudStyle.label(text, UiType.size(UiType.BODY), HudStyle.RUBRIC)


func _line(text: String, color: Color, node_name: String) -> Label:
	var label := HudStyle.label(text, UiType.size(UiType.CAPTION), color)
	label.name = node_name
	label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	_body.add_child(label)
	return label


## Lignes « position et obligations » : `[texte, couleur, nom de nœud]`.
static func lines(sheet: Dictionary, obligations: Dictionary) -> Array:
	var result: Array = []
	var titles := PackedStringArray()
	for title in sheet.get("titles", []):
		titles.append(str(title.get("name", "")))
	result.append(["Titres : %s." % ", ".join(titles), HudStyle.INK, "Titles"])
	var liege := str(sheet.get("liege_name", ""))
	if liege == "":
		result.append(["Souverain : ne doit l'hommage à personne.", HudStyle.INK, "Liege"])
	else:
		var chain := PackedStringArray()
		for link in sheet.get("liege_chain", []):
			chain.append(str(link.get("name", "")))
		var status := str(sheet.get("status_label", ""))
		result.append(["Suzerain : %s (loyauté %d, %s)%s." % [liege, int(sheet.get("loyalty", 0)), status,
			" ; au-dessus : %s" % " › ".join(chain.slice(1)) if chain.size() > 1 else ""], HudStyle.INK, "Liege"])
		result.append(["Tribut : %d %% des revenus, %s cette saison à %s." % [int(obligations.get("tribute_percent", 0)),
			Money.amount(int(obligations.get("tribute", 0))), liege], HudStyle.INK_SOFT, "Tribute"])
		var host := _names(obligations.get("host_against", []))
		result.append(["Ost dû contre : %s." % host if host != "" else "Ost : votre suzerain n'est en guerre contre personne.",
			HudStyle.POOR if host != "" else HudStyle.INK_SOFT, "Host"])
	var vassals := _names(sheet.get("direct_vassals", []))
	result.append(["Vassaux directs : %s." % vassals if vassals != "" else "Aucun vassal direct.", HudStyle.INK_SOFT, "Vassals"])
	var extra := _names(sheet.get("title_vassals", []))
	if extra != "":
		result.append(["Tiennent aussi un fief de vous : %s (double allégeance)." % extra, HudStyle.INK_SOFT, "TitleVassals"])
	var protect := PackedStringArray()
	for due in obligations.get("protect", []):
		protect.append("%s (attaqué par %s)" % [str(due.get("name", "")), _names(due.get("attackers", []))])
	if not protect.is_empty():
		result.append(["Protection due : %s." % ", ".join(protect), HudStyle.POOR, "Protect"])
	for case in obligations.get("felons", []):
		result.append(["Félonie de %s (%s) : commise possible encore %s." % [str(case.get("vassal_name", "")),
			str(case.get("reason", "")), FrText.count(int(case.get("turns_left", 0)), "tour")], HudStyle.RUBRIC, "Felon"])
	for case in obligations.get("own_felonies", []):
		result.append(["Vous êtes félon envers %s (%s) : la commise peut être prononcée contre vous (%s)." % [
			str(case.get("liege_name", "")), str(case.get("reason", "")), FrText.count(int(case.get("turns_left", 0)), "tour")],
			HudStyle.POOR, "OwnFelony"])
	return result


## Lignes d'objectifs : titres historiques, puis voies génériques (§ 4.8).
static func objective_lines(sheet: Dictionary) -> Array:
	var result: Array = []
	var index := 0
	for objective in sheet.get("objectives", []):
		var met := bool(objective.get("met", false))
		var text := "%s %s" % [MET if met else OPEN, str(objective.get("title", ""))]
		var description := str(objective.get("description", ""))
		if description != "":
			text += " — " + description
		result.append([text, HudStyle.GOOD if met else HudStyle.INK, "Objective%d" % index])
		index += 1
	if str(sheet.get("liege", "")) == "":
		result.append(["%s Indépendance tenue : %d / %d tours." % [OPEN, int(sheet.get("independent_turns", 0)),
			int(sheet.get("independence_turns", 0))], HudStyle.INK_SOFT, "Independence"])
	else:
		result.append(["%s Premier vassal du royaume : %d / %d tours." % [OPEN, int(sheet.get("first_vassal_turns", 0)),
			int(sheet.get("ascension_turns", 0))], HudStyle.INK_SOFT, "FirstVassal"])
	var crown := str(sheet.get("start_crown_name", ""))
	if crown != "":
		result.append(["%s Ceindre la couronne : %s." % [OPEN, crown], HudStyle.INK_SOFT, "Crown"])
	return result


static func _names(entries: Array) -> String:
	var names := PackedStringArray()
	for entry in entries:
		names.append(str((entry as Dictionary).get("name", "")))
	return ", ".join(names)
