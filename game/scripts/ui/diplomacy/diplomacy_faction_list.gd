class_name DiplomacyFactionList
extends DiplomacyView

## Colonne « Les puissances » : filtre par position diplomatique et une rangée par faction
## (blason, souverain, position, attitude ; raisons de l'attitude en infobulle).

signal faction_chosen(faction_id: String)

## Largeur plancher de la colonne (elle grandit avec l'écran).
const MIN_WIDTH := 230.0
const FILTERS := ["Toutes", "En guerre", "Alliés et vassaux", "En paix"]

var _entries: Array = []
var _selected: String = ""
var _filter := 0
var _list: VBoxContainer


func _init() -> void:
	super("Factions", 6, false)
	custom_minimum_size = Vector2(MIN_WIDTH, 0)
	add_child(_section("Les puissances"))
	var filter := OptionButton.new()
	for label in FILTERS:
		filter.add_item(label)
	filter.selected = _filter
	filter.item_selected.connect(func(index: int) -> void:
		_filter = index
		_render_list())
	add_child(filter)
	var scroll := ScrollContainer.new()
	scroll.size_flags_vertical = Control.SIZE_EXPAND_FILL
	scroll.horizontal_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
	_list = UiBuild.vbox(3)
	_list.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	scroll.add_child(_list)
	add_child(scroll)


## Reconstruit la liste pour les fiches `entries` (`get_diplomacy`), `selected` enfoncée.
func show_entries(entries: Array, selected: String) -> void:
	_entries = entries
	_selected = selected
	_render_list()


func _passes_filter(status: String) -> bool:
	match _filter:
		1:
			return status == "war"
		2:
			return status in ["alliance", "vassal", "suzerain"]
		3:
			return status in ["peace", "truce"]
	return true


func _render_list() -> void:
	UiBuild.clear_children(_list)
	for row_entry in _entries:
		var status := str(row_entry["status"])
		if not _passes_filter(status):
			continue
		_list.add_child(_faction_row(row_entry))


func _faction_row(row_entry: Dictionary) -> Control:
	var id := str(row_entry["id"])
	var status := str(row_entry["status"])
	var attitude := int(row_entry["attitude"])
	var row := Button.new()
	row.name = "Faction_%s" % id
	row.toggle_mode = true
	row.button_pressed = id == _selected
	row.custom_minimum_size = Vector2(0, 58)
	row.focus_mode = Control.FOCUS_NONE
	row.pressed.connect(func() -> void: faction_chosen.emit(id))
	var reasons := PackedStringArray(["Attitude envers nous : %+d" % attitude])
	for reason in row_entry.get("attitude_reasons", []):
		reasons.append("%+d  %s" % [int(reason["value"]), str(reason["text"])])
	TooltipHost.attach_plain(row, "faction_attitude", {"body": "\n".join(reasons)})
	var line := UiBuild.hbox(8)
	line.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	line.offset_left = 6
	line.offset_right = -6
	line.mouse_filter = Control.MOUSE_FILTER_IGNORE
	line.add_child(_heraldry(id, 36, str(row_entry.get("color", "#888888"))))
	var names := UiBuild.vbox(-2)
	names.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	names.alignment = BoxContainer.ALIGNMENT_CENTER
	names.mouse_filter = Control.MOUSE_FILTER_IGNORE
	var name_label := _label(str(row_entry["name"]), UiType.BODY, HudStyle.INK)
	name_label.mouse_filter = Control.MOUSE_FILTER_IGNORE
	names.add_child(name_label)
	var ruler := str(row_entry.get("ruler", ""))
	if ruler != "":
		var ruler_label := _label(ruler, UiType.CAPTION, HudStyle.INK_FADED)
		ruler_label.mouse_filter = Control.MOUSE_FILTER_IGNORE
		ruler_label.text_overrun_behavior = TextServer.OVERRUN_TRIM_ELLIPSIS
		names.add_child(ruler_label)
	line.add_child(names)
	var right := UiBuild.vbox(0)
	right.alignment = BoxContainer.ALIGNMENT_CENTER
	right.mouse_filter = Control.MOUSE_FILTER_IGNORE
	var symbol := Accessibility.relation_symbol(status) if Accessibility.colorblind() else ""
	var status_label := _label(("%s %s" % [symbol, STATUS_LABELS.get(status, status)]).strip_edges(), UiType.CAPTION, STATUS_COLORS.get(status, HudStyle.INK))
	status_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_RIGHT
	status_label.mouse_filter = Control.MOUSE_FILTER_IGNORE
	right.add_child(status_label)
	var attitude_label := _label("%+d" % attitude, UiType.BODY, HudStyle.GOOD if attitude >= 0 else HudStyle.POOR)
	attitude_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_RIGHT
	attitude_label.mouse_filter = Control.MOUSE_FILTER_IGNORE
	right.add_child(attitude_label)
	line.add_child(right)
	line.add_child(_attitude_gauge(attitude))
	row.add_child(line)
	return row


## Jauge verticale d'attitude (-100..100) : moitié haute verte, moitié basse rouge.
func _attitude_gauge(attitude: int) -> Control:
	var gauge := AttitudeGauge.new()
	gauge.attitude = attitude
	return gauge



## Jauge verticale d'attitude : trait médian, remplissage vert vers le haut, rouge vers le bas.
class AttitudeGauge:
	extends Control

	var attitude := 0

	func _init() -> void:
		custom_minimum_size = Vector2(8, 36)
		size_flags_vertical = Control.SIZE_SHRINK_CENTER
		mouse_filter = Control.MOUSE_FILTER_IGNORE

	func _draw() -> void:
		var mid := size.y * 0.5
		draw_rect(Rect2(Vector2.ZERO, size), HudStyle.PARCHMENT_DARK)
		var h := mid * clampf(absf(attitude) / 100.0, 0.0, 1.0)
		if attitude >= 0:
			draw_rect(Rect2(0, mid - h, size.x, h), HudStyle.GOOD)
		else:
			draw_rect(Rect2(0, mid, size.x, h), HudStyle.POOR)
		draw_line(Vector2(0, mid), Vector2(size.x, mid), HudStyle.INK, 1.0)
		draw_rect(Rect2(Vector2.ZERO, size), HudStyle.INK_SOFT, false, 1.0)
