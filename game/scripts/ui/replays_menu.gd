class_name ReplaysMenu
extends ListMenu

## EP13 — « Rejeux » du menu principal : les dernières batailles livrées, enregistrées à leur fin
## par le cœur (`BattleSim.save_replay`, dossier utilisateur `user://replays`, les N dernières
## gardées selon `data/rules/battle_replay.json`). Pour chacune : titre, armées, vainqueur, durée,
## date ; « Revoir » lance la scène de bataille en rejeu (`--replay=<fichier>`). Un rejeu d'un
## autre format de fichier est montré mais ne peut être revu. Échap ou « Fermer » : `closed`.

signal replay_started(path: String)

## Dossier des rejeux (les tests en imposent un autre).
static var dir_override: String = ""

var replays: Array = []
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


func _menu_title() -> String:
	return "Rejeux"


func _menu_hint() -> String:
	return "Revoyez les dernières batailles livrées, du premier trait à la déroute : lecture, pause, vitesse jusqu'à ×8, saut dans le temps, caméra libre. On regarde, on ne commande pas."


func _menu_width() -> float:
	return 780.0


func _scroll_height() -> float:
	return 520.0


func _load_entries() -> void:
	replays = load_replays()


func _build_entries(box: VBoxContainer) -> void:
	if replays.is_empty():
		empty_label = UiBuild.label("Aucun rejeu pour l'instant : chaque bataille livrée est enregistrée à sa fin.", 0, null, true, 720, box)
	for entry in replays:
		_add_replay(box, entry as Dictionary)


func _add_replay(box: VBoxContainer, entry: Dictionary) -> void:
	var path := str(entry.get("path", ""))
	box.add_child(HSeparator.new())
	var line := UiBuild.hbox(12, box)
	var texts := VBoxContainer.new()
	texts.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	line.add_child(texts)
	var name := UiBuild.label(str(entry.get("title", "")) if str(entry.get("title", "")) != "" else "Bataille")
	UiType.apply(name, UiType.HEADING)
	texts.add_child(name)
	var detail := UiBuild.label(summary(entry), 0, null, true, 560)
	UiType.apply(detail, UiType.CAPTION)
	detail.modulate = Color(1, 1, 1, 0.8)
	texts.add_child(detail)
	var button := UiBuild.button("Revoir")
	button.name = "Replay_%d" % buttons.size()
	button.size_flags_vertical = Control.SIZE_SHRINK_CENTER
	if bool(entry.get("readable", true)):
		button.pressed.connect(start.bind(path))
	else:
		button.disabled = true
		RichTooltip.attach_plain(button, "replay_wrong_format", {"body": "Enregistré par une autre version du jeu (format %d) : ce rejeu ne peut plus être revu." % int(entry.get("format", 0))})
	line.add_child(button)
	buttons[path] = button


## Lance la scène de bataille en rejeu du fichier `path`.
func start(path: String) -> void:
	replay_started.emit(path)
	_launch_battle(args_for(path))
