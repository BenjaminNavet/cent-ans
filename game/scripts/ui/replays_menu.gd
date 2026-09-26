class_name ReplaysMenu
extends Control

## EP13 — « Rejeux » du menu principal : les dernières batailles livrées, enregistrées à leur fin
## par le cœur (`BattleSim.save_replay`, dossier utilisateur `user://replays`, les N dernières
## gardées selon `data/rules/battle_replay.json`). Pour chacune : titre, armées, vainqueur, durée,
## date ; « Revoir » lance la scène de bataille en rejeu (`--replay=<fichier>`). Un rejeu d'un
## autre format de fichier est montré mais ne peut être revu. Échap ou « Fermer » : `closed`.

signal closed
signal replay_started(path: String)

const BATTLE_SCENE := "res://scenes/battle/battle.tscn"

## Dossier des rejeux (les tests en imposent un autre).
static var dir_override: String = ""

var replays: Array = []
var buttons: Dictionary = {}  # chemin -> Button (tests)
var empty_label: Label = null


## Dossier utilisateur des rejeux (chemin absolu, pour le cœur).
static func replays_dir() -> String:
	if dir_override != "":
		return dir_override
	return ProjectSettings.globalize_path("user://replays")


## Rejeux enregistrés, du plus récent au plus ancien (`[]` sans l'extension).
static func load_replays() -> Array:
	if not ClassDB.class_exists("BattleSim"):
		return []
	var sim: Object = ClassDB.instantiate("BattleSim")
	if not sim.has_method("list_replays"):
		return []
	return sim.call("list_replays", replays_dir())


## « 12 min 05 s ».
static func duration_fr(seconds: float) -> String:
	var total := int(round(seconds))
	if total < 60:
		return "%d s" % total
	return "%d min %02d s" % [total / 60, total % 60]


## « 26/09/2026 à 14 h 05 » (heure locale).
static func date_fr(unix: int) -> String:
	if unix <= 0:
		return ""
	var bias := int(Time.get_time_zone_from_system().get("bias", 0)) * 60
	var d := Time.get_datetime_dict_from_unix_time(unix + bias)
	return "%02d/%02d/%d à %d h %02d" % [d["day"], d["month"], d["year"], d["hour"], d["minute"]]


## Ligne de résumé d'un rejeu : armées, vainqueur, durée, date.
static func summary(entry: Dictionary) -> String:
	var attacker := str(entry.get("attacker", ""))
	var defender := str(entry.get("defender", ""))
	var winner := str(entry.get("winner", ""))
	var verdict := "bataille inachevée"
	if winner == "attacker":
		verdict = "victoire %s" % BattleScene.de(attacker)
	elif winner == "defender":
		verdict = "victoire %s" % BattleScene.de(defender)
	var parts := ["%s contre %s" % [attacker, defender], verdict, duration_fr(float(entry.get("duration", 0.0)))]
	var date := date_fr(int(entry.get("recorded_at", 0)))
	if date != "":
		parts.append(date)
	return " · ".join(parts)


## Options de la scène de bataille pour revoir `path`.
static func args_for(path: String) -> PackedStringArray:
	return PackedStringArray(["--replay=" + path])


func _ready() -> void:
	set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	theme = load("res://scenes/ui/parchment_theme.tres")
	mouse_filter = Control.MOUSE_FILTER_STOP
	replays = load_replays()
	var veil := ColorRect.new()
	veil.color = Color(0.05, 0.03, 0.01, 0.5)
	veil.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	add_child(veil)
	var center := CenterContainer.new()
	center.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	add_child(center)
	var panel := PanelContainer.new()
	panel.custom_minimum_size = Vector2(780, 0)
	center.add_child(panel)
	var scroll := ScrollContainer.new()
	scroll.custom_minimum_size = Vector2(760, 520)
	scroll.horizontal_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
	panel.add_child(scroll)
	var box := VBoxContainer.new()
	box.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	box.add_theme_constant_override("separation", 8)
	scroll.add_child(box)
	var title := Label.new()
	title.text = "Rejeux"
	title.add_theme_font_size_override("font_size", 26)
	box.add_child(title)
	var hint := Label.new()
	hint.text = "Revoyez les dernières batailles livrées, du premier trait à la déroute : lecture, pause, vitesse jusqu'à ×8, saut dans le temps, caméra libre. On regarde, on ne commande pas."
	hint.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	hint.custom_minimum_size = Vector2(720, 0)
	hint.add_theme_font_size_override("font_size", 15)
	hint.modulate = Color(1, 1, 1, 0.75)
	box.add_child(hint)
	if replays.is_empty():
		empty_label = Label.new()
		empty_label.text = "Aucun rejeu pour l'instant : chaque bataille livrée est enregistrée à sa fin."
		empty_label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
		empty_label.custom_minimum_size = Vector2(720, 0)
		box.add_child(empty_label)
	for entry in replays:
		_add_replay(box, entry as Dictionary)
	var row := HBoxContainer.new()
	row.alignment = BoxContainer.ALIGNMENT_END
	box.add_child(row)
	var close_button := Button.new()
	close_button.name = "CloseButton"
	close_button.text = "Fermer"
	close_button.pressed.connect(close)
	row.add_child(close_button)
	if not buttons.is_empty():
		(buttons.values()[0] as Button).grab_focus.call_deferred()
	else:
		close_button.grab_focus.call_deferred()


func _add_replay(box: VBoxContainer, entry: Dictionary) -> void:
	var path := str(entry.get("path", ""))
	box.add_child(HSeparator.new())
	var line := HBoxContainer.new()
	line.add_theme_constant_override("separation", 12)
	box.add_child(line)
	var texts := VBoxContainer.new()
	texts.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	line.add_child(texts)
	var name := Label.new()
	name.text = str(entry.get("title", "")) if str(entry.get("title", "")) != "" else "Bataille"
	name.add_theme_font_size_override("font_size", 20)
	texts.add_child(name)
	var detail := Label.new()
	detail.text = summary(entry)
	detail.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	detail.custom_minimum_size = Vector2(560, 0)
	detail.add_theme_font_size_override("font_size", 14)
	detail.modulate = Color(1, 1, 1, 0.8)
	texts.add_child(detail)
	var button := Button.new()
	button.name = "Replay_%d" % buttons.size()
	button.text = "Revoir"
	button.size_flags_vertical = Control.SIZE_SHRINK_CENTER
	if bool(entry.get("readable", true)):
		button.pressed.connect(start.bind(path))
	else:
		button.disabled = true
		button.tooltip_text = "Enregistré par une autre version du jeu (format %d) : ce rejeu ne peut plus être revu." % int(entry.get("format", 0))
	line.add_child(button)
	buttons[path] = button


func _unhandled_input(event: InputEvent) -> void:
	if visible and event.is_action_pressed("ui_cancel"):
		close()
		get_viewport().set_input_as_handled()


func close() -> void:
	closed.emit()
	queue_free()


## Lance la scène de bataille en rejeu du fichier `path`.
func start(path: String) -> void:
	BattleScene.demo_args = args_for(path)
	replay_started.emit(path)
	var audio := get_node_or_null("/root/AudioDirector")
	if audio != null and audio.has_method("stop_all"):
		audio.call("stop_all")
	get_tree().change_scene_to_file(BATTLE_SCENE)
