class_name BudgetTable
extends VBoxContainer

## Tableau du budget du panneau de faction (audit A3 E1/E2, lot U3), à la manière de 3K :
## rubriques en lignes, colonnes « Prévu » (prochaine fin de tour), « Saison passée » (ce qui a
## été réellement porté au trésor) et « Écart ». Recettes en vert, dépenses en rouge, solde en
## gras ; « Autres mouvements » regroupe ce que le budget n'explique pas (rançons, tributs,
## agents, chronique, butin). Tout vient de `get_faction_economy` (`budget_lines`,
## `net_income`, `net_income_last_turn`, `net_change`) : aucun calcul de règle ici.

## Libellés des rubriques (clés de `economy_balance::BudgetLineKind::key`).
const RUBRICS := {
	"receipts": "Impôts et commerce",
	"armies": "Entretien des armées",
	"buildings": "Entretien des bâtiments",
	"table": "Table des provinces",
	"administration": "Cour et administration",
	"other": "Autres mouvements",
}
const RUBRIC_HINTS := {
	"receipts": "Impôts des colonies, commerce et seigneuriage, selon le taux d'imposition.",
	"armies": "Solde et vivres des armées et des garnisons (plus chers si la monnaie est affaiblie).",
	"buildings": "Entretien des bâtiments des colonies ; réduit par la dévastation.",
	"table": "Régimes alimentaires des provinces (La Table).",
	"administration": "Part du revenu prise par la cour et l'administration (plus de provinces, plus de frais), refonte des monnaies comprise.",
	"other": "Ce que le budget n'explique pas : rançons, tributs, agents, choix de la chronique, butin des chevauchées. Connu seulement après coup.",
}
const PREMIUM_NAME := "Surprime des mercenaires"
const PREMIUM_HINT := "Ce que les compagnies coûtent au-delà de la solde ordinaire ; prélevé sur le trésor en fin de tour, en plus du solde. Impayée, une compagnie déserte ou pille."
const HEADER_COLOR := Color(0.40, 0.28, 0.14)
const FONT_SIZE := 15

var _grid: GridContainer
## Valeurs affichées (clé → `{projected, last, delta}` en texte), lues par les tests.
var cells: Dictionary = {}


func _init() -> void:
	add_theme_constant_override("separation", 4)
	_grid = GridContainer.new()
	_grid.columns = 4
	_grid.add_theme_constant_override("h_separation", 10)
	_grid.add_theme_constant_override("v_separation", 3)
	add_child(_grid)


## `economy` : `get_faction_economy` (lignes signées `budget_lines`).
func show_budget(economy: Dictionary) -> void:
	for child in _grid.get_children():
		_grid.remove_child(child)
		child.queue_free()
	cells.clear()
	var has_past := economy.has("net_change")
	_header_row(["Rubrique", "Prévu", "Saison passée", "Écart"])
	var lines: Array = economy.get("budget_lines", [])
	_section_row("Recettes")
	for line in lines:
		if not bool(line.get("charge", false)) and str(line.get("key", "")) == "receipts":
			_line_row(line, has_past)
	_section_row("Dépenses")
	for line in lines:
		if bool(line.get("charge", false)):
			_line_row(line, has_past)
	for line in lines:
		if str(line.get("key", "")) == "other" and has_past:
			_line_row(line, has_past)
	var premium := int(economy.get("mercenary_premium", 0))
	var premium_last := int(economy.get("mercenary_premium_last_turn", 0))
	if premium != 0 or premium_last != 0:
		_premium_row(premium, premium_last)
	_rule_row()
	var net := int(economy.get("net_income", 0))
	var last_net := int(economy.get("net_income_last_turn", 0))
	var net_line := {"key": "net", "projected": net}
	if has_past:
		net_line["last"] = last_net
		net_line["delta"] = int(economy.get("net_change", 0))
	_line_row(net_line, has_past, true)


func _header_row(titles: Array) -> void:
	for index in titles.size():
		var label := _cell(str(titles[index]), index > 0, 13)
		label.add_theme_color_override("font_color", HEADER_COLOR)
		_grid.add_child(label)


func _section_row(title: String) -> void:
	var label := _cell(title, false, 13)
	label.add_theme_color_override("font_color", HEADER_COLOR)
	label.uppercase = true
	_grid.add_child(label)
	for _i in 3:
		_grid.add_child(Control.new())


## NT6b : surprime des compagnies de mercenaires (valeur du core), prélevée sur le trésor en
## plus du solde ci-dessus ; ligne à part, hors du total.
func _premium_row(premium: int, last: int) -> void:
	var name_label := _cell("   " + PREMIUM_NAME, false, FONT_SIZE)
	TooltipHost.attach_plain(name_label, "budget_line_hint", {"title": PREMIUM_NAME, "body": PREMIUM_HINT})
	_grid.add_child(name_label)
	var projected_label := _cell(Money.signed(-premium), true, FONT_SIZE)
	projected_label.add_theme_color_override("font_color", Money.LOSS_COLOR if premium > 0 else Money.INK_COLOR)
	TooltipHost.attach_plain(projected_label, "budget_projected_hint")
	_grid.add_child(projected_label)
	var last_label := _cell(Money.signed(-last), true, FONT_SIZE - 1)
	TooltipHost.attach_plain(last_label, "budget_last_hint", {"body": "Surprime réellement prélevée à la dernière fin de tour."})
	_grid.add_child(last_label)
	_grid.add_child(Control.new())
	cells["mercenary_premium"] = {"projected": projected_label.text, "last": last_label.text, "delta": "—"}


func _rule_row() -> void:
	for _i in 4:
		var rule := HSeparator.new()
		rule.add_theme_constant_override("separation", 2)
		_grid.add_child(rule)


func _line_row(line: Dictionary, has_past: bool, is_total: bool = false) -> void:
	var key := str(line.get("key", ""))
	var name := "Solde de la saison" if is_total else str(RUBRICS.get(key, key))
	var projected := int(line.get("projected", 0))
	var name_label := _cell(name, false, FONT_SIZE + (1 if is_total else 0))
	if not is_total:
		name_label.text = "   " + name
	TooltipHost.attach_plain(name_label, "budget_line_hint", {"title": name, "body": "Recettes moins toutes les charges : le « Solde » de la barre du haut, ajouté au trésor en fin de tour." if is_total else str(RUBRIC_HINTS.get(key, ""))})
	_grid.add_child(name_label)
	var projected_text := "—" if key == "other" else Money.signed(projected)
	var projected_label := _cell(projected_text, true, FONT_SIZE + (1 if is_total else 0))
	if key != "other":
		projected_label.add_theme_color_override("font_color", Money.color_of(projected) if is_total else (Money.LOSS_COLOR if projected < 0 else Money.GAIN_COLOR if projected > 0 else Money.INK_COLOR))
	TooltipHost.attach_plain(projected_label, "budget_projected_hint")
	_grid.add_child(projected_label)
	var last_text := "—"
	var delta_text := "—"
	if has_past and line.has("last"):
		last_text = Money.signed(int(line["last"]))
	if has_past and line.has("delta") and key != "other":
		delta_text = Money.delta(int(line["delta"]))
	var last_label := _cell(last_text, true, FONT_SIZE - 1)
	TooltipHost.attach_plain(last_label, "budget_last_hint", {"body": "Ce qui a réellement été porté au trésor à la dernière fin de tour." if has_past else "Premier tour : pas encore de saison passée."})
	_grid.add_child(last_label)
	var delta_label := _cell(delta_text, true, FONT_SIZE - 2)
	if has_past and line.has("delta") and key != "other":
		delta_label.add_theme_color_override("font_color", Money.color_of(int(line["delta"])))
		TooltipHost.attach_plain(delta_label, "budget_delta_hint")
	_grid.add_child(delta_label)
	if is_total:
		for label: Label in [name_label, projected_label]:
			label.add_theme_color_override("font_outline_color", Color(0.22, 0.14, 0.07, 0.35))
			label.add_theme_constant_override("outline_size", 1)
	cells[key] = {"projected": projected_text, "last": last_text, "delta": delta_text}


func _cell(text: String, right: bool, font_size: int) -> Label:
	var label := RichLabel.new()
	label.text = text
	label.add_theme_font_size_override("font_size", font_size)
	label.mouse_filter = Control.MOUSE_FILTER_PASS
	if right:
		label.horizontal_alignment = HORIZONTAL_ALIGNMENT_RIGHT
	else:
		label.size_flags_horizontal = Control.SIZE_EXPAND_FILL
		# VN : la colonne des rubriques cède la place aux montants (panneau étroit) : le libellé
		# passe à la ligne au lieu de pousser « Écart » hors du cadre.
		label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	return label
