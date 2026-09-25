class_name NavalCampaign
extends Node

## Batailles navales depuis la carte de campagne (lot NV1) : quand une escadre ennemie
## intercepte une traversée (ordre « Embarquer », cœur `sim-campaign::naval`), l'écran
## d'avant-bataille navale s'ouvre avec « Combattre » (scène `naval_battle.tscn`),
## « Résolution automatique » ou « Retraite » ; le résultat est appliqué par le cœur
## (`resolve_naval_battle`, `auto_resolve_naval_battle`, `withdraw_naval_battle`).
## Un nœud enfant de la carte ; `campaign_map.gd` l'appelle en tête de `_offer_pending_battles`.

const NAVAL_SCENE := "res://scenes/naval/naval_battle.tscn"

var map: Node = null
var dialog: NavalPreBattleDialog = null


## Ouvre l'écran de la première bataille navale en attente ; `true` s'il y en a une (les
## batailles terrestres attendent qu'elle soit tranchée).
static func offer(p_map: Node) -> bool:
	var sim: Object = p_map.get("sim")
	if sim == null or not sim.has_method("get_pending_naval_battles"):
		return false
	var node := p_map.get_node_or_null("NavalCampaign") as NavalCampaign
	if node == null:
		node = NavalCampaign.new()
		node.name = "NavalCampaign"
		node.map = p_map
		p_map.add_child(node)
	return node._offer()


func _offer() -> bool:
	var sim: Object = map.get("sim")
	var pending: Array = sim.call("get_pending_naval_battles")
	if pending.is_empty():
		if dialog != null:
			dialog.visible = false
		return false
	if dialog == null:
		dialog = NavalPreBattleDialog.new()
		dialog.name = "NavalPreBattleDialog"
		(map.get("ui") as Node).add_child(dialog)
		dialog.fight_requested.connect(_on_fight)
		dialog.auto_requested.connect(_on_auto)
		dialog.withdraw_requested.connect(_on_withdraw)
	dialog.show_naval(sim, pending[0])
	return true


func _report(events: Array, suffix: String) -> void:
	var sim: Object = map.get("sim")
	var ui: Object = map.get("ui")
	if ui != null and ui.has_method("add_events"):
		ui.call("add_events", events, "%s%s" % [sim.call("get_date_label"), suffix])
	var flow: Object = map.get("flow")
	if flow != null and flow.has_method("report_late_events"):
		flow.call("report_late_events", events)


func _after() -> void:
	if map.has_method("refresh_all"):
		map.call("refresh_all")
	map.call("_offer_pending_battles")


func _on_auto(index: int) -> void:
	var result: Dictionary = map.get("sim").call("auto_resolve_naval_battle", index)
	if bool(result.get("ok", false)):
		_report(result.get("events", []), " (bataille navale, résolution automatique)")
	else:
		_toast(str(result.get("error", "?")))
	_after()


func _on_withdraw(index: int) -> void:
	var result: Dictionary = map.get("sim").call("withdraw_naval_battle", index)
	if bool(result.get("ok", false)):
		_report(result.get("events", []), "")
	else:
		_toast(str(result.get("error", "?")))
	_after()


func _on_fight(index: int, seed: int) -> void:
	var scene: Node = load(NAVAL_SCENE).instantiate()
	scene.call("configure", map.get("sim"), index, seed)
	scene.connect("returned", _on_returned.bind(scene))
	map.call("_set_campaign_active", false)
	get_tree().root.add_child(scene)


func _on_returned(result: Dictionary, scene: Node) -> void:
	scene.queue_free()
	map.call("_set_campaign_active", true)
	if bool(result.get("ok", false)):
		_report(result.get("events", []), " (bataille navale)")
	else:
		_toast("Résultat de bataille navale refusé : %s" % result.get("error", "?"))
	_after()


func _toast(text: String) -> void:
	var ui: Object = map.get("ui")
	if ui != null and ui.has_method("show_toast"):
		ui.call("show_toast", text, true)
