class_name VoicePool
extends RefCounted

## Pool de voix audio partagé (UI, bataille) : une voix = {player, event, priority, started, length}.
## Politique de `claim` : au-delà de `max_instances` du même événement, la plus ancienne de cet
## événement est remplacée ; sinon une voix libre ; sinon la voix de priorité la plus basse (puis
## la plus ancienne), si sa priorité est inférieure ou égale à celle du nouveau son.

var voices: Array = []
var _is_busy: Callable  # (voice: Dictionary) -> bool


func _init(busy_check: Callable) -> void:
	_is_busy = busy_check


func add(player: Node) -> Dictionary:
	var voice := {"player": player, "event": "", "priority": -1, "started": -1.0, "length": 0.0}
	voices.append(voice)
	return voice


func is_busy(voice: Dictionary) -> bool:
	return str(voice["event"]) != "" and bool(_is_busy.call(voice))


func busy_count() -> int:
	var count := 0
	for voice in voices:
		if is_busy(voice):
			count += 1
	return count


## Voix à utiliser pour `event_name` (dictionnaire vide si aucune ne peut être prise).
func claim(event_name: String, priority: int, max_instances: int) -> Dictionary:
	var same: Array = []
	var free: Dictionary = {}
	var victim: Dictionary = {}
	for voice in voices:
		if not is_busy(voice):
			if free.is_empty():
				free = voice
			continue
		if str(voice["event"]) == event_name:
			same.append(voice)
		if victim.is_empty() or int(voice["priority"]) < int(victim["priority"]) \
				or (int(voice["priority"]) == int(victim["priority"]) and float(voice["started"]) < float(victim["started"])):
			victim = voice
	if same.size() >= max_instances:
		var oldest: Dictionary = same[0]
		for voice in same:
			if float(voice["started"]) < float(oldest["started"]):
				oldest = voice
		return oldest
	if not free.is_empty():
		return free
	if not victim.is_empty() and int(victim["priority"]) <= priority:
		return victim
	return {}


static func assign(voice: Dictionary, event_name: String, priority: int, started: float, length: float = 0.0) -> void:
	voice["event"] = event_name
	voice["priority"] = priority
	voice["started"] = started
	voice["length"] = length
