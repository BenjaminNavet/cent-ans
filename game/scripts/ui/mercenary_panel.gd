class_name MercenaryPanel
extends PanelContainer

## TW2-T3 : panneau « Mercenaires » ouvert depuis le bandeau d'une armée du joueur. Affiche les
## compagnies de la région où se trouve l'armée (`CampaignSim.get_mercenaries`) avec les lignes du
## recrutement (`PanelWidgets.fill_recruitable` : prix, solde, réserve « N disponibles, +1 dans K
## saisons », motif du refus) ; un clic émet `hire_requested`. Aucune règle ici : prix, solde
## majorée, limites et refus viennent du cœur.

signal hire_requested(army_id: String, unit_type: String)

## Armée dont le panneau montre le marché.
var army_id: String = ""

var _title: Label
var _status: Label
var _list: HFlowContainer


func _init() -> void:
	name = "MercenaryPanel"
	add_theme_stylebox_override("panel", HudStyle.panel_box(10))
	custom_minimum_size = Vector2(420, 0)
	var box := UiBuild.vbox(6)
	add_child(box)
	var header := HBoxContainer.new()
	box.add_child(header)
	_title = HudStyle.label("Mercenaires", UiType.size(UiType.HEADING), HudStyle.RUBRIC)
	_title.name = "Title"
	_title.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	header.add_child(_title)
	var close := UiBuild.button("×")
	close.name = "Close"
	TooltipHost.attach_plain(close, "close_escape")
	close.pressed.connect(hide)
	header.add_child(close)
	_status = HudStyle.label("", UiType.size(UiType.CAPTION), HudStyle.INK_SOFT)
	_status.name = "Status"
	_status.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	box.add_child(_status)
	_list = HFlowContainer.new()
	_list.add_theme_constant_override("h_separation", 4)
	_list.add_theme_constant_override("v_separation", 4)
	_list.name = "List"
	box.add_child(_list)
	var note := HudStyle.label(
		"Engagées sur-le-champ, les compagnies coûtent cher et touchent une solde majorée ; impayées, elles désertent ou pillent la province.",
		UiType.size(UiType.CAPTION), HudStyle.INK_SOFT)
	note.name = "Note"
	note.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	box.add_child(note)


## Remplit le panneau avec `info` (`get_mercenaries(army)`).
func show_market(army: String, info: Dictionary) -> void:
	army_id = army
	var region := str(info.get("region_name", ""))
	_title.text = "Mercenaires — %s" % region if region != "" else "Mercenaires"
	_status.text = status_text(info)
	var blocked := str(info.get("blocked", ""))
	_status.add_theme_color_override("font_color", HudStyle.RUBRIC if blocked != "" else HudStyle.INK_SOFT)
	var rows: Array = []
	for option in info.get("options", []):
		rows.append(row_for(option as Dictionary))
	if rows.is_empty():
		PanelWidgets.clear(_list)
		PanelWidgets.placeholder(_list, "Aucune compagnie ne se loue dans cette région en ce moment.")
	else:
		fill_cards(rows)


## Cartes en grille (même carte que le panier de recrutement) : disponibles d'abord, puis les refus.
## Un clic gauche sur une carte disponible émet `hire_requested` ; aucune carte ne tient de panier.
func fill_cards(rows: Array) -> void:
	PanelWidgets.clear(_list)
	var ordered: Array = rows.filter(func(r: Dictionary) -> bool: return bool(r.get("available", false)))
	ordered.append_array(rows.filter(func(r: Dictionary) -> bool: return not bool(r.get("available", false))))
	for row: Dictionary in ordered:
		var card := RecruitBasket.build_card(row)
		var unit_type := str(row.get("unit_type", ""))
		var available := bool(row.get("available", false))
		card.gui_input.connect(func(event: InputEvent) -> void:
			var click := event as InputEventMouseButton
			if available and click != null and click.pressed and click.button_index == MOUSE_BUTTON_LEFT:
				hire_requested.emit(army_id, unit_type))
		_list.add_child(card)


## Ligne d'état : refus de la région, sinon engagements restants et solde de la saison passée.
static func status_text(info: Dictionary) -> String:
	var blocked := str(info.get("blocked", ""))
	if blocked != "":
		return "Aucun engagement : %s." % blocked
	var left := int(info.get("hires_left", 0))
	var text := "%s encore ce tour." % FrText.count(left, "engagement possible", "engagements possibles")
	var premium := int(info.get("premium_last_turn", 0))
	if premium > 0:
		text += " Surcoût de solde la saison passée : %s." % Money.amount(premium)
	return text


## Ligne de recrutement : le nom de la compagnie suit celui de l'unité s'il en diffère.
static func row_for(option: Dictionary) -> Dictionary:
	var row := option.duplicate()
	var band := str(option.get("band_name", ""))
	var unit_name := str(option.get("name", ""))
	if band != "" and not unit_name.contains(band):
		row["name"] = "%s (%s)" % [unit_name, band]
	return row
