class_name TownSoot
extends RefCounted

## Lot TB3 (ADR 0153), point 5 : suie **par ville** sur la carte de campagne (0-1), déduite de ce
## que le moteur expose déjà — aucune règle nouvelle :
## - dévastation de la province (`soot.devastation_*`) ;
## - siège en cours de la cité de la province (`soot.siege`) ;
## - prise de la place : son contrôleur change entre deux relectures. Saccage ou rasement si la
##   dévastation de la province monte en même temps d'au moins `soot.sack_devastation_rise`
##   (`data/rules/capture.json` : +25 et +40), sinon prise d'assaut si la place était assiégée.
##   Cette suie décroît de `soot.decay_per_turn` par tour.
## Le moteur ne garde pas d'état « ville saccagée » : la suie d'une prise vit dans cette session
## (elle ne survit pas à un rechargement ; celle de la dévastation, si).

var config: Dictionary = {}
var _controller: Dictionary = {}  # id → contrôleur à la dernière relecture
var _devastation: Dictionary = {}  # province → dévastation à la dernière relecture
var _besieged: Dictionary = {}  # province → assiégée à la dernière relecture
var _events: Dictionary = {}  # id → [suie de la prise, tour]
var _amounts: Dictionary = {}  # id → suie affichée
var _seen := false
var _turn := -1


func _init(p_config: Dictionary = {}) -> void:
	config = p_config


## Suie due à la seule dévastation de la province.
static func from_devastation(cfg: Dictionary, devastation: float) -> float:
	var start := float(cfg.get("devastation_start", 20.0))
	var full := maxf(float(cfg.get("devastation_full", 90.0)), start + 1.0)
	return float(cfg.get("devastation_max", 0.6)) * smoothstep(start, full, devastation)


## Relit l'état : `settlements` (dictionnaires `id`, `province`, `controller`, `kind`),
## `provinces` (province → `{devastation, besieged}`), `turn`. Rend les suies qui ont changé
## (id → suie).
func update(settlements: Array, provinces: Dictionary, turn: int) -> Dictionary:
	var changed := {}
	var decay := float(config.get("decay_per_turn", 0.1))
	# Partie chargée ou nouvelle campagne (le tour saute ou recule) : les contrôleurs relus ne
	# sont pas des prises, on repart de cet état.
	if _turn >= 0 and (turn < _turn or turn > _turn + 1):
		_seen = false
		_events.clear()
	_turn = turn
	for entry: Dictionary in settlements:
		var id := str(entry["id"])
		var province := str(entry["province"])
		var live: Dictionary = provinces.get(province, {})
		var devastation := float(live.get("devastation", 0.0))
		var controller := str(entry.get("controller", ""))
		if _seen and _controller.has(id) and str(_controller[id]) != controller:
			var rise := devastation - float(_devastation.get(province, devastation))
			var event := 0.0
			if rise >= float(config.get("sack_devastation_rise", 15.0)):
				event = float(config.get("sack", 0.9))
			elif bool(_besieged.get(province, false)):
				event = float(config.get("storm", 0.45))
			if event > 0.0:
				_events[id] = [event, turn]
		_controller[id] = controller
		var amount := from_devastation(config, devastation)
		if bool(live.get("besieged", false)) and str(entry.get("kind", "")) == "city":
			amount = maxf(amount, float(config.get("siege", 0.25)))
		if _events.has(id):
			var event: Array = _events[id]
			var left := float(event[0]) - decay * float(maxi(turn - int(event[1]), 0))
			if left <= 0.0:
				_events.erase(id)
			else:
				amount = maxf(amount, left)
		amount = snappedf(clampf(amount, 0.0, 1.0), 0.02)
		if not is_equal_approx(float(_amounts.get(id, 0.0)), amount):
			if amount > 0.0:
				_amounts[id] = amount
			else:
				_amounts.erase(id)
			changed[id] = amount
	for province: String in provinces:
		var live: Dictionary = provinces[province]
		_devastation[province] = float(live.get("devastation", 0.0))
		_besieged[province] = bool(live.get("besieged", false))
	_seen = true
	return changed


func amount_of(id: String) -> float:
	return float(_amounts.get(id, 0.0))


## Suie forcée d'une ville (captures, tests) : traitée comme une prise au tour `turn`.
func force(id: String, amount: float, turn: int = -1) -> void:
	_events[id] = [amount, turn if turn >= 0 else maxi(_turn, 0)]
	_amounts[id] = amount
