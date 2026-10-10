class_name MissionOfferController
extends Node

## WR turn (ADR 0304) : offre de mission en choix, à la manière des dilemmes de Warhammer III. Le
## cœur propose 2 ou 3 missions candidates (`CampaignSim.get_mission_offer`) ; ce contrôleur les
## affiche dans la fenêtre `ChronicleWindow` (même parchemin que la chronique et le sort d'une
## place prise : un bouton par mission avec objectif, délai et récompense, plus « Refuser »), et
## renvoie le choix par `choose_mission`. Aucune règle ici. Sans réponse, l'offre lapse au bout
## de quelques tours (« Plus tard » ne la retire pas).

const WINDOW_SCENE := "res://scenes/ui/chronicle_window.tscn"
## Index du bouton « Refuser » (après les candidates).
const REFUSE := -1

var map: Node = null  # CampaignMap
var window: ChronicleWindow
## Offre affichée (pour retrouver la mission d'un index de bouton).
var _shown: Dictionary = {}
## Offres déjà proposées d'office (ne pas rouvrir après « Plus tard »).
var _offered: Dictionary = {}


func setup(campaign_map: Node) -> void:
	map = campaign_map
	window = (load(WINDOW_SCENE) as PackedScene).instantiate()
	window.name = "MissionOfferWindow"
	window.hide()
	UiZones.put(UiZones.Zone.MODAL, window)
	map.ui.register_panel(window, PanelStack.Kind.CENTRAL)
	window.option_chosen.connect(_on_option_chosen)


func available() -> bool:
	return map != null and map.sim != null and map.sim.has_method("get_mission_offer")


## Offre en attente (`{}` s'il n'y en a pas).
func pending() -> Dictionary:
	return map.sim.call("get_mission_offer") if available() else {}


## Décision au format de `ChronicleWindow.show_decision` pour `offer`.
static func decision_for(offer: Dictionary) -> Dictionary:
	var candidates: Array = offer.get("candidates", [])
	var options: Array = []
	var text_lines := PackedStringArray()
	for candidate: Dictionary in candidates:
		var index := int(candidate.get("index", 0))
		var title := str(candidate.get("title", ""))
		var source := str(candidate.get("source", ""))
		text_lines.append("%d. %s — %s" % [index + 1, title, str(candidate.get("objective", ""))])
		options.append({
			"index": index,
			"text": title,
			"effects_text": "Délai : %d tours\nRécompense : %s" % [int(candidate.get("duration", 0)), str(candidate.get("reward", ""))],
			"allowed": true,
		})
		if source != "":
			text_lines.append("   (%s)" % source)
	options.append({
		"index": candidates.size(),
		"text": "Refuser toutes les missions",
		"effects_text": "Aucune mission pour l'instant ; une nouvelle offre viendra plus tard.",
		"allowed": true,
	})
	return {
		"id": int(offer.get("id", 0)),
		"kind": "mission",
		"kind_label": "✠ Offre de mission",
		"title": "Missions proposées",
		"text": "Votre conseil vous soumet plusieurs entreprises. Choisissez-en une, ou déclinez.\n\n" + "\n".join(text_lines),
		"expires_in": int(offer.get("turns_left", 1)),
		"options": options,
	}


## Ouvre la fenêtre sur l'offre en attente ; `false` s'il n'y en a pas.
func open_window() -> bool:
	var offer := pending()
	if offer.is_empty():
		window.hide()
		map.ui.modal_queue.cancel("mission_offer")
		return false
	show_offer(offer)
	return true


func show_offer(offer: Dictionary) -> void:
	_offered[int(offer.get("id", -1))] = true
	map.ui.modal_queue.request("mission_offer", ModalQueue.PRIORITY_DECISION, window, func() -> bool:
		_shown = offer
		window.show_decision(decision_for(offer), 1)
		return true)


## Après tout changement d'état : propose d'office chaque nouvelle offre, une fois.
func refresh() -> void:
	if not available():
		return
	var offer := pending()
	if offer.is_empty():
		if window.visible:
			window.hide()
		map.ui.modal_queue.cancel("mission_offer")
		return
	if window.visible or _offered.has(int(offer.get("id", -1))):
		return
	show_offer.call_deferred(offer)


func after_end_turn() -> void:
	refresh()


## Mission choisie par le bouton `option_index` (`REFUSE` : le dernier bouton, « Refuser »).
func choice_for(option_index: int) -> int:
	var candidates: Array = _shown.get("candidates", [])
	return option_index if option_index < candidates.size() else REFUSE


func _on_option_chosen(_decision_id: int, option_index: int) -> void:
	var choice := choice_for(option_index)
	var result: Dictionary = map.sim.call("choose_mission", choice)
	if result.get("ok", false):
		map.ui.show_toast("Mission acceptée." if choice != REFUSE else "Offre de mission refusée.")
	else:
		map.ui.show_toast(str(result.get("error", "Choix impossible")), true)
	_shown = {}
	map.refresh_all()
	if not open_window():
		window.hide()
