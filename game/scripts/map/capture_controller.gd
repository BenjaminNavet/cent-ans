class_name CaptureController
extends Node

## TW2-T1 : sort de la place prise (occuper, mettre à rançon, piller, raser). Quand le joueur prend
## une place, le cœur ouvre une décision en attente (`CampaignSim.get_pending_captures`) ; ce
## contrôleur l'affiche dans une fenêtre `ChronicleWindow` (même parchemin que la chronique :
## titre, texte, un bouton par choix avec ses effets chiffrés, raser grisé sur une cité) et renvoie
## le choix par `choose_capture_outcome`. Aucune règle ici. Sans réponse, la place est occupée à la
## fin du tour (« Plus tard »). `campaign_map` n'appelle que `setup`, `refresh` et `after_end_turn`.

const WINDOW_SCENE := "res://scenes/ui/chronicle_window.tscn"

var map: Node = null  # CampaignMap
var window: ChronicleWindow
## Décision affichée (pour retrouver le sort d'un index de bouton).
var _shown: Dictionary = {}
## Décisions déjà proposées d'office (ne pas rouvrir après « Plus tard »).
var _offered: Dictionary = {}


func setup(campaign_map: Node) -> void:
	map = campaign_map
	window = (load(WINDOW_SCENE) as PackedScene).instantiate()
	window.name = "CaptureWindow"
	window.hide()
	# P2d : `Zone.SIDE_PANEL` (384 px de large à 1280×720) déborde sous le contenu d'une décision de
	# sort de place (titre, texte, quatre choix chiffrés) — largeur minimale de `ChronicleWindow`
	# 620 px. `Zone.MODAL` (mêmes usages que la rencontre et la déclaration de guerre, PO1) : plus
	# grande, centrée, bloque la carte le temps de la décision — cohérent avec le poids de ce choix.
	UiZones.put(UiZones.Zone.MODAL, window)
	map.ui.register_panel(window, PanelStack.Kind.CENTRAL)
	window.option_chosen.connect(_on_option_chosen)


func available() -> bool:
	return map != null and map.sim != null and map.sim.has_method("get_pending_captures")


func pending() -> Array:
	return map.sim.call("get_pending_captures") if available() else []


## Ouvre la fenêtre sur la première place prise en attente ; `false` s'il n'y en a aucune.
func open_window() -> bool:
	var captures := pending()
	if captures.is_empty():
		window.hide()
		map.ui.modal_queue.cancel("capture")
		return false
	show_capture(captures[0], captures.size())
	return true


func show_capture(capture: Dictionary, queue_size: int) -> void:
	_offered[int(capture.get("id", -1))] = true
	# A6-L6 (U10) : file modale unique (la décision passe avant le rapport de saison).
	map.ui.modal_queue.request("capture", ModalQueue.PRIORITY_DECISION, window, func() -> bool:
		_shown = capture
		window.show_decision(capture, queue_size)
		return true)


## Après tout changement d'état (un ordre de marche ou un assaut peut avoir pris une place) :
## propose d'office chaque nouvelle place prise, une fois.
func refresh() -> void:
	if not available():
		return
	var captures := pending()
	if captures.is_empty():
		if window.visible:
			window.hide()
		map.ui.modal_queue.cancel("capture")
		return
	if window.visible:
		return
	for capture in captures:
		if not _offered.has(int(capture.get("id", -1))):
			show_capture.call_deferred(capture, captures.size())
			return


## Après `end_turn` : une place prise pendant la fin de tour (garnison affamée) est proposée.
func after_end_turn() -> void:
	_offered.clear()
	refresh()


## Sort (`occupy`, `ransom`, `sack`, `raze`) du bouton `index` de la décision affichée.
func outcome_for(index: int) -> String:
	for option in _shown.get("options", []):
		if int((option as Dictionary).get("index", -1)) == index:
			return str(option.get("outcome", ""))
	return ""


func _on_option_chosen(decision_id: int, option_index: int) -> void:
	var outcome := outcome_for(option_index)
	var result: Dictionary = map.sim.call("choose_capture_outcome", decision_id, outcome)
	if result.get("ok", false):
		map.ui.show_toast("Ordre donné : %s." % str(_label_for(option_index)).to_lower())
	else:
		map.ui.show_toast(str(result.get("error", "Choix impossible")), true)
	_shown = {}
	map.refresh_all()
	if not open_window():
		window.hide()


func _label_for(index: int) -> String:
	for option in _shown.get("options", []):
		if int((option as Dictionary).get("index", -1)) == index:
			return str(option.get("text", ""))
	return ""
