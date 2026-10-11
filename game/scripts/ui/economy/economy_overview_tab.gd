class_name EconomyOverviewTab
extends VBoxContainer

## RG12 : onglet « Vue d'ensemble » de la fenêtre Économie. Recettes et dépenses ventilées
## (`BudgetTable`, d'après `get_faction_economy` ; les recettes par source viennent de
## `get_income_breakdown`), solde, trésor et trésor du prochain tour, prévisions RG8 de
## prestige / hommes / troubles avec leurs principales causes, et l'historique du solde que garde
## le cœur (`budget_history`, 12 saisons au plus). Aucune règle ici.

const HISTORY_SHOWN := 8
const GAUGES := [
	["prestige", "Prestige", false],
	["men", "Hommes sous les armes", false],
	["unrest", "Troubles", true],
]

var budget_table: BudgetTable
var treasury_label: RichTextLabel
var drivers_label: RichTextLabel
var gauges_box: VBoxContainer
var history_box: VBoxContainer
## Lignes affichées (tests) : `income_total`, `net`, `treasury`, `gauges` (clés), `history` (nombre).
var shown: Dictionary = {}


func _init() -> void:
	name = "Overview"
	add_theme_constant_override("separation", 8)
	treasury_label = _rich()
	add_child(treasury_label)
	budget_table = BudgetTable.new()
	budget_table.name = "BudgetTable"
	add_child(budget_table)
	add_child(HSeparator.new())
	add_child(_title("Prévisions du prochain tour"))
	gauges_box = UiBuild.vbox(4)
	add_child(gauges_box)
	add_child(HSeparator.new())
	add_child(_title("Principales causes du solde"))
	drivers_label = _rich()
	add_child(drivers_label)
	add_child(HSeparator.new())
	add_child(_title("Historique du solde"))
	history_box = UiBuild.vbox(2)
	add_child(history_box)


## `sim` : `CampaignSim` ; `faction` : id du joueur.
func show_data(sim: Object, faction: String) -> void:
	shown.clear()
	var economy: Dictionary = sim.call("get_faction_economy", faction) if sim != null and sim.has_method("get_faction_economy") else {}
	var breakdown: Dictionary = sim.call("get_income_breakdown", faction) if sim != null and sim.has_method("get_income_breakdown") else {}
	var forecast: Dictionary = sim.call("get_faction_forecast", faction) if sim != null and sim.has_method("get_faction_forecast") else {}
	var provinces: Array = sim.call("get_income_top_provinces", faction, 3) if sim != null and sim.has_method("get_income_top_provinces") else []
	var treasury := int(economy.get("treasury", 0))
	var net := int(economy.get("net_income", 0))
	treasury_label.text = "[b]Trésor :[/b] %s — [b]solde de la saison :[/b] [color=#%s]%s[/color]\n%s" % [
		Money.amount(treasury), Money.color_of(net).to_html(false), Money.signed(net), ForecastText.treasury_line(treasury, net)]
	shown["treasury"] = treasury
	shown["net"] = net
	shown["income_total"] = int(breakdown.get("total", 0))
	# Le tableau montre les recettes par source : celles de `get_income_breakdown`.
	var table_input := economy.duplicate()
	if not breakdown.is_empty():
		table_input["income_lines"] = breakdown.get("lines", [])
	budget_table.show_budget(table_input)
	_show_gauges(forecast)
	var drivers := ForecastText.income_drivers(breakdown.get("lines", []), provinces)
	drivers_label.text = drivers if drivers != "" else "[i]Aucune recette.[/i]"
	_show_history(economy.get("budget_history", []))


func _show_gauges(forecast: Dictionary) -> void:
	UiBuild.clear_children(gauges_box)
	var keys: Array = []
	if forecast.is_empty():
		var empty := _rich()
		empty.text = "[i]Pas de prévision disponible.[/i]"
		gauges_box.add_child(empty)
	for gauge in GAUGES:
		var data: Dictionary = forecast.get(gauge[0], {})
		if data.is_empty():
			continue
		keys.append(gauge[0])
		var unit := "" if gauge[0] != "unrest" else " %"
		var head := UiBuild.label("%s : %s → %s %s" % [gauge[1], Money.digits(int(data.get("current", 0))), Money.digits(int(data.get("next", 0))), ForecastText.arrow(data)], UiType.size(UiType.BODY))
		head.add_theme_color_override("font_color", ForecastText.color_of(float(data.get("delta", 0)), bool(gauge[2])))
		gauges_box.add_child(head)
		var causes := _rich()
		causes.text = ForecastText.tooltip(data, "", bool(gauge[2]), gauge[0] == "unrest")
		gauges_box.add_child(causes)
	shown["gauges"] = keys


func _show_history(history: Array) -> void:
	UiBuild.clear_children(history_box)
	if history.is_empty():
		history_box.add_child(UiBuild.label("Pas encore de saison écoulée.", UiType.size(UiType.CAPTION)))
		shown["history"] = 0
		return
	var start := maxi(0, history.size() - HISTORY_SHOWN)
	var peak := 1
	for index in range(start, history.size()):
		peak = maxi(peak, absi(int(history[index].get("net", 0))))
	var count := 0
	for index in range(history.size() - 1, start - 1, -1):
		var record: Dictionary = history[index]
		var net := int(record.get("net", 0))
		var row := UiBuild.hbox(8)
		row.add_child(UiBuild.label("Tour %d" % int(record.get("turn", 0)), UiType.size(UiType.CAPTION), null, false, 64.0))
		var bar := ProgressBar.new()
		bar.min_value = 0
		bar.max_value = peak
		bar.value = absi(net)
		bar.show_percentage = false
		bar.custom_minimum_size = Vector2(120, 10)
		bar.size_flags_vertical = Control.SIZE_SHRINK_CENTER
		row.add_child(bar)
		var net_label := UiBuild.label(Money.signed(net), UiType.size(UiType.CAPTION), Money.color_of(net), false, 90.0)
		net_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_RIGHT
		row.add_child(net_label)
		row.add_child(UiBuild.label("trésor %s" % Money.amount(int(record.get("treasury", 0))), UiType.size(UiType.CAPTION)))
		history_box.add_child(row)
		count += 1
	shown["history"] = count


func _title(text: String) -> Label:
	var label := UiBuild.label(text, UiType.size(UiType.BODY))
	label.add_theme_color_override("font_color", BudgetTable.HEADER_COLOR)
	return label


func _rich() -> RichTextLabel:
	var label := RichTextLabel.new()
	label.bbcode_enabled = true
	label.fit_content = true
	label.scroll_active = false
	label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	label.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	label.add_theme_color_override("default_color", Money.INK_COLOR)
	return label
