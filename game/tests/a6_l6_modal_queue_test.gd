extends SceneTree

## Test headless du lot A6-L6 (U10) : file modale unique. Deux demandes simultanées : une seule
## fenêtre visible, l'autre après la fermeture de la première ; les décisions passent avant les
## rapports ; une demande de même clé que la fenêtre active la rafraîchit ; un opener qui ne montre
## rien ne bloque pas la file.
## Usage : godot --headless --path game --script res://tests/a6_l6_modal_queue_test.gd

var _failures := 0


func _init() -> void:
	await process_frame
	_test_two_requests()
	_test_priority()
	_test_same_key_refresh()
	_test_nothing_to_show()
	_test_held_toasts_signal()
	if _failures == 0:
		print("a6_l6 modal queue OK")
	quit(1 if _failures > 0 else 0)


func _check(condition: bool, message: String) -> bool:
	if not condition:
		_failures += 1
		push_error("a6_l6: " + message)
	return condition


func _window() -> Control:
	var window := Control.new()
	window.hide()
	root.add_child(window)
	return window


func _opener(window: Control, opened: Array, label: String) -> Callable:
	return func() -> bool:
		opened.append(label)
		window.show()
		return true


func _test_two_requests() -> void:
	var queue := ModalQueue.new()
	var first := _window()
	var second := _window()
	var opened: Array = []
	queue.request("capture", ModalQueue.PRIORITY_DECISION, first, _opener(first, opened, "capture"))
	queue.request("chronicle", ModalQueue.PRIORITY_DECISION, second, _opener(second, opened, "chronicle"))
	_check(first.visible and not second.visible, "two simultaneous requests: only the first is visible")
	_check(queue.active_key() == "capture" and queue.pending_keys() == ["chronicle"], "second waits in the queue")
	first.hide()
	_check(second.visible and not first.visible, "second shown after the first closes")
	_check(queue.active_key() == "chronicle" and queue.is_busy(), "second is now active")
	second.hide()
	_check(not queue.is_busy() and opened == ["capture", "chronicle"], "queue drained, each opened once: %s" % str(opened))
	first.queue_free()
	second.queue_free()


func _test_priority() -> void:
	var queue := ModalQueue.new()
	var active := _window()
	var report := _window()
	var decision := _window()
	var opened: Array = []
	queue.request("busy", ModalQueue.PRIORITY_DECISION, active, _opener(active, opened, "busy"))
	queue.request("report", ModalQueue.PRIORITY_REPORT, report, _opener(report, opened, "report"))
	queue.request("chronicle", ModalQueue.PRIORITY_DECISION, decision, _opener(decision, opened, "chronicle"))
	active.hide()
	_check(decision.visible and not report.visible, "decision before report")
	decision.hide()
	_check(report.visible, "report after the decision")
	report.hide()
	_check(not queue.is_busy(), "all closed")
	for window in [active, report, decision]:
		window.queue_free()


func _test_same_key_refresh() -> void:
	var queue := ModalQueue.new()
	var window := _window()
	var opened: Array = []
	queue.request("capture", ModalQueue.PRIORITY_DECISION, window, _opener(window, opened, "a"))
	queue.request("capture", ModalQueue.PRIORITY_DECISION, window, _opener(window, opened, "b"))
	_check(opened == ["a", "b"] and queue.pending_keys().is_empty(), "same key refreshes the active window (next decision)")
	window.hide()
	_check(not queue.is_busy(), "closed")
	window.queue_free()


func _test_nothing_to_show() -> void:
	var queue := ModalQueue.new()
	var empty := _window()
	var next := _window()
	var opened: Array = []
	queue.request("report", ModalQueue.PRIORITY_REPORT, empty, func() -> bool: return false)
	_check(not queue.is_busy(), "an opener with nothing to show does not block")
	queue.request("a", ModalQueue.PRIORITY_DECISION, next, _opener(next, opened, "a"))
	_check(next.visible, "next request shown")
	next.hide()
	empty.queue_free()
	next.queue_free()


func _test_held_toasts_signal() -> void:
	var queue := ModalQueue.new()
	var window := _window()
	var drained: Array = [0]
	queue.drained.connect(func() -> void: drained[0] += 1)
	queue.request("a", ModalQueue.PRIORITY_DECISION, window, func() -> bool:
		window.show()
		return true)
	_check(queue.is_busy() and drained[0] == 0, "busy while the window is open")
	window.hide()
	_check(drained[0] == 1 and not queue.is_busy(), "drained emitted once on close")
	window.queue_free()
