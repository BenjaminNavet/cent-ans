class_name EconomyCurrencyTab
extends VBoxContainer

## RG12 : onglet « Monnaie » de la fenêtre Économie, en lecture seule. Le nom de la monnaie
## (livre tournois, `Money.SYMBOL`), le niveau de frappe actuel et l'indice des prix, le tableau
## des quatre niveaux de frappe (seigneuriage, refonte, inflation, troubles, prestige) tel que le
## cœur le calcule (`get_coinage`), et l'explication des sources de revenu. Pour changer de
## monnaie : section « Monnaie » du panneau de faction (ordre `set_coinage`).

const CURRENCY_NAME := "livre tournois"
const INTRO := "La monnaie de compte du royaume est la %s (%s). Les rois « muent » la monnaie : moins d'argent fin dans chaque pièce rapporte un seigneuriage immédiat, mais les prix montent et les rentes fixes des bourgeois et du clergé fondent. Une monnaie affaiblie ralentit aussi le commerce."
const REVENUE_HELP := "Les recettes viennent de l'impôt des provinces (taille des classes, production, bâtiments), du commerce (routes et accords), du seigneuriage de la frappe et des aumônes de la croisade ; les embargos, la dévastation et la difficulté les modèrent."
const TITLES := ["Niveau", "Seigneuriage", "Refonte", "Inflation", "Troubles bourgeois", "Prestige"]

var name_label: Label
var current_label: Label
var price_label: Label
var table_grid: GridContainer
var sources_label: RichTextLabel
## Lignes du tableau affichées (tests) : niveau → texte de la ligne.
var rows: Dictionary = {}


func _init() -> void:
	name = "Currency"
	add_theme_constant_override("separation", 8)
	name_label = UiBuild.label("", UiType.size(UiType.HEADING))
	add_child(name_label)
	var intro := _rich()
	intro.text = INTRO % [CURRENCY_NAME, Money.SYMBOL]
	add_child(intro)
	current_label = UiBuild.label("", UiType.size(UiType.BODY))
	add_child(current_label)
	price_label = UiBuild.label("", UiType.size(UiType.CAPTION))
	add_child(price_label)
	add_child(HSeparator.new())
	table_grid = GridContainer.new()
	table_grid.columns = TITLES.size()
	table_grid.add_theme_constant_override("h_separation", 12)
	table_grid.add_theme_constant_override("v_separation", 3)
	add_child(table_grid)
	add_child(HSeparator.new())
	var sources_title := UiBuild.label("Sources de revenu", UiType.size(UiType.BODY))
	sources_title.add_theme_color_override("font_color", BudgetTable.HEADER_COLOR)
	add_child(sources_title)
	sources_label = _rich()
	add_child(sources_label)


func show_data(sim: Object, faction: String) -> void:
	var coinage: Dictionary = sim.call("get_coinage", faction) if sim != null and sim.has_method("get_coinage") else {}
	var chip := EconomyIcons.chip("currency", "Monnaie : %s (%s)" % [CURRENCY_NAME.capitalize(), Money.SYMBOL], "", 22.0, UiType.size(UiType.HEADING))
	name_label.text = ""
	for child in name_label.get_children():
		child.queue_free()
	name_label.add_child(chip)
	name_label.custom_minimum_size.y = 28.0
	current_label.text = "Frappe actuelle : %s" % str(coinage.get("label", "—"))
	price_label.text = "Niveau des prix : %s (100 = prix de départ)" % str(coinage.get("price_level", "—"))
	UiBuild.clear_children(table_grid)
	rows.clear()
	for title in TITLES:
		var head := UiBuild.label(title, 13)
		head.add_theme_color_override("font_color", BudgetTable.HEADER_COLOR)
		table_grid.add_child(head)
	for option in coinage.get("options", []):
		var current := bool(option.get("current", false))
		var cells := [
			str(option.get("label", "")) + (" (actuelle)" if current else ""),
			Money.signed(int(option.get("seigniorage", 0))),
			Money.signed(-int(option.get("recoinage", 0))),
			_fraction(option.get("inflation", 0)),
			_fraction(option.get("burgher_unrest", 0)),
			str(option.get("prestige", 0)),
		]
		for text in cells:
			var cell := UiBuild.label(str(text), 14)
			if current:
				cell.add_theme_font_size_override("font_size", 15)
				cell.add_theme_color_override("font_color", Money.GAIN_COLOR)
			table_grid.add_child(cell)
		rows[str(option.get("level", ""))] = " | ".join(PackedStringArray(cells))
	var lines: Array = []
	for key in IncomeLines.LABELS:
		lines.append("· " + IncomeLines.label_of(key))
	sources_label.text = REVENUE_HELP + "\n\n" + "\n".join(PackedStringArray(lines))


static func _fraction(value: Variant) -> String:
	var number := float(value)
	if absf(number) < 0.0001:
		return "0"
	return ("%+.1f" % number).replace(".", ",").replace("-", Money.MINUS)


func _rich() -> RichTextLabel:
	var label := RichTextLabel.new()
	label.bbcode_enabled = true
	label.fit_content = true
	label.scroll_active = false
	label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	label.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	label.add_theme_color_override("default_color", Money.INK_COLOR)
	return label
