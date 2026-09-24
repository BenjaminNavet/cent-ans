class_name BattleAudio
extends Node3D

## AU1 / T3 — audio de bataille spatialisé. Squelette : API publique, implémentation à venir.

static var active: BattleAudio = null
## Faux si un autre module (BV1) joue lui-même les sons de volée.
static var auto_volley: bool = true


## Joue un événement de la banque à une position monde ; false si rien n'est joué.
static func play_at(event_name: String, position: Vector3, gain_db: float = 0.0) -> bool:
	if active == null or not is_instance_valid(active):
		return false
	return active.play_event(event_name, position, gain_db)


## Comme `play_at`, après `delay` secondes (temps réel de la bataille).
static func play_at_delayed(event_name: String, position: Vector3, delay: float, gain_db: float = 0.0) -> void:
	if active == null or not is_instance_valid(active):
		return
	active.schedule(event_name, position, delay, gain_db)


func play_event(_event_name: String, _position: Vector3, _gain_db: float = 0.0) -> bool:
	return false


func schedule(_event_name: String, _position: Vector3, _delay: float, _gain_db: float = 0.0) -> void:
	pass
