class_name ChronicleController
extends Node

## Branche la chronique (M10) sur la carte de campagne : bouton « Chronique (n) » dans la barre,
## fenêtre `ChronicleWindow` ouverte en fin de tour quand une décision attend, file des décisions
## (la suivante s'affiche après chaque choix). Séparé de `campaign_map.gd` pour garder chaque
## jalon dans son fichier ; `campaign_map` n'appelle que `setup`, `refresh` et `after_end_turn`.

const WINDOW_SCENE := "res://scenes/ui/chronicle_window.tscn"

var map: Node = null  # CampaignMap
var window: ChronicleWindow
var button: Button


func setup(campaign_map: Node) -> void:
	map = campaign_map
	window = (load(WINDOW_SCENE) as PackedScene).instantiate()
	window.name = "ChronicleWindow"
	window.hide()
	map.ui.add_child(window)
	window.option_chosen.connect(_on_option_chosen)
	var court_button: Button = map.ui.court_button
	button = Button.new()
	button.text = "Chronique"
	button.add_theme_font_size_override("font_size", 18)
	button.tooltip_text = "Événements en attente de décision"
	court_button.get_parent().add_child(button)
	court_button.get_parent().move_child(button, court_button.get_index())
	button.pressed.connect(toggle_window)
	refresh()


func available() -> bool:
	return map != null and map.sim != null and map.sim.has_method("get_pending_decisions")


func pending() -> Array:
	return map.sim.call("get_pending_decisions") if available() else []


func toggle_window() -> void:
	if window.visible:
		window.hide()
	else:
		open_window()


## Ouvre la fenêtre sur la première décision en attente ; `false` s'il n'y en a aucune.
func open_window() -> bool:
	var decisions := pending()
	if decisions.is_empty():
		window.hide()
		if available():
			map.ui.show_toast("Aucun événement n'attend votre décision.")
		return false
	window.show_decision(decisions[0], decisions.size())
	return true


## Après tout changement d'état : compteur du bouton.
func refresh() -> void:
	if button == null:
		return
	button.visible = available()
	var count := pending().size()
	button.text = "Chronique (%d)" % count if count > 0 else "Chronique"
	button.disabled = count == 0


## Après `end_turn` : ouvre la fenêtre si une décision attend.
func after_end_turn() -> void:
	refresh()
	if not pending().is_empty():
		open_window()


func _on_option_chosen(decision_id: int, option_index: int) -> void:
	var result: Dictionary = map.sim.call("choose_event_option", decision_id, option_index)
	if result.get("ok", false):
		map.ui.show_toast("Décision prise.")
	else:
		map.ui.show_toast(str(result.get("error", "Choix impossible")), true)
	map.refresh_all()
	if pending().is_empty():
		window.hide()
	else:
		open_window()
	refresh()


## Mise en scène de la capture `--stage=chronicle` : joue des tours jusqu'à une décision
## historique (au plus 60), puis ouvre la fenêtre sur elle.
func stage_screenshot() -> void:
	if not available():
		return
	for _i in 60:
		var historical := pending().filter(func(d: Dictionary) -> bool: return d.get("historical", false))
		if not historical.is_empty():
			break
		map.sim.call("end_turn")
		# Les décisions aléatoires sont tranchées pour laisser la place à la chronique.
		for decision in pending():
			if not decision.get("historical", false):
				map.sim.call("choose_event_option", int(decision["id"]), 0)
	map.refresh_all()
	var date := str(map.sim.call("get_date_label"))
	map.ui.add_events(map.sim.call("get_events"), date)
	var decisions := pending()
	decisions.sort_custom(func(a: Dictionary, b: Dictionary) -> bool: return a.get("historical", false) and not b.get("historical", false))
	if not decisions.is_empty():
		window.show_decision(decisions[0], decisions.size())
	map.ui.set_date(date)
