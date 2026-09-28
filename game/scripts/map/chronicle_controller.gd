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
## Q2 : rapport de saison surveillé (la décision s'ouvre quand il est fermé).
var _watched_report: Control = null


func setup(campaign_map: Node) -> void:
	map = campaign_map
	window = (load(WINDOW_SCENE) as PackedScene).instantiate()
	window.name = "ChronicleWindow"
	window.hide()
	# PO1 : fenêtre de la chronique dans la zone `SIDE_PANEL` (un seul panneau à la fois), puis
	# inscrite dans la pile (Échap, exclusivité avec les grands panneaux) — dans cet ordre : un
	# reparentage après inscription la désinscrirait.
	UiZones.put(UiZones.Zone.SIDE_PANEL, window)
	map.ui.register_panel(window, PanelStack.Kind.CENTRAL)
	window.option_chosen.connect(_on_option_chosen)
	var court_button: Button = map.ui.court_button
	button = Button.new()
	button.text = "Chronique"
	UiType.apply(button, UiType.BODY)
	RichTooltip.attach_plain(button, "chronicle_pending")
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
	# UX2 : plus jamais grisé (le bouton paraissait désactivé) ; l'infobulle dit pourquoi il
	# est vide et quand il se remplira, un clic le rappelle aussi.
	button.disabled = false
	button.set_meta("count", count)
	var tooltip := "[b]Chronique[/b]\n"
	if count > 0:
		tooltip += "%s %s votre décision : deux saisons pour choisir, sinon le conseil tranche." % [FrText.count(count, "événement"), "attend" if count == 1 else "attendent"]
	else:
		tooltip += "Aucun événement n'attend votre décision pour l'instant. Les grands événements, historiques (Crécy, la Peste noire…) ou aléatoires, arrivent en fin de saison : le bouton affichera alors leur nombre."
	button.set_meta("tooltip", tooltip)
	if map.ui.has_method("refresh_top_button") and button.has_meta("top_label"):
		map.ui.refresh_top_button(button)
	else:
		button.text = "Chronique (%d)" % count if count > 0 else "Chronique"
		button.tooltip_text = tooltip


## Après `end_turn` : ouvre la fenêtre si une décision attend ; Q2 : après la fermeture du
## rapport de saison s'il s'affiche (plus deux fenêtres modales empilées à chaque tour).
func after_end_turn() -> void:
	refresh()
	if not pending().is_empty():
		_open_after_report.call_deferred()


func _open_after_report() -> void:
	var flow: Node = map.get("flow")
	var report: Control = flow.get("season_report") if flow != null else null
	if report != null and report.visible:
		if _watched_report != report:
			_watched_report = report
			report.visibility_changed.connect(_on_report_visibility)
		return
	if not pending().is_empty() and not window.visible:
		open_window()


func _on_report_visibility() -> void:
	if _watched_report == null or _watched_report.visible:
		return
	_watched_report.visibility_changed.disconnect(_on_report_visibility)
	_watched_report = null
	if not pending().is_empty() and not window.visible:
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
	map.ui.hide_province()
	var date := str(map.sim.call("get_date_label"))
	map.ui.add_events(map.sim.call("get_events"), date)
	var decisions := pending()
	decisions.sort_custom(func(a: Dictionary, b: Dictionary) -> bool: return a.get("historical", false) and not b.get("historical", false))
	if not decisions.is_empty():
		window.show_decision(decisions[0], decisions.size())
