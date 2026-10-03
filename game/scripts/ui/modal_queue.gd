class_name ModalQueue
extends RefCounted

## Lot A6-L6 (U10) : file unique des fenêtres modales de la campagne. Une seule est visible à la
## fois ; les décisions (chronique, sort d'une place prise) passent avant les rapports (rapport de
## saison). Les autres attendent la fermeture de la fenêtre active. Aucune règle de jeu : de la
## présentation pure.
##
## `request(key, priority, window, opener)` : `opener` (Callable sans argument, renvoie un
## booléen) montre la fenêtre quand son tour vient ; faux = rien à montrer. Une demande de même
## `key` que la fenêtre active rejoue seulement `opener` (contenu mis à jour, par exemple la
## décision suivante) ; de même `key` qu'une demande en attente, elle la remplace.

signal drained
signal changed

const PRIORITY_DECISION := 0
const PRIORITY_REPORT := 1

## Demandes en attente : `{key, priority, window, opener, seq}`.
var _pending: Array = []
var _active: Dictionary = {}
var _seq := 0


## Vrai tant qu'une fenêtre de la file est active ou en attente de son tour.
func is_busy() -> bool:
	return not _active.is_empty() or not _pending.is_empty()


## Clé de la fenêtre active (« » si aucune).
func active_key() -> String:
	return str(_active.get("key", ""))


## Clés en attente, dans l'ordre de passage.
func pending_keys() -> Array[String]:
	var ordered := _pending.duplicate()
	ordered.sort_custom(_before)
	var keys: Array[String] = []
	for entry: Dictionary in ordered:
		keys.append(str(entry["key"]))
	return keys


func request(key: String, priority: int, window: Control, opener: Callable) -> void:
	if not _active.is_empty() and str(_active["key"]) == key:
		_active["opener"] = opener
		opener.call()  # même fenêtre : contenu rafraîchi, elle reste la fenêtre active
		return
	for entry: Dictionary in _pending:
		if str(entry["key"]) == key:
			entry["opener"] = opener
			entry["priority"] = priority
			_pump()
			return
	_seq += 1
	_pending.append({"key": key, "priority": priority, "window": window, "opener": opener, "seq": _seq})
	_pump()


## Retire une demande en attente (le rapport d'un tour dépassé, par exemple).
func cancel(key: String) -> void:
	for i in range(_pending.size() - 1, -1, -1):
		if str((_pending[i] as Dictionary)["key"]) == key:
			_pending.remove_at(i)
	if _pending.is_empty() and _active.is_empty():
		drained.emit()


## Oublie tout (changement d'écran, chargement d'une partie).
func clear() -> void:
	_pending.clear()
	_release_active()
	changed.emit()


static func _before(a: Dictionary, b: Dictionary) -> bool:
	if int(a["priority"]) != int(b["priority"]):
		return int(a["priority"]) < int(b["priority"])
	return int(a["seq"]) < int(b["seq"])


func _pump() -> void:
	if not _active.is_empty() or _pending.is_empty():
		return
	_pending.sort_custom(_before)
	var entry: Dictionary = _pending.pop_front()
	_active = entry
	var window: Control = entry["window"]
	if is_instance_valid(window):
		var slot := _on_window_visibility.bind(entry)
		entry["slot"] = slot
		window.visibility_changed.connect(slot)
	var shown: bool = bool((entry["opener"] as Callable).call())
	if not shown and (not is_instance_valid(window) or not window.visible):
		_finish(entry)
		return
	entry["shown"] = is_instance_valid(window) and window.is_visible()
	changed.emit()


func _on_window_visibility(entry: Dictionary) -> void:
	if _active.is_empty() or _active["seq"] != entry["seq"]:
		return
	var window: Control = entry["window"]
	if not is_instance_valid(window):
		_finish(entry)
	elif window.visible:
		entry["shown"] = true  # un opener différé a fini par la montrer
	elif entry.get("shown", false):
		_finish(entry)


func _finish(entry: Dictionary) -> void:
	if _active.is_empty() or _active["seq"] != entry["seq"]:
		return
	_release_active()
	if _pending.is_empty():
		changed.emit()
		drained.emit()
	else:
		_pump()


func _release_active() -> void:
	if _active.is_empty():
		return
	var window: Control = _active["window"]
	var slot: Callable = _active.get("slot", Callable())
	if is_instance_valid(window) and slot.is_valid() and window.visibility_changed.is_connected(slot):
		window.visibility_changed.disconnect(slot)
	_active = {}
