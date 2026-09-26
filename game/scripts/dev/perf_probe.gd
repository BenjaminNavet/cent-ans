class_name PerfProbe
extends RefCounted

## Lot SZ6 : minuteries par section de l'image, pour attribuer les pics de la carte de campagne
## aux scripts (banc `--bench-map --bench-probe`). Coupées par défaut : un appel à
## `Time.get_ticks_usec` et un test booléen par section.
##
## Usage : `var t := Time.get_ticks_usec()` … `t = PerfProbe.lap("section", t)` … ; le banc lit et
## remet à zéro les temps de l'image avec `take_frame()`. Une sous-section (« parent/nom ») est
## comptée dans sa section parente : le banc ne l'ajoute pas au total attribué.

static var enabled := false
static var _frame: Dictionary = {}


## Ajoute le temps écoulé depuis `t0_usec` à `label` (image courante) et rend l'instant présent,
## point de départ de la section suivante.
static func lap(label: String, t0_usec: int) -> int:
	var now := Time.get_ticks_usec()
	if enabled:
		_frame[label] = int(_frame.get(label, 0)) + (now - t0_usec)
	return now


## Temps (µs) par section depuis le dernier appel ; remet à zéro.
static func take_frame() -> Dictionary:
	var frame := _frame
	_frame = {}
	return frame
