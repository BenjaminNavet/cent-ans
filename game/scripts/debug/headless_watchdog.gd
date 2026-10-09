extends Node
## Garde-fou des exécutions headless (tests, captures, outils) : un test bloqué qui répète une
## erreur à chaque image a rempli le disque (206 Go de journaux Godot, 03/10). En headless
## seulement :
## - plus de `CENT_ANS_MAX_ERRORS` erreurs (20 000 par défaut) : arrêt, quelle que soit la cause ;
## - un script `res://tests/…` qui dure plus de `CENT_ANS_TEST_TIMEOUT_S` secondes (2700 par
##   défaut) : arrêt.
## La surveillance tourne dans un fil à part, pour arrêter aussi un fil principal bloqué :
## `quit` différé d'abord, puis `OS.kill` si le processus est encore là 15 s plus tard.
## Une variable à 0 désactive la limite correspondante.

const DEFAULT_MAX_ERRORS := 20000
const DEFAULT_TEST_TIMEOUT_S := 2700
const KILL_GRACE_MS := 15000
const EXIT_CODE := 124


class ErrorCounter extends Logger:
	var count := 0
	var _mutex := Mutex.new()

	func _log_error(_function: String, _file: String, _line: int, _code: String, _rationale: String, _editor_notify: bool, _error_type: int, _script_backtraces: Array[ScriptBacktrace]) -> void:
		_mutex.lock()
		count += 1
		_mutex.unlock()

	func _log_message(_message: String, error: bool) -> void:
		if error:
			_mutex.lock()
			count += 1
			_mutex.unlock()

	func read() -> int:
		_mutex.lock()
		var value := count
		_mutex.unlock()
		return value


var _counter: ErrorCounter
var _thread: Thread
var _stop := false
var _max_errors := 0
var _timeout_ms := 0
var _start_ms := 0


func _ready() -> void:
	if DisplayServer.get_name() != "headless" or Engine.is_editor_hint():
		return
	_max_errors = _env_int("CENT_ANS_MAX_ERRORS", DEFAULT_MAX_ERRORS)
	if CmdArgs.is_test_run():
		_timeout_ms = _env_int("CENT_ANS_TEST_TIMEOUT_S", DEFAULT_TEST_TIMEOUT_S) * 1000
	if _max_errors <= 0 and _timeout_ms <= 0:
		return
	_counter = ErrorCounter.new()
	OS.add_logger(_counter)
	_start_ms = Time.get_ticks_msec()
	_thread = Thread.new()
	_thread.start(_watch)


func _exit_tree() -> void:
	_stop = true
	if _thread != null and _thread.is_started():
		_thread.wait_to_finish()
	if _counter != null:
		OS.remove_logger(_counter)


func _watch() -> void:
	while not _stop:
		OS.delay_msec(1000)
		var reason := ""
		if _max_errors > 0 and _counter.read() > _max_errors:
			reason = "plus de %d erreurs (CENT_ANS_MAX_ERRORS)" % _max_errors
		elif _timeout_ms > 0 and Time.get_ticks_msec() - _start_ms > _timeout_ms:
			reason = "test bloqué plus de %d s (CENT_ANS_TEST_TIMEOUT_S)" % (_timeout_ms / 1000)
		if reason == "":
			continue
		printerr("HeadlessWatchdog: arrêt de %s : %s" % [CmdArgs.main_script(), reason])
		get_tree().call_deferred("quit", EXIT_CODE)
		var deadline := Time.get_ticks_msec() + KILL_GRACE_MS
		while not _stop and Time.get_ticks_msec() < deadline:
			OS.delay_msec(250)
		if not _stop:
			printerr("HeadlessWatchdog: fil principal bloqué, arrêt forcé")
			OS.kill(OS.get_process_id())
		return


static func _env_int(name: String, fallback: int) -> int:
	var raw := OS.get_environment(name)
	return int(raw) if raw.is_valid_int() else fallback

