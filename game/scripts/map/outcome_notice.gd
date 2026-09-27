class_name OutcomeNotice
extends Node

## Lot CV3-4 : notice de la classe de résultat d'une bataille livrée sans la 3D (résolution
## automatique depuis le dialogue, fin de tour, attaque d'une armée IA) : bandeau
## `OutcomeBand` (« Victoire décisive · Normandie ») sous la barre du haut pendant quelques
## secondes (la ligne de chronique vient du cœur). Une bataille jouée en 3D a déjà son bandeau sur
## l'écran de fin : `mark_seen()` au retour sur la carte. Aucune règle : tout vient de
## `CampaignSim.get_last_battle_outcome()`.

const SHOW_SECONDS := 6.0
const FADE_SECONDS := 0.8

var map: Node = null  # CampaignMap
var band: OutcomeBand
var _seen := ""
## Faux jusqu'au premier `check` (campagne chargée) : le résultat d'une partie sauvegardée
## n'est pas annoncé.
var _primed := false
var _tween: Tween


func setup(campaign_map: Node) -> void:
	map = campaign_map
	name = "OutcomeNotice"
	band = OutcomeBand.create("", "", 24)
	band.name = "OutcomeNoticeBand"
	band.hide()
	map.ui.add_child(band)


func available() -> bool:
	return map != null and map.sim != null and map.sim.has_method("get_last_battle_outcome")


func _current() -> Dictionary:
	return map.sim.call("get_last_battle_outcome") if available() else {}


## Le dernier résultat ne donnera pas de notice (partie chargée, bataille 3D déjà montrée).
func mark_seen() -> void:
	_primed = true
	_seen = var_to_str(_current())


## Montre la notice si un nouveau résultat concerne le joueur ; vrai si elle s'affiche.
func check() -> bool:
	var outcome := _current()
	var signature := var_to_str(outcome)
	if not _primed:
		mark_seen()
		return false
	if outcome.is_empty() or signature == _seen:
		return false
	_seen = signature
	var picked := OutcomeBand.pick(outcome)
	if str(picked["label"]) == "":
		return false
	var text := str(picked["label"])
	var province := str(outcome.get("province", ""))
	if province != "" and map.has_method("province_name_of"):
		text += " · " + str(map.call("province_name_of", province))
	show_band(str(picked["class"]), text)
	return true


func show_band(key: String, text: String) -> void:
	band.set_outcome(key, text, 24)
	band.modulate.a = 1.0
	band.show()
	band.reset_size()
	var view: Vector2 = map.ui.get_viewport().get_visible_rect().size
	var top: Control = map.ui.get_node_or_null("TopBar")
	var y := (top.size.y if top != null else 40.0) + 110.0
	band.position = Vector2((view.x - band.size.x) * 0.5, y)
	if _tween != null:
		_tween.kill()
	_tween = band.create_tween()
	_tween.tween_interval(SHOW_SECONDS)
	_tween.tween_property(band, "modulate:a", 0.0, FADE_SECONDS)
	_tween.tween_callback(band.hide)
