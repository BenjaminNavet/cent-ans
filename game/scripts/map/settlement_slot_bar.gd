class_name SettlementSlotBar
extends PanelContainer

## A6-L15 (ADR 0181) : barre des emplacements d'une colonie, à la Total War. Bande horizontale
## en bas de l'écran : une case par emplacement de bâtiment (icône et niveau du bâtiment, « + »
## pour une case vide, case grisée pour une case verrouillée), infobulle IB par case, clic =
## `slot_activated`. Aucune règle ici : les cases viennent de `CampaignSim.settlement_slots`
## (cœur, `building_slots`) ; la barre ne fait que les dessiner.

signal slot_activated(settlement_id: String, slot: Dictionary)

const CELL_SIZE := Vector2(46.0, 56.0)
const ICON_SIZE := 26
const SEPARATION := 4
const TITLE_WIDTH := 76.0

var settlement_id: String = ""
var slots: Array = []
var title_label: Label
var scroll: ScrollContainer
var cells_box: HBoxContainer
var max_width: float = 1000.0


func _init() -> void:
	name = "SettlementSlotBar"
	mouse_filter = Control.MOUSE_FILTER_STOP
	add_theme_stylebox_override("panel", HudStyle.card_box(HudStyle.PARCHMENT, HudStyle.GOLD, 2))
	var row := HBoxContainer.new()
	row.add_theme_constant_override("separation", 8)
	add_child(row)
	title_label = HudStyle.label("Bâtiments", HudStyle.FONT_SMALL, HudStyle.INK_SOFT)
	title_label.name = "Title"
	title_label.custom_minimum_size.x = TITLE_WIDTH
	title_label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	title_label.mouse_filter = Control.MOUSE_FILTER_IGNORE
	row.add_child(title_label)
	scroll = ScrollContainer.new()
	scroll.name = "Scroll"
	scroll.horizontal_scroll_mode = ScrollContainer.SCROLL_MODE_AUTO
	scroll.vertical_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
	row.add_child(scroll)
	cells_box = HBoxContainer.new()
	cells_box.name = "Cells"
	cells_box.add_theme_constant_override("separation", SEPARATION)
	scroll.add_child(cells_box)
	hide()


## `rows` : `CampaignSim.settlement_slots(id)`. `player_owner` : les cases ne se cliquent que
## pour une colonie du joueur. `place_name` titre la bande (nom de la colonie).
func set_slots(id: String, rows: Array, player_owner: bool, place_name: String = "") -> void:
	settlement_id = id
	slots = rows
	for child in cells_box.get_children():
		cells_box.remove_child(child)
		child.queue_free()
	var built := 0
	for slot in rows:
		if str((slot as Dictionary).get("built", "")) != "":
			built += 1
		cells_box.add_child(_cell(slot, player_owner))
	title_label.text = "%s\n%d / %d" % ["Bâtiments" if place_name == "" else place_name, built, rows.size()]
	fit(max_width)


## Largeur maximale offerte à la bande ; au-delà, les cases défilent.
func fit(width: float) -> void:
	max_width = width
	var cells := slots.size()
	var content := cells * CELL_SIZE.x + maxi(cells - 1, 0) * SEPARATION
	var margins := get_theme_stylebox("panel").get_minimum_size().x
	var room := maxf(width - margins - TITLE_WIDTH - 8.0, CELL_SIZE.x)
	scroll.custom_minimum_size = Vector2(minf(content, room), CELL_SIZE.y + 8.0)  # +8 : barre de défilement


## Nombre de cases.
func cell_count() -> int:
	return cells_box.get_child_count()


func _cell(slot: Dictionary, player_owner: bool) -> Button:
	var state := str(slot.get("state", "empty"))
	var button := RichButton.new()
	button.name = "Slot_" + str(slot.get("root", ""))
	button.custom_minimum_size = CELL_SIZE
	button.clip_text = true
	button.focus_mode = Control.FOCUS_NONE
	button.vertical_icon_alignment = VERTICAL_ALIGNMENT_TOP
	button.icon_alignment = HORIZONTAL_ALIGNMENT_CENTER
	button.add_theme_font_size_override("font_size", HudStyle.FONT_SMALL)
	var next: Array = slot.get("next", [])
	var shown := str(slot.get("built", "")) if str(slot.get("built", "")) != "" else str(slot.get("root", ""))
	IconLibrary.decorate_button(button, shown, ICON_SIZE, "building")
	var level := int(slot.get("level", 0))
	var max_level := int(slot.get("max_level", 1))
	match state:
		"built":
			button.text = "Niv. %d" % level if max_level > 1 else "Bâti"
		"upgradable":
			button.text = "%d/%d +" % [level, max_level]
		"locked":
			button.text = "verrou"
			button.modulate = Color(1.0, 1.0, 1.0, 0.45)
		_:
			button.text = "+"
			button.modulate = Color(1.0, 1.0, 1.0, 0.8)
	# Infobulle IB : le bâtiment debout (effets, entretien), sinon le premier bâtiment offert.
	if str(slot.get("built", "")) != "":
		RichTooltip.set_tooltip(button, "building", str(slot["built"]), {"name": str(slot.get("built_name", "")), "category": str(slot.get("category", "")), "upkeep": int(slot.get("upkeep", 0))})
	elif not next.is_empty():
		var option: Dictionary = next[0]
		RichTooltip.set_tooltip(button, "building", str(option.get("building", shown)), option)
	if state == "locked" and str(slot.get("locked_reason", "")) != "":
		button.tooltip_text = "[b]Emplacement verrouillé[/b]\n%s" % str(slot["locked_reason"])
	var can_act := player_owner and not next.is_empty() and state != "locked"
	button.disabled = not can_act
	if can_act:
		button.pressed.connect(func() -> void: slot_activated.emit(settlement_id, slot))
	return button
