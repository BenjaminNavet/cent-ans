class_name MissionTracker
extends PanelContainer

## WH turn — suivi de missions toujours visible (haut droite, sous la barre du haut), repliable.
## Lit `CampaignSim.get_missions` (id, title, progress, progress_ratio, turns_left, province…) :
## une ligne par mission avec barre et « n saisons ». Clic sur une ligne : la province visée si
## elle existe, sinon le panneau Objectifs ; clic sur l'en-tête : plie / déplie. Aucune règle ici.

signal mission_activated(mission: Dictionary)
signal collapsed_changed(collapsed: bool)

const WIDTH := 250.0
## Échéance proche : la cloche s'allume aussi (`CampaignAlerts.mission_alerts`).
const DUE_TURNS := 1

var collapsed: bool = false
var missions: Array = []

var _box: VBoxContainer
var _title: Button
var _rows: VBoxContainer


func _init() -> void:
	name = "MissionTracker"
	custom_minimum_size.x = WIDTH
	mouse_filter = Control.MOUSE_FILTER_STOP
	add_theme_stylebox_override("panel", HudStyle.note_box(6))
	_box = UiBuild.vbox(3)
	add_child(_box)
	_title = Button.new()
	_title.name = "Title"
	_title.flat = true
	_title.alignment = HORIZONTAL_ALIGNMENT_LEFT
	_title.add_theme_font_size_override("font_size", UiType.size(UiType.CAPTION))
	_title.add_theme_color_override("font_color", HudStyle.RUBRIC)
	_title.pressed.connect(toggle)
	_box.add_child(_title)
	_rows = UiBuild.vbox(4)
	_rows.name = "Rows"
	_box.add_child(_rows)
	hide()


func toggle() -> void:
	set_collapsed(not collapsed)


func set_collapsed(value: bool) -> void:
	if value == collapsed:
		return
	collapsed = value
	_rebuild()
	collapsed_changed.emit(collapsed)


## Remplace les missions affichées ; masqué quand il n'y en a aucune.
func set_missions(new_missions: Array) -> void:
	missions = new_missions
	_rebuild()


## Libellé de l'échéance : « dernière saison » ou « n saisons ».
static func left_text(turns_left: int) -> String:
	return "dernière saison" if turns_left <= 1 else "%d saisons" % turns_left


## Mission dont l'échéance est proche (cloche).
static func is_due(mission: Dictionary) -> bool:
	return int(mission.get("turns_left", 99)) <= DUE_TURNS


func _rebuild() -> void:
	for child in _rows.get_children():
		child.queue_free()
		_rows.remove_child(child)
	visible = not missions.is_empty()
	_title.text = "%s Missions (%d)" % ["▸" if collapsed else "▾", missions.size()]
	_rows.visible = not collapsed
	if collapsed:
		return
	for mission: Dictionary in missions:
		_rows.add_child(_row(mission))


func _row(mission: Dictionary) -> Control:
	var row := VBoxContainer.new()
	row.name = "Mission%d" % int(mission.get("id", 0))
	row.mouse_filter = Control.MOUSE_FILTER_PASS
	var button := Button.new()
	button.name = "Head"
	button.flat = true
	button.alignment = HORIZONTAL_ALIGNMENT_LEFT
	button.clip_text = true
	var left := int(mission.get("turns_left", 0))
	button.text = "%s — %s" % [str(mission.get("title", "")), left_text(left)]
	button.tooltip_text = "%s\n%s\nRécompense : %s%s" % [
		str(mission.get("objective", "")), str(mission.get("progress", "")), str(mission.get("reward", "")),
		("\nSource : %s" % str(mission.get("source", ""))) if str(mission.get("source", "")) != "" else ""]
	button.add_theme_font_size_override("font_size", UiType.size(UiType.CAPTION))
	button.add_theme_color_override("font_color", HudStyle.RUBRIC if is_due(mission) else HudStyle.INK)
	button.pressed.connect(func() -> void: mission_activated.emit(mission))
	row.add_child(button)
	var bar := ProgressBar.new()
	bar.name = "Progress"
	bar.min_value = 0.0
	bar.max_value = 1.0
	bar.value = float(mission.get("progress_ratio", 0.0))
	bar.show_percentage = false
	bar.custom_minimum_size = Vector2(WIDTH - 20.0, 5)
	bar.mouse_filter = Control.MOUSE_FILTER_IGNORE
	row.add_child(bar)
	return row
